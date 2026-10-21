// Formatting, French labels and safe DOM construction shared by every view.
// Data from imported worlds is untrusted: text always goes through textContent.

const intFormat = new Intl.NumberFormat('fr-FR', { maximumFractionDigits: 0 });
const MINUS = '−';

export function fmtInt(n) {
  return Number.isFinite(n) ? intFormat.format(Math.round(n)) : '—';
}

export function fmtNum(n, digits = 1) {
  if (!Number.isFinite(n)) return '—';
  return new Intl.NumberFormat('fr-FR', {
    minimumFractionDigits: 0,
    maximumFractionDigits: digits,
  }).format(n);
}

export function fmtSigned(n, digits = 0) {
  if (!Number.isFinite(n)) return '—';
  if (n === 0) return '0';
  const body = digits === 0 ? fmtInt(Math.abs(n)) : fmtNum(Math.abs(n), digits);
  return (n > 0 ? '+' : MINUS) + body;
}

export function fmtPercent(ratio, digits = 1) {
  if (!Number.isFinite(ratio)) return '—';
  const body = fmtNum(Math.abs(ratio * 100), digits);
  return (ratio > 0 ? '+' : ratio < 0 ? MINUS : '') + body + ' %';
}

export function fmtYear(y) {
  if (!Number.isFinite(y)) return '—';
  return y < 0 ? `${MINUS}${Math.abs(y)}` : String(y);
}

export function fmtSpan(year, endYear) {
  if (!Number.isFinite(endYear) || endYear === year) return fmtYear(year);
  return `${fmtYear(year)}–${fmtYear(endYear)}`;
}

export function fmtMs(ms) {
  if (!Number.isFinite(ms)) return '—';
  return ms < 1000 ? `${fmtInt(ms)} ms` : `${fmtNum(ms / 1000, 1)} s`;
}

export function shortId(id, n = 10) {
  const s = String(id ?? '');
  return s.length > n ? s.slice(0, n) + '…' : s;
}

// Identifiers such as "trade_league" become "Trade league"; free text (roles,
// French labels with hyphenated names) is only capitalised.
function humanize(raw) {
  const t = String(raw ?? '').trim();
  const s = /^[a-z0-9_-]+$/.test(t) ? t.replace(/[_-]+/g, ' ') : t;
  return s ? s.charAt(0).toUpperCase() + s.slice(1) : '—';
}

// Event kinds: the contract lists controlled strings but allows more ("etc.").
// Unknown kinds keep their raw name and a neutral slate colour.
export const EVENT_KINDS = {
  foundation: { label: 'Fondation', color: '#5d7f52' },
  trade: { label: 'Commerce', color: '#b8893f' },
  fair: { label: 'Foire', color: '#c69c55' },
  war: { label: 'Guerre', color: '#ac5657' },
  battle: { label: 'Bataille', color: '#8a3c41' },
  destruction: { label: 'Destruction', color: '#6e3438' },
  epidemic: { label: 'Épidémie', color: '#6d4f7a' },
  famine: { label: 'Crise vivrière', color: '#9a6a3a' },
  migration: { label: 'Migration', color: '#4f8a8b' },
  succession: { label: 'Succession', color: '#2e5a6e' },
  vacancy: { label: 'Vacance', color: '#5f7480' },
  reform: { label: 'Réforme', color: '#47708a' },
  local: { label: 'Conflit local', color: '#8b7766' },
  hero: { label: 'Exploit', color: '#a37422' },
  reconstruction: { label: 'Reconstruction', color: '#7d8a4a' },
  discovery: { label: 'Découverte', color: '#4a74a0' },
  peace: { label: 'Paix', color: '#6f9a8a' },
  treaty: { label: 'Traité', color: '#6f9a8a' },
  death: { label: 'Décès', color: '#5b5560' },
  birth: { label: 'Naissance', color: '#7a9a6a' },
};

export function kindInfo(kind) {
  return EVENT_KINDS[kind] ?? { label: humanize(kind), color: '#6b7d86' };
}

const SITE_KINDS = {
  village: 'Village', hamlet: 'Hameau', town: 'Bourg', city: 'Ville',
  capital: 'Capitale', port: 'Port', gateway: 'Site-passerelle',
  road_site: 'Site-passerelle', fort: 'Place forte', castle: 'Château',
  monastery: 'Monastère', abbey: 'Abbaye', market: 'Marché', mine: 'Mine',
};
export const siteKindLabel = (k) => SITE_KINDS[k] ?? humanize(k);

