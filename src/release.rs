//! Whether a newer findopera has been published, and how to get it.
//!
//! This is the one thing here that talks to GitHub rather than to
//! findopera.com, because releases are published there and nothing on
//! findopera.com knows this program exists.
//!
//! It only ever *reads*. Replacing the binary is left to whatever put it
//! where it is — a package manager, an installer, or a person who copied it
//! onto a NAS — because that is the thing that knows where it lives and what
//! else is expected to be alongside it. A program that overwrites itself
//! behind its installer's back leaves the installer's record of the world
//! wrong, and on the machines this is aimed at the binary may not be writable
//! by whoever is running it anyway.

use semver::Version;
use std::path::Path;

/// Where releases are published.
pub const RELEASES_API: &str =
    "https://api.github.com/repos/captbaritone/findopera-cli/releases/latest";

/// The version this binary was built as.
pub const CURRENT: &str = env!("CARGO_PKG_VERSION");

/// What a check found.
#[derive(Debug)]
pub struct Check {
    pub current: Version,
    pub latest: Version,
    /// How to install the newer one, given where this copy appears to live.
    pub how: How,
}

impl Check {
    pub fn newer_available(&self) -> bool {
        self.latest > self.current
    }
}

/// How this copy looks to have been installed, and so how to replace it.
///
/// A guess from where the binary sits, and said as a guess. Getting it wrong
/// costs someone one wrong command to read past; asking the machine to prove
/// it costs a dependency on every package manager that might have been used.
#[derive(Debug, PartialEq, Eq)]
pub enum How {
    /// Under a Cargo bin directory, so Cargo put it there.
    Cargo,
    /// Anywhere else: the installer script is the documented route.
    Installer,
}

impl How {
    /// The command to run, as a person would type it.
    pub fn command(&self) -> &'static str {
        match self {
            How::Cargo => "cargo install --git https://github.com/captbaritone/findopera-cli",
            How::Installer if cfg!(windows) => {
                "irm https://github.com/captbaritone/findopera-cli/releases/latest/download/findopera-installer.ps1 | iex"
            }
            How::Installer => {
                "curl --proto '=https' --tlsv1.2 -LsSf https://github.com/captbaritone/findopera-cli/releases/latest/download/findopera-installer.sh | sh"
            }
        }
    }
}

/// Which route a binary at this path came by.
///
/// Split out from the lookup so it can be tested without a filesystem: the
/// interesting cases are paths, not directories that happen to exist.
pub fn how_installed(exe: &Path) -> How {
    // `~/.cargo/bin/findopera`, or wherever CARGO_HOME was pointed. Matching
    // on the pair of components rather than on `.cargo` alone keeps a library
    // that happens to live under a folder called `bin` from being mistaken
    // for an installation.
    let components: Vec<_> = exe.iter().collect();
    let cargo_bin = components
        .windows(2)
        .any(|w| (w[0] == ".cargo" || w[0] == "cargo") && w[1] == "bin");
    if cargo_bin {
        How::Cargo
    } else {
        How::Installer
    }
}

/// What went wrong looking.
#[derive(Debug)]
pub enum Error {
    /// GitHub could not be reached, or would not answer.
    Unreachable(String),
    /// It answered with something this cannot read.
    Unreadable(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Unreachable(why) => {
                write!(f, "could not ask GitHub for the latest release: {why}")
            }
            Error::Unreadable(why) => write!(f, "GitHub answered with something unexpected: {why}"),
        }
    }
}

/// The version out of a releases API response.
///
/// Tags are written `v0.7.2`; the leading `v` is a convention of the tag and
/// not part of the version, so it comes off before parsing. A release with a
/// tag that is not a version at all is not an error worth stopping for
/// elsewhere, but it is one here: there is nothing to compare against.
pub fn latest_in(payload: &serde_json::Value) -> Result<Version, Error> {
    let tag = payload["tag_name"]
        .as_str()
        .ok_or_else(|| Error::Unreadable("the release has no tag_name".to_string()))?;
    Version::parse(tag.trim_start_matches('v'))
        .map_err(|e| Error::Unreadable(format!("`{tag}` is not a version: {e}")))
}

