//! `findopera` — name a music library from FindOpera metadata.

use clap::{Args, Parser, Subcommand};
use findopera::config::{self, Config};
use findopera::credentials;
use findopera::model::{Recording, FIELDS};
use findopera::FieldDoc;
use findopera::{api, apply, plan, scan, Template};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

const AFTER_HELP: &str = "\
Each folder holds a text file of notes about the recording in it — cast,
conductor, year, orchestra, and where it came from — kept so that the folder
says what it contains without anything having to be opened. `findopera
annotate` writes one:

  cd '~/Music/Sosarme' && findopera annotate 10655

The id in its name is also how this program recognises the folder later, which
is what everything else here is built on.

Start with `findopera init`, then `findopera organize --help`.

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
    about = "Organize a library of opera recordings, using metadata from findopera.com",
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
Walk a library for notes files, work out what each recording's folder should
be called, and — with --write — build a tree of those folders at the
destination in the settings file.

NOTES FILES

Each folder holds a text file of notes about its recording — the cast, the
conductor, the year, the orchestra, and a link back to where it came from.
They are meant to be read: a folder with one in it says what it holds without
anything having to be opened, and keeps saying so on a disk that outlives this
program.

  findopera annotate 10655

writes one into the current folder, named as findopera.com names it:

  Sosarme, Re di Media-2026-Angioloni [findopera-10655].txt

That name is also how this program recognises the folder again. It looks for
`findopera-<id>` in it, so the file can be renamed freely as long as that part
survives. A bare `10655.txt` is not enough — a number and a .txt is what a
track listing or a year looks like, and the `findopera-` is what says the
number means a recording.

Only the name is matched on, never the contents, so a folder can be claimed by
hand — `touch 'findopera-10655.txt'` works — but that leaves a file with
nothing in it for anyone to read.

One folder may hold several of these; a box set covering several operas is
listed once for each recording in it. Where two folders hold the *same*
recording — a FLAC rip and an MP3 rip of one performance — nothing in the
recording tells them apart, so put a word after the id to say which is which:

  findopera annotate 10655 --variant flac
  findopera annotate 10655 --variant mp3

A template picks that word up as {{variant}}.

BUILDING

Nothing is written without --write. Nothing is ever deleted or overwritten.

With no destination set the folders are still worked out and shown, which is
what you want while you are still settling on a template.",
        after_help = "\
Examples:
  findopera organize ~/Music
  findopera organize ~/Music --write
  findopera organize ~/Music -t '{{opera.title}}[ ({{year}})]'
  findopera organize ~/Music --config ~/Music/by-conductor.toml"
    )]
    Organize(OrganizeArgs),

    /// Write a recording's notes into a folder.
    #[command(
        long_about = "\
Fetch a recording's notes from findopera.com and write them into a folder, so
that the folder says what it holds.

  cd '~/Music/Sosarme, Re di Media' && findopera annotate 10655

The id is the number in the recording's address on findopera.com. The file is
named as the site names it — cast and conductor and year in the title, and the
id in brackets on the end:

  Sosarme, Re di Media-2026-Angioloni [findopera-10655].txt

That name is taken from the server rather than assembled here, so there is one
authority for it. It is also what `organize` looks for later, which is why the
`findopera-<id>` part has to survive any renaming.

Where a folder holds one of two rips of the same recording, --variant says
which, and a template can pick it up as {{variant}}:

  findopera annotate 10655 --variant flac

Nothing is overwritten without --force.",
        after_help = "\
Examples:
  findopera annotate 10655
  findopera annotate 10655 ~/Music/Sosarme
  findopera annotate 10655 --variant flac"
    )]
    Annotate(AnnotateArgs),

    /// List every field a template may use, and the syntax.
    Fields,

    /// Send a GraphQL query to findopera.com and print the response.
    #[command(
        long_about = "\
Send a GraphQL document to findopera.com and print the whole response as
JSON, for looking up ids, checking what a recording holds, or anything the
other commands do not cover.

The query may be an argument, a file, or standard input:

  findopera graphql '{ searchOperas(query: \"Tosca\", first: 3) { id title } }'
  findopera graphql --file lookup.graphql
  echo '{ ... }' | findopera graphql