const ORG_KINDS = {
  village_community: 'Communauté villageoise', community: 'Communauté',
  monarchy: 'Monarchie', kingdom: 'Royaume', council: 'Conseil',
  trade_league: 'Ligue marchande', merchant_league: 'Ligue marchande', league: 'Ligue', guild: 'Guilde',
  informal_collective: 'Collectif informel', collective: 'Collectif',
};
export const orgKindLabel = (k) => ORG_KINDS[k] ?? humanize(k);

export const humanLabel = humanize;

// Succession rule identifiers interpreted by the core (closed list); others shown as-is.
const SUCCESSION = {
  hereditary: 'Héréditaire', hereditary_simple: 'Héréditaire simple', dynastic_selection: 'Choix dynastique',
  election: 'Élection', member_vote: 'Vote des membres', vote: 'Vote', appointment: 'Nomination',
  seniority: 'Ancienneté', vacancy: 'Vacance',
};
export const successionLabel = (k) => SUCCESSION[k] ?? humanize(k);

// Provenance status strings are fixed by the contract.
export const SOURCE_STATUS = {
  imported: { label: 'importée', cls: 'st-imported' },
  derived: { label: 'dérivée', cls: 'st-derived' },
  synthetic: { label: 'synthétique', cls: 'st-synthetic' },
  unavailable: { label: 'indisponible', cls: 'st-unavailable' },
};
export function sourceStatus(s) {
  return SOURCE_STATUS[s] ?? { label: String(s ?? 'inconnu'), cls: 'st-unknown' };
}

export function personSourceLabel(source) {
  if (source === 'lineage-lab') return 'importée de Lineage Lab';
  if (source === 'virtual') return 'personne virtuelle';
  return String(source ?? 'inconnue');
}

// --- DOM -----------------------------------------------------------------

export function h(tag, attrs, ...children) {
  const node = document.createElement(tag);
  if (attrs) {
    for (const [k, v] of Object.entries(attrs)) {
      if (v === null || v === undefined || v === false) continue;
      if (k === 'class') node.className = v;
      else if (k === 'text') node.textContent = v;
      else if (k === 'style' && typeof v === 'object') Object.assign(node.style, v);
      else if (k === 'dataset') Object.assign(node.dataset, v);
      else if (k.startsWith('on') && typeof v === 'function') node.addEventListener(k.slice(2), v);
      else if (v === true) node.setAttribute(k, '');
      else node.setAttribute(k, String(v));
    }
  }
  append(node, children);
  return node;
}

function append(node, children) {
  for (const c of children) {
    if (c === null || c === undefined || c === false) continue;
    if (Array.isArray(c)) append(node, c);
    else if (c instanceof Node) node.appendChild(c);
    else node.appendChild(document.createTextNode(String(c)));
  }
}

const SVG_NS = 'http://www.w3.org/2000/svg';
export function svg(tag, attrs, ...children) {
  const node = document.createElementNS(SVG_NS, tag);
  for (const [k, v] of Object.entries(attrs ?? {})) {
    if (v !== null && v !== undefined) node.setAttribute(k, String(v));
  }
  for (const c of children) if (c) node.appendChild(c);
  return node;
}

export function clear(node) {
  while (node.firstChild) node.removeChild(node.firstChild);
  return node;
}

export function swatch(color, extraClass = '') {
  return h('span', { class: `swatch ${extraClass}`.trim(), style: { background: color }, 'aria-hidden': 'true' });
}

export function compareText(a, b) {
  return String(a).localeCompare(String(b), 'fr', { sensitivity: 'base', numeric: true });
}

// Accent- and case-insensitive search key.
export function foldText(s) {
  return String(s ?? '').normalize('NFD').replace(/[\u0300-\u036f]/g, '').toLowerCase();
}

export function clamp(v, lo, hi) {
  return v < lo ? lo : v > hi ? hi : v;
}

// CSS colours from imported data are validated before reaching canvas or style.
export function safeColor(c, fallback = '#8a9a9f') {
  return typeof c === 'string' && /^#([0-9a-f]{3}|[0-9a-f]{6})$/i.test(c.trim()) ? c.trim() : fallback;
}
