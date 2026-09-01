//! `findopera` — render a FindOpera recording through a template.

use clap::Parser;
use findopera::model::FIELDS;
use findopera::{api, to_path, Template};

const AFTER_HELP: &str = "\
Examples:
  findopera '{{composer.lastName}}/{{opera.title}}' 10655
  findopera '{{opera.title}}[ ({{year}})]' 75 10655
  findopera '{{opera.englishTitle|opera.title}}' 10655
  findopera --fields

Exit codes:
  0  every id rendered
  1  a recording is not in the database, or its render is not a usable path
  2  the template or the arguments are wrong
  3  the API was unreachable or errored

Results go to stdout, one line per recording; everything else to stderr.";

#[derive(Parser)]
#[command(
    name = "findopera",
    version,
    about = "Render FindOpera recording metadata through a template",
    long_about = "\
Render FindOpera recording metadata through a template.

The template is checked against the schema before anything is fetched, so a
mistyped field — or one that may be absent, used without a fallback — fails
without a network round trip.",
    after_help = AFTER_HELP
)]
struct Cli {
    /// Template. `{{field}}` placeholders, `|`-separated fallbacks with a
    /// quoted literal last, and `[optional groups]` dropped when a placeholder
    /// inside them resolves to nothing.
    #[arg(value_name = "TEMPLATE", required_unless_present = "fields")]
    template: Option<String>,

    /// Recording ids — the number in a findopera.com/recording/<id> URL.
    #[arg(value_name = "ID", required_unless_present = "fields", value_parser = recording_id)]
    ids: Vec<String>,

    /// List every field a template may use, and exit.
    #[arg(short, long, conflicts_with_all = ["template", "ids"])]
    fields: bool,

    /// GraphQL endpoint.
    #[arg(long, default_value = api::DEFAULT_ENDPOINT, value_name = "URL")]
    endpoint: String,
}

fn recording_id(s: &str) -> Result<String, String> {
    if !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()) {
        return Ok(s.to_string());
    }
    Err("ids are the number in a findopera.com/recording/<id> URL".into())
}

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let cli = Cli::parse();

    if cli.fields {
        let width = FIELDS.iter().map(|f| f.path.len()).max().unwrap_or(0);
        for f in FIELDS {
            let always = if f.nullable { "" } else { "  (always present)" };
            println!("{:<width$}  {}{always}", f.path, f.description);
        }
        return 0;
    }

    let template = cli.template.expect("clap requires it without --fields");

    // Parse first: a bad template is the caller's mistake, and should not cost
    // a network round trip to discover.
    let tmpl = match Template::parse(&template, FIELDS) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("findopera: {e}");
            for line in e.underline(&template) {
                eprintln!("  {line}");
            }
            if let Some(help) = &e.help {
                eprintln!("  help: {help}");
            }
            return 2;
        }
    };

    let recordings = match api::recordings(&cli.endpoint, &cli.ids) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("findopera: {e}");
            return 3;
        }
    };

    let mut failed = false;
    for id in &cli.ids {
        let Some(rec) = recordings.get(id) else {
            eprintln!("findopera: recording {id} is not in the FindOpera database");
            failed = true;
            continue;
        };
        // Rendering cannot fail; only judging the result as a path can.
        let rendered = tmpl.render(rec);
        match to_path(&rendered) {
            Ok(segments) => println!("{}", segments.join("/")),
            Err(e) => {
                eprintln!("findopera: recording {id} {e}");
                failed = true;
            }
        }
    }
    i32::from(failed)
}
