use std::path::PathBuf;

use anyhow::Result;

use crate::catalog;
use crate::fix_safety::Applicability;
use crate::report::Report;
use crate::workspace::Workspace;

/// Whether a lint run writes fixes, and up to which level.
#[derive(Debug, Clone, Copy)]
pub(crate) enum FixMode {
    Apply(Option<Applicability>),
    Review(Applicability),
}

impl Workspace {
    pub(crate) fn check(&self, paths: &[PathBuf], mode: FixMode) -> Result<Report> {
        let files = self.collect_files(paths)?;
        let (scanned, fixed) = match mode {
            FixMode::Apply(fix) => {
                let scanned = self.scan_files(&files, fix)?;
                let fixed = scanned.fixed;
                (scanned, fixed)
            }
            FixMode::Review(allowed) => {
                let fixed = crate::review::apply(&files, self, allowed)?;
                (self.scan_files(&files, None)?, fixed)
            }
        };
        let mut issues = scanned.issues;
        issues.extend(self.package_issues(&scanned.calls)?);
        for path in &scanned.pass_limit {
            eprintln!(
                "xpd: {}: stopped fixing after reaching the pass limit; some fixable issues may remain",
                path.display()
            );
        }
        for warning in self
            .catalog
            .project
            .iter()
            .flat_map(catalog::LocalProject::warnings)
        {
            eprintln!("xpd: {warning}");
        }
        Ok(Report {
            issues,
            fixed,
            skipped: scanned.skipped,
        })
    }
}