Standard input is the one to reach for from a script: a GraphQL document is
full of braces and quotes, and often names people, so getting it through a
shell intact is harder than it looks.

The response goes to stdout exactly as it arrived, so it can be piped to jq.
If the server reports errors it is still printed — an error names the field it
objected to, which is the useful part — but the messages are repeated on
stderr and the exit status is 3, so a script cannot mistake a refusal for an
answer.

Requests are anonymous, which is enough to read. Anything needing an account,
mutations included, will be refused by the server.",
        after_help = "\
Examples:
  findopera graphql '{ getRecordingById(id: \"10655\") { id year } }'
  findopera graphql --file q.graphql --variables '{\"ids\": [\"10655\"]}'
  echo '{ listSingers(first: 3) { id lastName } }' | findopera graphql | jq ."
    )]
    Graphql(GraphqlArgs),

    /// Print the GraphQL schema, or one type from it.
    #[command(
        long_about = "\
Fetch the schema findopera.com is serving, and print it as SDL.

Fetched rather than built in, because the server gains fields between releases
of this program: what it prints is what the server will actually answer to
today, not what this binary was compiled against.

The whole schema is long. Naming a type prints just that one, which is usually
what you wanted:

  findopera schema Query        what can be asked for
  findopera schema Mutation     what can be changed
  findopera schema Recording",
        after_help = "\
Examples:
  findopera schema
  findopera schema Mutation
  findopera schema | grep -n 'search'"
    )]
    Schema(SchemaArgs),

    /// Send a note to whoever maintains findopera.com.
    #[command(long_about = "\
Say something to whoever maintains findopera.com — a recording that is wrong,
a field this cannot express, something that went badly.

The message may be an argument or standard input, so a session that just went
wrong can be sent as it stands:

  findopera feedback \"the template docs do not mention escaping\"
  findopera organize ~/Music 2>&1 | findopera feedback --kind bug

Add --email if you want an answer; nothing is sent to it otherwise, and it is
not published. The version of this program goes along with the message, since
the first thing anyone reading it will want to know is which one you have.

This works without a token, on the reasoning that being unable to get one is
exactly the sort of thing worth reporting.")]
    Feedback(FeedbackArgs),

    /// Store a token, so that requests say who is making them.
    #[command(long_about = "\
Read a token from standard input and keep it, so that later runs are made as
you rather than as nobody.

  findopera login < token.txt
  pbpaste | findopera login

It is read from standard input rather than taken as an argument so that it
does not end up in the shell's history, or visible to anyone who can list
processes on this machine.

It is kept in your own configuration directory — not in findopera.toml, which
lives inside the library being organized and is walked, linked and synced
along with it.

For anything unattended, set FINDOPERA_TOKEN instead and store nothing.

With --new there is nothing to paste: findopera.com issues one on the spot.
It asks for no account and no proof of who you are — the token exists so that
your requests can be told apart from everyone else's, not so that you can be
identified. Edits made with it are recorded under a name it gives you.

  findopera login --new
  findopera login --new --label laptop --email you@example.com

--email is optional and never verified. It is somewhere to reach you if
something you are doing turns out to be blocked, and is not published.")]
    Login(LoginArgs),

    /// Forget the stored token.
    Logout,

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
    /// Token to identify as. Overrides the environment and the stored one.
    ///
    /// Prefer `findopera login`, or the FINDOPERA_TOKEN environment variable:
    /// a token on the command line is visible to anyone who can list
    /// processes, and is kept in the shell's history.
    #[arg(long, value_name = "TOKEN")]
    token: Option<String>,
    /// GraphQL endpoint.
    #[arg(long, default_value = api::DEFAULT_ENDPOINT, value_name = "URL")]
    endpoint: String,
}

