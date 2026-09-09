//! `findopera` — name a music library from FindOpera metadata.
//!
//! The whole command line lives in `findopera::cli`, so that it can be run
//! without a process to run it in. This is only the wrapper that gives it one.

fn main() {
    std::process::exit(findopera::cli::run());
}
