// Year rail drawn under the map: event counts per year stacked by kind,
// multi-year events as bands, total population as a thin line, and the
// cursor of the displayed year. Keyboard control lives on the range input.

import { clamp, kindInfo, fmtInt, fmtYear, fmtSpan } from './util.js';
import { eventEnd } from './model.js';

const INK = '#1b3440';
// Half the range-input thumb width, so canvas years line up with the slider.
export const RAIL_PAD = 9;
const LANES = 4;
const LANE_H = 6;
const LANE_GAP = 2;

export class Timeline {
  constructor(wrap, { onYear, onSelectEvent }) {
    this.wrap = wrap;
    this.canvas = wrap.querySelector('canvas');
    this.tip = wrap.querySelector('.tl-tip');
    this.ctx = this.canvas.getContext('2d');
    this.onYear = onYear;
    this.onSelectEvent = onSelectEvent;
    this.ix = null;
    this.events = [];
    this.year = 0;
    this.selected = null;
    this.spans = [];
    this.pending = false;
    this.bind();
    new ResizeObserver(() => this.requestDraw()).observe(this.wrap);
  }

  setData(ix, events) {
    this.ix = ix;
    this.events = events;
    this.layoutSpans();
    this.requestDraw();
  }

  setYear(year) {
    this.year = year;
    this.requestDraw();
  }

  setSelected(event) {
    this.selected = event;
    this.requestDraw();
  }

  layoutSpans() {
    this.spans = [];
    const long = this.events.filter((e) => eventEnd(e) > e.year)
      .sort((a, b) => ((b.importance ?? 0) - (a.importance ?? 0)) || (a.year - b.year));
    const lanes = Array.from({ length: LANES }, () => []);
    for (const e of long) {
      const end = eventEnd(e);
      const lane = lanes.findIndex((l) => l.every((o) => end < o.year || e.year > eventEnd(o)));
      if (lane < 0) continue;
      lanes[lane].push(e);
      this.spans.push({ e, lane });
    }
  }

  geom() {
    const W = this.wrap.clientWidth;
    const H = this.wrap.clientHeight;
    const lo = this.ix?.minYear ?? 0;
    const hi = this.ix?.maxYear ?? 1;
    const plotW = Math.max(1, W - RAIL_PAD * 2);
    const span = Math.max(1, hi - lo);
    const x = (y) => RAIL_PAD + ((y - lo) / span) * plotW;
    const yearAt = (px) => Math.round(lo + clamp((px - RAIL_PAD) / plotW, 0, 1) * span);
    const top = 4;
    const lanesH = LANES * (LANE_H + LANE_GAP);
    const histTop = top + lanesH + 4;
    const histBottom = H - 18;
    return { W, H, lo, hi, plotW, span, x, yearAt, top, histTop, histBottom };
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
    const g = this.geom();
    const dpr = window.devicePixelRatio || 1;
    if (this.canvas.width !== Math.round(g.W * dpr) || this.canvas.height !== Math.round(g.H * dpr)) {
      this.canvas.width = Math.round(g.W * dpr);
      this.canvas.height = Math.round(g.H * dpr);
    }
    const ctx = this.ctx;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, g.W, g.H);
    if (!this.ix) return;

    // Selected event period.
    if (this.selected) {
      const a = g.x(this.selected.year);
      const b = g.x(eventEnd(this.selected) + 1);
      ctx.fillStyle = 'rgba(27, 52, 64, 0.09)';
      ctx.fillRect(a, 0, Math.max(2, b - a), g.histBottom);
    }

    this.drawHistogram(ctx, g);
    this.drawPopulation(ctx, g);
    this.drawSpans(ctx, g);
    this.drawAxis(ctx, g);

