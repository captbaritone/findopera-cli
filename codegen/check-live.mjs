// Do this build's queries still work against findopera.com as it is today?
//
// `generate.mjs` validates the same documents against `schema/schema.graphql`,
// the copy vendored in this repo. That proves the queries agree with the
// schema we last fetched; it cannot notice that the schema has moved since.
// The two are different questions, and only the second one breaks users.
//
// It went wrong exactly there: findopera.com dropped its Spotify track data,
// `GetSpotifyAlbum` went on selecting `tracks`, and because one invalid field
// fails a whole operation, `findopera get spotify-album` would have stopped
// working for a type that was never removed. Nothing in this repo had changed,
// so nothing here ran.
//
// Which is why this is worth running on a schedule rather than only on a push.
// The change that breaks this CLI is, by its nature, a commit in another repo.
//
// Only breakage fails the build. A schema that has grown fields we do not ask
// for is reported and forgiven: the API adds things continually, and a check
// that cried about every addition would be turned off within the month.
import { buildSchema, parse, validate } from "graphql";
import { readFileSync } from "node:fs";

const SCHEMA_URL = process.env.SCHEMA_URL ?? "https://findopera.com/schema.graphql";
const DOCUMENTS = [
  "schema/recording.graphql",
  "schema/search.graphql",
  "schema/get.graphql",
];

const at = (p) => new URL(`../${p}`, import.meta.url);
const version =
  readFileSync(at("Cargo.toml"), "utf8").match(/^version\s*=\s*"([^"]+)"/m)?.[1] ??
  "unknown";

async function fetchSchema() {
  // Retried rather than failed: a red build nobody caused teaches people to
  // ignore red builds.
  let lastError;
  for (let attempt = 1; attempt <= 3; attempt++) {
    try {
      const response = await fetch(SCHEMA_URL, {
        headers: { "User-Agent": `findopera-cli-ci/${version}` },
        signal: AbortSignal.timeout(30_000),
      });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      return await response.text();
    } catch (error) {
      lastError = error;
      if (attempt < 3) await new Promise((r) => setTimeout(r, attempt * 2000));
    }
  }
  throw new Error(`could not fetch ${SCHEMA_URL}: ${lastError.message}`);
}

const liveSrc = await fetchSchema();
const live = buildSchema(liveSrc);
const vendored = readFileSync(at("schema/schema.graphql"), "utf8");

let broken = 0;
for (const file of DOCUMENTS) {
  const errors = validate(live, parse(readFileSync(at(file), "utf8")));
  if (errors.length === 0) {
    console.log(`  ok    ${file}`);
    continue;
  }
  broken += errors.length;
  console.log(`  FAIL  ${file}`);
  for (const error of errors) {
    const line = error.locations?.[0]?.line;
    console.log(`          ${error.message}${line ? `  (line ${line})` : ""}`);
  }
}

if (broken > 0) {
  console.error(
    `\n${broken} of this build's queries no longer validate against ${SCHEMA_URL}.\n` +
      `Anyone running this version is getting an error instead of an answer.\n\n` +
      `To fix: npm run fetch-schema && npm run generate, then repair whatever\n` +
      `in schema/*.graphql asks for something the API no longer has.`,
  );
  process.exit(1);
}

// Compatible, but not identical. Worth saying, never worth failing over, and
// said without guessing which way it runs: the vendored copy is ahead of the
// live one whenever this repo is ready for a deploy that has not happened yet.
console.log(
  vendored.trim() === liveSrc.trim()
    ? "\nAll queries validate, and the vendored schema matches the live one."
    : "\nAll queries validate. The vendored schema differs from the live one, " +
        "which is\nexpected while a change is in flight: npm run fetch-schema " +
        "once it has landed.",
);
