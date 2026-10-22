//! The FindOpera recording model.
//!
//! Almost all of this module is generated, derived from `schema/schema.graphql`
//! and `schema/recording.graphql` by `codegen/generate.mjs`. What lives here by
//! hand is the part no schema can state: how the API spells "unknown".
//!
//! # Two sentinels, plus null
//!
//! The API returns `null`, `0`, and `""` for an absent value — sometimes for
//! the same field on different records. Recording 10655 has `month: 0` while
//! recording 75 has `month: null`, and both mean nobody knows the month.
//!
//! Collapsing them here, at the deserialization boundary, is deliberate. It
//! means the Rust type says what is true: `Option<String>` is `None` exactly
//! when there is no value, so [`crate::Fields`] has nothing left to decide
//! and a template reasons only about present-versus-absent.
//!
//! `@semanticNonNull` fields are left alone. The schema says they always have
//! a value, so they deserialize into a plain `String` — and a `null` there is
//! a deserialization error, which is the correct reading of the directive: it
//! means the response carried a matching entry in its `errors` array.

pub mod crud;
mod generated;

pub use generated::{Recording, FIELDS, QUERY};

/// A string value, or `None` if it is only whitespace.
///
/// Used both by the sentinel-collapsing deserializer and by the generated
/// projections out of a list, so an empty name never becomes an empty path
/// segment.
pub(crate) fn text(s: &str) -> Option<String> {
    let t = s.trim();
    (!t.is_empty()).then(|| t.to_string())
}

/// A person's years, the way the library writes them.
///
/// `1685-1759` when both are known, `b1947` when only the birth year is. With
/// no birth year there is nothing usable to say: a lone death year would need
/// a spelling that means "died", and the candidates are either awkward in a
/// filename or read as a negative number.
pub(crate) fn lifespan(born: Option<i64>, died: Option<i64>) -> Option<String> {
    let born = born?;
    Some(match died {
        Some(died) => format!("{born}-{died}"),
        None => format!("b{born}"),
    })
}

pub(crate) mod de {
    use serde::{Deserialize, Deserializer};

    /// `null` and `""` both mean absent.
    pub fn text<'de, D>(d: D) -> Result<Option<String>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Option::<String>::deserialize(d)?.and_then(|s| super::text(&s)))
    }

    /// `null` and `0` both mean absent — the API uses zero for an unknown
    /// year, month, day, and birth or death year.
    pub fn num<'de, D>(d: D) -> Result<Option<i64>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Option::<i64>::deserialize(d)?.filter(|n| *n != 0))
    }

    /// A record id, as a JSON number or a string.
    ///
    /// The API has sent most ids as numbers and is moving them all to
    /// strings. Taking both means neither side of that change breaks this
    /// build.
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Id {
        Num(i64),
        Str(String),
    }

    impl From<Id> for String {
        fn from(id: Id) -> String {
            match id {
                Id::Num(n) => n.to_string(),
                Id::Str(s) => s,
            }
        }
    }

    pub fn id<'de, D>(d: D) -> Result<String, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Id::deserialize(d)?.into())
    }

    /// The same, where the id may be absent.
    ///
    /// Codegen reaches for this when the recording query selects an id under
    /// a nullable parent (`walk.mjs`). The query has none at the moment, so
    /// nothing calls it, but deleting it would break the build the day one is
    /// added.
    #[allow(dead_code)]
    pub fn id_opt<'de, D>(d: D) -> Result<Option<String>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Option::<Id>::deserialize(d)?.map(String::from))
    }

    #[cfg(test)]
    mod tests {
        use serde::Deserialize;

        #[derive(Deserialize)]
        struct Row {
            #[serde(deserialize_with = "super::id")]
            id: String,
        }

        #[test]
        fn an_id_is_read_as_a_number_or_a_string() {
            let n: Row = serde_json::from_str(r#"{"id": 42}"#).unwrap();
            let s: Row = serde_json::from_str(r#"{"id": "42"}"#).unwrap();
            assert_eq!(n.id, "42");
            assert_eq!(s.id, "42");
        }
    }
}
