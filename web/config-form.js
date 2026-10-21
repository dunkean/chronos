// Settings form for every Config field. Bounds mirror Config::validate in
// crates/chronos-core/src/model.rs; the server stays the authority and its
// error message is shown if it disagrees.

import { h, clear, fmtInt, fmtNum } from './util.js';

const MAX_SAFE = Number.MAX_SAFE_INTEGER;
export const MAX_SITE_YEARS = 150000;

// Contract defaults, used only to fill the form if /api/defaults is unreachable.
export const CONTRACT_DEFAULTS = {
  seed: 42, start_year: 1300, years: 200, site_count: 50, population: 120000,
  world_radius_km: 6371, map_mode: 'stack', culture: 'french', war_rate: 0.35,
  epidemic_rate: 0.30, trade_intensity: 0.60, migration_rate: 0.25, growth_rate: 0.003,
  resource_abundance: 1.0, travel_km_per_day: 25.0, institution_mode: 'mixed',
  // Older servers omit `scenario`; their behaviour is the study scenario.
  scenario: 'pivot',
};

const GROUPS = [
  { id: 'world', title: 'Monde et durée', basic: true },
  { id: 'people', title: 'Peuplement', basic: true },
  { id: 'crises', title: 'Intensité des crises', basic: false },
  { id: 'economy', title: 'Ressources et échanges', basic: false },
  { id: 'space', title: 'Distances et déplacements', basic: false },
  { id: 'power', title: 'Institutions', basic: false },
];

export const FIELDS = [
  { key: 'seed', group: 'world', label: 'Graine', type: 'int', min: 0, max: MAX_SAFE, step: 1,
    help: 'Mêmes graine, réglages et données versionnées : mêmes faits canoniques (seule la durée de génération varie).', dice: true },
  { key: 'scenario', group: 'world', label: 'Scénario', type: 'select', options: [
    ['pivot', 'Démo de l’étude (1356, 1398)'], ['free', 'Histoire libre'],
  ], help: 'Démo de l’étude : l’épidémie de 1356 et la guerre avec sauvetage de 1398 sont imposées, quelles que soient les intensités. Histoire libre : seuls les réglages décident des crises.' },
  { key: 'start_year', group: 'world', label: 'Année de départ', type: 'int', min: -5000, max: 5000, step: 1 },
  { key: 'years', group: 'world', label: 'Durée', unit: 'ans', type: 'int', min: 1, max: 2000, step: 1,
    help: 'Jusqu’à 2 000 ans, dans la limite durée × lieux ≤ 150 000.' },
  { key: 'map_mode', group: 'world', label: 'Carte', type: 'radio', options: [
    ['stack', 'Carte importée', 'Relief, lieux et routes de la fixture préparée (Neural Earth, Magna Urbis).'],
    ['virtual', 'Monde virtuel', 'Repli déterministe autonome, étiqueté synthétique.'],
  ] },
  { key: 'culture', group: 'world', label: 'Culture des noms', type: 'select', options: [
    ['french', 'Française'], ['british', 'Britannique'], ['fantasy', 'Fantasy'],
  ], help: 'Style des lieux virtuels ou sans culture connue. Les lieux importés conservent leur culture ; dates et filiations restent inchangées.' },

  { key: 'site_count', group: 'people', label: 'Nombre de lieux', type: 'int', min: 8, max: 300, step: 1, siteNote: true },
  { key: 'population', group: 'people', label: 'Population initiale totale', unit: 'hab.', type: 'int',
    min: 1000, max: 50000000, step: 1000 },
  { key: 'growth_rate', group: 'people', label: 'Croissance annuelle', type: 'float', min: -0.02, max: 0.03,
    step: 0.0005, slider: true, digits: 4, help: 'Taux de base par an, avant crises.' },
  { key: 'migration_rate', group: 'people', label: 'Mobilité des habitants', type: 'float', min: 0, max: 1,
    step: 0.01, slider: true, help: 'Intensité relative des migrations, de 0 à 1.' },

  { key: 'war_rate', group: 'crises', label: 'Guerres', type: 'float', min: 0, max: 1, step: 0.01, slider: true,
    help: 'Intensité relative, pas une probabilité historique.' },
  { key: 'epidemic_rate', group: 'crises', label: 'Épidémies', type: 'float', min: 0, max: 1, step: 0.01, slider: true },

  { key: 'resource_abundance', group: 'economy', label: 'Abondance des ressources', type: 'float', min: 0.2, max: 5,
    step: 0.05, slider: true, digits: 2, help: 'Multiplie les potentiels initiaux et la capacité vivrière.' },
  { key: 'trade_intensity', group: 'economy', label: 'Intensité du commerce', type: 'float', min: 0, max: 1,
    step: 0.01, slider: true },

  { key: 'world_radius_km', group: 'space', label: 'Rayon de la planète', unit: 'km', type: 'float', min: 100,
    max: 100000, step: 1, digits: 1, radiusNote: true },
  { key: 'travel_km_per_day', group: 'space', label: 'Vitesse de voyage', unit: 'km/jour', type: 'float', min: 1,
    max: 500, step: 0.5, digits: 1 },

  { key: 'institution_mode', group: 'power', label: 'Régime dominant', type: 'select', options: [
    ['mixed', 'Mixte'], ['monarchy', 'Monarchies'], ['council', 'Conseils'],
  ] },
];

