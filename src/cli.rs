//! The command line: what the verbs are, and what each one does.
//!
//! Lives in the library rather than in `main.rs` so that a test can call it.
//! `main.rs` is the process wrapper and nothing else — argument parsing, the
//! verbs, and their output all live here, where they can be exercised without
//! spawning anything.

use crate::config::{self, Config};
use crate::credentials;
use crate::model::{crud, Recording, FIELDS};
use crate::FieldDoc;
use crate::{api, apply, plan, release, scan, Template};
use clap::{Args, Parser, Subcommand};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Where a command's output goes.
///
/// Threaded through rather than written to the process, so that a whole
/// command can be run in the same process a test is running in and what it
/// said read back afterwards. The two streams stay separate because they
/// carry different things: stdout is the result, stderr is everything about
/// producing it.
pub struct Session<'a> {
    out: &'a mut dyn Write,
    err: &'a mut dyn Write,
    /// How to reach findopera.com, where that is not simply "over the network".
    ///
    /// The commands ask the session for their client rather than building one,
    /// so a test can run a whole command against canned answers without the
    /// command knowing anything has changed.
    #[allow(clippy::type_complexity)]
    client: Option<Box<dyn Fn(&str, Option<String>) -> api::Client + Send + Sync>>,
}

impl<'a> Session<'a> {
    pub fn new(out: &'a mut dyn Write, err: &'a mut dyn Write) -> Session<'a> {
        Session {
            out,
            err,
            client: None,
        }
    }

    /// Answer this session's requests with something other than the network.
    pub fn served_by(
        mut self,
        make: impl Fn(&str, Option<String>) -> api::Client + Send + Sync + 'static,
    ) -> Session<'a> {
        self.client = Some(Box::new(make));
        self
    }

    fn api(&self, endpoint: &str, token: Option<String>) -> api::Client {
        match &self.client {
            Some(make) => make(endpoint, token),
            None => api::Client::new(endpoint, token),
        }
    }

    /// A result line. False once the reader has gone away.
    ///
    /// This command's output is a list, so it will be piped to `head` — and
    /// the default `println!` panics when the reader goes away. Stopping
    /// quietly is what every other line-producing tool does.
    pub fn say(&mut self, line: std::fmt::Arguments) -> bool {
        writeln!(self.out, "{line}").is_ok()
    }

    /// A diagnostic. Nothing downstream reads these, so a closed pipe is not
    /// a reason to stop.
    pub fn note(&mut self, line: std::fmt::Arguments) {
        let _ = writeln!(self.err, "{line}");
    }
}

/// `note!(ui, "…", args)` — the `eprintln!` of a command that can be tested.
macro_rules! note {
    ($ui:expr, $($t:tt)*) => { $ui.note(format_args!($($t)*)) };
}

const AFTER_HELP: &str = "\
Each folder holds a text file of notes about the recording in it — cast,
conductor, year, orchestra, and where it came from — kept so that the folder
says what it contains without anything having to be opened. `findopera
annotate` writes one:

  cd '~/Music/Sosarme' && findopera annotate 10655

The id in its name is also how this program recognises the folder later, which
is what everything else here is built on.

Start with `findopera init`, then `findopera organize --help`.

`findopera self update` says whether a newer one has been released.

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
pub struct Cli {
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

That filename is also how this program recognises the folder again: it looks
for `findopera-<id>` in the name, so the file may be renamed freely as long
as that part survives. A bare `10655.txt` is not enough — a number and a
.txt is what a track listing or a year looks like, and the `findopera-` is
what says the number means a recording.

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

Nothing is written without --write.

The destination is kept matching the plan, so a folder built for a recording
that has since left the library is removed again. Only folders this program
recorded building are ever removed — anything else in there is untouched, and
a destination it has no record of is refused rather than guessed at.

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

    /// Attach a UPC to a recording.
    #[command(long_about = "\
Attach a barcode to a recording, so that the release and the performance know
about each other.

  findopera link recording 264 --upc 0013491103020 -m 'back of the box'

The UPC has to exist already — `findopera create upc` makes one — because a
barcode conjured out of a typo is worse than a missing link. Barcodes are
matched exactly here, unlike `search --upc`: this says which record to attach,
and guessing at that is not the same as finding it.")]
    Link(LinkArgs),

    /// Take a UPC off a recording.
    #[command(long_about = "\
Detach a barcode from a recording. The UPC record itself stays; only the
connection between the two goes.

  findopera unlink recording 264 --upc 0013491103020 -m 'wrong release'")]
    Unlink(LinkArgs),

    /// Remove a record.
    #[command(long_about = "\
Remove a record.

Nothing here is really destroyed — every change is versioned and can be
reverted — but this removes a record, so it asks for --yes as well as a
reason. `findopera merge` is the other one that does, and asks the same.

Where a record is going because another record describes the same thing,
prefer merge: a merge leaves the old id pointing at the survivor, and a
delete does not.")]
    Delete(DeleteArgs),

    /// Fold one record into another describing the same performance.
    #[command(
        long_about = "\
Merge a record into another, in favour of the second, for when two of them
turn out to describe the same singer, opera or performance.

  findopera merge singer 133 --into 456 -m 'https://... — same person, two spellings'

The first id loses. Its record goes, keeping its history, and anyone arriving
with that id afterwards is sent to the survivor instead, so a link written
down before the merge still works.

A merge is refused while anything still points at the losing record —
recordings against a duplicate singer, say. Move those over first, so that
what became of them is a decision somebody made rather than a side effect. The
refusal comes from the server, and names what is still in the way.

Not every type can be merged. `findopera describe <type>` says whether one
can, and like `delete` this asks for --yes as well as a reason.",
        after_help = "\
Examples:
  findopera merge singer 133 --into 456 -m 'same person, two spellings' --yes
  findopera merge opera 12 --into 34 -m 'https://...' --yes --json"
    )]
    Merge(MergeArgs),

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
lives inside the library being organised and is walked, linked and synced
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

    /// This program itself: whether it is the current version.
    #[command(
        name = "self",
        long_about = "\
Things about this program rather than about a library or a record.

  findopera self update    is there a newer one?",
        subcommand_required = true,
        arg_required_else_help = true
    )]
    Zelf(SelfArgs),

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
    /// command line, so this flag is the only warning that a run is going to
    /// touch the disk.
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
struct MergeArgs {
    /// What kind of record.
    #[arg(value_name = "TYPE")]
    kind: String,
    /// The id that loses, and goes.
    #[arg(value_name = "ID")]
    id: String,
    /// The id that survives, and keeps its own fields.
    #[arg(long, value_name = "ID")]
    into: String,
    /// Source and context for this change, for the record's history.
    #[arg(long, short = 'm', value_name = "TEXT")]
    message: String,
    /// Say so out loud. Nothing is merged without it.
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

/// What a relationship command joins.
///
/// A subcommand rather than a value, as `search` is, so that each side can
/// document what it may be joined to. Recording and UPC is the only pair the
/// database has; another would be another subcommand here.
#[derive(Subcommand)]
enum Joining {
    /// A recording, to a UPC.
    Recording(LinkRecordingArgs),
}

#[derive(Args)]
struct LinkRecordingArgs {
    /// The recording's id.
    #[arg(value_name = "ID")]
    id: String,
    /// The barcode, which must already be a UPC record.
    #[arg(long, value_name = "CODE")]
    upc: String,
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
struct LinkArgs {
    #[command(subcommand)]
    what: Joining,
}

#[derive(Subcommand)]
enum Selfing {
    /// Say whether a newer version has been released.
    #[command(long_about = "\
Ask GitHub whether a newer findopera has been published, and say how to get
it if so.

