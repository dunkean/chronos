# Rebuild fixtures/stack.json from the sibling projects (data preparation only; Chronos itself
# never runs this). Run from the Chronos repository root:  powershell -File tools/prepare.ps1
# Requirements: Node.js, Rust (cargo), the siblings ../infinite_map, ../city_generator,
# ../namebench (with .venv and data/names.db), ../pecking-order, ../lineage-lab (with .venv).
# Siblings are only read. Outputs go to output/ (ignored) and fixtures/.
$ErrorActionPreference = 'Stop'
function Step([string]$label, [scriptblock]$command) {
    Write-Host "== $label"
    & $command
    if ($LASTEXITCODE -ne 0) { throw "$label failed ($LASTEXITCODE)" }
}
Step 'Neural Earth / Orogen fields (CPU)' { node tools/orogen/export_orogen.mjs ../infinite_map output/orogen-seed42 42 }
Step 'Build map exporter' { cargo build --release --manifest-path tools/export-map/Cargo.toml --target-dir output/target-tools }
Step 'Region, Magna Urbis place and roads' { output/target-tools/release/chronos-export-map.exe output/orogen-seed42 output/map/map-export.json }
Step 'Namebench pools' { ../namebench/.venv/Scripts/python.exe tools/namebench/export_names.py ../namebench output/names/names.json }
Step 'Pecking Order snapshot' { python tools/pecking_order/snapshot.py ../pecking-order output/pecking-order-snapshot }
Step 'Build blueprint exporter' { cargo build --release --manifest-path tools/export-blueprints/Cargo.toml --target-dir output/target-tools }
Step 'Pecking Order blueprints' { output/target-tools/release/chronos-export-blueprints.exe output/blueprints/blueprints.json }
Step 'Lineage Lab people' { ../lineage-lab/.venv/Scripts/python.exe tools/lineage/export_people.py ../lineage-lab output/map/map-export.json output/people/people.json }
Step 'Assemble fixture' { python tools/assemble_fixture.py }
Step 'Validate fixture' { python tools/validate_fixture.py }
