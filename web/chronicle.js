// "Chronique" tab (filtered event list) and "Index" tab (all named entities).
// Filters only select among loaded events; nothing is regenerated.

import { h, clear, swatch, kindInfo, fmtInt, fmtYear, foldText, safeColor, siteKindLabel, orgKindLabel, humanLabel } from './util.js';
import { eventActiveIn, stateAt, personStatusAt, personLiving } from './model.js';
import { eventRow } from './inspector.js';

const SCOPES = [
  ['year', 'Année affichée'],
  ['decade', 'Décennie affichée'],
  ['upto', 'Jusqu’à l’année affichée, récents d’abord'],
  ['all', 'Toute la chronique'],
];
const PAGE = 300;

export class Chronicle {
  constructor(root, { onFilterChange }) {
    this.root = root;
    this.onFilterChange = onFilterChange;
    this.ix = null;
    this.kinds = null; // null = every kind
    this.limit = PAGE;
    this.build();
  }

  build() {
    this.scope = h('select', { id: 'chr-scope' }, SCOPES.map(([v, l]) => h('option', { value: v, selected: v === 'decade' }, l)));
    this.site = h('select', { id: 'chr-site' }, h('option', { value: '' }, 'Tous les lieux'));
    this.search = h('input', { id: 'chr-search', type: 'search', placeholder: 'Titre ou description', autocomplete: 'off' });
    this.chips = h('div', { class: 'chips', role: 'group', 'aria-label': 'Types d’événements affichés' });
    this.summary = h('p', { class: 'chr-summary', 'aria-live': 'polite' });
    this.list = h('ol', { class: 'ev-list chr-list' });
    this.more = h('button', { type: 'button', class: 'btn btn-quiet more', hidden: true,
      onclick: () => { this.limit += PAGE; this.render(); } }, 'Afficher plus');
    let timer = 0;
    this.search.addEventListener('input', () => {
      clearTimeout(timer);
      timer = setTimeout(() => this.filterChanged(), 160);
    });
    this.scope.addEventListener('change', () => this.filterChanged(false));
    this.site.addEventListener('change', () => this.filterChanged());
    const allBtn = h('button', { type: 'button', class: 'btn btn-quiet', onclick: () => { this.kinds = null; this.filterChanged(); } }, 'Tous les types');
    const noneBtn = h('button', { type: 'button', class: 'btn btn-quiet', onclick: () => { this.kinds = new Set(); this.filterChanged(); } }, 'Aucun');
    this.root.append(
      h('div', { class: 'filters' },
        h('div', { class: 'field' }, h('label', { for: 'chr-scope' }, 'Période'), this.scope),
        h('div', { class: 'field' }, h('label', { for: 'chr-site' }, 'Lieu'), this.site),
        h('div', { class: 'field field-wide' }, h('label', { for: 'chr-search' }, 'Rechercher'), this.search),
        h('fieldset', { class: 'kind-filter' }, h('legend', null, 'Types'), this.chips, h('div', { class: 'chip-actions' }, allBtn, noneBtn))),
      this.summary, this.list, this.more);
  }

  setWorld(ix) {
    this.ix = ix;
    this.kinds = null;
    this.limit = PAGE;
    clear(this.site).appendChild(h('option', { value: '' }, 'Tous les lieux'));
    for (const s of ix.sitesSorted) {
      const n = ix.eventsBySite.get(s.id)?.length ?? 0;
      this.site.appendChild(h('option', { value: s.id }, `${s.name} (${n})`));
    }
    this.renderChips();
  }

  renderChips() {
    clear(this.chips);
    if (!this.ix) return;
    for (const k of this.ix.kinds) {
      const info = kindInfo(k);
      const on = !this.kinds || this.kinds.has(k);
      this.chips.appendChild(h('button', {
        type: 'button', class: 'chip', 'aria-pressed': on ? 'true' : 'false',
        onclick: () => {
          const set = this.kinds ? new Set(this.kinds) : new Set(this.ix.kinds);
          if (set.has(k)) set.delete(k); else set.add(k);
          this.kinds = set.size === this.ix.kinds.length ? null : set;
          this.filterChanged();
        },
      }, swatch(info.color), info.label, h('span', { class: 'chip-n' }, fmtInt(this.ix.kindCounts.get(k)))));
    }
  }

