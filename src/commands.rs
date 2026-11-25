//! `xpd commands`: the command tree as JSON, which the documentation site turns into its command
//! reference. It reads the same clap definitions that parse the arguments, so the reference
//! lists what this build of xpd accepts.

use clap::{Arg, ArgAction, Command};
use serde_json::{Value, json};

/// Every visible command under `root`, depth first, each with its own arguments and the groups
/// of arguments of which it needs exactly one. Building the tree first fills in each subcommand's
/// usage line and full name, such as `xpd rules new rule`.
pub(crate) fn to_json(mut root: Command) -> Value {
    root.build();
    Value::Array(commands(&root))
}

fn commands(parent: &Command) -> Vec<Value> {
    parent
        .get_subcommands()
        .filter(|cmd| !cmd.is_hide_set() && cmd.get_name() != "help")
        .flat_map(|cmd| {
            let mut out = vec![command(cmd)];
            out.extend(commands(cmd));
            out
        })
        .collect()
}

fn command(cmd: &Command) -> Value {
    let text = |s: Option<&clap::builder::StyledStr>| s.map(ToString::to_string);
    let arguments: Vec<Value> = cmd
        .get_arguments()
        .filter(|arg| !arg.is_hide_set() && !is_builtin(arg))
        .map(|arg| argument(cmd, arg))
        .collect();
    let groups: Vec<Value> = cmd
        .get_groups()
        .filter(|group| group.is_required_set())
        .map(|group| {
            let members: Vec<String> = group.get_args().map(ToString::to_string).collect();
            json!({ "id": group.get_id().as_str(), "args": members })
        })
        .collect();
    let subcommands: Vec<&str> = cmd
        .get_subcommands()
        .filter(|sub| !sub.is_hide_set() && sub.get_name() != "help")
        .map(Command::get_name)
        .collect();
    json!({
        "name": cmd.get_bin_name().unwrap_or(cmd.get_name()),
        "aliases": cmd.get_visible_aliases().collect::<Vec<_>>(),
        "about": text(cmd.get_about()),
        "after_help": text(cmd.get_after_long_help().or(cmd.get_after_help())),
        "usage": cmd.clone().render_usage().to_string().trim_start_matches("Usage: ").to_owned(),
        "subcommands": subcommands,
        "subcommand_required": cmd.is_subcommand_required_set(),
        "arguments": arguments,
        "groups": groups,
    })
}

/// The help and version flags that clap adds to every command.
fn is_builtin(arg: &Arg) -> bool {
    matches!(
        arg.get_action(),
        ArgAction::Help | ArgAction::HelpShort | ArgAction::HelpLong | ArgAction::Version
    )
}

fn argument(cmd: &Command, arg: &Arg) -> Value {
    let text = |s: Option<&clap::builder::StyledStr>| s.map(ToString::to_string);
    let takes_value = arg.get_action().takes_values();
    let num_args = arg.get_num_args();
    let possible: Vec<Value> = arg
        .get_possible_values()
        .iter()
        .filter(|value| !value.is_hide_set())
        .map(|value| json!({ "name": value.get_name(), "help": text(value.get_help()) }))
        .collect();
    let conflicts: Vec<String> = cmd
        .get_arg_conflicts_with(arg)
        .iter()
        .filter(|other| !other.is_hide_set())
        .map(|other| flag_name(other))
        .collect();
    let defaults: Vec<String> = arg
        .get_default_values()
        .iter()
        .map(|value| value.to_string_lossy().into_owned())
        .collect();
    json!({
        "id": arg.get_id().as_str(),
        "long": arg.get_long(),
        "short": arg.get_short().map(String::from),
        "aliases": arg.get_visible_aliases().unwrap_or_default(),
        "positional": arg.is_positional(),
        "value_names": arg.get_value_names().map(|names| names.iter().map(ToString::to_string).collect::<Vec<_>>()),
        "takes_value": takes_value,
        "optional_value": takes_value && num_args.is_some_and(|n| n.min_values() == 0),
        "multiple": matches!(arg.get_action(), ArgAction::Append | ArgAction::Count)
            || num_args.is_some_and(|n| n.max_values() > 1),
        "required": arg.is_required_set(),
        "trailing": arg.is_trailing_var_arg_set() || arg.is_last_set(),
        "help": text(arg.get_help()),
        "long_help": text(arg.get_long_help()),
        "defaults": defaults,
        "possible_values": possible,
        "conflicts_with": conflicts,
    })
}

/// How a reader writes the argument: `--fix`, `-c` or `<PATHS>`.
fn flag_name(arg: &Arg) -> String {
    match (arg.get_long(), arg.get_short()) {
        (Some(long), _) => format!("--{long}"),
        (None, Some(short)) => format!("-{short}"),
        (None, None) => format!("<{}>", arg.get_id().as_str().to_uppercase()),
    }
}
