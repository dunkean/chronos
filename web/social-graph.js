// Social-centred graph and kinship graph (§15.3), with Cytoscape.js bundled in
// web/vendor (global `cytoscape`), plus an equivalent accessible list.
// The drawing is never the only way to read a relation.

import { h, clear, foldText } from './util.js';
import { t, compareText } from './i18n.js';
import {
  CONDUCT, CONDUCT_ORDER, FAMILIES, kindLabel, predLabel, predFamily, familyLabel, conductLabel,
} from './history-model.js';
import { toggleIn } from './query-state.js';

const KIN = new Set(['parent_of', 'union_with', 'adopted_by']);
const FAMILY_COLOR = {
  structure: '#8a9ba3', family: '#47708a', affinity: '#3a6a4b', activity: '#8b7766',
  power: '#2e5a6e', economy: '#b8893f', history: '#ac5657',
};

// Bounded subgraph around the centre at `year`, following the shared state.
export function buildSubgraph(model, st, year, limits) {
  const center = st.center || st.sel;
  const out = { center, nodes: new Map(), edges: [], hiddenNodes: 0, hiddenEdges: 0, note: '' };
  if (!center || !model.get(center)) { out.note = t('graph.note.pick_center'); return out; }
  const kin = st.mode === 'kinship';
  const centerEntity = model.get(center);
  if (!kin && !model.exists(center, year)) {
    out.note = t('graph.note.not_existing', { name: model.name(center), year, from: centerEntity.existence[0], to: centerEntity.existence[1] ?? '…' });
    return out;
  }
  if (kin && centerEntity.kind !== 'person') { out.note = t('graph.note.kinship_needs_person'); return out; }
  const maxDepth = kin ? st.depth + 1 : st.depth;
  const familyOk = (p) => (kin ? KIN.has(p) : (!st.fam.length || st.fam.includes(predFamily(p))));
  const strengthOk = (r) => kin || (r.weight_bps ?? 10000) / 100 >= st.strength;
  const classOk = (id) => {
    if (!st.classes.length || id === center) return true;
    const c = model.conduct(id, year);
    return !c || st.classes.includes(c.cls);
  };
  // Kinship: deceased ancestors stay in the tree, and unions and filiations are kept after death.
  const present = (id) => {
    const e = model.get(id);
    return kin && e.kind === 'person' ? year >= e.existence[0] : model.exists(id, year);
  };
  const relsOf = (id) => (kin ? (model.adj.get(id) ?? []).filter((r) => r.valid[0] <= year) : model.relationsAt(id, year));
  const dist = new Map([[center, 0]]);
  const edges = new Map();
  let frontier = [center];
  for (let d = 1; d <= maxDepth; d++) {
    const next = [];
    for (const id of frontier) {
      for (const r of relsOf(id)) {
        if (!familyOk(r.predicate) || !strengthOk(r)) continue;
        const other = r.subject === id ? r.object : r.subject;
        if (!model.get(other) || !present(other) || !classOk(other)) continue;
        edges.set(r.id, r);
        if (!dist.has(other)) { dist.set(other, d); next.push(other); }
      }
    }
    frontier = next;
  }
  // Cross links between retained nodes, same filters.
  for (const id of dist.keys()) {
    for (const r of relsOf(id)) {
      if (familyOk(r.predicate) && strengthOk(r) && dist.has(r.subject) && dist.has(r.object)) edges.set(r.id, r);
    }
  }
  let ids = [...dist.keys()];
  if (ids.length > limits.max_nodes) { out.hiddenNodes = ids.length - limits.max_nodes; ids = ids.slice(0, limits.max_nodes); }
  const keep = new Set(ids);
  for (const id of ids) out.nodes.set(id, dist.get(id));
  let list = [...edges.values()].filter((r) => keep.has(r.subject) && keep.has(r.object));
  if (list.length > limits.max_edges) { out.hiddenEdges = list.length - limits.max_edges; list = list.slice(0, limits.max_edges); }
  out.edges = list;
  return out;
}

function nodeData(model, id, year, center, dist) {
  const e = model.get(id);
  const c = model.conduct(id, year);
  const shape = e.kind === 'person' ? 'ellipse' : e.kind === 'office' ? 'diamond'
    : (e.kind === 'village' || e.kind === 'jurisdiction') ? 'hexagon' : 'round-rectangle';
  return { id, label: model.name(id), shape, color: c ? c.color : '#9aa8ad', cls: c ? c.cls : 'none', size: id === center ? 46 : 32, dist, synthetic: model.isSynthetic(id) ? 1 : 0 };
}

