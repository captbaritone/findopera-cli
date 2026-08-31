//! `findopera` — render FindOpera recording metadata through a template.

mod api;
mod model;
mod output;
mod schema;
mod template;

use clap::{Args, Parser, Subcommand};
use output::{exit, Failure, Format};

const AFTER_HELP: &str = "\
Exit codes:
  0  success
  1  general error
  2  invalid arguments or template
  3  a recording id is not in the FindOpera database
  6  the FindOpera API was unreachable or errored (retryable)

Output is JSON when stdout is piped and plain text at a terminal; override
with --format.";

#[derive(Parser)]
#[command(
    name = "findopera",
    version,
    about = "Render FindOpera recording metadata through a template",
    long_about = "\
Render FindOpera recording metadata through a template.

Give it a recording id from findopera.com and a template, and it prints the
result. Intended as a naming primitive: what you do with the rendered string
is up to you.",
    after_help = AFTER_HELP,
    subcommand_required = true,
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Render one or more recordings through a template.
    #[command(long_about = "\
Render one or more recordings through a template.

Placeholders are {{field}}. Alternatives separated by | are tried left to
right, and a quoted literal serves as a last resort. A placeholder that
resolves to nothing with no fallback is an error, so absent data is never
silently dropped.

Examples:
  findopera render 10655 -t '{{composer.lastName}}/{{opera.title}}/{{year}}'
  findopera render 75 10655 -t '{{opera.title}} ({{year|\"n.d.\"}})'
  findopera render 10655 -t '{{opera.englishTitle|opera.title}}' --format json")]
    Render(RenderArgs),
    /// List the fields a template can interpolate, with descriptions.
    #[command(long_about = "\
List the fields a template can interpolate, with a one-line description of
each. Pass --example <ID> to see what each field actually renders to for a
real recording — the quickest way to find out whether a field is populated.

Examples:
  findopera fields
  findopera fields --example 10655
  findopera fields --format json | jq -r '.fields[].field'")]
    Fields(FieldsArgs),
    /// Print the command tree, flags, and template fields as JSON.
    #[command(after_help = "Examples:\n  \
        findopera schema --all\n  \
        findopera schema render")]
    Schema(SchemaArgs),
}

#[derive(Args)]
struct FormatArgs {
    /// Output format <optional, default: json when piped, text at a terminal>
    #[arg(long, value_enum, global = true)]
    format: Option<Format>,
}

#[derive(Args)]
struct ApiArgs {
    /// GraphQL endpoint <optional, default: https://findopera.com/api/graphql>
    #[arg(long, default_value = api::DEFAULT_ENDPOINT, hide_default_value = true, global = true)]
    endpoint: String,
    /// Per-request timeout in seconds <optional, default: 30>
    #[arg(long, default_value_t = 30, hide_default_value = true, global = true)]
    timeout: u64,
}

#[derive(Args)]
struct RenderArgs {
    /// FindOpera recording ids <required, one or more>
    #[arg(required = true, value_name = "ID")]
    ids: Vec<String>,
    /// Template <required>. `{{field}}`, `|`-separated fallbacks, quoted
    /// literal last: '{{opera.englishTitle|opera.title|"Untitled"}}'.
    /// Fields: `findopera fields`.
    #[arg(long, short = 't', value_name = "TEMPLATE")]
    template: String,
    #[command(flatten)]
    fmt: FormatArgs,
    #[command(flatten)]
    api: ApiArgs,
}

#[derive(Args)]
struct FieldsArgs {
    /// Show what each field renders to for this recording id <optional>
    #[arg(long, value_name = "ID")]
    example: Option<String>,
    #[command(flatten)]
    fmt: FormatArgs,
    #[command(flatten)]
    api: ApiArgs,
}

#[derive(Args)]
struct SchemaArgs {
    /// Print the whole command tree <optional>
    #[arg(long)]
    all: bool,
    /// Command to describe, e.g. `render` <optional>
    #[arg(value_name = "COMMAND")]
    path: Vec<String>,
}

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let cli = match Cli::try_parse() {
        Ok(c) => c,
        Err(e) => {
            // An explicit --help or --version is a successful request for
            // information. Everything else clap rejects — including a bare
            // invocation with no subcommand — is a usage error and exits 2.
            let is_help = matches!(
                e.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            );
            let _ = e.print();
            return if is_help { exit::OK } else { exit::USAGE };
        }
    };

    match cli.command {
        Command::Render(args) => cmd_render(args),
        Command::Fields(args) => cmd_fields(args),
        Command::Schema(args) => schema::print_schema(&args.all, &args.path),
    }
}

/// Reject anything that is not a bare recording number before spending a
/// request on it. Reports the failure and yields the exit code to return.
fn check_id(id: &str) -> Result<(), i32> {
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
        return Err(Failure::new(
            "invalid_recording_id",
            format!("`{id}` is not a numeric recording id"),
        )
        .input(id.to_string())
        .suggest("Ids are the number in https://findopera.com/recording/<id>.")
        .emit(exit::USAGE));
    }
    Ok(())
}

