// Shared state across views (§15.1): year, selection, graph centre, mode and
// filters. Kept in the URL fragment so a reload restores it.

const DEFAULTS = {
  year: null,      // null = the view's own year
  sel: '',         // selected id (sheet)
  center: '',      // id at the centre of the graph
  mode: 'social',  // social | kinship
  depth: 1,        // neighbourhood depth
  classes: [],     // retained conduct classes; empty = all
  fam: [],         // retained link families; empty = all
  strength: 0,     // minimum link strength, in percent
};

const LIST_KEYS = ['classes', 'fam'];

export class QueryState {
  constructor(defaults = {}) {
    this.defaults = { ...DEFAULTS, ...defaults };
    this.value = { ...this.defaults };
    this.listeners = new Set();
    this.lastHash = null;
    this.fromHash(location.hash);
    window.addEventListener('hashchange', () => {
      if (location.hash === this.lastHash) return;
      this.fromHash(location.hash);
      this.emit();
    });
  }
  fromHash(hash) {
    const p = new URLSearchParams(hash.replace(/^#/, ''));
    const v = { ...this.defaults };
    const year = Number.parseInt(p.get('year') ?? '', 10);
    if (Number.isFinite(year)) v.year = year;
    for (const k of ['sel', 'center']) if (p.get(k)) v[k] = p.get(k);
    if (p.get('mode') === 'kinship') v.mode = 'kinship';
    const depth = Number.parseInt(p.get('depth') ?? '', 10);
    if (depth >= 1 && depth <= 3) v.depth = depth;
    const strength = Number.parseInt(p.get('strength') ?? '', 10);
    if (strength >= 0 && strength <= 100) v.strength = strength;
    for (const k of LIST_KEYS) if (p.get(k)) v[k] = p.get(k).split(',').filter(Boolean);
    this.value = v;
  }
  toHash() {
    const p = new URLSearchParams();
    const v = this.value;
    if (v.year != null) p.set('year', v.year);
    if (v.sel) p.set('sel', v.sel);
    if (v.center) p.set('center', v.center);
    if (v.mode !== DEFAULTS.mode) p.set('mode', v.mode);
    if (v.depth !== DEFAULTS.depth) p.set('depth', v.depth);
    if (v.strength) p.set('strength', v.strength);
    for (const k of LIST_KEYS) if (v[k].length) p.set(k, v[k].join(','));
    return '#' + p.toString();
  }
  get() { return this.value; }
  set(patch) {
    this.value = { ...this.value, ...patch };
    this.lastHash = this.toHash();
    history.replaceState(null, '', this.lastHash);
    this.emit();
  }
  subscribe(fn) { this.listeners.add(fn); return () => this.listeners.delete(fn); }
  emit() { for (const fn of this.listeners) fn(this.value); }
}

export function toggleIn(list, item) {
  return list.includes(item) ? list.filter((x) => x !== item) : [...list, item];
}