  filterChanged(notify = true) {
    this.limit = PAGE;
    this.renderChips();
    if (notify) this.onFilterChange?.();
    else this.render();
  }

  // Kind filter as a Set (or null for all), shared with the map halos.
  get kindSet() {
    return this.kinds;
  }

  // Kind, place and text filters, without the period: feeds the year rail.
  matching() {
    if (!this.ix) return [];
    const text = foldText(this.search.value.trim());
    const site = this.site.value;
    return this.ix.events.filter((e) => (!this.kinds || this.kinds.has(e.kind))
      && (!site || (e.site_ids ?? []).includes(site))
      && (!text || foldText(`${e.title} ${e.description}`).includes(text)));
  }

  setYear(year) {
    this.year = year;
  }

  render(selectedId = this.selectedId) {
    this.selectedId = selectedId;
    clear(this.list);
    if (!this.ix) {
      this.summary.textContent = 'Aucun monde chargé.';
      this.more.hidden = true;
      return;
    }
    const year = this.year;
    const scope = this.scope.value;
    let events = this.matching();
    const total = events.length;
    let label;
    if (scope === 'year') {
      events = events.filter((e) => eventActiveIn(e, year, year));
      label = `en ${fmtYear(year)}`;
    } else if (scope === 'decade') {
      const d0 = Math.floor(year / 10) * 10;
      events = events.filter((e) => eventActiveIn(e, d0, d0 + 9));
      label = `de ${fmtYear(d0)} à ${fmtYear(d0 + 9)}`;
    } else if (scope === 'upto') {
      events = events.filter((e) => e.year <= year).reverse();
      label = `jusqu’en ${fmtYear(year)}`;
    } else {
      label = 'sur toute la chronique';
    }
    const all = this.ix.events.length;
    this.summary.textContent = all === 0
      ? 'Ce monde ne contient aucun événement.'
      : `${fmtInt(events.length)} événement${events.length > 1 ? 's' : ''} ${label}` +
        (total < all ? `, ${fmtInt(total)} sur ${fmtInt(all)} après filtres.` : '.');
    if (!events.length && all) {
      this.list.appendChild(h('li', { class: 'empty-line' },
        scope === 'year' || scope === 'decade'
          ? 'Rien pour cette période avec ces filtres. Élargissez la période ou changez d’année.'
          : 'Aucun événement ne correspond aux filtres.'));
    }
    for (const e of events.slice(0, this.limit)) {
      const row = eventRow(this.ix, e, year);
      if (e.id === selectedId) row.setAttribute('aria-current', 'true');
      const sites = (e.site_ids ?? []).map((id) => this.ix.sites.get(id)?.name).filter(Boolean);
      if (sites.length) {
        row.appendChild(h('span', { class: 'ev-sites' }, sites.length > 3 ? `${sites.slice(0, 3).join(', ')} et ${sites.length - 3} autres` : sites.join(', ')));
      }
      this.list.appendChild(h('li', null, row));
    }
    this.more.hidden = events.length <= this.limit;
    this.more.textContent = `Afficher plus (${fmtInt(events.length - this.limit)} restants)`;
  }

  markSelected(id) {
    this.selectedId = id;
    for (const b of this.list.querySelectorAll('[data-sel]')) {
      if (b.dataset.sel === `event:${id}`) b.setAttribute('aria-current', 'true');
      else b.removeAttribute('aria-current');
    }
  }
}

// Navigation helpers for previous/next event buttons.
export function neighbourEventYear(events, year, dir) {
  if (dir > 0) {
    for (const e of events) if (e.year > year) return e.year;
    return null;
  }
  let best = null;
  for (const e of events) if (e.year < year) best = e.year; else break;
  return best;
}

