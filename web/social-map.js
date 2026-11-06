// Historical overlay on the atlas of the host (§15.2): sites, routes valid in the year,
// one influence or legality channel, the places of the selected group, and the external
// contacts. Every position comes from the atlas; a context outside the map is named and
// listed, never given a position.

import { h, svg, clear } from './util.js';
import { t, fmtInt } from './i18n.js';

const W = 1000;
const CHANNEL_COLOR = { legality: '#8a2b33', recruitment: '#2e5a6e', economy: '#9a6a3a', culture: '#6d4f7a', religion: '#9a7410', military: '#7a1f2b', memory: '#4f8a8b' };

const siteName = (overlay, id) => overlay.sites.find((s) => s.site_id === id)?.label ?? id;

export function createMap(root, ctx) {
  const { state } = ctx;
  let channel = 'legality';
  let large = false;
  const controls = h('div', { class: 's-map-controls' });
  const figure = h('div', { class: 's-map-fig' });
  const lists = h('div', { class: 's-map-lists' });
  root.append(controls, figure, lists);

  function render() {
    const overlay = ctx.view.overlay;
    clear(controls); clear(figure); clear(lists);
    figure.classList.toggle('is-large', large);
    if (!overlay || !overlay.bounds) {
      figure.append(h('p', { class: 's-dim' }, ctx.fixture ? t('map.fixture_none') : t('map.none')));
      return;
    }
    const year = overlay.year;
    const names = overlay.channel_names;
    if (!names.includes(channel)) channel = names[0];
    controls.append(
      h('label', null, `${t('map.channel')} `, h('select', { id: 'map-channel', onchange: (e) => { channel = e.target.value; render(); } },
        names.map((n) => h('option', { value: n, selected: n === channel }, t(`channel.${n}`))))),
      h('button', { type: 'button', class: 'btn btn-quiet', id: 'map-enlarge', 'aria-pressed': String(large), onclick: () => { large = !large; render(); } }, t(large ? 'map.shrink' : 'map.enlarge')),
      h('span', { class: 's-dim' }, ` ${t('map.year', { year })} · ${t('map.frame', { frame: overlay.coordinate_frame })} · `),
      h('span', { class: `status st-${overlay.provenance.status}` }, t(`provenance.status.${overlay.provenance.status}`)));

    const b = overlay.bounds;
    const width = b.max_x_mm - b.min_x_mm, height = b.max_y_mm - b.min_y_mm;
    const H = Math.round((W * height) / width);
    const pad = 40;
    const X = (x) => pad + ((x - b.min_x_mm) / width) * (W - 2 * pad);
    const Y = (y) => pad + ((b.max_y_mm - y) / height) * (H - 2 * pad);
    const bySite = new Map(overlay.sites.map((s) => [s.site_id, s]));
    const values = overlay.channels[channel] ?? {};
    const maxValue = Math.max(1, ...Object.values(values).map((v) => v.value ?? 0));
    const color = CHANNEL_COLOR[channel] ?? '#2e5a6e';
    const sel = ctx.selection?.data ?? null;
    const focusSite = state.get().site || ctx.view.scope.site_id;

    const s = svg('svg', { viewBox: `0 0 ${W} ${H + 24}`, class: 's-map', role: 'img', 'aria-label': t('map.aria', { year }) });
    s.append(svg('rect', { x: pad / 2, y: pad / 2, width: W - pad, height: H - pad, class: 's-map-frame' }));
    for (const r of overlay.routes) {
      const a = bySite.get(r.from), c = bySite.get(r.to);
      if (!a || !c) continue;
      const line = svg('line', { x1: X(a.x_mm), y1: Y(a.y_mm), x2: X(c.x_mm), y2: Y(c.y_mm), class: `s-map-route s-map-route-${r.mode}` });
      const ti = svg('title'); ti.textContent = t('map.route_title', { from: a.label, to: c.label, days: r.travel_days }); line.append(ti);
      s.append(line);
    }
    // External contacts: a dashed ring at the attached site, no invented outside position.
    for (const c of overlay.external_contacts) {
      for (const end of c.site_ends) {
        const site = bySite.get(end.site_id);
        if (!site) continue;
        const ring = svg('circle', { cx: X(site.x_mm), cy: Y(site.y_mm), r: 22, class: 's-map-contact' });
        const ti = svg('title'); ti.textContent = t('map.contact_title', { site: site.label, context: end.external_context_id, population: fmtInt(end.population_expected) });
        ring.append(ti);
        s.append(ring);
      }
    }
    for (const site of overlay.sites) {
      const cx = X(site.x_mm), cy = Y(site.y_mm);
      const v = values[site.site_id];
      const level = v ? Math.min(1, (v.value ?? 0) / maxValue) : 0;
      const radius = 7 + Math.sqrt(site.population ?? site.population_expected_max ?? 0) / 6;
      const g = svg('g', { class: 's-map-site' + (site.site_id === focusSite ? ' is-focus' : ''), tabindex: 0, role: 'button', 'aria-label': site.label });
      g.append(svg('circle', {
        cx, cy, r: radius, class: 's-map-dot' + (site.materialized ? '' : ' is-unmaterialized'),
        fill: site.materialized ? color : 'none', 'fill-opacity': site.materialized ? 0.12 + 0.75 * level : 0,
      }));
      const residents = sel?.residents_by_site?.[site.site_id];
      if (residents) g.append(svg('circle', { cx, cy, r: radius + 5, class: 's-map-resident' }));
      if (sel?.seat_site_id === site.site_id) g.append(svg('path', { d: `M${cx},${cy - radius - 12} l4,9 l9,1 l-7,6 l2,9 l-8,-5 l-8,5 l2,-9 l-7,-6 l9,-1 z`, class: 's-map-seat' }));
      if (sel?.other_chapters?.includes(site.site_id)) g.append(svg('rect', { x: cx - 5, y: cy - 5, width: 10, height: 10, class: 's-map-chapter', transform: `rotate(45 ${cx} ${cy})` }));
      const label = svg('text', { x: cx, y: cy + radius + 13, class: 's-map-label', 'text-anchor': 'middle' });
      label.textContent = site.label;
      g.append(label);
      const ti = svg('title'); ti.textContent = siteSentence(site, v, residents); g.append(ti);
      const go = () => state.set({ site: site.site_id, sel: '', center: '' });
      g.addEventListener('click', go);
      g.addEventListener('keydown', (ev) => { if (ev.key === 'Enter' || ev.key === ' ') { ev.preventDefault(); go(); } });
      s.append(g);
    }
    figure.append(s, legend(sel));
    lists.append(siteTable(overlay, values, sel), contactList(overlay), selectionList(overlay, sel));
  }

  function siteSentence(site, v, residents) {
    const parts = [site.label];
    if (!site.materialized) parts.push(t('map.not_materialized'));
    if (site.population != null) parts.push(t('map.population', { n: fmtInt(site.population) }));
    if (v) parts.push(channel === 'legality' ? t('map.legality_counts', { licit: v.licit, tolerated: v.tolerated, proscribed: v.proscribed }) : t('map.channel_value', { value: fmtInt(v.value) }));
    if (residents) parts.push(t('map.residents', { n: residents }));
    return parts.join(' · ');
  }

  function legend(sel) {
    return h('ul', { class: 's-map-legend', 'aria-label': t('map.legend') },
      h('li', null, h('span', { class: 's-map-key s-map-key-dot' }), ` ${t('map.legend.site')}`),
      h('li', null, h('span', { class: 's-map-key s-map-key-route' }), ` ${t('map.legend.route')}`),
      h('li', null, h('span', { class: 's-map-key s-map-key-contact' }), ` ${t('map.legend.contact')}`),
      sel ? h('li', null, h('span', { class: 's-map-key s-map-key-resident' }), ` ${t('map.legend.resident')}`) : null,
      sel ? h('li', null, h('span', { class: 's-map-key s-map-key-chapter' }), ` ${t('map.legend.chapter')}`) : null);
  }

  function siteTable(overlay, values, sel) {
    const tb = h('tbody');
    for (const site of overlay.sites) {
      const v = values[site.site_id];
      tb.append(h('tr', { class: site.site_id === (state.get().site || ctx.view.scope.site_id) ? 'is-sel' : '' },
        h('th', { scope: 'row' }, h('button', { type: 'button', class: 's-link', onclick: () => state.set({ site: site.site_id, sel: '', center: '' }) }, site.label)),
        h('td', { class: 's-num' }, site.population != null ? fmtInt(site.population) : '—'),
        h('td', null, site.materialized ? t('map.materialized') : t('map.not_materialized')),
        h('td', { class: 's-num' }, v ? (channel === 'legality' ? `${v.licit} / ${v.tolerated} / ${v.proscribed}` : fmtInt(v.value)) : '—'),
        h('td', { class: 's-num' }, sel?.residents_by_site?.[site.site_id] ?? '—')));
    }
    return h('table', { class: 's-table' }, h('caption', null, t('map.table_caption', { channel: t(`channel.${channel}`) })),
      h('thead', null, h('tr', null, ['map.col.site', 'map.col.population', 'map.col.state', channel === 'legality' ? 'map.col.legality' : 'map.col.value', 'map.col.residents'].map((k) => h('th', { scope: 'col' }, t(k))))), tb);
  }

  function contactList(overlay) {
    const ul = h('ul', { class: 's-rels' });
    for (const c of overlay.external_contacts) {
      for (const end of c.site_ends) {
        ul.append(h('li', null, t('map.contact_line', {
          site: siteName(overlay, end.site_id), context: end.external_context_id, population: fmtInt(end.population_expected) })));
      }
    }
    return h('div', null, h('h3', null, t('map.contacts')), overlay.external_contacts.length ? ul : h('p', { class: 's-dim' }, t('map.no_contacts')),
      h('p', { class: 's-dim' }, t('map.no_invented_position')));
  }

  function selectionList(overlay, sel) {
    if (!sel) return h('p', { class: 's-dim' }, t('map.pick_group'));
    const rows = Object.entries(sel.residents_by_site).sort((a, b) => b[1] - a[1]);
    const influence = sel.influence[channel] ?? {};
    return h('div', null, h('h3', null, t('map.selection', { name: ctx.model.name(sel.group_id) })),
      h('p', null, t('map.seat', { site: siteName(overlay, sel.seat_site_id) })),
      rows.length ? h('ul', { class: 's-rels' }, rows.map(([site, n]) => h('li', null, t('map.residents_at', { site: siteName(overlay, site), n })))) : h('p', { class: 's-dim' }, t('map.no_residents')),
      h('p', { class: 's-dim' }, t('map.chapters', { n: sel.other_chapters.length })),
      Object.keys(influence).length ? h('p', { class: 's-dim' }, t('map.influence_sites', { channel: t(`channel.${channel}`), n: Object.keys(influence).length })) : null);
  }

  return { render };
}
