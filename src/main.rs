//! xpd is an opinionated linter for Python library habits that give slow or wrong results, for
//! humans and coding agents. Each library gets one rule pack under `rules/`.

mod author;
mod cases;
mod catalog;
mod cli;
mod doctor;
mod embed;
mod graph;
mod imports;
mod index;
mod lsp;
mod mcp;
mod names;
mod nodes;
mod outline;
mod pack;
mod package_check;
mod query;
mod recipe;
mod report;
mod review;
mod rule;
mod scan;
mod search;
mod skills;
mod steps;
mod venv;
mod version;

use std::process::ExitCode;

fn main() -> ExitCode {
    cli::run()
}
