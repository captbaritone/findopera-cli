//! `findopera` — organize opera recordings into a canonical directory tree.

mod api;
mod library;
mod model;
mod output;
mod scan;
mod schema;
mod template;

use clap::{Args, Parser, Subcommand};
use output::{exit, Failure, Format};
use std::path::PathBuf;

const AFTER_HELP: &str = "\
Exit codes:
  0   success
  1   general error
  2   invalid arguments or template
  3   a marker names a recording not in the database
  4   permission denied reading or writing a path
  5   destination exists and was not created by findopera
  6   the FindOpera API was unreachable or errored (retryable)
  10  plan produced and safe to run with --apply

Output is JSON when stdout is piped and a table at a terminal; override with
--format json|table|ndjson.";

#[derive(Parser)]
#[command(
    name = "findopera",
    version,
    about = "Organize opera recordings into a canonical tree using findopera.com",
    long_about = "Organize opera recordings into a canonical tree using findopera.com.\n\n\
                  Download https://findopera.com/recording/<id>.txt into each recording's\n\
                  directory, then point this tool at those directories: it reads the marker\n\
                  files, looks the recordings up in the FindOpera database, and builds a tree\n\
                  of symlinks named by a template you supply.",
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
    /// Build and inspect the canonical tree of symlinks.
    #[command(subcommand)]
    Library(LibraryCommand),
    /// Look up recordings in the FindOpera database.
    #[command(subcommand)]
    Recording(RecordingCommand),
    /// Find and inspect FindOpera marker files on disk.
    #[command(subcommand)]
    Marker(MarkerCommand),
    /// Print the command tree, flags, and template fields as JSON.
    #[command(after_help = "Examples:\n  \
        findopera schema --all\n  \
        findopera schema library sync")]
    Schema(SchemaArgs),
}

#[derive(Subcommand)]
enum LibraryCommand {
    /// Preview the tree without touching the disk.
    #[command(long_about = "\
Preview the tree without touching the disk. Exits 10 when the plan is clean,
so a caller can tell \"safe to apply\" from \"nothing to do\".

Examples:
  findopera library plan -s ~/Music/Opera -d ~/Opera \\
    -t '{{composer.lastName}}/{{opera.title}}/{{year}} - {{conductor.lastName}}'

  findopera library plan -s ~/Music/Opera -d ~/Opera \\
    -t '{{opera.title}} ({{year|\"n.d.\"}})' --format json | jq '.links[]'")]
    Plan(SyncArgs),
    /// Rebuild the tree. Previews by default; pass --apply to write.
    #[command(long_about = "\
Rebuild the tree. Previews by default; nothing is written until --apply.

The destination is wiped and rebuilt on every run, so it always matches the
markers exactly. A non-empty destination that findopera did not create is
refused unless --force is given.

Examples:
  # preview
  findopera library sync -s ~/Music/Opera -d ~/Opera \\
    -t '{{composer.lastName}}/{{opera.title}}/{{year}}'

  # write it
  findopera library sync -s ~/Music/Opera -d ~/Opera \\
    -t '{{composer.lastName}}/{{opera.title}}/{{year}}' --apply")]
    Sync(SyncArgs),
    /// List the fields a --template can interpolate, with descriptions.
    #[command(long_about = "\
List the fields a --template can interpolate, with a one-line description of
each. Pass --example <ID> to see what each field actually renders to for a real
recording, which is the quickest way to find out whether a field is populated.

Examples:
  findopera library fields
  findopera library fields --example 10655
  findopera library fields --format json | jq -r '.fields[].field'")]
    Fields(FieldsArgs),
}

#[derive(Subcommand)]
enum RecordingCommand {
    /// Fetch one or more recordings by id.
    #[command(after_help = "Examples:\n  \
        findopera recording get 10655\n  \
        findopera recording get 75 10655 --format ndjson")]
    Get(RecordingGetArgs),
}

#[derive(Subcommand)]
enum MarkerCommand {
    /// List every .txt marker found under the given directories.
    #[command(after_help = "Examples:\n  \
        findopera marker list --source ~/Music/Opera\n  \
        findopera marker list --source ~/Music/Opera --format json | jq '.markers[].recordingId'")]
    List(MarkerListArgs),
}

