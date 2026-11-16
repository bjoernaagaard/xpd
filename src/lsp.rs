//! `xpd lsp`: a language server for editors, over stdin and stdout. It lints the text the
//! editor holds with the code that `xpd check` runs, so both report the same issues.

mod project;
mod text;
mod transport;
mod uri;

use std::collections::{BTreeSet, HashMap, VecDeque};
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::rc::Rc;

use anyhow::{Context, Result};
use ast_grep_config::Severity;
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, RequestId, Response};
use lsp_types::notification::{
    Cancel, DidChangeConfiguration, DidChangeTextDocument, DidChangeWatchedFiles,
    DidChangeWorkspaceFolders, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument,
    Exit, Notification as NotificationType, PublishDiagnostics, ShowMessage,
};
use lsp_types::request::{
    CodeActionRequest, DocumentDiagnosticRequest, HoverRequest, Request as RequestType, Shutdown,
    WorkspaceDiagnosticRefresh,
};
use lsp_types::{
    CancelParams, CodeAction, CodeActionKind, CodeActionOptions, CodeActionOrCommand,
    CodeActionParams, CodeActionProviderCapability, CodeDescription, Diagnostic, DiagnosticOptions,
    DiagnosticServerCapabilities, DiagnosticSeverity, DidChangeConfigurationParams,
    DidChangeTextDocumentParams, DidChangeWatchedFilesParams, DidChangeWorkspaceFoldersParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
    DocumentChanges, DocumentDiagnosticParams, DocumentDiagnosticReport,
    FullDocumentDiagnosticReport, Hover, HoverContents, HoverParams, HoverProviderCapability,
    InitializeParams, InitializeResult, MarkupContent, MarkupKind, MessageType, NumberOrString,
    OneOf, OptionalVersionedTextDocumentIdentifier, PublishDiagnosticsParams,
    RelatedFullDocumentDiagnosticReport, SaveOptions, ServerCapabilities, ServerInfo,
    ShowMessageParams, TextDocumentEdit, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextDocumentSyncOptions, TextEdit, Uri, WorkspaceEdit, WorkspaceFoldersServerCapabilities,
    WorkspaceServerCapabilities,
};
use serde::Deserialize;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use self::project::{Place, Workspace};
use self::text::{Encoding, Lines};
use crate::catalog;
use crate::pack::Pack;
use crate::report::Issue;
use crate::rule::Applicability;
use crate::scan::{self, Found, Replacement};

/// The name on every diagnostic, and the last part of the fix-all kind.
const SOURCE: &str = "xpd";
/// The kind of the action that applies every fix, which an editor can run on save.
const FIX_ALL: CodeActionKind = CodeActionKind::new("source.fixAll.xpd");

/// Serves one client until it sends `exit`. The exit code is 0 when the client asked for
/// `shutdown` first and 1 otherwise, as the protocol says.
pub(crate) fn run() -> ExitCode {
    let (connection, io_threads) = match transport::connect() {
        Ok(connected) => connected,
        Err(e) => {
            eprintln!("xpd lsp: {e}");
            return ExitCode::FAILURE;
        }
    };
    let code = match serve(&connection) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("xpd lsp: {e:#}");
            // The thread that reads stdin may wait for a client that sends nothing more.
            return ExitCode::FAILURE;
        }
    };
    drop(connection);
    if let Err(e) = io_threads.join() {
        eprintln!("xpd lsp: {e}");
        return ExitCode::FAILURE;
    }
    code
}

