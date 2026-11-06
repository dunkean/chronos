// Only access point to the data of the social page.
// Default: the /api/v2 host (S5). `?fixture=1` keeps the hand-written HistoryView of
// web/fixtures/ available, and only then does the page show the fixture banner.
//
// A request whose packets are not ready answers 202 with a job: this module polls it and
// reports a non-canonical progress, then returns the same object a ready request returns.

export const API = '/api/v2';
export const FIXTURE_URL = './fixtures/social-view.demo.json';
export const DEFAULT_WINDOW = 25;

const params = () => new URLSearchParams(location.search);
export const isFixtureMode = () => params().get('fixture') === '1';
export const requestedSeed = () => {
  const seed = Number.parseInt(params().get('seed') ?? '', 10);
  return Number.isFinite(seed) ? seed : 11;
};

export class ApiError extends Error {
  constructor(status, body) {
    const e = body?.error ?? {};
    super(e.key ?? `HTTP ${status}`);
    this.status = status;
    this.key = e.key ?? null;
    this.params = e.params ?? {};
    this.owner = e.owner ?? null;
  }
}

async function request(method, path, body) {
  const res = await fetch(`${API}${path}`, {
    method,
    headers: { Accept: 'application/json', ...(body === undefined ? {} : { 'Content-Type': 'application/json' }) },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  let data = null;
  try { data = await res.json(); } catch { /* empty body */ }
  if (res.status === 202) return { pending: data };
  if (!res.ok) throw new ApiError(res.status, data);
  return { data };
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// Polls a job until it is done; `progress` receives {stage, done, total, elapsed_ms}.
async function follow(job, progress) {
  let current = job;
  for (;;) {
    progress?.(current.progress, current);
    if (current.state === 'done') return current.result;
    if (current.state === 'failed') throw new ApiError(500, { error: current.error });
    await sleep(current.progress?.elapsed_ms > 5000 ? 1000 : 400);
    const res = await request('GET', `/jobs/${encodeURIComponent(current.job_id)}`);
    current = res.data;
  }
}

async function call(method, path, body, progress) {
  const res = await request(method, path, body);
  if (res.pending) return follow(res.pending, progress);
  return res.data;
}

export async function openWorld(seed = requestedSeed()) {
  return call('POST', '/worlds', { preset: 'region-demo', seed });
}

const enc = encodeURIComponent;
export const queryView = (world, body, progress) => call('POST', `/worlds/${enc(world.canon_id)}/query`, body, progress);
export const graphAround = (world, body, progress) => call('POST', `/worlds/${enc(world.canon_id)}/graph`, body, progress);
export const explainFact = (world, id, progress) => call('GET', `/worlds/${enc(world.canon_id)}/explain/${enc(id)}`, undefined, progress);
export const entitySheet = (world, id, year, progress) => call('GET', `/worlds/${enc(world.canon_id)}/entities/${enc(id)}?at_year=${year}`, undefined, progress);
export const exportUrl = (world, materialized) => `${API}/worlds/${enc(world.canon_id)}/export${materialized ? '?materialized=1' : ''}`;

// Window of facts around a year: the whole timeline stays reachable through the slider.
export function windowAround(world, year, half = DEFAULT_WINDOW) {
  const t = world.timeline;
  const start = Math.max(t.start_year, year - half);
  const end = Math.min(t.end_year, year + half + 1);
  return { start_year: start, end_year: end };
}

export function siteQuery(world, siteId, year, extra = {}) {
  return {
    scope: { site_id: siteId },
    at_year: year,
    period: windowAround(world, year),
    include: ['offices', 'organizations', 'people', 'events', 'measures', 'overlay'],
    detail: 'local',
    limits: { people: 120, relations: 600, groups: 40 },
    ...extra,
  };
}

export async function loadHistoryView(url = FIXTURE_URL) {
  const res = await fetch(url, { headers: { Accept: 'application/json' } });
  if (!res.ok) throw new Error(`Lecture de ${url} impossible (HTTP ${res.status}).`);
  const view = await res.json();
  if (!view || typeof view !== 'object' || !Array.isArray(view.entities)) {
    throw new Error('Format HistoryView invalide : « entities » manquant.');
  }
  // The fixture predates `timeline`: its period is its whole span.
  view.timeline ??= view.period;
  return view;
}