export class EntityIndex {
  constructor(root) {
    this.root = root;
    this.ix = null;
    this.world = null;
    this.search = h('input', { id: 'idx-search', type: 'search', placeholder: 'Nom de lieu, personne, organisation…', autocomplete: 'off' });
    this.body = h('div', { class: 'idx-body' });
    let timer = 0;
    this.search.addEventListener('input', () => {
      clearTimeout(timer);
      timer = setTimeout(() => this.render(), 160);
    });
    this.open = { polity: true, site: true, org: true, person: true };
    root.append(h('div', { class: 'field' }, h('label', { for: 'idx-search' }, 'Rechercher dans l’index'), this.search), this.body);
  }

  setWorld(world, ix) {
    this.world = world;
    this.ix = ix;
  }

  setYear(year) {
    this.year = year;
  }

  render() {
    clear(this.body);
    const ix = this.ix;
    if (!ix) {
      this.body.appendChild(h('p', { class: 'empty-line' }, 'Aucun monde chargé.'));
      return;
    }
    const q = foldText(this.search.value.trim());
    const match = (...parts) => !q || foldText(parts.join(' ')).includes(q);
    const year = this.year;
    const groups = [
      ['polity', 'Puissances', ix.politiesSorted.filter((p) => match(p.name)), (p) => [
        swatch(safeColor(p.color)), h('span', { class: 'idx-name' }, p.name)]],
      ['site', 'Lieux', ix.sitesSorted.filter((s) => match(s.name, s.kind)), (s) => {
        const st = stateAt(ix, s.id, year);
        const pop = st ? st.population : s.initial_population;
        return [h('span', { class: 'idx-name' }, s.name), h('span', { class: 'meta' }, `${siteKindLabel(s.kind)}, ${fmtInt(pop)} hab.`)];
      }],
      ['org', 'Organisations', ix.orgsSorted.filter((o) => match(o.name, o.office_title)), (o) => [
        h('span', { class: 'idx-name' }, o.name), h('span', { class: 'meta' }, orgKindLabel(o.kind))]],
      ['person', 'Personnes', ix.peopleSorted.filter((p) => match(p.name, p.role, p.external_id ?? '')), (p) => {
        const st = personStatusAt(p, year, ix);
        const life = `${fmtYear(p.birth_year)}–${Number.isFinite(p.death_year) ? fmtYear(p.death_year) : ''}`;
        return [h('span', { class: `idx-name${personLiving(st) ? '' : ' is-off'}` }, p.name),
          h('span', { class: 'meta' }, `${life}${p.role ? `, ${humanLabel(p.role)}` : ''}`)];
      }],
    ];
    let any = false;
    for (const [type, label, items, render] of groups) {
      const det = h('details', { class: 'idx-group', open: this.open[type] || !!q },
        h('summary', null, `${label} (${fmtInt(items.length)})`));
      det.addEventListener('toggle', () => { if (!q) this.open[type] = det.open; });
      if (!items.length) {
        det.appendChild(h('p', { class: 'empty-line' }, q ? 'Aucun résultat.' : 'Aucune entrée dans ce monde.'));
      } else {
        any = true;
        const ul = h('ul', { class: 'idx-list' });
        const addRange = (from, to) => {
          for (const it of items.slice(from, to)) {
            ul.appendChild(h('li', null, h('button', { type: 'button', class: 'idx-item', 'data-sel': `${type}:${it.id}` }, render(it))));
          }
        };
        addRange(0, 150);
        det.appendChild(ul);
        if (items.length > 150) {
          const more = h('button', { type: 'button', class: 'btn btn-quiet more',
            onclick: () => { addRange(150, items.length); more.remove(); } }, `Afficher les ${fmtInt(items.length - 150)} autres`);
          det.appendChild(more);
        }
      }
      this.body.appendChild(det);
    }
    if (!any && q) this.body.prepend(h('p', { class: 'empty-line' }, `Rien ne correspond à « ${this.search.value.trim()} ».`));
  }
}

