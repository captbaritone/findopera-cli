//! `findopera schema` — machine-readable introspection of the command tree.
//!
//! Principle 7 of the agent CLI guide: let a caller query the schema on demand
//! rather than guessing flag names. Built by walking clap's own command tree,
//! so it cannot drift from the real parser.

use crate::model::Recording;
use crate::output::print_json;
use crate::output::{exit, Failure};
use clap::CommandFactory;
use serde_json::{json, Value};

fn describe(cmd: &clap::Command) -> Value {
    let flags: Vec<Value> = cmd
        .get_arguments()
        .filter(|a| !a.is_hide_set())
        .map(|a| {
            let possible: Vec<String> = a
                .get_possible_values()
                .iter()
                .map(|p| p.get_name().to_string())
                .collect();
            let mut v = json!({
                "name": a.get_id().to_string(),
                "long": a.get_long().map(|l| format!("--{l}")),
                "short": a.get_short().map(|s| format!("-{s}")),
                "required": a.is_required_set(),
                "repeatable": matches!(
                    a.get_action(),
                    clap::ArgAction::Append | clap::ArgAction::Count
                ),
                "takesValue": a.get_num_args().is_none_or(|n| n.takes_values()),
                "help": a.get_help().map(|h| h.to_string()),
            });
            if !possible.is_empty() {
                v["values"] = json!(possible);
            }
            if let Some(d) = a.get_default_values().first() {
                v["default"] = json!(d.to_string_lossy());
            }
            v
        })
        .collect();

    let subcommands: Vec<Value> = cmd
        .get_subcommands()
        .filter(|s| !s.is_hide_set())
        .map(describe)
        .collect();

    let mut v = json!({
        "name": cmd.get_name(),
        "about": cmd.get_about().map(|a| a.to_string()),
        "flags": flags,
    });
    if !subcommands.is_empty() {
        v["subcommands"] = json!(subcommands);
    }
    v
}

pub fn print_schema(all: &bool, path: &[String]) -> i32 {
    // `build` populates the auto-generated args (--help, --version) and
    // propagates global flags down, so the dump matches what the parser
    // actually accepts.
    let mut root = crate::Cli::command();
    root.build();
    let root = root;

    if path.is_empty() || *all {
        let mut doc = describe(&root);
        doc["exitCodes"] = json!({
            "0": "success",
            "1": "general error",
            "2": "invalid arguments or template",
            "3": "recording not found in the database",
            "6": "API unreachable or errored (retryable)",
        });
        doc["templateFields"] = json!(Recording::FIELDS
            .iter()
            .map(|f| json!({ "field": f.path, "description": f.description }))
            .collect::<Vec<_>>());
        print_json(&doc);
        return exit::OK;
    }

    // Walk down to the requested subcommand.
    let mut current = &root;
    for segment in path {
        match current.get_subcommands().find(|s| s.get_name() == segment) {
            Some(next) => current = next,
            None => {
                let available: Vec<String> = current
                    .get_subcommands()
                    .map(|s| s.get_name().to_string())
                    .collect();
                return Failure::new(
                    "unknown_command",
                    format!("`{segment}` is not a findopera command"),
                )
                .input(path.join(" "))
                .details(available)
                .suggest("Run `findopera schema --all` to see the whole tree.")
                .emit(exit::USAGE);
            }
        }
    }
    print_json(&describe(current));
    exit::OK
}
