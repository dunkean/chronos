// Social v2 page (S5, first slice): loads a HistoryView and wires the views to
// one shared state. No rule is recomputed here, except the conduct class, a pure
// function of the axes (§27.2).

import { loadHistoryView } from './data-source.js';
import { initI18n, t, tk, applyStatic, getLocale, setLocale, onLocaleChange, LOCALES } from './i18n.js';
import { Model } from './history-model.js';
import { QueryState } from './query-state.js';
import { createGraphView } from './social-graph.js';
import { createTimeline } from './social-timeline.js';
import { createBalance } from './population-balance.js';
import { createSheet } from './entity-sheet.js';
import { h } from './util.js';

const $ = (id) => document.getElementById(id);

async function main() {
  try {
    await initI18n();
  } catch (err) {
    $('error').hidden = false;
    $('error-text').textContent = `i18n: ${err.message}`;
    return;
  }
  let view;
  try {
    view = await loadHistoryView();
  } catch (err) {
    $('error').hidden = false;
    $('error-text').textContent = t('app.load_failed', { message: err.message });
    return;
  }
  const model = new Model(view);
  const state = new QueryState({ year: view.at_year });
  const ctx = { view, model, state, year: () => state.get().year ?? view.at_year };

  const from = view.period.from, to = view.period.to;
  const yearInput = $('year'), yearRange = $('year-range');
  for (const el of [yearInput, yearRange]) { el.min = from; el.max = to; }
  const clampYear = (v) => Math.min(to, Math.max(from, v));
  yearInput.addEventListener('change', () => { const v = parseInt(yearInput.value, 10); state.set({ year: clampYear(Number.isFinite(v) ? v : view.at_year) }); });
  yearRange.addEventListener('input', () => state.set({ year: clampYear(parseInt(yearRange.value, 10)) }));
  $('year-prev').addEventListener('click', () => state.set({ year: clampYear(ctx.year() - 1) }));
  $('year-next').addEventListener('click', () => state.set({ year: clampYear(ctx.year() + 1) }));
  $('btn-1340').addEventListener('click', () => state.set({ year: 1340 }));
  $('btn-1360').addEventListener('click', () => state.set({ year: 1360 }));
  const mayorOf = (year) => model.tenureAt(view.default_selection.office_id, year)?.person_id ?? '';
  $('btn-mayor').addEventListener('click', () => {
    const year = view.default_selection.year;
    const id = mayorOf(year) || view.default_selection.office_id;
    state.set({ year, sel: id, center: mayorOf(year), mode: 'social', classes: [], fam: [], depth: 1, strength: 0 });
  });
  $('btn-mayor-year').addEventListener('click', () => {
    const id = mayorOf(ctx.year());
    state.set({ sel: id || view.default_selection.office_id, center: id || state.get().center });
  });
  $('btn-reset').addEventListener('click', () => state.set({ sel: '', center: '', classes: [], fam: [], depth: 1, strength: 0, mode: 'social', year: view.at_year }));

  // Language selector (EN/FR), always visible.
  const lang = $('lang');
  for (const l of LOCALES) lang.append(h('option', { value: l, selected: l === getLocale() }, ''));
  lang.addEventListener('change', () => setLocale(lang.value));

  const graph = createGraphView($('graph'), ctx);
  const timeline = createTimeline($('timeline'), ctx);
  const balance = createBalance($('balance'), ctx);
  const sheet = createSheet($('sheet'), ctx);

  const renderAll = () => {
    const y = ctx.year();
    applyStatic();
    document.title = t('page.title');
    for (const opt of lang.options) opt.textContent = t(`locale.${opt.value}`);
    lang.value = getLocale();
    yearInput.value = y;
    yearRange.value = y;
    $('year-label').textContent = t('page.year_heading', { name: view.scope.name, year: y });
    $('fixture-label').textContent = tk(view, 'fixture_label');
    const status = view.provenance.status;
    $('prov').replaceChildren(
      h('span', { class: `status st-${status}` }, t(`provenance.status.${status}`)),
      ` ${tk(view.provenance, 'detail')} ${t('page.canon')} : ${view.canon_id}. ${t('page.coverage')} : ${t(`coverage.${view.coverage}`)}.`);
    for (const [name, v] of [['graph', graph], ['timeline', timeline], ['balance', balance], ['sheet', sheet]]) {
      try { v.render(); } catch (err) { console.error(name, err); $(name).append(h('p', { class: 's-warn' }, t('app.render_error', { view: name, message: err.message }))); }
    }
  };
  state.subscribe(renderAll);
  onLocaleChange(renderAll);

  // First paint: if the URL says nothing, open the mayor route.
  const s0 = state.get();
  if (!s0.sel && !s0.center) {
    const id = mayorOf(ctx.year());
    state.set({ sel: id, center: id });
  } else {
    renderAll();
  }
}

main();