fn serve(connection: &Connection) -> Result<ExitCode> {
    let (id, params) = connection
        .initialize_start()
        .context("the client sent no initialize request")?;
    // The protocol names the client's support for a refresh `workspace.diagnostics`, and
    // lsp-types 0.97 reads `workspace.diagnostic`, so this one answer comes from the JSON.
    let refresh = match params
        .pointer("/capabilities/workspace/diagnostics/refreshSupport")
        .and_then(Value::as_bool)
    {
        Some(true) => Refresh::Supported,
        _ => Refresh::Unsupported,
    };
    let params: InitializeParams =
        serde_json::from_value(params).context("the initialize request has invalid params")?;
    let mut server = Server::new(connection, &params, refresh);
    let watches = params
        .capabilities
        .workspace
        .as_ref()
        .and_then(|workspace| workspace.did_change_watched_files.as_ref())
        .is_some_and(|watcher| watcher.dynamic_registration == Some(true));
    let options = params.initialization_options.unwrap_or_default();
    let invalid_options = server.settings.update(&options).err();
    let result = InitializeResult {
        capabilities: server.capabilities(),
        server_info: Some(ServerInfo {
            name: SOURCE.to_owned(),
            version: Some(env!("CARGO_PKG_VERSION").to_owned()),
        }),
    };
    connection
        .initialize_finish(id, serde_json::to_value(result)?)
        .context("the client did not confirm the initialize result")?;
    if watches {
        server.send(
            Request::new(
                RequestId::from("xpd-rule-watchers".to_owned()),
                "client/registerCapability".to_owned(),
                serde_json::json!({"registrations": [{
                    "id": "xpd-rule-watchers",
                    "method": "workspace/didChangeWatchedFiles",
                    "registerOptions": {"watchers": [{
                        "globPattern": "**/*.{yml,yaml}",
                        "kind": 7,
                    }]},
                }]}),
            )
            .into(),
        );
    }
    if let Some(e) = invalid_options {
        server.show(MessageType::WARNING, format!("xpd: {e:#}"));
    }
    // A lint of a long file takes seconds, and the client sends more in that time. The server
    // takes every message that waits before it handles the next one, so it sees a request
    // that was cancelled, and it lints a document only when nothing waits or a request needs
    // the issues. Ten changes that wait cost one lint.
    let mut waiting = VecDeque::new();
    loop {
        for message in connection.receiver.try_iter() {
            server.queue(&mut waiting, message);
        }
        let Some(message) = waiting.pop_front() else {
            if server.publish_next() {
                continue;
            }
            match connection.receiver.recv() {
                Ok(message) => server.queue(&mut waiting, message),
                // The client closed the stream without `shutdown` and `exit`.
                Err(_) => return Ok(ExitCode::FAILURE),
            }
            continue;
        };
        match message {
            Message::Request(request) if request.method == Shutdown::METHOD => {
                server.send(Response::new_ok(request.id, ()).into());
                return Ok(wait_for_exit(connection, waiting));
            }
            Message::Request(request) => server.request(request),
            Message::Notification(notification) if notification.method == Exit::METHOD => {
                return Ok(ExitCode::FAILURE);
            }
            Message::Notification(notification) => server.notification(notification),
            // The only request the server sends asks the client to pull again, and its answer
            // says nothing.
            Message::Response(_) => {}
        }
    }
}

/// After `shutdown`, the server only waits for `exit` and refuses every request.
fn wait_for_exit(connection: &Connection, waiting: VecDeque<Message>) -> ExitCode {
    for message in waiting.into_iter().chain(&connection.receiver) {
        match message {
            Message::Notification(notification) if notification.method == Exit::METHOD => {
                return ExitCode::SUCCESS;
            }
            Message::Request(request) => {
                let refusal = failure(ErrorCode::InvalidRequest, "the server is shut down");
                let response = Response {
                    id: request.id,
                    response_result: Err(refusal),
                };
                // A failed send means the client is gone, and then the loop ends.
                let _ = connection.sender.send(response.into());
            }
            Message::Notification(_) | Message::Response(_) => {}
        }
    }
    ExitCode::FAILURE
}

fn failure(code: ErrorCode, message: impl Into<String>) -> lsp_server::ResponseError {
    #[expect(
        clippy::as_conversions,
        reason = "ErrorCode is a C-like enum of the protocol's i32 codes, without a From impl"
    )]
    let code = code as i32;
    lsp_server::ResponseError {
        code,
        message: message.into(),
        data: None,
    }
}

/// The options of `xpd check` that mean something in an editor.
#[derive(Debug, Default)]
struct Settings {
    /// `--unsafe-fixes`: the fix-all action also applies the fixes that may change behavior.
    unsafe_fixes: bool,
}

/// The settings a client sent. A key it leaves out keeps its value.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SettingsUpdate {
    unsafe_fixes: Option<bool>,
}

impl Settings {
    /// Reads `initializationOptions` or the settings of `workspace/didChangeConfiguration`.
    /// A client may send xpd's settings alone or under the key `xpd`.
    fn update(&mut self, sent: &Value) -> Result<()> {
        let sent = sent.get(SOURCE).unwrap_or(sent);
        if sent.is_null() {
            return Ok(());
        }
        let update: SettingsUpdate = serde_json::from_value(sent.clone())
            .context("invalid settings. xpd reads {\"unsafeFixes\": true or false}")?;
        if let Some(unsafe_fixes) = update.unsafe_fixes {
            self.unsafe_fixes = unsafe_fixes;
        }
        Ok(())
    }

    /// The fix level of the fix-all action, as `--fix` and `--fix --unsafe-fixes` pick it.
    fn allowed(&self) -> Applicability {
        if self.unsafe_fixes {
            Applicability::Unsafe
        } else {
            Applicability::Safe
        }
    }
}

/// How the client wants its diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Delivery {
    /// The server sends `textDocument/publishDiagnostics` after every change.
    Push,
    /// The client asks with `textDocument/diagnostic`. Pushing as well would show every issue
    /// twice.
    Pull(Refresh),
}

