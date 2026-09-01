#!/usr/bin/env node
// Derives src/model's Rust from the GraphQL schema and the query.
//
//   node generate.mjs
//
// The query is validated against the schema first, so a field that has been
// renamed or removed upstream fails here rather than at runtime.

import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { buildSchema, parse, validate, getNamedType } from "graphql";

import { walk } from "./walk.mjs";
import { emit } from "./emit.mjs";
import { snake } from "./schema.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const at = (p) => resolve(here, "..", p);

const schema = buildSchema(readFileSync(at("schema/schema.graphql"), "utf8"));
const querySrc = readFileSync(at("schema/recording.graphql"), "utf8");
const document = parse(querySrc);

// Real validation, from the reference implementation: unknown fields, bad
// arguments, wrong variable types.
const errors = validate(schema, document);
if (errors.length) {
  console.error("query does not validate against the schema:");
  for (const e of errors) console.error("  " + e.message);
  process.exit(1);
}

// The lookup queries generate no Rust, but they are still strings this binary
// sends, so they are validated here too: a field renamed upstream should fail
// the build rather than someone's search.
const lookups = parse(readFileSync(at("schema/search.graphql"), "utf8"));
const lookupErrors = validate(schema, lookups);
if (lookupErrors.length) {
  console.error("search.graphql does not validate against the schema:");
  for (const e of lookupErrors) console.error("  " + e.message);
  process.exit(1);
}

const overlay = (await import(at("schema/fields.mjs"))).default;

// The operation selects a single root field; the model is that field's type.
const op = document.definitions.find((d) => d.kind === "OperationDefinition");
const rootField = op.selectionSet.selections[0];
const rootDef = schema.getQueryType().getFields()[rootField.name.value];
const rootType = getNamedType(rootDef.type);

let { fields, structs, lists } = walk(schema, rootType, rootField.selectionSet);

// ---- apply the overlay ------------------------------------------------

// Hoist subtrees to shorter template paths.
for (const [from, to] of Object.entries(overlay.rename ?? {})) {
  let hit = false;
  for (const f of fields) {
    if (f.path === from || f.path.startsWith(from + ".")) {
      f.path = to + f.path.slice(from.length);
      hit = true;
    }
  }
  if (!hit) throw new Error(`rename ${from}: no such path in the query`);
}

// A second name for one path.
for (const [alias, target] of Object.entries(overlay.alias ?? {})) {
  const f = fields.find((f) => f.path === target);
  if (!f) throw new Error(`alias ${alias}: ${target} is not a field`);
  fields.push({ ...f, path: alias });
}

for (const [path, format] of Object.entries(overlay.format ?? {})) {
  const f = fields.find((f) => f.path === path);
  if (!f) throw new Error(`format ${path}: no such field`);
  f.format = format;
}

// A computed field is built from others the query already selects, so it can
// only name fields that are there.
const computed = [];
for (const c of overlay.computed ?? []) {
  if (c.rule !== "lifespan") {
    throw new Error(`computed ${c.as}: no rule named ${c.rule}`);
  }
  for (const ns of c.on) {
    const parts = ["born", "died"].map((leaf) => {
      const f = fields.find((f) => f.path === `${ns}.${leaf}`);
      if (!f) {
        throw new Error(
          `computed ${ns}.${c.as}: the query does not select ${ns}.${leaf}`,
        );
      }
      return f;
    });
    computed.push({ path: `${ns}.${c.as}`, rule: c.rule, parts, doc: c.doc });
  }
}

const derived = (overlay.derived ?? []).map((d) => {
  const list = lists.get(d.from);
  if (!list) {
    throw new Error(
      `derived ${d.path}: the query does not select a list at ${d.from}`,
    );
  }
  const elem = structs.get(list.elem);
  const selectSnake = snake(d.select);
  if (!elem?.fields.some((f) => f.rust_name === selectSnake)) {
    throw new Error(
      `derived ${d.path}: the query does not select ${d.from}.${d.select}`,
    );
  }
  return { ...d, selectSnake };
});

for (const f of fields) {
  f.doc = overlay.doc?.[f.path] ?? f.doc;
  f.leaf = f.access[f.access.length - 1].leaf;
}

// Every path must be unique, or two match arms would collide.
const seen = new Set();
for (const f of [...fields, ...computed, ...derived]) {
  if (seen.has(f.path)) throw new Error(`duplicate template path ${f.path}`);
  seen.add(f.path);
}

const out = emit({ fields, structs, computed, derived, lists, query: querySrc });
writeFileSync(at("src/model/generated.rs"), out);
console.error(
  `wrote src/model/generated.rs: ${fields.length} field(s) from the query, ` +
    `${computed.length} computed, ${derived.length} derived, ${structs.size} struct(s)`,
);
