// Fails if web/i18n/en.json and fr.json differ in keys or placeholders, or if a
// key used by the social v2 page (code, HTML or fixture) is missing.
// Usage: node tools/check_i18n.mjs
import fs from 'node:fs';
import path from 'node:path';

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, '$1')), '..');
const web = path.join(root, 'web');
const load = (f) => JSON.parse(fs.readFileSync(path.join(web, 'i18n', f), 'utf8'));
const en = load('en.json');
const fr = load('fr.json');
const errors = [];

for (const k of Object.keys(en)) if (!(k in fr)) errors.push(`missing in fr: ${k}`);
for (const k of Object.keys(fr)) if (!(k in en)) errors.push(`missing in en: ${k}`);
const params = (s) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort().join(',');
for (const k of Object.keys(en)) if (k in fr && params(en[k]) !== params(fr[k])) errors.push(`placeholder mismatch: ${k}`);
for (const [lang, d] of [['en', en], ['fr', fr]]) for (const [k, v] of Object.entries(d)) if (!String(v).trim()) errors.push(`empty value: ${lang}:${k}`);

// Keys used literally in code and markup.
const used = new Set();
const files = ['social-app.js', 'social-graph.js', 'social-timeline.js', 'population-balance.js', 'entity-sheet.js', 'history-model.js', 'social.html'];
for (const f of files) {
  const src = fs.readFileSync(path.join(web, f), 'utf8');
  for (const m of src.matchAll(/\bt\('([\w.]+)'/g)) used.add(m[1]);
  for (const m of src.matchAll(/'((?:graph|sheet|balance|timeline|page|app|term)\.[\w.]+)'/g)) used.add(m[1]);
  for (const m of src.matchAll(/data-i18n(?:-[\w-]+)?="([\w.]+)"/g)) used.add(m[1]);
}
// Keys built from a data value: the allowed values of each family.
const families = {
  'conduct.{}.person': ['lawless', 'crooked', 'ordinary', 'upright', 'saintly'],
  'conduct.{}.org': ['lawless', 'crooked', 'ordinary', 'upright', 'saintly'],
  'legal.{}': ['licit', 'tolerated', 'proscribed'],
  'family.{}': ['structure', 'family', 'affinity', 'activity', 'power', 'economy', 'history'],
  'predicate.{}': Object.keys(JSON.parse(JSON.stringify({}))).concat(
    ['component_of', 'member_of', 'affiliated_with', 'represents', 'parent_of', 'union_with', 'adopted_by', 'trusts', 'helps', 'rivals', 'shares_past_with',
      'employed_by', 'supplies', 'holds_office', 'governs', 'protects', 'commands', 'owes_allegiance_to', 'owns', 'controls', 'claims', 'trades_with',
      'allied_with', 'in_conflict_with', 'successor_of']),
  'kind.{}': ['person', 'village', 'jurisdiction', 'municipality', 'family', 'guild', 'criminal_network', 'charitable_order', 'enterprise', 'office'],
  'nature.{}': ['person', 'organized_group', 'community', 'category', 'population'],
  'person_source.{}': ['lineage_simulated', 'lineage_projected', 'procedural', 'author'],
  'provenance.status.{}': ['imported', 'derived', 'synthetic', 'unavailable'],
  'coverage.{}': ['aggregate', 'partial', 'complete'],
  'precision.{}': ['expected', 'realized', 'exact'],
  'tenure_mode.{}': ['effective', 'vacant', 'claimed', 'regent'],
  'locale.{}': ['en', 'fr'],
};
for (const [pattern, values] of Object.entries(families)) for (const v of values) used.add(pattern.replace('{}', v));

// Keys carried by the fixture (<field>_key, {key}, and data-driven families).
const fixture = JSON.parse(fs.readFileSync(path.join(web, 'fixtures', 'social-view.demo.json'), 'utf8'));
const dataUsed = (node) => {
  if (Array.isArray(node)) return node.forEach(dataUsed);
  if (!node || typeof node !== 'object') return;
  for (const [k, v] of Object.entries(node)) {
    if ((k.endsWith('_key') || k === 'key') && typeof v === 'string') used.add(v);
    else if (k === 'unit' && typeof v === 'string') used.add(`unit.${v}`);
    else if (k === 'basis' && typeof v === 'string' && v) used.add(`basis.${v}`);
    else if (k === 'appointment_rule') used.add(`rule.${v}`);
    else if (k === 'kind' && typeof v === 'string' && node.year !== undefined) used.add(`event_kind.${v}`);
    else dataUsed(v);
  }
};
dataUsed(fixture);

// Diagnostics: every stable code raised by the Rust v2 core and host has a `diagnostics.` message, and no message is orphaned.
const diagNamespaces = ['address', 'alignment', 'atlas', 'canon', 'canonical', 'catalogue', 'domain', 'entity', 'event', 'graph', 'id', 'impact', 'interval',
  'legal', 'name_tree', 'object_ref', 'office', 'person_table', 'plan', 'population', 'region', 'relation', 'social', 'target'];
const hostNamespaces = ['orchestrator', 'store', 'worker'];
const diagSources = [[path.join(root, 'crates', 'chronos-core', 'src', 'v2'), diagNamespaces], [path.join(root, 'crates', 'chronos-app', 'src', 'v2'), hostNamespaces]];
const codes = new Set();
for (const [dir, namespaces] of diagSources) for (const f of fs.readdirSync(dir).filter((n) => n.endsWith('.rs'))) {
  const src = fs.readFileSync(path.join(dir, f), 'utf8');
  for (const m of src.matchAll(/"([a-z0-9_]+(?:\.[a-z0-9_]+)+)"/g)) if (namespaces.includes(m[1].split('.')[0])) codes.add(m[1]);
}
for (const c of codes) used.add(`diagnostics.${c}`);
for (const k of Object.keys(en)) if (k.startsWith('diagnostics.') && !codes.has(k.slice('diagnostics.'.length))) errors.push(`diagnostic never raised: ${k}`);

for (const k of used) {
  if (!(k in en)) errors.push(`used but missing in en: ${k}`);
  if (!(k in fr)) errors.push(`used but missing in fr: ${k}`);
}
if (errors.length) {
  console.error(errors.join('\n'));
  process.exit(1);
}
console.log(`i18n ok: ${Object.keys(en).length} keys in both locales, ${used.size} used keys checked.`);
