//! `findopera` — name a music library from FindOpera metadata.

use clap::{Args, Parser, Subcommand};
use findopera::config::{self, Config};
use findopera::credentials;
use findopera::model::{crud, Recording, FIELDS};
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

    /// Show one record whole.
    #[command(long_about = "\
Fetch a record and print everything worth knowing about it, including the
records it points at.

  findopera get recording 264
  findopera get singer 133 --json

`findopera describe` lists the types.")]
    Get(GetArgs),

    /// Add a record.
    #[command(long_about = "\
Add a record from a JSON object on standard input, or from a file.

  echo '{\"firstName\":\"Maria\",\"lastName\":\"Callas\"}' \\
    | findopera create singer -m 'https://en.wikipedia.org/wiki/Maria_Callas'

`findopera describe singer` says which fields it takes and which are required;
--json there gives the same as a JSON Schema. The fields are checked here
before anything is sent, so a misspelling is answered in the terms you used
rather than as a GraphQL error about a type you never mentioned.

Every change needs -m: a source, ideally a URL, and enough context for someone
reading the history later to judge it. The server requires one and will refuse
without it.")]
    Create(CreateArgs),

    /// Change a record.
    #[command(long_about = "\
Change some fields of a record, from a JSON object on standard input or a
file. Only the fields present are touched.

  echo '{\"died\":1977}' | findopera edit singer 133 -m 'https://...'

Every change needs -m, as `create` does.")]
    Edit(EditArgs),

    /// Remove a record.
    #[command(long_about = "\
Remove a record.

Nothing here is really destroyed — every change is versioned and can be
reverted — but this is still the only command that takes something away, so it
asks for --yes as well as a reason.")]
    Delete(DeleteArgs),

    /// List the types, or say what one holds.
    #[command(long_about = "\
With no argument, list every type these commands work on.

With one, say what that type holds and what it takes to make one:

  findopera describe singer
  findopera describe singer --json    a JSON Schema for the create input

The JSON form is a schema in the ordinary sense — draft 2020-12 — so anything
that already validates JSON can check an input before sending it.")]
    Describe(DescribeArgs),

    /// Look up an id by name.
    #[command(
        long_about = "\
Find the id of a recording, or of the people and works one is made of.

Everything else here takes an id. This is how you get one.

  findopera search recording tosca --singer callas
  findopera search singer callas
  findopera search opera tosca
  findopera search character scarpia

The first column is the id, so a result can be handed straight to another
command:

  findopera search recording tosca --tabs | head -1 | cut -f1 | xargs findopera annotate

Each kind takes what applies to it. A recording can be narrowed by more than
its title — --singer may be repeated, --year matches within two years either
side, and --upc takes the barcode off the box in whatever form it is printed.
The rest take a name and nothing else, so `findopera search singer --help` is
short.

Searching is case and accent insensitive and matches part of a name, so
`boheme` finds `La Bohème`.

Only the first --first matches are shown, 10 by default. When there are more,
it says so — there is no way to page through them, because narrowing is the
better answer, but a full page should never be mistaken for the whole of it.

Composers, singers, conductors and characters are what a recording is made of,
and their ids are what it takes to describe a new one.",
        after_help = "\
Examples:
  findopera search recording boheme --singer pavarotti --singer freni
  findopera search recording tosca --conductor 'de sabata' --year 1953
  findopera search conductor karajan --first 10"
    )]
    Search(SearchArgs),

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

    /// The template language: its syntax, and every field.
    #[command(long_about = "\
Print the template language — the syntax, and every field a template may use.

A template says what a folder should be called. `schema` does the same job for
the API: both print the reference for a language this program understands.

  findopera template
  findopera template | grep singer

The list goes to stdout and the syntax to stderr, so grepping for a field
works without the preamble getting in the way.")]
    Template,

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
    #[arg(long, conflicts_with = "json")]
    tabs: bool,
    /// Print the plan as JSON.
    #[arg(long)]
    json: bool,
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
struct GetArgs {
    /// What kind of record.
    #[arg(value_name = "TYPE")]
    kind: String,
    /// Its id.
    #[arg(value_name = "ID")]
    id: String,
    /// Print it as JSON.
    #[arg(long)]
    json: bool,
    #[arg(long, default_value = api::DEFAULT_ENDPOINT, value_name = "URL")]
    endpoint: String,
    #[arg(long, value_name = "TOKEN")]
    token: Option<String>,
}