#[derive(Args)]
struct FormatArgs {
    /// Output format <optional, default: json when piped, table at a terminal>
    #[arg(long, value_enum, global = true)]
    format: Option<Format>,
    /// Disable ANSI color <optional>
    #[arg(long, global = true)]
    no_color: bool,
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
struct SyncArgs {
    /// Directory to search for marker files <required, repeatable>
    #[arg(long, short = 's', required = true, value_name = "DIR")]
    source: Vec<PathBuf>,
    /// Directory to build the canonical tree in <required>
    #[arg(long, short = 'd', value_name = "DIR")]
    destination: PathBuf,
    /// Path template <required>. `{{field}}`, `|`-separated fallbacks, quoted
    /// literal last: '{{opera.englishTitle|opera.title|"Untitled"}}/{{year}}'.
    /// Fields: `findopera library fields`.
    #[arg(long, short = 't', value_name = "TEMPLATE")]
    template: String,
    /// Write the tree. Without this the command only prints the plan.
    #[arg(long)]
    apply: bool,
    /// Print the plan and exit, even if --apply was given <optional>
    #[arg(long, conflicts_with = "apply")]
    dry_run: bool,
    /// Wipe the destination even if findopera did not create it <optional>
    #[arg(long)]
    force: bool,
    /// Follow symlinks while searching for markers <optional>
    #[arg(long)]
    follow_links: bool,
    #[command(flatten)]
    fmt: FormatArgs,
    #[command(flatten)]
    api: ApiArgs,
}

#[derive(Args)]
struct RecordingGetArgs {
    /// FindOpera recording ids <required, one or more>
    #[arg(required = true, value_name = "ID")]
    ids: Vec<String>,
    #[command(flatten)]
    fmt: FormatArgs,
    #[command(flatten)]
    api: ApiArgs,
}

#[derive(Args)]
struct MarkerListArgs {
    /// Directory to search for marker files <required, repeatable>
    #[arg(long, short = 's', required = true, value_name = "DIR")]
    source: Vec<PathBuf>,
    /// Follow symlinks while searching <optional>
    #[arg(long)]
    follow_links: bool,
    #[command(flatten)]
    fmt: FormatArgs,
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
    /// Command path to describe, e.g. `library sync` <optional>
    #[arg(value_name = "COMMAND")]
    path: Vec<String>,
}

fn main() {
    let code = run();
    std::process::exit(code);
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
        Command::Library(LibraryCommand::Plan(args)) => cmd_sync(args, true),
        Command::Library(LibraryCommand::Sync(args)) => cmd_sync(args, false),
        Command::Library(LibraryCommand::Fields(args)) => cmd_fields(args),
        Command::Recording(RecordingCommand::Get(args)) => cmd_recording_get(args),
        Command::Marker(MarkerCommand::List(args)) => cmd_marker_list(args),
        Command::Schema(args) => schema::print_schema(&args.all, &args.path),
    }
}

fn cmd_sync(args: SyncArgs, plan_only: bool) -> i32 {
    let format = Format::resolve(args.fmt.format);
    let color = output::use_color(args.fmt.no_color);

    // `plan` shares its flags with `sync`, so `--apply` parses here. Silently
    // ignoring it would let a caller believe the tree had been written.
    if plan_only && args.apply {
        return Failure::new(
            "flag_not_supported",
            "`library plan` never writes; it does not accept --apply",
        )
        .suggest("Run `findopera library sync … --apply` to write the tree.")
        .emit(exit::USAGE);
    }

    let tmpl = match template::Template::parse(&args.template) {
        Ok(t) => t,
        Err(e) => {
            let mut f = Failure::new(e.code(), format!("invalid --template: {e}"))
                .input(args.template.clone());
            if let template::TemplateError::UnknownField { path } = &e {
                f = f.suggest(format!(
                    "`{path}` is not a template field. Run `findopera library fields` \
                     to list them."
                ));
            }
            return f.emit(exit::USAGE);
        }
    };

    for source in &args.source {
        if !source.is_dir() {
            return Failure::new(
                "source_not_found",
                format!("--source {} is not a directory", source.display()),
            )
            .input(source.display().to_string())
            .emit(exit::USAGE);
        }
    }

    let report = scan::scan(&args.source, args.follow_links);
    let ids: Vec<String> = {
        let mut v: Vec<String> = report
            .markers
            .iter()
            .map(|m| m.recording_id.clone())
            .collect();
        v.sort();
        v.dedup();
        v
    };

    let client = api::Client::new(args.api.endpoint.clone(), args.api.timeout);
    let recordings = if ids.is_empty() {
        Default::default()
    } else {
        match client.recordings(&ids) {
            Ok(r) => r,
            Err(e) => {
                return Failure::new(e.code, e.to_string())
                    .retryable(e.retryable)
                    .suggest(if e.retryable {
                        "The API may be temporarily unavailable; retry in a moment."
                    } else {
                        "Check --endpoint, or run `findopera recording get <id>` to test."
                    })
                    .emit(exit::API)
            }
        }
    };

    let plan = library::plan(
        &args.destination,
        &args.template,
        &tmpl,
        &report.markers,
        &recordings,
        report.skipped.len(),
    );

    let will_apply = args.apply && !plan_only && !args.dry_run;

    if !will_apply {
        render_plan(&plan, &report, format, color);
        // Conflicts or problems mean the plan is not safe to apply as-is.
        if !plan.conflicts.is_empty() {
            return exit::CONFLICT;
        }
        if plan
            .problems
            .iter()
            .any(|p| p.error == "recording_not_found")
        {
            return exit::NOT_FOUND;
        }
        if !plan.problems.is_empty() {
            return exit::GENERAL;
        }
        return exit::DRY_RUN_OK;
    }

    // Refuse to write a tree that is knowably wrong.
    if !plan.conflicts.is_empty() {
        render_plan(&plan, &report, format, color);
        return Failure::new(
            "path_conflict",
            format!(
                "{} canonical path(s) are claimed by more than one recording",
                plan.conflicts.len()
            ),
        )
        .suggest(
            "Add a distinguishing field to --template, such as {{conductor.lastName}} \
             or {{id}}.",
        )
        .emit(exit::CONFLICT);
    }

    match library::apply(&args.destination, &plan, args.force) {
        Ok(result) => {
            match format {
                Format::Json | Format::Ndjson => output::print_json(&serde_json::json!({
                    "applied": true,
                    "destination": result.destination,
                    "linksCreated": result.links_created,
                    "directoriesCreated": result.directories_created,
                    "problems": plan.problems,
                })),
                Format::Table => {
                    println!(
                        "Created {} link(s) in {}",
                        result.links_created, result.destination
                    );
                    for p in &plan.problems {
                        eprintln!("  skipped: {}", p.message);
                    }
                }
            }
            if plan
                .problems
                .iter()
                .any(|p| p.error == "recording_not_found")
            {
                return exit::NOT_FOUND;
            }
            exit::OK
        }
        Err(library::ApplyError::Unmanaged { path }) => Failure::new(
            "destination_unmanaged",
            format!(
                "{} is not empty and has no {} stamp, so findopera will not wipe it",
                path.display(),
                library::STAMP
            ),
        )
        .input(path.display().to_string())
        .suggest(
            "Point --destination at an empty or findopera-created directory, \
             or pass --force to wipe it anyway.",
        )
        .emit(exit::CONFLICT),
        Err(library::ApplyError::Io { path, source }) => {
            let denied = source.kind() == std::io::ErrorKind::PermissionDenied;
            Failure::new("io_error", format!("{}: {source}", path.display()))
                .input(path.display().to_string())
                .emit(if denied {
                    exit::PERMISSION
                } else {
                    exit::GENERAL
                })
        }
    }
}

