//! Namebench naming-1 evaluator. Expansion is canonical; rendering is a pure view.
use super::ids::random_semantics_v2::uniform_below;
use super::model::{ErrorCode, HistoryError, NameTree};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

pub const PACK_BYTES: &[u8] = include_bytes!("../../../../fixtures/naming/naming-pack.json");
pub const MANIFEST_BYTES: &[u8] =
    include_bytes!("../../../../fixtures/naming/naming-manifest.json");
pub const NAMING_RULE: &str = "chronos-namebench-1";
const TONES: [&str; 5] = ["lawless", "crooked", "ordinary", "upright", "saintly"];

fn invalid(key: &str) -> HistoryError {
    HistoryError::new(ErrorCode::InvalidCanonicalValue, key)
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Pack {
    pub concepts: BTreeMap<String, Concept>,
    pub cultures: BTreeMap<String, Culture>,
    pub patterns: Vec<Pattern>,
    #[serde(default)]
    pub exclusions: Vec<Exclusion>,
    pub languages: BTreeMap<String, Value>,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Concept {
    pub id: String,
    pub class: String,
    #[serde(default)]
    pub tags: BTreeSet<String>,
    #[serde(default)]
    pub stems: BTreeSet<String>,
    pub forms: BTreeMap<String, Value>,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Culture {
    pub palette: BTreeSet<String>,
    pub language: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Pattern {
    pub id: String,
    pub kind: String,
    pub shape: String,
    pub weight: usize,
    #[serde(default)]
    pub requires: Vec<String>,
    pub tones: Option<BTreeSet<String>>,
    pub slots: Vec<Slot>,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Slot {
    pub name: String,
    #[serde(rename = "type")]
    pub part_type: String,
    pub class: Option<String>,
    #[serde(default)]
    pub tags_any: BTreeSet<String>,
    #[serde(default)]
    pub palette: bool,
    #[serde(default)]
    pub activity_filter: bool,
    #[serde(rename = "ref")]
    pub reference: Option<String>,
    pub fits: Option<String>,
    pub pairs_with: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Exclusion {
    pub pattern: String,
    pub parts: BTreeMap<String, String>,
}
#[derive(Clone, Debug, Default, Deserialize)]
pub struct NameContext {
    #[serde(default)]
    pub refs: BTreeMap<String, String>,
    pub activity: Option<String>,
    pub tone: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
struct Part {
    #[serde(rename = "type")]
    part_type: String,
    id: String,
}
/// Embedded, immutable snapshot. A malformed or absent injected pack can use the
/// declared social fallback; a broken embedded snapshot is a build-data error.
pub fn frozen_pack() -> &'static Pack {
    static PACK: OnceLock<Pack> = OnceLock::new();
    PACK.get_or_init(|| serde_json::from_slice(PACK_BYTES).expect("frozen Namebench pack"))
}
pub fn generation_fingerprint() -> &'static str {
    static VALUE: OnceLock<Value> = OnceLock::new();
    VALUE.get_or_init(|| serde_json::from_slice(MANIFEST_BYTES).expect("naming manifest"))
        ["fingerprints"]["generation"]
        .as_str()
        .expect("generation fingerprint")
}
pub fn rendering_fingerprint() -> &'static str {
    static VALUE: OnceLock<Value> = OnceLock::new();
    VALUE.get_or_init(|| serde_json::from_slice(MANIFEST_BYTES).expect("naming manifest"))
        ["fingerprints"]["rendering"]
        .as_str()
        .expect("rendering fingerprint")
}
fn candidates<'a>(
    pack: &'a Pack,
    slot: &Slot,
    palette: &BTreeSet<String>,
    context: &NameContext,
    tone: &str,
) -> Vec<&'a Concept> {
    let palette_tags: Vec<_> = palette.iter().map(|p| format!("palette:{p}")).collect();
    let activity_tag = format!("activity:{}", context.activity.as_deref().unwrap_or("None"));
    let tone_tag = format!("tone:{tone}");
    pack.concepts
        .values()
        .filter(|c| {
            slot.class.as_ref() == Some(&c.class)
                && (slot.tags_any.is_empty() || !c.tags.is_disjoint(&slot.tags_any))
                && (!slot.palette || palette_tags.iter().any(|p| c.tags.contains(p)))
                && (!slot.activity_filter
                    || context.activity.is_some() && c.tags.contains(&activity_tag))
                && (!c.tags.iter().any(|t| t.starts_with("tone:")) || c.tags.contains(&tone_tag))
        })
        .collect()
}
fn narrow<'a>(
    pack: &Pack,
    cands: Vec<&'a Concept>,
    part: Option<&Part>,
    part_prefix: &str,
    prefix: &str,
) -> Vec<&'a Concept> {
    let Some(part) = part.filter(|p| p.part_type == "concept") else {
        return cands;
    };
    let wanted: BTreeSet<_> = pack.concepts[&part.id]
        .tags
        .iter()
        .filter_map(|t| t.strip_prefix(part_prefix))
        .collect();
    if wanted.is_empty() {
        return cands;
    }
    let kept: Vec<_> = cands
        .iter()
        .copied()
        .filter(|c| {
            let tags: BTreeSet<_> = c
                .tags
                .iter()
                .filter_map(|t| t.strip_prefix(prefix))
                .collect();
            tags.is_empty() || !tags.is_disjoint(&wanted)
        })
        .collect();
    if kept.is_empty() {
        cands
    } else {
        kept
    }
}
/// Nearest dotted kind with patterns, then eligibility at that level only.
/// `choose` receives exactly the Python semantic paths and integer pool sizes.
pub fn expand(
    pack: &Pack,
    kind: &str,
    culture: &str,
    context: &NameContext,
    mut choose: impl FnMut(&str, usize) -> Result<usize, HistoryError>,
) -> Result<Option<NameTree>, HistoryError> {
    let culture_profile = pack
        .cultures
        .get(culture)
        .ok_or_else(|| invalid("naming.unknown_culture"))?;
    let tone = context.tone.as_deref().unwrap_or("ordinary");
    if !TONES.contains(&tone) {
        return Err(invalid("naming.unknown_tone"));
    }
    let mut level = kind;
    let mut patterns;
    loop {
        patterns = pack
            .patterns
            .iter()
            .filter(|p| p.kind == level)
            .collect::<Vec<_>>();
        if !patterns.is_empty() {
            break;
        }
        level = level
            .rsplit_once('.')
            .ok_or_else(|| invalid("naming.unknown_kind"))?
            .0;
    }
    patterns.sort_by(|a, b| a.id.cmp(&b.id));
    let palette = &culture_profile.palette;
    patterns.retain(|p| {
        p.requires.iter().all(|r| context.refs.contains_key(r))
            && p.tones.as_ref().is_none_or(|t| t.contains(tone))
            && p.slots
                .iter()
                .filter(|s| s.part_type == "concept")
                .all(|s| {
                    (!s.activity_filter || context.activity.is_some())
                        && candidates(pack, s, palette, context, tone).len()
                            >= if s.name == "emblem2" { 2 } else { 1 }
                })
    });
    if patterns.is_empty() {
        return Ok(None);
    }
    let mut draw = |path: &str, n: usize| {
        let value = choose(path, n)?;
        if value >= n {
            Err(invalid("naming.choice_out_of_range"))
        } else {
            Ok(value)
        }
    };
    let total = patterns
        .iter()
        .try_fold(0usize, |n, p| n.checked_add(p.weight))
        .ok_or_else(|| invalid("naming.weight_overflow"))?;
    if total == 0 {
        return Err(invalid("naming.zero_weight"));
    }
    let mut r = draw("pattern", total)?;
    let pattern = patterns
        .into_iter()
        .find(|p| {
            if r < p.weight {
                true
            } else {
                r -= p.weight;
                false
            }
        })
        .expect("weighted pattern");
    let mut parts: BTreeMap<String, Part> = BTreeMap::new();
    for slot in &pattern.slots {
        let name = &slot.name;
        let id = if slot.part_type == "ref" {
            context
                .refs
                .get(slot.reference.as_deref().unwrap_or(""))
                .ok_or_else(|| invalid("naming.missing_ref"))?
                .clone()
        } else {
            let mut banned = BTreeSet::new();
            for ex in &pack.exclusions {
                if ex.pattern == pattern.id
                    && ex.parts.contains_key(name)
                    && ex
                        .parts
                        .iter()
                        .filter(|(k, _)| *k != name)
                        .all(|(k, v)| parts.get(k).is_some_and(|p| &p.id == v))
                {
                    banned.insert(ex.parts[name].as_str());
                }
            }
            if name == "emblem2" {
                banned.insert(
                    parts
                        .get("emblem")
                        .ok_or_else(|| invalid("naming.missing_emblem"))?
                        .id
                        .as_str(),
                );
            }
            let stems: BTreeSet<_> = parts
                .values()
                .filter(|p| p.part_type == "concept")
                .flat_map(|p| pack.concepts[&p.id].stems.iter())
                .collect();
            let mut cands = candidates(pack, slot, palette, context, tone);
            cands.retain(|c| {
                !banned.contains(c.id.as_str()) && c.stems.iter().all(|s| !stems.contains(s))
            });
            if cands.is_empty() {
                return Ok(None);
            }
            if let Some(fits) = &slot.fits {
                cands = narrow(pack, cands, parts.get(fits), "sem:", "fits:");
            }
            if let Some(pair) = &slot.pairs_with {
                cands = narrow(pack, cands, parts.get(pair), "pair:", "pair:");
            }
            cands[draw(&format!("slot/{name}"), cands.len())?]
                .id
                .clone()
        };
        parts.insert(
            name.clone(),
            Part {
                part_type: slot.part_type.clone(),
                id,
            },
        );
    }
    Ok(Some(NameTree::new(
        json!({"pattern":pattern.id,"kind":kind,"culture":culture,"register":"formal","parts":parts}),
    )?))
}
/// Chronos draws are addressed by entity identity and path, never query order.
pub fn expand_keyed(
    pack: &Pack,
    kind: &str,
    culture: &str,
    context: &NameContext,
    seed: u64,
    entity_id: &str,
) -> Result<Option<NameTree>, HistoryError> {
    expand(pack, kind, culture, context, |path, n| {
        uniform_below(seed, &[NAMING_RULE, entity_id, path], 0, n as u64).map(|v| v as usize)
    })
}