/// Whether the client answers `workspace/diagnostic/refresh`, which asks it to pull again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Refresh {
    Supported,
    Unsupported,
}

/// How an edit names the document it changes.
#[derive(Debug, Clone, Copy)]
enum EditForm {
    /// `documentChanges`, which carries the document's version, so the client refuses an edit
    /// that was made for older text.
    Versioned,
    /// `changes`, which every client reads.
    Plain,
}

/// What the client said it can do, for the parts where the server has a choice.
#[derive(Debug, Clone, Copy)]
struct Client {
    encoding: Encoding,
    delivery: Delivery,
    edit_form: EditForm,
    hover_format: HoverFormat,
}

/// The format of a hover. A rule's documentation is Markdown, and a client that cannot draw
/// Markdown gets the same text as it is.
#[derive(Debug, Clone, Copy)]
enum HoverFormat {
    Markdown,
    PlainText,
}

impl Client {
    fn new(params: &InitializeParams, refresh: Refresh) -> Self {
        let capabilities = &params.capabilities;
        let general = capabilities.general.as_ref();
        let text_document = capabilities.text_document.as_ref();
        let workspace = capabilities.workspace.as_ref();
        let encodings = general.and_then(|general| general.position_encodings.as_deref());
        let pulls = text_document.is_some_and(|document| document.diagnostic.is_some());
        let versioned = workspace
            .and_then(|workspace| workspace.workspace_edit.as_ref())
            .and_then(|edit| edit.document_changes)
            .unwrap_or(false);
        let markdown = text_document
            .and_then(|document| document.hover.as_ref())
            .and_then(|hover| hover.content_format.as_ref())
            .is_some_and(|formats| formats.contains(&MarkupKind::Markdown));
        Self {
            encoding: Encoding::negotiate(encodings),
            delivery: if pulls {
                Delivery::Pull(refresh)
            } else {
                Delivery::Push
            },
            edit_form: if versioned {
                EditForm::Versioned
            } else {
                EditForm::Plain
            },
            hover_format: if markdown {
                HoverFormat::Markdown
            } else {
                HoverFormat::PlainText
            },
        }
    }
}

/// One open document: the text the editor holds, which may differ from the file on disk.
#[derive(Debug)]
struct Document {
    uri: Uri,
    /// `None` for a document the server read from disk because the client never opened it.
    version: Option<i32>,
    text: String,
    place: Place,
    /// The issues of `text` at `place`, once something asked for them. `None` after the text,
    /// the place or the project's rules changed.
    linted: Option<Linted>,
}

/// A document's issues, with the rules that found them.
#[derive(Debug)]
struct Linted {
    packs: Rc<[Pack]>,
    found: Vec<Found>,
}

/// The issues of `document`, linted now unless the document still holds them. Diagnostics, a
/// hover and a code action for one version of a document share one lint this way.
fn lint<'d>(
    workspace: &mut Workspace,
    document: &'d mut Document,
) -> Result<(&'d Document, &'d Linted)> {
    if document.linted.is_none() {
        let packs = Rc::clone(&workspace.project(&document.place.root)?.packs);
        let disk = document.place.disk();
        let file = scan::File {
            path: &document.place.path,
            disk: Some(&disk),
        };
        let found = scan::lint_source(file, &document.text, &packs);
        document.linted = Some(Linted { packs, found });
    }
    let document: &'d Document = document;
    let linted = document
        .linted
        .as_ref()
        .context("the document was not linted")?;
    Ok((document, linted))
}

/// The part of a match that the editor marks: from where the match starts, which is the
/// position `xpd check` prints, to where it ends or its first line ends. Marking a whole loop
/// for an issue about its first line would hide the other issues inside it.
fn marked(found: &Found, lines: &Lines<'_>) -> Range<usize> {
    let start = found.range.start;
    start..found.range.end.min(lines.line_end(start))
}

fn lsp_range(lines: &Lines<'_>, bytes: &Range<usize>) -> lsp_types::Range {
    lsp_types::Range {
        start: lines.position(bytes.start),
        end: lines.position(bytes.end),
    }
}

fn severity(severity: &Severity) -> DiagnosticSeverity {
    match severity {
        Severity::Error => DiagnosticSeverity::ERROR,
        Severity::Warning => DiagnosticSeverity::WARNING,
        Severity::Info => DiagnosticSeverity::INFORMATION,
        // A rule that is off is not loaded, so no issue has that severity.
        Severity::Hint | Severity::Off => DiagnosticSeverity::HINT,
    }
}

