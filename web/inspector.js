// Entity records ("fiches") for the side panel. Each builder reads the loaded
// world through the index; links carry data-sel="type:id" and are handled by
// one delegated listener in app.js.

import {
  h, svg, swatch, fmtInt, fmtNum, fmtSigned, fmtPercent, fmtYear, fmtSpan, kindInfo,
  siteKindLabel, orgKindLabel, humanLabel, successionLabel, personSourceLabel, safeColor, compareText,
} from './util.js';
import {
  snapIndexAt, exactSnapshot, snapIndexFrom, eventEnd, eventActiveIn,
  sitePolityAt, personStatusAt, personLiving, siblingsOf, holderAt,
} from './model.js';

const TYPE_LABEL = { site: 'Lieu', event: 'Événement', org: 'Organisation', person: 'Personne', polity: 'Puissance' };
export const typeLabel = (t) => TYPE_LABEL[t] ?? t;

function nameOf(ix, type, id) {
  if (type === 'site') return ix.sites.get(id)?.name;
  if (type === 'org') return ix.orgs.get(id)?.name;
  if (type === 'person') return ix.people.get(id)?.name;
  if (type === 'polity') return ix.polities.get(id)?.name;
  if (type === 'event') return ix.eventById.get(id)?.title;
  return undefined;
}

export function link(ix, type, id, label) {
  if (id === null || id === undefined || id === '') return h('span', { class: 'missing' }, 'aucun');
  const name = label ?? nameOf(ix, type, id);
  if (name === undefined) {
    return h('span', { class: 'missing', title: `ID ${id}` }, `absent de l’export (${id})`);
  }
  return h('button', { type: 'button', class: 'link', 'data-sel': `${type}:${id}` }, name || id);
}

function polityLink(ix, id) {
  const p = ix.polities.get(id);
  if (!id) return h('span', { class: 'missing' }, 'sans puissance');
  if (!p) return link(ix, 'polity', id);
  return h('span', { class: 'with-swatch' }, swatch(safeColor(p.color)), link(ix, 'polity', id));
}

function section(title, ...body) {
  return h('section', { class: 'fiche-sec' }, h('h3', null, title), ...body);
}

function facts(rows) {
  return h('dl', { class: 'facts' }, rows.filter(Boolean).map(([k, v]) => [h('dt', null, k), h('dd', null, v)]));
}

function empty(text) {
  return h('p', { class: 'empty-line' }, text);
}

// A list that shows `cap` items then offers to show the rest.
function capped(items, cap, render, cls = 'ref-list') {
  const ul = h('ul', { class: cls });
  const shown = items.slice(0, cap);
  for (const it of shown) ul.appendChild(h('li', null, render(it)));
  if (items.length <= cap) return ul;
  const rest = items.length - cap;
  const more = h('button', {
    type: 'button', class: 'btn btn-quiet more',
    onclick: () => {
      for (const it of items.slice(cap)) ul.appendChild(h('li', null, render(it)));
      more.remove();
    },
  }, `Afficher les ${fmtInt(rest)} autres`);
  return h('div', null, ul, more);
}

export function eventRow(ix, e, year) {
  const k = kindInfo(e.kind);
  const now = year !== undefined && eventActiveIn(e, year, year);
  return h('button', { type: 'button', class: `ev-row${now ? ' is-now' : ''}`, 'data-sel': `event:${e.id}` },
    h('span', { class: 'ev-year' }, fmtSpan(e.year, eventEnd(e))),
    swatch(k.color, 'ev-kind'),
    h('span', { class: 'ev-title' }, e.title || k.label),
  );
}

function lifeSpan(p) {
  const b = Number.isFinite(p.birth_year) ? fmtYear(p.birth_year) : '?';
  const d = Number.isFinite(p.death_year) ? fmtYear(p.death_year) : '';
  return `${b}–${d}`;
}

function personRef(ix, p, year) {
  const st = personStatusAt(p, year, ix);
  const age = personLiving(st) && st.age !== null ? `, ${st.age} ans en ${fmtYear(year)}` : '';
  return h('span', { class: 'person-ref' }, link(ix, 'person', p.id),
    h('span', { class: 'meta' }, ` ${lifeSpan(p)}${age}`));
}

