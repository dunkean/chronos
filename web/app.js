// Chronos workshop: wires the API, settings, map, year rail and side panel.
// The browser never invents facts: it displays the World the server returns,
// and navigation (year, filters, records) only reads it.

import { api } from './api.js';
import {
  h, clear, swatch, fmtInt, fmtNum, fmtYear, fmtMs, shortId, kindInfo, siteKindLabel,
  sourceStatus, safeColor, humanLabel, clamp,
} from './util.js';
import { normalizeWorld, buildIndex, snapIndexAt, stateAt, eventActiveIn, sitePolityAt } from './model.js';
import { ConfigForm, CONTRACT_DEFAULTS, configsDiffer } from './config-form.js';
import { MapView, mapGeometry, rampCss, routeStyle } from './map-view.js';
import { Timeline } from './timeline.js';
import { Chronicle, EntityIndex, neighbourEventYear } from './chronicle.js';
import { renderSite, renderEvent, renderOrg, renderPerson, renderPolity } from './inspector.js';

const $ = (id) => document.getElementById(id);

const state = {
  world: null,
  rawText: null,
  ix: null,
  problems: [],
  defaultConfig: null,
  serverSources: [],
  capabilities: [],
  year: 1300,
  selection: null,
  back: [],
  forward: [],
  tab: 'chronicle',
  busy: false,
  playTimer: 0,
};

// --- Notices -----------------------------------------------------------------

function setStatus(text) {
  $('status').textContent = text;
}

function showError(text) {
  $('error-text').textContent = text;
  $('error').hidden = false;
}

function clearError() {
  $('error').hidden = true;
}

function mapEmpty(mode, title = '', text = '') {
  const box = $('map-empty');
  box.hidden = mode === 'none';
  box.dataset.mode = mode;
  $('map-empty-title').textContent = title;
  $('map-empty-text').textContent = text;
  $('map-empty-actions').hidden = mode !== 'empty';
}

// --- Components ----------------------------------------------------------------

const form = new ConfigForm($('cfg-fields'), {
  onChange: updateDirty,
  onPreset: (p) => setStatus(`Préréglage « ${p.label} » appliqué au formulaire. Cliquez sur Générer pour produire ce monde.`),
});

const map = new MapView($('map-wrap'), {
  onSelect: (sel) => select(sel),
  describeSite,
});

const timeline = new Timeline($('tl-wrap'), {
  onYear: (y) => { stopPlay(); setYear(y); },
  onSelectEvent: (e) => select({ type: 'event', id: e.id }),
});

const chronicle = new Chronicle($('panel-chronicle'), { onFilterChange: filtersChanged });
const entityIndex = new EntityIndex($('panel-index'));

// --- World loading ---------------------------------------------------------------

function loadWorld(raw, text, origin) {
  let norm;
  try {
    norm = normalizeWorld(raw);
  } catch (err) {
    showError(`Monde inutilisable : ${err.message}`);
    return false;
  }
  const { world, problems } = norm;
  const ix = buildIndex(world);
  state.world = world;
  state.rawText = text;
  state.ix = ix;
  state.problems = problems;
  state.back = [];
  state.forward = [];

  map.setWorld(world, ix);
  form.setValues(world.config);
  form.setWorldInfo({ mapMode: actualMapMode(world), radiusKm: world.map?.radius_km, siteCount: world.sites.length });
  chronicle.setWorld(ix);
  entityIndex.setWorld(world, ix);

  const range = $('year-range');
  range.min = String(ix.minYear);
  range.max = String(ix.maxYear);
  range.disabled = false;
  const yi = $('year-input');
  yi.min = String(ix.minYear);
  yi.max = String(ix.maxYear);
  yi.disabled = false;
  $('btn-export').disabled = !text;
  $('rail-info').textContent = ix.snaps.length
    ? `${fmtYear(ix.minYear)}–${fmtYear(ix.maxYear)}, ${fmtInt(ix.snaps.length)} relevés annuels`
    : `${fmtYear(ix.minYear)}–${fmtYear(ix.maxYear)}, aucun relevé d’état`;

  // Keep the year and record across regeneration when they still exist.
  const fromHash = readHash();
  const wanted = state.selection ?? fromHash.selection;
  state.selection = wanted && entityExists(wanted) ? wanted : null;
  const startYear = Number.isFinite(world.config.start_year) ? world.config.start_year : ix.minYear;
  const keepYear = state.loadedOnce ? state.year : (fromHash.year ?? startYear);
  state.loadedOnce = true;

  renderHeader();
  renderLegend();
  renderSourcesSummary();
  if ($('sources-dialog').open) renderSources();
  mapEmpty('none');
  $('cartouche').hidden = false;
  $('map-legend').hidden = false;
  filtersChanged(false);
  setYear(keepYear, true);
  applySelection(false);
  updateDirty();

  const w = world;
  const parts = [
    `${origin} ${fmtInt(w.sites.length)} lieux, ${fmtInt(w.routes.length)} routes, ${fmtInt(w.events.length)} événements, ${fmtInt(w.people.length)} personnes`,
  ];
  if (Number.isFinite(w.generation_ms)) parts.push(`généré en ${fmtMs(w.generation_ms)}`);
  const nWarn = w.warnings.length + problems.length;
  if (nWarn) parts.push(`${nWarn} avertissement${nWarn > 1 ? 's' : ''}, voir Sources`);
  setStatus(parts.join(' ; ') + '.');
  return true;
}

