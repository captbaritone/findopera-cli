//! Types mirroring the FindOpera GraphQL `Recording` graph.
//!
//! The API uses two different sentinels for "unknown": SQL `NULL` and a zero /
//! empty value (`month: 0`, `librettist: ""`). [`Field::get`] collapses both so
//! templates only ever have to reason about present-vs-absent.

use serde::Deserialize;

/// Every field the CLI exposes to templates. Kept in sync with `QUERY`.
pub const QUERY: &str = r#"query($ids: [String!]!) {
  getRecordingByIds(ids: $ids) {
    id url year month day orchestra chorus
    conductor { fullName firstName lastName born died }
    notedSingers { fullName firstName lastName }
    opera {
      title englishTitle librettist url
      language { name abbreviation }
      composer { fullName firstName lastName born died }
    }
    upcs { upc }
  }
}"#;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recording {
    pub id: Option<i64>,
    pub url: Option<String>,
    pub year: Option<i64>,
    pub month: Option<i64>,
    pub day: Option<i64>,
    pub orchestra: Option<String>,
    pub chorus: Option<String>,
    pub conductor: Option<Person>,
    pub noted_singers: Option<Vec<Person>>,
    pub opera: Option<Opera>,
    pub upcs: Option<Vec<Upc>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Opera {
    pub title: Option<String>,
    pub english_title: Option<String>,
    pub librettist: Option<String>,
    pub url: Option<String>,
    pub language: Option<Language>,
    pub composer: Option<Person>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub full_name: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub born: Option<i64>,
    pub died: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct Language {
    pub name: Option<String>,
    pub abbreviation: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Upc {
    pub upc: Option<String>,
}

/// A resolved template field: `None` for absent, `Some` for a usable value.
pub type Field = Option<String>;

/// One template field and what it means, for `library fields` and `schema`.
pub struct FieldDoc {
    pub path: &'static str,
    pub description: &'static str,
}

impl FieldDoc {
    const fn new(path: &'static str, description: &'static str) -> Self {
        FieldDoc { path, description }
    }
}

fn text(v: &Option<String>) -> Field {
    match v {
        Some(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        _ => None,
    }
}

/// `0` is the API's "unknown" for year/month/day and for birth/death years.
fn num(v: &Option<i64>) -> Field {
    match v {
        Some(n) if *n != 0 => Some(n.to_string()),
        _ => None,
    }
}

fn pad2(v: &Option<i64>) -> Field {
    match v {
        Some(n) if *n != 0 => Some(format!("{n:02}")),
        _ => None,
    }
}

impl Recording {
    /// Resolve a dotted template path against this recording.
    ///
    /// Returns `Err` for a path that is not part of the documented surface, so
    /// a hallucinated or mistyped placeholder fails loudly at plan time rather
    /// than silently producing an empty path segment.
    pub fn get(&self, path: &str) -> Result<Field, String> {
        let opera = self.opera.as_ref();
        let composer = opera.and_then(|o| o.composer.as_ref());
        let conductor = self.conductor.as_ref();
        let language = opera.and_then(|o| o.language.as_ref());

        let person = |p: Option<&Person>, part: &str| -> Field {
            let p = p?;
            match part {
                "fullName" => text(&p.full_name),
                "firstName" => text(&p.first_name),
                "lastName" => text(&p.last_name),
                "born" => num(&p.born),
                "died" => num(&p.died),
                _ => None,
            }
        };

        let value = match path {
            "id" => num(&self.id),
            "url" => text(&self.url),
            "year" => num(&self.year),
            "month" => pad2(&self.month),
            "day" => pad2(&self.day),
            "orchestra" => text(&self.orchestra),
            "chorus" => text(&self.chorus),

            "opera.title" => opera.and_then(|o| text(&o.title)),
            "opera.englishTitle" => opera.and_then(|o| text(&o.english_title)),
            "opera.librettist" => opera.and_then(|o| text(&o.librettist)),
            "opera.url" => opera.and_then(|o| text(&o.url)),
            "opera.language" | "opera.language.name" => language.and_then(|l| text(&l.name)),
            "opera.language.abbreviation" => language.and_then(|l| text(&l.abbreviation)),

            "upc" => self
                .upcs
                .as_ref()
                .and_then(|u| u.first())
                .and_then(|u| text(&u.upc)),

            "singers" => self.noted_singers.as_ref().and_then(|s| {
                let names: Vec<String> = s.iter().filter_map(|p| text(&p.full_name)).collect();
                (!names.is_empty()).then(|| names.join(", "))
            }),
            "singers.lastNames" => self.noted_singers.as_ref().and_then(|s| {
                let names: Vec<String> = s.iter().filter_map(|p| text(&p.last_name)).collect();
                (!names.is_empty()).then(|| names.join(", "))
            }),

            // Indexed access, so a template can place its own separators:
            // `[{{singer1}}][, {{singer2}}]` joins without a join filter.
            "singer1" | "singer2" | "singer3" => {
                let i = path.as_bytes()[6] - b'1';
                self.nth_singer(i as usize).and_then(|p| text(&p.full_name))
            }
            "singer1.lastName" | "singer2.lastName" | "singer3.lastName" => {
                let i = path.as_bytes()[6] - b'1';
                self.nth_singer(i as usize).and_then(|p| text(&p.last_name))
            }

            _ => {
                let (prefix, part) = path.split_once('.').ok_or_else(|| path.to_string())?;
                let target = match prefix {
                    "composer" => composer,
                    "conductor" => conductor,
                    _ => return Err(path.to_string()),
                };
                // Validate the sub-field separately from the lookup: `person`
                // returns `None` both for an absent person and an unknown part,
                // and only the latter is a template error.
                if !matches!(
                    part,
                    "fullName" | "firstName" | "lastName" | "born" | "died"
                ) {
                    return Err(path.to_string());
                }
                person(target, part)
            }
        };
        Ok(value)
    }

    /// Every valid template path, with the one-line description shown by
    /// `findopera fields`. This is the single source of truth for what
    /// a template may reference.
    pub const FIELDS: &'static [FieldDoc] = &[
        FieldDoc::new(
            "id",
            "FindOpera recording id — always present, good for disambiguating",
        ),
        FieldDoc::new("url", "Canonical findopera.com URL for the recording"),
        FieldDoc::new("year", "Year recorded. Often absent on older entries"),
        FieldDoc::new("month", "Month recorded, zero-padded (04). Usually absent"),
        FieldDoc::new("day", "Day recorded, zero-padded (09). Usually absent"),
        FieldDoc::new("orchestra", "Orchestra name"),
        FieldDoc::new("chorus", "Chorus name. Often absent"),
        FieldDoc::new("upc", "First barcode listed for the release. Often absent"),
        FieldDoc::new("singers", "All noted singers, full names, comma-joined"),
        FieldDoc::new(
            "singers.lastNames",
            "All noted singers, surnames only, comma-joined",
        ),
        // Indexed access. With 0-3 noted singers, these plus optional groups
        // replace a join filter: `[{{singer1}}][, {{singer2}}]` joins with
        // whatever separator you write, including a different last one.
        FieldDoc::new("singer1", "First noted singer, full name"),
        FieldDoc::new("singer2", "Second noted singer, if any"),
        FieldDoc::new("singer3", "Third noted singer, if any"),
        FieldDoc::new("singer1.lastName", "First noted singer, surname only"),
        FieldDoc::new("singer2.lastName", "Second noted singer, surname only"),
        FieldDoc::new("singer3.lastName", "Third noted singer, surname only"),
        FieldDoc::new(
            "opera.title",
            "Title in the original language — the safest title field",
        ),
        FieldDoc::new(
            "opera.englishTitle",
            "English title. Absent unless the opera has one",
        ),
        FieldDoc::new("opera.librettist", "Librettist. Absent for most recordings"),
        FieldDoc::new("opera.url", "Canonical findopera.com URL for the opera"),
        FieldDoc::new("opera.language", "Language sung, e.g. German"),
        FieldDoc::new("opera.language.abbreviation", "Language code, e.g. de"),
        FieldDoc::new("composer.fullName", "Composer, e.g. George Frideric Handel"),
        FieldDoc::new("composer.firstName", "Composer given name(s)"),
        FieldDoc::new(
            "composer.lastName",
            "Composer surname — the usual top-level folder",
        ),
        FieldDoc::new("composer.born", "Composer year of birth"),
        FieldDoc::new("composer.died", "Composer year of death"),
        FieldDoc::new("conductor.fullName", "Conductor, e.g. Charles Mackerras"),
        FieldDoc::new("conductor.firstName", "Conductor given name(s)"),
        FieldDoc::new("conductor.lastName", "Conductor surname"),
        FieldDoc::new("conductor.born", "Conductor year of birth"),
        FieldDoc::new("conductor.died", "Conductor year of death"),
    ];

    fn nth_singer(&self, i: usize) -> Option<&Person> {
        self.noted_singers.as_ref()?.get(i)
    }

    /// Is this a template path the renderer understands?
    pub fn is_known(path: &str) -> bool {
        Self::FIELDS.iter().any(|f| f.path == path)
    }
}