  findopera self update

It does not replace this binary. Whatever installed it — an installer, a
package manager, or you with `scp` — knows where the binary lives and what
belongs beside it, and is what should replace it. On the machines this is
usually run on, the binary may not even be writable by whoever runs it.

So this reports, and names the command to run. Which command that is depends
on how this copy looks to have been installed, which is guessed from where it
sits and said as a guess.

The check is one anonymous request to GitHub's releases API, which allows a
limited number of those an hour from one address.")]
    Update(SelfUpdateArgs),
}

#[derive(Args)]
struct SelfArgs {
    #[command(subcommand)]
    what: Selfing,
}

#[derive(Args)]
struct SelfUpdateArgs {
    /// Print the result as JSON.
    #[arg(long)]
    json: bool,
    /// Where to ask. For testing against something other than GitHub.
    #[arg(long, hide = true, default_value = release::RELEASES_API, value_name = "URL")]
    releases_url: String,
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

/// Write one result line, reporting whether stdout is still listening.
///
/// This command's output is a list, so it will be piped to `head` — and the
/// default `println!` panics when the reader goes away. Stopping quietly is
/// what every other line-producing tool does.
fn emit(ui: &mut Session, line: std::fmt::Arguments) -> bool {
    ui.say(line)
}

/// Parse the real argv and run, returning the process exit code.
pub fn run() -> i32 {
    let stdout = std::io::stdout();
    let stderr = std::io::stderr();
    let mut out = stdout.lock();
    let mut err = stderr.lock();
    let mut ui = Session::new(&mut out, &mut err);
    dispatch(&mut ui, Cli::parse())
}

impl Cli {
    /// Parse an argv, giving back clap's own message rather than printing it.
    ///
    /// `Cli::parse` writes usage errors straight to the process and exits,
    /// which a test cannot see and must not be subjected to.
    pub fn try_parse_from_argv<S: AsRef<str>>(argv: &[S]) -> Result<Cli, NotACommand> {
        <Cli as Parser>::try_parse_from(argv.iter().map(|a| a.as_ref())).map_err(|e| NotACommand {
            // Help and a version are answers, and go where answers go. Only a
            // complaint about the arguments is a failure.
            to_stdout: !e.use_stderr(),
            code: e.exit_code(),
            text: e.to_string(),
        })
    }
}

/// What was printed instead of a command being parsed.
///
/// `--help` and `--version` are among these, and are not failures: they are
/// what was asked for, so they go to stdout and the run succeeds.
pub struct NotACommand {
    pub text: String,
    pub to_stdout: bool,
    pub code: i32,
}

/// Run one already-parsed command line against a session.
///
/// Separate from `run` so that a test can hand it somewhere to write and read
/// back what a whole command said, without a process to capture.
pub fn dispatch(ui: &mut Session, cli: Cli) -> i32 {
    match cli.command {
        Command::Organize(args) => cmd_organize(ui, args),
        Command::Get(args) => cmd_get(ui, args),
        Command::Create(args) => cmd_create(ui, args),
        Command::Edit(args) => cmd_edit(ui, args),
        Command::Link(args) => cmd_link(ui, args, true),
        Command::Unlink(args) => cmd_link(ui, args, false),
        Command::Delete(args) => cmd_delete(ui, args),
        Command::Merge(args) => cmd_merge(ui, args),
        Command::Describe(args) => cmd_describe(ui, args),
        Command::Search(args) => cmd_search(ui, args),
        Command::Annotate(args) => cmd_annotate(ui, args),
        Command::Template => cmd_template(ui),
        Command::Graphql(args) => cmd_graphql(ui, args),
        Command::Schema(args) => cmd_schema(ui, args),
        Command::Feedback(args) => cmd_feedback(ui, args),
        Command::Login(args) => cmd_login(ui, args),
        Command::Logout => cmd_logout(ui),
        Command::Zelf(args) => match args.what {
            Selfing::Update(a) => cmd_self_update(ui, a),
        },
        Command::Init(args) => cmd_init(ui, args),
    }
}

/// The client for this run, with whatever identity it has.
fn client(ui: &mut Session, endpoint: &str, token: Option<&String>) -> Result<api::Client, i32> {
    match credentials::resolve(token.map(String::as_str)) {
        Ok(token) => Ok(ui.api(endpoint, token)),
        Err(why) => {
            note!(ui, "findopera: {why}");
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
fn failed(ui: &mut Session, e: &api::ApiError, json: bool) -> i32 {
    if json {
        note!(
            ui,
            "{}",
            serde_json::to_string_pretty(&e.to_json()).unwrap_or_default()
        );
    } else {
        note!(ui, "findopera: {e}");
    }
    3
}

/// Report something wrong with the request before it was ever sent.
fn refused(ui: &mut Session, message: &str, code: &str, json: bool, exit: i32) -> i32 {
    if json {
        let body = serde_json::json!({ "errors": [{ "message": message, "code": code }] });
        note!(
            ui,
            "{}",
            serde_json::to_string_pretty(&body).unwrap_or_default()
        );
    } else {
        note!(ui, "findopera: {message}");
    }
    exit
}

fn cmd_get(ui: &mut Session, args: GetArgs) -> i32 {
    let kind = match kind_named(ui, &args.kind) {
        Ok(k) => k,
        Err(code) => return code,
    };
    let api = match client(ui, &args.endpoint, args.token.as_ref()) {
        Ok(c) => c,
        Err(code) => return code,
    };
    let record = match api.get(kind, &args.id) {
        Ok(r) => r,
        Err(e) => return failed(ui, &e, args.json),
    };
    if args.json {
        emit(
            ui,
            format_args!(
                "{}",
                serde_json::to_string_pretty(&record).unwrap_or_default()
            ),
        );
    } else {
        render(ui, &record, 0);
    }
    0
}

fn cmd_create(ui: &mut Session, args: CreateArgs) -> i32 {
    let kind = match kind_named(ui, &args.kind) {
        Ok(k) => k,
        Err(code) => return code,
    };
    if args.message.trim().is_empty() {
        return refused(
            ui,
            "-m needs a source and some context; it goes into the record's history",
            "NO_JUSTIFICATION",
            args.json,
            2,
        );
    }
    let input = match read_input(args.input.as_ref()) {
        Ok(v) => v,
        Err(why) => return refused(ui, &why, "BAD_INPUT", args.json, 2),
    };
    let extras = kind.composite.as_ref().map_or(&[][..], |c| c.extras);
    if let Err(why) = check_input(&input, kind.create, extras, kind.name) {
        return refused(ui, &why, "BAD_INPUT", args.json, 2);
    }
    if let Err(why) = check_elements(&input, extras) {
        return refused(ui, &why, "BAD_INPUT", args.json, 2);
    }
    if let Err(why) = check_cast(&input) {
        return refused(ui, &why, "BAD_INPUT", args.json, 2);
    }

    let api = match client(ui, &args.endpoint, args.token.as_ref()) {
        Ok(c) => c,
        Err(code) => return code,
    };
    match api.create(kind, input, &args.message) {
        Ok(id) => {
            if args.json {
                emit(ui, format_args!("{}", serde_json::json!({ "id": id })));
            } else {
                emit(ui, format_args!("{id}"));
                note!(ui, "findopera: added {} {id}", kind.name);
            }
            0
        }
        Err(e) => failed(ui, &e, args.json),
    }
}

fn cmd_edit(ui: &mut Session, args: EditArgs) -> i32 {
    let kind = match kind_named(ui, &args.kind) {
        Ok(k) => k,
        Err(code) => return code,
    };
    if args.message.trim().is_empty() {
        return refused(
            ui,
            "-m needs a source and some context; it goes into the record's history",
            "NO_JUSTIFICATION",
            args.json,
            2,
        );
    }
    let input = match read_input(args.input.as_ref()) {
        Ok(v) => v,
        Err(why) => return refused(ui, &why, "BAD_INPUT", args.json, 2),
    };
    // An edit with nothing in it would be recorded as a change that changed
    // nothing, which is worse than being told to say what you meant.
    if input.as_object().is_some_and(|o| o.is_empty()) {
        return refused(ui, "nothing to change", "BAD_INPUT", args.json, 2);
    }
    if let Err(why) = check_input(&input, kind.edit, &[], kind.name) {
        return refused(ui, &why, "BAD_INPUT", args.json, 2);
    }

    let api = match client(ui, &args.endpoint, args.token.as_ref()) {
        Ok(c) => c,
        Err(code) => return code,
    };
    match api.edit(kind, &args.id, input, &args.message) {
        Ok(id) => {
            if args.json {
                emit(ui, format_args!("{}", serde_json::json!({ "id": id })));
            } else {
                emit(ui, format_args!("{id}"));
                note!(ui, "findopera: changed {} {id}", kind.name);
            }
            0
        }
        Err(e) => failed(ui, &e, args.json),
    }
}

fn cmd_delete(ui: &mut Session, args: DeleteArgs) -> i32 {
    let kind = match kind_named(ui, &args.kind) {
        Ok(k) => k,
        Err(code) => return code,
    };
    if args.message.trim().is_empty() {
        return refused(
            ui,
            "-m needs a source and some context; it goes into the record's history",
            "NO_JUSTIFICATION",
            args.json,
            2,
        );
    }
    if !args.yes {
        return refused(
            ui,
            &format!(
                "this would remove {} {}. Pass --yes to do it.",
                kind.name, args.id
            ),
            "NEEDS_CONFIRMATION",
            args.json,
            2,
        );
    }
    let api = match client(ui, &args.endpoint, args.token.as_ref()) {
        Ok(c) => c,
        Err(code) => return code,
    };
    match api.delete(kind, &args.id, &args.message) {
        Ok(()) => {
            if args.json {
                emit(
                    ui,
                    format_args!("{}", serde_json::json!({ "deleted": args.id })),
                );
            } else {
                note!(ui, "findopera: removed {} {}", kind.name, args.id);
            }
            0
        }
        Err(e) => failed(ui, &e, args.json),
    }
}

fn cmd_merge(ui: &mut Session, args: MergeArgs) -> i32 {
    let kind = match kind_named(ui, &args.kind) {
        Ok(k) => k,
        Err(code) => return code,
    };
    // Answered here rather than by the server, because the useful reply is the
    // list of types that *can* be merged, and the server can only say that
    // this mutation does not exist.
    if kind.merge.is_none() {
        let mergeable: Vec<&str> = crud::TYPES
            .iter()
            .filter(|t| t.merge.is_some())
            .map(|t| t.name)
            .collect();
        return refused(
            ui,
            &format!(
                "{} records cannot be merged. These can be: {}.",
                kind.name,
                mergeable.join(", ")
            ),
            "NOT_MERGEABLE",
            args.json,
            2,
        );
    }
    if args.message.trim().is_empty() {
        return refused(
            ui,
            "-m needs a source and some context; it goes into the record's history",
            "NO_JUSTIFICATION",
            args.json,
            2,
        );
    }
    // A record merged into itself would either do nothing or destroy the only
    // copy, depending on what the server makes of it. Neither is worth finding
    // out by trying.
    if args.id == args.into {
        return refused(
            ui,
            &format!("{} {} is already itself", kind.name, args.id),
            "BAD_INPUT",
            args.json,
            2,
        );
    }
    if !args.yes {
        return refused(
            ui,
            &format!(
                "this would fold {} {} into {} and remove it. Pass --yes to do it.",
                kind.name, args.id, args.into
            ),
            "NEEDS_CONFIRMATION",
            args.json,
            2,
        );
    }

    let api = match client(ui, &args.endpoint, args.token.as_ref()) {
        Ok(c) => c,
        Err(code) => return code,
    };
    match api.merge(kind, &args.id, &args.into, &args.message) {
        Ok(id) => {
            if args.json {
                emit(
                    ui,
                    format_args!("{}", serde_json::json!({ "merged": args.id, "into": id })),
                );
            } else {
                // The surviving id, so this composes with everything else that
                // takes one — including where it is not the id that was asked
                // for, the survivor having itself been merged since.
                emit(ui, format_args!("{id}"));
                note!(
                    ui,
                    "findopera: {} {} is now {} {id}",
                    kind.name,
                    args.id,
                    kind.name
                );
            }
            0
        }
        Err(e) => failed(ui, &e, args.json),
    }
}

fn cmd_link(ui: &mut Session, args: LinkArgs, on: bool) -> i32 {
    let Joining::Recording(a) = args.what;
    if a.message.trim().is_empty() {
        return refused(
            ui,
            "-m needs a source and some context; it goes into the record's history",
            "NO_JUSTIFICATION",
            a.json,
            2,
        );
    }
    // Punctuation is dropped because a barcode copied off a box carries
    // whatever was printed with it. Nothing is padded, though: this names a
    // record to attach rather than searching for one, and 12 and 13 digit
    // forms are different ids even when they mean the same product.
    let upc: String = a.upc.chars().filter(char::is_ascii_digit).collect();
    if upc.is_empty() {
        return refused(
            ui,
            &format!("`{}` has no digits in it", a.upc),
            "BAD_INPUT",
            a.json,
            2,
        );
    }

    let api = match client(ui, &a.endpoint, a.token.as_ref()) {
        Ok(c) => c,
        Err(code) => return code,
    };
    match api.link_upc(&a.id, &upc, &a.message, on) {
        Ok(()) => {
            if a.json {
                emit(
                    ui,
                    format_args!(
                        "{}",
                        serde_json::json!({ "recording": a.id, "upc": upc, "linked": on })
                    ),
                );
            } else if on {
                note!(ui, "findopera: recording {} now carries {upc}", a.id);
            } else {
                note!(ui, "findopera: recording {} no longer carries {upc}", a.id);
            }
            0
        }
        Err(e) => failed(ui, &e, a.json),
    }
}

fn cmd_self_update(ui: &mut Session, args: SelfUpdateArgs) -> i32 {
    // Where this binary sits is what the advice is built from. If the path
    // cannot be had — which is possible, and not worth failing over — the
    // installer is the documented route and so the better guess.
    let exe = std::env::current_exe().ok();
    let check = match release::check(&args.releases_url, exe.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            if args.json {
                let body = serde_json::json!({
                    "errors": [{ "message": e.to_string(), "code": "UNREACHABLE" }]
                });
                note!(
                    ui,
                    "{}",
                    serde_json::to_string_pretty(&body).unwrap_or_default()
                );
            } else {
                note!(ui, "findopera: {e}");
            }
            return 3;
        }
    };

    if args.json {
        emit(
            ui,
            format_args!(
                "{}",
                serde_json::json!({
                    "current": check.current.to_string(),
                    "latest": check.latest.to_string(),
                    "update_available": check.newer_available(),
                    "how": check.how.command(),
                })
            ),
        );
        return 0;
    }

    if !check.newer_available() {
        // Ahead of the latest release is what a build from a checkout looks
        // like, and saying "up to date" there would be a small lie.
        if check.current > check.latest {
            note!(
                ui,
                "findopera: this is {}, which is ahead of the latest release ({}).",
                check.current,
                check.latest
            );
        } else {
            note!(ui, "findopera: {} is the latest release.", check.current);
        }
        return 0;
    }

    emit(ui, format_args!("{}", check.latest));
    note!(
        ui,
        "findopera: {} has been released; this is {}.\n\n  {}\n",
        check.latest,
        check.current,
        check.how.command()
    );
    note!(
        ui,
        "That is a guess from where this binary sits. If you installed it some \
         other way, update it that way — this does not replace itself."
    );
    0
}

fn cmd_describe(ui: &mut Session, args: DescribeArgs) -> i32 {
    let Some(name) = args.kind else {
        if args.json {
            let names: Vec<&str> = crud::TYPES.iter().map(|t| t.name).collect();
            emit(
                ui,
                format_args!("{}", serde_json::json!({ "types": names })),
            );
            return 0;
        }
        let width = crud::TYPES.iter().map(|t| t.name.len()).max().unwrap_or(0);
        for t in crud::TYPES {
            let required = t.create.iter().filter(|f| f.required).count();
            if !emit(
                ui,
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
    let kind = match kind_named(ui, &name) {
        Ok(k) => k,
        Err(code) => return code,
    };

    if args.json {
        emit(
            ui,
            format_args!(
                "{}",
                serde_json::to_string_pretty(&json_schema(kind)).unwrap_or_default()
            ),
        );
        return 0;
    }

    note!(ui, "{} — the fields a create takes\n", kind.name);
    let extras = kind.composite.as_ref().map_or(&[][..], |c| c.extras);
    let width = kind
        .create
        .iter()
        .map(|f| f.name.len())
        .chain(extras.iter().map(|e| e.name.len()))
        .max()
        .unwrap_or(0);
    for f in kind.create {
        let required = if f.required { "required" } else { "        " };
        let about = if f.about.is_empty() {
            String::new()
        } else {
            format!("  {}", f.about)
        };
        if !emit(
            ui,
            format_args!("{:<width$}  {:<7}  {required}{about}", f.name, f.json),
        ) {
            break;
        }
    }
    for e in extras {
        let about = if e.about.is_empty() {
            String::new()
        } else {
            format!("  {}", e.about)
        };
        if !emit(
            ui,
            format_args!("{:<width$}  {:<7}          {about}", e.name, e.json),
        ) {
            break;
        }
        // What one of them looks like. `noted` in particular is required here
        // and optional on a portrayal created by itself, which is not
        // something anyone would guess.
        for item in e.items {
            let required = if item.required { "required" } else { "" };
            if !emit(
                ui,
                format_args!(
                    "  {:<w$}  {:<7}  {required}",
                    item.name,
                    item.json,
                    w = width.saturating_sub(2)
                ),
            ) {
                break;
            }
        }
    }
    if !extras.is_empty() {
        note!(
            ui,
            "\nThe last of those are built in the same transaction as the {}, so a \
             failure leaves nothing half-made.",
            kind.name
        );
    }
    note!(
        ui,
        "\nAn edit takes the same fields, none of them required, and none of the \
         extras. `findopera describe {} --json` gives this as a JSON Schema.",
        kind.name
    );
    if kind.merge.is_some() {
        note!(
            ui,
            "\nTwo of these can turn out to be the same thing: `findopera merge {} \
             <id> --into <id>` folds one into the other.",
            kind.name
        );
    }
    0
}

/// Check the objects inside a list-valued input.
///
/// The keys of an input are checked against what the type accepts; without
/// this the objects inside a list were not checked against anything, so a
/// misspelling one level down travelled to the server and came back as a
/// GraphQL error about an input type the caller never named.
fn check_elements(input: &serde_json::Value, extras: &[crud::Extra]) -> Result<(), String> {
    for extra in extras.iter().filter(|e| !e.items.is_empty()) {
        let Some(list) = input.get(extra.name).and_then(|v| v.as_array()) else {
            continue;
        };
        for (i, element) in list.iter().enumerate() {
            let what = format!("{} {}", extra.name, i + 1);
            check_input(element, extra.items, &[], &what)
                .map_err(|why| why.replace(&format!("a {what}"), &what))?;
        }
    }
    Ok(())
}

/// How many of a cast may be top-billed.
///
/// The server refuses a create whose cast does not have exactly this many
/// noted, or all of them if there are fewer. Twelve roles is a large thing to
/// send and have refused, so the same count is checked here first.
const NOTED: usize = 3;

/// Check the cast before a large payload is sent for nothing.
///
/// Only a create is checked. The rule belongs to this one mutation rather than
/// to the data — a portrayal added on its own is not counted, and a recording
/// can drift past it — so it is not enforced anywhere else and is not
/// presented as a fact about recordings.
fn check_cast(input: &serde_json::Value) -> Result<(), String> {
    let Some(cast) = input.get("portrayalInputs").and_then(|c| c.as_array()) else {
        return Ok(());
    };
    if cast.is_empty() {
        return Ok(());
    }
    let noted = cast
        .iter()
        .filter(|p| p["noted"].as_bool().unwrap_or(false))
        .count();
    let wanted = NOTED.min(cast.len());
    if noted == wanted {
        return Ok(());
    }
    Err(format!(
        "{} of the {} roles are marked `noted`, and this call wants {wanted}. \
         Top billing goes to {NOTED}, or to all of them when there are fewer.",
        noted,
        cast.len()
    ))
}

/// The schema of one object, from the fields it has.
fn object_schema(fields: &[crud::InputField]) -> serde_json::Value {
    let mut properties = serde_json::Map::new();
    for f in fields {
        let mut prop = serde_json::Map::new();
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
    let required: Vec<&str> = fields
        .iter()
        .filter(|f| f.required)
        .map(|f| f.name)
        .collect();
    serde_json::json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false,
    })
}

/// A JSON Schema for what a create takes.
///
/// Draft 2020-12 rather than a shape of our own, because anything that already
/// validates JSON can then check an input before it is sent, and nobody has to
/// be taught a vocabulary that exists only here.
fn json_schema(kind: &crud::Type) -> serde_json::Value {
    let mut properties = serde_json::Map::new();
    for e in kind.composite.as_ref().map_or(&[][..], |c| c.extras) {
        let mut prop = serde_json::Map::new();
        prop.insert("type".into(), serde_json::json!([e.json, "null"]));
        if !e.about.is_empty() {
            prop.insert("description".into(), e.about.into());
        }
        // A list without this says only "array", which is the one thing in an
        // input with any structure and the one thing a validator could not
        // check.
        if !e.items.is_empty() {
            prop.insert("items".into(), object_schema(e.items));
        }
        properties.insert(e.name.to_string(), serde_json::Value::Object(prop));
    }
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

fn cmd_search(ui: &mut Session, args: SearchArgs) -> i32 {
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
            ui,
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
        note!(ui, "findopera: nothing to search for");
        note!(ui, "  help: findopera search {} <name>", kind_word(kind));
        return 2;
    }

    let api = match client(ui, &looking.endpoint, looking.token.as_ref()) {
        Ok(c) => c,
        Err(code) => return code,
    };
    let results = match api.search(kind, &criteria) {
        Ok(r) => r,
        Err(e) => return failed(ui, &e, looking.json),
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
        emit(
            ui,
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
        note!(ui, "findopera: nothing found");
        // Partial names match, so a search that finds nothing is usually
        // spelled differently rather than absent.
        note!(
            ui,
            "  help: try less of the name — matching is partial, and ignores accents"
        );
        return 1;
    }

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
        if !emit(ui, format_args!("{}", line.trim_end())) {
            return 0;
        }
    }
    // Said after the results, and on stderr, so it cannot be mistaken for one
    // of them. Without it a full page reads as the whole answer, and something
    // that matched perfectly well looks like it does not exist.
    if results.more {
        note!(
            ui,
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

fn cmd_annotate(ui: &mut Session, args: AnnotateArgs) -> i32 {
    let id = args.id.trim().trim_start_matches('#');
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
        note!(ui, "findopera: `{}` is not a recording id", args.id);
        note!(ui, "  help: the id is the number in the address, as in");
        note!(ui, "        https://findopera.com/recording/10655");
        return 2;
    }

    let api = match client(ui, &args.endpoint, args.token.as_ref()) {
        Ok(c) => c,
        Err(code) => return code,
    };
    let notes = match api.notes(id) {
        Ok(n) => n,
        Err(e) => {
            note!(ui, "findopera: {e}");
            return 3;
        }
    };

    let filename = match args.variant.as_deref() {
        None => notes.filename.clone(),
        Some(variant) => match with_variant(&notes.filename, id, variant) {
            Some(name) => name,
            None => {
                note!(
                    ui,
                    "findopera: cannot put a variant in `{}` — it does not carry `findopera-{id}`",
                    notes.filename
                );
                return 3;
            }
        },
    };

    if !args.dir.is_dir() {
        note!(ui, "findopera: {} is not a folder", args.dir.display());
        return 2;
    }
    let path = args.dir.join(&filename);
    if path.exists() && !args.force {
        note!(ui, "findopera: {} is already there", path.display());
        note!(ui, "  help: pass --force to replace it");
        return 1;
    }
    if let Err(e) = std::fs::write(&path, &notes.body) {
        note!(ui, "findopera: cannot write {}: {e}", path.display());
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

fn cmd_feedback(ui: &mut Session, args: FeedbackArgs) -> i32 {
    let message = match &args.message {
        Some(m) => m.clone(),
        None => match std::io::read_to_string(std::io::stdin()) {
            Ok(m) => m,
            Err(e) => {
                note!(
                    ui,
                    "findopera: cannot read the message from standard input: {e}"
                );
                return 2;
            }
        },
    };
    if message.trim().is_empty() {
        note!(ui, "findopera: nothing to send");
        note!(ui, "  help: findopera feedback \"what went wrong\"");
        return 2;
    }

    // Which version this came from is the first thing anyone reading it will
    // want, and the last thing anyone thinks to include.
    let about = match &args.about {
        Some(text) => format!("{} — {text}", api::USER_AGENT),
        None => api::USER_AGENT.to_string(),
    };

    let api = match client(ui, &args.endpoint, args.token.as_ref()) {
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
            note!(ui, "findopera: {e}");
            return 3;
        }
    };
    if let Some(said) = api::refusal(&payload) {
        note!(ui, "findopera: {said}");
        return 3;
    }

    note!(ui, "findopera: sent — thank you");
    if args.email.is_none() {
        note!(
            ui,
            "findopera: no reply is possible; pass --email if you want one"
        );
    }
    0
}

/// Ask the server for a token, and say who it made you.
fn request_token(ui: &mut Session, args: &LoginArgs) -> Result<(String, String), i32> {
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
            note!(ui, "findopera: {e}");
            return Err(3);
        }
    };
    if let Some(said) = api::refusal(&payload) {
        note!(ui, "findopera: {said}");
        return Err(3);
    }
    let issued = &payload["data"]["createAccessToken"];
    match (issued["token"].as_str(), issued["username"].as_str()) {
        (Some(token), Some(username)) => Ok((token.to_string(), username.to_string())),
        _ => {
            note!(ui, "findopera: the server issued no token");
            Err(3)
        }
    }
}

fn cmd_login(ui: &mut Session, args: LoginArgs) -> i32 {
    if args.new {
        let (token, username) = match request_token(ui, &args) {
            Ok(pair) => pair,
            Err(code) => return code,
        };
        return match credentials::store(&token) {
            Ok(path) => {
                println!("{}", path.display());
                note!(ui, "findopera: your edits will be recorded as {username}");
                if args.email.is_none() {
                    note!(
                        ui,
                        "findopera: nothing to reach you on. `--email` is optional, and only used \
                     if\n\x20           something you are doing turns out to be blocked."
                    );
                }
                // Only the server can show the token, and only once. Saying so
                // here is cheaper than someone discovering it later.
                note!(
                    ui,
                    "findopera: the token itself is in that file and nowhere else — \
                     findopera.com keeps only a hash of it"
                );
                0
            }
            Err(why) => {
                note!(ui, "findopera: {why}");
                1
            }
        };
    }
    let token = match std::io::read_to_string(std::io::stdin()) {
        Ok(t) => t.trim().to_string(),
        Err(e) => {
            note!(
                ui,
                "findopera: cannot read the token from standard input: {e}"
            );
            return 2;
        }
    };
    if token.is_empty() {
        note!(ui, "findopera: no token given");
        note!(ui, "  help: findopera login < token.txt");
        return 2;
    }
    match credentials::store(&token) {
        Ok(path) => {
            // The path, not the token. Nothing should print the token, and the
            // path is what someone would want in order to remove it by hand.
            println!("{}", path.display());
            note!(ui, "findopera: requests will now say who is making them");
            0
        }
        Err(why) => {
            note!(ui, "findopera: {why}");
            1
        }
    }
}

fn cmd_logout(ui: &mut Session) -> i32 {
    match credentials::forget() {
        Ok(true) => {
            note!(ui, "findopera: the stored token is gone");
            0
        }
        Ok(false) => {
            note!(ui, "findopera: there was no stored token");
            0
        }
        Err(why) => {
            note!(ui, "findopera: {why}");
            1
        }
    }
}

fn cmd_organize(ui: &mut Session, args: OrganizeArgs) -> i32 {
    let api = match client(ui, &args.endpoint, args.token.as_ref()) {
        Ok(c) => c,
        Err(code) => return code,
    };
    let p = match prepare(
        ui,
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
        if args.json {
            let rows: Vec<serde_json::Value> =
                plan.rows.iter().map(|r| row_json(r, None)).collect();
            emit(
                ui,
                format_args!(
                    "{}",
                    serde_json::to_string_pretty(&rows).unwrap_or_default()
                ),
            );
            report(ui, &plan, p.require_variants);
            return i32::from(plan.blocked(p.require_variants));
        }
        for line in &listing {
            if !emit(ui, format_args!("{line}")) {
                return 0;
            }
        }
        report(ui, &plan, p.require_variants);
        note!(
            ui,
            "findopera: no destination set, so these are only the folder names. Add one to \
             build it:\n\x20   destination = \"/path/to/named\""
        );
        return i32::from(plan.blocked(p.require_variants));
    };

    if let Err(why) = apply::preflight(&plan, &args.root, &destination, link, p.require_variants) {
        report(ui, &plan, p.require_variants);
        note!(ui, "findopera: {why}");
        return 2;
    }

    // Say where, before doing it. Nothing on the command line names the
    // destination, so this is the only place it is stated.
    let what = match link {
        config::Link::Symlink => "a link to each folder",
        config::Link::Hardlink => "a hard link to every file",
        config::Link::Reflink => "a clone of every file",
        config::Link::Copy => "a copy of every file",
    };
    note!(
        ui,
        "findopera: {} {} in {}",
        if dry_run { "would build" } else { "building" },
        what,
        destination.display()
    );

    // What an earlier run left. Absent is ordinary — preflight has already
    // established the destination is empty in that case.
    let previous = match crate::state::load(&destination) {
        Ok(state) => state,
        Err(crate::state::StateError::Absent) => Default::default(),
        Err(crate::state::StateError::Unreadable(why)) => {
            note!(ui, "findopera: {why}");
            return 2;
        }
    };

    let (gone, done) = apply::reconcile(&previous, &plan, &destination, link, dry_run);

    if !dry_run {
        // Written after the fact, so a run that died halfway leaves a record
        // of what was there before rather than what it meant to make.
        let mut keeping = apply::built(&plan, &done, link);
        keeping.sort_by(|a, b| a.path.cmp(&b.path));
        if let Err(why) = crate::state::save(&destination, keeping) {
            note!(ui, "findopera: {why}");
            note!(
                ui,
                "    The tree is built, but without this record the next run cannot tell \
                 what it made."
            );
            return 1;
        }

        // After the record, and never read back: the list is an answer to
        // somebody's question rather than anything this program relies on.
        if let Some(index) = p.settings.as_ref().and_then(|s| s.index.as_ref()) {
            match crate::index::parse(&index.template) {
                Ok(template) => {
                    let at = destination.join(&index.file);
                    let header = index
                        .header
                        .as_deref()
                        .and_then(|h| crate::index::parse_header(h).ok());
                    let body =
                        crate::index::render(&plan, &p.recordings, &template, header.as_ref());
                    match std::fs::write(&at, format!("{body}\n")) {
                        Ok(()) => note!(ui, "findopera: listed them in {}", at.display()),
                        // The tree is built, which is the part that matters.
                        Err(e) => note!(ui, "findopera: cannot write {}: {e}", at.display()),
                    }
                }
                // Settled when the settings were read; here for completeness.
                Err(e) => note!(
                    ui,
                    "findopera: the index template is not valid: {}",
                    e.message
                ),
            }
        }
    }
    if args.json {
        let rows: Vec<serde_json::Value> = plan
            .rows
            .iter()
            .zip(&done.entries)
            .map(|(row, entry)| row_json(row, Some(&entry.outcome)))
            .collect();
        emit(
            ui,
            format_args!(
                "{}",
                serde_json::to_string_pretty(&rows).unwrap_or_default()
            ),
        );
        report(ui, &plan, p.require_variants);
        return i32::from(done.troubled() || plan.blocked(p.require_variants));
    }
    // Removals first, because that is the order they happen in. Printed the
    // other way round, a build nested under a removal reads as a folder made
    // inside one about to go — which is what this program used to do, and
    // what somebody checking whether it still does will think they are
    // looking at. It cost a reader that once already.
    for path in &gone.removed {
        if !emit(ui, format_args!("- {}", relative(path, &destination))) {
            return 0;
        }
    }
    for (line, entry) in listing.iter().zip(&done.entries) {
        let mark = match &entry.outcome {
            apply::Outcome::Created => '+',
            apply::Outcome::Skipped => ' ',
            apply::Outcome::Conflict(_) | apply::Outcome::Failed(_) => '!',
        };
        if !emit(ui, format_args!("{mark} {line}")) {
            return 0;
        }
    }
    for entry in &done.entries {
        if let apply::Outcome::Conflict(why) | apply::Outcome::Failed(why) = &entry.outcome {
            note!(ui, "findopera: {}", entry.destination.display());
            note!(ui, "    {why}");
        }
    }
    for (path, why) in &gone.changed {
        note!(ui, "findopera: {}", path.display());
        note!(ui, "    {why} — left alone");
    }
    for (path, why) in &gone.failed {
        note!(ui, "findopera: cannot remove {}: {why}", path.display());
    }
    report(ui, &plan, p.require_variants);

    let (made, skipped, trouble) = done.counts();
    let removed = gone.removed.len();
    note!(
        ui,
        "findopera: {made} {}, {skipped} already there, {trouble} left alone{}",
        if dry_run { "to build" } else { "built" },
        if removed == 0 {
            String::new()
        } else if dry_run {
            format!(", {removed} to remove")
        } else {
            format!(", {removed} removed")
        }
    );
    // Only worth saying when there is something to write; a run that found
    // everything already in place has nothing to offer.
    if dry_run && made > 0 {
        note!(ui, "findopera: nothing was written. To build it, run:");
        note!(ui, "    {}", rerun_with_write(&args));
    }
    i32::from(done.troubled() || plan.blocked(p.require_variants))
}

/// A path as it reads inside the destination.
fn relative(path: &Path, destination: &Path) -> String {
    path.strip_prefix(destination)
        .unwrap_or(path)
        .display()
        .to_string()
}

/// Whatever the plan has to say, on stderr, with its indented lines kept.
fn report(ui: &mut Session, plan: &plan::Plan, strict: bool) {
    let lines = plan.report(strict);
    if lines.is_empty() {
        return;
    }
    note!(ui, "");
    for line in lines {
        if line.starts_with(' ') {
            note!(ui, "{line}");
        } else {
            note!(ui, "findopera: {line}");
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

fn cmd_graphql(ui: &mut Session, args: GraphqlArgs) -> i32 {
    let query = match read_query(&args) {
        Ok(q) => q,
        Err(why) => {
            note!(ui, "findopera: {why}");
            return 2;
        }
    };
    if query.trim().is_empty() {
        note!(ui, "findopera: no query given");
        return 2;
    }

    // Parsed here rather than passed through as text, so that a typo in the
    // variables is caught before the round trip and reported as the caller's
    // mistake instead of the server's.
    let variables = match args.variables.as_deref().map(serde_json::from_str) {
        None => None,
        Some(Ok(v)) => Some(v),
        Some(Err(e)) => {
            note!(ui, "findopera: --variables is not valid JSON: {e}");
            return 2;
        }
    };

    let api = match client(ui, &args.endpoint, args.token.as_ref()) {
        Ok(c) => c,
        Err(code) => return code,
    };
    let payload = match api.post(&query, variables) {
        Ok(p) => p,
        Err(e) => {
            note!(ui, "findopera: {e}");
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
    if !emit(ui, format_args!("{rendered}")) {
        return 0;
    }

    match api::refusal(&payload) {
        Some(said) => {
            note!(ui, "findopera: {said}");
            // Anonymous is enough to read, so a refusal is where the lack of a
            // token first shows up — and "Unauthorized" is not obviously about
            // this program rather than about the account behind it.
            if !api.is_identified() {
                note!(
                    ui,
                    "  note: this request was anonymous. Changing anything needs a token —"
                );
                note!(ui, "        see `findopera login --help`");
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

fn cmd_schema(ui: &mut Session, args: SchemaArgs) -> i32 {
    let api = match client(ui, &args.endpoint, args.token.as_ref()) {
        Ok(c) => c,
        Err(code) => return code,
    };
    let sdl = match api.schema() {
        Ok(s) => s,
        Err(e) => {
            note!(ui, "findopera: {e}");
            return 3;
        }
    };
    let Some(name) = args.name else {
        emit(ui, format_args!("{}", sdl.trim_end()));
        return 0;
    };
    match definition(&sdl, &name) {
        Some(block) => {
            emit(ui, format_args!("{block}"));
            0
        }
        None => {
            note!(ui, "findopera: the schema has no `{name}`");
            // The names are the schema's own, so the only honest suggestion is
            // to go and look at them.
            note!(ui, "  help: `findopera schema` prints all of it");
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

fn cmd_init(ui: &mut Session, args: InitArgs) -> i32 {
    let path = args.dir.join(config::FILE_NAME);
    if path.exists() {
        note!(ui, "findopera: {} is already there", path.display());
        return 2;
    }
    match std::fs::write(&path, config::starter()) {
        Ok(()) => {
            println!("{}", path.display());
            note!(
                ui,
                "findopera: edit the template in there, then run `findopera organize`"
            );
            0
        }
        Err(e) => {
            note!(ui, "findopera: cannot write {}: {e}", path.display());
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
    ui: &mut Session,
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
            note!(ui, "findopera: {e}");
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
            note!(ui, "findopera: {why}");
            return Err(2);
        }
    };
    let report = scan::scan(root, follow_links, &ignore);
    for (path, why) in &report.unreadable {
        note!(ui, "findopera: {}: {why}", path.display());
    }
    if report.markers.is_empty() {
        note!(ui, "findopera: no marker files under {}", root.display());
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
            note!(ui, "findopera: {e}");
            for line in e.underline(&template) {
                note!(ui, "  {line}");
            }
            if let Some(help) = &e.help {
                note!(ui, "  help: {help}");
            }
            // The engine has no business knowing what this program is called,
            // so the pointer to it is added here. `template` lists the syntax as
            // well as the fields, which makes it the right answer whether the
            // template named something that is not there or was malformed.
            note!(
                ui,
                "  see `findopera template` for every field and the syntax"
            );
            return Err(2);
        }
    };

    let ids: Vec<String> = report.markers.iter().map(|m| m.id.clone()).collect();
    let recordings = match api.recordings(&ids) {
        Ok(r) => r,
        Err(e) => {
            note!(ui, "findopera: {e}");
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

fn cmd_template(ui: &mut Session) -> i32 {
    // The list is the result and goes to stdout, so `findopera template | grep
    // singer` stays useful. Everything explaining it goes to stderr.
    note!(ui, "{}", crate::SYNTAX);
    note!(
        ui,
        "\nA field marked `always` has a value for every recording and needs no\n\
         fallback. The rest want one — {{{{field|\"Unknown\"}}}} — or a [ … ] around\n\
         them, and findopera will say so if they have neither.\n"
    );

    // The same schema a template is actually checked against: the model's
    // fields, plus the one that comes from the marker rather than the
    // recording. Listing only the model's left `variant` out of a command
    // whose whole job is to say what a template may use.
    let mut schema: Vec<FieldDoc> = FIELDS.to_vec();
    schema.push(scan::VARIANT);

    let width = schema.iter().map(|f| f.path.len()).max().unwrap_or(0);
    for f in &schema {
        let always = if f.nullable { "      " } else { "always" };
        if !emit(
            ui,
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
fn render(ui: &mut Session, value: &serde_json::Value, indent: usize) -> bool {
    let pad = " ".repeat(indent);
    let serde_json::Value::Object(map) = value else {
        return emit(ui, format_args!("{pad}{}", scalar(value)));
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
                if !emit(ui, format_args!("{pad}{key}")) {
                    return false;
                }
                if !render(ui, child, indent + 2) {
                    return false;
                }
            }
            serde_json::Value::Array(items) => {
                if items.is_empty() {
                    if !emit(ui, format_args!("{pad}{key:width$}  —")) {
                        return false;
                    }
                    continue;
                }
                // A list of plain values belongs on one line; a list of records
                // does not.
                if items.iter().all(|i| !i.is_object()) {
                    let joined: Vec<String> = items.iter().map(scalar).collect();
                    if !emit(ui, format_args!("{pad}{key:width$}  {}", joined.join(", "))) {
                        return false;
                    }
                    continue;
                }
                if !emit(ui, format_args!("{pad}{key}")) {
                    return false;
                }
                for item in items {
                    if !render(ui, item, indent + 2) {
                        return false;
                    }
                    if !emit(ui, format_args!("")) {
                        return false;
                    }
                }
            }
            _ => {
                if !emit(ui, format_args!("{pad}{key:width$}  {}", scalar(child))) {
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
fn kind_named(ui: &mut Session, name: &str) -> Result<&'static crud::Type, i32> {
    if let Some(t) = crud::TYPES.iter().find(|t| t.name == name) {
        return Ok(t);
    }
    note!(ui, "findopera: there is no type called `{name}`");
    note!(ui, "  help: `findopera describe` lists them all");
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
    extras: &[crud::Extra],
    what: &str,
) -> Result<(), String> {
    let serde_json::Value::Object(map) = value else {
        return Err(format!(
            "a {what} takes a JSON object, with one key per field"
        ));
    };
    for key in map.keys() {
        if accepted.iter().any(|f| f.name == key) || extras.iter().any(|e| e.name == key) {
            continue;
        }
        let near = accepted
            .iter()
            .map(|f| f.name)
            .chain(extras.iter().map(|e| e.name))
            .find(|name| name.eq_ignore_ascii_case(key))
            .map(|name| format!(" — did you mean `{name}`?"));
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
            &[],
            "singer",
        )
        .expect_err("a typo is refused");
        // Case is the likeliest slip, and the server would answer it with a
        // GraphQL error naming an input type the caller never mentioned.
        assert!(why.contains("firstName"), "got: {why}");
    }

    #[test]
    fn the_objects_inside_a_list_are_checked_too() {
        // Without this a list validated as "an array" and nothing more, so a
        // key misspelled one level down reached the server and came back as an
        // error about an input type the caller never named.
        let recording = crud::TYPES.iter().find(|t| t.name == "recording").unwrap();
        let extras = recording.composite.as_ref().unwrap().extras;

        let why = super::check_elements(
            &serde_json::json!({ "portrayalInputs": [
                { "singerid": "133", "characterId": "908", "noted": true }
            ] }),
            extras,
        )
        .expect_err("a misspelled key inside a portrayal");
        assert!(why.contains("singerId"), "got: {why}");
        assert!(why.contains("portrayalInputs 1"), "got: {why}");
    }

    #[test]
    fn noted_is_required_on_a_portrayal_given_to_a_create() {
        // It is optional on a portrayal created by itself and required here,
        // which is not something anyone would guess, so it is worth failing on
        // before the request rather than after.
        let recording = crud::TYPES.iter().find(|t| t.name == "recording").unwrap();
        let extras = recording.composite.as_ref().unwrap().extras;
        let cast = extras.iter().find(|e| e.name == "portrayalInputs").unwrap();
        assert!(cast.items.iter().any(|f| f.name == "noted" && f.required));

        assert!(super::check_elements(
            &serde_json::json!({ "portrayalInputs": [
                { "singerId": "133", "characterId": "908" }
            ] }),
            extras,
        )
        .is_err());
    }

    #[test]
    fn a_list_is_described_well_enough_to_validate_against() {
        // `describe --json` says it is a schema in the ordinary sense. A list
        // with no `items` cannot check the one part of an input with any
        // structure, which is what made this worth fixing.
        let recording = crud::TYPES.iter().find(|t| t.name == "recording").unwrap();
        let schema = super::json_schema(recording);
        let items = &schema["properties"]["portrayalInputs"]["items"];
        assert_eq!(items["type"], "object");
        assert_eq!(items["additionalProperties"], false);
        let required: Vec<&str> = items["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert!(required.contains(&"noted"), "got: {required:?}");
    }

    #[test]
    fn a_cast_needs_exactly_three_top_billed() {
        let cast = |noted: usize, total: usize| {
            serde_json::json!({ "portrayalInputs":
                (0..total).map(|i| serde_json::json!({ "noted": i < noted })).collect::<Vec<_>>() })
        };
        // Refused here rather than after twelve roles have crossed the wire.
        assert!(super::check_cast(&cast(4, 4)).is_err());
        assert!(super::check_cast(&cast(0, 12)).is_err());
        assert!(super::check_cast(&cast(3, 12)).is_ok());
    }

    #[test]
    fn a_cast_smaller_than_three_may_have_all_of_it_billed() {
        let cast = |noted: usize, total: usize| {
            serde_json::json!({ "portrayalInputs":
                (0..total).map(|i| serde_json::json!({ "noted": i < noted })).collect::<Vec<_>>() })
        };
        assert!(super::check_cast(&cast(2, 2)).is_ok());
        assert!(super::check_cast(&cast(1, 1)).is_ok());
        // But not fewer than it has.
        assert!(super::check_cast(&cast(1, 2)).is_err());
    }

    #[test]
    fn no_cast_at_all_is_not_a_cast_problem() {
        // A plain create goes through the same mutation with nothing in it, so
        // the rule must not fire on a recording that names no roles.
        assert!(super::check_cast(&serde_json::json!({ "operaId": "88" })).is_ok());
        assert!(super::check_cast(&serde_json::json!({ "portrayalInputs": [] })).is_ok());
    }

    #[test]
    fn a_composite_type_accepts_its_extra_keys() {
        // The extras are arguments of the mutation rather than fields of the
        // input object, so they would otherwise read as unknown fields.
        let recording = crud::TYPES.iter().find(|t| t.name == "recording").unwrap();
        let extras = recording
            .composite
            .as_ref()
            .expect("recording has one")
            .extras;
        assert!(extras.iter().any(|e| e.name == "portrayalInputs"));
        super::check_input(
            &serde_json::json!({ "operaId": "88", "conductorId": "106", "upc": "012" }),
            recording.create,
            extras,
            "recording",
        )
        .expect("upc is one of the extras");
    }

    #[test]
    fn a_missing_required_field_is_named() {
        let fields = &[crud::InputField {
            name: "firstName",
            json: "string",
            required: true,
            about: "",
        }];
        let why = super::check_input(&serde_json::json!({}), fields, &[], "singer")
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
            &[],
            "singer",
        )
        .expect("the optional one is optional");
    }

    #[test]
    fn something_that_is_not_an_object_is_refused() {
        let why = super::check_input(&serde_json::json!([1, 2]), &[], &[], "singer")
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
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let mut ui = super::Session::new(&mut out, &mut err);
        for t in crud::TYPES {
            assert!(
                super::kind_named(&mut ui, t.name).is_ok(),
                "{} is not findable",
                t.name
            );
        }
        assert!(super::kind_named(&mut ui, "no-such-type").is_err());

        // Now that the session is a value rather than the process's stderr,
        // what it said about the miss is part of what this can check.
        let said = String::from_utf8_lossy(&err);
        assert!(
            said.contains("no type called `no-such-type`"),
            "got: {said}"
        );
        assert!(
            said.contains("`findopera describe` lists them all"),
            "got: {said}"
        );
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