#[path = "naming_render.rs"]
mod display;
#[path = "naming_unicode.rs"]
mod unicode;
pub use display::{render, RenderOptions, RenderResult};

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keyed_names_preserve_subkind_and_rendering_never_draws() {
        let pack = frozen_pack();
        let context = NameContext {
            refs: BTreeMap::from([
                ("site".into(), "place-1".into()),
                ("surname".into(), "family-1".into()),
            ]),
            activity: Some("craft".into()),
            tone: Some("upright".into()),
        };
        // Both kinds reach the same nearest pattern level, keeping their precise kind.
        let parent = expand_keyed(
            pack,
            "organization.guild",
            "bastide",
            &context,
            11,
            "group-1",
        )
        .unwrap()
        .unwrap();
        let tree = expand_keyed(
            pack,
            "organization.guild.future_subkind",
            "bastide",
            &context,
            11,
            "group-1",
        )
        .unwrap()
        .unwrap();
        assert_eq!(tree.value()["kind"], "organization.guild.future_subkind");
        assert_eq!(tree.value()["parts"], parent.value()["parts"]);
        assert_eq!(tree.value()["pattern"], parent.value()["pattern"]);
        assert!(
            expand(pack, "organization.guild", "bastide", &context, |_, n| Ok(
                n
            ))
            .is_err()
        );
        assert!(
            expand(pack, "missing.kind", "bastide", &context, |_, _| panic!(
                "unknown kind draws nothing"
            ))
            .is_err()
        );
        let before = tree.clone();
        let place_tree = NameTree::new(json!({"pattern":"band.head_place","kind":"organization.band","culture":"bastide","register":"formal","parts":{"head":{"type":"concept","id":"head.reavers"},"place":{"type":"ref","id":"place-1"}}})).unwrap();
        let refs = BTreeMap::from([("place-1".into(), " E\u{301}breuil ".into())]);
        let fr = RenderOptions {
            target: "fr".into(),
            ..RenderOptions::default()
        };
        assert_eq!(
            render(pack, &place_tree, &refs, &fr).unwrap().text,
            "Écumeurs d'Ébreuil"
        );
        assert_eq!(
            unicode::nfc("A\u{30a}\u{301} \u{1100}\u{1161}\u{11a8}"),
            "Ǻ 각"
        );
        assert_eq!(tree, before);
        assert_eq!(
            expand_keyed(
                pack,
                "organization.guild.future_subkind",
                "bastide",
                &context,
                11,
                "group-1"
            )
            .unwrap()
            .unwrap(),
            tree
        );
    }
    #[test]
    fn namebench_2820b50_conformance() {
        let data: Value = serde_json::from_slice(include_bytes!(
            "../../../../fixtures/naming/naming-conformance.json"
        ))
        .unwrap();
        assert_eq!(data["pack"]["generation"], generation_fingerprint());
        assert_eq!(data["pack"]["rendering"], rendering_fingerprint());
        let pack = frozen_pack();
        let mut trees = 0;
        let mut renders = 0;
        for vector in data["vectors"].as_array().unwrap() {
            let context: NameContext = serde_json::from_value(vector["context"].clone()).unwrap();
            let mut used = BTreeMap::new();
            let tree = expand(
                pack,
                vector["kind"].as_str().unwrap(),
                vector["culture"].as_str().unwrap(),
                &context,
                |path, n| {
                    let v = vector["choices"][path]
                        .as_u64()
                        .unwrap_or_else(|| panic!("{} missing {path}", vector["id"]))
                        as usize;
                    assert!(v < n, "{} {path}: {v} >= {n}", vector["id"]);
                    used.insert(path.to_owned(), v);
                    Ok(v)
                },
            )
            .unwrap()
            .unwrap();
            assert_eq!(tree.value(), &vector["tree"], "{}", vector["id"]);
            assert_eq!(
                serde_json::to_value(used).unwrap(),
                vector["choices"],
                "unused choices: {}",
                vector["id"]
            );
            trees += 1;
            let refs = serde_json::from_value(vector["ref_texts"].clone()).unwrap();
            for expected in vector["renders"].as_array().unwrap() {
                let options: RenderOptions =
                    serde_json::from_value(expected["options"].clone()).unwrap();
                let actual = render(pack, &tree, &refs, &options).unwrap();
                assert_eq!(
                    actual,
                    serde_json::from_value::<RenderResult>(expected.clone()).unwrap(),
                    "{} {:?}",
                    vector["id"],
                    options
                );
                renders += 1;
            }
        }
        assert_eq!((trees, renders), (306, 3366));
    }
}
