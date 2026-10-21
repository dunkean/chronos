// Canvas atlas: relief from MapData.elevation_m (numbers, not a picture),
// routes, sites sized by the snapshot population, active-event halos.
// Pan/zoom and hit testing only change the view, never the world.

import { clamp, safeColor, kindInfo, fmtInt } from './util.js';
import { snapIndexAt, eventActiveIn } from './model.js';

// Hypsometric ramp in metres. Sea stops lead to the contract water blue,
// land passes through the contract green.
const RAMP = [
  [-6000, [70, 108, 126]],
  [-2000, [104, 141, 158]],
  [-200, [134, 168, 183]],
  [-1, [153, 182, 194]],
  [0, [172, 192, 168]],
  [150, [146, 171, 150]],
  [500, [117, 148, 134]],
  [1000, [150, 154, 118]],
  [1800, [176, 156, 118]],
  [2800, [164, 152, 140]],
  [4200, [228, 226, 220]],
];
const MISSING = [207, 214, 212];
export const RAMP_STOPS = RAMP;

function rampColor(e) {
  if (e <= RAMP[0][0]) return RAMP[0][1];
  for (let i = 1; i < RAMP.length; i += 1) {
    const [z1, c1] = RAMP[i];
    if (e <= z1) {
      const [z0, c0] = RAMP[i - 1];
      const t = (e - z0) / (z1 - z0 || 1);
      return [c0[0] + (c1[0] - c0[0]) * t, c0[1] + (c1[1] - c0[1]) * t, c0[2] + (c1[2] - c0[2]) * t];
    }
  }
  return RAMP[RAMP.length - 1][1];
}

export function rampCss(e) {
  const c = rampColor(e);
  return `rgb(${c.map((v) => Math.round(v)).join(',')})`;
}

const INK = '#1b3440';
const HALO = 'rgba(231, 238, 236, 0.92)';

// Physical geometry of the map, derived from bounds and radius when they make sense.
export function mapGeometry(map) {
  const geo = { valid: false, aspect: 1, cellWkm: NaN, cellHkm: NaN, widthKm: NaN };
  if (!map) return geo;
  const w = map.width || 1;
  const hgt = map.height || 1;
  geo.aspect = w / hgt;
  const b = map.bounds;
  const R = Number(map.radius_km);
  if (Array.isArray(b) && b.length === 4 && b.every(Number.isFinite) && b[2] > b[0] && b[3] > b[1] && R > 0) {
    const latMid = ((b[1] + b[3]) / 2) * Math.PI / 180;
    const rad = Math.PI / 180;
    geo.cellWkm = ((b[2] - b[0]) / w) * rad * R * Math.cos(latMid);
    geo.cellHkm = ((b[3] - b[1]) / hgt) * rad * R;
    if (geo.cellWkm > 0 && geo.cellHkm > 0) {
      geo.valid = true;
      geo.aspect = clamp((w * geo.cellWkm) / (hgt * geo.cellHkm), 0.2, 5);
      geo.widthKm = w * geo.cellWkm;
    }
  }
  return geo;
}

