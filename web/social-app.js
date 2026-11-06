// Social v2 page: loads a HistoryView from /api/v2 (or the fixture with ?fixture=1) and
// wires the views to one shared state. No rule is recomputed here, except the conduct
// class, a pure function of the axes (§27.2).

import {
  ApiError, DEFAULT_WINDOW, exportUrl, graphAround, isFixtureMode, loadHistoryView, openWorld, queryView,
  siteQuery, windowAround,
} from './data-source.js';
import { initI18n, t, tk, applyStatic, getLocale, setLocale, onLocaleChange, LOCALES } from './i18n.js';
import { Model } from './history-model.js';
import { QueryState } from './query-state.js';
import { createGraphView } from './social-graph.js';
import { createTimeline } from './social-timeline.js';
import { createBalance } from './population-balance.js';
import { createSheet } from './entity-sheet.js';
import { createMap } from './social-map.js';
import { h } from './util.js';

const $ = (id) => document.getElementById(id);
const DEFAULT_YEAR = 1360;

function describeError(err) {
  if (err instanceof ApiError && err.key) {
    const key = `diagnostics.${err.key}`;
    const text = t(key);
    return text === key ? err.key : text;
  }
  return err.message;
}

async function main() {
  try {
    await initI18n();
  } catch (err) {
    $('error').hidden = false;
    $('error-text').textContent = `i18n: ${err.message}`;
    return;
  }
  const fixture = isFixtureMode();
  // The banner exists only for the fixture; the real host shows the provenance line.
  $('fixture-label').hidden = !fixture;
  for (const el of document.querySelectorAll('[data-api-only]')) el.hidden = fixture;

  const progressBox = $('progress');
  const showError = (err) => {
    $('error').hidden = false;
    $('error-text').textContent = t('app.load_failed', { message: describeError(err) });
  };
  const track = (label) => (p) => {
    if (!p) return;
    progressBox.hidden = false;
    progressBox.textContent = t('progress.running', {
      what: label, stage: t(`progress.stage.${p.stage}`), done: p.done, total: p.total, seconds: Math.round((p.elapsed_ms ?? 0) / 1000),
    });
  };
  const idle = () => { progressBox.hidden = true; progressBox.textContent = ''; };

  let world = null;
  let view;
  let lastBody = null;
  let site = null;
  try {
    if (fixture) {
      view = await loadHistoryView();
    } else {
      progressBox.hidden = false;
      progressBox.textContent = t('progress.opening');
      world = await openWorld();
      const wanted = new URLSearchParams(location.hash.replace(/^#/, '')).get('site');
      site = world.sites.some((s) => s.site_id === wanted) ? wanted : world.default_scope.site_id;
      const yearFromUrl = Number.parseInt(new URLSearchParams(location.hash.replace(/^#/, '')).get('year') ?? '', 10);
      const year = Number.isFinite(yearFromUrl) ? yearFromUrl : DEFAULT_YEAR;
      lastBody = siteQuery(world, site, year);
      view = await queryView(world, lastBody, track(t('progress.what_view')));
      idle();
    }
  } catch (err) {
    idle();
    showError(err);
    return;
  }
  const model = new Model(view);
  const state = new QueryState({ year: view.at_year, site: site ?? '' });
  if (!fixture) state.value.site ||= site;
  const ctx = { view, model, state, world, fixture, selection: null, year: () => state.get().year ?? view.at_year };
  const timeline = () => view.timeline ?? view.period;

  // --- Year controls ---
  const yearInput = $('year'), yearRange = $('year-range');
  const clampYear = (v) => Math.min(timeline().to, Math.max(timeline().from, v));
  for (const el of [yearInput, yearRange]) { el.min = timeline().from; el.max = timeline().to; }
  yearInput.addEventListener('change', () => { const v = parseInt(yearInput.value, 10); state.set({ year: clampYear(Number.isFinite(v) ? v : view.at_year) }); });
  yearRange.addEventListener('input', () => state.set({ year: clampYear(parseInt(yearRange.value, 10)) }));
  $('year-prev').addEventListener('click', () => state.set({ year: clampYear(ctx.year() - 1) }));
  $('year-next').addEventListener('click', () => state.set({ year: clampYear(ctx.year() + 1) }));

  // --- Data loading that follows the year and the site ---
  let loadToken = 0;
  const needsReload = (year, siteId) => !fixture && (
    siteId !== view.scope.site_id || year < view.period.from || year > view.period.to || Math.abs(year - view.at_year) > DEFAULT_WINDOW / 2);
  async function ensureView(year, siteId) {
    if (!needsReload(year, siteId)) return false;
    const token = ++loadToken;
    try {
      const body = siteQuery(world, siteId, year);
      const next = await queryView(world, body, track(t('progress.what_view')));
      if (token !== loadToken) return false;
      lastBody = body;
      for (const k of Object.keys(view)) delete view[k];
      Object.assign(view, next);
      model.index();
      expanded.clear();
      idle();
      return true;
    } catch (err) {
      idle();
      showError(err);
      return false;
    }
  }

  // Expansion of the neighbourhood of the entity in focus (POST /graph), merged by ID.
  const expanded = new Set();
  async function expand(id) {
    if (fixture || !id || model.events.has(id)) return;
    const e = model.get(id);
    if (!e || e.kind === 'office') return;
    const year = ctx.year();
    const key = `${id}|${year}|${state.get().depth}`;
    if (expanded.has(key)) return;
    expanded.add(key);
    const isGroup = e.kind !== 'person' && e.kind !== 'polity';
    try {
      const frag = await graphAround(world, {
        center: id, at_year: year, period: windowAround(world, year), depth: state.get().depth >= 2 ? 2 : 1,
        include: isGroup ? ['people', 'events', 'overlay'] : ['people', 'events'],
        limits: { people: 80, relations: 400, groups: 40 },
      }, track(t('progress.what_graph')));
      idle();
      const added = model.merge(frag);
      const before = ctx.selection;
      ctx.selection = isGroup ? { id, year, data: frag.overlay?.selection ?? null } : (before?.id === id ? null : before);
      if (added || isGroup) renderAll();
    } catch (err) {
      expanded.delete(key);
      idle();
      showError(err);
    }
  }

  const mayorOf = (year) => model.tenureAt(view.default_selection.office_id, year)?.person_id ?? '';
  $('btn-mayor').addEventListener('click', async () => {
    const year = view.default_selection.year ?? DEFAULT_YEAR;
    await ensureView(year, state.get().site || site);
    const id = mayorOf(year) || view.default_selection.office_id;
    state.set({ year, sel: id, center: mayorOf(year), mode: 'social', classes: [], fam: [], depth: 1, strength: 0 });
  });
  $('btn-mayor-year').addEventListener('click', () => {
    const id = mayorOf(ctx.year());
    state.set({ sel: id || view.default_selection.office_id, center: id || state.get().center });
  });
  $('btn-1340').addEventListener('click', () => state.set({ year: 1340 }));
  $('btn-1360').addEventListener('click', () => state.set({ year: 1360 }));
  $('btn-reset').addEventListener('click', () => state.set({ sel: '', center: '', classes: [], fam: [], depth: 1, strength: 0, mode: 'social', year: DEFAULT_YEAR }));

  // --- Region: the site selector ---
  const siteSelect = $('site');
  if (world) {
    for (const s of world.sites) siteSelect.append(h('option', { value: s.site_id, selected: s.site_id === state.get().site }, s.label));
    siteSelect.addEventListener('change', () => state.set({ site: siteSelect.value, sel: '', center: '' }));
    $('export-canon').href = exportUrl(world, false);
    $('export-full').href = exportUrl(world, true);
  }

  // --- Load more (cursor) ---
  const more = $('load-more');
  async function loadMore() {
    const cursor = view.limits.cursor;
    if (!cursor || !lastBody) return;
    try {
      const page = await queryView(world, { ...lastBody, cursor }, track(t('progress.what_page')));
      idle();
      model.merge(page);
      view.limits.cursor = page.limits.cursor;
      view.limits.returned_nodes += page.limits.returned_nodes;
      view.limits.returned_edges += page.limits.returned_edges;
      view.limits.complete = !page.limits.cursor;
      renderAll();
    } catch (err) {
      idle();
      showError(err);
    }
  }
  more.addEventListener('click', loadMore);

  // Language selector (EN/FR), always visible.
  const lang = $('lang');
  for (const l of LOCALES) lang.append(h('option', { value: l, selected: l === getLocale() }, ''));
  lang.addEventListener('change', () => setLocale(lang.value));

  const graph = createGraphView($('graph'), ctx);
  const timelineView = createTimeline($('timeline'), ctx);
  const balance = createBalance($('balance'), ctx);
  const sheet = createSheet($('sheet'), ctx);
  const map = createMap($('overlay-map'), ctx);

  function renderAll() {
    const y = ctx.year();
    applyStatic();
    document.title = t('page.title');
    for (const opt of lang.options) opt.textContent = t(`locale.${opt.value}`);
    lang.value = getLocale();
    yearInput.value = y;
    yearRange.value = y;
    const name = view.scope.name ?? '';
    $('year-label').textContent = t('page.year_heading', { name, year: y });
    if (fixture) $('fixture-label').textContent = tk(view, 'fixture_label');
    const status = view.provenance.status;
    $('prov').replaceChildren(
      h('span', { class: `status st-${status}` }, t(`provenance.status.${status}`)),
      ` ${tk(view.provenance, 'detail')} ${t('page.canon')} : ${view.canon_id}. ${t('page.coverage')} : ${t(`coverage.${view.coverage}`)}.`
      + (view.view_fingerprint ? ` ${t('page.view_fingerprint')} : ${view.view_fingerprint.slice(0, 12)}.` : ''));
    more.hidden = !view.limits.cursor;
    more.textContent = t('page.load_more', { shown: view.limits.returned_nodes, total: view.limits.total_nodes ?? view.limits.returned_nodes });
    for (const [label, v] of [['graph', graph], ['timeline', timelineView], ['balance', balance], ['sheet', sheet], ['overlay-map', map]]) {
      try { v.render(); } catch (err) { console.error(label, err); $(label).append(h('p', { class: 's-warn' }, t('app.render_error', { view: label, message: err.message }))); }
    }
  }

  let last = { year: ctx.year(), site: state.get().site, sel: state.get().sel, center: state.get().center, depth: state.get().depth };
  state.subscribe(async () => {
    const st = state.get();
    const year = ctx.year();
    if (!fixture && (st.site || site) && (st.site || site) !== view.scope.site_id) {
      // A new site: the view is replaced before anything is drawn.
      await ensureView(year, st.site || site);
    }
    renderAll();
    if (!fixture) {
      const reloaded = await ensureView(year, st.site || site);
      if (reloaded) renderAll();
      if (st.center !== last.center || st.sel !== last.sel || year !== last.year || st.depth !== last.depth) {
        await expand(st.center || st.sel);
        if (st.sel && st.sel !== st.center) await expand(st.sel);
      }
    }
    last = { year, site: st.site, sel: st.sel, center: st.center, depth: st.depth };
  });
  onLocaleChange(renderAll);

  // First paint: if the URL says nothing, open the mayor route.
  const s0 = state.get();
  if (!s0.sel && !s0.center) {
    const id = mayorOf(ctx.year());
    state.set({ sel: id, center: id });
  } else {
    renderAll();
    if (!fixture) expand(s0.center || s0.sel);
  }
}

main();
