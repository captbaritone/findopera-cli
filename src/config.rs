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

use serde::Deserialize;
use std::path::{Path, PathBuf};

/// The file `scan` looks for beside what it is scanning.
pub const FILE_NAME: &str = "findopera.toml";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Config {
    /// The name each recording's folder should have.
    pub template: String,
    /// Refuse to number two folders that want the same name.
    #[serde(default)]
    pub require_variants: bool,
    /// Follow symlinks while walking.
    #[serde(default)]
    pub follow_links: bool,
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
        toml::from_str(&text).map_err(|e| ConfigError::Invalid {
            path: path.to_path_buf(),
            why: explain(&e.to_string()),
        })
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
    r#"# How this library is named.
#
# Run `findopera scan` in this folder to see what each recording's folder
# would be called. Nothing is renamed or moved; it only ever prints.

# The name to give each folder. `/` separates folder levels.
#
#   {{field}}            a value from the recording
#   {{a|b|"Unknown"}}    try a, then b, then fall back to the text
#   [ … ]                a part to leave out when what is inside it is missing
#   \[ \] \{ \}          a literal bracket or brace
#
# For every field you can use here, run `findopera fields`. Fields marked
# "always present" need no fallback; the rest want one, or a [ … ] around
# them, and `findopera scan` will say so if they have neither.
template = '''
{{composer.lastName}}/{{opera.title}}/[{{year}} ]{{conductor.lastName}} \[{{id}}\][ - {{variant}}]
'''

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

# Follow symlinks while walking. Off by default: a library built out of
# symlinks would otherwise report every recording twice.
follow-links = false
"#
    .to_string()
}