#[derive(Args)]
struct CreateArgs {
    /// What kind of record.
    #[arg(value_name = "TYPE")]
    kind: String,
    /// A JSON object. Omit to read it from standard input.
    #[arg(long, short = 'i', value_name = "FILE")]
    input: Option<PathBuf>,
    /// Source and context for this change, for the record's history.
    #[arg(long, short = 'm', value_name = "TEXT")]
    message: String,
    /// Print the result as JSON.
    #[arg(long)]
    json: bool,
    #[arg(long, default_value = api::DEFAULT_ENDPOINT, value_name = "URL")]
    endpoint: String,
    #[arg(long, value_name = "TOKEN")]
    token: Option<String>,
}

#[derive(Args)]
struct EditArgs {
    /// What kind of record.
    #[arg(value_name = "TYPE")]
    kind: String,
    /// Its id.
    #[arg(value_name = "ID")]
    id: String,
    /// A JSON object of the fields to change. Omit to read from standard input.
    #[arg(long, short = 'i', value_name = "FILE")]
    input: Option<PathBuf>,
    /// Source and context for this change, for the record's history.
    #[arg(long, short = 'm', value_name = "TEXT")]
    message: String,
    /// Print the result as JSON.
    #[arg(long)]
    json: bool,
    #[arg(long, default_value = api::DEFAULT_ENDPOINT, value_name = "URL")]
    endpoint: String,
    #[arg(long, value_name = "TOKEN")]
    token: Option<String>,
}

#[derive(Args)]
struct DeleteArgs {
    /// What kind of record.
    #[arg(value_name = "TYPE")]
    kind: String,
    /// Its id.
    #[arg(value_name = "ID")]
    id: String,
    /// Source and context for this change, for the record's history.
    #[arg(long, short = 'm', value_name = "TEXT")]
    message: String,
    /// Say so out loud. Nothing is removed without it.
    #[arg(long)]
    yes: bool,
    /// Print the result as JSON.
    #[arg(long)]
    json: bool,
    #[arg(long, default_value = api::DEFAULT_ENDPOINT, value_name = "URL")]
    endpoint: String,
    #[arg(long, value_name = "TOKEN")]
    token: Option<String>,
}

#[derive(Args)]
struct DescribeArgs {
    /// The type. Omit to list them all.
    #[arg(value_name = "TYPE")]
    kind: Option<String>,
    /// Print a JSON Schema for the create input.
    #[arg(long)]
    json: bool,
}

/// What every search shares, whatever it is looking for.
#[derive(Args)]
struct Looking {
    /// The name, or part of one.
    #[arg(value_name = "QUERY", default_value = "")]
    query: Vec<String>,
    /// How many results, up to 200.
    #[arg(long, default_value_t = 10, value_name = "N")]
    first: u32,
    /// Separate the columns with a tab instead of padding, for piping.
    #[arg(long, conflicts_with = "json")]
    tabs: bool,
    /// Print the results as JSON.
    #[arg(long)]
    json: bool,
    /// GraphQL endpoint.
    #[arg(long, default_value = api::DEFAULT_ENDPOINT, value_name = "URL")]
    endpoint: String,
    /// Token to identify as.
    #[arg(long, value_name = "TOKEN")]
    token: Option<String>,
}

#[derive(Args)]
struct SearchRecordingArgs {
    #[command(flatten)]
    looking: Looking,
    /// A singer on the recording. May be repeated; all must appear.
    #[arg(long, value_name = "NAME")]
    singer: Vec<String>,
    /// The conductor of the recording.
    #[arg(long, value_name = "NAME")]
    conductor: Option<String>,
    /// Recorded within two years of this.
    #[arg(long, value_name = "YEAR")]
    year: Option<i64>,
    /// The barcode off the box.
    ///
    /// However it is written: spaces and dashes are ignored, and the 12, 13
    /// and 14 digit forms of the same code all find each other.
    #[arg(long, value_name = "CODE")]
    upc: Option<String>,
}

