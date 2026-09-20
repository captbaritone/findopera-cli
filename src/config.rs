//! The settings that should not change between runs.
//!
//! A template typed at the prompt is a template that will eventually be typed
//! differently, and a library named by two slightly different templates is
//! worse than one named by either. So the template lives in a file, next to
//! the markers it reads.
//!
//! ```toml
//! template = '''
//! {{composer.lastName}}[ ({{composer.dates}})]/{{opera.title}}
//! '''
//! require-variants = false
//! ```
//!
//! The template goes in a `'''` block, which is the only form TOML offers
//! where what you write is exactly what you would have typed at the prompt:
//! a plain `'…'` string cannot hold the apostrophe in `{{opera.title|"L'…"}}`,
//! and a `"…"` string needs every `\[` written `\\[`.
//!
//! There is no searching up the tree. The file is the one beside what you
//! scanned, or the one you named with `--config` — so a source can carry
//! several, one per way of naming it, and which one ran is never a guess.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The file `organize` looks for beside the library it is reading.
pub const FILE_NAME: &str = "findopera.toml";

/// How a recording's folder gets to the destination.
///
/// These are not four spellings of one operation; the system forces them
/// apart. A directory cannot be hard-linked or cloned at all, so `Hardlink`,
/// `Reflink` and `Copy` have to walk the folder and treat each file
/// separately, while `Symlink` is a single link to the folder itself — which
/// stays live, so a track added to the source turns up in the destination
/// without another run. Hard links also cannot cross a filesystem, which rules
/// them out between a network mount and a local disk.
///
/// `Reflink` sits between hard-linking and copying, and exists for the case
/// neither covers. A clone is a real, independent file that happens to share
/// its contents with the original until one of them is written to, so it costs
/// nothing to make and nothing to keep. Unlike a hard link it may cross a
/// boundary that a hard link cannot: on btrfs every share or subvolume reports
/// its own device id, so `ln` there fails with `Invalid cross-device link`
/// while a clone succeeds. That is the whole reason to reach for it — a NAS
/// that puts each share in its own subvolume cannot hard-link a library into a
/// second share, and copying it would take the space twice.
///
/// It needs a filesystem that supports cloning: btrfs, XFS with reflinks, ZFS
/// on recent versions, APFS, or ReFS. Elsewhere it fails, and says so.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Link {
    #[default]
    Symlink,
    Hardlink,
    Reflink,
    Copy,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Config {
    /// The name each recording's folder should have.
    pub template: String,
    /// Where `organize --write` builds the folders.
    #[serde(default)]
    pub destination: Option<PathBuf>,
    /// How each folder gets there.
    #[serde(default)]
    pub link: Link,
    /// Refuse to number two folders that want the same name.
    #[serde(default)]
    pub require_variants: bool,
    /// Follow symlinks while walking.
    #[serde(default)]
    pub follow_links: bool,
    /// Folders to leave alone.
    #[serde(default)]
    pub ignore: Vec<String>,
    /// A list of the collection, left at the destination for someone to read.
    #[serde(default)]
    pub index: Option<Index>,
    /// The list on findopera.com this library is a copy of.
    #[serde(default)]
    pub collection: Option<CollectionRef>,
}

/// Which list `organize --write` keeps in step with this library.
///
/// A list is named by its id, never by its name: renaming one is a supported
/// thing to do and the schema promises the id and contents survive it, so a
/// setting keyed on the name would break on an ordinary operation. The id is
/// the identity.
///
/// An address rather than the bare number, and only an address: two ways of
/// naming one list is a question for whoever reads the file and a branch for
/// whoever maintains it, in exchange for nothing. This is also the spelling a
/// browser puts on the clipboard.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct CollectionRef {
    /// The list's page, as the address bar gives it.
    pub url: String,
}

/// The id inside a collection's address.
///
/// Read out of the path rather than matched against a whole address, so that
/// a trailing slash, a query string, a port, or a findopera running somewhere
/// other than findopera.com all still work. What it insists on is the
/// `collection` segment: a bare number would be a typo as easily as an id,
/// and pointing a sync at the wrong list is the expensive mistake here.
fn id_in_url(url: &str) -> Option<i64> {
    let mut parts = url
        .split(['/', '?', '#'])
        .filter(|p| !p.is_empty())
        .peekable();
    while let Some(part) = parts.next() {
        if part.eq_ignore_ascii_case("collection") {
            return parts.peek().and_then(|n| n.parse().ok());
        }
    }
    None
}

