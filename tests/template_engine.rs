//! The engine against a resolver of its own.
//!
//! Everything the template language does is exercised through `organize` in
//! `tests/snapshots/template/`, where a case is judged on the folder name a
//! person would get. One thing cannot be: a resolver that hands back an empty
//! string.
//!
//! `Template` and `Fields` are public, and a caller supplying their own
//! resolver can return `""` for a field. Through findopera.com it never
//! arrives — an empty string is one of the ways the API spells "unknown", and
//! the generated model turns it into absence before the engine sees it. So
//! this is the seam where the two disagree, and the only place left that
//! needs a resolver written by hand.

use findopera::{to_path, FieldDoc, Fields, Template};
use std::collections::BTreeMap;

static FIELDS: &[FieldDoc] = &[
    FieldDoc::non_null("opera.title", "Title in the original language"),
    FieldDoc::new("year", "Year recorded"),
];

struct Record(BTreeMap<String, String>);

impl Fields for Record {
    fn required(&self, path: &str) -> String {
        self.0.get(path).cloned().unwrap_or_default()
    }
    fn optional(&self, path: &str) -> Option<String> {
        self.0.get(path).cloned()
    }
}

#[test]
fn an_empty_string_is_a_value_so_no_fallback_fires() {
    // Absence is what makes a fallback fire, and an empty string is not
    // absence — it is a value that happens to be empty. Rendering therefore
    // succeeds and produces nothing, which is caught one step later as a name
    // that is not a usable path.
    let record = Record(BTreeMap::from([("year".to_string(), String::new())]));
    let template = Template::parse(r#"{{year|"unknown"}}"#, FIELDS).expect("template parses");

    let rendered = template.render(&record);
    assert_eq!(rendered, "", "the empty value wins, not the literal");

    let refused = to_path(&rendered).expect_err("an empty name is not a path");
    assert_eq!(refused.code(), "path_empty", "{refused}");
}

#[test]
fn an_absent_value_is_what_makes_the_fallback_fire() {
    // The other half, so the pair says what the difference is rather than
    // leaving it to be inferred from one of them.
    let record = Record(BTreeMap::new());
    let template = Template::parse(r#"{{year|"unknown"}}"#, FIELDS).expect("template parses");
    assert_eq!(template.render(&record), "unknown");
}