function statusText(ix, p, year) {
  const st = personStatusAt(p, year, ix);
  if (st.phase === 'before') return `Avant la naissance (en ${fmtYear(p.birth_year)}).`;
  if (st.phase === 'after') return `Après le décès (en ${fmtYear(p.death_year)}).`;
  if (st.phase === 'dies') return `Année du décès${st.age !== null ? `, à ${st.age} ans` : ''}.`;
  if (st.phase === 'unknown') {
    return `Inconnu : aucun décès enregistré, et l’échantillon importé s’arrête vers ${fmtYear(st.end)}${ix.genealogyEndEstimated ? ' (estimation)' : ''}.`;
  }
  return st.age === null ? 'En vie.' : `En vie, ${st.age} ans.`;
}

function title(text, sub, chip) {
  return h('header', { class: 'fiche-head' },
    h('h2', { class: 'fiche-title', tabindex: '-1' }, chip ?? null, text || 'Sans nom'),
    sub ? h('p', { class: 'fiche-sub' }, sub) : null);
}

function amount(v) {
  if (!Number.isFinite(v)) return '—';
  return Math.abs(v) >= 100 ? fmtInt(v) : fmtNum(v, 1);
}

// Population of a site across all snapshots, with event ticks and the year cursor.
function sparkline(ix, site, year) {
  const n = ix.snaps.length;
  if (n < 2) return null;
  const values = ix.snapStates.map((m) => m.get(site.id)?.population ?? null);
  let max = 0;
  for (const v of values) if (v !== null && v > max) max = v;
  const W = 320;
  const H = 64;
  const lo = ix.snapYears[0];
  const hi = ix.snapYears[n - 1];
  const x = (y) => ((y - lo) / Math.max(1, hi - lo)) * W;
  const y = (v) => H - 6 - (max ? (v / max) * (H - 14) : 0);
  let d = '';
  values.forEach((v, i) => {
    if (v === null) return;
    d += `${d ? 'L' : 'M'}${x(ix.snapYears[i]).toFixed(1)},${y(v).toFixed(1)}`;
  });
  const ticks = (ix.eventsBySite.get(site.id) ?? []).map((e) => svg('rect', {
    x: x(e.year).toFixed(1), y: H - 4, width: 2, height: 4, fill: kindInfo(e.kind).color,
  }));
  const cur = Math.min(Math.max(year, lo), hi);
  return h('figure', { class: 'spark' },
    svg('svg', { viewBox: `0 0 ${W} ${H}`, preserveAspectRatio: 'none', role: 'img',
      'aria-label': `Population de ${site.name} de ${fmtYear(lo)} à ${fmtYear(hi)}, maximum ${fmtInt(max)} habitants.` },
    svg('path', { d, fill: 'none', stroke: '#1b3440', 'stroke-width': 1.5, 'vector-effect': 'non-scaling-stroke' }),
    svg('line', { x1: x(cur), x2: x(cur), y1: 0, y2: H, stroke: '#ac5657', 'stroke-width': 1.5, 'vector-effect': 'non-scaling-stroke' }),
    ...ticks),
    h('figcaption', null, `Population de ${fmtYear(lo)} à ${fmtYear(hi)} (max. ${fmtInt(max)}). Repères : événements du lieu.`));
}

function delta(now, before) {
  if (!Number.isFinite(now) || !Number.isFinite(before)) return null;
  const d = now - before;
  const pct = before > 0 ? ` (${fmtPercent(d / before)})` : '';
  return `${fmtSigned(d)}${pct}`;
}

// --- Site ----------------------------------------------------------------