#[derive(Args)]
struct GraphqlArgs {
    /// The query. Omit to read it from standard input.
    #[arg(value_name = "QUERY")]
    query: Option<String>,
    /// Read the query from a file instead. `-` means standard input.
    #[arg(long, short = 'f', value_name = "FILE", conflicts_with = "query")]
    file: Option<PathBuf>,
    /// Variables, as a JSON object.
    #[arg(long, value_name = "JSON")]
    variables: Option<String>,
    /// Print the response on one line.
    #[arg(long)]
    compact: bool,
    /// Token to identify as. Overrides the environment and the stored one.
    ///
    /// Prefer `findopera login`, or the FINDOPERA_TOKEN environment variable:
    /// a token on the command line is visible to anyone who can list
    /// processes, and is kept in the shell's history.
    #[arg(long, value_name = "TOKEN")]
    token: Option<String>,
    /// GraphQL endpoint.
    #[arg(long, default_value = api::DEFAULT_ENDPOINT, value_name = "URL")]
    endpoint: String,
}

#[derive(Args)]
struct AnnotateArgs {
    /// The recording's id, from its address on findopera.com.
    #[arg(value_name = "ID")]
    id: String,
    /// Folder to write into.
    #[arg(value_name = "DIR", default_value = ".")]
    dir: PathBuf,
    /// Which rip this folder holds, when there is more than one.
    #[arg(long, value_name = "NAME")]
    variant: Option<String>,
    /// Replace a file that is already there.
    #[arg(long)]
    force: bool,
    /// GraphQL endpoint. The notes are fetched from the same server.
    #[arg(long, default_value = api::DEFAULT_ENDPOINT, value_name = "URL")]
    endpoint: String,
    /// Token to identify as.
    #[arg(long, value_name = "TOKEN")]
    token: Option<String>,
}

#[derive(Args)]
struct FeedbackArgs {
    /// What you want to say. Omit to read it from standard input.
    #[arg(value_name = "MESSAGE")]
    message: Option<String>,
    /// What this is about.
    #[arg(long, value_name = "KIND", default_value = "general",
          value_parser = ["bug", "suggestion", "error", "recording", "album", "general"])]
    kind: String,
    /// Where to reach you, if you want an answer.
    #[arg(long, value_name = "ADDRESS")]
    email: Option<String>,
    /// What you were doing, or a link to what you were looking at.
    #[arg(long, value_name = "TEXT")]
    about: Option<String>,
    /// GraphQL endpoint.
    #[arg(long, default_value = api::DEFAULT_ENDPOINT, value_name = "URL")]
    endpoint: String,
    /// Token to identify as.
    #[arg(long, value_name = "TOKEN")]
    token: Option<String>,
}

#[derive(Args)]
struct LoginArgs {
    /// Ask findopera.com for a new token instead of reading one.
    #[arg(long)]
    new: bool,
    /// What to call the new token, so it can be told from your others.
    #[arg(long, value_name = "TEXT", requires = "new")]
    label: Option<String>,
    /// Optional address to reach you on, if something you do gets blocked.
    ///
    /// Never verified, never published, and not a login. It exists so that
    /// someone stuck against a limit or a bug at the other end can be told
    /// about it. Leave it out and everything still works.
    #[arg(long, value_name = "ADDRESS", requires = "new")]
    email: Option<String>,
    /// GraphQL endpoint.
    #[arg(long, default_value = api::DEFAULT_ENDPOINT, value_name = "URL")]
    endpoint: String,
}

#[derive(Args)]
struct SchemaArgs {
    /// Print only this type. Omit for the whole schema.
    #[arg(value_name = "TYPE")]
    name: Option<String>,
    /// Token to identify as. Overrides the environment and the stored one.
    ///
    /// Prefer `findopera login`, or the FINDOPERA_TOKEN environment variable:
    /// a token on the command line is visible to anyone who can list
    /// processes, and is kept in the shell's history.
    #[arg(long, value_name = "TOKEN")]
    token: Option<String>,
    /// GraphQL endpoint. The schema is fetched from the same server.
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
        Command::Annotate(args) => cmd_annotate(args),
        Command::Fields => cmd_fields(),
        Command::Graphql(args) => cmd_graphql(args),
        Command::Schema(args) => cmd_schema(args),
        Command::Feedback(args) => cmd_feedback(args),
        Command::Login(args) => cmd_login(args),
        Command::Logout => cmd_logout(),
        Command::Init(args) => cmd_init(args),
    }
}

