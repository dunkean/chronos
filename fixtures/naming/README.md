# Frozen Namebench naming pack

Imported from Namebench commit `2820b509169039bbf66c73e96061766c7edf8e06` (2820b50).
Chronos embeds these snapshots; generation never reads the live Namebench repository.

File SHA-256 fingerprints:

- `naming-pack.json`: `2da297ada780757b850c8913ab67af38c4d9e568544639469d0fc9e4ea4ed7d4`
- `naming-manifest.json`: `762d0c442b7b4e842455261205c1dc917eb05fda279a868887c3475b0c600651`
- `naming-conformance.json`: `d63562a1c5b86b07a510d0aadb609df933bfc07b6fe46c742851c6a7b45aa65d`

Generation: `0fbc7da14ae58a8934e78ec543220d93ec5d3046a8fcbf4ab94cac09ce867fe4`.
Rendering: `aca3c2342d2346aeb7216537112f63b8b4fe6c7d3a19d561fbd80009d3073107`.
Only generation enters world identity; rendering belongs to display metadata.

`unicode-display.json` is a separate display-only Unicode 15.1.0 table generated
from Python 3.13 `unicodedata`: canonical decompositions, nonzero combining
classes, canonical two-codepoint compositions (excluding composition exclusions),
and contiguous `str.isalpha()` ranges. Hangul decomposition/composition is
algorithmic in Rust. This supports NFC reference labels and the Python vowel
rules without a new dependency; it is outside canonical generation.
SHA-256: `1a645786a6bae45c6668ddf4e56bba9273d821747ba770c39da1a3ab6a9a7c38`.