export function renderSite(ctx, s) {
  const { ix, year, world } = ctx;
  const polityId = sitePolityAt(ix, s, year);
  const capitalOf = world.polities.filter((p) => p.capital_id === s.id);
  const sub = [siteKindLabel(s.kind)];
  const node = h('article', { class: 'fiche' });
  node.appendChild(title(s.name, h('span', null,
    sub.join(', '), ', ', polityLink(ix, polityId),
    capitalOf.length ? h('span', null, ', capitale') : null)));

  const si = snapIndexAt(ix, year);
  const st = si >= 0 ? ix.snapStates[si].get(s.id) : null;
  if (st) {
    const prev = si > 0 ? ix.snapStates[si - 1].get(s.id) : null;
    const snapYear = ix.snapYears[si];
    const note = snapYear !== year ? ` (relevé de ${fmtYear(snapYear)})` : '';
    node.appendChild(section(`En ${fmtYear(year)}${note}`, facts([
      ['Population', h('span', null, h('strong', null, fmtInt(st.population)),
        st.population === 0 ? h('span', { class: 'flag' }, ' déserté') : null)],
      prev && ['Depuis ' + fmtYear(ix.snapYears[si - 1]), delta(st.population, prev.population) ?? '—'],
      ['Depuis l’origine', delta(st.population, s.initial_population) ?? '—'],
      ['Vivres', amount(st.food)],
      ['Richesse', amount(st.wealth)],
      ['Stabilité', fmtNum(st.stability, 2)],
    ])));
  } else {
    node.appendChild(section(`En ${fmtYear(year)}`, empty('Aucun relevé d’état pour cette année ; population initiale : ' + fmtInt(s.initial_population) + '.')));
  }
  const sp = sparkline(ix, s, year);
  if (sp) node.appendChild(sp);

  // Effects on this site from events active this year.
  const effects = [];
  for (const e of ix.eventsBySite.get(s.id) ?? []) {
    if (!eventActiveIn(e, year, year)) continue;
    for (const ef of e.effects ?? []) if (ef.site_id === s.id) effects.push({ e, ef });
  }
  if (effects.length) {
    node.appendChild(section(`Effets en cours en ${fmtYear(year)}`, h('ul', { class: 'ref-list' }, effects.map(({ e, ef }) => h('li', null,
      link(ix, 'event', e.id), h('span', { class: 'meta' }, ` ${fmtSigned(ef.population_delta)} hab.`),
      ef.summary ? h('div', { class: 'small' }, ef.summary) : null)))));
  }

  const r = s.resources ?? {};
  const resRows = [['food', 'Vivres (capacité annuelle)'], ['water', 'Eau'], ['timber', 'Bois'], ['iron', 'Fer'], ['stone', 'Pierre'], ['salt', 'Sel']];
  node.appendChild(section('Ressources initiales',
    h('table', { class: 'num-table' }, h('tbody', null, resRows.map(([k, l]) => h('tr', null, h('th', { scope: 'row' }, l), h('td', null, amount(r[k])))))),
    h('p', { class: 'hint' }, 'Potentiels de départ en unités arbitraires, pas des gisements relevés.')));

  const routes = ix.routesBySite.get(s.id) ?? [];
  node.appendChild(section('Routes', routes.length ? capped(routes, 12, (rt) => {
    const other = rt.from === s.id ? rt.to : rt.from;
    return h('span', null, link(ix, 'site', other),
      h('span', { class: 'meta' }, ` ${fmtNum(rt.distance_km, 0)} km, ${fmtNum(rt.travel_days, 1)} j, ${humanLabel(rt.kind)}`));
  }) : empty('Aucune route depuis ce lieu.')));

  const orgs = ix.orgsBySite.get(s.id) ?? [];
  if (orgs.length) node.appendChild(section('Organisations', capped(orgs, 10, (o) => h('span', null, link(ix, 'org', o.id), h('span', { class: 'meta' }, ` ${orgKindLabel(o.kind)}`)))));

  const people = ix.peopleBySite.get(s.id) ?? [];
  if (people.length) {
    const ordered = [...people].sort((a, b) => {
      const aa = personLiving(personStatusAt(a, year, ix)) ? 0 : 1;
      const bb = personLiving(personStatusAt(b, year, ix)) ? 0 : 1;
      return aa - bb || a.birth_year - b.birth_year;
    });
    node.appendChild(section('Personnes résidentes', capped(ordered, 12, (p) => personRef(ix, p, year))));
  }

  const evs = ix.eventsBySite.get(s.id) ?? [];
  node.appendChild(section(`Événements (${fmtInt(evs.length)})`, evs.length
    ? capped(evs, 40, (e) => eventRow(ix, e, year), 'ev-list')
    : empty('Aucun événement ne touche ce lieu.')));

  node.appendChild(idFacts(ctx, [['ID', s.id], ['Région', s.region_id], ['Culture', s.culture_id], ['Position', coords(world, s)]]));
  return node;
}

