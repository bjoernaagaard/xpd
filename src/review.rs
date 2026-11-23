//! Reviews proposed rule fixes before writing the accepted edits.

use std::fs;
use std::io::{self, IsTerminal as _, Write as _};
use std::path::PathBuf;

use anyhow::{Context as _, Result, ensure};

use crate::fix_safety::Applicability;
use crate::scan::EditBatch;
use crate::source::File;
use crate::workspace::Workspace;

pub(crate) fn apply(
    files: &[PathBuf],
    workspace: &Workspace,
    allowed: Applicability,
) -> Result<usize> {
    ensure!(
        io::stdin().is_terminal(),
        "--review needs an interactive terminal"
    );
    let mut fixed = 0;
    let mut accept_all = false;
    let mut quit = false;
    for path in files {
        if quit {
            break;
        }
        // The scan after the review names and counts a file that cannot be read.
        let disk = workspace.root.join(path);
        let Ok(source) = fs::read_to_string(&disk) else {
            continue;
        };
        let analysis = workspace.analyze(
            File {
                path,
                disk: Some(&disk),
            },
            &source,
        );
        let mut accepted = EditBatch::default();
        for (found, edit) in analysis.proposals(allowed) {
            if !accepted.fits(edit) {
                continue;
            }
            if !accept_all {
                let mut proposed = source.clone();
                proposed.replace_range(edit.range.clone(), &edit.text);
                let name = path.to_string_lossy();
                eprintln!("{}", found.issue().line(allowed));
                eprint!(
                    "{}",
                    similar::TextDiff::from_lines(&source, &proposed)
                        .unified_diff()
                        .header(&name, &name)
                );
                match answer()? {
                    Answer::No => continue,
                    Answer::Quit => {
                        quit = true;
                        break;
                    }
                    Answer::All => accept_all = true,
                    Answer::Yes => {}
                }
            }
            accepted.push(edit);
        }
        if accepted.is_empty() {
            continue;
        }
        ensure!(
            fs::read_to_string(&disk)? == source,
            "{} changed during review; no reviewed edits were written to it",
            path.display()
        );
        let rewritten = accepted.apply(&source);
        ensure!(
            crate::scan::accepts_rewrite(path, &source, &rewritten),
            "{} accepted fixes combine into invalid syntax; no reviewed edits were written to it",
            path.display()
        );
        fs::write(&disk, rewritten).with_context(|| format!("cannot write {}", path.display()))?;
        fixed += accepted.len();
    }
    Ok(fixed)
}

enum Answer {
    Yes,
    No,
    All,
    Quit,
}

fn answer() -> Result<Answer> {
    loop {
        eprint!("Apply this fix? [y]es, [n]o, [a]ll, [q]uit: ");
        io::stderr().flush()?;
        let mut answer = String::new();
        ensure!(
            io::stdin().read_line(&mut answer)? > 0,
            "input ended during fix review"
        );
        match answer.trim().to_ascii_lowercase().as_str() {
            "y" | "yes" => return Ok(Answer::Yes),
            "" | "n" | "no" => return Ok(Answer::No),
            "a" | "all" => return Ok(Answer::All),
            "q" | "quit" => return Ok(Answer::Quit),
            _ => eprintln!("Enter y, n, a or q."),
        }
    }
}