impl CollectionRef {
    /// The list this address names.
    pub fn resolve(&self) -> Result<i64, String> {
        id_in_url(&self.url).ok_or_else(|| {
            format!(
                "`{}` does not look like a list's address. It should have a `collection` part \
                 with the list's number after it, as in https://findopera.com/collection/1",
                self.url
            )
        })
    }
}

/// What to write, and what each line should say.
///
/// A file rather than a flag, because the name is the whole of how anybody
/// finds it: somebody browsing the destination has to notice it without being
/// told, so it is worth choosing a name that says what it is and sorts near
/// the top.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Index {
    /// The name to write it under, inside the destination.
    pub file: PathBuf,
    /// One line of it, in the same language folder names are written in.
    pub template: String,
    /// A note at the top: what this is, and how to read a line of it.
    ///
    /// Somebody opening the file has no other context, and the line format is
    /// yours — so only you can say what `(1959.07)` and `[flac, mp3]` mean.
    /// It sits beside the template it describes, which is as close as the two
    /// can be kept.
    #[serde(default)]
    pub header: Option<String>,
}

#[derive(Debug)]
pub enum ConfigError {
    Missing { path: PathBuf },
    Unreadable { path: PathBuf, why: String },
    Invalid { path: PathBuf, why: String },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing { path } => write!(
                f,
                "no {FILE_NAME} at {} — run `findopera init` there to write one, \
                 or pass --template",
                path.display()
            ),
            Self::Unreadable { path, why } => write!(f, "cannot read {}: {why}", path.display()),
            // Indented like every other continuation, so what the settings
            // parser said reads as part of this message rather than as
            // something that escaped alongside it.
            Self::Invalid { path, why } => {
                let quoted: String = why
                    .to_string()
                    .lines()
                    .map(|l| format!("    {l}").trim_end().to_string())
                    .collect::<Vec<_>>()
                    .join("\n");
                write!(f, "{} is not valid:\n{quoted}", path.display())
            }
        }
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    pub fn load(path: &Path) -> Result<Config, ConfigError> {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(ConfigError::Missing {
                    path: path.to_path_buf(),
                })
            }
            Err(e) => {
                return Err(ConfigError::Unreadable {
                    path: path.to_path_buf(),
                    why: e.to_string(),
                })
            }
        };
        let config: Config = toml::from_str(&text).map_err(|e| ConfigError::Invalid {
            path: path.to_path_buf(),
            why: explain(&e.to_string()),
        })?;
        // A pattern that cannot be compiled is a mistake in this file, so it
        // is reported against this file — not later, halfway through a walk,
        // with nothing to say which line it came from.
        crate::scan::Ignore::new(&config.ignore).map_err(|why| ConfigError::Invalid {
            path: path.to_path_buf(),
            why,
        })?;
        // Checked here, against this file, for the same reason a pattern is:
        // a template that will not parse is a mistake in the settings, and
        // saying so at the end of a build is saying so too late.
        if let Some(index) = &config.index {
            // Framed the way a folder template's mistake is framed. It is the
            // same language and the same class of mistake, and an answer that
            // names the field but not where it sits is a worse answer for
            // being in a different file.
            crate::index::parse(&index.template).map_err(|e| {
                let mut why = format!("the list template is not valid: {e}");
                for line in e.underline(&index.template) {
                    why.push_str(&format!("\n  {line}"));
                }
                if let Some(help) = &e.help {
                    why.push_str(&format!("\n  help: {help}"));
                }
                why.push_str("\n  see `findopera template` for every field and the syntax");
                ConfigError::Invalid {
                    path: path.to_path_buf(),
                    why,
                }
            })?;
            if let Some(header) = &index.header {
                crate::index::parse_header(header).map_err(|e| {
                    let mut why = format!("the list header is not valid: {e}");
                    for line in e.underline(header) {
                        why.push_str(&format!("\n  {line}"));
                    }
                    if let Some(help) = &e.help {
                        why.push_str(&format!("\n  help: {help}"));
                    }
                    why.push_str("\n  a header may name {{count}} and {{date}}, and nothing else");
                    ConfigError::Invalid {
                        path: path.to_path_buf(),
                        why,
                    }
                })?;
            }
            if index.file.as_os_str().is_empty() {
                return Err(ConfigError::Invalid {
                    path: path.to_path_buf(),
                    why: "the index needs a filename to be written under".to_string(),
                });
            }
        }
        // Checked here rather than where it is used, because where it is used
        // is the far end of a build: an address with a typo in it should be a
        // refusal before any folders are made, naming the file it is in, and
        // not a surprise after the tree is finished.
        if let Some(reference) = &config.collection {
            reference.resolve().map_err(|why| ConfigError::Invalid {
                path: path.to_path_buf(),
                why,
            })?;
        }
        Ok(config)
    }
}