/// The kinds, as subcommands rather than a value.
///
/// A recording is narrowed by things no other kind has, and as one flat
/// command `--help` offered `--singer` and `--upc` to someone looking for an
/// opera. Each kind now documents only what applies to it, and passing the
/// wrong flag stops being a mistake this has to catch and explain.
#[derive(Subcommand)]
enum Searching {
    /// A recording, by its opera, cast, conductor, year or barcode.
    Recording(SearchRecordingArgs),
    /// An opera, by title.
    Opera(Looking),
    /// A singer, by name.
    Singer(Looking),
    /// A conductor, by name.
    Conductor(Looking),
    /// A composer, by name.
    Composer(Looking),
    /// A character, by name.
    Character(Looking),
}

#[derive(Args)]
struct SearchArgs {
    #[command(subcommand)]
    what: Searching,
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
        Command::Get(args) => cmd_get(args),
        Command::Create(args) => cmd_create(args),
        Command::Edit(args) => cmd_edit(args),
        Command::Delete(args) => cmd_delete(args),
        Command::Describe(args) => cmd_describe(args),
        Command::Search(args) => cmd_search(args),
        Command::Annotate(args) => cmd_annotate(args),
        Command::Template => cmd_template(),
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

// ---- the CRUD commands ----------------------------------------------------
//
// One convention for failure, applied by every command here. A person gets the
// server's words on stderr; a program asking for --json gets the same facts as
// JSON, in GraphQL's own shape, also on stderr — results go to stdout and a
// failure has no result, so mixing the two would mean every caller had to
// check before parsing. The exit code says which happened either way.

/// Report a request that did not produce an answer.
fn failed(e: &api::ApiError, json: bool) -> i32 {
    if json {
        eprintln!(
            "{}",
            serde_json::to_string_pretty(&e.to_json()).unwrap_or_default()
        );
    } else {
        eprintln!("findopera: {e}");
    }
    3
}

/// Report something wrong with the request before it was ever sent.
fn refused(message: &str, code: &str, json: bool, exit: i32) -> i32 {
    if json {
        let body = serde_json::json!({ "errors": [{ "message": message, "code": code }] });
        eprintln!(
            "{}",
            serde_json::to_string_pretty(&body).unwrap_or_default()
        );
    } else {
        eprintln!("findopera: {message}");
    }
    exit
}

fn cmd_get(args: GetArgs) -> i32 {
    let kind = match kind_named(&args.kind) {
        Ok(k) => k,
        Err(code) => return code,
    };
    let api = match client(&args.endpoint, args.token.as_ref()) {
        Ok(c) => c,
        Err(code) => return code,
    };
    let record = match api.get(kind, &args.id) {
        Ok(r) => r,
        Err(e) => return failed(&e, args.json),
    };
    let mut out = std::io::stdout().lock();
    if args.json {
        emit(
            &mut out,
            format_args!(
                "{}",
                serde_json::to_string_pretty(&record).unwrap_or_default()
            ),
        );
    } else {
        render(&mut out, &record, 0);
    }
    0
}

fn cmd_create(args: CreateArgs) -> i32 {
    let kind = match kind_named(&args.kind) {
        Ok(k) => k,
        Err(code) => return code,
    };
    if args.message.trim().is_empty() {
        return refused(
            "-m needs a source and some context; it goes into the record's history",
            "NO_JUSTIFICATION",
            args.json,
            2,
        );
    }
    let input = match read_input(args.input.as_ref()) {
        Ok(v) => v,
        Err(why) => return refused(&why, "BAD_INPUT", args.json, 2),
    };
    if let Err(why) = check_input(&input, kind.create, kind.name) {
        return refused(&why, "BAD_INPUT", args.json, 2);
    }

    let api = match client(&args.endpoint, args.token.as_ref()) {
        Ok(c) => c,
        Err(code) => return code,
    };
    match api.create(kind, input, &args.message) {
        Ok(id) => {
            let mut out = std::io::stdout().lock();
            if args.json {
                emit(
                    &mut out,
                    format_args!("{}", serde_json::json!({ "id": id })),
                );
            } else {
                emit(&mut out, format_args!("{id}"));
                eprintln!("findopera: added {} {id}", kind.name);
            }
            0
        }
        Err(e) => failed(&e, args.json),
    }
}

fn cmd_edit(args: EditArgs) -> i32 {
    let kind = match kind_named(&args.kind) {
        Ok(k) => k,
        Err(code) => return code,
    };
    if args.message.trim().is_empty() {
        return refused(
            "-m needs a source and some context; it goes into the record's history",
            "NO_JUSTIFICATION",
            args.json,
            2,
        );
    }
    let input = match read_input(args.input.as_ref()) {
        Ok(v) => v,
        Err(why) => return refused(&why, "BAD_INPUT", args.json, 2),
    };
    // An edit with nothing in it would be recorded as a change that changed
    // nothing, which is worse than being told to say what you meant.
    if input.as_object().is_some_and(|o| o.is_empty()) {
        return refused("nothing to change", "BAD_INPUT", args.json, 2);
    }
    if let Err(why) = check_input(&input, kind.edit, kind.name) {
        return refused(&why, "BAD_INPUT", args.json, 2);
    }

    let api = match client(&args.endpoint, args.token.as_ref()) {
        Ok(c) => c,
        Err(code) => return code,
    };
    match api.edit(kind, &args.id, input, &args.message) {
        Ok(id) => {
            let mut out = std::io::stdout().lock();
            if args.json {
                emit(
                    &mut out,
                    format_args!("{}", serde_json::json!({ "id": id })),
                );
            } else {
                emit(&mut out, format_args!("{id}"));
                eprintln!("findopera: changed {} {id}", kind.name);
            }
            0
        }
        Err(e) => failed(&e, args.json),
    }
}

fn cmd_delete(args: DeleteArgs) -> i32 {
    let kind = match kind_named(&args.kind) {
        Ok(k) => k,
        Err(code) => return code,
    };
    if args.message.trim().is_empty() {
        return refused(
            "-m needs a source and some context; it goes into the record's history",
            "NO_JUSTIFICATION",
            args.json,
            2,
        );
    }
    if !args.yes {
        return refused(
            &format!(
                "this would remove {} {}. Pass --yes to do it.",
                kind.name, args.id
            ),
            "NEEDS_CONFIRMATION",
            args.json,
            2,
        );
    }
    let api = match client(&args.endpoint, args.token.as_ref()) {
        Ok(c) => c,
        Err(code) => return code,
    };
    match api.delete(kind, &args.id, &args.message) {
        Ok(()) => {
            if args.json {
                let mut out = std::io::stdout().lock();
                emit(
                    &mut out,
                    format_args!("{}", serde_json::json!({ "deleted": args.id })),
                );
            } else {
                eprintln!("findopera: removed {} {}", kind.name, args.id);
            }
            0
        }
        Err(e) => failed(&e, args.json),
    }
}

fn cmd_describe(args: DescribeArgs) -> i32 {
    let mut out = std::io::stdout().lock();
    let Some(name) = args.kind else {
        if args.json {
            let names: Vec<&str> = crud::TYPES.iter().map(|t| t.name).collect();
            emit(
                &mut out,
                format_args!("{}", serde_json::json!({ "types": names })),
            );
            return 0;
        }
        let width = crud::TYPES.iter().map(|t| t.name.len()).max().unwrap_or(0);
        for t in crud::TYPES {
            let required = t.create.iter().filter(|f| f.required).count();
            if !emit(
                &mut out,
                format_args!(
                    "{:<width$}  {} field{}, {required} required",
                    t.name,
                    t.create.len(),
                    if t.create.len() == 1 { "" } else { "s" }
                ),
            ) {
                break;
            }
        }
        return 0;
    };
    let kind = match kind_named(&name) {
        Ok(k) => k,
        Err(code) => return code,
    };

    if args.json {
        emit(
            &mut out,
            format_args!(
                "{}",
                serde_json::to_string_pretty(&json_schema(kind)).unwrap_or_default()
            ),
        );
        return 0;
    }

    eprintln!("{} — the fields a create takes\n", kind.name);
    let width = kind.create.iter().map(|f| f.name.len()).max().unwrap_or(0);
    for f in kind.create {
        let required = if f.required { "required" } else { "        " };
        let about = if f.about.is_empty() {
            String::new()
        } else {
            format!("  {}", f.about)
        };
        if !emit(
            &mut out,
            format_args!("{:<width$}  {:<7}  {required}{about}", f.name, f.json),
        ) {
            break;
        }
    }
    eprintln!(
        "\nAn edit takes the same fields, none of them required. \
         `findopera describe {} --json` gives this as a JSON Schema.",
        kind.name
    );
    0
}

/// A JSON Schema for what a create takes.
///
/// Draft 2020-12 rather than a shape of our own, because anything that already
/// validates JSON can then check an input before it is sent, and nobody has to
/// be taught a vocabulary that exists only here.
fn json_schema(kind: &crud::Type) -> serde_json::Value {
    let mut properties = serde_json::Map::new();
    for f in kind.create {
        let mut prop = serde_json::Map::new();
        // Every field but the required ones may be sent as null to mean absent.
        prop.insert(
            "type".into(),
            if f.required {
                f.json.into()
            } else {
                serde_json::json!([f.json, "null"])
            },
        );
        if !f.about.is_empty() {
            prop.insert("description".into(), f.about.into());
        }
        properties.insert(f.name.to_string(), serde_json::Value::Object(prop));
    }
    let required: Vec<&str> = kind
        .create
        .iter()
        .filter(|f| f.required)
        .map(|f| f.name)
        .collect();
    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": format!("Create{}Input", kind.graphql),
        "description": format!("What `findopera create {}` accepts.", kind.name),
        "type": "object",
        "properties": properties,
        "required": required,
        // The command refuses an unknown key before sending, so saying so here
        // lets a validator agree with it rather than pass something we reject.
        "additionalProperties": false,
    })
}

