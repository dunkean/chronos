// Expected versus realised balances (§15.4, §7.4). Gaps are shown as they are:
// no value is pulled toward its target and no uncertainty band is drawn, since
// the fixture computes none.

import { h, svg, clear } from './util.js';
import { t, tk, fmtInt, fmtNum, fmtSigned } from './i18n.js';
import { CONDUCT, CONDUCT_ORDER, conductLabel } from './history-model.js';

function snapshotFor(series, year) {
  let pick = series[0];
  for (const s of series) if (s.at_year <= year) pick = s;
  return pick;
}

export function createBalance(root, ctx) {
  const { model, view } = ctx;
  const box = h('div', { class: 's-bal' });
  root.append(box);
  const unitLabel = (u) => t(`unit.${u}`);

  function measuresTable(year) {
    const tb = h('tbody');
    for (const m of view.measures) {
      if (!m.series.length) continue;
      const snap = snapshotFor(m.series, year);
      const e = snap.expected, r = snap.realized;
      let gap = '—';
      let gapCls = '';
      if (r && e.value != null && r.value != null) {
        const d = r.value - e.value;
        gap = `${fmtSigned(d)}${e.value ? ` (${fmtSigned((d / e.value) * 100, 1)} %)` : ''}`;
        gapCls = d === 0 ? '' : Math.abs(d) >= Math.max(1, e.value * 0.05) ? 's-gap-big' : 's-gap';
      }
      const note = tk(m, 'note');
      tb.append(h('tr', null,
        h('th', { scope: 'row' }, tk(m, 'label'), h('span', { class: 's-dim s-block' }, `${t('balance.snapshot_of', { year: snap.at_year })}${note ? ` — ${note}` : ''}`)),
        h('td', { class: 's-num' }, `${fmtInt(e.value)} `, h('span', { class: 's-dim' }, unitLabel(e.unit))),
        h('td', { class: 's-num' }, r ? [`${fmtInt(r.value)} `, h('span', { class: 's-tag' }, t(`coverage.${r.coverage}`))] : h('span', { class: 's-none' }, t('balance.not_realized'))),
        h('td', { class: `s-num ${gapCls}` }, gap)));
    }
    return h('table', { class: 's-table' }, h('caption', null, t('balance.measures_caption', { year })),
      h('thead', null, h('tr', null, ['balance.col.measure', 'balance.col.expected', 'balance.col.realized', 'balance.col.gap'].map((k) => h('th', { scope: 'col' }, t(k))))), tb);
  }

  function niceStep(range) {
    const raw = range / 5;
    const pow = 10 ** Math.floor(Math.log10(raw || 1));
    return [1, 2, 5, 10].map((m) => m * pow).find((v) => v >= raw) ?? pow;
  }

  function popChart(year) {
    const m = view.measures.find((x) => x.id === 'm/population');
    const flows = view.balances.flows;
    if (!m || !flows.length) return h('p', { class: 's-dim' }, t('balance.no_site_series'));
    const W = 700, Ht = 190, L = 52, B = 24, T = 10;
    const from = view.period.from, to = view.period.to;
    const vals = [...m.series.map((s) => s.expected.value), ...m.series.map((s) => s.realized?.value), ...flows.map((f) => f.population)].filter((v) => v != null);
    const step = niceStep(Math.max(...vals) - Math.min(...vals) || 10);
    const lo = Math.floor(Math.min(...vals) / step) * step, hi = Math.ceil(Math.max(...vals) / step) * step || lo + step;
    const X = (y) => L + ((y - from) / Math.max(1, to - from)) * (W - L - 8);
    const Y = (v) => T + (1 - (v - lo) / (hi - lo)) * (Ht - T - B);
    const s = svg('svg', { viewBox: `0 0 ${W} ${Ht}`, class: 's-chart', role: 'img', 'aria-label': t('balance.chart_aria', { from, to }) });
    for (let v = lo; v <= hi; v += step) {
      s.append(svg('line', { x1: L, x2: W - 8, y1: Y(v), y2: Y(v), class: 's-tl-grid' }));
      const tick = svg('text', { x: L - 6, y: Y(v) + 3, 'text-anchor': 'end', class: 's-tl-tick' }); tick.textContent = String(v); s.append(tick);
    }
    for (let y = Math.ceil(from / 5) * 5; y <= to; y += 5) {
      const tick = svg('text', { x: X(y), y: Ht - 6, 'text-anchor': 'middle', class: 's-tl-tick' }); tick.textContent = String(y); s.append(tick);
    }
    s.append(svg('polyline', { points: m.series.map((p) => `${X(p.at_year)},${Y(p.expected.value)}`).join(' '), fill: 'none', stroke: '#2e5a6e', 'stroke-width': 2, 'stroke-dasharray': '6 4' }));
    for (const p of m.series) {
      s.append(svg('circle', { cx: X(p.at_year), cy: Y(p.expected.value), r: 3, fill: '#f3f7f6', stroke: '#2e5a6e', 'stroke-width': 1.5 }));
      if (p.realized) s.append(svg('circle', { cx: X(p.at_year), cy: Y(p.realized.value), r: 4.5, fill: '#ac5657' }));
    }
    s.append(svg('polyline', { points: flows.map((f) => `${X(f.year)},${Y(f.population)}`).join(' '), fill: 'none', stroke: '#ac5657', 'stroke-width': 2 }));
    s.append(svg('line', { x1: X(year), x2: X(year), y1: T, y2: Ht - B, class: 's-tl-cursor' }));
    return h('figure', { class: 's-fig' }, s,
      h('figcaption', null, h('span', { class: 's-key s-key-exp' }), ` ${t('balance.legend_expected')}   `,
        h('span', { class: 's-key s-key-real' }), ` ${t('balance.legend_realized', { from: flows[0].year, to: flows[flows.length - 1].year })}`));
  }

  function flowsTable(year) {
    const tb = h('tbody');
    for (const f of view.balances.flows.filter((x) => Math.abs(x.year - year) <= 10)) {
      const calc = f.population_previous + f.births - f.deaths + f.entries - f.exits;
      const ok = calc === f.population;
      tb.append(h('tr', { class: f.year === year ? 'is-sel' : '' },
        h('th', { scope: 'row' }, String(f.year)),
        h('td', { class: 's-num' }, fmtInt(f.population_previous)), h('td', { class: 's-num' }, '+' + fmtInt(f.births)),
        h('td', { class: 's-num' }, '−' + fmtInt(f.deaths)), h('td', { class: 's-num' }, '+' + fmtInt(f.entries)),
        h('td', { class: 's-num' }, '−' + fmtInt(f.exits)), h('td', { class: 's-num' }, fmtInt(f.population)),
        h('td', { class: ok ? 's-ok' : 's-gap-big' }, ok ? t('balance.closes') : t('balance.does_not_close', { gap: fmtSigned(f.population - calc) }))));
    }
    return h('table', { class: 's-table' }, h('caption', null, t('balance.flows_caption')),
      h('thead', null, h('tr', null, ['balance.col.year', 'balance.col.prev', 'balance.col.births', 'balance.col.deaths', 'balance.col.entries', 'balance.col.exits', 'balance.col.pop', 'balance.col.closure']
        .map((k) => h('th', { scope: 'col' }, t(k))))), tb);
  }

  // Realized classes at the nearest earlier marker date. No target is modelled for the
  // class mix (section 18.3 measures it), so none is drawn.
  function classesTable(year) {
    const rows = view.balances.conduct ?? [];
    if (!rows.length) return h('p', { class: 's-dim' }, t('balance.no_classes'));
    let row = rows[0];
    for (const r of rows) if (r.year <= year) row = r;
    const out = [];
    for (const key of ['person', 'organization']) {
      const isOrg = key === 'organization';
      const counts = row[key];
      const n = CONDUCT_ORDER.reduce((sum, c) => sum + (counts[c] ?? 0), 0);
      const tb = h('tbody');
      for (const c of CONDUCT_ORDER) {
        tb.append(h('tr', null,
          h('th', { scope: 'row' }, h('span', { class: 's-badge', style: { '--c': CONDUCT[c].color } }, `${CONDUCT[c].glyph} ${conductLabel(c, isOrg)}`)),
          h('td', { class: 's-num' }, fmtInt(counts[c] ?? 0)),
          h('td', { class: 's-num' }, n ? `${fmtNum(((counts[c] ?? 0) / n) * 100, 2)} %` : '—')));
      }
      out.push(h('table', { class: 's-table' }, h('caption', null, t(isOrg ? 'balance.classes_orgs' : 'balance.classes_people', { year: row.year, n })),
        h('thead', null, h('tr', null, ['balance.col.class', 'balance.col.realized', 'balance.col.share'].map((k) => h('th', { scope: 'col' }, t(k))))), tb));
    }
    return h('div', null, ...out, h('p', { class: 's-dim' }, t('balance.classes_note_real')));
  }

  function render() {
    const year = ctx.year();
    clear(box).append(
      h('div', { class: 's-bal-grid' },
        h('div', null, h('h3', null, t('balance.population')), popChart(year), measuresTable(year)),
        h('div', null, h('h3', null, t('balance.flows')), flowsTable(year), h('h3', null, t('balance.classes')), classesTable(year))));
  }
  return { render };
}
