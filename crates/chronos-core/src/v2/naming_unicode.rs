//! Frozen Unicode tables for the reference renderer's NFC and vowel handling.
//! Display only: these tables are outside the generation fingerprint.
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::OnceLock;

#[derive(Deserialize)]
struct Tables {
    decomposition: BTreeMap<String, Vec<u32>>,
    combining: BTreeMap<String, u8>,
    composition: BTreeMap<String, u32>,
    letters: Vec<[u32; 2]>,
}
fn tables() -> &'static Tables {
    static TABLES: OnceLock<Tables> = OnceLock::new();
    TABLES.get_or_init(|| {
        serde_json::from_slice(include_bytes!(
            "../../../../fixtures/naming/unicode-display.json"
        ))
        .expect("Unicode display tables")
    })
}
fn combining(ch: char) -> u8 {
    tables()
        .combining
        .get(&u32::from(ch).to_string())
        .copied()
        .unwrap_or(0)
}
fn decompose(ch: char, out: &mut Vec<char>) {
    let n = u32::from(ch);
    if (0xac00..0xac00 + 11172).contains(&n) {
        let s = n - 0xac00;
        out.push(char::from_u32(0x1100 + s / 588).unwrap());
        out.push(char::from_u32(0x1161 + (s % 588) / 28).unwrap());
        if s % 28 != 0 {
            out.push(char::from_u32(0x11a7 + s % 28).unwrap());
        }
    } else if let Some(parts) = tables().decomposition.get(&n.to_string()) {
        for p in parts {
            decompose(char::from_u32(*p).unwrap(), out);
        }
    } else {
        out.push(ch);
    }
}
fn composite(first: char, second: char) -> Option<char> {
    let (a, b) = (u32::from(first), u32::from(second));
    let n = if (0x1100..0x1100 + 19).contains(&a) && (0x1161..0x1161 + 21).contains(&b) {
        Some(0xac00 + ((a - 0x1100) * 21 + b - 0x1161) * 28)
    } else if (0xac00..0xac00 + 11172).contains(&a)
        && (a - 0xac00) % 28 == 0
        && (0x11a8..0x11a7 + 28).contains(&b)
    {
        Some(a + b - 0x11a7)
    } else {
        tables().composition.get(&format!("{a},{b}")).copied()
    };
    n.and_then(char::from_u32)
}
pub(super) fn base(ch: char) -> char {
    let mut out = Vec::new();
    decompose(ch, &mut out);
    out[0]
}
pub(super) fn letter(ch: char) -> bool {
    let n = u32::from(ch);
    let ranges = &tables().letters;
    let index = ranges.partition_point(|r| r[1] < n);
    ranges.get(index).is_some_and(|r| r[0] <= n)
}
pub(super) fn nfc(text: &str) -> String {
    let mut chars = Vec::new();
    for ch in text.chars() {
        let start = chars.len();
        decompose(ch, &mut chars);
        for mut i in start..chars.len() {
            let class = combining(chars[i]);
            while class > 0 && i > 0 && combining(chars[i - 1]) > class {
                chars.swap(i - 1, i);
                i -= 1;
            }
        }
    }
    let mut out = Vec::new();
    let mut starter: Option<usize> = None;
    let mut last_class = 0;
    for ch in chars {
        let class = combining(ch);
        if let Some(index) = starter {
            if last_class == 0 || last_class < class {
                if let Some(c) = composite(out[index], ch) {
                    out[index] = c;
                    continue;
                }
            }
        }
        if class == 0 {
            starter = Some(out.len());
        }
        out.push(ch);
        last_class = class;
    }
    out.into_iter().collect()
}
