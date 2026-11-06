// Entity sheet: conduct class first, legal status per jurisdiction, provenance
// and precision, relations, conduct history and the explanation of a choice.

import { h, clear } from './util.js';
import { t, tk, tv, fmtInt } from './i18n.js';
import {
  kindLabel, natureLabel, legalLabel, LEGAL_CLASS, personSourceLabel, eventKindLabel, predLabel, inRange,
} from './history-model.js';
import { entitySheet, explainFact } from './data-source.js';

// `kind_key` style parameters are keys themselves.
function resolveParams(params = {}) {
  const out = {};
  for (const [k, v] of Object.entries(params)) out[k.endsWith('_key') ? k.slice(0, -4) : k] = k.endsWith('_key') ? t(String(v)) : v;
  return out;
}

const span = (v) => `${v[0]}–${v[1] ?? '…'}`;

export function createSheet(root, ctx) {
  const { model, state } = ctx;
  const link = (id, text) => h('button', { type: 'button', class: 's-link', onclick: () => state.set({ sel: id }) }, text ?? model.name(id));
  const section = (title, ...kids) => h('section', { class: 's-sec' }, h('h3', null, title), ...kids);
  const explainBox = h('div', { class: 's-explain', role: 'status', 'aria-live': 'polite' });

  // Sentence first, then rule, source and dependencies (section 15.4).
  async function explain(id) {
    clear(explainBox);
    if (ctx.fixture || !ctx.world) { explainBox.append(h('p', { class: 's-dim' }, t('explain.fixture'))); return; }
    explainBox.append(h('p', { class: 's-dim' }, t('explain.loading')));
    try {
      const r = await explainFact(ctx.world, id);
      clear(explainBox);
      const rows = [
        [t('explain.rule'), r.rule_id ?? '\u2014'],
        [t('explain.source'), r.provenance ? `${t(`provenance.status.${r.provenance.status}`)} \u00b7 ${r.provenance.source}` : '\u2014'],
        r.domain_id ? [t('explain.domain'), r.domain_id] : null,
        r.selection ? [t('explain.selection'), t('explain.selection_detail', {
          candidates: r.selection.candidates ?? 0, selected: fmtInt(r.selection.selected_weight ?? 0), total: fmtInt(r.selection.total_weight ?? 0) })] : null,
        r.constraints ? [t('explain.constraints'), t('explain.constraints_detail', {
          min: r.constraints.min_age, max: r.constraints.max_age, term: r.constraints.term_years ?? '\u221e', site: model.siteName(r.constraints.residence_site_id) || String(r.constraints.residence_site_id).split('/').pop() })] : null,
      ];
      explainBox.append(h('p', { class: 's-explain-sentence' }, t(r.sentence_key, resolveParams(r.sentence_params))), dl(rows),
        r.causes?.length ? h('div', null, h('h4', null, t('explain.causes')),
          h('ol', { class: 's-rels' }, r.causes.map((c) => h('li', null, `${c.year} \u00b7 `, link(c.id, t(c.label_key))))))
          : h('p', { class: 's-dim' }, t('explain.no_cause')));
    } catch (err) {
      clear(explainBox);
      explainBox.append(h('p', { class: 's-warn' }, t('explain.failed', { message: err.key ?? err.message })));
    }
  }
  const explainButton = (id) => h('button', { type: 'button', class: 's-mini', onclick: () => explain(id) }, t('explain.button'));
  const dl = (rows) => h('dl', { class: 's-dl' }, rows.filter(Boolean).flatMap(([k, v]) => [h('dt', null, k), h('dd', null, v)]));

  function provenanceBlock(p, extra = []) {
    const status = p?.status ?? 'unavailable';
    return section(t('sheet.provenance'), dl([
      [t('sheet.status'), h('span', { class: `status st-${status}` }, t(`provenance.status.${status}`))],
      [t('sheet.source'), h('span', { title: p?.source ?? '' }, p?.source && p.source.length > 48 ? `${p.source.slice(0, 48)}…` : (p?.source ?? '—'))], [t('sheet.rule'), p?.rule_id ?? '—'], ...extra,
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
        r.cause_event_id ? [h('span', { class: 's-dim' }, ` · ${t('sheet.cause')} `), link(r.cause_event_id)] : null,
        explainButton(r.id)));
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
    return section(t('sheet.events'), h('ul', { class: 's-rels' }, evs.map((ev) => h('li', null, `${ev.year} · `, link(ev.id, model.eventTitle(ev))))));
  }

  function officeBody(e, year) {
    const tn = model.tenureAt(e.id, year);
    const tenures = ctx.view.tenures.filter((x) => x.office_id === e.id).sort((a, b) => a.valid[0] - b.valid[0]);
    const proof = tn?.selection_proof;
    const fits = proof?.fits ? Object.entries(proof.fits).map(([k, v]) => `${t(`fit.${k}`)} ${fmtInt(v)}`).join(' \u00b7 ') : '';
    return [
      section(t('sheet.holder_in', { year }),
        tn ? (tn.mode === 'vacant' || !tn.person_id ? h('p', { class: 's-vacant' }, t('sheet.vacant_note')) : h('p', { class: 's-holder' }, link(tn.person_id)))
          : h('p', { class: 's-dim' }, t('sheet.no_tenure')),
        tn ? dl([[t('sheet.tenure'), span(tn.valid)], [t('sheet.mode'), t(`tenure_mode.${tn.mode}`)],
          [t('sheet.cause'), tn.cause_event_id ? link(tn.cause_event_id) : t('sheet.before_observed')],
          tn.closed_by ? [t('sheet.closed_by'), t(`tenure_end.${tn.closed_by}`)] : null,
          [t('sheet.appointment_rule'), t(`rule.${e.appointment_rule}`)]]) : null,
        tn ? explainButton(tn.tenure_id) : null),
      proof ? section(t('sheet.why'), h('p', null, tk(proof, 'summary')),
        dl([
          [t('sheet.proof.candidates'), fmtInt(proof.candidates)],
          [t('sheet.proof.weight'), t('sheet.proof.weight_detail', { selected: fmtInt(proof.selected_weight), total: fmtInt(proof.total_weight) })],
          [t('sheet.proof.fits'), fits],
          [t('sheet.proof.alignment_fit'), fmtInt(proof.alignment_fit)],
          [t('sheet.proof.year'), String(proof.year)],
        ]), h('p', { class: 's-dim' }, t('sheet.proof.note'))) : null,
      explainBox,
      section(t('sheet.tenures'), h('ul', { class: 's-segs' }, tenures.map((x) => h('li', { class: inRange(x.valid, year) ? 'is-now' : '' },
        `${span(x.valid)} : `, x.mode === 'vacant' || !x.person_id ? t('term.vacant') : link(x.person_id))))),
    ];
  }

  function measureText(m) {
    return `${fmtInt(m.value)} ${t(`unit.${m.unit}`)} — ${t(`precision.${m.precision}`)}, ${t(`coverage.${m.coverage}`)} (${t(`basis.${m.basis}`)}, ${m.at_year})`;
  }

  function entityBody(e, year) {
    const alive = model.exists(e.id, year);
    const head = h('header', { class: 's-sheet-head' },
      h('p', { class: 's-kicker' }, kindLabel(e.kind)),
      h('h2', null, model.name(e.id), e.name_synthetic ? h('span', { class: 'status st-synthetic s-synth', title: t('sheet.synthetic_name') }, t('provenance.status.synthetic')) : null),
      !alive ? h('p', { class: 's-warn' }, t('sheet.not_existing', { year, span: span(e.existence ?? [0, null]) })) : null,
      h('details', { class: 's-tech' }, h('summary', null, t('sheet.tech_details')),
        h('p', { class: 's-id' }, h('span', { class: 's-dim' }, `${t('sheet.tech_id')} `), h('code', null, e.id)),
        e.name_provenance ? h('p', { class: 's-dim' }, t('sheet.name_source', { status: t(`provenance.status.${e.name_provenance.status}`), source: e.name_provenance.source ?? e.name_provenance.source_ref ?? '' })) : null),
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
    if (e.member_count) {
      const live = model.membersAt(e.id, year);
      const realized = live == null ? e.member_count.realized : { ...e.member_count.realized, value: live, at_year: year };
      extra.push([t('sheet.expected_headcount'), measureText(e.member_count.expected)], [t('sheet.realized_headcount'), measureText(realized)]);
    }
    parts.push(provenanceBlock(e.provenance, extra), relationsBlock(e, year), historyBlock(e), eventsBlock(e));
    if (e.kind === 'person' && ctx.world) parts.push(datedRecord(e, year));
    if (e.member_series && ctx.world) {
      parts.push(section(t('sheet.on_map'), h('p', { class: 's-dim' }, t('sheet.on_map_note')),
        h('button', { type: 'button', class: 'btn btn-quiet', onclick: () => document.getElementById('overlay-map')?.scrollIntoView({ block: 'center' }) }, t('sheet.show_map'))));
    }
    parts.push(explainBox);
    return parts;
  }

  // The dated sheet served by GET /entities/{id}: family and affiliations at the year.
  function datedRecord(e, year) {
    const box = h('div', { class: 's-dated' }, h('p', { class: 's-dim' }, t('explain.loading')));
    entitySheet(ctx.world, e.id, year).then((r) => {
      const sh = r.sheet;
      const names = (ids) => (ids.length ? ids.map((id) => link(id)).flatMap((x, i) => (i ? [', ', x] : [x])) : '\u2014');
      clear(box).append(dl([
        [t('sheet.dated.parents'), names(sh.family.parents)], [t('sheet.dated.partners'), names(sh.family.partners)],
        [t('sheet.dated.children'), names(sh.family.children)],
        [t('sheet.dated.memberships'), sh.memberships.length ? sh.memberships.map((m) => model.name(m.group_id)).join(', ') : '\u2014'],
        [t('sheet.dated.offices'), sh.offices.length ? String(sh.offices.length) : '\u2014'],
        [t('sheet.dated.packets'), String(r.packets_needed.length)],
      ]));
    }).catch((err) => { clear(box).append(h('p', { class: 's-warn' }, t('explain.failed', { message: err.key ?? err.message }))); });
    return section(t('sheet.dated', { year }), box);
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
        h('h2', null, model.eventTitle(ev)), ev.description_key ? h('p', null, tk(ev, 'description')) : null,
        h('div', { class: 's-actions' }, h('button', { type: 'button', class: 'btn btn-quiet', onclick: () => state.set({ year: ev.year }) }, t('sheet.go_to_year', { year: ev.year })), explainButton(ev.id))),
      explainBox,
      ev.transition ? section(t('sheet.before_after'), h('p', { class: 's-dim' }, tk(ev.transition, 'label')),
        h('div', { class: 's-ba' },
          h('div', null, h('span', { class: 's-dim' }, t('sheet.before')), h('strong', null, tv(ev.transition.before))), h('span', { 'aria-hidden': 'true' }, '→'),
          h('div', null, h('span', { class: 's-dim' }, t('sheet.after')), h('strong', null, tv(ev.transition.after))))) : null,
      section(t('sheet.cause_section'), chain.length
        ? h('ol', { class: 's-rels' }, chain.map((id) => h('li', null, `${model.events.get(id)?.year} · `, link(id))))
        : h('p', { class: 's-dim' }, t('sheet.root_event'))),
      cons.length ? section(t('sheet.consequences'), h('ul', { class: 's-rels' }, cons.map((c) => h('li', null, `${c.year} · `, link(c.id, model.eventTitle(c)))))) : null,
      section(t('sheet.concerned'), h('ul', { class: 's-rels' }, ev.subject_ids.map((id) => h('li', null, link(id))))),
      provenanceBlock(ev.provenance),
    ];
  }

  function render() {
    const st = state.get();
    const year = ctx.year();
    clear(root);
    clear(explainBox);
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