fn fetch(
    api: &ApiArgs,
    ids: &[String],
) -> Result<std::collections::BTreeMap<String, model::Recording>, i32> {
    let client = api::Client::new(api.endpoint.clone(), api.timeout);
    client.recordings(ids).map_err(|e| {
        Failure::new(e.code, e.to_string())
            .retryable(e.retryable)
            .suggest(if e.retryable {
                "The API may be temporarily unavailable; retry in a moment."
            } else {
                "Check --endpoint."
            })
            .emit(exit::API)
    })
}

fn cmd_render(args: RenderArgs) -> i32 {
    let format = Format::resolve(args.fmt.format);

    // Parse the template first: a bad template is the caller's mistake and
    // should not cost a network round trip.
    let tmpl = match template::Template::parse(&args.template) {
        Ok(t) => t,
        Err(e) => {
            let mut f = Failure::new(e.code, format!("invalid --template: {}", e.message))
                .input(args.template.clone())
                .details(e.underline(&args.template));
            if let Some(help) = &e.help {
                f = f.suggest(help.clone());
            }
            return f.emit(exit::USAGE);
        }
    };

    for id in &args.ids {
        if let Err(code) = check_id(id) {
            return code;
        }
    }

    let recordings = match fetch(&args.api, &args.ids) {
        Ok(r) => r,
        Err(code) => return code,
    };

    let mut rows = Vec::new();
    let mut problems = Vec::new();
    for id in &args.ids {
        let Some(rec) = recordings.get(id) else {
            problems.push(serde_json::json!({
                "id": id,
                "error": "recording_not_found",
                "message": format!("recording {id} is not in the FindOpera database"),
            }));
            continue;
        };
        match tmpl.render(rec) {
            Ok(segments) => rows.push(serde_json::json!({
                "id": id,
                "rendered": segments.join("/"),
                "segments": segments,
            })),
            Err(e) => problems.push(serde_json::json!({
                "id": id,
                "error": e.code(),
                "message": e.to_string(),
            })),
        }
    }

    match format {
        Format::Json => output::print_json(&serde_json::json!({
            "template": args.template,
            "results": rows,
            "problems": problems,
        })),
        Format::Ndjson => output::print_ndjson(&rows),
        Format::Text => {
            for row in &rows {
                println!("{}", row["rendered"].as_str().unwrap_or_default());
            }
            for p in &problems {
                eprintln!("{}", p["message"].as_str().unwrap_or_default());
            }
        }
    }

    if problems.is_empty() {
        exit::OK
    } else if problems.iter().all(|p| p["error"] == "recording_not_found") {
        exit::NOT_FOUND
    } else {
        exit::GENERAL
    }
}

fn cmd_fields(args: FieldsArgs) -> i32 {
    let format = Format::resolve(args.fmt.format);

    // With --example, fetch one recording so the caller can see which fields
    // are actually populated — the fastest way to learn that, say,
    // `opera.librettist` is empty for most entries.
    let example = match &args.example {
        None => None,
        Some(id) => {
            if let Err(code) = check_id(id) {
                return code;
            }
            let map = match fetch(&args.api, std::slice::from_ref(id)) {
                Ok(m) => m,
                Err(code) => return code,
            };
            match map.into_values().next() {
                Some(rec) => Some(rec),
                None => {
                    return Failure::new(
                        "recording_not_found",
                        format!("recording {id} is not in the FindOpera database"),
                    )
                    .input(id.clone())
                    .emit(exit::NOT_FOUND)
                }
            }
        }
    };

    let rows: Vec<serde_json::Value> = model::Recording::FIELDS
        .iter()
        .map(|f| {
            let mut row = serde_json::json!({
                "field": f.path,
                "description": f.description,
            });
            if let Some(rec) = &example {
                row["value"] = match rec.get(f.path) {
                    Ok(Some(v)) => serde_json::Value::String(v),
                    _ => serde_json::Value::Null,
                };
            }
            row
        })
        .collect();

    match format {
        Format::Json => output::print_json(&serde_json::json!({
            "fields": rows,
            "syntax": {
                "placeholder": "{{field}}",
                "fallback": "{{a|b}} tries a, then b",
                "literal": "{{a|\"Unknown\"}} falls back to a quoted literal",
            },
        })),
        Format::Ndjson => output::print_ndjson(&rows),
        Format::Text => {
            let width = model::Recording::FIELDS
                .iter()
                .map(|f| f.path.len())
                .max()
                .unwrap_or(28);
            for (f, row) in model::Recording::FIELDS.iter().zip(&rows) {
                if example.is_some() {
                    let v = row["value"].as_str().unwrap_or("—");
                    println!("{:<width$}  {}", f.path, v);
                } else {
                    println!("{:<width$}  {}", f.path, f.description);
                }
            }
            eprintln!(
                "\nUse as {{{{field}}}}. Fallbacks: {{{{opera.englishTitle|opera.title|\"Untitled\"}}}}"
            );
            if example.is_some() {
                eprintln!("Values above are for the recording you named; — means absent.");
            } else {
                eprintln!(
                    "Run with --example <ID> to see what each field holds for a real recording."
                );
            }
        }
    }
    exit::OK
}