function coords(world, s) {
  const b = world.map?.bounds;
  if (Array.isArray(b) && b.length === 4 && b.every(Number.isFinite) && b[2] > b[0] && b[3] > b[1]) {
    const lon = b[0] + s.x * (b[2] - b[0]);
    const lat = b[3] - s.y * (b[3] - b[1]);
    return `${fmtNum(Math.abs(lat), 2)}°${lat >= 0 ? 'N' : 'S'}, ${fmtNum(Math.abs(lon), 2)}°${lon >= 0 ? 'E' : 'O'}`;
  }
  return `x ${fmtNum(s.x, 3)}, y ${fmtNum(s.y, 3)}`;
}

function idFacts(ctx, rows) {
  return h('details', { class: 'ids' }, h('summary', null, 'Identifiants'),
    facts(rows.filter(([, v]) => v !== undefined && v !== null && v !== '').map(([k, v]) => [k, String(v)])));
}

// --- Event ---------------------------------------------------------------

export function renderEvent(ctx, e) {
  const { ix } = ctx;
  const k = kindInfo(e.kind);
  const end = eventEnd(e);
  const node = h('article', { class: 'fiche' });
  node.appendChild(title(e.title || k.label, h('span', null,
    h('span', { class: 'with-swatch' }, swatch(k.color), k.label), `, ${end > e.year ? `de ${fmtYear(e.year)} à ${fmtYear(end)}` : `en ${fmtYear(e.year)}`}`,
    Number.isFinite(e.importance) ? `, importance ${fmtNum(e.importance, 2)}` : '')));
  if (e.description) node.appendChild(h('p', { class: 'narrative' }, e.description));

  const sites = e.site_ids ?? [];
  node.appendChild(section(`Lieux (${fmtInt(sites.length)})`, sites.length
    ? capped(sites, 16, (id) => link(ix, 'site', id))
    : empty('Aucun lieu rattaché.')));

  if ((e.organization_ids ?? []).length) {
    node.appendChild(section('Organisations', capped(e.organization_ids, 10, (id) => link(ix, 'org', id))));
  }
  if ((e.person_ids ?? []).length) {
    node.appendChild(section('Personnes', capped(e.person_ids, 12, (id) => {
      const p = ix.people.get(id);
      return p ? personRef(ix, p, e.year) : link(ix, 'person', id);
    })));
  }

  const effects = e.effects ?? [];
  if (effects.length) {
    const before = exactSnapshot(ix, e.year);
    const afterIdx = snapIndexFrom(ix, end + 1);
    const afterYear = afterIdx >= 0 ? ix.snapYears[afterIdx] : null;
    const head = h('tr', null,
      h('th', { scope: 'col' }, 'Lieu'),
      h('th', { scope: 'col', class: 'num' }, 'Habitants'),
      h('th', { scope: 'col', class: 'num' }, `${fmtYear(e.year)} → ${afterYear !== null ? fmtYear(afterYear) : '?'}`),
      h('th', { scope: 'col', class: 'num' }, 'Vivres'),
      h('th', { scope: 'col', class: 'num' }, 'Richesse'));
    const rows = effects.map((ef) => {
      const b = before >= 0 ? ix.snapStates[before].get(ef.site_id) : null;
      const a = afterIdx >= 0 ? ix.snapStates[afterIdx].get(ef.site_id) : null;
      const ba = b && a ? `${fmtInt(b.population)} → ${fmtInt(a.population)}` : '—';
      return [
        h('tr', null,
          h('th', { scope: 'row' }, link(ix, 'site', ef.site_id)),
          h('td', { class: `num ${ef.population_delta < 0 ? 'neg' : ef.population_delta > 0 ? 'pos' : ''}` }, fmtSigned(ef.population_delta)),
          h('td', { class: 'num' }, ba),
          h('td', { class: 'num' }, fmtSigned(ef.food_delta, 1)),
          h('td', { class: 'num' }, fmtSigned(ef.wealth_delta, 1))),
        ef.summary ? h('tr', { class: 'sum-row' }, h('td', { colspan: '5' }, ef.summary)) : null,
      ];
    });
    node.appendChild(section('Effets', h('div', { class: 'table-scroll' },
      h('table', { class: 'num-table effects' }, h('thead', null, head), h('tbody', null, rows))),
    h('p', { class: 'hint' }, `Relevés au début de chaque année. L’écart avant/après inclut aussi la croissance et les autres événements de la période ; la colonne « Habitants » est l’effet propre de cet événement.`)));
  }

  // Umbrella events (wars, epidemic waves) carry no effect themselves; their
  // dated children cite them as a cause and carry the effects.
  const cons = ix.consequences.get(e.id) ?? [];
  if (!effects.length) {
    const child = [];
    for (const c of cons) for (const ef of c.effects ?? []) child.push({ c, ef });
    if (child.length) {
      const total = child.reduce((acc, x) => acc + (Number(x.ef.population_delta) || 0), 0);
      const nSites = new Set(child.map((x) => x.ef.site_id)).size;
      const rows = [...child].sort((a, b) => a.c.year - b.c.year || a.ef.population_delta - b.ef.population_delta);
      node.appendChild(section('Effets des événements rattachés',
        h('p', { class: 'small' }, `Pas d’effet direct. Les ${fmtInt(new Set(child.map((x) => x.c.id)).size)} événements qui le citent comme cause totalisent ${fmtSigned(total)} habitants sur ${fmtInt(nSites)} lieu${nSites > 1 ? 'x' : ''}.`),
        h('div', { class: 'table-scroll' }, h('table', { class: 'num-table' },
          h('thead', null, h('tr', null, h('th', { scope: 'col' }, 'Année'), h('th', { scope: 'col' }, 'Lieu'), h('th', { scope: 'col' }, 'Événement'), h('th', { scope: 'col', class: 'num' }, 'Habitants'))),
          h('tbody', null, rows.slice(0, 40).map(({ c, ef }) => h('tr', null,
            h('td', null, fmtYear(c.year)),
            h('th', { scope: 'row' }, link(ix, 'site', ef.site_id)),
            h('td', null, link(ix, 'event', c.id, kindInfo(c.kind).label)),
            h('td', { class: `num ${ef.population_delta < 0 ? 'neg' : ef.population_delta > 0 ? 'pos' : ''}` }, fmtSigned(ef.population_delta))))))),
        rows.length > 40 ? h('p', { class: 'hint' }, `${fmtInt(rows.length - 40)} lignes de plus dans la liste des conséquences.`) : null));
    } else {
      node.appendChild(section('Effets', empty('Aucun effet chiffré enregistré.')));
    }
  }

  const causes = e.causes ?? [];
  node.appendChild(section('Causes', causes.length
    ? h('ul', { class: 'ev-list' }, causes.map((id) => h('li', null, ix.eventById.get(id) ? eventRow(ix, ix.eventById.get(id)) : link(ix, 'event', id))))
    : empty('Aucune cause enregistrée.')));
  if (cons.length) node.appendChild(section('Conséquences', capped(cons, 12, (c) => eventRow(ix, c), 'ev-list')));
  node.appendChild(idFacts(ctx, [['ID', e.id], ['Type', e.kind]]));
  return node;
}

