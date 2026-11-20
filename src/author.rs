//! `xpd new`: creates a rule project, and the rules, cases and utilities in it. Every file it
//! writes is checked first, as ast-grep and xpd will read it, and no file is overwritten.

mod document;
mod project;

use std::fmt;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::LazyLock;

use anyhow::{Context, Result, bail};
use ast_grep_language::SupportLang;
use clap::builder::PossibleValue;
use clap::{Subcommand, ValueEnum};
use inquire::validator::Validation;
use serde::Serialize;

use crate::fix_safety::Applicability;
use crate::output::Format;
use crate::rule_id::RuleId;
use crate::severity::Severity;
use crate::version::Version;
use document::{Name, RuleDoc};
use project::{Changes, Gate, Pack, Project};

/// Create a rule project, or a rule, test case or utility in one.
///
/// Without an item it asks what to create. Without a terminal, or with --yes, it never asks, and
/// it fails when a required flag is missing.
#[derive(Debug, clap::Args)]
#[command(after_long_help = "Examples:
  xpd new
  xpd new project --pack team
  xpd new rule team/no-iterrows --pattern '$DF.iterrows()' --message 'Loop over columns instead.' --yes
  xpd new rule utc-now --language python --pattern 'datetime.utcnow()' \\
    --fix 'datetime.now(UTC)' --fix-safety unsafe --message 'utcnow is naive.' --with-test --yes
  xpd new test team/no-iterrows --yes
  xpd new util new-figure --language python --pattern 'go.Figure($$$)' --yes")]
pub(crate) struct NewArgs {
    #[command(subcommand)]
    item: Option<Item>,
    #[command(flatten)]
    options: Options,
}

#[derive(Debug, clap::Args)]
struct Options {
    /// The project folder, which holds sgconfig.yml. Nothing is written outside it. Default:
    /// the nearest folder at or above the current one with sgconfig.yml, or the current folder
    /// for a new project.
    #[arg(long, global = true, value_name = "DIR")]
    directory: Option<PathBuf>,
    /// Never ask. Fail when a required flag is missing.
    #[arg(long, short, global = true)]
    yes: bool,
    /// Print the result as text, or as JSON with `createdFiles` and `modifiedFiles`.
    #[arg(long, value_enum, global = true, default_value = "text")]
    format: Format,
}

#[derive(Debug, Subcommand)]
enum Item {
    /// Create sgconfig.yml, xpd.yml and the rules, rule-tests and utils folders.
    ///
    /// Each --pack declares a rule pack in xpd.yml, with its rules in rules/<module>. A pack's
    /// rules run on the Python files that import the module, or on every file with
    /// --every-file. The paths are relative, so the project can be moved and shared.
    Project(ProjectArgs),
    /// Create a rule from a pattern, with the Markdown that explains it beside the rule file.
    ///
    /// A rule in a pack is a Python rule with the id <pack>/<name>, in the pack's folder. A rule
    /// without a pack is a structural rule for any language, in the first rules folder. The
    /// rule is checked as `xpd check` and `ast-grep scan` read it before it is written.
    #[command(after_long_help = "Examples:
  xpd new rule team/no-iterrows --pattern '$DF.iterrows()' --message 'Loop over columns.' --yes
  xpd new rule old-api --pack team --pattern 'lib.old($X)' --fix 'lib.new($X)' \\
    --fix-safety safe --until 2.0 --message 'old was removed in 2.0.' --yes
  xpd new rule unwrap --language rust --pattern '$X.unwrap()' --message 'Handle the error.' --yes")]
    Rule(Box<RuleArgs>),
    /// Create an empty test case for a rule, with no valid or invalid code yet.
    ///
    /// The id is a rule's id, or the public id that several variants of a rule share.
    Test(TestArgs),
    /// Create a global utility rule, which any rule can use with `matches: <name>`.
    Util(UtilArgs),
}

#[derive(Debug, clap::Args)]
struct ProjectArgs {
    /// A rule pack to declare, named after the module its rules are about.
    #[arg(long = "pack", value_name = "MODULE", value_parser = document::module)]
    packs: Vec<String>,
    /// Run the new packs' rules on every Python file, not only on files that import the module.
    #[arg(long)]
    every_file: bool,
}