function buildRelief(map, geo) {
  const w = map.width;
  const hgt = map.height;
  const elev = map.elevation_m;
  const canvas = document.createElement('canvas');
  canvas.width = w;
  canvas.height = hgt;
  const ctx = canvas.getContext('2d');
  const img = ctx.createImageData(w, hgt);
  const at = (x, y) => {
    const v = elev[clamp(y, 0, hgt - 1) * w + clamp(x, 0, w - 1)];
    return Number.isFinite(v) ? v : null;
  };
  const dx = (geo.valid ? geo.cellWkm : 1) * 1000;
  const dy = (geo.valid ? geo.cellHkm : 1) * 1000;
  // Gradients first, so shading can be normalised to the data's own relief.
  const gx = new Float32Array(w * hgt);
  const gy = new Float32Array(w * hgt);
  const mags = [];
  for (let y = 0; y < hgt; y += 1) {
    for (let x = 0; x < w; x += 1) {
      const c = at(x, y);
      if (c === null) continue;
      const l = at(x - 1, y) ?? c;
      const r = at(x + 1, y) ?? c;
      const n = at(x, y - 1) ?? c;
      const s = at(x, y + 1) ?? c;
      const i = y * w + x;
      gx[i] = (r - l) / (2 * dx);
      gy[i] = (n - s) / (2 * dy); // northward gradient; rows run north to south
      if (c >= 0) mags.push(Math.hypot(gx[i], gy[i]));
    }
  }
  mags.sort((a, b) => a - b);
  const p95 = mags.length ? mags[Math.floor(mags.length * 0.95)] : 0;
  const k = p95 > 0 ? 1 / p95 : 0; // p95 land slope renders like a 45° slope
  const L = [-0.5, 0.5, Math.SQRT1_2]; // light from the north-west, 45° high
  const flat = L[2];
  for (let i = 0; i < w * hgt; i += 1) {
    const e = elev[i];
    const o = i * 4;
    if (!Number.isFinite(e)) {
      img.data[o] = MISSING[0]; img.data[o + 1] = MISSING[1]; img.data[o + 2] = MISSING[2]; img.data[o + 3] = 255;
      continue;
    }
    const c = rampColor(e);
    const nx = -gx[i] * k;
    const ny = -gy[i] * k;
    const len = Math.hypot(nx, ny, 1);
    const shade = (nx * L[0] + ny * L[1] + L[2]) / len;
    const strength = e < 0 ? 0.35 : 1;
    const m = clamp(1 + (shade - flat) * 0.9 * strength, 0.55, 1.22);
    img.data[o] = clamp(c[0] * m, 0, 255);
    img.data[o + 1] = clamp(c[1] * m, 0, 255);
    img.data[o + 2] = clamp(c[2] * m, 0, 255);
    img.data[o + 3] = 255;
  }
  ctx.putImageData(img, 0, 0);
  let min = Infinity;
  let max = -Infinity;
  for (const v of elev) if (Number.isFinite(v)) { if (v < min) min = v; if (v > max) max = v; }
  return { canvas, min, max };
}

function niceStep(span, target) {
  const raw = span / target;
  const pow = 10 ** Math.floor(Math.log10(raw));
  for (const m of [1, 2, 2.5, 5, 10]) if (m * pow >= raw) return m * pow;
  return 10 * pow;
}

export function routeStyle(kind) {
  const k = String(kind ?? '').toLowerCase();
  if (/sea|mer|marit|ferry|naval/.test(k)) return { color: '#4d7486', dash: [5, 4], width: 1.4 };
  if (/coarse|corridor|grossier|virtual|link/.test(k)) return { color: '#a98546', dash: [1.5, 3], width: 1.5 };
  if (/path|sentier|minor|footpath/.test(k)) return { color: '#b8955a', dash: [], width: 1 };
  return { color: '#c69c55', dash: [], width: 1.8 };
}

export class MapView {
  constructor(wrap, { onSelect, describeSite }) {
    this.wrap = wrap;
    this.canvas = wrap.querySelector('canvas');
    this.tooltip = wrap.querySelector('.map-tooltip');
    this.ctx = this.canvas.getContext('2d');
    this.onSelect = onSelect;
    this.describeSite = describeSite;
    this.view = { scale: 1, cx: 0.5, cy: 0.5 };
    this.pixelated = false;
    this.frame = { year: 0, selection: null, focusSites: new Set(), eventSites: [], kindFilter: null };
    this.hits = [];
    this.hoverId = null;
    this.world = null;
    this.ix = null;
    this.relief = null;
    this.geo = mapGeometry(null);
    this.pending = false;
    this.bindEvents();
    new ResizeObserver(() => this.requestDraw()).observe(this.wrap);
  }

  setWorld(world, ix) {
    this.world = world;
    this.ix = ix;
    this.geo = mapGeometry(world?.map);
    this.relief = world?.map?.elevation_m ? buildRelief(world.map, this.geo) : null;
    let maxImp = 0;
    for (const e of ix?.events ?? []) if (Number.isFinite(e.importance) && e.importance > maxImp) maxImp = e.importance;
    this.maxImportance = maxImp || 1;
    this.resetView();
  }