/// The client for this run, with whatever identity it has.
fn client(endpoint: &str, token: Option<&String>) -> Result<api::Client, i32> {
    match credentials::resolve(token.map(String::as_str)) {
        Ok(token) => Ok(api::Client::new(endpoint, token)),
        Err(why) => {
            eprintln!("findopera: {why}");
            Err(2)
        }
    }
}

fn cmd_annotate(args: AnnotateArgs) -> i32 {
    let id = args.id.trim().trim_start_matches('#');
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
        eprintln!("findopera: `{}` is not a recording id", args.id);
        eprintln!("  help: the id is the number in the address, as in");
        eprintln!("        https://findopera.com/recording/10655");
        return 2;
    }

    let api = match client(&args.endpoint, args.token.as_ref()) {
        Ok(c) => c,
        Err(code) => return code,
    };
    let notes = match api.notes(id) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("findopera: {e}");
            return 3;
        }
    };

    let filename = match args.variant.as_deref() {
        None => notes.filename.clone(),
        Some(variant) => match with_variant(&notes.filename, id, variant) {
            Some(name) => name,
            None => {
                eprintln!(
                    "findopera: cannot put a variant in `{}` — it does not carry `findopera-{id}`",
                    notes.filename
                );
                return 3;
            }
        },
    };

    if !args.dir.is_dir() {
        eprintln!("findopera: {} is not a folder", args.dir.display());
        return 2;
    }
    let path = args.dir.join(&filename);
    if path.exists() && !args.force {
        eprintln!("findopera: {} is already there", path.display());
        eprintln!("  help: pass --force to replace it");
        return 1;
    }
    if let Err(e) = std::fs::write(&path, &notes.body) {
        eprintln!("findopera: cannot write {}: {e}", path.display());
        return 1;
    }
    println!("{}", path.display());
    0
}

/// Put a variant into a name, right after the id it belongs to.
///
/// Inserted rather than appended, because the id is usually in brackets at the
/// end and a variant outside them would not be read back: `scan` takes what
/// follows `findopera-<id>` and trims the punctuation off both ends, so
/// `[findopera-10655 flac]` gives `flac` while `[findopera-10655] flac` gives
/// nothing.
fn with_variant(filename: &str, id: &str, variant: &str) -> Option<String> {
    let variant = variant.trim();
    let token = format!("findopera-{id}");
    let at = filename.find(&token)? + token.len();
    Some(format!("{} {variant}{}", &filename[..at], &filename[at..]))
}

fn cmd_feedback(args: FeedbackArgs) -> i32 {
    let message = match &args.message {
        Some(m) => m.clone(),
        None => match std::io::read_to_string(std::io::stdin()) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("findopera: cannot read the message from standard input: {e}");
                return 2;
            }
        },
    };
    if message.trim().is_empty() {
        eprintln!("findopera: nothing to send");
        eprintln!("  help: findopera feedback \"what went wrong\"");
        return 2;
    }

    // Which version this came from is the first thing anyone reading it will
    // want, and the last thing anyone thinks to include.
    let about = match &args.about {
        Some(text) => format!("{} — {text}", api::USER_AGENT),
        None => api::USER_AGENT.to_string(),
    };

    let api = match client(&args.endpoint, args.token.as_ref()) {
        Ok(c) => c,
        Err(code) => return code,
    };
    let payload = match api.post(
        "mutation Say($kind: FeedbackKind!, $message: String!, $email: String, $url: String) {\n\
         \x20 submitFeedback(kind: $kind, message: $message, email: $email, url: $url)\n\
         }",
        Some(serde_json::json!({
            "kind": args.kind,
            "message": message,
            "email": args.email,
            "url": about,
        })),
    ) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("findopera: {e}");
            return 3;
        }
    };
    if let Some(said) = api::refusal(&payload) {
        eprintln!("findopera: {said}");
        return 3;
    }

    eprintln!("findopera: sent — thank you");
    if args.email.is_none() {
        eprintln!("findopera: no reply is possible; pass --email if you want one");
    }
    0
}