fn cmd_search(args: SearchArgs) -> i32 {
    // Each kind carries only the ways it can be narrowed, so the shape of the
    // request is settled by the time this runs.
    let (kind, looking, criteria) = match args.what {
        Searching::Recording(a) => {
            let text = a.looking.query.join(" ");
            let first = a.looking.first;
            (
                api::Kind::Recording,
                a.looking,
                api::Criteria {
                    text,
                    singers: a.singer,
                    conductor: a.conductor,
                    year: a.year,
                    upc: a.upc,
                    first,
                },
            )
        }
        Searching::Opera(l) => plain(api::Kind::Opera, l),
        Searching::Singer(l) => plain(api::Kind::Singer, l),
        Searching::Conductor(l) => plain(api::Kind::Conductor, l),
        Searching::Composer(l) => plain(api::Kind::Composer, l),
        Searching::Character(l) => plain(api::Kind::Character, l),
    };

    if looking.first > api::MAX_FIRST {
        return refused(
            &format!(
                "--first is {} at most, and {} was asked for. Narrow the search rather than \
                 widening the page.",
                api::MAX_FIRST,
                looking.first
            ),
            "TOO_MANY",
            looking.json,
            2,
        );
    }

    let narrowed = !criteria.singers.is_empty()
        || criteria.conductor.is_some()
        || criteria.year.is_some()
        || criteria.upc.is_some();
    if criteria.text.trim().is_empty() && !narrowed {
        eprintln!("findopera: nothing to search for");
        eprintln!("  help: findopera search {} <name>", kind_word(kind));
        return 2;
    }

    let api = match client(&looking.endpoint, looking.token.as_ref()) {
        Ok(c) => c,
        Err(code) => return code,
    };
    let results = match api.search(kind, &criteria) {
        Ok(r) => r,
        Err(e) => return failed(&e, looking.json),
    };
    let found = &results.found;
    if looking.json {
        let rows: Vec<serde_json::Value> = found
            .iter()
            .map(|f| serde_json::json!({ "id": f.id, "name": f.name, "about": f.about }))
            .collect();
        // An object rather than a bare list, because `truncated` has to travel
        // with the results. A caller reading the list and not the flag is the
        // exact mistake this exists to prevent, and a list cannot carry it.
        let body = serde_json::json!({ "results": rows, "truncated": results.more });
        let mut out = std::io::stdout().lock();
        emit(
            &mut out,
            format_args!(
                "{}",
                serde_json::to_string_pretty(&body).unwrap_or_default()
            ),
        );
        // Nothing found is not an error in JSON: an empty list is a perfectly
        // good answer, and a caller can see it is empty.
        return i32::from(found.is_empty());
    }
    if found.is_empty() {
        eprintln!("findopera: nothing found");
        // Partial names match, so a search that finds nothing is usually
        // spelled differently rather than absent.
        eprintln!("  help: try less of the name — matching is partial, and ignores accents");
        return 1;
    }

    let mut out = std::io::stdout().lock();
    let id_width = found.iter().map(|f| f.id.len()).max().unwrap_or(0);
    let name_width = found
        .iter()
        .map(|f| f.name.chars().count())
        .max()
        .unwrap_or(0);
    for f in found {
        let line = if looking.tabs {
            format!("{}\t{}\t{}", f.id, f.name, f.about)
        } else {
            let pad = name_width.saturating_sub(f.name.chars().count());
            format!("{:>id_width$}  {}{:pad$}  {}", f.id, f.name, "", f.about)
        };
        if !emit(&mut out, format_args!("{}", line.trim_end())) {
            return 0;
        }
    }
    // Said after the results, and on stderr, so it cannot be mistaken for one
    // of them. Without it a full page reads as the whole answer, and something
    // that matched perfectly well looks like it does not exist.
    if results.more {
        drop(out);
        eprintln!(
            "findopera: these are the first {}, and there are more. Narrow the search, or \
             raise --first.",
            found.len()
        );
    }
    0
}

