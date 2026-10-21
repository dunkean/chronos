"""Copy the Pecking Order Rust crate (manifest, build script, sources, data) into a frozen
snapshot directory and record SHA-256 digests, so blueprint export is reproducible even while
the sibling repository is being edited. The sibling is only read.

Usage: python tools/pecking_order/snapshot.py ../pecking-order output/pecking-order-snapshot
"""
import hashlib
import json
import shutil
import sys
from pathlib import Path

PARTS = ("Cargo.toml", "build.rs", "src", "data", "web")


def main():
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    source, target = Path(sys.argv[1]).resolve(), Path(sys.argv[2])
    if target.exists():
        shutil.rmtree(target)
    target.mkdir(parents=True)
    digests = {}
    for part in PARTS:
        path = source / part
        if not path.exists():
            continue
        files = [path] if path.is_file() else sorted(p for p in path.rglob("*") if p.is_file() and "__pycache__" not in p.parts)
        for file in files:
            relative = file.relative_to(source).as_posix()
            data = file.read_bytes()
            (target / relative).parent.mkdir(parents=True, exist_ok=True)
            (target / relative).write_bytes(data)
            digests[relative] = hashlib.sha256(data).hexdigest()
    tree = hashlib.sha256(json.dumps(digests, sort_keys=True).encode()).hexdigest()
    (target / "SNAPSHOT.json").write_text(json.dumps({"source": "pecking-order", "files": digests, "tree_sha256": tree}, indent=1, sort_keys=True))
    print(f"snapshot {target}: {len(digests)} files, tree {tree}")


if __name__ == "__main__":
    main()
