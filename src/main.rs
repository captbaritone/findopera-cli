//! `findopera` — render a FindOpera recording through a template.

use findopera::model::FIELDS;
use findopera::{api, to_path, Template};

const USAGE: &str = "\
findopera — render FindOpera recording metadata through a template

Usage:
  findopera <TEMPLATE> <ID>...

  <TEMPLATE>  {{field}} placeholders, `|`-separated fallbacks with a quoted
              literal last, and [optional groups] dropped when a placeholder
              inside them resolves to nothing.
  <ID>        The number in a findopera.com/recording/<id> URL.

Options:
  -f, --fields    List every field a template may use, and exit
  -h, --help      Show this message

Examples:
  findopera '{{composer.lastName}}/{{opera.title}}' 10655
  findopera '{{opera.title}}[ ({{year}})]' 75 10655
  findopera '{{opera.englishTitle|opera.title}}' 10655

Exit codes:
  0  every id rendered
  1  a recording is not in the database, or its render is not a usable path
  2  the template or the arguments are wrong
  3  the API was unreachable or errored
";

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        print!("{USAGE}");
        return if args.is_empty() { 2 } else { 0 };
    }
    if args.iter().any(|a| a == "-f" || a == "--fields") {
        let width = FIELDS.iter().map(|f| f.path.len()).max().unwrap_or(0);
        for f in FIELDS {
            let always = if f.nullable { "" } else { "  (always present)" };
            println!("{:<width$}  {}{always}", f.path, f.description);
        }
        return 0;
    }

    let (template, ids) = args.split_first().expect("checked non-empty");
    if ids.is_empty() {
        eprintln!("findopera: give at least one recording id\n");
        print!("{USAGE}");
        return 2;
    }
    if let Some(bad) = ids
        .iter()
        .find(|id| !id.chars().all(|c| c.is_ascii_digit()))
    {
        eprintln!("findopera: `{bad}` is not a recording id — ids are the number in a findopera.com/recording/<id> URL");
        return 2;
    }

    // Parse first: a bad template is the caller's mistake, and should not cost
    // a network round trip to discover.
    let tmpl = match Template::parse(template, FIELDS) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("findopera: {e}");
            for line in e.underline(template) {
                eprintln!("  {line}");
            }
            if let Some(help) = &e.help {
                eprintln!("  help: {help}");
            }
            return 2;
        }
    };

    let recordings = match api::recordings(api::DEFAULT_ENDPOINT, ids) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("findopera: {e}");
            return 3;
        }
    };

    let mut failed = false;
    for id in ids {
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