fn render_plan(plan: &library::Plan, report: &scan::ScanReport, format: Format, color: bool) {
    match format {
        Format::Json => output::print_json(plan),
        Format::Ndjson => output::print_ndjson(&plan.links),
        Format::Table => {
            let (dim, reset) = if color {
                ("\x1b[2m", "\x1b[0m")
            } else {
                ("", "")
            };
            // The arrow reads `link -> target`, matching `ls -l` and the
            // symlink itself. Say so: read as a pipeline it looks backwards.
            if !plan.links.is_empty() {
                eprintln!(
                    "Symlinks to create under {} — shown as `link -> target`, as ls -l prints them:\n",
                    plan.destination
                );
            }
            for link in &plan.links {
                println!("{}{dim} -> {}{reset}", link.path, link.target);
            }
            if !plan.conflicts.is_empty() {
                eprintln!();
                for c in &plan.conflicts {
                    eprintln!(
                        "conflict: {} claimed by recordings {}",
                        c.path,
                        c.recording_ids.join(", ")
                    );
                }
            }
            if !plan.problems.is_empty() {
                eprintln!();
                for p in &plan.problems {
                    eprintln!("problem: {}", p.message);
                }
            }
            for (path, err) in &report.unreadable {
                eprintln!("unreadable: {}: {err}", path.display());
            }
            eprintln!(
                "\n{} marker(s), {} link(s), {} conflict(s), {} problem(s)",
                plan.summary.markers_found,
                plan.summary.links_planned,
                plan.summary.conflicts,
                plan.summary.problems
            );
        }
    }
}

