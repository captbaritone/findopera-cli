// Emits the table of types the CRUD commands work through.
//
// Twenty types, each with the same four operations and a pair of input
// objects. Written by hand that is eighty things to keep in step with a schema
// that moves; derived from the schema it is one thing that cannot drift, and
// CI fails when it does.

import {
  getNamedType,
  isNonNullType,
  isListType,
  isInputObjectType,
} from "graphql";

/** `RecordingURL` -> `recording-url`, `SpotifyAlbum` -> `spotify-album`. */
export function kebab(name) {
  return name
    .replace(/([a-z0-9])([A-Z])/g, "$1-$2")
    .replace(/([A-Z]+)([A-Z][a-z])/g, "$1-$2")
    .toLowerCase();
}

/** What a field looks like once it is JSON rather than GraphQL. */
function jsonType(type) {
  // Nullability comes off first. Checking for a list before unwrapping it
  // misses `[T!]!`, which is a non-null list rather than a list, and reports
  // whatever T happens to be — so a cast of twelve roles was described as a
  // string.
  if (isNonNullType(type)) return jsonType(type.ofType);
  if (isListType(type)) return "array";
  switch (getNamedType(type).name) {
    case "Int":
      return "integer";
    case "Float":
      return "number";
    case "Boolean":
      return "boolean";
    default:
      return "string";
  }
}

function rustStr(s) {
  return JSON.stringify(s ?? "");
}

/**
 * The fields of one element, for a field that holds a list of them.
 *
 * Without this a list is described as `{"type": "array"}` and nothing more, so
 * a schema that promises to validate an input cannot check the one part of it
 * with any structure. A misspelled key inside a portrayal passed, and was
 * refused by the server instead.
 */
function elementFields(type) {
  const inner = isNonNullType(type) ? type.ofType : type;
  if (!isListType(inner)) return [];
  const element = getNamedType(inner);
  return isInputObjectType(element) ? fields(element) : [];
}

function fields(inputType) {
  if (!inputType) return [];
  return Object.values(inputType.getFields())
    .map((f) => ({
      name: f.name,
      json: jsonType(f.type),
      required: isNonNullType(f.type),
      about: f.description ?? null,
    }))
    .sort((a, b) => {
      // Required first, so the shape of the thing is visible before its
      // options; then alphabetical, so the order never depends on the schema's.
      if (a.required !== b.required) return a.required ? -1 : 1;
      return a.name.localeCompare(b.name);
    });
}

/**
 * A create that does several things at once.
 *
 * Found rather than declared: a mutation called `add<Type>With<Something>` is
 * taken to be the composite create for that type. The server has one today —
 * addRecordingWithPortrayals — and if it grows another this picks it up
 * without anyone remembering to say so.
 *
 * Its arguments beyond `input` and `justification` become extra keys on the
 * input, named exactly as the mutation names them, so the JSON a caller writes
 * maps one-to-one onto what is sent and no table has to be kept in step.
 */
function composite(schema, mutations, graphql) {
  const name = Object.keys(mutations).find((m) =>
    new RegExp(`^add${graphql}With[A-Z]`).test(m),
  );
  if (!name) return null;

  const extras = mutations[name].args
    .filter((a) => a.name !== "input" && a.name !== "justification")
    .map((a) => ({
      name: a.name,
      json: jsonType(a.type),
      // As GraphQL writes it — `[RecordingPortrayalsInput!]!` — so the variable
      // declaration can be built from the schema rather than guessed at. A
      // second composite would otherwise need this spelled out by hand in Rust.
      gql: a.type.toString(),
      required: isNonNullType(a.type),
      about: a.description ?? null,
      items: elementFields(a.type),
    }))
    .sort((a, b) => {
      if (a.required !== b.required) return a.required ? -1 : 1;
      return a.name.localeCompare(b.name);
    });

  return { mutation: name, extras };
}

