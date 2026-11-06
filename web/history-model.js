// Read-only index over a HistoryView plus pure derived functions.
// Nothing is drawn at random here; the conduct class is derived from the axes (§27.2).
// All user-facing text goes through i18n keys; names of world entities are data.

import { t, tk } from './i18n.js';

export const CONDUCT = {
  lawless: { color: '#7a1f2b', glyph: '◆' },
  crooked: { color: '#b4652d', glyph: '▲' },
  ordinary: { color: '#5f7480', glyph: '●' },
  upright: { color: '#2f7a5e', glyph: '■' },
  saintly: { color: '#9a7410', glyph: '★' },
};
export const CONDUCT_ORDER = ['lawless', 'crooked', 'ordinary', 'upright', 'saintly'];
export const conductLabel = (cls, isOrg) => t(`conduct.${cls}.${isOrg ? 'org' : 'person'}`);

export const LEGAL_CLASS = { licit: 'lg-licit', tolerated: 'lg-tolerated', proscribed: 'lg-proscribed' };
export const legalLabel = (status) => t(`legal.${status}`);

// predicate -> relation family (§5.1)
export const PREDICATE_FAMILY = {
  component_of: 'structure', member_of: 'structure', affiliated_with: 'structure', represents: 'structure',
  parent_of: 'family', union_with: 'family', adopted_by: 'family',
  trusts: 'affinity', helps: 'affinity', rivals: 'affinity', shares_past_with: 'affinity',
  employed_by: 'activity', supplies: 'activity',
  holds_office: 'power', governs: 'power', protects: 'power', commands: 'power', owes_allegiance_to: 'power',
  owns: 'economy', controls: 'economy', claims: 'economy', trades_with: 'economy',
  allied_with: 'history', in_conflict_with: 'history', successor_of: 'history',
};
export const FAMILIES = ['structure', 'family', 'affinity', 'activity', 'power', 'economy', 'history'];
export const familyLabel = (f) => t(`family.${f}`);
export const predFamily = (p) => PREDICATE_FAMILY[p] ?? 'history';
export const predLabel = (p) => (p in PREDICATE_FAMILY ? t(`predicate.${p}`) : p.replace(/_/g, ' '));

export const kindLabel = (kind) => t(`kind.${kind}`);
export const natureLabel = (nature) => t(`nature.${nature}`);
export const personSourceLabel = (s) => t(`person_source.${s}`);
export const eventKindLabel = (k) => t(`event_kind.${k}`);

export const EVENT_COLOR = {
  charter: '#2e5a6e', foundation: '#5d7f52', edict: '#8a3c41', epidemic: '#6d4f7a', death: '#5b5560', election: '#47708a',
  debt_default: '#9a6a3a', affiliation: '#4f8a8b', crime: '#7a1f2b', famine: '#9a6a3a', heresy_trial: '#7a4a6a',
  banishment: '#6e3438', succession: '#2e5a6e',
};

// Pure function of the axes (§27.2). `isOrg` selects the organisation thresholds.
export function conductClass(m, l, isOrg) {
  if (m <= -6000 && l <= -3000) return 'lawless';
  if (m <= -2500) return 'crooked';
  if (m < 2500) return 'ordinary';
  if (m < (isOrg ? 5500 : 7000)) return 'upright';
  return 'saintly';
}

export const inRange = (valid, y) => y >= valid[0] && (valid[1] == null || y < valid[1]);

// Name of an entity: a fixture name is data, a generated one is a key plus parameters.
// A parameter ending in `_key` is itself a key (e.g. the kind of an organization).
export function nameOf(e) {
  if (!e) return '';
  if (e.name_key && e.name_params) {
    const params = {};
    for (const [k, v] of Object.entries(e.name_params)) {
      if (k.endsWith('_key')) params[k.slice(0, -4)] = t(String(v));
      else params[k] = v;
    }
    return t(e.name_key, params);
  }
  return tk(e, 'name');
}