function entityExists(sel) {
  const ix = state.ix;
  if (!ix || !sel) return false;
  const maps = { site: ix.sites, event: ix.eventById, org: ix.orgs, person: ix.people, polity: ix.polities };
  return !!maps[sel.type]?.has(sel.id);
}

// Map actually used, from provenance rather than the requested map_mode:
// a stack request falls back to the virtual map when the fixture is missing.
function actualMapMode(world) {
  const stack = world.sources.find((s) => s.id === 'stack-fixture');
  if (stack) return stack.status === 'imported' ? 'stack' : 'fallback';
  const src = String(world.map?.source ?? '');
  if (world.config.map_mode === 'stack' && src && !/virtual|synth/i.test(src)) return 'stack';
  return world.config.map_mode === 'stack' ? 'fallback' : 'virtual';
}

const MAP_MODE_LABEL = {
  stack: 'carte importée',
  fallback: 'repli virtuel (carte importée indisponible)',
  virtual: 'monde virtuel',
};

function renderHeader() {
  const w = state.world;
  const c = clear($('canon'));
  if (!w) {
    c.textContent = 'Aucun monde chargé';
    return;
  }
  const cfg = w.config;
  const end = Number.isFinite(cfg.start_year) && Number.isFinite(cfg.years) ? cfg.start_year + cfg.years : state.ix.maxYear;
  c.append(
    h('span', { class: 'canon-id', title: `Identité du canon : ${w.canon_id}` }, `Canon ${shortId(String(w.canon_id).replace(/^canon-/, ''), 12)}`),
    h('span', null, `graine ${cfg.seed}`),
    h('span', null, `${fmtYear(cfg.start_year)}–${fmtYear(end)}`),
    h('span', null, MAP_MODE_LABEL[actualMapMode(w)]),
    h('span', null, (cfg.scenario ?? 'pivot') === 'free' ? 'histoire libre' : 'scénario de l’étude (1356, 1398)'),
  );
}

// --- Year ---------------------------------------------------------------------

function setYear(y, force = false) {
  const ix = state.ix;
  if (!ix) return;
  const year = clamp(Math.round(Number(y)), ix.minYear, ix.maxYear);
  if (!Number.isFinite(year)) return;
  if (!force && year === state.year) {
    $('year-input').value = String(year);
    return;
  }
  state.year = year;
  $('year-input').value = String(year);
  $('year-range').value = String(year);
  $('year-range').setAttribute('aria-valuetext', `Année ${fmtYear(year)}`);
  $('cart-year').textContent = fmtYear(year);
  renderCartouche();
  updateMapFrame();
  timeline.setYear(year);
  chronicle.setYear(year);
  chronicle.render(state.selection?.type === 'event' ? state.selection.id : null);
  entityIndex.setYear(year);
  if (state.tab === 'index') entityIndex.render();
  if (state.selection && state.tab === 'fiche') renderFiche(false);
  const n = state.world.sites.length;
  $('map').setAttribute('aria-label', `Carte de ${n} lieux en ${fmtYear(year)}.`);
  writeHash();
}

function renderCartouche() {
  const ix = state.ix;
  const list = clear($('cart-polities'));
  const si = snapIndexAt(ix, state.year);
  const held = new Map();
  for (const s of state.world.sites) {
    const st = si >= 0 ? ix.snapStates[si].get(s.id) : null;
    if (st && st.population === 0) continue;
    const pid = (st && st.polity_id) || s.polity_id;
    if (pid) held.set(pid, (held.get(pid) ?? 0) + 1);
  }
  const ordered = ix.politiesSorted.filter((p) => held.has(p.id)).sort((a, b) => held.get(b.id) - held.get(a.id));
  for (const p of ordered.slice(0, 6)) {
    list.appendChild(h('li', null, h('button', { type: 'button', class: 'cart-polity', 'data-sel': `polity:${p.id}` },
      swatch(safeColor(p.color)), h('span', { class: 'cart-name' }, p.name), h('span', { class: 'cart-n' }, `${held.get(p.id)} lieux`))));
  }
  if (ordered.length > 6) list.appendChild(h('li', { class: 'cart-more' }, `et ${ordered.length - 6} autres puissances`));
  if (!ordered.length) list.appendChild(h('li', { class: 'cart-more' }, 'Aucune puissance enregistrée'));
}

