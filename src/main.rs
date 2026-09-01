//! `findopera` — name a music library from FindOpera metadata.

use clap::{Args, Parser, Subcommand};
use findopera::model::FIELDS;
use findopera::FieldDoc;
use findopera::{api, plan, scan, Template};
use std::io::Write;
use std::path::PathBuf;

const AFTER_HELP: &str = "\
Exit codes:
  0  everything rendered
  1  a recording is missing, its render is not a usable path, or two
     directories want the same name
  2  the template or the arguments are wrong
  3  the API was unreachable or errored

Results go to stdout; everything else to stderr.";

#[derive(Parser)]
#[command(
    name = "findopera",
    version,
    about = "Name a music library from FindOpera metadata, through a template",
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
    /// Render every recording found under a directory.
    #[command(
        long_about = "\
Walk directories for FindOpera marker files and render each recording they
name, printing the directory beside the result.

A marker is any .txt file whose text contains a findopera.com/recording/<id>
URL — exactly what https://findopera.com/recording/<id>.txt serves. What it
identifies is the folder holding it, so the workflow is to save the .txt into
each recording's directory.",
        after_help = "\
Examples:
  findopera scan '{{composer.lastName}}/{{opera.title}}' ~/Music
  findopera scan '{{opera.title}}[ ({{year}})]' . --null | xargs -0 …"
    )]
    Scan(ScanArgs),

    /// List every field a template may use.
    Fields,
}

#[derive(Args)]
struct ScanArgs {
    /// Template. `{{field}}` placeholders, `|`-separated fallbacks with a
    /// quoted literal last, and `[optional groups]` dropped when a placeholder
    /// inside them resolves to nothing.
    #[arg(value_name = "TEMPLATE")]
    template: String,
    /// Directories to walk.
    #[arg(value_name = "DIR", default_value = ".")]
    roots: Vec<PathBuf>,
    /// Follow symlinks while walking.
    #[arg(long)]
    follow_links: bool,
    /// Separate the directory from the result with a tab instead of padding,
    /// for feeding another program.
    #[arg(long)]
    tabs: bool,
    /// Fail if any directory had to be numbered.
    ///
    /// Two rips of one recording get a number each unless a marker says which
    /// is which, and a number taken from walk order shifts as the library
    /// changes. This insists every one of them be named.
    #[arg(long)]
    require_variants: bool,
    /// GraphQL endpoint.
    #[arg(long, default_value = api::DEFAULT_ENDPOINT, value_name = "URL")]
    endpoint: String,
}

fn main() {
    std::process::exit(run());
}

/// Write one result line, reporting whether stdout is still listening.
///
/// This command's output is a list, so it will be piped to `head` — and the
/// default `println!` panics when the reader goes away. Stopping quietly is
/// what every other line-producing tool does.
fn emit(out: &mut impl Write, line: std::fmt::Arguments) -> bool {
    writeln!(out, "{line}").is_ok()
}

fn run() -> i32 {
    match Cli::parse().command {
        Command::Scan(args) => cmd_scan(args),
        Command::Fields => cmd_fields(),
    }
}

fn cmd_scan(args: ScanArgs) -> i32 {
    let report = scan::scan(&args.roots, args.follow_links);
    for (path, why) in &report.unreadable {
        eprintln!("findopera: {}: {why}", path.display());
    }
    if report.markers.is_empty() {
        eprintln!(
            "findopera: no marker files under {}",
            args.roots
                .iter()
                .map(|r| r.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
        return 1;
    }

    // The schema a scan template is checked against is the model's, plus the
    // one field that comes from the marker rather than the recording.
    let mut schema: Vec<FieldDoc> = FIELDS.to_vec();
    schema.push(scan::VARIANT);

    // Parse before fetching: a bad template is the caller's mistake, and
    // should not cost a network round trip to discover.
    let tmpl = match Template::parse(&args.template, &schema) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("findopera: {e}");
            for line in e.underline(&args.template) {
                eprintln!("  {line}");
            }
            if let Some(help) = &e.help {
                eprintln!("  help: {help}");
            }
            return 2;
        }
    };

    let ids: Vec<String> = report.markers.iter().map(|m| m.id.clone()).collect();
    let recordings = match api::recordings(&args.endpoint, &ids) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("findopera: {e}");
            return 3;
        }
    };

    let plan = plan::plan(&report.markers, &recordings, &tmpl);

    let mut out = std::io::stdout().lock();
    for line in plan.listing(args.tabs) {
        if !emit(&mut out, format_args!("{line}")) {
            return 0;
        }
    }
    let report_lines = plan.report(args.require_variants);
    if !report_lines.is_empty() {
        eprintln!();
        for line in report_lines {
            // The continuation lines are indented and belong to the line
            // above; prefixing each of them would break the shape.
            if line.starts_with(' ') {
                eprintln!("{line}");
            } else {
                eprintln!("findopera: {line}");
            }
        }
    }
    i32::from(plan.blocked(args.require_variants))
}

fn cmd_fields() -> i32 {
    let mut out = std::io::stdout().lock();
    let width = FIELDS.iter().map(|f| f.path.len()).max().unwrap_or(0);
    for f in FIELDS {
        let always = if f.nullable { "" } else { "  (always present)" };
        if !emit(
            &mut out,
            format_args!("{:<width$}  {}{always}", f.path, f.description),
        ) {
            break;
        }
    }
    0
}
