// Minimal i18n for the social v2 page. English is the source language and the
// default; French is a complete translation. Both dictionaries must have the
// same keys (checked by tools/check_i18n.mjs). A key missing from the active
// locale is reported once in the console and falls back to English.

export const LOCALES = ['en', 'fr'];
export const DEFAULT_LOCALE = 'en';
const STORAGE_KEY = 'chronos.social.locale';

let locale = DEFAULT_LOCALE;
let dicts = {};
const warned = new Set();
const listeners = new Set();

function warnOnce(lang, key) {
  const id = `${lang}:${key}`;
  if (warned.has(id)) return;
  warned.add(id);
  console.warn(`[i18n] missing key "${key}" in locale "${lang}"`);
}

export async function initI18n(base = './i18n/') {
  const loaded = await Promise.all(LOCALES.map(async (lang) => {
    const res = await fetch(`${base}${lang}.json`);
    if (!res.ok) throw new Error(`Cannot load ${lang}.json (HTTP ${res.status})`);
    return [lang, await res.json()];
  }));
  dicts = Object.fromEntries(loaded);
  let wanted = DEFAULT_LOCALE;
  try { wanted = localStorage.getItem(STORAGE_KEY) || DEFAULT_LOCALE; } catch { /* storage unavailable */ }
  const fromUrl = new URLSearchParams(location.search).get('lang');
  if (LOCALES.includes(fromUrl)) wanted = fromUrl;
  locale = LOCALES.includes(wanted) ? wanted : DEFAULT_LOCALE;
  document.documentElement.lang = locale;
}

export function getLocale() { return locale; }

export function setLocale(next) {
  if (!LOCALES.includes(next) || next === locale) return;
  locale = next;
  document.documentElement.lang = locale;
  try { localStorage.setItem(STORAGE_KEY, locale); } catch { /* storage unavailable */ }
  for (const fn of listeners) fn(locale);
}

export function onLocaleChange(fn) { listeners.add(fn); }

// Translate a stable key; `{name}` placeholders are filled from `params`.
export function t(key, params) {
  let s = dicts[locale]?.[key];
  if (s === undefined) {
    warnOnce(locale, key);
    s = dicts[DEFAULT_LOCALE]?.[key];
    if (s === undefined) { warnOnce(DEFAULT_LOCALE, key); return key; }
  }
  return params ? s.replace(/\{(\w+)\}/g, (m, k) => (k in params ? String(params[k]) : m)) : s;
}

// A fixture field that carries a key under `<field>_key`.
export function tk(obj, field) {
  const key = obj?.[`${field}_key`];
  return key ? t(key) : (obj?.[field] ?? '');
}

// Value shown for transition before/after: {key} or {name}.
export function tv(v) { return v?.key ? t(v.key) : (v?.name ?? ''); }

// Translate static markup: data-i18n (text) and data-i18n-<attr> for attributes.
export function applyStatic(root = document) {
  for (const el of root.querySelectorAll('[data-i18n]')) el.textContent = t(el.dataset.i18n);
  for (const el of root.querySelectorAll('*')) {
    for (const attr of ['aria-label', 'title', 'placeholder']) {
      const key = el.getAttribute(`data-i18n-${attr}`);
      if (key) el.setAttribute(attr, t(key));
    }
  }
}

// --- Locale-aware number formatting ---------------------------------------

const MINUS = '−';
export function fmtInt(n) {
  return Number.isFinite(n) ? new Intl.NumberFormat(locale, { maximumFractionDigits: 0 }).format(Math.round(n)) : '—';
}
export function fmtNum(n, digits = 1) {
  return Number.isFinite(n) ? new Intl.NumberFormat(locale, { minimumFractionDigits: 0, maximumFractionDigits: digits }).format(n) : '—';
}
export function fmtSigned(n, digits = 0) {
  if (!Number.isFinite(n)) return '—';
  if (n === 0) return '0';
  const body = digits === 0 ? fmtInt(Math.abs(n)) : fmtNum(Math.abs(n), digits);
  return (n > 0 ? '+' : MINUS) + body;
}
export function compareText(a, b) {
  return String(a).localeCompare(String(b), locale, { sensitivity: 'base', numeric: true });
}