/// TOML's own message, plus a hint where its wording will not land.
///
/// `template = {{opera.title}}` is the mistake everyone makes once, and TOML
/// reports it as "missing key for inline table element", because `{` opens an
/// inline table. That is true and useless: nothing about it says the value
/// needed quoting.
fn explain(message: &str) -> String {
    if message.contains("inline table") {
        return format!(
            "{message}\n\
             hint: a template has to sit in a `'''` block, on its own lines:\n\
             \x20         template = '''\n\
             \x20         {{{{composer.lastName}}}}/{{{{opera.title}}}}\n\
             \x20         '''"
        );
    }
    message.to_string()
}

/// The file `findopera init` writes.
///
/// Every setting is present and explained, so nobody has to start from an
/// empty file wondering what the keys are.
pub fn starter(collection: Option<&str>) -> String {
    // The syntax is described once, in the template module, and quoted here.
    // Two copies of it would eventually say different things.
    let syntax = crate::SYNTAX
        .lines()
        .map(|l| format!("#   {l}"))
        .collect::<Vec<_>>()
        .join("\n");
    let head = r#"# How this library is organised.
#
# Run `findopera organize` in this folder to see what each recording's folder
# would be called. Nothing is renamed or moved; it only ever prints.

# The name to give each folder.
#
"#;
    let rest = r#"
#
# For every field you can use here, run `findopera template`. The ones it marks
# `always` need no fallback; the rest want one, or a [ … ] around them, and
# `findopera organize` will say so if they have neither.
template = '''
{{composer.lastName}}/{{opera.title}}/[{{year}} ]{{conductor.lastName}} \[{{id}}\][ - {{variant}}]
'''

# A fuller one, in use on a real library, if you want more in the name. It
# gives the composer their first name and dates, the opera its English title
# where there is one, and the recording its noted singers — each in a group, so
# any of them being absent drops just that part rather than leaving brackets
# with nothing in them.
#
# template = '''
# {{composer.lastName}}, {{composer.firstName}}[ ({{composer.dates}})]/{{opera.title}}[ ({{opera.englishTitle}})]/[{{year}} ]{{conductor.lastName}}[ ({{singers.lastNames}})][ \[{{variant}}\]]
# '''

# The id and the variant are both there on purpose. Two recordings of one
# opera in one year by one conductor are common, and two rips of a single
# recording are commoner still; without something to tell them apart, findopera
# has to stop and say so. Take them out only once you know your library has no
# duplicates in it.
#
# A fuller scheme, if you want more in the name:
#
#   {{composer.lastName}}, {{composer.firstName}}[ ({{composer.dates}})]/{{opera.title}}/[{{year}} ]{{conductor.lastName}}[ ({{singers.lastNames}})] \[{{id}}\][ - {{variant}}]

# Two folders can hold the same recording — a FLAC rip and an MP3 rip of one
# performance. Nothing in the recording tells them apart, so say which is
# which in the marker's name: findopera-332 flac.txt and findopera-332 mp3.txt.
#
# Where you have not, findopera numbers them 1, 2, 3 and says so. Those
# numbers move around as the library changes, so turn this on to be stopped
# instead of numbered.
require-variants = false

# Where `findopera organize --write` builds the folders. Until this is set it
# has nowhere to build, and only shows what it would call things.
#
# destination = "/path/to/named"

# How each folder gets there.
#
#   symlink    one link pointing at the folder — nothing is copied, and a
#              track added to the original turns up here too
#   hardlink   every file linked separately, sharing its contents. Cannot
#              cross a disk, so the destination must be on the same one
#   reflink    every file cloned. Independent files that share their contents
#              until one is written to, so they cost nothing — and unlike a
#              hard link a clone can cross between two shares of one btrfs
#              filesystem, which is how a Synology keeps them apart. Needs a
#              filesystem that can clone: btrfs, XFS, ZFS, APFS, ReFS
#   copy       every file copied. Takes the space twice over
#
# Only folders this program built are ever removed, and nothing is written at all
# unless `findopera organize` is given --write.
link = "symlink"

# Folders to skip. A folder that matches is not looked inside at all, which
# on a network drive is worth more than it sounds — every folder looked at
# costs a round trip.
#
# A pattern is matched against a folder's own name and against its path from
# here, so "@eaDir" skips one wherever it turns up while "Unsorted/**" skips
# only that one.
#
# ignore = [
#   "@eaDir",          # Synology leaves these beside its media
#   "Artwork",
#   "Incomplete",      # half-finished downloads are not worth organising
#   "**/.grab/**",
# ]

# Follow symlinks while walking. Off by default: a library built out of
# symlinks would otherwise report every recording twice.
follow-links = false

# A list of the collection, left at the destination after a build.
#
# The tree can only be filed one way, and whichever way you choose hides the
# rest: a library by composer says nothing about who sang, and finding a Tosca
# in it means knowing that Tosca is Puccini's. This is the one file where the
# whole collection can be read at once, so give it a name somebody browsing
# will notice.
#
# One line per recording, however many rips of it there are — the tree has a
# folder for each, and `{{variants}}` names them together. `{{path}}` says
# where to go. `findopera template` lists every field.
#
# Written after a build, never read back: delete it, edit it, or take this
# section out, and nothing else changes.
#
# It stays at the bottom. Everything below a `[…]` heading belongs to it, so a
# setting written under this one would be read as part of the list rather than
# as a setting — put anything you add above here.

[index]
file = "00 - What is in here.txt"

# A note at the top. Whoever opens the file has no other context, and the
# shape of a line is yours — so the second line here says how to read one.
#
# It may name {{count}} and {{date}}, and nothing else: those are the two a
# person cannot keep right by hand, and the two that tell a reader whether
# they are looking at something current. Brackets are literal only when
# written `\[` and `\]`, as in a folder name.
header = '''
List of the {{count}} opera recordings in this collection, as of {{date}}.

Composer, first - Opera (year.month.day) Conductor \[singers\] \[rips\] (findopera id) — folder
'''

template = '''
{{composer.lastName}}, {{composer.firstName}} - {{opera.title}}[ ({{year}}[.{{month}}][.{{day}}])] {{conductor.lastName}}[ \[{{singers.lastNames}}\]][ \[{{variants}}\]] (findopera {{id}}) — {{path}}
'''

# The list on findopera.com this library is a copy of.
#
# With this set, `findopera organize --write` makes that list hold exactly the
# recordings the tree holds — so the catalog knows what you have, and
# `findopera search recording --local` can ask it.
#
# The address is the one in the browser's bar when you are looking at the
# list, and `findopera collection create` prints it for a list it has just
# made.
#
# It is the number that matters, not the name: renaming a list leaves its id
# and its contents alone, so this keeps pointing at the right one afterwards.
#
# A run that finds no recordings at all will not empty the list — an unplugged
# disk is not an answer — but one that finds fewer will remove the rest. The
# tree is what the list is a copy of.
#
"#;
    // Commented out unless a list has actually been made, because a setting
    // that points at nothing is worse than one that is not there: the first
    // run would refuse, naming an address nobody chose.
    let list = match collection {
        Some(url) => format!("[collection]\nurl = \"{url}\"\n"),
        None => "# [collection]\n# url = \"https://findopera.com/collection/1\"\n".to_string(),
    };
    format!("{head}{syntax}{rest}{list}")
}

