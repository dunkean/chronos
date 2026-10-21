// Export the initial Neural Earth / Orogen graph fields on CPU, without the Python
// backend, CUDA or neural refinement.
//
// Mirrors infinite_map/backend/terrain_orogen_stages.py: the same request built by
// `_request(seed, config)` for the maintained "orogen" base profile
// (terrain_generation._defaults('orogen')), then three Node runs of the sibling's
// unmodified native/orogen/pipeline.mjs: relief (skipPost, skipClimate) -> erosion
// (resume retained state) -> climate (resume). Only the CUDA raster projection is
// omitted; Chronos samples the graph itself (tools/export-map).
//
// Usage: node tools/orogen/export_orogen.mjs <infinite_map dir> <output dir> [seed]
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import {spawnSync} from 'node:child_process';

const [root, out, seedArg] = process.argv.slice(2);
if (!root || !out) {
  console.error('usage: node export_orogen.mjs <infinite_map dir> <output dir> [seed]');
  process.exit(2);
}
const seed = Number(seedArg ?? 42);
if (!Number.isInteger(seed) || seed < 0 || seed > 0x7fffffff) throw Error('seed must be a 31-bit integer');
const native = path.join(root, 'native', 'orogen');
const pipeline = path.join(native, 'pipeline.mjs');
fs.mkdirSync(out, {recursive: true});

// terrain_generation.OROGEN_PARAMETERS defaults with orogen_defaults('continents')
// (the 'orogen' base profile uses continental_style='continents': 80 plates, 3 continents, 30 % land).
const config = {
  plate_count: 80, continent_count: 3, land_coverage: 0.30, continent_variety: 0.85,
  motion_strength: 1.0, convergence_threshold: 0.75, detail: 100000, spread: 5.0,
  roughness: 0.4, warp: 0.75, smoothing: 0.1, hydraulic: 0.5, thermal: 0.1, glacial: 0.5,
  sharpening: 0.5, temperature_offset: 0.0, precipitation_offset: 0.0,
};
const climateFile = path.join(native, 'climate-parameters.json');
const climate = JSON.parse(fs.readFileSync(climateFile, 'utf8'));
const climateSettings = Object.fromEntries(Object.entries(climate).map(([k, v]) => [k.replace(/^orogen_/, ''), v.default]));
// terrain_orogen_stages._request: seeds <= 0x7fffffff are used directly.
const base = {
  seed, N: config.detail, P: config.plate_count, jitter: 0.75, nMag: config.roughness,
  numContinents: config.continent_count, continentSizeVariety: config.continent_variety,
  landCoverage: config.land_coverage, smoothing: config.smoothing, terrainWarp: config.warp,
  hydraulicErosion: config.hydraulic, thermalErosion: config.thermal, glacialErosion: config.glacial,
  ridgeSharpening: config.sharpening, temperatureOffset: config.temperature_offset,
  precipitationOffset: config.precipitation_offset, threshold: config.convergence_threshold,
  motion: config.motion_strength, spread: config.spread, climateSettings,
};

const sha = (file) => crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
function run(stage, extra) {
  const dir = path.join(out, stage);
  fs.mkdirSync(dir, {recursive: true});
  const request = {...base, ...extra};
  const requestPath = path.join(dir, 'request.json');
  fs.writeFileSync(requestPath, JSON.stringify(request));
  const started = Date.now();
  const result = spawnSync(process.execPath, [pipeline, requestPath, dir], {encoding: 'utf8', maxBuffer: 1 << 26});
  if (result.status !== 0) throw Error(`Orogen ${stage} failed: ${result.stderr?.slice(-4000)}`);
  console.log(`${stage}: ${Date.now() - started} ms`);
  return dir;
}

const relief = run('relief', {skipPost: true, skipClimate: true, snapshotOutput: path.join(out, 'relief', 'snapshot.bin')});
const erosion = run('erosion', {resumeFile: path.join(relief, 'snapshot.bin'), resumeCommand: 'erosion',
  legacyHeightConvention: false, imported: false, skipPost: false, skipClimate: true,
  snapshotOutput: path.join(out, 'erosion', 'snapshot.bin')});
const climateDir = run('climate', {resumeFile: path.join(erosion, 'snapshot.bin'), resumeCommand: 'climate',
  legacyHeightConvention: false, imported: false, skipPost: false, skipClimate: false});

// Source identity: every file of native/orogen that the run could load (vendor + pipeline).
const sources = {};
for (const file of fs.readdirSync(native, {recursive: true}).map(String).sort()) {
  const full = path.join(native, file);
  if (fs.statSync(full).isFile() && !file.includes('__pycache__') && !file.startsWith('gpu')) sources[file.replaceAll('\\', '/')] = sha(full);
}
const manifest = {
  generator: 'neural-earth-orogen-initial (native/orogen/pipeline.mjs, staged relief->erosion->climate, CPU)',
  backend_mirror: 'infinite_map/backend/terrain_orogen_stages.py _request + _native; base profile "orogen"',
  seed, config, climate_settings: climateSettings, node_version: process.version,
  hypsometry: 'coastal-slope-v1: ocean 10000*e; land 6000*t^4*(5-4*t)+1000*t*(1-t)^4',
  stages: Object.fromEntries([['relief', relief], ['erosion', erosion], ['climate', climateDir]].map(([k, d]) => [k, {
    graph_sha256: sha(path.join(d, 'graph.json')), fields_sha256: sha(path.join(d, 'fields.f32')),
  }])),
  final: {graph: 'climate/graph.json', fields: 'climate/fields.f32',
    // fields.f32 concatenates these arrays in this order (graph.json object order).
    layout: Object.entries(JSON.parse(fs.readFileSync(path.join(climateDir, 'graph.json'), 'utf8')).fields)},
  sources,
};
fs.writeFileSync(path.join(out, 'manifest.json'), JSON.stringify(manifest, null, 1));
console.log(`wrote ${path.join(out, 'manifest.json')}`);
