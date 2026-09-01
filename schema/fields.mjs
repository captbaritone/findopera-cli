// What the query cannot say on its own.
//
// Everything else about the template surface — which paths exist, and which
// are always present — is read from schema.graphql and recording.graphql.
// This file only covers the gaps: paths that need a different name than their
// position in the query, values projected out of a list, and descriptions the
// schema does not carry.

export default {
  // Hoist a whole subtree to a shorter template path. The composer hangs off
  // the opera in the graph, but reads better at the top level in a template.
  rename: {
    "opera.composer": "composer",
  },

  // A second name for one path. `{{opera.language}}` is the obvious spelling
  // of what is really `opera.language.name`.
  alias: {
    "opera.language": "opera.language.name",
  },

  // How a value is rendered, where the raw number is not what you want.
  format: {
    month: "pad2",
    day: "pad2",
  },

  // Values built from more than one field. Named rules rather than an
  // expression language: the set is small and each one is a decision about how
  // the library reads, not a calculation anybody needs to write inline.
  //
  // `lifespan` gives `1685-1759`, or `b1947` for someone still living. With no
  // birth year it gives nothing at all — a lone death year would have to be
  // spelled some way that says "died", and every candidate for that is either
  // a character filesystems dislike or a bare `-1602` that reads as a negative
  // number. The library agrees: its one such composer has no dates on him.
  computed: [
    {
      rule: "lifespan",
      on: ["composer", "conductor"],
      as: "dates",
      doc: "Years, as 1685-1759 — or b1947 for someone still living",
    },
  ],

  // Values projected out of a list. A query can select `notedSingers`, but it
  // has no way to say "the second one" or "all of them, comma-joined", and the
  // template language has no list syntax — by design, since optional groups
  // cover the same ground: `[{{singer1}}][, {{singer2}}]` joins with whatever
  // separator you write, including a different last one.
  //
  // Every derived field is nullable: a list the schema guarantees is present
  // can still be empty.
  derived: [
    { path: "upc", from: "upcs", index: 0, select: "upc",
      doc: "First barcode listed for the release. Often absent" },

    { path: "singers", from: "notedSingers", join: ", ", select: "fullName",
      doc: "All noted singers, full names, comma-joined" },
    { path: "singers.lastNames", from: "notedSingers", join: ", ", select: "lastName",
      doc: "All noted singers, surnames only, comma-joined" },

    { path: "singer1", from: "notedSingers", index: 0, select: "fullName",
      doc: "First noted singer, full name" },
    { path: "singer2", from: "notedSingers", index: 1, select: "fullName",
      doc: "Second noted singer, if any" },
    { path: "singer3", from: "notedSingers", index: 2, select: "fullName",
      doc: "Third noted singer, if any" },
    { path: "singer1.lastName", from: "notedSingers", index: 0, select: "lastName",
      doc: "First noted singer, surname only" },
    { path: "singer2.lastName", from: "notedSingers", index: 1, select: "lastName",
      doc: "Second noted singer, surname only" },
    { path: "singer3.lastName", from: "notedSingers", index: 2, select: "lastName",
      doc: "Third noted singer, surname only" },
  ],

  // The schema documents almost none of the fields a template would use.
  doc: {
    id: "FindOpera recording id — good for disambiguating",
    url: "Canonical findopera.com URL for the recording",
    year: "Year recorded. Often absent on older entries",
    month: "Month recorded, zero-padded (04). Usually absent",
    day: "Day recorded, zero-padded (09). Usually absent",
    orchestra: "Orchestra name",
    chorus: "Chorus name. Often absent",
    "opera.title": "Title in the original language — the safest title field",
    "opera.englishTitle": "English title. Absent unless the opera has one",
    "opera.librettist": "Librettist. Absent for most recordings",
    "opera.url": "Canonical findopera.com URL for the opera",
    "opera.language": "Language sung, e.g. German",
    "opera.language.name": "Language sung, e.g. German",
    "opera.language.abbreviation": "Language code, e.g. de",
    "composer.fullName": "Composer, e.g. George Frideric Handel",
    "composer.firstName": "Composer given name(s)",
    "composer.lastName": "Composer surname — the usual top-level folder",
    "composer.born": "Composer year of birth",
    "composer.died": "Composer year of death",
    "conductor.fullName": "Conductor, e.g. Charles Mackerras",
    "conductor.firstName": "Conductor given name(s)",
    "conductor.lastName": "Conductor surname",
    "conductor.born": "Conductor year of birth",
    "conductor.died": "Conductor year of death",
  },
};