// --- Organization ----------------------------------------------------------

export function renderOrg(ctx, o) {
  const { ix, year, world } = ctx;
  const node = h('article', { class: 'fiche' });
  node.appendChild(title(o.name, h('span', null, orgKindLabel(o.kind), ', ', polityLink(ix, o.polity_id))));
  const startYear = world.config?.start_year;
  const holderRef = (id) => {
    if (!id) return h('span', { class: 'missing' }, 'vacant');
    const p = ix.people.get(id);
    return p ? personRef(ix, p, year) : link(ix, 'person', id);
  };
  if (o.collective_office === true) {
    // Collective body from the source model (e.g. an assembly): no personal holder, no succession.
    node.appendChild(section('Office', facts([
      ['Instance', `instance collective (${o.office_title || 'assemblée'})`],
      ['Titulaire', h('span', { class: 'missing' }, 'aucun titulaire personnel')],
      o.succession ? ['Règle du modèle', successionLabel(o.succession)] : null,
    ]), h('p', { class: 'hint' }, 'Office collectif du modèle source : ses membres ne sont pas exportés.')));
  } else {
    const now = holderAt(ix, o, year);
    // Holding an office needs the person active that year: alive and before the death year.
    const activeNote = (id) => {
      const p = id ? ix.people.get(id) : null;
      if (!p) return null;
      const st = personStatusAt(p, year, ix);
      if (st.phase === 'dies' || st.phase === 'after') return `pas de titulaire actif : décès de ${p.name} en ${fmtYear(p.death_year)}`;
      if (st.phase === 'unknown') return `pas de titulaire actif connu : la trace de ${p.name} s’arrête vers ${fmtYear(st.end)}`;
      if (st.phase === 'before') return `pas de titulaire actif : naissance de ${p.name} enregistrée en ${fmtYear(p.birth_year)}`;
      return null;
    };
    const note = year !== startYear ? activeNote(now.id) : null;
    node.appendChild(section('Office', facts([
      ['Titre', o.office_title || '—'],
      ['Succession', o.succession ? successionLabel(o.succession) : '—'],
      [`Titulaire en ${fmtYear(startYear)}`, holderRef(o.holder_id)],
      year !== startYear && [`Titulaire en ${fmtYear(year)}`, h('span', null,
        note ? h('span', { class: 'missing' }, note) : holderRef(now.id),
        !note && now.since ? h('span', { class: 'meta' }, ' depuis ', link(ix, 'event', now.since.id, `la succession de ${fmtYear(now.since.year)}`)) : null)],
    ]), h('p', { class: 'hint' }, 'Titulaire à l’année affichée déduit des événements de succession de l’export (nouveau titulaire cité en premier) ; il doit être en vie et avant son année de décès.')));
  }

  const sites = o.site_ids ?? [];
  node.appendChild(section(`Lieux (${fmtInt(sites.length)})`, sites.length
    ? capped([...sites].sort((a, b) => compareText(ix.sites.get(a)?.name ?? a, ix.sites.get(b)?.name ?? b)), 16, (id) => link(ix, 'site', id))
    : empty('Aucun lieu rattaché.')));

  const evs = ix.eventsByOrg.get(o.id) ?? [];
  node.appendChild(section(`Événements (${fmtInt(evs.length)})`, evs.length
    ? capped(evs, 40, (e) => eventRow(ix, e, year), 'ev-list')
    : empty('Aucun événement ne cite cette organisation.')));
  node.appendChild(idFacts(ctx, [['ID', o.id], ['Modèle', o.blueprint_id], ['Type', o.kind]]));
  return node;
}