#[cfg(test)]
mod collection_tests {
    use super::{id_in_url, CollectionRef};

    fn from(url: &str) -> CollectionRef {
        CollectionRef {
            url: url.to_string(),
        }
    }

    #[test]
    fn an_address_is_read_for_its_id() {
        // The shape the site gives today, and the shapes a clipboard tends to
        // add to it. None of these should need explaining to somebody who
        // copied the page they were looking at.
        for url in [
            "https://findopera.com/collection/1",
            "https://findopera.com/collection/1/",
            "http://findopera.com/collection/1",
            "findopera.com/collection/1",
            "/collection/1",
            "https://findopera.com/collection/1?from=somewhere",
            "https://findopera.com/collection/1#top",
            // A findopera somewhere else, which is what --endpoint is for.
            "http://localhost:3000/collection/1",
        ] {
            assert_eq!(id_in_url(url), Some(1), "could not read {url}");
        }
    }

    #[test]
    fn something_that_is_not_a_list_is_refused() {
        // A bare number is the interesting one: it would be a perfectly good
        // id and a perfectly good typo, and syncing to the wrong list replaces
        // its contents. The `collection` part is what makes it an address.
        for url in [
            "1",
            "https://findopera.com/",
            "https://findopera.com/singer/2/peter-glossop",
            "https://findopera.com/collection/",
            "https://findopera.com/collection/not-a-number",
        ] {
            assert_eq!(id_in_url(url), None, "{url} should not read as a list");
        }
    }

    #[test]
    fn an_address_resolves_and_a_number_alone_does_not() {
        assert_eq!(from("https://findopera.com/collection/7").resolve(), Ok(7));
        // The whole reason for insisting on the `collection` part: a bare
        // number is as likely to be a typo as an id, and the sync replaces
        // whatever list it is pointed at.
        assert!(from("7").resolve().is_err());
    }
}