// --- Filters ---------------------------------------------------------------------

function filtersChanged(render = true) {
  if (!state.ix) return;
  const events = chronicle.matching();
  state.filtered = events;
  timeline.setData(state.ix, events);
  updateMapFrame();
  if (render) chronicle.render(state.selection?.type === 'event' ? state.selection.id : null);
}

function updateMapFrame() {
  const ix = state.ix;
  if (!ix) return;
  const sel = state.selection;
  let focus = [];
  let eventSites = [];
  let eventColor = null;
  if (sel?.type === 'event') {
    const e = ix.eventById.get(sel.id);
    focus = e?.site_ids ?? [];
    eventSites = focus;
    eventColor = e ? kindInfo(e.kind).color : null;
  } else if (sel?.type === 'org') {
    focus = ix.orgs.get(sel.id)?.site_ids ?? [];
  } else if (sel?.type === 'person') {
    const p = ix.people.get(sel.id);
    focus = p?.site_id ? [p.site_id] : [];
  } else if (sel?.type === 'polity') {
    focus = state.world.sites.filter((s) => sitePolityAt(ix, s, state.year) === sel.id).map((s) => s.id);
  }
  map.setFrame({
    year: state.year,
    selection: sel,
    focusSites: new Set(focus),
    eventSites,
    eventColor,
    kindFilter: chronicle.kindSet,
  });
}

function describeSite(id) {
  const ix = state.ix;
  const s = ix?.sites.get(id);
  if (!s) return null;
  const st = stateAt(ix, id, state.year);
  const pop = st ? st.population : s.initial_population;
  const pol = ix.polities.get((st && st.polity_id) || s.polity_id);
  const active = (ix.eventsBySite.get(id) ?? []).filter((e) => eventActiveIn(e, state.year, state.year));
  return [
    h('strong', null, s.name),
    h('span', null, `${siteKindLabel(s.kind)}, ${fmtInt(pop)} hab. en ${fmtYear(state.year)}`),
    pol ? h('span', { class: 'with-swatch' }, swatch(safeColor(pol.color)), pol.name) : null,
    active.length ? h('span', null, active.slice(0, 2).map((e) => e.title).join(' ; ') + (active.length > 2 ? ` et ${active.length - 2} autres` : '')) : null,
  ].filter(Boolean);
}

// --- Selection & records ---------------------------------------------------------------

function select(sel, { record = true } = {}) {
  if (!entityExists(sel)) return;
  const cur = state.selection;
  if (record && cur && !(cur.type === sel.type && cur.id === sel.id)) {
    state.back.push(cur);
    if (state.back.length > 60) state.back.shift();
    state.forward = [];
  }
  state.selection = sel;
  if (sel.type === 'event') {
    const e = state.ix.eventById.get(sel.id);
    if (e && !eventActiveIn(e, state.year, state.year)) setYear(e.year);
  }
  showTab('fiche', false);
  applySelection(true);
}

function applySelection(focusTitle) {
  const sel = state.selection;
  const ev = sel?.type === 'event' ? state.ix.eventById.get(sel.id) : null;
  timeline.setSelected(ev ?? null);
  chronicle.markSelected(ev ? ev.id : null);
  updateMapFrame();
  renderFiche(focusTitle);
  writeHash();
}

function renderFiche(focusTitle) {
  const body = $('fiche-body');
  const scroll = $('panel-fiche').scrollTop;
  clear(body);
  $('fiche-back').disabled = state.back.length === 0;
  $('fiche-fwd').disabled = state.forward.length === 0;
  $('fiche-close').disabled = !state.selection;
  const ix = state.ix;
  const sel = state.selection;
  if (!ix) {
    body.appendChild(h('p', { class: 'empty-line' }, 'Aucun monde chargé.'));
    return;
  }
  if (!sel) {
    body.appendChild(h('div', { class: 'fiche-empty' },
      h('p', null, 'Aucune fiche ouverte.'),
      h('p', { class: 'hint' }, 'Cliquez un lieu sur la carte, un événement de la chronique ou du rail des années, ou une entrée de l’index.')));
    return;
  }
  const ctx = { world: state.world, ix, year: state.year };
  let node = null;
  if (sel.type === 'site') node = renderSite(ctx, ix.sites.get(sel.id));
  else if (sel.type === 'event') node = renderEvent(ctx, ix.eventById.get(sel.id));
  else if (sel.type === 'org') node = renderOrg(ctx, ix.orgs.get(sel.id));
  else if (sel.type === 'person') node = renderPerson(ctx, ix.people.get(sel.id));
  else if (sel.type === 'polity') node = renderPolity(ctx, ix.polities.get(sel.id));
  if (!node) return;
  body.appendChild(node);
  if (focusTitle) {
    $('panel-fiche').scrollTop = 0;
    node.querySelector('.fiche-title')?.focus({ preventScroll: true });
    revealSide();
  } else {
    $('panel-fiche').scrollTop = scroll;
  }
}