// Presets only touch intensity parameters, never the seed.
export const PRESETS = [
  { id: 'pivot', label: 'Démo 1300', help: 'Valeurs par défaut du serveur et scénario de l’étude (1356, 1398).', values: { scenario: 'pivot' } },
  { id: 'calm', label: 'Siècles calmes', help: 'Histoire libre : peu de guerres et d’épidémies, commerce soutenu.',
    values: { scenario: 'free', war_rate: 0.1, epidemic_rate: 0.1, trade_intensity: 0.85, growth_rate: 0.004 } },
  { id: 'crisis', label: 'Temps de crises', help: 'Histoire libre : guerres et épidémies fréquentes, échanges réduits.',
    values: { scenario: 'free', war_rate: 0.75, epidemic_rate: 0.65, trade_intensity: 0.35, migration_rate: 0.45 } },
  { id: 'virtual', label: 'Monde virtuel', help: 'Histoire libre sur repli autonome, noms fantasy, 80 lieux.',
    values: { scenario: 'free', map_mode: 'virtual', culture: 'fantasy', site_count: 80 } },
];

function inputId(key) {
  return `cfg-${key}`;
}

export class ConfigForm {
  constructor(root, { onChange, onPreset }) {
    this.root = root;
    this.onChange = onChange;
    this.onPreset = onPreset;
    this.defaults = { ...CONTRACT_DEFAULTS };
    this.controls = new Map();
    this.mapInfo = null;
    this.render();
  }

  render() {
    clear(this.root);
    const presets = h('fieldset', { class: 'presets' },
      h('legend', null, 'Préréglages'),
      h('div', { class: 'preset-row' }, PRESETS.map((p) => h('button', {
        type: 'button', class: 'btn btn-quiet', title: p.help, 'data-preset': p.id,
        onclick: () => this.applyPreset(p),
      }, p.label))),
      h('p', { class: 'hint' }, 'Un préréglage remplit le formulaire ; rien n’est généré avant « Générer ».'),
    );
    this.root.appendChild(presets);

    const advanced = h('details', { class: 'advanced' }, h('summary', null, 'Réglages avancés'));
    for (const g of GROUPS) {
      const fs = h('fieldset', { class: 'cfg-group' }, h('legend', null, g.title));
      for (const f of FIELDS.filter((x) => x.group === g.id)) fs.appendChild(this.renderField(f));
      (g.basic ? this.root : advanced).appendChild(fs);
    }
    this.root.appendChild(advanced);
    this.loadNote = h('p', { class: 'hint load-note', hidden: true });
    this.validation = h('p', { class: 'cfg-error', role: 'alert', hidden: true });
    this.root.append(this.validation, this.loadNote,
      h('div', { class: 'cfg-footer' },
        h('button', { type: 'button', class: 'btn btn-quiet', onclick: () => this.setValues(this.defaults, true) },
          'Revenir aux valeurs par défaut')));
  }

