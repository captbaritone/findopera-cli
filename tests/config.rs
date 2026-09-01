//! What a settings file says when it is wrong.
//!
//! One file is one case: some TOML, and the message it produces. These exist
//! because a settings file is the first thing a person meets, and the error it
//! gives when they get it slightly wrong is most of what decides whether the
//! format was a good choice.
//!
//! ```text
//! --- toml
//! template = '''{{opera.title}}'''
//! requre-variants = true
//! --- result
//! …
//! ```
//!
//! `UPDATE_EXPECT=1 cargo test` rewrites the `--- result` blocks.

use findopera::config::Config;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

mod fixture;

#[test]
fn cases() {
    fixture::run(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/config-cases"),
        &["toml"],
        &[],
        &["result"],
        |name, input| {
            let slug: String = name.chars().filter(|c| c.is_alphanumeric()).collect();
            let dir =
                std::env::temp_dir().join(format!("findopera-cfg-{slug}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).expect("temp dir");
            let path = dir.join("findopera.toml");
            fs::write(&path, &input["toml"]).expect("write");

            let result = match Config::load(&path) {
                Ok(c) => format!(
                    "template = {:?}\ndestination = {:?}\nlink = {:?}\n\
                     require-variants = {}\nfollow-links = {}",
                    c.template.trim_end_matches('\n'),
                    c.destination,
                    c.link,
                    c.require_variants,
                    c.follow_links
                ),
                // The temporary directory is different every run; the message
                // is the thing under test, not where the file happened to be.
                Err(e) => e
                    .to_string()
                    .replace(&path.display().to_string(), "findopera.toml"),
            };
            let _ = fs::remove_dir_all(&dir);
            BTreeMap::from([("result".to_string(), result)])
        },
    );
}
