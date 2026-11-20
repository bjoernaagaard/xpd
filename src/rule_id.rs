use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};

const LOCAL_PACKAGE: &str = "local";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Document {
    path: PathBuf,
    number: usize,
}

/// The name of a rule, optionally tied to a particular document that implements it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct RuleId {
    pack: Option<String>,
    name: String,
    document: Option<Document>,
}

impl RuleId {
    pub(crate) fn new(pack: Option<String>, name: String) -> Self {
        Self {
            pack,
            name,
            document: None,
        }
    }

    pub(crate) fn qualified(pack: &str, name: &str) -> Self {
        Self::new(Some(pack.to_owned()), name.to_owned())
    }

    pub(crate) fn parse(text: &str) -> Self {
        match text.split_once('/') {
            Some((pack, name)) => Self::qualified(pack, name),
            None => Self::new(None, text.to_owned()),
        }
    }

    pub(crate) fn declared(text: &str, pack: Option<&str>) -> Self {
        let mut id = Self::parse(text);
        if id.pack.is_none() {
            id.pack = Some(pack.unwrap_or(LOCAL_PACKAGE).to_owned());
        }
        id
    }

    pub(crate) fn reported(
        text: &str,
        pack: Option<&str>,
        source: PathBuf,
        number: usize,
    ) -> Result<Self> {
        let mut id = Self::declared(text, pack);
        if let Some(pack) = pack {
            ensure!(
                id.package() == pack,
                "the rule is named {text}, and its file belongs to the pack {pack}"
            );
        }
        ensure!(
            !id.package().is_empty() && !id.name.is_empty(),
            "the rule needs a name, as <rule> or <package>/<rule>, and has {text:?}"
        );
        id.document = Some(Document {
            path: source,
            number,
        });
        Ok(id)
    }

    pub(crate) fn pack(&self) -> Option<&str> {
        self.pack.as_deref()
    }

    pub(crate) fn package(&self) -> &str {
        self.pack().unwrap_or(LOCAL_PACKAGE)
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn source(&self) -> Option<&Path> {
        self.document
            .as_ref()
            .map(|document| document.path.as_path())
    }

    pub(crate) fn matches_case(&self, declared: &Self, case: &str) -> bool {
        if self.name == case {
            return true;
        }
        let case = Self::parse(case);
        if case.pack.is_some() {
            (self.pack == case.pack && self.name == case.name) || *declared == case
        } else {
            declared.name.rsplit('/').next() == Some(case.name())
        }
    }

    pub(crate) fn document_name(
        id: &str,
        path: &Path,
        index: usize,
        count: usize,
    ) -> Result<String> {
        if !id.is_empty() {
            return Ok(id.to_owned());
        }
        let stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .context("a rule without an id needs a file name to take it from")?;
        Ok(if count > 1 {
            format!("{stem}-{index}")
        } else {
            stem.to_owned()
        })
    }
}

impl fmt::Display for RuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.pack {
            Some(pack) => write!(f, "{pack}/{}", self.name),
            None => f.write_str(&self.name),
        }
    }
}