/// The diagnostic for `issue` at `range`: its rule as the code, with the rule's section of the
/// rules page as the code's link.
fn diagnostic(issue: &Issue, range: lsp_types::Range) -> Diagnostic {
    Diagnostic {
        range,
        severity: Some(severity(&issue.severity)),
        code: Some(NumberOrString::String(issue.rule.clone())),
        code_description: catalog::url(&issue.rule)
            .parse()
            .ok()
            .map(|href| CodeDescription { href }),
        source: Some(SOURCE.to_owned()),
        message: issue.message.clone(),
        ..Diagnostic::default()
    }
}

fn diagnostics(document: &Document, found: &[Found], encoding: Encoding) -> Vec<Diagnostic> {
    let lines = Lines::new(&document.text, encoding);
    found
        .iter()
        .map(|found| diagnostic(&found.issue, lsp_range(&lines, &marked(found, &lines))))
        .collect()
}

/// The range of `old` that differs from `new`, with the text `new` has there. The fix-all
/// action sends this and not the whole document, so the editor keeps its cursor and folds.
fn changed_part(old: &str, new: &str) -> Replacement {
    let same = |(a, b): &(u8, u8)| a == b;
    let prefix = old.bytes().zip(new.bytes()).take_while(same).count();
    let prefix = old.floor_char_boundary(prefix);
    let (old_rest, new_rest) = (&old[prefix..], &new[prefix..]);
    let suffix = old_rest
        .bytes()
        .rev()
        .zip(new_rest.bytes().rev())
        .take_while(same)
        .count();
    let old_end = old.ceil_char_boundary(old.len() - suffix);
    let new_end = new.len() - (old.len() - old_end);
    Replacement {
        range: prefix..old_end,
        text: new[prefix..new_end].to_owned(),
    }
}

/// Whether a client that asks for the kinds in `only` wants an action of `kind`. A kind
/// includes the kinds below it, so `source.fixAll` asks for `source.fixAll.xpd` too.
fn wants(only: Option<&[CodeActionKind]>, kind: &CodeActionKind) -> bool {
    only.is_none_or(|only| {
        only.iter().any(|asked| {
            kind.as_str()
                .strip_prefix(asked.as_str())
                .is_some_and(|rest| rest.is_empty() || rest.starts_with('.'))
        })
    })
}

struct Server<'c> {
    connection: &'c Connection,
    client: Client,
    settings: Settings,
    workspace: Workspace,
    /// The open documents, by their address as the client wrote it.
    documents: HashMap<String, Document>,
    /// The file that holds each project's package warnings, by the project's folder.
    package_files: HashMap<PathBuf, Uri>,
    /// The open documents whose issues the client does not have yet, when the server pushes
    /// them.
    unpublished: BTreeSet<String>,
    /// The number of requests the server has sent, which makes the next request's id.
    sent_requests: i32,
}

