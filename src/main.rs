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
  0  nothing to report
  1  a recording is missing, a name is not a usable path, two folders want
     the same name, or something was in the way of building
  2  the settings, the template or the arguments are wrong
  3  the API was unreachable, or refused

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
    /// Work out what each folder should be called, and optionally build it.
    #[command(
        long_about = "\
Walk a library for marker files, work out what each recording's folder should
be called, and — with --write — build a tree of those names at the destination
in the settings file.

Nothing is written without --write. Nothing is ever deleted or overwritten.

With no destination set the naming is still worked out and shown, which is
what you want while you are still settling on a template.",
        after_help = "\
Examples:
  findopera organize ~/Music
  findopera organize ~/Music --write
  findopera organize ~/Music -t '{{opera.title}}[ ({{year}})]'
  findopera organize ~/Music --config ~/Music/by-conductor.toml"
    )]
    Organize(OrganizeArgs),

    /// List every field a template may use, and the syntax.
    Fields,

    /// Write a starter findopera.toml, explaining every setting.
    #[command(long_about = "\
Write a starter findopera.toml into a directory, with every setting present
and explained, so there is no blank file to guess at.

It refuses to overwrite one that is already there.")]
    Init(InitArgs),
}

#[derive(Args)]
struct OrganizeArgs {
    /// Directory to walk.
    #[arg(value_name = "DIR", default_value = ".")]
    root: PathBuf,
    /// Settings file. Defaults to findopera.toml beside DIR.
    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,
    /// Template, overriding the one in the settings file.
    ///
    /// `{{field}}` placeholders, `|`-separated fallbacks with a quoted literal
    /// last, and `[optional groups]` dropped when a placeholder inside them
    /// turns out to be absent.
    #[arg(long, short = 't', value_name = "TEMPLATE")]
    template: Option<String>,
    /// Actually build it.
    ///
    /// Without this, nothing is written: the command says what it would do and
    /// stops. The destination lives in the settings file rather than on the
    /// command line, so this is the only thing that says out loud that a run
    /// is going to touch the disk.
    #[arg(long)]
    write: bool,
    /// Say so explicitly: make no changes.
    ///
    /// This is what happens anyway. It exists so that a run can state its
    /// intention rather than rely on the absence of a flag, and so that
    /// saying both things at once is an error rather than a silent winner.
    #[arg(long, conflicts_with = "write")]
    dry_run: bool,
    /// Follow symlinks while walking.
    #[arg(long)]
    follow_links: bool,
    /// Separate the columns with a tab instead of padding, for piping.
    #[arg(long)]
    tabs: bool,
    /// Fail if any folder had to be numbered.
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

#[derive(Args)]
struct InitArgs {
    /// Directory to write findopera.toml into.
    #[arg(value_name = "DIR", default_value = ".")]
    dir: PathBuf,
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
        Command::Organize(args) => cmd_organize(args),
        Command::Fields => cmd_fields(),
        Command::Init(args) => cmd_init(args),
    }
}

fn cmd_organize(args: OrganizeArgs) -> i32 {
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
    let plan = plan::plan(&p.report.markers, &p.recordings, &p.template);
    let listing = plan.listing(args.tabs);

    let destination = p.settings.as_ref().and_then(|c| c.destination.clone());
    let link = p.settings.as_ref().map(|c| c.link).unwrap_or_default();
    let dry_run = !args.write;

    // Without somewhere to build, the naming is still worth having — it is
    // what you are looking at while you settle on a template, before there is
    // any question of a destination.
    let Some(destination) = destination else {
        let mut out = std::io::stdout().lock();
        for line in &listing {
            if !emit(&mut out, format_args!("{line}")) {
                return 0;
            }
        }
        report(&plan, p.require_variants);
        eprintln!(
            "findopera: no destination set, so this is the naming only. Add one to \
             build it:\n\x20   destination = \"/path/to/named\""
        );
        return i32::from(plan.blocked(p.require_variants));
    };

    if let Err(why) = apply::preflight(&plan, &args.root, &destination, link, p.require_variants) {
        report(&plan, p.require_variants);
        eprintln!("findopera: {why}");
        return 2;
    }

    // Say where, before doing it. Nothing on the command line names the
    // destination, so this is the only place it is stated.
    let what = match link {
        config::Link::Symlink => "a link to each folder",
        config::Link::Hardlink => "a hard link to every file",
        config::Link::Copy => "a copy of every file",
    };
    eprintln!(
        "findopera: {} {} in {}",
        if dry_run { "would build" } else { "building" },
        what,
        destination.display()
    );

    let done = apply::apply(&plan, &destination, link, dry_run);
    let mut out = std::io::stdout().lock();
    for (line, entry) in listing.iter().zip(&done.entries) {
        let mark = match &entry.outcome {
            apply::Outcome::Created => '+',
            apply::Outcome::Skipped => ' ',
            apply::Outcome::Conflict(_) | apply::Outcome::Failed(_) => '!',
        };
        if !emit(&mut out, format_args!("{mark} {line}")) {
            return 0;
        }
    }
    for entry in &done.entries {
        if let apply::Outcome::Conflict(why) | apply::Outcome::Failed(why) = &entry.outcome {
            eprintln!("findopera: {}", entry.destination.display());
            eprintln!("    {why}");
        }
    }
    report(&plan, p.require_variants);

    let (made, skipped, trouble) = done.counts();
    eprintln!(
        "findopera: {made} {}, {skipped} already there, {trouble} left alone",
        if dry_run { "to build" } else { "built" }
    );
    // Only worth saying when there is something to write; a run that found
    // everything already in place has nothing to offer.
    if dry_run && made > 0 {
        eprintln!("findopera: nothing was written. To build it, run:");
        eprintln!("    {}", rerun_with_write(&args));
    }
    i32::from(done.troubled() || plan.blocked(p.require_variants))
}

/// Whatever the plan has to say, on stderr, with its indented lines kept.
fn report(plan: &plan::Plan, strict: bool) {
    let lines = plan.report(strict);
    if lines.is_empty() {
        return;
    }
    eprintln!();
    for line in lines {
        if line.starts_with(' ') {
            eprintln!("{line}");
        } else {
            eprintln!("findopera: {line}");
        }
    }
}

/// The same command again, with `--write` on the end.
///
/// Spelled out rather than described, because the arguments that matter may
/// not be the ones the reader typed most recently.
fn rerun_with_write(args: &OrganizeArgs) -> String {
    let quote = |p: &Path| {
        let s = p.display().to_string();
        if s.contains(' ') {
            format!("'{}'", s.replace('\'', r"'\''"))
        } else {
            s
        }
    };
    let mut parts = vec![
        "findopera".to_string(),
        "organize".to_string(),
        quote(&args.root),
    ];
    if let Some(config) = &args.config {
        parts.push("--config".to_string());
        parts.push(quote(config));
    }
    parts.push("--write".to_string());
    parts.join(" ")
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
            eprintln!("findopera: edit the template in there, then run `findopera organize`");
            0
        }
        Err(e) => {
            eprintln!("findopera: cannot write {}: {e}", path.display());
            1
        }
    }
}

/// Everything needed before the naming can be worked out.
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