/// Ask the server for a token, and say who it made you.
fn request_token(args: &LoginArgs) -> Result<(String, String), i32> {
    // Anonymous by necessity: this is how a caller stops being anonymous, so
    // needing a token to ask for one would be a closed loop.
    let api = api::Client::new(&args.endpoint, None);
    let payload = match api.post(
        "mutation NewToken($label: String, $email: String) {\n\
         \x20 createAccessToken(label: $label, email: $email) { token username }\n\
         }",
        Some(serde_json::json!({ "label": args.label, "email": args.email })),
    ) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("findopera: {e}");
            return Err(3);
        }
    };
    if let Some(said) = api::refusal(&payload) {
        eprintln!("findopera: {said}");
        return Err(3);
    }
    let issued = &payload["data"]["createAccessToken"];
    match (issued["token"].as_str(), issued["username"].as_str()) {
        (Some(token), Some(username)) => Ok((token.to_string(), username.to_string())),
        _ => {
            eprintln!("findopera: the server issued no token");
            Err(3)
        }
    }
}

fn cmd_login(args: LoginArgs) -> i32 {
    if args.new {
        let (token, username) = match request_token(&args) {
            Ok(pair) => pair,
            Err(code) => return code,
        };
        return match credentials::store(&token) {
            Ok(path) => {
                println!("{}", path.display());
                eprintln!("findopera: your edits will be recorded as {username}");
                if args.email.is_none() {
                    eprintln!(
                        "findopera: nothing to reach you on. `--email` is optional, and only used \
                     if\n\x20           something you are doing turns out to be blocked."
                    );
                }
                // Only the server can show the token, and only once. Saying so
                // here is cheaper than someone discovering it later.
                eprintln!(
                    "findopera: the token itself is in that file and nowhere else — \
                     findopera.com keeps only a hash of it"
                );
                0
            }
            Err(why) => {
                eprintln!("findopera: {why}");
                1
            }
        };
    }
    let token = match std::io::read_to_string(std::io::stdin()) {
        Ok(t) => t.trim().to_string(),
        Err(e) => {
            eprintln!("findopera: cannot read the token from standard input: {e}");
            return 2;
        }
    };
    if token.is_empty() {
        eprintln!("findopera: no token given");
        eprintln!("  help: findopera login < token.txt");
        return 2;
    }
    match credentials::store(&token) {
        Ok(path) => {
            // The path, not the token. Nothing should print the token, and the
            // path is what someone would want in order to remove it by hand.
            println!("{}", path.display());
            eprintln!("findopera: requests will now say who is making them");
            0
        }
        Err(why) => {
            eprintln!("findopera: {why}");
            1
        }
    }
}

fn cmd_logout() -> i32 {
    match credentials::forget() {
        Ok(true) => {
            eprintln!("findopera: the stored token is gone");
            0
        }
        Ok(false) => {
            eprintln!("findopera: there was no stored token");
            0
        }
        Err(why) => {
            eprintln!("findopera: {why}");
            1
        }
    }
}

