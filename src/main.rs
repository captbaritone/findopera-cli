//! `findopera` — name a music library from FindOpera metadata.

use clap::{Args, Parser, Subcommand};
use findopera::model::{Recording, FIELDS};
use findopera::{api, scan, to_path, Template};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

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

    // Parse before fetching: a bad template is the caller's mistake, and
    // should not cost a network round trip to discover.
    let tmpl = match Template::parse(&args.template, FIELDS) {
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

    // Pad the directory column so the two line up, unless the caller wants
    // this piped somewhere.
    let width = if args.tabs {
        0
    } else {
        report
            .markers
            .iter()
            .map(|m| m.directory.display().to_string().chars().count())
            .max()
            .unwrap_or(0)
    };

    let mut out = std::io::stdout().lock();
    let mut failed = false;
    // Two directories can legitimately name one recording — a FLAC rip and an
    // MP3 rip of the same performance. The template cannot tell them apart,
    // because nothing in the recording does: they *are* the same recording.
    // What matters is that this is said out loud, since acting on a proposal
    // where two sources share one destination loses a directory.
    let mut proposed: BTreeMap<String, Vec<&Path>> = BTreeMap::new();
    for marker in &report.markers {
        let Some(path) = render_one(&tmpl, &recordings, &marker.id) else {
            eprintln!("  (from {})", marker.marker_path.display());
            failed = true;
            continue;
        };
        proposed
            .entry(path.clone())
            .or_default()
            .push(&marker.directory);
        let dir = marker.directory.display().to_string();
        let ok = if args.tabs {
            emit(&mut out, format_args!("{dir}\t{path}"))
        } else {
            emit(&mut out, format_args!("{dir:<width$}  {path}"))
        };
        if !ok {
            return 0;
        }
    }

    let clashes: Vec<_> = proposed.iter().filter(|(_, v)| v.len() > 1).collect();
    if !clashes.is_empty() {
        let dirs: usize = clashes.iter().map(|(_, v)| v.len()).sum();
        eprintln!(
            "\nfindopera: {dirs} directories want {} name(s) — the recording is the \
             same, so no template can separate them:",
            clashes.len()
        );
        for (path, sources) in &clashes {
            eprintln!("  {path}");
            for s in sources.iter() {
                eprintln!("      {}", s.display());
            }
        }
        failed = true;
    }
    i32::from(failed)
}

/// Render one id, reporting to stderr and yielding `None` if it cannot be.
fn render_one(
    tmpl: &Template,
    recordings: &BTreeMap<String, Recording>,
    id: &str,
) -> Option<String> {
    let Some(rec) = recordings.get(id) else {
        eprintln!("findopera: recording {id} is not in the FindOpera database");
        return None;
    };
    // Rendering cannot fail; only judging the result as a path can.
    let rendered = tmpl.render(rec);
    match to_path(&rendered) {
        Ok(segments) => Some(segments.join("/")),
        Err(e) => {
            eprintln!("findopera: recording {id} {e}");
            None
        }
    }
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
