//! The agent skills under `skills/`, embedded at compile time by build.rs.

use serde_json::{Value, json};

/// One skill folder, `skills/<name>/`.
#[derive(Debug)]
struct SkillSource {
    name: &'static str,
    files: &'static [SkillFile],
}

/// One file of a skill, with its path below the skill folder written with `/`.
#[derive(Debug)]
struct SkillFile {
    path: &'static str,
    text: &'static str,
}

/// Every skill under `skills/`, in name order.
const SKILLS: &[SkillSource] = include!(concat!(env!("OUT_DIR"), "/skills.rs"));

/// Every skill with the path and text of each of its files, for `xpd skill --json`.
pub(crate) fn to_json() -> Value {
    let skills: Vec<Value> = SKILLS
        .iter()
        .map(|skill| {
            let files: Vec<Value> = skill
                .files
                .iter()
                .map(|file| json!({"path": file.path, "text": file.text}))
                .collect();
            json!({"name": skill.name, "files": files})
        })
        .collect();
    Value::Array(skills)
}
