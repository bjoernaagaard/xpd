//! `xpd doctor` reports settings worth changing in Ruff, ty and pytest. Each suggestion gives
//! the file, the reason, and any evidence that makes the advice relevant.
//!
//! The decisions in `doctor/decisions.jsonl` cover Ruff and ty's pinned releases and the pytest
//! settings named in the testing policy. Missing tools leave their advice unverified locally.

mod advice;
pub mod catalog;
mod evidence;
mod placement;
mod probe;
mod pytest;
mod ruff;
mod ty;

use std::path::PathBuf;

use anyhow::Result;

use crate::output::Format;

pub use evidence::evidence_rule_ids;

/// Prints the advice for the project in the working directory.
pub(crate) fn run(format: Format) -> Result<()> {
    let root = std::env::current_dir()?;
    let catalog = catalog::Catalog::packaged()?;
    let mut files: Vec<PathBuf> = crate::source::python_files(std::slice::from_ref(&root))?;
    files.sort();
    let evidence = evidence::find(&files)?;
    let ruff = ruff::analyze(
        &root,
        &files,
        &catalog,
        probe::Checker::find(catalog::Tool::Ruff, &root),
    );
    let ty = ty::analyze(
        &root,
        &catalog,
        probe::Checker::find(catalog::Tool::Ty, &root),
    );
    let report = advice::evaluate(&root, &files, &catalog, &evidence, &ruff, &ty);
    match format {
        Format::Json => println!("{}", report.to_json()),
        Format::Text => print!("{}", report.to_text()),
    }
    Ok(())
}