export class Model {
  constructor(view) {
    this.view = view;
    this.index();
  }
  // The view object is replaced in place when the year moves; the model re-reads it.
  index() {
    const view = this.view;
    this.byId = new Map();
    for (const e of view.entities) this.byId.set(e.id, e);
    for (const o of view.offices ?? []) this.byId.set(o.id, o);
    this.events = new Map(view.events.map((e) => [e.id, e]));
    // A holder not (yet) in the view keeps the label the tenure carries.
    this.holders = new Map(view.tenures.filter((x) => x.person_id && x.person_name).map((x) => [x.person_id, x.person_name]));
    this.relations = view.relations;
    this.adj = new Map();
    for (const r of this.relations) {
      for (const id of [r.subject, r.object]) {
        if (!this.adj.has(id)) this.adj.set(id, []);
        this.adj.get(id).push(r);
      }
    }
  }
  // Adds the facts of a fragment (graph expansion) without duplicating any ID.
  merge(fragment) {
    const view = this.view;
    const add = (list, extra, key) => {
      const have = new Set(list.map((x) => x[key]));
      let n = 0;
      for (const x of extra ?? []) if (!have.has(x[key])) { list.push(x); have.add(x[key]); n++; }
      return n;
    };
    let added = add(view.entities, fragment.entities, 'id');
    added += add(view.offices, fragment.offices, 'id');
    added += add(view.relations, fragment.relations, 'id');
    added += add(view.legal_standings, fragment.legal_standings, 'id');
    added += add(view.events, fragment.events, 'id');
    added += add(view.tenures, fragment.tenures, 'tenure_id');
    if (added) this.index();
    return added;
  }
  get(id) { return this.byId.get(id); }
  // Display name: a proper name is data, an office or event name is a key.
  name(id) {
    const e = this.byId.get(id);
    if (e) return nameOf(e);
    const ev = this.events.get(id);
    if (ev) return tk(ev, 'label');
    const holder = this.holders.get(id);
    return holder ? nameOf(holder) : id;
  }
  isSynthetic(id) { return !!this.byId.get(id)?.name_synthetic; }
  exists(id, y) {
    const e = this.byId.get(id);
    if (!e) return false;
    return e.kind === 'office' ? true : inRange(e.existence, y);
  }
  alignment(id, y) {
    const e = this.byId.get(id);
    return e?.alignment_episodes?.find((a) => inRange(a.valid, y)) ?? null;
  }
  // A category, a community or an office has no conduct class.
  conduct(id, y) {
    const e = this.byId.get(id);
    if (!e || e.kind === 'office' || e.nature === 'category' || e.nature === 'population') return null;
    const a = this.alignment(id, y);
    if (!a) return null;
    const org = e.nature !== 'person';
    const cls = conductClass(a.morality_bps, a.lawfulness_bps, org);
    return { cls, label: conductLabel(cls, org), ...CONDUCT[cls], axes: a };
  }
  relationsAt(id, y) { return (this.adj.get(id) ?? []).filter((r) => inRange(r.valid, y)); }
  tenureAt(officeId, y) { return this.view.tenures.find((x) => x.office_id === officeId && inRange(x.valid, y)) ?? null; }
  // Realized members of a group in a year, from the series carried by the view.
  membersAt(id, y) {
    const s = this.byId.get(id)?.member_series;
    return s && y >= s.from && y - s.from < s.counts.length ? s.counts[y - s.from] : null;
  }
  legalFor(subjectId) { return this.view.legal_standings.filter((s) => s.subject === subjectId); }
  jurisdictions(subjectId) { return [...new Set(this.legalFor(subjectId).map((s) => s.jurisdiction))]; }
  // No segment means licit (§27.3).
  legalAt(subjectId, jurisdictionId, y) {
    return this.legalFor(subjectId).find((s) => s.jurisdiction === jurisdictionId && inRange(s.valid, y)) ?? null;
  }
  consequences(eventId) { return this.view.events.filter((e) => e.cause_ids.includes(eventId)); }
  eventsFor(id) { return this.view.events.filter((e) => e.subject_ids.includes(id)); }
}