  renderField(f) {
    const id = inputId(f.key);
    const helpId = f.help ? `${id}-help` : null;
    const wrap = h('div', { class: `field field-${f.type}` });
    if (f.type === 'radio') {
      const group = h('fieldset', { class: 'radio-group' }, h('legend', null, f.label));
      const radios = f.options.map(([value, label, help]) => {
        const r = h('input', { type: 'radio', name: id, value, id: `${id}-${value}` });
        r.addEventListener('change', () => this.changed());
        group.appendChild(h('label', { class: 'radio', for: `${id}-${value}` }, r,
          h('span', null, h('span', { class: 'radio-label' }, label), h('span', { class: 'hint' }, help))));
        return r;
      });
      wrap.appendChild(group);
      this.controls.set(f.key, { f, radios });
      return wrap;
    }
    const labelText = f.unit ? `${f.label} (${f.unit})` : f.label;
    wrap.appendChild(h('label', { for: id }, labelText));
    let input;
    if (f.type === 'select') {
      input = h('select', { id, 'aria-describedby': helpId }, f.options.map(([v, l]) => h('option', { value: v }, l)));
      wrap.appendChild(input);
    } else {
      input = h('input', {
        id, type: 'number', inputmode: f.type === 'int' ? 'numeric' : 'decimal',
        min: f.min, max: f.max, step: f.step, required: true, 'aria-describedby': helpId,
      });
      const row = h('div', { class: 'input-row' }, input);
      if (f.slider) {
        const slider = h('input', { type: 'range', min: f.min, max: f.max, step: f.step, tabindex: '-1', 'aria-hidden': 'true' });
        slider.addEventListener('input', () => { input.value = slider.value; this.changed(); });
        input.addEventListener('input', () => { slider.value = input.value; });
        row.prepend(slider);
        this.controls.set(f.key, { f, input, slider });
      }
      if (f.dice) {
        row.appendChild(h('button', {
          type: 'button', class: 'btn btn-quiet', title: 'Tirer une nouvelle graine au hasard',
          onclick: () => {
            const buf = new Uint32Array(1);
            crypto.getRandomValues(buf);
            input.value = String(buf[0]);
            this.changed();
          },
        }, 'Au hasard'));
      }
      wrap.appendChild(row);
    }
    input.addEventListener('input', () => this.changed());
    input.addEventListener('change', () => this.changed());
    if (f.help) wrap.appendChild(h('p', { class: 'hint', id: helpId }, f.help));
    if (f.radiusNote) {
      this.radiusNote = h('p', { class: 'hint', id: `${id}-note` });
      input.setAttribute('aria-describedby', `${id}-note`);
      wrap.appendChild(this.radiusNote);
    }
    if (f.siteNote) {
      this.siteNote = h('p', { class: 'hint', id: `${id}-note` });
      input.setAttribute('aria-describedby', `${id}-note`);
      wrap.appendChild(this.siteNote);
    }
    if (!this.controls.has(f.key)) this.controls.set(f.key, { f, input });
    return wrap;
  }

  setDefaults(defaults, fromServer) {
    this.defaults = { ...CONTRACT_DEFAULTS, ...(defaults ?? {}) };
    this.loadNote.hidden = fromServer;
    this.loadNote.textContent = fromServer ? '' : 'Valeurs par défaut du contrat : le serveur n’a pas fourni les siennes.';
  }

  setValues(values, notify = false) {
    const v = { ...this.defaults, ...(values ?? {}) };
    for (const [key, c] of this.controls) {
      const val = v[key];
      if (c.radios) {
        for (const r of c.radios) r.checked = r.value === String(val);
      } else {
        c.input.value = val === undefined || val === null ? '' : String(val);
        if (c.slider) c.slider.value = c.input.value;
      }
    }
    this.updateNotes();
    if (notify) this.changed();
  }

  applyPreset(p) {
    // Preset = server defaults + its overrides; the current seed is kept.
    this.setValues({ ...this.defaults, seed: this.read().values.seed, ...p.values });
    this.changed();
    this.onPreset?.(p);
  }

