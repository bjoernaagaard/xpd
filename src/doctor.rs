//! `xpd doctor`: the Ruff and ty settings worth changing in this project, each with the file it
//! belongs in, the reason, and the evidence that makes a conditional one relevant.
//!
//! The decisions come from a review of every rule and setting of one Ruff and one ty release,
//! which `doctor/decisions.jsonl` holds. Ruff and ty are optional: doctor runs an installed one
//! to read the settings it resolves, and labels the advice for a missing one unverified locally.

mod advice;
pub(crate) mod catalog;
mod evidence;
mod probe;
mod ruff;
pub(crate) mod toml;
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
    let report = advice::evaluate(&root, &files, &catalog, &evidence, &ruff, &ty);
    match format {
        Format::Json => println!("{}", report.to_json()),
        Format::Text => print!("{}", report.to_text()),
    }
    Ok(())
}