impl<'c> Server<'c> {
    fn new(connection: &'c Connection, params: &InitializeParams, refresh: Refresh) -> Self {
        let mut workspace = Workspace::default();
        #[expect(
            deprecated,
            reason = "a client older than workspace folders names its one folder in rootUri"
        )]
        let root = params.root_uri.as_ref();
        match &params.workspace_folders {
            Some(folders) => folders
                .iter()
                .for_each(|folder| workspace.add_folder(&folder.uri)),
            None => root.into_iter().for_each(|root| workspace.add_folder(root)),
        }
        Self {
            connection,
            client: Client::new(params, refresh),
            settings: Settings::default(),
            workspace,
            documents: HashMap::new(),
            package_files: HashMap::new(),
            unpublished: BTreeSet::new(),
            sent_requests: 0,
        }
    }

    /// Only what the server answers. xpd has no data for completion, definitions, symbols,
    /// formatting or anything else that is missing here.
    fn capabilities(&self) -> ServerCapabilities {
        ServerCapabilities {
            position_encoding: Some(self.client.encoding.kind()),
            text_document_sync: Some(TextDocumentSyncCapability::Options(
                TextDocumentSyncOptions {
                    open_close: Some(true),
                    change: Some(TextDocumentSyncKind::INCREMENTAL),
                    save: Some(
                        SaveOptions {
                            include_text: Some(false),
                        }
                        .into(),
                    ),
                    ..TextDocumentSyncOptions::default()
                },
            )),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            code_action_provider: Some(CodeActionProviderCapability::Options(CodeActionOptions {
                code_action_kinds: Some(vec![CodeActionKind::QUICKFIX, FIX_ALL]),
                ..CodeActionOptions::default()
            })),
            diagnostic_provider: Some(DiagnosticServerCapabilities::Options(DiagnosticOptions {
                identifier: Some(SOURCE.to_owned()),
                // A rule reads one file at a time.
                inter_file_dependencies: false,
                workspace_diagnostics: false,
                ..DiagnosticOptions::default()
            })),
            workspace: Some(WorkspaceServerCapabilities {
                workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                    supported: Some(true),
                    change_notifications: Some(OneOf::Left(true)),
                }),
                file_operations: None,
            }),
            ..ServerCapabilities::default()
        }
    }

    /// A failed send means the client is gone, and then the loop that reads its messages ends.
    fn send(&self, message: Message) {
        let _ = self.connection.sender.send(message);
    }

    fn notify<N: NotificationType>(&self, params: N::Params) {
        self.send(Notification::new(N::METHOD.to_owned(), params).into());
    }

    fn show(&self, typ: MessageType, message: String) {
        self.notify::<ShowMessage>(ShowMessageParams { typ, message });
    }

    /// Puts a message of the client behind the ones that wait in `waiting`. A
    /// `$/cancelRequest` takes the request it names out of them and answers it as cancelled.
    /// It does nothing for a request the server has answered, as the protocol allows.
    fn queue(&self, waiting: &mut VecDeque<Message>, message: Message) {
        let cancelled = match &message {
            Message::Notification(notification) if notification.method == Cancel::METHOD => {
                serde_json::from_value::<CancelParams>(notification.params.clone()).ok()
            }
            _ => {
                waiting.push_back(message);
                return;
            }
        };
        let Some(cancelled) = cancelled else {
            return;
        };
        let id = match cancelled.id {
            NumberOrString::Number(id) => RequestId::from(id),
            NumberOrString::String(id) => RequestId::from(id),
        };
        let named = waiting.iter().position(
            |waits| matches!(waits, Message::Request(request) if request.id == id && request.method != Shutdown::METHOD),
        );
        if let Some(Message::Request(request)) = named.and_then(|at| waiting.remove(at)) {
            let response = Response {
                id: request.id,
                response_result: Err(failure(
                    ErrorCode::RequestCanceled,
                    format!("the client cancelled {}", request.method),
                )),
            };
            self.send(response.into());
        }
    }

    /// Sends the issues of one document that waits for them, and returns whether one did.
    fn publish_next(&mut self) -> bool {
        let Some(key) = self.unpublished.pop_first() else {
            return false;
        };
        if let Err(e) = self.publish(&key) {
            self.show(MessageType::ERROR, format!("xpd: {e:#}"));
        }
        true
    }

    fn request(&mut self, request: Request) {
        let Request { id, method, params } = request;
        let answer = match method.as_str() {
            HoverRequest::METHOD => self.answer(params, Self::hover),
            CodeActionRequest::METHOD => self.answer(params, Self::code_actions),
            DocumentDiagnosticRequest::METHOD => self.answer(params, Self::pull_diagnostics),
            _ => Err(failure(
                ErrorCode::MethodNotFound,
                format!("xpd does not answer {method}"),
            )),
        };
        let response = Response {
            id,
            response_result: answer,
        };
        self.send(response.into());
    }

    fn answer<P: DeserializeOwned, R: Serialize>(
        &mut self,
        params: Value,
        handler: fn(&mut Self, P) -> Result<R>,
    ) -> Result<Value, lsp_server::ResponseError> {
        let params = serde_json::from_value(params)
            .map_err(|e| failure(ErrorCode::InvalidParams, e.to_string()))?;
        handler(self, params)
            .and_then(|result| Ok(serde_json::to_value(result)?))
            .map_err(|e| failure(ErrorCode::InternalError, format!("{e:#}")))
    }

    fn notification(&mut self, notification: Notification) {
        let Notification { method, params } = notification;
        let handled = match method.as_str() {
            DidOpenTextDocument::METHOD => self.handle(params, Self::did_open),
            DidChangeTextDocument::METHOD => self.handle(params, Self::did_change),
            DidSaveTextDocument::METHOD => self.handle(params, Self::did_save),
            DidCloseTextDocument::METHOD => self.handle(params, Self::did_close),
            DidChangeConfiguration::METHOD => self.handle(params, Self::did_change_configuration),
            DidChangeWatchedFiles::METHOD => self.handle(params, Self::did_change_watched_files),
            DidChangeWorkspaceFolders::METHOD => {
                self.handle(params, Self::did_change_workspace_folders)
            }
            // The protocol lets a server ignore a notification it does not know.
            _ => Ok(()),
        };
        if let Err(e) = handled {
            self.show(MessageType::ERROR, format!("xpd: {method}: {e:#}"));
        }
    }

    fn handle<P: DeserializeOwned>(
        &mut self,
        params: Value,
        handler: fn(&mut Self, P) -> Result<()>,
    ) -> Result<()> {
        handler(self, serde_json::from_value(params)?)
    }

    fn did_open(&mut self, params: DidOpenTextDocumentParams) -> Result<()> {
        let opened = params.text_document;
        let place = self.workspace.place(&opened.uri);
        let root = place.root.clone();
        let document = Document {
            uri: opened.uri,
            version: Some(opened.version),
            text: opened.text,
            place,
            linted: None,
        };
        let key = document.uri.as_str().to_owned();
        self.documents.insert(key.clone(), document);
        if self.client.delivery == Delivery::Push {
            self.unpublished.insert(key);
        }
        self.refresh_project(&root)
    }

    fn did_change(&mut self, params: DidChangeTextDocumentParams) -> Result<()> {
        let key = params.text_document.uri.as_str();
        let document = self
            .documents
            .get_mut(key)
            .with_context(|| format!("{key} is not open"))?;
        for change in params.content_changes {
            text::apply(&mut document.text, change, self.client.encoding);
        }
        document.version = Some(params.text_document.version);
        document.linted = None;
        // The issues go out when no message waits, so one lint serves every change that does.
        if self.client.delivery == Delivery::Push {
            self.unpublished.insert(key.to_owned());
        }
        Ok(())
    }

    /// A save is when the project may have changed too, for example after `uv add`, so the
    /// server reads the venv again. The saved file can also be a module that another open
    /// document imports, which a rule reads from disk, so the documents of the project are
    /// linted again.
    fn did_save(&mut self, params: DidSaveTextDocumentParams) -> Result<()> {
        let uri = params.text_document.uri;
        let Some(document) = self.documents.get(uri.as_str()) else {
            return Ok(());
        };
        let root = document.place.root.clone();
        self.workspace.refresh(&root)?;
        self.publish_package_issues(&root)?;
        self.lint_again(&root);
        Ok(())
    }

    #[expect(
        clippy::unnecessary_wraps,
        reason = "every notification handler has the same signature"
    )]
    fn did_close(&mut self, params: DidCloseTextDocumentParams) -> Result<()> {
        let uri = params.text_document.uri;
        let Some(closed) = self.documents.remove(uri.as_str()) else {
            return Ok(());
        };
        self.unpublished.remove(uri.as_str());
        // The issues are about the editor's text, so they go when the editor lets go of it.
        if self.client.delivery == Delivery::Push {
            self.notify::<PublishDiagnostics>(PublishDiagnosticsParams::new(uri, Vec::new(), None));
        }
        let root = closed.place.root;
        if !self.roots().contains(&root) {
            self.close_project(&root);
        }
        Ok(())
    }

    fn did_change_configuration(&mut self, params: DidChangeConfigurationParams) -> Result<()> {
        let settings = params.settings;
        self.settings.update(&settings)
    }

    fn did_change_watched_files(&mut self, params: DidChangeWatchedFilesParams) -> Result<()> {
        let changed = params.changes.into_iter().any(|change| {
            uri::to_path(&change.uri).is_some_and(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "yml" || extension == "yaml")
            })
        });
        if !changed {
            return Ok(());
        }
        for root in self.roots() {
            self.refresh_project(&root)?;
        }
        Ok(())
    }

    /// A document may now belong to another project, with another venv and another path for
    /// the globs of the rules, so every document is placed and linted again.
    fn did_change_workspace_folders(
        &mut self,
        params: DidChangeWorkspaceFoldersParams,
    ) -> Result<()> {
        let event = params.event;
        for folder in event.removed {
            self.workspace.remove_folder(&folder.uri);
        }
        for folder in event.added {
            self.workspace.add_folder(&folder.uri);
        }
        let before = self.roots();
        for (key, document) in &mut self.documents {
            document.place = self.workspace.place(&document.uri);
            document.linted = None;
            if self.client.delivery == Delivery::Push {
                self.unpublished.insert(key.clone());
            }
        }
        let now = self.roots();
        for root in before.difference(&now) {
            self.close_project(root);
        }
        for root in &now {
            self.refresh_project(root)?;
        }
        if self.client.delivery == Delivery::Pull(Refresh::Supported) {
            self.ask_to_pull_again();
        }
        Ok(())
    }

    /// The project folders of the open documents.
    fn roots(&self) -> BTreeSet<PathBuf> {
        self.documents
            .values()
            .map(|document| document.place.root.clone())
            .collect()
    }

    /// Reads the venv of the project in `root` again, and brings what the client shows for the
    /// project up to date.
    fn refresh_project(&mut self, root: &Path) -> Result<()> {
        let rules_changed = self.workspace.refresh(root)?;
        self.publish_package_issues(root)?;
        if rules_changed {
            self.lint_again(root);
        }
        Ok(())
    }

    /// Drops the issues of every open document of the project in `root`, and has the client
    /// show the new ones.
    fn lint_again(&mut self, root: &Path) {
        let documents = self.documents.iter_mut();
        for (key, document) in documents.filter(|(_, document)| document.place.root == root) {
            document.linted = None;
            if self.client.delivery == Delivery::Push {
                self.unpublished.insert(key.clone());
            }
        }
        if self.client.delivery == Delivery::Pull(Refresh::Supported) {
            self.ask_to_pull_again();
        }
    }

    /// Takes back the package warnings of the project in `root`, after its last document
    /// closed or moved to another project.
    fn close_project(&mut self, root: &Path) {
        if let Some(file) = self.package_files.remove(root) {
            self.notify::<PublishDiagnostics>(PublishDiagnosticsParams::new(
                file,
                Vec::new(),
                None,
            ));
        }
        self.workspace.forget(root);
    }

    fn ask_to_pull_again(&mut self) {
        self.sent_requests += 1;
        let id = RequestId::from(self.sent_requests);
        let method = WorkspaceDiagnosticRefresh::METHOD.to_owned();
        self.send(Request::new(id, method, ()).into());
    }

    /// Sends the issues of the open document `key`.
    fn publish(&mut self, key: &str) -> Result<()> {
        let document = self
            .documents
            .get_mut(key)
            .context("the document is not open")?;
        let (document, linted) = lint(&mut self.workspace, document)?;
        let params = PublishDiagnosticsParams::new(
            document.uri.clone(),
            diagnostics(document, &linted.found, self.client.encoding),
            document.version,
        );
        self.notify::<PublishDiagnostics>(params);
        Ok(())
    }

    /// Sends the warnings about packages that the venv of the project in `root` lacks. They
    /// are about the project and not about a document, so they go on the first line of the
    /// project's `pyproject.toml`, which is the file `xpd check` names for them. The client
    /// never asks for that file, so they are pushed to every client.
    fn publish_package_issues(&mut self, root: &Path) -> Result<()> {
        let issues = &self.workspace.project(root)?.package_issues;
        // Without a `pyproject.toml`, `xpd check` names the venv's folder, and an editor
        // cannot show an issue on a folder.
        let file = issues
            .first()
            .map(|issue| root.join(&issue.file))
            .filter(|file| file.is_file())
            .and_then(|file| uri::from_path(&file));
        let top = lsp_types::Range::default();
        let diagnostics: Vec<Diagnostic> = file
            .iter()
            .flat_map(|_| issues.iter().map(|issue| diagnostic(issue, top)))
            .collect();
        let before = match &file {
            Some(file) => self.package_files.insert(root.to_path_buf(), file.clone()),
            None => self.package_files.remove(root),
        };
        if let Some(before) = before.filter(|before| Some(before) != file.as_ref()) {
            self.notify::<PublishDiagnostics>(PublishDiagnosticsParams::new(
                before,
                Vec::new(),
                None,
            ));
        }
        if let Some(file) = file {
            self.notify::<PublishDiagnostics>(PublishDiagnosticsParams::new(
                file,
                diagnostics,
                None,
            ));
        }
        Ok(())
    }

    /// `textDocument/diagnostic`. A document the client has not opened is read from disk.
    fn pull_diagnostics(
        &mut self,
        params: DocumentDiagnosticParams,
    ) -> Result<DocumentDiagnosticReport> {
        let uri = params.text_document.uri;
        let mut on_disk;
        let document = if let Some(document) = self.documents.get_mut(uri.as_str()) {
            Some(document)
        } else {
            on_disk = uri::to_path(&uri)
                .and_then(|path| fs::read_to_string(path).ok())
                .map(|text| Document {
                    place: self.workspace.place(&uri),
                    uri,
                    version: None,
                    text,
                    linted: None,
                });
            on_disk.as_mut()
        };
        let items = match document {
            Some(document) => {
                let (document, linted) = lint(&mut self.workspace, document)?;
                diagnostics(document, &linted.found, self.client.encoding)
            }
            None => Vec::new(),
        };
        Ok(DocumentDiagnosticReport::Full(
            RelatedFullDocumentDiagnosticReport {
                related_documents: None,
                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                    result_id: None,
                    items,
                },
            },
        ))
    }

    /// The documentation of the rules that report the place under the cursor.
    fn hover(&mut self, params: HoverParams) -> Result<Option<Hover>> {
        let at = params.text_document_position_params;
        let Some(document) = self.documents.get_mut(at.text_document.uri.as_str()) else {
            return Ok(None);
        };
        let (document, linted) = lint(&mut self.workspace, document)?;
        let lines = Lines::new(&document.text, self.client.encoding);
        let offset = lines.offset(at.position);
        let mut sections: Vec<String> = Vec::new();
        let mut range = None;
        for found in &linted.found {
            let marked = marked(found, &lines);
            // An empty mark, as for a rule about the whole file, is under a cursor at it.
            if !(marked.contains(&offset) || marked == (offset..offset)) {
                continue;
            }
            let issue = &found.issue;
            let Some(doc) = linted
                .packs
                .iter()
                .find(|pack| pack.module == issue.package)
                .and_then(|pack| pack.doc(&issue.rule))
            else {
                continue;
            };
            let link = format!(
                "[{}/{}]({})",
                issue.package,
                issue.rule,
                catalog::url(&issue.rule)
            );
            let section = format!("{}\n\n{link}", doc.trim_end());
            // Two matches of one rule at the cursor share one section.
            if !sections.contains(&section) {
                sections.push(section);
            }
            range.get_or_insert_with(|| lsp_range(&lines, &marked));
        }
        if sections.is_empty() {
            return Ok(None);
        }
        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: match self.client.hover_format {
                    HoverFormat::Markdown => MarkupKind::Markdown,
                    HoverFormat::PlainText => MarkupKind::PlainText,
                },
                value: sections.join("\n\n---\n\n"),
            }),
            range,
        }))
    }

    /// One quick fix for each fixable issue in the asked range, and one action that applies
    /// every fix of the document as `xpd check --fix` does.
    fn code_actions(&mut self, params: CodeActionParams) -> Result<Vec<CodeActionOrCommand>> {
        let CodeActionParams {
            text_document,
            range,
            context,
            ..
        } = params;
        let Some(document) = self.documents.get_mut(text_document.uri.as_str()) else {
            return Ok(Vec::new());
        };
        let only = context.only.as_deref();
        let form = self.client.edit_form;
        let (document, linted) = lint(&mut self.workspace, document)?;
        let lines = Lines::new(&document.text, self.client.encoding);
        let asked = lines.offset(range.start)..lines.offset(range.end);
        let mut actions = Vec::new();
        if wants(only, &CodeActionKind::QUICKFIX) {
            for found in &linted.found {
                let marked = marked(found, &lines);
                let (Some(level), Some(fix)) = (found.issue.fix, &found.fix) else {
                    continue;
                };
                // A cursor at either end of the mark counts as on it.
                if marked.start > asked.end || asked.start > marked.end {
                    continue;
                }
                let issue = &found.issue;
                let name = format!("{}/{}", issue.package, issue.rule);
                actions.push(CodeAction {
                    title: match level {
                        Applicability::Safe => format!("xpd: fix {name}"),
                        Applicability::Unsafe => {
                            format!("xpd: fix {name} (unsafe fix, may change behavior)")
                        }
                    },
                    kind: Some(CodeActionKind::QUICKFIX),
                    diagnostics: Some(vec![diagnostic(issue, lsp_range(&lines, &marked))]),
                    edit: Some(edit(form, document, &lines, fix)),
                    // The editor applies a preferred fix on its "auto fix" key, so only a fix
                    // that keeps what the code does is preferred.
                    is_preferred: Some(level == Applicability::Safe),
                    ..CodeAction::default()
                });
            }
        }
        let allowed = self.settings.allowed();
        // Without an issue that has a fix of the allowed level, the pass below changes nothing.
        let fixable = |found: &Found| found.issue.fix.is_some_and(|level| level <= allowed);
        if wants(only, &FIX_ALL) && linted.found.iter().any(fixable) {
            let disk = document.place.disk();
            let file = scan::File {
                path: &document.place.path,
                disk: Some(&disk),
            };
            let (fixed, count) =
                scan::fix_source(file, document.text.clone(), &linted.packs, allowed);
            if count > 0 {
                let change = changed_part(&document.text, &fixed);
                actions.push(CodeAction {
                    title: match allowed {
                        Applicability::Safe => "xpd: apply every safe fix".to_owned(),
                        Applicability::Unsafe => {
                            "xpd: apply every fix, also unsafe ones that may change behavior"
                                .to_owned()
                        }
                    },
                    kind: Some(FIX_ALL),
                    edit: Some(edit(form, document, &lines, &change)),
                    ..CodeAction::default()
                });
            }
        }
        Ok(actions.into_iter().map(CodeActionOrCommand::from).collect())
    }
}

/// An edit that writes `change` to `document`.
fn edit(
    form: EditForm,
    document: &Document,
    lines: &Lines<'_>,
    change: &Replacement,
) -> WorkspaceEdit {
    let edit = TextEdit {
        range: lsp_range(lines, &change.range),
        new_text: change.text.clone(),
    };
    let uri = document.uri.clone();
    match form {
        EditForm::Versioned => WorkspaceEdit {
            document_changes: Some(DocumentChanges::Edits(vec![TextDocumentEdit {
                text_document: OptionalVersionedTextDocumentIdentifier {
                    uri,
                    version: document.version,
                },
                edits: vec![OneOf::Left(edit)],
            }])),
            ..WorkspaceEdit::default()
        },
        EditForm::Plain => WorkspaceEdit {
            changes: Some(HashMap::from([(uri, vec![edit])])),
            ..WorkspaceEdit::default()
        },
    }
}
