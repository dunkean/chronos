// Read-only indexes over a World returned by the API. Nothing here generates
// or alters facts: every value comes from the loaded world.

import { compareText } from './util.js';

const ARRAY_FIELDS = ['sources', 'warnings', 'sites', 'routes', 'polities', 'organizations', 'people', 'events', 'snapshots'];

// Tolerates missing arrays (contract says they are always present) and
// reports structural problems instead of throwing, so the UI can still show
// what is usable.
export function normalizeWorld(raw) {
  const problems = [];
  if (!raw || typeof raw !== 'object') throw new Error('Le monde reçu n’est pas un objet JSON.');
  const world = { ...raw };
  for (const f of ARRAY_FIELDS) {
    if (!Array.isArray(world[f])) {
      if (world[f] !== undefined) problems.push(`Champ « ${f} » non tableau, ignoré.`);
      world[f] = [];
    }
  }
  world.config = world.config && typeof world.config === 'object' ? world.config : {};
  const m = world.map;
  if (!m || typeof m !== 'object') {
    problems.push('Aucune carte (champ « map ») : les lieux sont placés sur un fond neutre.');
    world.map = null;
  } else if (!Number.isInteger(m.width) || !Number.isInteger(m.height) || m.width <= 0 || m.height <= 0
    || !Array.isArray(m.elevation_m) || m.elevation_m.length !== m.width * m.height) {
    problems.push(`Grille d’altitude incohérente (${m.width}×${m.height}, ${Array.isArray(m.elevation_m) ? m.elevation_m.length : 0} valeurs) : relief non affiché.`);
    world.map = { ...m, elevation_m: null };
  }
  return { world, problems };
}

function pushTo(map, key, value) {
  if (key === null || key === undefined) return;
  const list = map.get(key);
  if (list) list.push(value);
  else map.set(key, [value]);
}

function byEventOrder(a, b) {
  return (a.year - b.year) || ((b.importance ?? 0) - (a.importance ?? 0)) || compareText(a.id, b.id);
}

export function buildIndex(world) {
  const ix = {};
  ix.sites = new Map(world.sites.map((s) => [s.id, s]));
  ix.polities = new Map(world.polities.map((p) => [p.id, p]));
  ix.orgs = new Map(world.organizations.map((o) => [o.id, o]));
  ix.people = new Map(world.people.map((p) => [p.id, p]));
  ix.routes = new Map(world.routes.map((r) => [r.id, r]));
  ix.capitals = new Set(world.polities.map((p) => p.capital_id).filter(Boolean));

  ix.sitesSorted = [...world.sites].sort((a, b) => compareText(a.name, b.name));
  ix.orgsSorted = [...world.organizations].sort((a, b) => compareText(a.name, b.name));
  ix.politiesSorted = [...world.polities].sort((a, b) => compareText(a.name, b.name));
  ix.peopleSorted = [...world.people].sort((a, b) => compareText(a.name, b.name) || (a.birth_year - b.birth_year));

  // Snapshots, sorted by year; per-snapshot lookup by site.
  const snaps = world.snapshots.filter((s) => Number.isFinite(s.year)).sort((a, b) => a.year - b.year);
  ix.snaps = snaps;
  ix.snapYears = snaps.map((s) => s.year);
  ix.snapStates = snaps.map((s) => new Map((s.sites ?? []).map((st) => [st.site_id, st])));
  ix.totalPop = snaps.map((s) => (s.sites ?? []).reduce((acc, st) => acc + (Number(st.population) || 0), 0));
  ix.maxSitePop = 0;
  for (const s of snaps) for (const st of s.sites ?? []) if (st.population > ix.maxSitePop) ix.maxSitePop = st.population;
  for (const s of world.sites) if (s.initial_population > ix.maxSitePop) ix.maxSitePop = s.initial_population;

  // Events.
  ix.events = [...world.events].sort(byEventOrder);
  ix.eventById = new Map(ix.events.map((e) => [e.id, e]));
  ix.eventsBySite = new Map();
  ix.eventsByOrg = new Map();
  ix.eventsByPerson = new Map();
  ix.consequences = new Map();
  ix.kindCounts = new Map();
  for (const e of ix.events) {
    for (const id of e.site_ids ?? []) pushTo(ix.eventsBySite, id, e);
    for (const id of e.organization_ids ?? []) pushTo(ix.eventsByOrg, id, e);
    for (const id of e.person_ids ?? []) pushTo(ix.eventsByPerson, id, e);
    for (const id of e.causes ?? []) pushTo(ix.consequences, id, e);
    ix.kindCounts.set(e.kind, (ix.kindCounts.get(e.kind) ?? 0) + 1);
  }
  ix.kinds = [...ix.kindCounts.keys()].sort((a, b) => (ix.kindCounts.get(b) - ix.kindCounts.get(a)) || compareText(a, b));

  // Relations.
  ix.routesBySite = new Map();
  for (const r of world.routes) {
    pushTo(ix.routesBySite, r.from, r);
    if (r.to !== r.from) pushTo(ix.routesBySite, r.to, r);
  }
  ix.orgsBySite = new Map();
  ix.orgsByPolity = new Map();
  ix.orgsByHolder = new Map();
  for (const o of ix.orgsSorted) {
    for (const id of o.site_ids ?? []) pushTo(ix.orgsBySite, id, o);
    pushTo(ix.orgsByPolity, o.polity_id, o);
    pushTo(ix.orgsByHolder, o.holder_id, o);
  }
  ix.peopleBySite = new Map();
  ix.children = new Map();
  for (const p of ix.peopleSorted) {
    pushTo(ix.peopleBySite, p.site_id, p);
    pushTo(ix.children, p.father_id, p);
    if (p.mother_id !== p.father_id) pushTo(ix.children, p.mother_id, p);
  }
  for (const list of ix.children.values()) list.sort((a, b) => a.birth_year - b.birth_year || compareText(a.name, b.name));
  ix.genealogyEnd = Number.isFinite(world.genealogy_end_year) ? world.genealogy_end_year : null;
  ix.genealogyEndEstimated = false;
  if (ix.genealogyEnd === null) {
    for (const p of world.people) {
      if (p.source === 'virtual') continue;
      const y = Number.isFinite(p.death_year) ? p.death_year : p.birth_year;
      if (Number.isFinite(y) && (ix.genealogyEnd === null || y > ix.genealogyEnd)) ix.genealogyEnd = y;
    }
    ix.genealogyEndEstimated = ix.genealogyEnd !== null;
  }

  // Displayed time domain: snapshots first, configuration otherwise, widened to cover events.
  const cfg = world.config ?? {};
  let lo = snaps.length ? snaps[0].year : Number(cfg.start_year);
  let hi = snaps.length ? snaps[snaps.length - 1].year : Number(cfg.start_year) + Number(cfg.years);
  for (const e of ix.events) {
    if (Number.isFinite(e.year) && (!Number.isFinite(lo) || e.year < lo)) lo = e.year;
    const end = Number.isFinite(e.end_year) ? e.end_year : e.year;
    if (Number.isFinite(end) && (!Number.isFinite(hi) || end > hi)) hi = end;
  }
  if (!Number.isFinite(lo)) lo = 1300;
  if (!Number.isFinite(hi) || hi < lo) hi = lo;
  ix.minYear = lo;
  ix.maxYear = hi;
  return ix;
}