// On the one-column layout the side panel sits below the map.
function revealSide() {
  if (!window.matchMedia('(max-width: 899px)').matches) return;
  const side = document.querySelector('.side');
  const r = side.getBoundingClientRect();
  if (r.top > window.innerHeight - 80) {
    side.scrollIntoView({ block: 'start', behavior: window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth' });
  }
}

function goBack() {
  const prev = state.back.pop();
  if (!prev) return;
  if (state.selection) state.forward.push(state.selection);
  state.selection = entityExists(prev) ? prev : null;
  applySelection(true);
}

function goForward() {
  const next = state.forward.pop();
  if (!next) return;
  if (state.selection) state.back.push(state.selection);
  state.selection = entityExists(next) ? next : null;
  applySelection(true);
}

function closeFiche() {
  if (state.selection) state.back.push(state.selection);
  state.selection = null;
  applySelection(false);
  showTab('chronicle', true);
}

// --- Tabs ---------------------------------------------------------------------

const TABS = ['chronicle', 'fiche', 'index'];

function showTab(name, focus) {
  state.tab = name;
  for (const t of TABS) {
    const tab = $(`tab-${t}`);
    const on = t === name;
    tab.setAttribute('aria-selected', on ? 'true' : 'false');
    tab.tabIndex = on ? 0 : -1;
    $(`panel-${t}`).hidden = !on;
  }
  if (name === 'index') entityIndex.render();
  if (name === 'fiche' && !state.selection) renderFiche(false);
  if (focus) $(`tab-${name}`).focus();
}

// --- Settings ---------------------------------------------------------------------

function updateDirty() {
  const el = $('cfg-dirty');
  if (!state.world) {
    el.hidden = true;
    return;
  }
  const differs = configsDiffer(form.read().values, state.world.config);
  el.hidden = !differs;
  el.textContent = differs ? 'Réglages modifiés : la carte montre encore le monde précédent.' : '';
  $('btn-generate').classList.toggle('is-dirty', differs);
}

function setSettingsOpen(open) {
  document.body.classList.toggle('settings-open', open);
  $('btn-settings').setAttribute('aria-expanded', open ? 'true' : 'false');
  requestAnimationFrame(() => { map.requestDraw(); timeline.requestDraw(); });
}

async function generate() {
  if (state.busy) return;
  const { values, errors } = form.read();
  form.showErrors(errors);
  if (errors.length) {
    setSettingsOpen(true);
    showError(`Réglages invalides : ${errors[0]}`);
    $('settings').querySelector('[aria-invalid="true"]')?.focus();
    return;
  }
  clearError();
  setBusy(true, 'Génération en cours…');
  const t0 = performance.now();
  try {
    const { data, text } = await api.generate(values);
    if (loadWorld(data, text, 'Monde généré :')) {
      setStatus(`${$('status').textContent} Réponse en ${fmtMs(performance.now() - t0)}.`);
    }
  } catch (err) {
    showError(`Génération impossible : ${err.message}`);
    setStatus('La génération a échoué ; le monde précédent reste affiché.');
  } finally {
    setBusy(false);
  }
}

function setBusy(busy, label = '') {
  state.busy = busy;
  document.getElementById('layout').setAttribute('aria-busy', busy ? 'true' : 'false');
  for (const b of document.querySelectorAll('#btn-generate, #cfg-form button[type="submit"], #btn-import')) b.disabled = busy;
  $('btn-generate').textContent = busy ? 'Génération…' : 'Générer';
  form.setDisabled(busy);
  clearInterval(state.busyTimer);
  if (busy) {
    const t0 = performance.now();
    const tick = () => mapEmpty(state.world ? 'busy' : 'loading', label, `${fmtNum((performance.now() - t0) / 1000, 0)} s écoulées`);
    tick();
    state.busyTimer = setInterval(tick, 1000);
  } else if (state.world) {
    mapEmpty('none');
  } else {
    mapEmpty('empty', 'Aucun monde chargé', 'Réglez les paramètres puis cliquez sur Générer, ou importez un monde exporté.');
  }
}

// --- Import / export --------------------------------------------------------------

async function importFile(file) {
  if (!file) return;
  clearError();
  let text;
  try {
    text = await file.text();
  } catch {
    showError(`Lecture de « ${file.name} » impossible.`);
    return;
  }
  let parsed;
  try {
    parsed = JSON.parse(text);
  } catch (err) {
    showError(`« ${file.name} » n’est pas un JSON valide : ${err.message}`);
    return;
  }
  if (!parsed || typeof parsed !== 'object' || !('canon_id' in parsed) || !('schema_version' in parsed)) {
    showError(`« ${file.name} » ne ressemble pas à un monde exporté par Chronos : champs schema_version et canon_id attendus.`);
    return;
  }
  setBusy(true, `Import de « ${file.name} »…`);
  try {
    const { data, text: served } = await api.importWorld(text);
    loadWorld(data, served, `Monde importé depuis « ${file.name} » :`);
  } catch (err) {
    showError(`Import refusé par le serveur : ${err.message}`);
  } finally {
    setBusy(false);
  }
}

function exportWorld() {
  if (!state.rawText || !state.world) return;
  const cfg = state.world.config;
  const safe = String(state.world.canon_id ?? 'monde').replace(/[^A-Za-z0-9_-]+/g, '').slice(0, 16) || 'monde';
  const name = `chronos-graine${cfg.seed}-${safe}.json`;
  const url = URL.createObjectURL(new Blob([state.rawText], { type: 'application/json' }));
  const a = h('a', { href: url, download: name });
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 2000);
  setStatus(`Monde exporté : ${name} (JSON tel que renvoyé par le serveur).`);
}

// --- Legend & sources ------------------------------------------------------------

function renderLegend() {
  const body = clear($('legend-body'));
  const range = map.elevationRange;
  if (range && Number.isFinite(range.min)) {
    const n = 12;
    const stops = [];
    for (let i = 0; i <= n; i += 1) {
      const e = range.min + ((range.max - range.min) * i) / n;
      stops.push(`${rampCss(e)} ${((i / n) * 100).toFixed(1)}%`);
    }
    body.appendChild(h('div', { class: 'lg-block' },
      h('p', { class: 'lg-title' }, 'Altitude'),
      h('div', { class: 'lg-ramp', style: { background: `linear-gradient(to right, ${stops.join(', ')})` }, 'aria-hidden': 'true' }),
      h('p', { class: 'lg-ticks' }, h('span', null, `${fmtInt(range.min)} m`), h('span', null, `${fmtInt(range.max)} m`))));
  } else {
    body.appendChild(h('p', { class: 'lg-title' }, 'Relief non disponible : fond neutre.'));
  }
  const kinds = new Map();
  for (const r of state.world.routes) kinds.set(r.kind, (kinds.get(r.kind) ?? 0) + 1);
  if (kinds.size) {
    body.appendChild(h('div', { class: 'lg-block' }, h('p', { class: 'lg-title' }, 'Routes'),
      h('ul', { class: 'lg-list' }, [...kinds].map(([k, n]) => {
        const st = routeStyle(k);
        const style = st.dash.length ? (st.dash[0] < 2 ? 'dotted' : 'dashed') : 'solid';
        return h('li', null, h('span', { class: 'lg-line', style: { borderTop: `2px ${style} ${st.color}` }, 'aria-hidden': 'true' }),
          `${humanLabel(k)} (${fmtInt(n)})`);
      }))));
  }
  body.appendChild(h('div', { class: 'lg-block' }, h('p', { class: 'lg-title' }, 'Lieux'),
    h('ul', { class: 'lg-list' },
      h('li', null, h('span', { class: 'lg-dot' }), 'Surface : population à l’année affichée ; couleur : puissance'),
      h('li', null, h('span', { class: 'lg-dot lg-cap' }), 'Anneau extérieur : capitale'),
      h('li', null, h('span', { class: 'lg-dot lg-empty' }), 'Cercle vide barré : lieu déserté'),
      h('li', null, h('span', { class: 'lg-dot lg-halo' }), 'Halo coloré : événement en cours, couleur du type'))));
  const geo = mapGeometry(state.world.map);
  if (state.world.map) {
    body.appendChild(h('p', { class: 'hint' }, geo.valid
      ? `Grille ${state.world.map.width} × ${state.world.map.height}, environ ${fmtNum(geo.cellWkm, 1)} × ${fmtNum(geo.cellHkm, 1)} km par cellule.`
      : `Grille ${state.world.map.width} × ${state.world.map.height} ; emprise sans échelle physique exploitable.`));
  }
}

function sourcesList() {
  return state.world ? state.world.sources : state.serverSources;
}

function renderSourcesSummary() {
  const el = clear($('src-summary'));
  const counts = new Map();
  for (const s of sourcesList()) counts.set(s.status, (counts.get(s.status) ?? 0) + 1);
  const parts = [...counts].map(([st, n]) => `${n} ${sourceStatus(st).label}${n > 1 ? 's' : ''}`);
  const warn = state.world ? state.world.warnings.length + state.problems.length : 0;
  if (warn) parts.push(`${warn} avert.`);
  el.textContent = parts.length ? `(${parts.join(', ')})` : '';
}

function genealogyCoverage() {
  const ix = state.ix;
  const w = state.world;
  const lo = Number.isFinite(w.genealogy_start_year) ? fmtYear(w.genealogy_start_year) : null;
  if (ix.genealogyEnd === null) return 'aucune personne importée';
  const hi = fmtYear(ix.genealogyEnd);
  if (ix.genealogyEndEstimated) return `fin estimée vers ${hi} (dernière date enregistrée ; l’export ne fournit pas la couverture)`;
  return lo ? `de ${lo} à ${hi} (couverture exportée)` : `jusqu’en ${hi} (couverture exportée)`;
}

function renderSources() {
  const body = clear($('sources-body'));
  const w = state.world;
  if (w) {
    const m = w.map;
    const geo = mapGeometry(m);
    const b = m?.bounds;
    body.appendChild(h('section', { class: 'src-sec' }, h('h3', null, 'Monde chargé'),
      h('dl', { class: 'facts' },
        h('dt', null, 'Identité du canon'), h('dd', null, h('span', { class: 'fp' }, String(w.canon_id))),
        h('dt', null, 'Version du schéma'), h('dd', null, String(w.schema_version)),
        h('dt', null, 'Carte utilisée'), h('dd', null, MAP_MODE_LABEL[actualMapMode(w)]),
        h('dt', null, 'Scénario'), h('dd', null, w.config.scenario === undefined
          ? 'non indiqué par le serveur (scénario de l’étude)'
          : (w.config.scenario === 'free' ? 'histoire libre' : 'scénario de l’étude (1356, 1398)')),
        h('dt', null, 'Généalogie importée'), h('dd', null, genealogyCoverage()),
        h('dt', null, 'Graine'), h('dd', null, String(w.config.seed)),
        h('dt', null, 'Durée de génération'), h('dd', null, `${fmtMs(w.generation_ms)} (non canonique, hors empreinte)`))));
    if (m) {
      body.appendChild(h('section', { class: 'src-sec' }, h('h3', null, 'Carte'),
        h('dl', { class: 'facts' },
          h('dt', null, 'Source'), h('dd', null, String(m.source ?? '—')),
          h('dt', null, 'Grille'), h('dd', null, `${m.width} × ${m.height} cellules${geo.valid ? `, environ ${fmtNum(geo.cellWkm, 1)} × ${fmtNum(geo.cellHkm, 1)} km chacune` : ''}`),
          h('dt', null, 'Emprise'), h('dd', null, Array.isArray(b) && b.length === 4
            ? `longitudes ${fmtNum(b[0], 2)}° à ${fmtNum(b[2], 2)}°, latitudes ${fmtNum(b[1], 2)}° à ${fmtNum(b[3], 2)}°` : '—'),
          h('dt', null, 'Rayon planétaire de la carte'), h('dd', null, `${fmtNum(m.radius_km, 1)} km`),
          h('dt', null, 'Rayon demandé'), h('dd', null, `${fmtNum(w.config.world_radius_km, 1)} km`)),
        h('p', { class: 'hint' }, 'Le relief est calculé dans le navigateur à partir des altitudes numériques ; le lissage d’affichage n’ajoute aucune information. Cochez « Cellules brutes » pour voir la grille réelle. Avec une carte importée, le rayon demandé change les distances et durées calculées, pas la géométrie source.')));
    }
  }
  const sources = sourcesList();
  body.appendChild(h('section', { class: 'src-sec' }, h('h3', null, w ? 'Sources du monde chargé' : 'Sources disponibles sur le serveur'),
    sources.length ? h('ul', { class: 'src-list' }, sources.map((s) => {
      const st = sourceStatus(s.status);
      return h('li', null,
        h('p', { class: 'src-line' }, h('span', { class: `src-badge ${st.cls}` }, st.label), h('strong', null, s.label || s.id)),
        s.detail ? h('p', { class: 'src-detail' }, s.detail) : null,
        h('p', { class: 'src-meta' }, `ID ${s.id}`, s.fingerprint ? h('span', null, ', empreinte ', h('span', { class: 'fp', title: s.fingerprint }, shortId(s.fingerprint, 20))) : ', sans empreinte'));
    })) : h('p', { class: 'empty-line' }, 'Aucune source déclarée.')));

  const warns = w ? [...w.warnings, ...state.problems] : [];
  body.appendChild(h('section', { class: 'src-sec' }, h('h3', null, `Avertissements (${warns.length})`),
    warns.length ? h('ul', { class: 'warn-list' }, warns.map((t) => h('li', null, String(t)))) : h('p', { class: 'empty-line' }, 'Aucun avertissement.')));

  if (state.capabilities.length) {
    body.appendChild(h('section', { class: 'src-sec' }, h('h3', null, 'Capacités déclarées par le serveur'),
      h('ul', { class: 'cap-list' }, state.capabilities.map((c) => h('li', null, String(c))))));
  }
  body.appendChild(h('section', { class: 'src-sec' }, h('h3', null, 'Limites de ce prototype'),
    h('ul', { class: 'warn-list' },
      h('li', null, 'Le monde est généré en entier d’un coup, puis seulement lu : changer d’année, filtrer ou ouvrir une fiche ne relance aucun tirage. Le raffinement à la demande n’existe pas encore.'),
      h('li', null, 'Les réglages de guerre, d’épidémie, de commerce ou de migration sont des intensités de worldbuilding, pas des lois historiques.'),
      h('li', null, 'Les ressources sont des potentiels initiaux en unités arbitraires, pas des gisements relevés.'),
      h('li', null, 'Les relevés d’état sont pris au début de chaque année.'))));
}

// --- URL state (year and record only; never the configuration) ---------------------

function readHash() {
  const p = new URLSearchParams(location.hash.slice(1));
  const year = p.has('an') ? Number(p.get('an')) : null;
  const f = p.get('fiche');
  let selection = null;
  if (f && f.includes(':')) {
    const i = f.indexOf(':');
    selection = { type: f.slice(0, i), id: f.slice(i + 1) };
  }
  return { year: Number.isFinite(year) ? year : null, selection };
}

let hashTimer = 0;
function writeHash() {
  clearTimeout(hashTimer);
  hashTimer = setTimeout(() => {
    const p = new URLSearchParams();
    p.set('an', String(state.year));
    if (state.selection) p.set('fiche', `${state.selection.type}:${state.selection.id}`);
    history.replaceState(null, '', `#${p.toString()}`);
  }, 250);
}

function restoreHash() {
  if (!state.ix) return;
  clearTimeout(hashTimer);
  stopPlay();
  // Read once before rendering can schedule a replacement of the URL.
  const target = readHash();
  state.selection = target.selection && entityExists(target.selection) ? target.selection : null;
  state.back = [];
  state.forward = [];
  setYear(target.year ?? state.year, true);
  showTab(state.selection ? 'fiche' : 'chronicle', false);
  applySelection(false);
}

// --- Playback -------------------------------------------------------------------

function togglePlay() {
  if (state.playTimer) { stopPlay(); return; }
  if (!state.ix) return;
  if (state.year >= state.ix.maxYear) setYear(state.ix.minYear);
  $('play').setAttribute('aria-pressed', 'true');
  $('play').textContent = 'Pause';
  state.playTimer = setInterval(() => {
    if (state.year >= state.ix.maxYear) { stopPlay(); return; }
    setYear(state.year + 1);
  }, 280);
}

function stopPlay() {
  if (!state.playTimer) return;
  clearInterval(state.playTimer);
  state.playTimer = 0;
  $('play').setAttribute('aria-pressed', 'false');
  $('play').textContent = 'Lecture';
}

// --- Wiring -------------------------------------------------------------------

function bind() {
  window.addEventListener('hashchange', restoreHash);
  window.addEventListener('popstate', restoreHash);
  $('btn-generate').addEventListener('click', generate);
  $('cfg-form').addEventListener('submit', (ev) => { ev.preventDefault(); generate(); });
  $('btn-settings').addEventListener('click', () => setSettingsOpen(!document.body.classList.contains('settings-open')));
  $('btn-import').addEventListener('click', () => $('file-import').click());
  $('file-import').addEventListener('change', (ev) => {
    const file = ev.target.files?.[0];
    ev.target.value = '';
    importFile(file);
  });
  $('btn-export').addEventListener('click', exportWorld);
  $('btn-sources').addEventListener('click', () => { renderSources(); $('sources-dialog').showModal(); });
  $('sources-close').addEventListener('click', () => $('sources-dialog').close());
  $('sources-dialog').addEventListener('click', (ev) => { if (ev.target === ev.currentTarget) ev.currentTarget.close(); });
  $('error-close').addEventListener('click', clearError);
  $('map-empty-actions').addEventListener('click', (ev) => {
    const a = ev.target.closest('[data-action]')?.dataset.action;
    if (a === 'generate') generate();
    if (a === 'import') $('file-import').click();
  });

  // One delegated handler for every entity link.
  document.addEventListener('click', (ev) => {
    const el = ev.target.closest('[data-sel]');
    if (!el) return;
    const v = el.dataset.sel;
    const i = v.indexOf(':');
    select({ type: v.slice(0, i), id: v.slice(i + 1) });
  });

  // Tabs with arrow-key navigation.
  for (const t of TABS) $(`tab-${t}`).addEventListener('click', () => showTab(t, false));
  document.querySelector('.tabs').addEventListener('keydown', (ev) => {
    const i = TABS.indexOf(state.tab);
    let next = null;
    if (ev.key === 'ArrowRight') next = TABS[(i + 1) % TABS.length];
    else if (ev.key === 'ArrowLeft') next = TABS[(i + TABS.length - 1) % TABS.length];
    else if (ev.key === 'Home') next = TABS[0];
    else if (ev.key === 'End') next = TABS[TABS.length - 1];
    if (next) { ev.preventDefault(); showTab(next, true); }
  });

  $('fiche-back').addEventListener('click', goBack);
  $('fiche-fwd').addEventListener('click', goForward);
  $('fiche-close').addEventListener('click', closeFiche);

  // Year controls.
  $('year-range').addEventListener('input', (ev) => { stopPlay(); setYear(ev.target.value); });
  $('year-input').addEventListener('change', (ev) => { stopPlay(); setYear(ev.target.value); });
  $('year-input').addEventListener('keydown', (ev) => { if (ev.key === 'Enter') { stopPlay(); setYear(ev.target.value); } });
  $('year-dec').addEventListener('click', () => { stopPlay(); setYear(state.year - 1); });
  $('year-inc').addEventListener('click', () => { stopPlay(); setYear(state.year + 1); });
  $('prev-event').addEventListener('click', () => {
    stopPlay();
    const y = neighbourEventYear(state.filtered ?? [], state.year, -1);
    if (y === null) setStatus('Aucun événement filtré avant cette année.'); else setYear(y);
  });
  $('next-event').addEventListener('click', () => {
    stopPlay();
    const y = neighbourEventYear(state.filtered ?? [], state.year, 1);
    if (y === null) setStatus('Aucun événement filtré après cette année.'); else setYear(y);
  });
  $('play').addEventListener('click', togglePlay);

  // Map tools.
  $('zoom-in').addEventListener('click', () => map.zoomBy(1.5));
  $('zoom-out').addEventListener('click', () => map.zoomBy(1 / 1.5));
  $('zoom-reset').addEventListener('click', () => map.resetView());
  $('pixelated').addEventListener('change', (ev) => map.setPixelated(ev.target.checked));

  window.addEventListener('keydown', (ev) => {
    if (ev.key === 'Escape' && !$('sources-dialog').open && state.playTimer) stopPlay();
  });
}

function readPref(key) {
  try { return localStorage.getItem(key); } catch { return null; }
}

function writePref(key, value) {
  try { localStorage.setItem(key, value); } catch { /* storage unavailable: preference lasts this page only */ }
}

async function init() {
  bind();
  const wide = window.matchMedia('(min-width: 1280px)').matches;
  setSettingsOpen(wide);
  // The legend starts collapsed (it hides much of the map); a deliberate toggle is remembered.
  const legend = $('map-legend');
  legend.open = readPref('chronos.legend') === 'open';
  legend.addEventListener('toggle', () => writePref('chronos.legend', legend.open ? 'open' : 'closed'));
  renderFiche(false);
  chronicle.render();
  mapEmpty('loading', 'Chargement du monde…', 'Lecture des valeurs par défaut et du monde courant du serveur.');

  const [defaults, world] = await Promise.allSettled([api.defaults(), api.world()]);
  if (defaults.status === 'fulfilled') {
    const d = defaults.value.data;
    state.defaultConfig = d.config ?? null;
    state.serverSources = Array.isArray(d.sources) ? d.sources : [];
    state.capabilities = Array.isArray(d.capabilities) ? d.capabilities : [];
    form.setDefaults(d.config, !!d.config);
  } else {
    form.setDefaults(CONTRACT_DEFAULTS, false);
  }
  form.setValues(state.defaultConfig ?? CONTRACT_DEFAULTS);

  if (world.status === 'fulfilled') {
    loadWorld(world.value.data, world.value.text, 'Monde courant chargé :');
  } else {
    const msg = world.reason?.message ?? 'erreur inconnue';
    showError(`Monde courant indisponible : ${msg}`);
    mapEmpty('empty', 'Aucun monde chargé', 'Réglez les paramètres puis cliquez sur Générer, ou importez un monde exporté.');
    setSettingsOpen(true);
    setStatus('Aucun monde affiché.');
  }
  if (defaults.status === 'rejected' && world.status === 'fulfilled') {
    showError(`Valeurs par défaut indisponibles : ${defaults.reason?.message ?? 'erreur inconnue'}. Le formulaire reprend la configuration du monde chargé.`);
  }
  renderSourcesSummary();
}

init();