export function crud(schema, getOperations) {
  const mutations = schema.getMutationType().getFields();
  const queries = schema.getQueryType().getFields();

  const types = [];
  for (const [name, field] of Object.entries(mutations)) {
    const m = /^add([A-Z][A-Za-z]*)$/.exec(name);
    if (!m) continue;
    const graphql = m[1];
    // Only types with the whole set; anything partial is a special case and
    // does not belong in a uniform command.
    if (!mutations[`update${graphql}`] || !mutations[`delete${graphql}`]) continue;
    const root = `get${graphql}ById`;
    if (!queries[root]) continue;

    const operation = getOperations.get(graphql);
    if (!operation) {
      throw new Error(
        `schema/get.graphql has no query for ${graphql}; every type with CRUD needs one`,
      );
    }

    const inputArg = field.args.find((a) => a.name === "input");
    const updateArg = mutations[`update${graphql}`].args.find((a) => a.name === "input");

    for (const [where, list] of [
      ["create", inputArg && getNamedType(inputArg.type)],
      ["edit", updateArg && getNamedType(updateArg.type)],
    ]) {
      if (!list) continue;
      for (const f of Object.values(list.getFields())) {
        const inner = isNonNullType(f.type) ? f.type.ofType : f.type;
        if (isListType(inner)) {
          throw new Error(
            `${graphql}.${where} field \`${f.name}\` is a list. Lists need an ` +
              `items subschema or describe --json cannot validate them; that ` +
              `is handled for composite arguments but not here yet.`,
          );
        }
      }
    }

    types.push({
      name: kebab(graphql),
      graphql,
      get: operation,
      root,
      add: name,
      update: `update${graphql}`,
      remove: `delete${graphql}`,
      // The input object names, taken from the schema rather than assembled
      // from the type name, so a server that ever spells one differently is
      // followed instead of guessed at.
      createInput: getNamedType(inputArg.type).name,
      editInput: getNamedType(updateArg.type).name,
      create: fields(inputArg && getNamedType(inputArg.type)),
      edit: fields(updateArg && getNamedType(updateArg.type)),
      composite: composite(schema, mutations, graphql),
    });
  }
  types.sort((a, b) => a.name.localeCompare(b.name));

  const L = [];
  L.push("//! The types the CRUD commands work through.");
  L.push("//!");
  L.push("//! Generated by `codegen/crud.mjs` from the schema and");
  L.push("//! `schema/get.graphql`. Do not edit; run `npm run generate`.");
  L.push("");
  L.push("/// One field of a create or edit input.");
  L.push("pub struct InputField {");
  L.push("    pub name: &'static str,");
  L.push("    /// What this is once it is JSON, for the schema `describe` prints.");
  L.push("    pub json: &'static str,");
  L.push("    pub required: bool,");
  L.push("    pub about: &'static str,");
  L.push("}");
  L.push("");
  L.push("/// A type, and the four things that can be done to it.");
  L.push("pub struct Type {");
  L.push("    /// What it is called on the command line.");
  L.push("    pub name: &'static str,");
  L.push("    pub graphql: &'static str,");
  L.push("    /// The operation in `schema/get.graphql`, and the field it returns.");
  L.push("    pub get: &'static str,");
  L.push("    pub root: &'static str,");
  L.push("    pub add: &'static str,");
  L.push("    pub update: &'static str,");
  L.push("    pub remove: &'static str,");
  L.push("    /// The GraphQL input object a create takes.");
  L.push("    pub create_input: &'static str,");
  L.push("    /// The same for an edit, where every field is optional.");
  L.push("    pub edit_input: &'static str,");
  L.push("    pub create: &'static [InputField],");
  L.push("    pub edit: &'static [InputField],");
  L.push("    /// A create that does more than one thing, in one transaction.");
  L.push("    pub composite: Option<Composite>,");
  L.push("}");
  L.push("");
  L.push("/// A mutation that builds a record and everything hanging off it at once.");
  L.push("pub struct Composite {");
  L.push("    /// Called instead of `add`.");
  L.push("    pub mutation: &'static str,");
  L.push("    /// Arguments beside `input`, which the caller supplies as extra keys.");
  L.push("    pub extras: &'static [Extra],");
  L.push("}");
  L.push("");
  L.push("/// One argument of a composite create.");
  L.push("pub struct Extra {");
  L.push("    pub name: &'static str,");
  L.push("    /// The GraphQL type, as the schema writes it, for the variable.");
  L.push("    pub gql: &'static str,");
  L.push("    pub json: &'static str,");
  L.push("    pub required: bool,");
  L.push("    pub about: &'static str,");
  L.push("    /// For a list of objects, the fields one element has.");
  L.push("    pub items: &'static [InputField],");
  L.push("}");
  L.push("");
  L.push(`/// Every type with a complete set of operations. ${types.length} of them.`);
  L.push("pub const TYPES: &[Type] = &[");
  for (const t of types) {
    L.push("    Type {");
    L.push(`        name: ${rustStr(t.name)},`);
    L.push(`        graphql: ${rustStr(t.graphql)},`);
    L.push(`        get: ${rustStr(t.get)},`);
    L.push(`        root: ${rustStr(t.root)},`);
    L.push(`        add: ${rustStr(t.add)},`);
    L.push(`        update: ${rustStr(t.update)},`);
    L.push(`        remove: ${rustStr(t.remove)},`);
    L.push(`        create_input: ${rustStr(t.createInput)},`);
    L.push(`        edit_input: ${rustStr(t.editInput)},`);
    for (const [key, list] of [["create", t.create], ["edit", t.edit]]) {
      if (list.length === 0) {
        L.push(`        ${key}: &[],`);
        continue;
      }
      L.push(`        ${key}: &[`);
      for (const f of list) {
        L.push(
          `            InputField { name: ${rustStr(f.name)}, json: ${rustStr(f.json)}, ` +
            `required: ${f.required}, about: ${rustStr(f.about)} },`,
        );
      }
      L.push("        ],");
    }
    if (t.composite) {
      L.push("        composite: Some(Composite {");
      L.push(`            mutation: ${rustStr(t.composite.mutation)},`);
      L.push("            extras: &[");
      for (const f of t.composite.extras) {
        const items = f.items.length
          ? "&[" +
            f.items
              .map(
                (i) =>
                  `InputField { name: ${rustStr(i.name)}, json: ${rustStr(i.json)}, ` +
                  `required: ${i.required}, about: ${rustStr(i.about)} }`,
              )
              .join(", ") +
            "]"
          : "&[]";
        L.push(
          `                Extra { name: ${rustStr(f.name)}, gql: ${rustStr(f.gql)}, ` +
            `json: ${rustStr(f.json)}, required: ${f.required}, about: ${rustStr(f.about)}, ` +
            `items: ${items} },`,
        );
      }
      L.push("            ],");
      L.push("        }),");
    } else {
      L.push("        composite: None,");
    }
    L.push("    },");
  }
  L.push("];");
  L.push("");
  return L.join("\n");
}
