//! `findopera` — name a music library from FindOpera metadata.

use clap::{Args, Parser, Subcommand};
use findopera::config::{self, Config};
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

    /// Write a starter findopera.toml, explaining every setting.
    #[command(long_about = "\
Write a starter findopera.toml into a directory, with every setting present
and explained, so there is no blank file to guess at.

It refuses to overwrite one that is already there.")]
    Init(InitArgs),
}

#[derive(Args)]
struct InitArgs {
    /// Directory to write findopera.toml into.
    #[arg(value_name = "DIR", default_value = ".")]
    dir: PathBuf,
}

#[derive(Args)]
struct ScanArgs {
    /// Directories to walk.
    #[arg(value_name = "DIR", default_value = ".")]
    roots: Vec<PathBuf>,
    /// Settings file. Defaults to findopera.toml beside the first DIR.
    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,
    /// Template, overriding the one in the settings file.
    ///
    /// `{{field}}` placeholders, `|`-separated fallbacks with a quoted literal
    /// last, and `[optional groups]` dropped when a placeholder inside them
    /// turns out to be absent.
    #[arg(long, short = 't', value_name = "TEMPLATE")]
    template: Option<String>,
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
        Command::Init(args) => cmd_init(args),
    }
}

fn cmd_init(args: InitArgs) -> i32 {
    let path = args.dir.join(config::FILE_NAME);
    if path.exists() {
        eprintln!("findopera: {} is already there", path.display());
        return 2;
    }
    match std::fs::write(&path, config::starter()) {
        Ok(()) => {
            println!("{}", path.display());
            eprintln!("findopera: edit the template in there, then run `findopera scan`");
            0
        }
        Err(e) => {
            eprintln!("findopera: cannot write {}: {e}", path.display());
            1
        }
    }
}

fn cmd_scan(args: ScanArgs) -> i32 {
    // The settings live beside what is being scanned, or wherever --config
    // says. Nothing searches up the tree: a source can carry several of these,
    // one per way of naming it, and which one ran should never be a guess.
    let config_path = args
        .config
        .clone()
        .unwrap_or_else(|| args.roots[0].join(config::FILE_NAME));
    let settings = match Config::load(&config_path) {
        Ok(c) => Some(c),
        // A template on the command line is reason enough not to need a file.
        Err(config::ConfigError::Missing { .. }) if args.template.is_some() => None,
        Err(e) => {
            eprintln!("findopera: {e}");
            return 2;
        }
    };
    let template = match args
        .template
        .clone()
        .or_else(|| settings.as_ref().map(|c| c.template.clone()))
    {
        Some(t) => t,
        None => unreachable!("a missing config without --template already returned"),
    };
    let follow_links = args.follow_links || settings.as_ref().is_some_and(|c| c.follow_links);
    let require_variants =
        args.require_variants || settings.as_ref().is_some_and(|c| c.require_variants);

    let report = scan::scan(&args.roots, follow_links);
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
    let tmpl = match Template::parse(&template, &schema) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("findopera: {e}");
            for line in e.underline(&template) {
                eprintln!("  {line}");
            }
            if let Some(help) = &e.help {
                eprintln!("  help: {help}");
            }
            // The engine has no business knowing what this program is called,
            // so the pointer to it is added here. `fields` lists the syntax as
            // well as the fields, which makes it the right answer whether the
            // template named something that is not there or was malformed.
            eprintln!("  see `findopera fields` for every field and the syntax");
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
    let report_lines = plan.report(require_variants);
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
    i32::from(plan.blocked(require_variants))
}

fn cmd_fields() -> i32 {
    // The list is the result and goes to stdout, so `findopera fields | grep
    // singer` stays useful. Everything explaining it goes to stderr.
    eprintln!("{}", findopera::SYNTAX);
    eprintln!(
        "\nA field marked `always` has a value for every recording and needs no\n\
         fallback. The rest want one — {{{{field|\"Unknown\"}}}} — or a [ … ] around\n\
         them, and findopera will say so if they have neither.\n"
    );

    let mut out = std::io::stdout().lock();
    let width = FIELDS.iter().map(|f| f.path.len()).max().unwrap_or(0);
    for f in FIELDS {
        let always = if f.nullable { "      " } else { "always" };
        if !emit(
            &mut out,
            format_args!("{:<width$}  {always}  {}", f.path, f.description),
        ) {
            break;
        }
    }
    0
}