fn cmd_organize(args: OrganizeArgs) -> i32 {
    let api = match client(&args.endpoint, args.token.as_ref()) {
        Ok(c) => c,
        Err(code) => return code,
    };
    let p = match prepare(
        &args.root,
        args.config.as_ref(),
        args.template.as_ref(),
        args.follow_links,
        args.require_variants,
        &api,
    ) {
        Ok(p) => p,
        Err(code) => return code,
    };
    let plan = plan::plan(&p.report.markers, &p.recordings, &p.template);
    let listing = plan.listing(args.tabs);

    let destination = p.settings.as_ref().and_then(|c| c.destination.clone());
    let link = p.settings.as_ref().map(|c| c.link).unwrap_or_default();
    let dry_run = !args.write;

    // Without somewhere to build, the folder names are still worth having — they are
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
            "findopera: no destination set, so these are only the folder names. Add one to \
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

fn cmd_graphql(args: GraphqlArgs) -> i32 {
    let query = match read_query(&args) {
        Ok(q) => q,
        Err(why) => {
            eprintln!("findopera: {why}");
            return 2;
        }
    };
    if query.trim().is_empty() {
        eprintln!("findopera: no query given");
        return 2;
    }

    // Parsed here rather than passed through as text, so that a typo in the
    // variables is caught before the round trip and reported as the caller's
    // mistake instead of the server's.
    let variables = match args.variables.as_deref().map(serde_json::from_str) {
        None => None,
        Some(Ok(v)) => Some(v),
        Some(Err(e)) => {
            eprintln!("findopera: --variables is not valid JSON: {e}");
            return 2;
        }
    };

    let api = match client(&args.endpoint, args.token.as_ref()) {
        Ok(c) => c,
        Err(code) => return code,
    };
    let payload = match api.post(&query, variables) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("findopera: {e}");
            return 3;
        }
    };

    // Printed before anything is said about it. A refusal names the field it
    // objected to and where, which is worth having whichever way this ends.
    let rendered = if args.compact {
        payload.to_string()
    } else {
        serde_json::to_string_pretty(&payload).unwrap_or_else(|_| payload.to_string())
    };
    if !emit(&mut std::io::stdout().lock(), format_args!("{rendered}")) {
        return 0;
    }

    match api::refusal(&payload) {
        Some(said) => {
            eprintln!("findopera: {said}");
            // Anonymous is enough to read, so a refusal is where the lack of a
            // token first shows up — and "Unauthorized" is not obviously about
            // this program rather than about the account behind it.
            if !api.is_identified() {
                eprintln!("  note: this request was anonymous. Changing anything needs a token —");
                eprintln!("        see `findopera login --help`");
            }
            3
        }
        None => 0,
    }
}

/// The query text, from wherever this run keeps it.
fn read_query(args: &GraphqlArgs) -> Result<String, String> {
    match (&args.query, &args.file) {
        (Some(q), _) => Ok(q.clone()),
        (None, Some(f)) if f.as_os_str() != "-" => {
            std::fs::read_to_string(f).map_err(|e| format!("cannot read {}: {e}", f.display()))
        }
        // No query and no file is not a mistake: it is the pipe.
        _ => std::io::read_to_string(std::io::stdin())
            .map_err(|e| format!("cannot read the query from standard input: {e}")),
    }
}

fn cmd_schema(args: SchemaArgs) -> i32 {
    let api = match client(&args.endpoint, args.token.as_ref()) {
        Ok(c) => c,
        Err(code) => return code,
    };
    let sdl = match api.schema() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("findopera: {e}");
            return 3;
        }
    };
    let mut out = std::io::stdout().lock();
    let Some(name) = args.name else {
        emit(&mut out, format_args!("{}", sdl.trim_end()));
        return 0;
    };
    match definition(&sdl, &name) {
        Some(block) => {
            emit(&mut out, format_args!("{block}"));
            0
        }
        None => {
            eprintln!("findopera: the schema has no `{name}`");
            // The names are the schema's own, so the only honest suggestion is
            // to go and look at them.
            eprintln!("  help: `findopera schema` prints all of it");
            2
        }
    }
}

/// One named definition out of an SDL document, with its leading doc comment.
///
/// SDL as served is formatted one definition per block, opening on a line of
/// its own and closing on a `}` in the first column, so that is what this
/// looks for rather than parsing the language.
fn definition<'a>(sdl: &'a str, name: &str) -> Option<&'a str> {
    let lines: Vec<&str> = sdl.lines().collect();
    let keywords = [
        "type",
        "input",
        "enum",
        "interface",
        "union",
        "scalar",
        "directive",
    ];
    let start = lines.iter().position(|line| {
        let Some(rest) = keywords.iter().find_map(|k| line.strip_prefix(*k)) else {
            return false;
        };
        rest.strip_prefix(' ')
            .and_then(|r| r.strip_prefix(name))
            // What follows the name has to be a boundary, or `Recording` would
            // match `RecordingURL`.
            .is_some_and(|after| !after.starts_with(|c: char| c.is_alphanumeric() || c == '_'))
    })?;

    // Take the description above it too: in this schema that is where a field
    // says what it means, which is the reason to be reading it at all.
    let mut first = start;
    while first > 0 {
        let above = lines[first - 1].trim_start();
        if above.starts_with('"') || above.starts_with('#') {
            first -= 1;
        } else {
            break;
        }
    }

    // A single-line definition — `scalar Date` — closes itself.
    let mut last = start;
    if lines[start].contains('{') {
        last = (start + 1..lines.len()).find(|&i| lines[i].starts_with('}'))?;
    }
    let head: usize = lines[..first].iter().map(|l| l.len() + 1).sum();
    let len: usize = lines[first..=last].iter().map(|l| l.len() + 1).sum();
    Some(sdl[head..head + len].trim_end())
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

