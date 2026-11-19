//! `xpd doctor` reports settings worth changing in Ruff, ty and pytest. Each suggestion gives
//! the file, the reason, and any evidence that makes the advice relevant.
//!
//! The decisions in `doctor/decisions.jsonl` cover Ruff and ty's pinned releases and the pytest
//! settings named in the testing policy. Missing tools leave their advice unverified locally.

mod advice;
pub(crate) mod catalog;
mod evidence;
mod probe;
mod pytest;
mod ruff;
mod toml;
mod ty;

use std::path::PathBuf;

use anyhow::Result;

use crate::cli::Format;

/// Prints the advice for the project in the working directory.
pub(crate) fn run(format: Format) -> Result<()> {
    let root = std::env::current_dir()?;
    let catalog = catalog::Catalog::packaged()?;
    let mut files: Vec<PathBuf> = crate::scan::python_files(std::slice::from_ref(&root))?;
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
    let mut report = advice::evaluate(&root, &files, &catalog, &evidence, &ruff, &ty);
    report.pytest(&root, files.len(), &catalog, &pytest::analyze(&root));
    match format {
        Format::Json => println!("{}", report.to_json()),
        Format::Text => print!("{}", report.to_text()),
    }
    Ok(())
}
