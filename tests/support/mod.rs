//! What the test suites share.
//!
//! Two ways to stand in for a server, because the suites want different
//! things from one. `server` is a real socket, for the handful of tests whose
//! subject *is* the HTTP — the headers that go out, a `Retry-After` coming
//! back. `scripted` replaces only the socket, for everything above it, which
//! is almost everything.
//!
//! Both existed five times over before this, once per suite, each slightly
//! different.

#![allow(dead_code)]

pub mod markdown;
pub mod scripted;
pub mod server;
pub mod snapshot;

/// A path as a test writes it, whichever separator this platform uses.
pub fn slashes(s: &str) -> String {
    s.replace(std::path::MAIN_SEPARATOR, "/")
}
