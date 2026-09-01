//! Where the token lives between runs.
//!
//! Deliberately not [`crate::config`]. That file sits inside the library being
//! organized: [`crate::scan`] walks it, `organize --write` can link the folder
//! holding it into the destination tree, and libraries live on network shares
//! and in sync folders. A secret kept there leaks by construction, however
//! carefully it was written. This one goes in the user's own configuration
//! directory instead, readable by nobody else.

use std::io::Write;
use std::path::{Path, PathBuf};

/// The environment variable holding a token, for scripts and CI.
///
/// Anything running unattended should use this rather than a stored file:
/// there is nothing to install, and nothing left behind afterwards.
pub const ENV: &str = "FINDOPERA_TOKEN";

/// The token to use, from wherever this run keeps it.
///
/// An argument beats the environment, which beats what was stored — the more
/// deliberate the mention, the more it wins, so a one-off never needs the
/// stored token moved out of the way first.
/// A stored token that cannot be trusted is an error rather than a shrug: a
/// run that quietly carried on anonymously would look like a server that had
/// forgotten who you were, and the file would stay readable.
pub fn resolve(argument: Option<&str>) -> Result<Option<String>, String> {
    if let Some(token) = argument {
        return Ok(Some(token.to_string()));
    }
    if let Ok(token) = std::env::var(ENV) {
        if !token.trim().is_empty() {
            return Ok(Some(token.trim().to_string()));
        }
    }
    load()
}

/// Where the token is kept.
///
/// `XDG_CONFIG_HOME` if it is set, and the platform's own place otherwise, so
/// that this sits with everything else a person has configured rather than
/// inventing a dotfile of its own.
pub fn path() -> Option<PathBuf> {
    let dir = if let Some(xdg) = env_path("XDG_CONFIG_HOME") {
        xdg
    } else if cfg!(windows) {
        env_path("APPDATA")?
    } else {
        env_path("HOME")?.join(".config")
    };
    Some(dir.join("findopera").join("credentials"))
}

fn env_path(key: &str) -> Option<PathBuf> {
    match std::env::var_os(key) {
        Some(v) if !v.is_empty() => Some(PathBuf::from(v)),
        _ => None,
    }
}

/// Read the stored token, if there is one.
///
/// `Ok(None)` means nothing is stored, which is the ordinary state and not a
/// problem. `Err` means something is stored that should not be trusted.
pub fn load() -> Result<Option<String>, String> {
    let Some(path) = path() else {
        return Ok(None);
    };
    let contents = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("cannot read {}: {e}", path.display())),
    };
    // A secret others can read is not a secret. Better to stop and say so than
    // to go on using it as though the file said what it was meant to.
    if let Some(why) = too_open(&path) {
        return Err(why);
    }
    let token = contents.trim();
    if token.is_empty() {
        return Ok(None);
    }
    Ok(Some(token.to_string()))
}

/// Whether anyone but the owner can read the file.
#[cfg(unix)]
fn too_open(path: &Path) -> Option<String> {
    use std::os::unix::fs::PermissionsExt;
    let mode = std::fs::metadata(path).ok()?.permissions().mode();
    if mode & 0o077 == 0 {
        return None;
    }
    Some(format!(
        "{} can be read by others (mode {:o}). Run `chmod 600 {}`, and treat \
         the token in it as compromised.",
        path.display(),
        mode & 0o777,
        path.display()
    ))
}

#[cfg(not(unix))]
fn too_open(_path: &Path) -> Option<String> {
    // Windows inherits the directory's ACL, and there is no mode to check.
    None
}

/// Store a token, replacing whatever was there.
pub fn store(token: &str) -> Result<PathBuf, String> {
    let path = path()
        .ok_or("cannot tell where to keep the token: neither XDG_CONFIG_HOME nor HOME is set")?;
    let dir = path.parent().expect("the path always has a directory");
    std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    private(dir);

    // Created with its permissions already right, rather than written and then
    // tightened: between those two there is a moment where the token is
    // readable, and that is the whole thing worth preventing.
    let mut file =
        create_private(&path).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    writeln!(file, "{token}").map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok(path)
}

/// Forget the stored token. Says whether there was one.
pub fn forget() -> Result<bool, String> {
    let Some(path) = path() else {
        return Ok(false);
    };
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(format!("cannot remove {}: {e}", path.display())),
    }
}

#[cfg(unix)]
fn create_private(path: &Path) -> std::io::Result<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
}

#[cfg(not(unix))]
fn create_private(path: &Path) -> std::io::Result<std::fs::File> {
    std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
}

/// Keep the directory to its owner as well, where that means anything.
#[cfg(unix)]
fn private(dir: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
}

#[cfg(not(unix))]
fn private(_dir: &Path) {}