export function createGraphView(root, ctx) {
  const { model, state, view } = ctx;
  const cyBox = h('div', { class: 's-cy', role: 'img' });
  const note = h('p', { class: 's-graph-note', role: 'status' });
  const counter = h('p', { class: 's-graph-count', role: 'status' });
  const filters = h('div', { class: 's-filters' });
  const list = h('div', { class: 's-glist' });
  const search = h('input', { type: 'search', id: 'glist-search' });
  const searchLabel = h('label', { class: 'sr-only', for: 'glist-search' });
  const listTitle = h('h3');
  let query = '';
  search.addEventListener('input', () => { query = foldText(search.value); renderList(); });
  root.append(filters, note, h('div', { class: 's-graph-grid' }, cyBox, h('div', { class: 's-glist-wrap' }, listTitle, searchLabel, search, list)), counter);

  let cy = null;
  let last = null;
  if (typeof cytoscape === 'function') {
    cy = cytoscape({
      container: cyBox, elements: [], minZoom: 0.3, maxZoom: 2.5,
      style: [
        { selector: 'node', style: {
          'background-color': 'data(color)', shape: 'data(shape)', width: 'data(size)', height: 'data(size)',
          label: 'data(label)', 'font-size': 11, 'font-family': 'Bahnschrift, Segoe UI, sans-serif', color: '#1b3440',
          'text-valign': 'bottom', 'text-margin-y': 4, 'text-wrap': 'wrap', 'text-max-width': 90,
          'border-width': 2, 'border-color': '#f3f7f6', 'text-background-color': '#f3f7f6', 'text-background-opacity': 0.85, 'text-background-padding': 1 } },
        { selector: 'node[synthetic = 1]', style: { 'border-style': 'dashed' } },
        { selector: 'node[cls = "lawless"]', style: { 'border-color': '#2a0a10', 'border-width': 3 } },
        { selector: 'node[cls = "saintly"]', style: { 'border-color': '#d6b13a', 'border-width': 4 } },
        { selector: 'node.center', style: { 'border-color': '#1b3440', 'border-width': 4 } },
        { selector: 'node.sel', style: { 'overlay-color': '#1b3440', 'overlay-opacity': 0.18, 'overlay-padding': 7 } },
        { selector: 'edge', style: {
          width: 1.6, 'line-color': 'data(color)', 'target-arrow-color': 'data(color)', 'target-arrow-shape': 'triangle',
          'curve-style': 'bezier', label: 'data(label)', 'font-size': 9, color: '#46606b', 'text-rotation': 'autorotate',
          'text-background-color': '#f3f7f6', 'text-background-opacity': 0.9, 'text-background-padding': 1, 'arrow-scale': 0.9 } },
        { selector: 'edge[kind = "union"]', style: { 'target-arrow-shape': 'none', 'line-style': 'dashed' } },
      ],
    });
    cy.on('tap', 'node', (ev) => state.set({ sel: ev.target.id() }));
    cy.on('dbltap', 'node', (ev) => state.set({ center: ev.target.id(), sel: ev.target.id() }));
  }

  function renderFilters() {
    const st = state.get();
    clear(filters);
    const seg = (label, value) => h('button', { type: 'button', class: 's-seg', 'aria-pressed': String(st.mode === value), onclick: () => state.set({ mode: value }) }, label);
    filters.append(
      h('div', { class: 's-fgroup', role: 'group', 'aria-label': t('graph.mode') }, seg(t('graph.mode.social'), 'social'), seg(t('graph.mode.kinship'), 'kinship')),
      h('label', { class: 's-fgroup' }, `${t('graph.depth')} `,
        h('select', { onchange: (e) => state.set({ depth: Number(e.target.value) }) },
          [1, 2, 3].map((n) => h('option', { value: n, selected: st.depth === n }, String(n))))),
      h('label', { class: 's-fgroup' }, `${t('graph.min_strength', { value: st.strength })} `,
        h('input', { type: 'range', min: 0, max: 100, step: 10, value: st.strength, disabled: st.mode === 'kinship',
          'aria-label': t('graph.min_strength_aria'), onchange: (e) => state.set({ strength: Number(e.target.value) }) })),
      h('div', { class: 's-fgroup', role: 'group', 'aria-label': t('graph.conduct_classes') }, h('span', { class: 's-flabel' }, t('graph.conduct')),
        CONDUCT_ORDER.map((c) => h('button', { type: 'button', class: 's-chip', 'aria-pressed': String(st.classes.includes(c)),
          style: { '--chip': CONDUCT[c].color }, onclick: () => state.set({ classes: toggleIn(st.classes, c) }) },
          `${CONDUCT[c].glyph} ${conductLabel(c, false)} / ${conductLabel(c, true)}`))),
      h('div', { class: 's-fgroup', role: 'group', 'aria-label': t('graph.link_families') }, h('span', { class: 's-flabel' }, t('graph.links')),
        FAMILIES.map((f) => h('button', { type: 'button', class: 's-chip s-chip-plain', 'aria-pressed': String(st.fam.includes(f)),
          disabled: st.mode === 'kinship', onclick: () => state.set({ fam: toggleIn(st.fam, f) }) }, familyLabel(f)))),
    );
  }

  function renderList() {
    clear(list);
    if (!last) return;
    const year = ctx.year();
    const st = state.get();
    const items = [...last.nodes.keys()].map((id) => model.get(id))
      .filter((e) => !query || foldText(`${model.name(e.id)} ${kindLabel(e.kind)}`).includes(query))
      .sort((a, b) => compareText(model.name(a.id), model.name(b.id)));
    const ul = h('ul', { class: 's-glist-items' });
    for (const e of items) {
      const c = model.conduct(e.id, year);
      ul.append(h('li', { class: e.id === st.sel ? 'is-sel' : '' },
        h('button', { type: 'button', class: 's-link', 'aria-current': e.id === st.sel ? 'true' : null, onclick: () => state.set({ sel: e.id }) }, model.name(e.id)),
        h('span', { class: 's-dim' }, ` ${kindLabel(e.kind)}`),
        model.isSynthetic(e.id) ? h('span', { class: 's-tag', title: t('sheet.synthetic_name') }, t('provenance.status.synthetic')) : null,
        c ? h('span', { class: 's-badge', style: { '--c': c.color } }, `${c.glyph} ${c.label}`) : null,
        e.id === last.center ? h('span', { class: 's-dim' }, ` ${t('graph.center_suffix')}`)
          : h('button', { type: 'button', class: 's-mini', onclick: () => state.set({ center: e.id }) }, t('graph.center_button'))));
    }
    list.append(ul);
    const rels = h('ul', { class: 's-glist-rels', 'aria-label': t('graph.relations_shown') });
    for (const r of last.edges) {
      rels.append(h('li', null, `${model.name(r.subject)} `, h('em', null, predLabel(r.predicate)), ` ${model.name(r.object)}`,
        h('span', { class: 's-dim' }, ` ${r.valid[1] ? t('graph.since_until', { from: r.valid[0], to: r.valid[1] }) : t('graph.since', { from: r.valid[0] })}`)));
    }
    list.append(h('h4', null, t('graph.relations_count', { n: last.edges.length })), rels);
  }

  function renderStatic() {
    cyBox.setAttribute('aria-label', t('graph.aria'));
    search.setAttribute('placeholder', t('graph.search_placeholder'));
    search.setAttribute('aria-label', t('graph.search_aria'));
    searchLabel.textContent = t('graph.search_label');
    listTitle.textContent = t('graph.list_title');
  }

  function render() {
    const st = state.get();
    const year = ctx.year();
    renderStatic();
    renderFilters();
    last = buildSubgraph(model, st, year, view.limits);
    note.textContent = cy ? last.note : t('graph.no_cytoscape');
    note.hidden = !note.textContent;
    const hidden = last.hiddenNodes || last.hiddenEdges;
    counter.textContent = last.nodes.size
      ? t('graph.counter', { nodes: last.nodes.size, edges: last.edges.length, year })
        + ' ' + (hidden ? t('graph.truncated', { nodes: last.hiddenNodes, edges: last.hiddenEdges, maxNodes: view.limits.max_nodes, maxEdges: view.limits.max_edges })
          : view.limits.cursor ? t('graph.paged', { shown: view.limits.returned_nodes, total: view.limits.total_nodes })
          : t('graph.not_truncated'))
      : '';
    if (cy) {
      cy.elements().remove();
      const els = [];
      for (const [id, d] of last.nodes) els.push({ group: 'nodes', data: nodeData(model, id, year, last.center, d) });
      for (const r of last.edges) {
        els.push({ group: 'edges', data: {
          id: r.id, source: r.subject, target: r.object, label: predLabel(r.predicate),
          color: FAMILY_COLOR[predFamily(r.predicate)] ?? '#8a9ba3', kind: r.predicate === 'union_with' ? 'union' : 'rel' } });
      }
      cy.add(els);
      if (last.nodes.size) {
        const layout = st.mode === 'kinship'
          ? { name: 'breadthfirst', directed: true, roots: cy.nodes().filter((n) => n.incomers('edge[kind = "rel"]').length === 0).map((n) => n.id()), spacingFactor: 1.15, padding: 24, animate: false }
          : { name: 'concentric', concentric: (n) => 10 - n.data('dist'), levelWidth: () => 1, minNodeSpacing: 34, spacingFactor: 1.2, padding: 24, animate: false };
        cy.layout(layout).run();
        cy.$id(last.center).addClass('center');
        if (st.sel) cy.$id(st.sel).addClass('sel');
        cy.fit(undefined, 24);
      }
    }
    renderList();
  }
  return { render };
}