  // Returns { values, errors } with numbers already typed for serde.
  read() {
    const values = {};
    const errors = [];
    for (const [key, c] of this.controls) {
      const f = c.f;
      if (c.radios) {
        const sel = c.radios.find((r) => r.checked);
        values[key] = sel ? sel.value : this.defaults[key];
        continue;
      }
      if (f.type === 'select') {
        values[key] = c.input.value || this.defaults[key];
        continue;
      }
      const raw = c.input.value.trim().replace(',', '.');
      const n = Number(raw);
      let bad = raw === '' || !Number.isFinite(n);
      if (!bad && f.type === 'int' && !Number.isInteger(n)) bad = true;
      if (!bad && (n < f.min || n > f.max)) bad = true;
      // A field disabled because it does not apply (site count on an imported map) never blocks generation.
      const ignored = c.input.disabled && !this.disabled;
      c.input.setAttribute('aria-invalid', bad && !ignored ? 'true' : 'false');
      if (bad) {
        if (!ignored) {
          const bound = (v) => (f.type === 'int' ? fmtInt(v) : fmtNum(v, f.digits ?? 2));
          errors.push(`${f.label} : ${f.type === 'int' ? 'entier' : 'nombre'} entre ${bound(f.min)} et ${bound(f.max)} attendu.`);
        }
        values[key] = this.defaults[key];
      } else {
        values[key] = n;
      }
    }
    if (!errors.length && values.map_mode !== 'stack' && values.years * values.site_count > MAX_SITE_YEARS) {
      errors.push(`Durée × nombre de lieux = ${fmtInt(values.years * values.site_count)} : la limite est ${fmtInt(MAX_SITE_YEARS)}. Réduisez l’un des deux.`);
    }
    return { values, errors };
  }

  showErrors(errors) {
    this.validation.hidden = errors.length === 0;
    this.validation.textContent = errors.join(' ');
  }

  changed() {
    const { errors } = this.read();
    this.showErrors(errors);
    this.updateNotes();
    this.onChange?.();
  }

  // Facts about the loaded world, used to explain what radius and site count do.
  setWorldInfo(info) {
    this.worldInfo = info;
    this.updateNotes();
  }

  updateNotes() {
    const mode = this.controls.get('map_mode')?.radios?.find((r) => r.checked)?.value;
    const info = this.worldInfo;
    const loadedStack = info && info.mapMode === 'stack';
    if (this.radiusNote) {
      let text;
      if (mode === 'stack') {
        text = 'Carte importée : relief et positions restent ceux de la source ; le rayon multiplie seulement les distances et durées de trajet (avertissement dans Sources s’il diffère du rayon source).';
        if (info && Number.isFinite(info.radiusKm)) text += ` Monde chargé : ${fmtInt(info.radiusKm)} km.`;
      } else {
        text = 'Monde virtuel : le rayon fixe l’échelle des distances entre lieux synthétiques.';
      }
      this.radiusNote.textContent = text;
    }
    const sc = this.controls.get('site_count');
    if (sc && this.siteNote) {
      // The imported map fixes its sites; the server ignores this field there.
      sc.input.disabled = this.disabled || mode === 'stack';
      this.siteNote.textContent = mode === 'stack'
        ? `Carte importée : le nombre de lieux est fixé par la fixture, ce réglage est ignoré.${loadedStack ? ` Monde chargé : ${fmtInt(info.siteCount)} lieux.` : ''}`
        : 'Lieux synthétiques placés par le monde virtuel.';
    }
  }

  setDisabled(disabled) {
    this.disabled = disabled;
    this.root.querySelectorAll('input, select, button').forEach((el) => { el.disabled = disabled; });
    this.updateNotes();
  }
}

export function configsDiffer(a, b) {
  if (!a || !b) return true;
  return FIELDS.some((f) => {
    // A field an older server does not report: `scenario` means the study scenario, others cannot be compared.
    const fallback = f.key === 'scenario' ? CONTRACT_DEFAULTS.scenario : undefined;
    const x = a[f.key] ?? fallback;
    const y = b[f.key] ?? fallback;
    if (x === undefined || y === undefined) return false;
    if (typeof x === 'number' && typeof y === 'number') return Math.abs(x - y) > 1e-9 * Math.max(1, Math.abs(x));
    return String(x) !== String(y);
  });
}