fn cmd_recording_get(args: RecordingGetArgs) -> i32 {
    let format = Format::resolve(args.fmt.format);
    for id in &args.ids {
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
            return Failure::new(
                "invalid_recording_id",
                format!("`{id}` is not a numeric FindOpera recording id"),
            )
            .input(id.clone())
            .suggest("Ids are the number in https://findopera.com/recording/<id>.")
            .emit(exit::USAGE);
        }
    }
    let client = api::Client::new(args.api.endpoint.clone(), args.api.timeout);
    let recordings = match client.recordings(&args.ids) {
        Ok(r) => r,
        Err(e) => {
            return Failure::new(e.code, e.to_string())
                .retryable(e.retryable)
                .emit(exit::API)
        }
    };

    let rows: Vec<serde_json::Value> = args
        .ids
        .iter()
        .filter_map(|id| recordings.get(id).map(|r| recording_json(id, r)))
        .collect();

    match format {
        Format::Json => output::print_json(&rows),
        Format::Ndjson => output::print_ndjson(&rows),
        Format::Table => {
            for row in &rows {
                for f in model::Recording::FIELDS {
                    if let Some(v) = row.get(f.path).and_then(|v| v.as_str()) {
                        println!("{:<28} {v}", f.path);
                    }
                }
                println!();
            }
        }
    }

    let missing: Vec<&String> = args
        .ids
        .iter()
        .filter(|id| !recordings.contains_key(*id))
        .collect();
    if !missing.is_empty() {
        return Failure::new(
            "recording_not_found",
            format!(
                "{} recording id(s) are not in the FindOpera database",
                missing.len()
            ),
        )
        .details(missing.iter().map(|m| m.to_string()).collect())
        .emit(exit::NOT_FOUND);
    }
    exit::OK
}

/// Flat map of template path -> value, so `recording get` output and the
/// template surface stay identical.
fn recording_json(id: &str, rec: &model::Recording) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    map.insert("id".to_string(), serde_json::Value::String(id.to_string()));
    for f in model::Recording::FIELDS {
        let value = match rec.get(f.path) {
            Ok(Some(v)) => serde_json::Value::String(v),
            _ => serde_json::Value::Null,
        };
        map.insert(f.path.to_string(), value);
    }
    serde_json::Value::Object(map)
}

fn cmd_marker_list(args: MarkerListArgs) -> i32 {
    let format = Format::resolve(args.fmt.format);
    for source in &args.source {
        if !source.is_dir() {
            return Failure::new(
                "source_not_found",
                format!("--source {} is not a directory", source.display()),
            )
            .input(source.display().to_string())
            .emit(exit::USAGE);
        }
    }
    let report = scan::scan(&args.source, args.follow_links);
    let rows: Vec<serde_json::Value> = report
        .markers
        .iter()
        .map(|m| {
            serde_json::json!({
                "recordingId": m.recording_id,
                "recordingDir": m.recording_dir.display().to_string(),
                "marker": m.marker_path.display().to_string(),
            })
        })
        .collect();

    match format {
        Format::Json => output::print_json(&serde_json::json!({
            "markers": rows,
            "summary": {
                "markersFound": report.markers.len(),
                "txtFilesSkipped": report.skipped.len(),
                "unreadable": report.unreadable.len(),
            },
        })),
        Format::Ndjson => output::print_ndjson(&rows),
        Format::Table => {
            for m in &report.markers {
                println!("{:<10} {}", m.recording_id, m.recording_dir.display());
            }
            eprintln!(
                "\n{} marker(s), {} .txt file(s) without a FindOpera URL",
                report.markers.len(),
                report.skipped.len()
            );
        }
    }
    exit::OK
}

/// `findopera library fields` — the template surface, with descriptions and,
/// optionally, what each field renders to for a real recording.
fn cmd_fields(args: FieldsArgs) -> i32 {
    let format = Format::resolve(args.fmt.format);

    // With --example, fetch one recording so the user can see which fields are
    // actually populated. This is the fastest way to discover that, say,
    // `opera.librettist` is empty for most entries.
    let example = match &args.example {
        None => None,
        Some(id) => {
            if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
                return Failure::new(
                    "invalid_recording_id",
                    format!("`{id}` is not a numeric FindOpera recording id"),
                )
                .input(id.clone())
                .suggest("Ids are the number in https://findopera.com/recording/<id>.")
                .emit(exit::USAGE);
            }
            let client = api::Client::new(args.api.endpoint.clone(), args.api.timeout);
            match client.recordings(std::slice::from_ref(id)) {
                Ok(map) => match map.into_values().next() {
                    Some(rec) => Some(rec),
                    None => {
                        return Failure::new(
                            "recording_not_found",
                            format!("recording {id} is not in the FindOpera database"),
                        )
                        .input(id.clone())
                        .emit(exit::NOT_FOUND)
                    }
                },
                Err(e) => {
                    return Failure::new(e.code, e.to_string())
                        .retryable(e.retryable)
                        .emit(exit::API)
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
                "separator": "/ in the template is a directory separator; \
                              / inside a value is replaced",
            },
        })),
        Format::Ndjson => output::print_ndjson(&rows),
        Format::Table => {
            let width = model::Recording::FIELDS
                .iter()
                .map(|f| f.path.len())
                .max()
                .unwrap_or(28);
            for (f, row) in model::Recording::FIELDS.iter().zip(&rows) {
                match example.is_some() {
                    false => println!("{:<width$}  {}", f.path, f.description),
                    true => {
                        let v = row["value"].as_str().unwrap_or("—");
                        println!("{:<width$}  {}", f.path, v);
                    }
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