/// Everything needed before the folders can be worked out.
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
    api: &api::Client,
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

    let patterns = settings
        .as_ref()
        .map(|c| c.ignore.clone())
        .unwrap_or_default();
    let ignore = match scan::Ignore::new(&patterns) {
        Ok(i) => i,
        Err(why) => {
            eprintln!("findopera: {why}");
            return Err(2);
        }
    };
    let report = scan::scan(root, follow_links, &ignore);
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
    let recordings = match api.recordings(&ids) {
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

#[cfg(test)]
mod tests {
    use super::{definition, with_variant};

    #[test]
    fn a_variant_goes_inside_the_brackets_with_the_id() {
        // `scan` reads what follows `findopera-<id>` and trims punctuation off
        // both ends, so a variant inside the brackets comes back and one after
        // them does not. Getting this backwards would write files that look
        // right and are silently unreadable.
        assert_eq!(
            with_variant(
                "Sosarme-2026-Angioloni [findopera-10655].txt",
                "10655",
                "flac"
            )
            .as_deref(),
            Some("Sosarme-2026-Angioloni [findopera-10655 flac].txt")
        );
    }

    #[test]
    fn a_name_without_the_id_cannot_take_a_variant() {
        assert_eq!(with_variant("something-else.txt", "10655", "flac"), None);
    }

    #[test]
    fn a_variant_is_trimmed_before_it_is_inserted() {
        assert_eq!(
            with_variant("[findopera-75].txt", "75", "  mp3  ").as_deref(),
            Some("[findopera-75 mp3].txt")
        );
    }

    const SDL: &str = concat!(
        "\"\"\"A recorded performance.\"\"\"\n",
        "type Recording {\n",
        "  id: Int!\n",
        "}\n",
        "\n",
        "type RecordingURL {\n",
        "  url: String!\n",
        "}\n",
        "\n",
        "scalar Date\n",
        "\n",
        "input UpdateSingerInput {\n",
        "  lastName: String\n",
        "}\n",
    );

    #[test]
    fn a_type_comes_back_whole() {
        let block = definition(SDL, "Recording").expect("Recording is there");
        assert!(
            block.starts_with("\"\"\"A recorded performance."),
            "got: {block}"
        );
        assert!(block.ends_with('}'), "got: {block}");
        assert!(block.contains("id: Int!"), "got: {block}");
    }

    #[test]
    fn a_longer_name_starting_the_same_way_is_not_it() {
        // Asking for Recording must not hand back RecordingURL, nor stop at it.
        let block = definition(SDL, "Recording").expect("Recording is there");
        assert!(!block.contains("url: String!"), "got: {block}");
        let other = definition(SDL, "RecordingURL").expect("RecordingURL is there");
        assert!(other.contains("url: String!"), "got: {other}");
    }

    #[test]
    fn a_definition_without_a_body_is_just_its_line() {
        assert_eq!(definition(SDL, "Date"), Some("scalar Date"));
    }

    #[test]
    fn inputs_are_findable_too() {
        // Anything writing a mutation needs these, not just the output types.
        let block = definition(SDL, "UpdateSingerInput").expect("the input is there");
        assert!(block.starts_with("input UpdateSingerInput"), "got: {block}");
    }

    #[test]
    fn a_name_the_schema_does_not_have_is_absent() {
        assert_eq!(definition(SDL, "Nope"), None);
    }
}