#[derive(Debug, clap::Args)]
struct RuleArgs {
    /// The rule's name, or <pack>/<name>.
    name: Option<String>,
    /// The pack the rule belongs to. A pack that xpd.yml lacks is added to it.
    #[arg(long, value_name = "MODULE", value_parser = document::module)]
    pack: Option<String>,
    /// Run a pack this command adds on every Python file.
    #[arg(long)]
    every_file: bool,
    /// The language of a structural rule. A pack's rules are Python.
    #[arg(long, short, visible_alias = "lang")]
    language: Option<Lang>,
    /// The code to find, with $X for any one node and $$$X for any number of them.
    #[arg(long, short)]
    pattern: Option<String>,
    /// What the issue says. It may name the pattern's variables, as $X.
    #[arg(long, short)]
    message: Option<String>,
    /// Default: warning.
    #[arg(long, value_enum)]
    severity: Option<Severity>,
    /// A longer explanation, for the rule's note and its Markdown.
    #[arg(long)]
    note: Option<String>,
    /// The code that replaces a match, with the pattern's variables.
    #[arg(long)]
    fix: Option<String>,
    /// Whether the fix keeps the code's behavior. `xpd check --fix` applies safe fixes, and
    /// --unsafe-fixes adds the unsafe ones. Required with --fix.
    #[arg(long, value_enum, requires = "fix")]
    fix_safety: Option<Applicability>,
    /// The first version of the pack's library the rule applies to.
    #[arg(long, value_name = "VERSION")]
    since: Option<String>,
    /// The first version of the pack's library the rule no longer applies to.
    #[arg(long, value_name = "VERSION")]
    until: Option<String>,
    /// Keep a match only when the variable holds a value of the rule's shape, as
    /// VAR='{kind: call}'.
    #[arg(long, value_name = "VAR=RULE")]
    holds: Vec<String>,
    /// Drop a match when the variable holds a value of the rule's shape.
    #[arg(long, value_name = "VAR=RULE")]
    unless_holds: Vec<String>,
    /// Silence the rule in a file that imports a module with a module-level statement of the
    /// rule's shape.
    #[arg(long, value_name = "RULE")]
    unless_imported: Option<String>,
    /// The id that issues show, for one of several variants of a rule with their own ids,
    /// such as two rules for the releases before and after a version.
    #[arg(long, value_name = "ID")]
    public_id: Option<String>,
    /// Also create an empty test case for the rule.
    #[arg(long)]
    with_test: bool,
}

#[derive(Debug, clap::Args)]
struct TestArgs {
    /// The rule's id or public id.
    id: Option<String>,
}

#[derive(Debug, clap::Args)]
struct UtilArgs {
    /// The utility's name, which rules use as `matches: <name>`.
    name: Option<String>,
    /// The language of the code the utility matches.
    #[arg(long, short, visible_alias = "lang")]
    language: Option<Lang>,
    /// The code the utility matches.
    #[arg(long, short)]
    pattern: Option<String>,
}

/// A language that ast-grep parses, named on the command line by any of its aliases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Lang(SupportLang);

static LANGS: LazyLock<Vec<Lang>> =
    LazyLock::new(|| SupportLang::all_langs().iter().copied().map(Lang).collect());

impl Lang {
    const PYTHON: Self = Self(SupportLang::Python);

    /// The alias that matches the language's own name, such as `typescript`, which ast-grep
    /// reads in a rule's `language:`.
    fn name(self) -> &'static str {
        let own = self.0.to_string();
        let aliases = self.0.aliases();
        aliases
            .iter()
            .find(|alias| alias.eq_ignore_ascii_case(&own))
            .or(aliases.first())
            .expect("every language has an alias")
    }
}

impl ValueEnum for Lang {
    fn value_variants<'a>() -> &'a [Self] {
        &LANGS
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        let name = self.name();
        let others = self.0.aliases().iter().filter(|alias| **alias != name);
        Some(PossibleValue::new(name).aliases(others.copied()))
    }
}