// --- Person ----------------------------------------------------------------

function parentLine(ix, id, year) {
  if (!id) return h('span', { class: 'missing' }, 'inconnu');
  const p = ix.people.get(id);
  return p ? personRef(ix, p, year) : h('span', { class: 'missing' }, `absent de l’export (ID ${id})`);
}

export function renderPerson(ctx, p) {
  const { ix, year } = ctx;
  const node = h('article', { class: 'fiche' });
  node.appendChild(title(p.name, h('span', null, p.role ? humanLabel(p.role) : 'Sans rôle', ', ',
    h('span', { class: `src-badge ${p.source === 'lineage-lab' ? 'st-imported' : 'st-synthetic'}` }, personSourceLabel(p.source)))));
  node.appendChild(section('Vie', facts([
    ['Naissance', fmtYear(p.birth_year)],
    ['Décès', Number.isFinite(p.death_year) ? fmtYear(p.death_year) : (p.source === 'virtual' ? 'non enregistré' : 'non enregistré dans l’échantillon importé')],
    [`En ${fmtYear(year)}`, statusText(ix, p, year)],
    ['Résidence', link(ix, 'site', p.site_id)],
    p.external_id ? ['ID externe', p.external_id] : null,
  ])));

  const children = ix.children.get(p.id) ?? [];
  const sibs = siblingsOf(ix, p);
  node.appendChild(section('Famille',
    h('div', { class: 'family' },
      h('div', { class: 'fam-row fam-parents' },
        h('div', null, h('span', { class: 'fam-role' }, 'Père'), parentLine(ix, p.father_id, year)),
        h('div', null, h('span', { class: 'fam-role' }, 'Mère'), parentLine(ix, p.mother_id, year))),
      h('div', { class: 'fam-row fam-self' }, h('span', { class: 'fam-role' }, 'Personne'), h('strong', null, p.name || p.id)),
      h('div', { class: 'fam-row fam-children' }, h('span', { class: 'fam-role' }, `Enfants (${children.length})`),
        children.length ? capped(children, 12, (c) => personRef(ix, c, year)) : h('span', { class: 'missing' }, 'aucun dans l’export'))),
    sibs.length ? h('div', { class: 'fam-sibs' }, h('span', { class: 'fam-role' }, `Fratrie (${sibs.length})`), capped(sibs, 10, (c) => personRef(ix, c, year))) : null,
    h('p', { class: 'hint' }, 'Filiation telle qu’exportée ; un parent hors de l’échantillon apparaît comme absent.')));

  const offices = ix.orgsByHolder.get(p.id) ?? [];
  if (offices.length) {
    node.appendChild(section(`Titulaire en ${fmtYear(ctx.world.config?.start_year)}`, capped(offices, 10, (o) => h('span', null, link(ix, 'org', o.id), o.office_title ? h('span', { class: 'meta' }, ` ${o.office_title}`) : null))));
  }
  const evs = ix.eventsByPerson.get(p.id) ?? [];
  node.appendChild(section(`Événements (${fmtInt(evs.length)})`, evs.length
    ? capped(evs, 30, (e) => eventRow(ix, e, year), 'ev-list')
    : empty('Aucun événement ne cite cette personne.')));
  node.appendChild(idFacts(ctx, [['ID', p.id], ['Source', p.source]]));
  return node;
}

