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
/// These are not three spellings of one operation; the system forces them
/// apart. A directory cannot be hard-linked at all, so `Hardlink` and `Copy`
/// have to walk the folder and treat each file separately, while `Symlink` is
/// a single link to the folder itself — which stays live, so a track added to
/// the source turns up in the destination without another run. Hard links also
/// cannot cross a filesystem, which rules them out between a network mount and
/// a local disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Link {
    #[default]
    Symlink,
    Hardlink,
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
            Self::Invalid { path, why } => write!(f, "{} is not valid:\n{why}", path.display()),
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
pub fn starter() -> String {
    // The syntax is described once, in the template module, and quoted here.
    // Two copies of it would eventually say different things.
    let syntax = crate::SYNTAX
        .lines()
        .map(|l| format!("#   {l}"))
        .collect::<Vec<_>>()
        .join("\n");
    let head = r#"# How this library is organized.
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
"#;
    format!("{head}{syntax}{rest}")
}