  get elevationRange() {
    return this.relief ? { min: this.relief.min, max: this.relief.max } : null;
  }

  setFrame(frame) {
    Object.assign(this.frame, frame);
    this.requestDraw();
  }

  setPixelated(on) {
    this.pixelated = on;
    this.requestDraw();
  }

  resetView() {
    this.view = { scale: 1, cx: 0.5, cy: 0.5 };
    this.updateTouchAction();
    this.requestDraw();
  }

  zoomBy(factor, sx, sy) {
    const { W, H } = this.size();
    const px = sx ?? W / 2;
    const py = sy ?? H / 2;
    const before = this.toMap(px, py);
    this.view.scale = clamp(this.view.scale * factor, 1, 16);
    const after = this.toMap(px, py);
    this.view.cx += before[0] - after[0];
    this.view.cy += before[1] - after[1];
    this.clampView();
    this.updateTouchAction();
    this.requestDraw();
  }

  panBy(dxPx, dyPx) {
    const { bw, bh } = this.base();
    this.view.cx -= dxPx / (bw * this.view.scale);
    this.view.cy -= dyPx / (bh * this.view.scale);
    this.clampView();
    this.requestDraw();
  }

  clampView() {
    const half = 0.5 / this.view.scale;
    this.view.cx = clamp(this.view.cx, half, 1 - half);
    this.view.cy = clamp(this.view.cy, half, 1 - half);
  }

  updateTouchAction() {
    // At full view, let touch scroll the page; once zoomed, drags pan the map.
    this.canvas.style.touchAction = this.view.scale > 1.001 ? 'none' : 'pan-y pinch-zoom';
  }

  size() {
    return { W: this.wrap.clientWidth, H: this.wrap.clientHeight };
  }

  base() {
    const { W, H } = this.size();
    const pad = 14;
    const aw = Math.max(10, W - pad * 2);
    const ah = Math.max(10, H - pad * 2);
    const a = this.geo.aspect || 1;
    let bw = aw;
    let bh = aw / a;
    if (bh > ah) { bh = ah; bw = ah * a; }
    return { W, H, bw, bh };
  }

  toScreen(x, y) {
    const { W, H, bw, bh } = this.base();
    const s = this.view.scale;
    return [W / 2 + (x - this.view.cx) * bw * s, H / 2 + (y - this.view.cy) * bh * s];
  }

  toMap(sx, sy) {
    const { W, H, bw, bh } = this.base();
    const s = this.view.scale;
    return [this.view.cx + (sx - W / 2) / (bw * s), this.view.cy + (sy - H / 2) / (bh * s)];
  }

  requestDraw() {
    if (this.pending) return;
    this.pending = true;
    requestAnimationFrame(() => {
      this.pending = false;
      this.draw();
    });
  }

  draw() {
    const { W, H } = this.size();
    const dpr = window.devicePixelRatio || 1;
    if (this.canvas.width !== Math.round(W * dpr) || this.canvas.height !== Math.round(H * dpr)) {
      this.canvas.width = Math.round(W * dpr);
      this.canvas.height = Math.round(H * dpr);
    }
    const ctx = this.ctx;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, W, H);
    this.hits = [];
    if (!this.world) return;