impl fmt::Display for Lang {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// What one `xpd new` made.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Outcome {
    item: &'static str,
    /// The rule's, case's or utility's id. A project has none.
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    created_files: Vec<PathBuf>,
    modified_files: Vec<PathBuf>,
}

impl Outcome {
    fn of(item: &'static str, id: Option<String>, changes: Changes) -> Result<Self> {
        let (created_files, modified_files) = changes.apply()?;
        Ok(Self {
            item,
            id,
            created_files,
            modified_files,
        })
    }
}

/// Whether `xpd new` may ask for a value that the flags leave out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Ask,
    FlagsOnly,
}

impl Mode {
    /// Asks only on a terminal, so that a run from a script or an agent fails instead of
    /// waiting for an answer.
    fn detect(options: &Options) -> Self {
        if !options.yes && io::stdin().is_terminal() && io::stderr().is_terminal() {
            Self::Ask
        } else {
            Self::FlagsOnly
        }
    }

    /// Fails, in a run that may not ask, naming each flag whose absence is `true`.
    fn require(self, flags: &[(bool, &str)]) -> Result<()> {
        let missing: Vec<&str> = flags
            .iter()
            .filter(|(absent, _)| *absent)
            .map(|(_, flag)| *flag)
            .collect();
        if self == Self::FlagsOnly && !missing.is_empty() {
            bail!(
                "missing {}. Pass them, or run on a terminal without --yes to be asked",
                missing.join(", ")
            );
        }
        Ok(())
    }

    /// The flag's value, or the answer to the prompt.
    fn or_ask<T>(self, value: Option<T>, flag: &str, ask: impl FnOnce() -> Result<T>) -> Result<T> {
        match (value, self) {
            (Some(value), _) => Ok(value),
            (None, Self::Ask) => ask(),
            (None, Self::FlagsOnly) => bail!("missing {flag}"),
        }
    }

    /// The flag's value, or the answer to the prompt for an optional value.
    fn or_maybe_ask<T>(
        self,
        value: Option<T>,
        ask: impl FnOnce() -> Result<Option<T>>,
    ) -> Result<Option<T>> {
        match (value, self) {
            (Some(value), _) => Ok(Some(value)),
            (None, Self::Ask) => ask(),
            (None, Self::FlagsOnly) => Ok(None),
        }
    }
}

