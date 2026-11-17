//! Reviews proposed rule fixes before writing the accepted edits.

use std::fs;
use std::io::{self, IsTerminal as _, Write as _};
use std::path::PathBuf;

use anyhow::{Context as _, Result, ensure};

use crate::pack::Pack;
use crate::rule::Applicability;
use crate::scan::{self, File, Replacement, Scanned};

pub(crate) fn scan(files: &[PathBuf], packs: &[Pack], allowed: Applicability) -> Result<Scanned> {
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
        let Ok(source) = fs::read_to_string(path) else {
            continue;
        };
        let mut proposals = scan::lint_source(File::on_disk(path), &source, packs);
        proposals.sort_by_key(|found| found.fix.as_ref().map(|edit| edit.range.start));
        let mut accepted: Vec<Replacement> = Vec::new();
        let mut end = 0;
        for found in proposals {
            if found.issue.fix.is_none_or(|fix| fix > allowed) {
                continue;
            }
            let Some(edit) = found.fix else { continue };
            if edit.range.start < end {
                continue;
            }
            if !accept_all {
                let mut proposed = source.clone();
                proposed.replace_range(edit.range.clone(), &edit.text);
                let name = path.to_string_lossy();
                eprintln!("{}", found.issue.line(allowed));
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
            end = edit.range.end;
            accepted.push(edit);
        }
        if accepted.is_empty() {
            continue;
        }
        ensure!(
            fs::read_to_string(path)? == source,
            "{} changed during review; no reviewed edits were written to it",
            path.display()
        );
        let mut updated = source;
        for edit in accepted.iter().rev() {
            updated.replace_range(edit.range.clone(), &edit.text);
        }
        fs::write(path, updated).with_context(|| format!("cannot write {}", path.display()))?;
        fixed += accepted.len();
    }
    let mut scanned = scan::scan(files, packs, None)?;
    scanned.fixed = fixed;
    Ok(scanned)
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