// Index of the snapshot in force at `year` (last one with snapshot.year <= year), or -1.
export function snapIndexAt(ix, year) {
  const ys = ix.snapYears;
  let lo = 0;
  let hi = ys.length - 1;
  let found = -1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    if (ys[mid] <= year) { found = mid; lo = mid + 1; } else hi = mid - 1;
  }
  return found;
}

// Index of the first snapshot with snapshot.year >= year, or -1.
export function snapIndexFrom(ix, year) {
  const ys = ix.snapYears;
  for (let i = 0; i < ys.length; i += 1) if (ys[i] >= year) return i;
  return -1;
}

export function stateAt(ix, siteId, year) {
  const i = snapIndexAt(ix, year);
  return i < 0 ? null : ix.snapStates[i].get(siteId) ?? null;
}

export function exactSnapshot(ix, year) {
  const i = snapIndexAt(ix, year);
  return i >= 0 && ix.snapYears[i] === year ? i : -1;
}

export function eventEnd(e) {
  return Number.isFinite(e.end_year) && e.end_year >= e.year ? e.end_year : e.year;
}

export function eventActiveIn(e, from, to) {
  return e.year <= to && eventEnd(e) >= from;
}

export function sitePolityAt(ix, site, year) {
  const st = stateAt(ix, site.id, year);
  return (st && st.polity_id) || site.polity_id || null;
}

// Life status at `year`. A recorded death year is the last year of life
// (participation stops before it). An imported person without a recorded
// death is right-censored: known only up to the end of the imported sample,
// not immortal. The coverage end comes from an explicit field when the world
// provides one, otherwise from the same estimate as the core (latest recorded
// birth or death among imported people).
export function personStatusAt(p, year, ix) {
  const age = Number.isFinite(p.birth_year) ? year - p.birth_year : null;
  if (Number.isFinite(p.birth_year) && year < p.birth_year) return { phase: 'before' };
  if (Number.isFinite(p.death_year)) {
    if (year > p.death_year) return { phase: 'after' };
    if (year === p.death_year) return { phase: 'dies', age };
    return { phase: 'alive', age };
  }
  if (p.source !== 'virtual') {
    const end = Number.isFinite(p.known_until_year) ? p.known_until_year : ix?.genealogyEnd;
    if (Number.isFinite(end) && year > end) return { phase: 'unknown', end };
  }
  return { phase: 'alive', age };
}

export function personLiving(st) {
  return st.phase === 'alive' || st.phase === 'dies';
}

// Office holder at `year`, read from the export: `holder_id` is the holder at
// start_year and each later `succession` event citing the organization lists
// person_ids = [new holder, previous holder] (core convention, either may be
// missing when the office is vacant). Returns { id, since } where `since` is
// the succession event that installed the holder, or null for the initial one.
export function holderAt(ix, org, year) {
  let id = org.holder_id ?? null;
  let since = null;
  for (const e of ix.eventsByOrg.get(org.id) ?? []) {
    if (e.year > year) break;
    if (e.kind !== 'succession') continue;
    const ids = e.person_ids ?? [];
    if (ids.length >= 2) id = ids[0];
    else if (ids.length === 1) id = ids[0] === id ? null : ids[0];
    else id = null;
    since = e;
  }
  return { id, since };
}

export function siblingsOf(ix, p) {
  const seen = new Map();
  for (const parent of [p.father_id, p.mother_id]) {
    if (!parent) continue;
    for (const c of ix.children.get(parent) ?? []) if (c.id !== p.id) seen.set(c.id, c);
  }
  return [...seen.values()].sort((a, b) => a.birth_year - b.birth_year);
}