/// Ask GitHub what the latest release is.
///
/// Through the same seam every other request goes through, so a test can
/// answer this one too. It used to open its own socket, which meant the only
/// way to exercise the command around it was to let it reach GitHub.
pub fn check(
    transport: &dyn crate::api::Transport,
    url: &str,
    exe: Option<&Path>,
) -> Result<Check, Error> {
    let request = crate::api::Request {
        method: crate::api::Method::Get,
        url: url.to_string(),
        headers: vec![
            ("User-Agent", crate::api::USER_AGENT.to_string()),
            // The documented way to pin the shape of the answer, so a future
            // default cannot quietly change the field this reads.
            ("Accept", "application/vnd.github+json".to_string()),
        ],
        body: None,
    };
    let reply = transport.round_trip(&request).map_err(Error::Unreachable)?;

    if !reply.is_success() {
        // Worth naming: an unauthenticated caller gets sixty of these an hour
        // from one address, and "403" on its own would send someone looking
        // for a permission they do not need.
        let why = if reply.status == 403 || reply.status == 429 {
            "it is rate limiting this address, which it does after a number \
             of anonymous requests an hour. Try again later."
                .to_string()
        } else {
            format!("it answered {}", reply.status)
        };
        return Err(Error::Unreachable(why));
    }

    let payload: serde_json::Value =
        serde_json::from_str(&reply.body).map_err(|e| Error::Unreadable(e.to_string()))?;
    let latest = latest_in(&payload)?;
    let current = Version::parse(CURRENT)
        .map_err(|e| Error::Unreadable(format!("this binary's own version is unreadable: {e}")))?;

    Ok(Check {
        current,
        latest,
        how: exe.map_or(How::Installer, how_installed),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tag_is_read_without_its_v() {
        let payload = serde_json::json!({ "tag_name": "v0.8.0" });
        assert_eq!(latest_in(&payload).unwrap(), Version::new(0, 8, 0));
    }

    #[test]
    fn a_tag_without_a_v_reads_the_same() {
        let payload = serde_json::json!({ "tag_name": "0.8.0" });
        assert_eq!(latest_in(&payload).unwrap(), Version::new(0, 8, 0));
    }

    #[test]
    fn a_prerelease_does_not_count_as_newer_than_the_release_it_precedes() {
        // Ordinary semver, and the reason to use it rather than compare three
        // numbers: 0.8.0-rc.1 is *older* than 0.8.0, and a hand-rolled
        // comparison that stops at the patch number calls them equal.
        let current = Version::parse("0.8.0").unwrap();
        let latest = Version::parse("0.8.0-rc.1").unwrap();
        assert!(latest < current);
    }

    #[test]
    fn a_version_this_binary_cannot_read_is_an_error_not_a_shrug() {
        // Silently treating an unreadable tag as "no update" would make a
        // broken check indistinguishable from a current install.
        let payload = serde_json::json!({ "tag_name": "nightly" });
        assert!(matches!(latest_in(&payload), Err(Error::Unreadable(_))));
        let payload = serde_json::json!({});
        assert!(matches!(latest_in(&payload), Err(Error::Unreadable(_))));
    }

    #[test]
    fn a_binary_in_a_cargo_bin_directory_says_so() {
        assert_eq!(
            how_installed(Path::new("/Users/someone/.cargo/bin/findopera")),
            How::Cargo
        );
        // CARGO_HOME pointed elsewhere, which is common on CI and on shared
        // machines.
        assert_eq!(
            how_installed(Path::new("/opt/cargo/bin/findopera")),
            How::Cargo
        );
    }

    #[test]
    fn anything_else_is_taken_to_have_come_from_the_installer() {
        assert_eq!(
            how_installed(Path::new("/usr/local/bin/findopera")),
            How::Installer
        );
        assert_eq!(
            how_installed(Path::new("/Users/someone/.local/bin/findopera")),
            How::Installer
        );
        // A `bin` that is not a Cargo one.
        assert_eq!(
            how_installed(Path::new("/volume1/homes/me/bin/findopera")),
            How::Installer
        );
    }
}
