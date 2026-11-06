// Entity sheet: conduct class first, legal status per jurisdiction, provenance
// and precision, relations, conduct history and the explanation of a choice.

import { h, clear } from './util.js';
import { t, tk, tv, fmtInt } from './i18n.js';
import {
  kindLabel, natureLabel, legalLabel, LEGAL_CLASS, personSourceLabel, eventKindLabel, predLabel, inRange,
} from './history-model.js';

const span = (v) => `${v[0]}–${v[1] ?? '…'}`;

export function createSheet(root, ctx) {
  const { model, state } = ctx;
  const link = (id, text) => h('button', { type: 'button', class: 's-link', onclick: () => state.set({ sel: id }) }, text ?? model.name(id));
  const section = (title, ...kids) => h('section', { class: 's-sec' }, h('h3', null, title), ...kids);
  const dl = (rows) => h('dl', { class: 's-dl' }, rows.filter(Boolean).flatMap(([k, v]) => [h('dt', null, k), h('dd', null, v)]));

  function provenanceBlock(p, extra = []) {
    const status = p?.status ?? 'unavailable';
    return section(t('sheet.provenance'), dl([
      [t('sheet.status'), h('span', { class: `status st-${status}` }, t(`provenance.status.${status}`))],
      [t('sheet.source'), p?.source ?? '—'], [t('sheet.rule'), p?.rule_id ?? '—'], ...extra,
    ]));
  }

  function conductBlock(e, year) {
    const c = model.conduct(e.id, year);
    if (!c) {
      const why = e.kind === 'office' ? t('sheet.no_conduct.office')
        : e.nature === 'category' ? t('sheet.no_conduct.category')
        : e.nature === 'community' ? t('sheet.no_conduct.community')
        : t('sheet.no_conduct.no_episode');
      return h('div', { class: 's-conduct s-conduct-none' }, h('p', { class: 's-conduct-label' }, t('sheet.no_conduct')), h('p', { class: 's-dim' }, why));
    }
    const bar = (v, label) => h('div', { class: 's-axis' }, h('span', null, label),
      h('span', { class: 's-axis-track', role: 'img', 'aria-label': t('sheet.axis_aria', { label, value: v }) },
        h('span', { class: 's-axis-zero' }), h('span', { class: 's-axis-fill', style: { left: v < 0 ? `${50 + v / 200}%` : '50%', width: `${Math.abs(v) / 200}%`, background: c.color } })),
      h('span', { class: 's-num' }, (v > 0 ? '+' : '') + fmtInt(v)));
    return h('div', { class: 's-conduct', style: { '--c': c.color } },
      h('p', { class: 's-conduct-kicker' }, t('sheet.conduct_class_in', { year })),
      h('p', { class: 's-conduct-label' }, h('span', { 'aria-hidden': 'true' }, c.glyph + ' '), c.label),
      h('p', { class: 's-dim' }, `${e.nature === 'person' ? t('sheet.label_person') : t('sheet.label_org')} · ${t('sheet.derived_note')}`),
      bar(c.axes.morality_bps, t('sheet.morality')), bar(c.axes.lawfulness_bps, t('sheet.lawfulness')),
      e.visibility === 'covert' ? h('p', { class: 's-covert' }, t('sheet.covert')) : null);
  }

  function legalBlock(e, year) {
    const segs = model.legalFor(e.id);
    if (!segs.length) return section(t('sheet.legal'), h('p', { class: 's-dim' }, t('sheet.legal_default')));
    const rows = model.jurisdictions(e.id).map((j) => {
      const now = model.legalAt(e.id, j, year);
      const all = segs.filter((s) => s.jurisdiction === j).sort((a, b) => a.valid[0] - b.valid[0]);
      return h('li', null,
        h('p', null, link(j), ' : ',
          now ? h('span', { class: `s-legal ${LEGAL_CLASS[now.status]}` }, legalLabel(now.status)) : h('span', { class: 's-legal lg-licit' }, t('sheet.legal_none_licit')),
          now ? h('span', { class: 's-dim' }, ` ${t('sheet.since', { year: now.valid[0] })}`) : null),
        h('ul', { class: 's-segs' }, all.map((s) => h('li', { class: inRange(s.valid, year) ? 'is-now' : '' },
          `${legalLabel(s.status)} ${span(s.valid)} — `,
          s.cause_event_id ? link(s.cause_event_id, t('sheet.cause_named', { name: model.name(s.cause_event_id) })) : t('sheet.observed_start'),
          s.end_cause_event_id && s.valid[1] ? [` ; ${t('sheet.end')} : `, link(s.end_cause_event_id, model.name(s.end_cause_event_id))] : null))));
    });
    return section(t('sheet.legal_by_jurisdiction'), h('ul', { class: 's-jur' }, rows), h('p', { class: 's-dim' }, t('sheet.legal_independent')));
  }

  function relationsBlock(e, year) {
    const rels = model.relationsAt(e.id, year);
    if (!rels.length) return section(t('sheet.relations'), h('p', { class: 's-dim' }, t('sheet.no_relation', { year })));
    const ul = h('ul', { class: 's-rels' });
    for (const r of rels) {
      const out = r.subject === e.id;
      ul.append(h('li', null,
        out ? [h('em', null, predLabel(r.predicate)), ' ', link(r.object)] : [link(r.subject), ' ', h('em', null, predLabel(r.predicate)), ` ${t('sheet.this_entity')}`],
        h('span', { class: 's-dim' }, ` ${span(r.valid)}`),
        r.cause_event_id ? [h('span', { class: 's-dim' }, ` · ${t('sheet.cause')} `), link(r.cause_event_id)] : null));
    }
    return section(t('sheet.relations_in', { year, n: rels.length }), ul);
  }

  function historyBlock(e) {
    if (!e.alignment_episodes?.length || e.nature === 'category') return null;
    const ul = h('ul', { class: 's-segs' });
    for (const a of e.alignment_episodes) {
      const c = model.conduct(e.id, a.valid[0]);
      if (!c) continue;
      ul.append(h('li', null, `${span(a.valid)} : `, h('span', { class: 's-badge', style: { '--c': c.color } }, `${c.glyph} ${c.label}`),
        ` (${a.morality_bps} / ${a.lawfulness_bps})`, a.cause_event_id ? [' · ', link(a.cause_event_id, t('sheet.cause_named', { name: model.name(a.cause_event_id) }))] : null));
    }
    return section(t('sheet.conduct_history'), ul);
  }

  function eventsBlock(e) {
    const evs = model.eventsFor(e.id);
    if (!evs.length) return null;
    return section(t('sheet.events'), h('ul', { class: 's-rels' }, evs.map((ev) => h('li', null, `${ev.year} · `, link(ev.id, tk(ev, 'label'))))));
  }

  function officeBody(e, year) {
    const tn = model.tenureAt(e.id, year);
    const tenures = ctx.view.tenures.filter((x) => x.office_id === e.id);
    const proof = tn?.selection_proof;
    return [
      section(t('sheet.holder_in', { year }),
        tn ? (tn.mode === 'vacant' ? h('p', { class: 's-vacant' }, t('sheet.vacant_note')) : h('p', { class: 's-holder' }, link(tn.person_id)))
          : h('p', { class: 's-dim' }, t('sheet.no_tenure')),
        tn ? dl([[t('sheet.tenure'), span(tn.valid)], [t('sheet.mode'), t(`tenure_mode.${tn.mode}`)],
          [t('sheet.cause'), tn.cause_event_id ? link(tn.cause_event_id) : t('sheet.before_observed')],
          [t('sheet.appointment_rule'), t(`rule.${e.appointment_rule}`)]]) : null),
      proof ? section(t('sheet.why'), h('p', null, tk(proof, 'summary')),
        proof.candidates ? h('table', { class: 's-table' },
          h('thead', null, h('tr', null, ['sheet.col.rank', 'sheet.col.candidate', 'sheet.col.weight', 'sheet.col.remark'].map((k) => h('th', { scope: 'col' }, t(k))))),
          h('tbody', null, proof.candidates.map((c) => h('tr', null, h('td', null, String(c.rank)), h('td', null, c.name), h('td', { class: 's-num' }, fmtInt(c.weight)), h('td', null, tk(c, 'note')))))) : null) : null,
      section(t('sheet.tenures'), h('ul', { class: 's-segs' }, tenures.map((x) => h('li', { class: inRange(x.valid, year) ? 'is-now' : '' },
        `${span(x.valid)} : `, x.mode === 'vacant' ? t('term.vacant') : link(x.person_id))))),
    ];
  }

  function measureText(m) {
    return `${fmtInt(m.value)} ${t(`unit.${m.unit}`)} — ${t(`precision.${m.precision}`)}, ${t(`coverage.${m.coverage}`)} (${t(`basis.${m.basis}`)}, ${m.at_year})`;
  }

  function entityBody(e, year) {
    const alive = model.exists(e.id, year);
    const head = h('header', { class: 's-sheet-head' },
      h('p', { class: 's-kicker' }, `${kindLabel(e.kind)} · ${e.id}`),
      h('h2', null, model.name(e.id)),
      !alive ? h('p', { class: 's-warn' }, t('sheet.not_existing', { year, span: span(e.existence ?? [0, null]) })) : null,
      h('div', { class: 's-actions' },
        h('button', { type: 'button', class: 'btn btn-quiet', onclick: () => state.set({ center: e.id }) }, t('sheet.center_graph')),
        e.kind === 'person' ? h('button', { type: 'button', class: 'btn btn-quiet', onclick: () => state.set({ center: e.id, mode: 'kinship' }) }, t('sheet.show_kinship')) : null));
    const parts = [head, conductBlock(e, year)];
    if (e.kind === 'office') { parts.push(...officeBody(e, year)); return parts; }
    parts.push(dl([
      [t('sheet.nature'), natureLabel(e.nature)], [t('sheet.archetype'), e.archetype_id], [t('sheet.existence'), span(e.existence)],
      e.kind === 'person' ? [t('sheet.birth_death'), `${e.birth_year} / ${e.death_year ?? '—'}`] : null,
      e.note_key ? [t('sheet.note'), tk(e, 'note')] : null,
    ]));
    parts.push(legalBlock(e, year));
    const holds = model.relationsAt(e.id, year).filter((r) => r.predicate === 'holds_office' && r.subject === e.id);
    if (holds.length) parts.push(section(t('sheet.offices'), h('ul', { class: 's-rels' }, holds.map((r) => h('li', null, link(r.object), h('span', { class: 's-dim' }, ` ${span(r.valid)}`))))));
    const extra = [];
    if (e.kind === 'person') extra.push([t('sheet.person_source'), personSourceLabel(e.person_source)], [t('sheet.observed_genealogy'), span(e.observed_span)]);
    if (e.member_count) extra.push([t('sheet.expected_headcount'), measureText(e.member_count.expected)], [t('sheet.realized_headcount'), measureText(e.member_count.realized)]);
    parts.push(provenanceBlock(e.provenance, extra), relationsBlock(e, year), historyBlock(e), eventsBlock(e));
    return parts;
  }

  function eventBody(ev) {
    const chain = [];
    const seen = new Set([ev.id]);
    let frontier = ev.cause_ids;
    while (frontier.length && chain.length < 12) {
      const next = [];
      for (const id of frontier) {
        if (seen.has(id)) continue;
        seen.add(id); chain.push(id);
        next.push(...(model.events.get(id)?.cause_ids ?? []));
      }
      frontier = next;
    }
    const cons = model.consequences(ev.id);
    return [
      h('header', { class: 's-sheet-head' },
        h('p', { class: 's-kicker' }, `${eventKindLabel(ev.kind)} · ${ev.end_year > ev.year ? `${ev.year}–${ev.end_year}` : ev.year}`),
        h('h2', null, tk(ev, 'label')), h('p', null, tk(ev, 'description')),
        h('div', { class: 's-actions' }, h('button', { type: 'button', class: 'btn btn-quiet', onclick: () => state.set({ year: ev.year }) }, t('sheet.go_to_year', { year: ev.year })))),
      ev.transition ? section(t('sheet.before_after'), h('p', { class: 's-dim' }, tk(ev.transition, 'label')),
        h('div', { class: 's-ba' },
          h('div', null, h('span', { class: 's-dim' }, t('sheet.before')), h('strong', null, tv(ev.transition.before))), h('span', { 'aria-hidden': 'true' }, '→'),
          h('div', null, h('span', { class: 's-dim' }, t('sheet.after')), h('strong', null, tv(ev.transition.after))))) : null,
      section(t('sheet.cause_section'), chain.length
        ? h('ol', { class: 's-rels' }, chain.map((id) => h('li', null, `${model.events.get(id)?.year} · `, link(id))))
        : h('p', { class: 's-dim' }, t('sheet.root_event'))),
      cons.length ? section(t('sheet.consequences'), h('ul', { class: 's-rels' }, cons.map((c) => h('li', null, `${c.year} · `, link(c.id, tk(c, 'label')))))) : null,
      section(t('sheet.concerned'), h('ul', { class: 's-rels' }, ev.subject_ids.map((id) => h('li', null, link(id))))),
      provenanceBlock(ev.provenance),
    ];
  }

  function render() {
    const st = state.get();
    const year = ctx.year();
    clear(root);
    const id = st.sel;
    if (!id) { root.append(h('p', { class: 's-dim' }, t('sheet.empty'))); return; }
    const ev = model.events.get(id);
    const e = model.get(id);
    if (ev) root.append(...eventBody(ev).filter(Boolean));
    else if (e) root.append(...entityBody(e, year).filter(Boolean));
    else root.append(h('p', { class: 's-warn' }, t('sheet.unknown_id', { id })));
  }
  return { render };
}
