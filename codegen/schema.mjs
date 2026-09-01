// Reading nullability out of the schema.
//
// FindOpera's schema declares nothing with `!` — every field is nullable in
// the strict GraphQL sense, because a null anywhere can be an error. What it
// does carry is `@semanticNonNull`, which says a position is only ever null
// when there is a matching entry in the response's `errors` array. That is
// exactly the property a template needs: "this field has a value for every
// record", as distinct from "this key is never JSON null".

import { getNamedType, isListType, isNonNullType, isLeafType } from "graphql";

/** The levels `@semanticNonNull` was applied at, or null if it was not. */
function semanticLevels(field) {
  const d = field.astNode?.directives?.find(
    (d) => d.name.value === "semanticNonNull",
  );
  if (!d) return null;
  const levels = d.arguments?.find((a) => a.name.value === "levels");
  if (!levels) return [0]; // the directive's own default
  return levels.value.values.map((v) => Number(v.value));
}

/**
 * Is this field's own value always present?
 *
 * Level 0 is the field position itself; deeper levels describe list elements,
 * which only matter to the derived fields declared in the overlay.
 */
export function alwaysPresent(field) {
  if (isNonNullType(field.type)) return true;
  const levels = semanticLevels(field);
  return levels !== null && levels.includes(0);
}

export function classify(type) {
  const bare = isNonNullType(type) ? type.ofType : type;
  if (isListType(bare)) return "list";
  return isLeafType(getNamedType(bare)) ? "leaf" : "object";
}

/** The Rust type a GraphQL leaf deserializes into, and how it becomes a string. */
export function rustLeaf(name) {
  switch (name) {
    case "String":
    case "ID":
    case "DateTime":
      return { rust: "String", kind: "text" };
    case "Int":
      return { rust: "i64", kind: "num" };
    case "Float":
      return { rust: "f64", kind: "num" };
    case "Boolean":
      return { rust: "bool", kind: "bool" };
    default:
      throw new Error(`no Rust mapping for leaf type ${name}`);
  }
}

export const snake = (s) => s.replace(/[A-Z]/g, (c) => "_" + c.toLowerCase());