// --- Polity ----------------------------------------------------------------

export function renderPolity(ctx, pol) {
  const { ix, year, world } = ctx;
  const color = safeColor(pol.color);
  const node = h('article', { class: 'fiche' });
  node.appendChild(title(pol.name, h('span', null, 'Capitale : ', link(ix, 'site', pol.capital_id)), swatch(color, 'title-swatch')));

  const si = snapIndexAt(ix, year);
  const held = [];
  for (const s of world.sites) {
    const st = si >= 0 ? ix.snapStates[si].get(s.id) : null;
    const pid = (st && st.polity_id) || (si < 0 ? s.polity_id : null);
    if (pid === pol.id) held.push({ s, pop: st ? st.population : s.initial_population });
  }
  held.sort((a, b) => b.pop - a.pop);
  const total = held.reduce((acc, x) => acc + x.pop, 0);
  node.appendChild(section(`En ${fmtYear(year)}`, facts([
    ['Lieux tenus', fmtInt(held.length)],
    ['Habitants', fmtInt(total)],
  ]), held.length ? capped(held, 15, ({ s, pop }) => h('span', null, link(ix, 'site', s.id), h('span', { class: 'meta' }, ` ${fmtInt(pop)} hab.`))) : empty('Aucun lieu tenu à cette date.')));

  const orgs = ix.orgsByPolity.get(pol.id) ?? [];
  if (orgs.length) node.appendChild(section('Organisations', capped(orgs, 12, (o) => h('span', null, link(ix, 'org', o.id), h('span', { class: 'meta' }, ` ${orgKindLabel(o.kind)}`)))));

  // Events citing its organizations, or touching a site it held that year.
  const orgIds = new Set(orgs.map((o) => o.id));
  const evs = ix.events.filter((e) => (e.organization_ids ?? []).some((id) => orgIds.has(id))
    || (e.site_ids ?? []).some((id) => {
      const site = ix.sites.get(id);
      return site && sitePolityAt(ix, site, e.year) === pol.id;
    }));
  node.appendChild(section(`Événements (${fmtInt(evs.length)})`, evs.length
    ? capped(evs, 40, (e) => eventRow(ix, e, year), 'ev-list')
    : empty('Aucun événement rattaché.')));
  node.appendChild(idFacts(ctx, [['ID', pol.id], ['Modèle', pol.blueprint_id], ['Couleur', pol.color]]));
  return node;
}
