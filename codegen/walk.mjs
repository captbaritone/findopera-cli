// Walking the query against the schema to produce the template field surface.

import { getNamedType, isNonNullType } from "graphql";
import { alwaysPresent, classify, rustLeaf, snake } from "./schema.mjs";

/**
 * Walk one selection set, collecting template fields and the Rust structs
 * needed to deserialize them.
 *
 * A field's presence is the AND of every link above it: `opera.title` is only
 * always-present if the recording always has an opera *and* that opera always
 * has a title. Getting this backwards is the one dangerous direction — it
 * would let a template pass parsing and then fail on the first record that
 * lacks the field.
 */
export function walk(schema, rootType, selectionSet) {
  const fields = []; // template fields, in query order
  const structs = new Map(); // GraphQL type name -> Rust struct definition
  const lists = new Map(); // path -> what a derived field can project from

  // `collect` is false inside a list: the element's scalars build a struct so
  // the data can be deserialized, but they are not template paths — the
  // language has no way to say which element you meant. Derived fields in the
  // overlay are how a single value gets projected out.
  function visit(parentType, selections, path, access, present, collect) {
    const struct = structs.get(parentType.name) ?? {
      name: parentType.name,
      fields: [],
    };
    structs.set(parentType.name, struct);

    for (const sel of selections.selections) {
      if (sel.kind !== "Field") {
        throw new Error(`only plain fields are supported, found ${sel.kind}`);
      }
      const name = sel.name.value;
      const out = sel.alias?.value ?? name;
      const def = parentType.getFields()[name];
      if (!def) throw new Error(`${parentType.name}.${name} is not in the schema`);

      // Two different questions, and conflating them was a bug worth naming.
      // `local` is whether this field is present within a parent that exists —
      // that is what the Rust struct field type must say. `here` is whether
      // the whole chain is present, which is what a template path needs: a
      // language's name is never missing, but the language itself may be.
      const local = alwaysPresent(def);
      const here = present && local;
      const childPath = path ? `${path}.${out}` : out;
      const rustName = snake(out);
      const named = getNamedType(def.type);
      const kind = classify(def.type);

      // Deserializing the same type twice must agree, and it will: the type is
      // keyed by name, so a second visit adds nothing new.
      const known = struct.fields.some((f) => f.rust_name === rustName);

      if (kind === "leaf") {
        const leaf = rustLeaf(named.name);
        if (!known) {
          struct.fields.push({
            rust_name: rustName,
            wire_name: out,
            rust_type: local ? leaf.rust : `Option<${leaf.rust}>`,
            collapse: local ? null : leaf.kind, // sentinel handling
            present: local,
          });
        }
        if (collect) {
          fields.push({
            path: childPath,
            access: [...access, { rust_name: rustName, present: local, leaf: leaf.kind }],
            present: here,
            doc: def.description ?? "",
          });
        }
      } else if (kind === "object") {
        if (!known) {
          struct.fields.push({
            rust_name: rustName,
            wire_name: out,
            rust_type: local ? named.name : `Option<${named.name}>`,
            collapse: null,
            present: local,
          });
        }
        visit(named, sel.selectionSet, childPath, [...access, { rust_name: rustName, present: local }], here, collect);
      } else {
        // A list cannot be a template field on its own — there is no list
        // syntax in the language. It becomes a source the overlay projects
        // single values out of.
        const elemNonNull = isNonNullType(def.type.ofType ?? def.type)
          ? true
          : true; // element nullability is carried by `!` inside the list
        if (!known) {
          struct.fields.push({
            rust_name: rustName,
            wire_name: out,
            rust_type: local ? `Vec<${named.name}>` : `Option<Vec<${named.name}>>`,
            collapse: null,
            present: local,
          });
        }
        lists.set(childPath, {
          rust_name: rustName,
          elem: named.name,
          present: local,
          elemNonNull,
        });
        visit(named, sel.selectionSet, `${childPath}[]`, [], true, false);
      }
    }
  }

  visit(rootType, selectionSet, "", [], true, true);
  return { fields, structs, lists };
}
