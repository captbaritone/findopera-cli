//! `findopera` — name a music library from FindOpera metadata.

use clap::{Args, Parser, Subcommand};
use findopera::config::{self, Config};
use findopera::model::{Recording, FIELDS};
use findopera::FieldDoc;
use findopera::{api, apply, plan, scan, Template};
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

    /// Build the named tree from the settings file.
    #[command(long_about = "\
Build the tree of names `scan` describes, at the destination in the settings
file, by the means it names — a symlink to each folder, a hard link to every
file in it, or a copy.

Nothing is ever deleted or overwritten. Anything already in place is left as
it is, so running this again after adding one recording does one thing, and
meeting something unexpected stops it rather than deciding for you.")]
    Apply(ApplyArgs),

    /// Write a starter findopera.toml, explaining every setting.
    #[command(long_about = "\
Write a starter findopera.toml into a directory, with every setting present
and explained, so there is no blank file to guess at.

It refuses to overwrite one that is already there.")]
    Init(InitArgs),
}

#[derive(Args)]
struct ApplyArgs {
    /// Directory to walk.
    #[arg(value_name = "DIR", default_value = ".")]
    root: PathBuf,
    /// Settings file. Defaults to findopera.toml beside DIR.
    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,
    /// Say what would be built, and build nothing.
    #[arg(long)]
    dry_run: bool,
}

#[derive(Args)]
struct InitArgs {
    /// Directory to write findopera.toml into.
    #[arg(value_name = "DIR", default_value = ".")]
    dir: PathBuf,
}

#[derive(Args)]
struct ScanArgs {
    /// Directory to walk.
    #[arg(value_name = "DIR", default_value = ".")]
    root: PathBuf,
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
        Command::Apply(args) => cmd_apply(args),
        Command::Init(args) => cmd_init(args),
    }
}

fn cmd_apply(args: ApplyArgs) -> i32 {
    let p = match prepare(
        &args.root,
        args.config.as_ref(),
        None,
        false,
        false,
        api::DEFAULT_ENDPOINT,
    ) {
        Ok(p) => p,
        Err(code) => return code,
    };
    let Some(settings) = p.settings.as_ref() else {
        eprintln!("findopera: apply needs a settings file, for the destination if nothing else");
        return 2;
    };
    let Some(destination) = settings.destination.as_ref() else {
        eprintln!(
            "findopera: no destination set — add one to the settings file:\n\
             \x20   destination = \"/path/to/named\""
        );
        return 2;
    };

    let plan = plan::plan(&p.report.markers, &p.recordings, &p.template);
    for line in plan.report(p.require_variants) {
        if line.starts_with(' ') {
            eprintln!("{line}");
        } else {
            eprintln!("findopera: {line}");
        }
    }

    // Everything knowable before writing is settled here, because half a tree
    // is worse than none.
    if let Err(why) = apply::preflight(
        &plan,
        &args.root,
        destination,
        settings.link,
        p.require_variants,
    ) {
        eprintln!("findopera: {why}");
        return 2;
    }

    // Say where, before doing it. Nothing on the command line names the
    // destination, so this is the only place it is stated.
    let what = match settings.link {
        config::Link::Symlink => "a link to each folder",
        config::Link::Hardlink => "a hard link to every file",
        config::Link::Copy => "a copy of every file",
    };
    eprintln!(
        "findopera: {} {} in {}",
        if args.dry_run {
            "would build"
        } else {
            "building"
        },
        what,
        destination.display()
    );

    let done = apply::apply(&plan, destination, settings.link, args.dry_run);
    let mut out = std::io::stdout().lock();
    for entry in &done.entries {
        let mark = match &entry.outcome {
            apply::Outcome::Created => "+",
            apply::Outcome::Skipped => " ",
            apply::Outcome::Conflict(_) | apply::Outcome::Failed(_) => "!",
        };
        if !emit(
            &mut out,
            format_args!("{mark} {}", entry.destination.display()),
        ) {
            return 0;
        }
        match &entry.outcome {
            apply::Outcome::Conflict(why) | apply::Outcome::Failed(why) => {
                eprintln!("    {why}");
                eprintln!("    (for {})", entry.source.display());
            }
            _ => {}
        }
    }
    let (made, skipped, trouble) = done.counts();
    eprintln!(
        "findopera: {made} {}, {skipped} already there, {trouble} left alone",
        if args.dry_run { "to build" } else { "built" }
    );
    i32::from(done.troubled())
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

/// Everything `scan` and `apply` both need before they can differ.
struct Prepared {
    settings: Option<Config>,
    template: Template,
    report: scan::Report,
    recordings: BTreeMap<String, Recording>,
    require_variants: bool,
}

/// Load the settings, parse the template, walk the library, fetch what it names.
///
/// The settings live beside what is being scanned, or wherever `--config`
/// says. Nothing searches up the tree: a library can carry several of these,
/// one per way of naming it, and which one ran should never be a guess.
fn prepare(
    root: &Path,
    config: Option<&PathBuf>,
    template_arg: Option<&String>,
    follow_links_arg: bool,
    require_variants_arg: bool,
    endpoint: &str,
) -> Result<Prepared, i32> {
    let config_path = config
        .cloned()
        .unwrap_or_else(|| root.join(config::FILE_NAME));
    let settings = match Config::load(&config_path) {
        Ok(c) => Some(c),
        // A template on the command line is reason enough not to need a file.
        Err(config::ConfigError::Missing { .. }) if template_arg.is_some() => None,
        Err(e) => {
            eprintln!("findopera: {e}");
            return Err(2);
        }
    };
    let template = template_arg
        .cloned()
        .or_else(|| settings.as_ref().map(|c| c.template.clone()))
        .expect("a missing config without --template already returned");
    let follow_links = follow_links_arg || settings.as_ref().is_some_and(|c| c.follow_links);
    let require_variants =
        require_variants_arg || settings.as_ref().is_some_and(|c| c.require_variants);

    let report = scan::scan(root, follow_links);
    for (path, why) in &report.unreadable {
        eprintln!("findopera: {}: {why}", path.display());
    }
    if report.markers.is_empty() {
        eprintln!("findopera: no marker files under {}", root.display());
        return Err(1);
    }

    // The schema a template is checked against is the model's, plus the one
    // field that comes from the marker rather than the recording.
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
            return Err(2);
        }
    };

    let ids: Vec<String> = report.markers.iter().map(|m| m.id.clone()).collect();
    let recordings = match api::recordings(endpoint, &ids) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("findopera: {e}");
            return Err(3);
        }
    };
    Ok(Prepared {
        settings,
        template: tmpl,
        report,
        recordings,
        require_variants,
    })
}

fn cmd_scan(args: ScanArgs) -> i32 {
    let p = match prepare(
        &args.root,
        args.config.as_ref(),
        args.template.as_ref(),
        args.follow_links,
        args.require_variants,
        &args.endpoint,
    ) {
        Ok(p) => p,
        Err(code) => return code,
    };
    let require_variants = p.require_variants;
    let plan = plan::plan(&p.report.markers, &p.recordings, &p.template);

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