    const [x0, y0] = this.toScreen(0, 0);
    const [x1, y1] = this.toScreen(1, 1);
    ctx.save();
    ctx.beginPath();
    ctx.rect(x0, y0, x1 - x0, y1 - y0);
    ctx.clip();
    if (this.relief) {
      ctx.imageSmoothingEnabled = !this.pixelated;
      ctx.drawImage(this.relief.canvas, x0, y0, x1 - x0, y1 - y0);
    } else {
      ctx.fillStyle = '#dfe7e4';
      ctx.fillRect(x0, y0, x1 - x0, y1 - y0);
    }
    this.drawGraticule(ctx, x0, y0, x1, y1);
    this.drawRoutes(ctx);
    this.drawSites(ctx);
    ctx.restore();
    ctx.strokeStyle = INK;
    ctx.lineWidth = 1;
    ctx.strokeRect(Math.round(x0) + 0.5, Math.round(y0) + 0.5, Math.round(x1 - x0), Math.round(y1 - y0));
    this.drawScale(ctx, W, H);
  }

  drawGraticule(ctx, x0, y0, x1, y1) {
    const b = this.world.map?.bounds;
    if (!this.geo.valid || !b) return;
    const lonSpan = b[2] - b[0];
    const latSpan = b[3] - b[1];
    const step = niceStep(Math.max(lonSpan, latSpan) / this.view.scale, 4);
    ctx.save();
    ctx.strokeStyle = 'rgba(27, 52, 64, 0.16)';
    ctx.lineWidth = 1;
    ctx.fillStyle = 'rgba(27, 52, 64, 0.62)';
    ctx.font = '11px Bahnschrift, "Segoe UI", system-ui, sans-serif';
    const fmt = (v, pos, neg) => `${Math.abs(+v.toFixed(2))}°${v >= 0 ? pos : neg}`;
    for (let lon = Math.ceil(b[0] / step) * step; lon <= b[2]; lon += step) {
      const [sx] = this.toScreen((lon - b[0]) / lonSpan, 0);
      ctx.beginPath(); ctx.moveTo(sx, y0); ctx.lineTo(sx, y1); ctx.stroke();
      ctx.fillText(fmt(lon, 'E', 'O'), sx + 3, Math.max(y0, 0) + 12);
    }
    for (let lat = Math.ceil(b[1] / step) * step; lat <= b[3]; lat += step) {
      const [, sy] = this.toScreen(0, (b[3] - lat) / latSpan);
      ctx.beginPath(); ctx.moveTo(x0, sy); ctx.lineTo(x1, sy); ctx.stroke();
      ctx.fillText(fmt(lat, 'N', 'S'), Math.max(x0, 0) + 4, sy - 3);
    }
    ctx.restore();
  }

  siteXY(id) {
    const s = this.ix.sites.get(id);
    return s && Number.isFinite(s.x) && Number.isFinite(s.y) ? this.toScreen(s.x, s.y) : null;
  }

  drawRoutes(ctx) {
    const sel = this.frame.selection;
    const selSite = sel?.type === 'site' ? sel.id : null;
    const zoomW = Math.min(2, 0.8 + 0.3 * Math.log2(this.view.scale + 1));
    ctx.save();
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    for (const r of this.world.routes) {
      const pts = [];
      if (Array.isArray(r.path) && r.path.length >= 2) {
        for (const p of r.path) if (Array.isArray(p) && Number.isFinite(p[0]) && Number.isFinite(p[1])) pts.push(this.toScreen(p[0], p[1]));
      }
      if (pts.length < 2) {
        const a = this.siteXY(r.from);
        const b = this.siteXY(r.to);
        if (!a || !b) continue;
        pts.length = 0;
        pts.push(a, b);
      }
      const st = routeStyle(r.kind);
      const isSel = selSite && (r.from === selSite || r.to === selSite);
      ctx.strokeStyle = isSel ? INK : st.color;
      ctx.globalAlpha = selSite && !isSel ? 0.55 : 0.95;
      ctx.lineWidth = (isSel ? st.width + 1.2 : st.width) * zoomW;
      ctx.setLineDash(st.dash);
      ctx.beginPath();
      ctx.moveTo(pts[0][0], pts[0][1]);
      for (let i = 1; i < pts.length; i += 1) ctx.lineTo(pts[i][0], pts[i][1]);
      ctx.stroke();
    }
    ctx.restore();
  }

  drawSites(ctx) {
    const { year, selection, focusSites, eventSites, kindFilter } = this.frame;
    const ix = this.ix;
    const si = snapIndexAt(ix, year);
    const states = si >= 0 ? ix.snapStates[si] : null;
    const maxPop = ix.maxSitePop || 1;
    const zoomR = 1 + 0.22 * Math.log2(this.view.scale);
    const focusing = focusSites.size > 0;

    // Active events at this year, by site, honouring the kind filter.
    const halos = new Map();
    for (const e of ix.events) {
      if (e.year > year) break;
      if (!eventActiveIn(e, year, year)) continue;
      if (kindFilter && !kindFilter.has(e.kind)) continue;
      for (const id of e.site_ids ?? []) {
        const list = halos.get(id);
        if (list) list.push(e); else halos.set(id, [e]);
      }
    }

    const drawn = [];
    for (const s of this.world.sites) {
      if (!Number.isFinite(s.x) || !Number.isFinite(s.y)) continue;
      const st = states?.get(s.id) ?? null;
      const pop = st ? st.population : s.initial_population;
      const polityId = (st && st.polity_id) || s.polity_id;
      const color = safeColor(ix.polities.get(polityId)?.color, '#9aa6a8');
      const r = (2.4 + 8.5 * Math.sqrt(Math.max(0, pop) / maxPop)) * zoomR;
      const [x, y] = this.toScreen(s.x, s.y);
      drawn.push({ s, x, y, r, pop, color, known: !!st });
    }
    drawn.sort((a, b) => a.pop - b.pop);

    // Shared-event links: one event, several sites.
    if (eventSites.length > 1) {
      ctx.save();
      ctx.setLineDash([4, 4]);
      ctx.strokeStyle = this.frame.eventColor ?? INK;
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      let first = true;
      for (const id of eventSites) {
        const p = this.siteXY(id);
        if (!p) continue;
        if (first) ctx.moveTo(p[0], p[1]); else ctx.lineTo(p[0], p[1]);
        first = false;
      }
      ctx.stroke();
      ctx.restore();
    }

    for (const d of drawn) {
      const evs = halos.get(d.s.id);
      if (!evs) continue;
      let rr = d.r + 3;
      for (const e of evs.slice(0, 3)) {
        const imp = clamp((e.importance ?? 0) / this.maxImportance, 0, 1);
        ctx.beginPath();
        ctx.arc(d.x, d.y, rr + 2 + imp * 4, 0, Math.PI * 2);
        ctx.strokeStyle = kindInfo(e.kind).color;
        ctx.globalAlpha = 0.85;
        ctx.lineWidth = 2;
        ctx.stroke();
        rr += 4;
      }
      ctx.globalAlpha = 1;
    }

    const selectedSite = selection?.type === 'site' ? selection.id : null;
    for (const d of drawn) {
      const dim = focusing && !focusSites.has(d.s.id) && d.s.id !== selectedSite;
      ctx.globalAlpha = dim ? 0.35 : 1;
      ctx.beginPath();
      ctx.arc(d.x, d.y, d.r, 0, Math.PI * 2);
      if (d.known && d.pop === 0) {
        // Abandoned or destroyed at this date: hollow mark.
        ctx.setLineDash([2, 2]);
        ctx.strokeStyle = INK;
        ctx.lineWidth = 1.2;
        ctx.stroke();
        ctx.setLineDash([]);
        ctx.beginPath();
        ctx.moveTo(d.x - 3, d.y - 3); ctx.lineTo(d.x + 3, d.y + 3);
        ctx.moveTo(d.x + 3, d.y - 3); ctx.lineTo(d.x - 3, d.y + 3);
        ctx.stroke();
      } else {
        ctx.fillStyle = d.color;
        ctx.fill();
        ctx.strokeStyle = INK;
        ctx.lineWidth = 1;
        ctx.stroke();
      }
      if (ix.capitals.has(d.s.id)) {
        ctx.beginPath();
        ctx.arc(d.x, d.y, d.r + 2.5, 0, Math.PI * 2);
        ctx.strokeStyle = INK;
        ctx.lineWidth = 1.2;
        ctx.stroke();
      }
      ctx.globalAlpha = 1;
      this.hits.push({ id: d.s.id, x: d.x, y: d.y, r: d.r });
    }

    // Selection rings on top.
    const ringed = new Set(focusSites);
    if (selectedSite) ringed.add(selectedSite);
    for (const d of drawn) {
      if (!ringed.has(d.s.id)) continue;
      ctx.beginPath();
      ctx.arc(d.x, d.y, d.r + 5, 0, Math.PI * 2);
      ctx.strokeStyle = HALO;
      ctx.lineWidth = 4;
      ctx.stroke();
      ctx.strokeStyle = INK;
      ctx.lineWidth = d.s.id === selectedSite ? 2.2 : 1.4;
      ctx.stroke();
    }
    if (this.hoverId) {
      const d = drawn.find((q) => q.s.id === this.hoverId);
      if (d) {
        ctx.beginPath();
        ctx.arc(d.x, d.y, d.r + 3, 0, Math.PI * 2);
        ctx.strokeStyle = INK;
        ctx.lineWidth = 1;
        ctx.stroke();
      }
    }
    this.drawLabels(ctx, drawn, ringed);
  }

  drawLabels(ctx, drawn, ringed) {
    const ix = this.ix;
    const rank = (d) => (ringed.has(d.s.id) ? 3e15 : 0) + (ix.capitals.has(d.s.id) ? 1e15 : 0) + d.pop;
    const order = [...drawn].filter((d) => d.s.name).sort((a, b) => rank(b) - rank(a));
    const limit = clamp(Math.round(9 * this.view.scale ** 1.25), 9, 80);
    const placed = [];
    const { W, H } = this.size();
    ctx.save();
    ctx.lineJoin = 'round';
    ctx.textBaseline = 'middle';
    let count = 0;
    for (const d of order) {
      if (count >= limit && !ringed.has(d.s.id)) break;
      const big = ix.capitals.has(d.s.id) || ringed.has(d.s.id);
      ctx.font = big ? '600 13px Georgia, "Times New Roman", serif' : '12px Georgia, "Times New Roman", serif';
      const tw = ctx.measureText(d.s.name).width;
      const th = big ? 15 : 13;
      const gap = d.r + 4;
      const candidates = [
        [d.x + gap, d.y, 'left'], [d.x - gap, d.y, 'right'],
        [d.x, d.y - gap - th / 2, 'center'], [d.x, d.y + gap + th / 2, 'center'],
      ];
      for (const [lx, ly, align] of candidates) {
        const left = align === 'left' ? lx : align === 'right' ? lx - tw : lx - tw / 2;
        const box = [left - 2, ly - th / 2, left + tw + 2, ly + th / 2];
        if (box[0] < 0 || box[2] > W || box[1] < 0 || box[3] > H) continue;
        if (placed.some((p) => box[0] < p[2] && box[2] > p[0] && box[1] < p[3] && box[3] > p[1])) continue;
        // Avoid covering other site marks.
        if (this.hits.some((q) => q.id !== d.s.id && q.x + q.r > box[0] && q.x - q.r < box[2] && q.y + q.r > box[1] && q.y - q.r < box[3])) continue;
        placed.push(box);
        ctx.textAlign = align;
        ctx.strokeStyle = HALO;
        ctx.lineWidth = 3.5;
        ctx.strokeText(d.s.name, lx, ly);
        ctx.fillStyle = INK;
        ctx.fillText(d.s.name, lx, ly);
        count += 1;
        break;
      }
    }
    ctx.restore();
  }

  drawScale(ctx, W, H) {
    if (!this.geo.valid) return;
    const { bw } = this.base();
    const kmPerPx = this.geo.widthKm / (bw * this.view.scale);
    const target = 110 * kmPerPx;
    const km = niceStep(target, 1);
    const px = km / kmPerPx;
    const x = 16;
    const y = H - 18;
    ctx.save();
    ctx.fillStyle = HALO;
    ctx.fillRect(x - 6, y - 16, px + 60, 24);
    ctx.strokeStyle = INK;
    ctx.fillStyle = INK;
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    ctx.moveTo(x, y - 4); ctx.lineTo(x, y); ctx.lineTo(x + px, y); ctx.lineTo(x + px, y - 4);
    ctx.moveTo(x + px / 2, y - 3); ctx.lineTo(x + px / 2, y);
    ctx.stroke();
    ctx.font = '12px Bahnschrift, "Segoe UI", system-ui, sans-serif';
    ctx.textBaseline = 'alphabetic';
    ctx.fillText(`${fmtInt(km)} km`, x + px + 6, y);
    ctx.restore();
  }

  hitTest(sx, sy) {
    let best = null;
    let bestD = Infinity;
    for (const hpt of this.hits) {
      const d = Math.hypot(hpt.x - sx, hpt.y - sy);
      if (d <= Math.max(hpt.r + 5, 9) && d < bestD) { best = hpt; bestD = d; }
    }
    return best;
  }

  bindEvents() {
    const c = this.canvas;
    let down = null;
    const local = (ev) => {
      const r = c.getBoundingClientRect();
      return [ev.clientX - r.left, ev.clientY - r.top];
    };
    c.addEventListener('pointerdown', (ev) => {
      const [x, y] = local(ev);
      down = { x, y, lx: x, ly: y, moved: false, id: ev.pointerId };
      if (this.view.scale > 1.001 || ev.pointerType === 'mouse') c.setPointerCapture(ev.pointerId);
    });
    c.addEventListener('pointermove', (ev) => {
      const [x, y] = local(ev);
      if (down && down.id === ev.pointerId) {
        if (!down.moved && Math.hypot(x - down.x, y - down.y) > 4) down.moved = true;
        if (down.moved && this.view.scale > 1.001) {
          this.panBy(x - down.lx, y - down.ly);
          this.hideTooltip();
        }
        down.lx = x;
        down.ly = y;
        if (down.moved) return;
      }
      this.hover(x, y);
    });
    const end = (ev) => {
      if (!down || down.id !== ev.pointerId) return;
      const wasClick = !down.moved;
      down = null;
      if (ev.type !== 'pointerup' || !wasClick) return;
      const [x, y] = local(ev);
      const hit = this.hitTest(x, y);
      if (hit) this.onSelect({ type: 'site', id: hit.id });
    };
    c.addEventListener('pointerup', end);
    c.addEventListener('pointercancel', end);
    c.addEventListener('pointerleave', () => { if (!down) this.hover(-1e6, -1e6); });
    c.addEventListener('wheel', (ev) => {
      if (!this.world) return;
      ev.preventDefault();
      const [x, y] = local(ev);
      this.zoomBy(Math.exp(-ev.deltaY * (ev.deltaMode === 1 ? 0.05 : 0.0015)), x, y);
    }, { passive: false });
    c.addEventListener('keydown', (ev) => {
      const step = 40;
      const keys = {
        '+': () => this.zoomBy(1.4), '=': () => this.zoomBy(1.4), '-': () => this.zoomBy(1 / 1.4),
        0: () => this.resetView(), ArrowLeft: () => this.panBy(step, 0), ArrowRight: () => this.panBy(-step, 0),
        ArrowUp: () => this.panBy(0, step), ArrowDown: () => this.panBy(0, -step),
      };
      const fn = keys[ev.key];
      if (fn) { ev.preventDefault(); fn(); }
    });
  }

  hover(x, y) {
    const hit = this.hitTest(x, y);
    const id = hit ? hit.id : null;
    if (id !== this.hoverId) {
      this.hoverId = id;
      this.canvas.style.cursor = id ? 'pointer' : this.view.scale > 1.001 ? 'grab' : 'default';
      this.requestDraw();
    }
    if (!hit) { this.hideTooltip(); return; }
    const info = this.describeSite(id);
    if (!info) { this.hideTooltip(); return; }
    const t = this.tooltip;
    t.replaceChildren(...info);
    t.hidden = false;
    const { W, H } = this.size();
    const tw = t.offsetWidth;
    const th = t.offsetHeight;
    let left = hit.x + hit.r + 10;
    let top = hit.y - th / 2;
    if (left + tw > W - 6) left = hit.x - hit.r - 10 - tw;
    top = clamp(top, 6, Math.max(6, H - th - 6));
    t.style.transform = `translate(${Math.round(left)}px, ${Math.round(top)}px)`;
  }

  hideTooltip() {
    if (this.tooltip) this.tooltip.hidden = true;
  }
}