/// A kind that is looked up by name and nothing else.
fn plain(kind: api::Kind, looking: Looking) -> (api::Kind, Looking, api::Criteria) {
    let criteria = api::Criteria {
        text: looking.query.join(" "),
        first: looking.first,
        ..Default::default()
    };
    (kind, looking, criteria)
}

/// What to call a kind when telling someone how to ask again.
fn kind_word(kind: api::Kind) -> &'static str {
    match kind {
        api::Kind::Recording => "recording",
        api::Kind::Opera => "opera",
        api::Kind::Singer => "singer",
        api::Kind::Conductor => "conductor",
        api::Kind::Composer => "composer",
        api::Kind::Character => "character",
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

    /// One row of the plan, as a program sees it.
    ///
    /// The same fields the columns show, named, plus the two things the
    /// columns cannot say: which marker a row came from, and whether its name
    /// was chosen or fallen back to.
    fn row_json(row: &plan::Row, outcome: Option<&apply::Outcome>) -> serde_json::Value {
        let mut o = serde_json::json!({
            "directory": row.marker.directory.display().to_string(),
            "marker": row.marker.marker_path.display().to_string(),
            "id": row.marker.id,
            "path": row.path,
            "segments": row.segments,
            "variant": row.marker.variant,
            "numbered": row.derived,
        });
        if let Some(outcome) = outcome {
            let (state, why) = match outcome {
                apply::Outcome::Created => ("created", None),
                apply::Outcome::Skipped => ("skipped", None),
                apply::Outcome::Conflict(w) => ("conflict", Some(w.clone())),
                apply::Outcome::Failed(w) => ("failed", Some(w.clone())),
            };
            o["outcome"] = state.into();
            if let Some(why) = why {
                o["reason"] = why.into();
            }
        }
        o
    }

    let destination = p.settings.as_ref().and_then(|c| c.destination.clone());
    let link = p.settings.as_ref().map(|c| c.link).unwrap_or_default();
    let dry_run = !args.write;

    // Without somewhere to build, the folder names are still worth having — they are
    // what you are looking at while you settle on a template, before there is
    // any question of a destination.
    let Some(destination) = destination else {
        let mut out = std::io::stdout().lock();
        if args.json {
            let rows: Vec<serde_json::Value> =
                plan.rows.iter().map(|r| row_json(r, None)).collect();
            emit(
                &mut out,
                format_args!(
                    "{}",
                    serde_json::to_string_pretty(&rows).unwrap_or_default()
                ),
            );
            report(&plan, p.require_variants);
            return i32::from(plan.blocked(p.require_variants));
        }
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
    if args.json {
        let rows: Vec<serde_json::Value> = plan
            .rows
            .iter()
            .zip(&done.entries)
            .map(|(row, entry)| row_json(row, Some(&entry.outcome)))
            .collect();
        emit(
            &mut out,
            format_args!(
                "{}",
                serde_json::to_string_pretty(&rows).unwrap_or_default()
            ),
        );
        report(&plan, p.require_variants);
        return i32::from(done.troubled() || plan.blocked(p.require_variants));
    }
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
            // so the pointer to it is added here. `template` lists the syntax as
            // well as the fields, which makes it the right answer whether the
            // template named something that is not there or was malformed.
            eprintln!("  see `findopera template` for every field and the syntax");
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

fn cmd_template() -> i32 {
    // The list is the result and goes to stdout, so `findopera template | grep
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

/// Print a record as indented lines.
///
/// One renderer rather than twenty, because the curation lives in
/// `schema/get.graphql` — what is fetched is decided there, and this only has
/// to lay out whatever came back. A per-type renderer would be a second place
/// to keep in step with the first.
fn render(out: &mut impl Write, value: &serde_json::Value, indent: usize) -> bool {
    let pad = " ".repeat(indent);
    let serde_json::Value::Object(map) = value else {
        return emit(out, format_args!("{pad}{}", scalar(value)));
    };

    // Keys are laid out in a column, but only against their own siblings: a
    // nested object padded to its parent's width reads as though it were part
    // of it.
    // Everything that ends up on one line shares the column, which includes an
    // empty list and a list of plain values — they print inline too, and
    // leaving them out of the count makes them the only crooked rows.
    let inline = |v: &serde_json::Value| match v {
        serde_json::Value::Object(_) => false,
        serde_json::Value::Array(items) => items.iter().all(|i| !i.is_object()),
        _ => true,
    };
    let width = map
        .iter()
        .filter(|(_, v)| inline(v))
        .map(|(k, _)| k.chars().count())
        .max()
        .unwrap_or(0);

    // A record's own values first, then the records it points at. Alphabetical
    // order alone interleaves them, and a nested block between two scalars
    // makes both harder to read than either would be alone.
    let (nested, plain): (Vec<_>, Vec<_>) = map.iter().partition(|(_, v)| {
        v.is_object() || matches!(v, serde_json::Value::Array(a) if a.iter().any(|i| i.is_object()))
    });

    for (key, child) in plain.into_iter().chain(nested) {
        match child {
            serde_json::Value::Object(_) => {
                if !emit(out, format_args!("{pad}{key}")) {
                    return false;
                }
                if !render(out, child, indent + 2) {
                    return false;
                }
            }
            serde_json::Value::Array(items) => {
                if items.is_empty() {
                    if !emit(out, format_args!("{pad}{key:width$}  —")) {
                        return false;
                    }
                    continue;
                }
                // A list of plain values belongs on one line; a list of records
                // does not.
                if items.iter().all(|i| !i.is_object()) {
                    let joined: Vec<String> = items.iter().map(scalar).collect();
                    if !emit(
                        out,
                        format_args!("{pad}{key:width$}  {}", joined.join(", ")),
                    ) {
                        return false;
                    }
                    continue;
                }
                if !emit(out, format_args!("{pad}{key}")) {
                    return false;
                }
                for item in items {
                    if !render(out, item, indent + 2) {
                        return false;
                    }
                    if !emit(out, format_args!("")) {
                        return false;
                    }
                }
            }
            _ => {
                if !emit(out, format_args!("{pad}{key:width$}  {}", scalar(child))) {
                    return false;
                }
            }
        }
    }
    true
}

/// One value, as a person would read it.
///
/// An absent value is shown rather than dropped: a field that is empty is
/// something to fill in, and a reader deciding what to send needs to know the
/// difference between empty and not a field at all.
fn scalar(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "—".to_string(),
        serde_json::Value::String(s) if s.is_empty() => "—".to_string(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Bool(b) => if *b { "yes" } else { "no" }.to_string(),
        other => other.to_string(),
    }
}

/// The type this command is about.
fn kind_named(name: &str) -> Result<&'static crud::Type, i32> {
    if let Some(t) = crud::TYPES.iter().find(|t| t.name == name) {
        return Ok(t);
    }
    eprintln!("findopera: there is no type called `{name}`");
    eprintln!("  help: `findopera describe` lists them all");
    Err(2)
}

/// JSON from a file, or from standard input.
fn read_input(from: Option<&PathBuf>) -> Result<serde_json::Value, String> {
    let text = match from {
        Some(path) if path.as_os_str() != "-" => std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?,
        _ => std::io::read_to_string(std::io::stdin())
            .map_err(|e| format!("cannot read the input from standard input: {e}"))?,
    };
    if text.trim().is_empty() {
        return Err("no input given".to_string());
    }
    serde_json::from_str(&text).map_err(|e| format!("that is not valid JSON: {e}"))
}

/// Check the keys against what the type accepts, before spending a request.
///
/// The server would refuse an unknown field too, but it would name it in a
/// GraphQL validation error against an input type the caller never mentioned.
/// A typo is the likeliest mistake here and deserves an answer in the terms
/// the caller used.
fn check_input(
    value: &serde_json::Value,
    accepted: &[crud::InputField],
    what: &str,
) -> Result<(), String> {
    let serde_json::Value::Object(map) = value else {
        return Err(format!(
            "a {what} takes a JSON object, with one key per field"
        ));
    };
    for key in map.keys() {
        if accepted.iter().any(|f| f.name == key) {
            continue;
        }
        let near = accepted
            .iter()
            .find(|f| f.name.eq_ignore_ascii_case(key))
            .map(|f| format!(" — did you mean `{}`?", f.name));
        return Err(format!(
            "`{key}` is not a field of a {what}{}",
            near.unwrap_or_default()
        ));
    }
    for field in accepted.iter().filter(|f| f.required) {
        if !map.contains_key(field.name) {
            return Err(format!("a {what} needs `{}`", field.name));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::crud;
    use super::{definition, with_variant};

    #[test]
    fn an_unknown_field_is_refused_with_the_nearest_real_one() {
        let fields = &[
            crud::InputField {
                name: "firstName",
                json: "string",
                required: true,
                about: "",
            },
            crud::InputField {
                name: "born",
                json: "integer",
                required: false,
                about: "",
            },
        ];
        let why = super::check_input(
            &serde_json::json!({ "firstname": "Maria" }),
            fields,
            "singer",
        )
        .expect_err("a typo is refused");
        // Case is the likeliest slip, and the server would answer it with a
        // GraphQL error naming an input type the caller never mentioned.
        assert!(why.contains("firstName"), "got: {why}");
    }

    #[test]
    fn a_missing_required_field_is_named() {
        let fields = &[crud::InputField {
            name: "firstName",
            json: "string",
            required: true,
            about: "",
        }];
        let why = super::check_input(&serde_json::json!({}), fields, "singer")
            .expect_err("required fields are checked");
        assert!(why.contains("firstName"), "got: {why}");
    }

    #[test]
    fn optional_fields_may_be_left_out() {
        let fields = &[
            crud::InputField {
                name: "firstName",
                json: "string",
                required: true,
                about: "",
            },
            crud::InputField {
                name: "born",
                json: "integer",
                required: false,
                about: "",
            },
        ];
        super::check_input(
            &serde_json::json!({ "firstName": "Maria" }),
            fields,
            "singer",
        )
        .expect("the optional one is optional");
    }

    #[test]
    fn something_that_is_not_an_object_is_refused() {
        let why = super::check_input(&serde_json::json!([1, 2]), &[], "singer")
            .expect_err("a list is not an input");
        assert!(why.contains("JSON object"), "got: {why}");
    }

    #[test]
    fn absent_values_are_shown_rather_than_dropped() {
        // A reader deciding what to fill in needs to see that a field exists
        // and is empty, which is not the same as it not being a field.
        assert_eq!(super::scalar(&serde_json::Value::Null), "—");
        assert_eq!(super::scalar(&serde_json::json!("")), "—");
        assert_eq!(super::scalar(&serde_json::json!(true)), "yes");
        assert_eq!(super::scalar(&serde_json::json!(1923)), "1923");
    }

    #[test]
    fn every_type_is_reachable_by_the_name_it_is_listed_under() {
        for t in crud::TYPES {
            assert!(
                super::kind_named(t.name).is_ok(),
                "{} is not findable",
                t.name
            );
        }
        assert!(super::kind_named("no-such-type").is_err());
    }

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