/// Runs `xpd new`.
pub(crate) fn run(args: NewArgs) -> Result<ExitCode> {
    let NewArgs { item, options } = args;
    let mode = Mode::detect(&options);
    let item = match item {
        Some(item) => item,
        None if mode == Mode::Ask => choose_item()?,
        None => bail!("name what to create: xpd new project, rule, test or util"),
    };
    let directory = options.directory.as_deref();
    let outcome = match item {
        Item::Project(args) => new_project(args, directory, mode)?,
        Item::Rule(args) => new_rule(*args, &find_project(directory, mode)?, mode)?,
        Item::Test(args) => new_test(args, &find_project(directory, mode)?, mode)?,
        Item::Util(args) => new_util(args, &find_project(directory, mode)?, mode)?,
    };
    match options.format {
        Format::Json => println!("{}", serde_json::to_string(&outcome)?),
        Format::Text => {
            let id = outcome.id.as_deref().map(|id| format!(" {id}"));
            println!("Created {}{}", outcome.item, id.unwrap_or_default());
            for path in &outcome.created_files {
                println!("  created  {}", path.display());
            }
            for path in &outcome.modified_files {
                println!("  modified {}", path.display());
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn choose_item() -> Result<Item> {
    let kinds = ["project", "rule", "test", "util"];
    let kind = ask_select("What do you want to create?", kinds.to_vec(), 1)?;
    Ok(match kind {
        "project" => Item::Project(ProjectArgs {
            packs: Vec::new(),
            every_file: false,
        }),
        "rule" => Item::Rule(Box::new(RuleArgs {
            name: None,
            pack: None,
            every_file: false,
            language: None,
            pattern: None,
            message: None,
            severity: None,
            note: None,
            fix: None,
            fix_safety: None,
            since: None,
            until: None,
            holds: Vec::new(),
            unless_holds: Vec::new(),
            unless_imported: None,
            public_id: None,
            with_test: false,
        })),
        "test" => Item::Test(TestArgs { id: None }),
        _ => Item::Util(UtilArgs {
            name: None,
            language: None,
            pattern: None,
        }),
    })
}

/// The folder `--directory` names, or the one the user picks, starting from `default`.
fn destination(directory: Option<&Path>, mode: Mode, default: &Path) -> Result<PathBuf> {
    let chosen = match (directory, mode) {
        (Some(directory), _) => directory.to_path_buf(),
        (None, Mode::FlagsOnly) => default.to_path_buf(),
        (None, Mode::Ask) => {
            let default = default.display().to_string();
            PathBuf::from(ask_text_default("Project folder:", &default)?)
        }
    };
    // `absolute` also drops the `.` of `--directory .`, so the paths that `new` prints hold none.
    std::path::absolute(&chosen).with_context(|| format!("cannot read {}", chosen.display()))
}

/// The project in `--directory`, which must hold its sgconfig.yml so that a caller can confine
/// `xpd new` to one folder. Without it, the project at or above the current folder.
fn find_project(directory: Option<&Path>, mode: Mode) -> Result<Project> {
    let cwd = std::env::current_dir().context("no current folder")?;
    let found = crate::project::nearest_config(&cwd).map_or(cwd, |config| {
        config.parent().map(Path::to_path_buf).unwrap_or_default()
    });
    Project::find(&destination(directory, mode, &found)?)
}

fn new_project(args: ProjectArgs, directory: Option<&Path>, mode: Mode) -> Result<Outcome> {
    let cwd = std::env::current_dir().context("no current folder")?;
    let dir = destination(directory, mode, &cwd)?;
    let mut packs = args.packs;
    let mut gate = Gate::from(args.every_file);
    if mode == Mode::Ask && packs.is_empty() {
        let module = ask_optional(
            "First rule pack, named after its module (empty for none):",
            |text| document::module(text).map(drop),
        )?;
        if let Some(module) = module {
            gate = ask_gate()?;
            packs.push(module);
        }
    }
    let packs: Vec<Pack> = packs
        .into_iter()
        .map(|module| Pack::new(module, gate))
        .collect();
    Outcome::of("project", None, Project::create(&dir, &packs)?)
}

/// Splits `<pack>/<name>` from the name and `--pack`, which must agree.
fn split_rule_name(
    name: Option<String>,
    pack: Option<String>,
) -> Result<(Option<String>, Option<String>)> {
    let Some(name) = name else {
        return Ok((None, pack));
    };
    match name.split_once('/') {
        None => Ok((Some(name), pack)),
        Some((prefix, rest)) => {
            let prefix = document::module(prefix)?;
            if let Some(pack) = pack.filter(|pack| *pack != prefix) {
                bail!("the name says pack {prefix}, and --pack says {pack}");
            }
            Ok((Some(rest.to_owned()), Some(prefix)))
        }
    }
}

fn new_rule(args: RuleArgs, project: &Project, mode: Mode) -> Result<Outcome> {
    let (name, pack) = split_rule_name(args.name, args.pack)?;
    mode.require(&[
        (name.is_none(), "NAME"),
        (args.pattern.is_none(), "--pattern"),
        (args.message.is_none(), "--message"),
        (
            pack.is_none() && args.language.is_none(),
            "--language or --pack",
        ),
        (
            args.fix.is_some() && args.fix_safety.is_none(),
            "--fix-safety",
        ),
    ])?;
    let name: Name = mode
        .or_ask(name, "NAME", || {
            ask_text("Rule name:", |text| text.parse::<Name>().map(drop))
        })?
        .parse()?;
    let pack = match (pack, mode) {
        (Some(module), _) => Some(project.pack_or_new(&module, Gate::from(args.every_file))),
        (None, Mode::FlagsOnly) => None,
        (None, Mode::Ask) => choose_pack(project)?,
    };
    let language = match (&pack, args.language) {
        (Some(_), Some(language)) if language != Lang::PYTHON => {
            bail!("a pack's rules are Python. Leave out --pack for a {language} rule")
        }
        (Some(_), _) => Lang::PYTHON,
        (None, language) => mode.or_ask(language, "--language", ask_language)?,
    };
    let id = RuleId::new(
        pack.as_ref().map(|pack| pack.module.clone()),
        name.to_string(),
    );
    project.check_new_rule_id(&id)?;
    let pattern = mode.or_ask(args.pattern, "--pattern", || ask_pattern(language))?;
    let message = mode.or_ask(args.message, "--message", || {
        ask_text("Message:", |text| document::non_empty(text, "the message"))
    })?;
    let severity = match (args.severity, mode) {
        (Some(severity), _) => severity,
        (None, Mode::FlagsOnly) => Severity::Warning,
        (None, Mode::Ask) => ask_select("Severity:", Severity::value_variants().to_vec(), 1)?,
    };
    let fix = choose_fix(args.fix, args.fix_safety, mode)?;
    let (since, until) = match pack {
        Some(_) => (
            mode.or_maybe_ask(args.since, || {
                ask_version("First library version it applies to (empty for all):")
            })?,
            mode.or_maybe_ask(args.until, || {
                ask_version("First library version it no longer applies to (empty for none):")
            })?,
        ),
        None => (args.since, args.until),
    };
    let note = mode.or_maybe_ask(args.note, || {
        ask_optional("Note for the rule page (empty for none):", |_| Ok(()))
    })?;
    let with_test = args.with_test
        || (mode == Mode::Ask
            && project.has_tests()
            && ask_confirm("Also create an empty test case?")?);
    let doc = RuleDoc {
        id,
        language: language.0,
        language_name: language.name(),
        severity,
        message,
        note,
        pattern,
        fix,
        since,
        until,
        holds: document::conditions(&args.holds, "--holds")?,
        unless_holds: document::conditions(&args.unless_holds, "--unless-holds")?,
        unless_imported: (args.unless_imported.as_deref().map(document::yaml_rule)).transpose()?,
        public_id: args.public_id,
    };
    let mut changes = Changes::default();
    project.add_rule(&doc, pack.as_ref(), &mut changes)?;
    if with_test {
        project.add_test(&doc.id.to_string(), None, &mut changes)?;
    }
    Outcome::of("rule", Some(doc.id.to_string()), changes)
}

struct FixChoice(Applicability);

impl fmt::Display for FixChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let description = match self.0 {
            Applicability::Safe => "it keeps the code's behavior",
            Applicability::Unsafe => "it may change the behavior",
        };
        write!(f, "{}: {description}", self.0)
    }
}

/// The fix and how safe it is.
fn choose_fix(
    fix: Option<String>,
    safety: Option<Applicability>,
    mode: Mode,
) -> Result<Option<(String, Applicability)>> {
    let Some(fix) = mode.or_maybe_ask(fix, || ask_optional("Fix (empty for none):", |_| Ok(())))?
    else {
        return Ok(None);
    };
    let safety = mode.or_ask(safety, "--fix-safety", || {
        let choices = Applicability::value_variants()
            .iter()
            .copied()
            .map(FixChoice)
            .collect();
        ask_select("Does the fix keep the code's behavior?", choices, 1).map(|choice| choice.0)
    })?;
    Ok(Some((fix, safety)))
}

/// The pack of a new rule: a declared one, a new one, or none for a structural rule.
fn choose_pack(project: &Project) -> Result<Option<Pack>> {
    const NONE: &str = "no pack: a structural rule for any language";
    const NEW: &str = "a new pack";
    let declared = project.packs().iter().map(|pack| pack.module.clone());
    let options: Vec<String> = declared.chain([NONE, NEW].map(str::to_owned)).collect();
    Ok(match ask_select("Pack:", options, 0)?.as_str() {
        NONE => None,
        NEW => {
            let module = ask_text("Module the pack is about:", |text| {
                document::module(text).map(drop)
            })?;
            Some(project.pack_or_new(&module, ask_gate()?))
        }
        module => Some(project.pack_or_new(module, Gate::Import)),
    })
}

fn new_test(args: TestArgs, project: &Project, mode: Mode) -> Result<Outcome> {
    mode.require(&[(args.id.is_none(), "ID")])?;
    let ids = project.rule_ids()?;
    let id = mode.or_ask(args.id, "ID", || {
        if ids.is_empty() {
            bail!("the project has no rules yet. Create one with xpd new rule");
        }
        ask_select("Rule to test:", ids.clone(), 0)
    })?;
    if !ids.contains(&id) {
        bail!("no rule in the project has the id or public id {id}");
    }
    let dirs = project.test_dirs();
    let test_dir = match mode {
        Mode::Ask if dirs.len() > 1 => Some(ask_select("Test folder:", dirs, 0)?),
        _ => None,
    };
    let mut changes = Changes::default();
    project.add_test(&id, test_dir.as_deref(), &mut changes)?;
    Outcome::of("test", Some(id), changes)
}

fn new_util(args: UtilArgs, project: &Project, mode: Mode) -> Result<Outcome> {
    mode.require(&[
        (args.name.is_none(), "NAME"),
        (args.language.is_none(), "--language"),
        (args.pattern.is_none(), "--pattern"),
    ])?;
    let name: Name = mode
        .or_ask(args.name, "NAME", || {
            ask_text("Utility name:", |text| text.parse::<Name>().map(drop))
        })?
        .parse()?;
    let language = mode.or_ask(args.language, "--language", ask_language)?;
    let pattern = mode.or_ask(args.pattern, "--pattern", || ask_pattern(language))?;
    let dirs = project.util_dirs();
    let util_dir = match mode {
        Mode::Ask if dirs.len() > 1 => Some(ask_select("Utility folder:", dirs.to_vec(), 0)?),
        _ => None,
    };
    let mut changes = Changes::default();
    let util_dir = util_dir.as_deref();
    project.add_util(
        &name,
        language.0,
        language.name(),
        &pattern,
        util_dir,
        &mut changes,
    )?;
    Outcome::of("util", Some(name.to_string()), changes)
}

fn validator(
    check: impl Fn(&str) -> Result<()> + Clone + 'static,
) -> impl inquire::validator::StringValidator {
    move |text: &str| {
        Ok(match check(text) {
            Ok(()) => Validation::Valid,
            Err(error) => Validation::Invalid(format!("{error:#}").into()),
        })
    }
}

fn ask_text(prompt: &str, check: impl Fn(&str) -> Result<()> + Clone + 'static) -> Result<String> {
    Ok(inquire::Text::new(prompt)
        .with_validator(validator(check))
        .prompt()?)
}

fn ask_text_default(prompt: &str, default: &str) -> Result<String> {
    Ok(inquire::Text::new(prompt).with_default(default).prompt()?)
}

/// An answer that may be empty, which gives `None`.
fn ask_optional(
    prompt: &str,
    check: impl Fn(&str) -> Result<()> + Clone + 'static,
) -> Result<Option<String>> {
    let answer = inquire::Text::new(prompt)
        .with_validator(validator(move |text: &str| {
            if text.is_empty() { Ok(()) } else { check(text) }
        }))
        .prompt()?;
    Ok(Some(answer).filter(|answer| !answer.is_empty()))
}

fn ask_select<T: fmt::Display>(prompt: &str, options: Vec<T>, start: usize) -> Result<T> {
    Ok(inquire::Select::new(prompt, options)
        .with_starting_cursor(start)
        .prompt()?)
}

fn ask_confirm(prompt: &str) -> Result<bool> {
    Ok(inquire::Confirm::new(prompt).with_default(true).prompt()?)
}

fn ask_language() -> Result<Lang> {
    let start = LANGS.iter().position(|lang| *lang == Lang::PYTHON);
    ask_select("Language:", LANGS.clone(), start.unwrap_or_default())
}

fn ask_pattern(language: Lang) -> Result<String> {
    ask_text("Pattern:", move |text| {
        document::check_pattern(text, language.0)
    })
}

fn ask_version(prompt: &str) -> Result<Option<String>> {
    ask_optional(prompt, |text| text.parse::<Version>().map(drop))
}

fn ask_gate() -> Result<Gate> {
    ask_select("Its rules run on:", vec![Gate::Import, Gate::EveryFile], 0)
}
