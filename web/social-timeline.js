// Social timeline (§15.4): events, group existence, office tenures and the
// affiliations of the focused entity. A click on a mark selects it; a click on
// the background changes the year. An accessible table repeats the events.

import { h, svg, clear } from './util.js';
import { t, tk, tv } from './i18n.js';
import { EVENT_COLOR, eventKindLabel, predLabel } from './history-model.js';

const W = 1000, LEFT = 190, RIGHT = 14, ROW = 24;

export function createTimeline(root, ctx) {
  const { model, state, view } = ctx;
  const from = view.period.from, to = view.period.to;
  const x = (y) => LEFT + ((Math.min(Math.max(y, from), to) - from) / (to - from)) * (W - LEFT - RIGHT);
  const box = h('div', { class: 's-tl-box' });
  const table = h('details', { class: 's-tl-table' });
  root.append(box, table);

  const classKept = (id, y) => {
    const f = state.get().classes;
    if (!f.length) return true;
    const c = model.conduct(id, y);
    return !c || f.includes(c.cls);
  };
  const spanText = (ev) => (ev.end_year > ev.year ? `${ev.year}–${ev.end_year}` : String(ev.year));

  function render() {
    const st = state.get();
    const year = ctx.year();
    const rows = [];
    rows.push({ kind: 'head', label: t('timeline.events') });
    rows.push({ kind: 'events' });
    rows.push({ kind: 'head', label: t('timeline.groups') });
    for (const e of view.entities) {
      if (e.nature !== 'organized_group' || !classKept(e.id, year)) continue;
      rows.push({ kind: 'group', e });
    }
    rows.push({ kind: 'head', label: t('timeline.tenures') });
    for (const o of view.offices) rows.push({ kind: 'office', o });
    const focus = st.center || st.sel;
    const fe = model.get(focus);
    const aff = [];
    if (fe && fe.kind !== 'office') {
      for (const r of model.relations) {
        if (r.predicate !== 'member_of' && r.predicate !== 'affiliated_with') continue;
        if (fe.kind === 'person' && r.subject === focus) aff.push(r);
        else if (fe.kind !== 'person' && r.object === focus) aff.push(r);
      }
    }
    rows.push({ kind: 'head', label: fe ? t('timeline.affiliations_of', { name: model.name(focus) }) : t('timeline.affiliations') });
    if (!aff.length) rows.push({ kind: 'empty', label: t('timeline.no_affiliation') });
    for (const r of aff) rows.push({ kind: 'aff', r, focusPerson: fe.kind === 'person' });

    let yPos = 8;
    for (const row of rows) {
      row.y = yPos;
      yPos += row.kind === 'head' ? 22 : row.kind === 'events' ? 56 : ROW;
    }
    const H = yPos + 24;
    const s = svg('svg', { viewBox: `0 0 ${W} ${H}`, class: 's-tl', role: 'group', 'aria-label': t('timeline.aria') });
    for (let y = Math.ceil(from / 5) * 5; y <= to; y += 5) {
      s.append(svg('line', { x1: x(y), x2: x(y), y1: 0, y2: H - 18, class: 's-tl-grid' }));
      const tick = svg('text', { x: x(y), y: H - 5, class: 's-tl-tick', 'text-anchor': 'middle' });
      tick.textContent = String(y);
      s.append(tick);
    }
    const label = (txt, ypos, cls = 's-tl-label') => {
      const el = svg('text', { x: 6, y: ypos, class: cls });
      el.textContent = txt;
      return el;
    };
    const bar = (a, b, ypos, fill, opts = {}) => {
      const r = svg('rect', { x: x(a), y: ypos, width: Math.max(2, x(b) - x(a)), height: ROW - 10, rx: 3, fill, class: opts.cls ?? 's-tl-bar' });
      if (opts.title) { const ti = svg('title'); ti.textContent = opts.title; r.append(ti); }
      return r;
    };
    const barText = (a, b, ypos, txt) => {
      if (x(b) - x(a) < 40) return null;
      const el = svg('text', { x: x(a) + 4, y: ypos + ROW - 14, class: 's-tl-bartext' });
      el.textContent = txt;
      return el;
    };
    const clickable = (g, go) => {
      g.addEventListener('click', (ev) => { ev.stopPropagation(); go(); });
      g.addEventListener('keydown', (ev) => { if (ev.key === 'Enter' || ev.key === ' ') { ev.preventDefault(); go(); } });
      return g;
    };
    s.append(svg('defs', null, svg('pattern', { id: 'hatch', width: 6, height: 6, patternUnits: 'userSpaceOnUse', patternTransform: 'rotate(45)' },
      svg('rect', { width: 6, height: 6, fill: '#f3f7f6' }), svg('line', { x1: 0, y1: 0, x2: 0, y2: 6, stroke: '#8a9ba3', 'stroke-width': 2 }))));

    for (const row of rows) {
      const yy = row.y;
      if (row.kind === 'head') { s.append(label(row.label, yy + 14, 's-tl-head')); continue; }
      if (row.kind === 'empty') { s.append(label(row.label, yy + 14, 's-tl-dim')); continue; }
      if (row.kind === 'events') {
        s.append(svg('line', { x1: LEFT, x2: W - RIGHT, y1: yy + 38, y2: yy + 38, class: 's-tl-axis' }));
        let k = 0;
        for (const ev of view.events) {
          const col = EVENT_COLOR[ev.kind] ?? '#6b7d86';
          const cx = x(ev.year);
          const lane = (k++ % 2) * 14;
          const g = svg('g', { class: 's-tl-ev' + (st.sel === ev.id ? ' is-sel' : ''), tabindex: 0, role: 'button', 'aria-label': `${ev.year}, ${tk(ev, 'label')}` });
          if (ev.end_year > ev.year) g.append(svg('rect', { x: cx, y: yy + 33, width: Math.max(3, x(ev.end_year + 1) - cx), height: 10, fill: col, opacity: 0.45, rx: 3 }));
          g.append(svg('line', { x1: cx, x2: cx, y1: yy + 38 - lane - 8, y2: yy + 38, stroke: col, 'stroke-width': 1 }));
          g.append(svg('circle', { cx, cy: yy + 38 - lane - 8, r: 5.5, fill: col }));
          const ti = svg('title'); ti.textContent = `${ev.year} — ${tk(ev, 'label')}`; g.append(ti);
          s.append(clickable(g, () => state.set({ sel: ev.id })));
        }
        continue;
      }
      if (row.kind === 'group') {
        const e = row.e;
        if (st.sel === e.id || st.center === e.id) s.append(svg('rect', { x: 0, y: yy - 2, width: W, height: ROW, class: 's-tl-focus' }));
        s.append(label(model.name(e.id), yy + 12));
        const end = e.existence[1] ?? to;
        for (const a of e.alignment_episodes) {
          const a0 = Math.max(a.valid[0], e.existence[0], from), a1 = Math.min(a.valid[1] ?? to, end, to);
          if (a1 <= a0) continue;
          const c = model.conduct(e.id, a0);
          const g = svg('g', { class: 's-tl-click', tabindex: 0, role: 'button', 'aria-label': `${model.name(e.id)}, ${c.label}, ${a0}–${a1}` });
          g.append(bar(a0, a1, yy, c.color, { title: `${model.name(e.id)} · ${c.label} · ${a0}–${a1}` }));
          const bt = barText(a0, a1, yy, `${c.glyph} ${c.label}`);
          if (bt) { bt.setAttribute('fill', '#fff'); g.append(bt); }
          s.append(clickable(g, () => state.set({ sel: e.id })));
        }
        continue;
      }
      if (row.kind === 'office') {
        const o = row.o;
        s.append(label(model.name(o.id), yy + 12));
        for (const tn of view.tenures.filter((x2) => x2.office_id === o.id)) {
          const a0 = Math.max(tn.valid[0], from), a1 = Math.min(tn.valid[1] ?? to, to);
          if (a1 <= a0) continue;
          const vacant = tn.mode === 'vacant';
          const nm = vacant ? t('term.vacant') : model.name(tn.person_id);
          const g = svg('g', { class: 's-tl-click', tabindex: 0, role: 'button', 'aria-label': `${model.name(o.id)}, ${nm}, ${tn.valid[0]}–${tn.valid[1] ?? t('term.ongoing')}` });
          g.append(bar(a0, a1, yy, vacant ? 'url(#hatch)' : '#47708a', { cls: vacant ? 's-tl-bar s-tl-vacant' : 's-tl-bar', title: `${model.name(o.id)} · ${nm} · ${tn.valid[0]}–${tn.valid[1] ?? '…'}` }));
          const bt = barText(a0, a1, yy, nm);
          if (bt) { if (!vacant) bt.setAttribute('fill', '#fff'); g.append(bt); }
          s.append(clickable(g, () => state.set({ sel: tn.person_id ?? tn.cause_event_id ?? o.id })));
        }
        continue;
      }
      if (row.kind === 'aff') {
        const r = row.r;
        const other = row.focusPerson ? r.object : r.subject;
        const sentence = `${model.name(r.subject)} ${predLabel(r.predicate)} ${model.name(r.object)}`;
        s.append(label(model.name(other), yy + 12));
        const a0 = Math.max(r.valid[0], from), a1 = Math.min(r.valid[1] ?? to, to);
        const g = svg('g', { class: 's-tl-click', tabindex: 0, role: 'button', 'aria-label': sentence });
        g.append(bar(a0, a1, yy, '#4f8a8b', { title: `${sentence} · ${r.valid[0]}–${r.valid[1] ?? '…'}` }));
        const bt = barText(a0, a1, yy, row.focusPerson ? t('timeline.member') : model.name(r.subject));
        if (bt) { bt.setAttribute('fill', '#fff'); g.append(bt); }
        if (r.end_cause_event_id && r.valid[1]) g.append(svg('path', { d: `M${x(r.valid[1])},${yy - 1} l-4,-5 h8 z`, fill: '#8a3c41' }));
        s.append(clickable(g, () => state.set({ sel: other })));
      }
    }
    s.append(svg('line', { x1: x(year), x2: x(year), y1: 0, y2: H - 18, class: 's-tl-cursor' }));
    const yl = svg('text', { x: x(year), y: 10, class: 's-tl-cursor-label', 'text-anchor': 'middle' });
    yl.textContent = String(year);
    s.append(yl);
    s.addEventListener('click', (e) => {
      const r = s.getBoundingClientRect();
      const px = ((e.clientX - r.left) / r.width) * W;
      if (px < LEFT) return;
      const y = Math.round(from + ((px - LEFT) / (W - LEFT - RIGHT)) * (to - from));
      state.set({ year: Math.min(to, Math.max(from, y)) });
    });
    clear(box).append(s);

    clear(table);
    table.append(h('summary', null, t('timeline.table_summary', { n: view.events.length })));
    const tb = h('tbody');
    for (const ev of view.events) {
      tb.append(h('tr', { class: ev.id === st.sel ? 'is-sel' : '' },
        h('td', null, spanText(ev)), h('td', null, eventKindLabel(ev.kind)),
        h('td', null, h('button', { type: 'button', class: 's-link', onclick: () => state.set({ sel: ev.id }) }, tk(ev, 'label'))),
        h('td', null, ev.transition ? `${tv(ev.transition.before)} → ${tv(ev.transition.after)}` : '—')));
    }
    table.append(h('table', { class: 's-table' }, h('thead', null, h('tr', null,
      ['timeline.col.year', 'timeline.col.type', 'timeline.col.event', 'timeline.col.before_after'].map((k) => h('th', { scope: 'col' }, t(k))))), tb));
  }
  return { render };
}