    const cx = Math.round(g.x(this.year)) + 0.5;
    ctx.strokeStyle = INK;
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.moveTo(cx, 0);
    ctx.lineTo(cx, g.histBottom + 4);
    ctx.stroke();
    ctx.fillStyle = INK;
    ctx.beginPath();
    ctx.moveTo(cx - 5, 0); ctx.lineTo(cx + 5, 0); ctx.lineTo(cx, 6);
    ctx.fill();
  }

  bins(g) {
    const years = g.hi - g.lo + 1;
    const size = Math.max(1, Math.ceil(years / Math.max(1, g.plotW / 3)));
    const count = Math.ceil(years / size);
    const kinds = this.ix.kinds;
    const kindIndex = new Map(kinds.map((k, i) => [k, i]));
    const counts = Array.from({ length: count }, () => new Uint32Array(kinds.length));
    for (const e of this.events) {
      const b = Math.floor((e.year - g.lo) / size);
      if (b < 0 || b >= count) continue;
      counts[b][kindIndex.get(e.kind)] += 1;
    }
    return { size, count, counts, kinds };
  }

  drawHistogram(ctx, g) {
    const { size, counts, kinds } = this.bins(g);
    this.binSize = size;
    let max = 1;
    for (const c of counts) {
      let t = 0;
      for (const v of c) t += v;
      if (t > max) max = t;
    }
    const histH = g.histBottom - g.histTop;
    const unit = histH / max;
    counts.forEach((c, b) => {
      const y0 = g.lo + b * size;
      const xa = g.x(y0);
      const xb = g.x(y0 + size);
      const w = Math.max(1, xb - xa - (xb - xa > 3 ? 1 : 0));
      let y = g.histBottom;
      for (let k = 0; k < kinds.length; k += 1) {
        if (!c[k]) continue;
        const hgt = c[k] * unit;
        ctx.fillStyle = kindInfo(kinds[k]).color;
        ctx.fillRect(xa, y - hgt, w, hgt);
        y -= hgt;
      }
    });
    ctx.fillStyle = 'rgba(27, 52, 64, 0.35)';
    ctx.fillRect(RAIL_PAD, g.histBottom, g.plotW, 1);
  }

  drawPopulation(ctx, g) {
    const ys = this.ix.snapYears;
    const pop = this.ix.totalPop;
    if (ys.length < 2) return;
    let max = 0;
    for (const v of pop) if (v > max) max = v;
    if (!max) return;
    const top = g.histTop;
    const hgt = g.histBottom - g.histTop;
    ctx.save();
    ctx.strokeStyle = 'rgba(27, 52, 64, 0.7)';
    ctx.lineWidth = 1.25;
    ctx.beginPath();
    ys.forEach((yr, i) => {
      const px = g.x(yr);
      const py = top + hgt * (1 - pop[i] / max) * 0.92 + hgt * 0.04;
      if (i === 0) ctx.moveTo(px, py); else ctx.lineTo(px, py);
    });
    ctx.stroke();
    ctx.restore();
  }

  drawSpans(ctx, g) {
    this.spanHits = [];
    for (const { e, lane } of this.spans) {
      const a = g.x(e.year);
      const b = g.x(eventEnd(e) + 1);
      const y = g.top + lane * (LANE_H + LANE_GAP);
      const w = Math.max(3, b - a);
      ctx.fillStyle = kindInfo(e.kind).color;
      ctx.globalAlpha = this.selected && this.selected.id !== e.id ? 0.75 : 1;
      ctx.fillRect(a, y, w, LANE_H);
      if (this.selected && this.selected.id === e.id) {
        ctx.strokeStyle = INK;
        ctx.lineWidth = 1.5;
        ctx.strokeRect(a - 0.5, y - 0.5, w + 1, LANE_H + 1);
      }
      ctx.globalAlpha = 1;
      this.spanHits.push({ e, x0: a, x1: a + w, y0: y - 1, y1: y + LANE_H + 1 });
    }
  }

  drawAxis(ctx, g) {
    const minGap = 56;
    const steps = [1, 2, 5, 10, 20, 25, 50, 100, 200, 250, 500, 1000];
    const pxPerYear = g.plotW / g.span;
    const step = steps.find((s) => s * pxPerYear >= minGap) ?? 1000;
    ctx.fillStyle = INK;
    ctx.font = '12px Georgia, "Times New Roman", serif';
    ctx.textBaseline = 'alphabetic';
    ctx.textAlign = 'center';
    for (let y = Math.ceil(g.lo / step) * step; y <= g.hi; y += step) {
      const px = g.x(y);
      ctx.fillRect(Math.round(px), g.histBottom, 1, 4);
      const tw = ctx.measureText(fmtYear(y)).width;
      if (px - tw / 2 < 0 || px + tw / 2 > g.W) continue;
      ctx.fillText(fmtYear(y), px, g.H - 3);
    }
  }

  spanAt(px, py) {
    return (this.spanHits ?? []).find((s) => px >= s.x0 - 2 && px <= s.x1 + 2 && py >= s.y0 && py <= s.y1) ?? null;
  }

  bind() {
    const c = this.canvas;
    let scrubbing = null;
    const local = (ev) => {
      const r = c.getBoundingClientRect();
      return [ev.clientX - r.left, ev.clientY - r.top];
    };
    c.addEventListener('pointerdown', (ev) => {
      if (!this.ix) return;
      const [x, y] = local(ev);
      const span = this.spanAt(x, y);
      if (span) {
        this.onSelectEvent(span.e);
        return;
      }
      scrubbing = ev.pointerId;
      c.setPointerCapture(ev.pointerId);
      this.onYear(this.geom().yearAt(x));
    });
    c.addEventListener('pointermove', (ev) => {
      if (!this.ix) return;
      const [x, y] = local(ev);
      if (scrubbing === ev.pointerId) {
        this.onYear(this.geom().yearAt(x));
      }
      this.showTip(x, y);
    });
    const stop = () => { scrubbing = null; };
    c.addEventListener('pointerup', stop);
    c.addEventListener('pointercancel', stop);
    c.addEventListener('pointerleave', () => { this.tip.hidden = true; });
  }

  showTip(x, y) {
    const g = this.geom();
    const span = this.spanAt(x, y);
    let text;
    if (span) {
      text = `${fmtSpan(span.e.year, eventEnd(span.e))} : ${span.e.title}`;
      this.canvas.style.cursor = 'pointer';
    } else {
      const yr = g.yearAt(x);
      const size = this.binSize ?? 1;
      const from = g.lo + Math.floor((yr - g.lo) / size) * size;
      const n = this.events.reduce((acc, e) => acc + (e.year >= from && e.year < from + size ? 1 : 0), 0);
      const label = size > 1 ? `${fmtYear(from)}–${fmtYear(from + size - 1)}` : fmtYear(yr);
      text = `${label} : ${n ? `${fmtInt(n)} événement${n > 1 ? 's' : ''}` : 'aucun événement'}`;
      this.canvas.style.cursor = 'ew-resize';
    }
    this.tip.textContent = text;
    this.tip.hidden = false;
    const tw = this.tip.offsetWidth;
    const left = clamp(x - tw / 2, 0, Math.max(0, g.W - tw));
    this.tip.style.transform = `translate(${Math.round(left)}px, 0)`;
  }
}
