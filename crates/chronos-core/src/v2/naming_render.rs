//! Display-only port of Namebench render.py and morph.py (naming-1).
use super::{invalid, Pack, Part};
use crate::v2::model::{HistoryError, NameTree};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct RenderOptions {
    pub target: String,
    pub head: String,
    pub emblem: String,
    pub gloss: bool,
    pub script: String,
}
impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            target: "en".into(),
            head: "target".into(),
            emblem: "target".into(),
            gloss: false,
            script: "latin".into(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RenderResult {
    pub text: String,
    pub status: String,
    pub fallbacks: Vec<String>,
}
#[derive(Deserialize)]
struct Tree {
    pattern: String,
    culture: String,
    parts: BTreeMap<String, Part>,
}
fn string<'a>(v: &'a Value, k: &str) -> &'a str {
    v[k].as_str().unwrap_or("")
}
fn flag(v: &Value, k: &str) -> bool {
    v[k].as_bool().unwrap_or(false)
}
#[derive(Clone)]
struct Piece {
    text: String,
    form: Value,
    loan: bool,
}

fn vowel_start(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if "ŒœÆæ".contains(first) {
        return true;
    }
    match super::unicode::base(first) {
        'Y' | 'y' => chars.next().is_some_and(|c| {
            let base = super::unicode::base(c).to_lowercase().next().unwrap_or(c);
            super::unicode::letter(base) && !"aeiouy".contains(base)
        }),
        ch => "AEIOUaeiou".contains(ch),
    }
}
fn fr_of_concept(form: &Value) -> String {
    let text = string(form, "text");
    if !flag(form, "article") {
        return format!("{}{text}", if flag(form, "elide") { "d'" } else { "de " });
    }
    let prefix = if string(form, "number") == "pl" {
        "des "
    } else if flag(form, "elide") {
        "de l'"
    } else if string(form, "gender") == "f" {
        "de la "
    } else {
        "du "
    };
    format!("{prefix}{text}")
}
fn fr_of_place(text: &str) -> String {
    let text = text.trim();
    let low = text.to_lowercase();
    for (article, prefix) in [
        ("le ", "du "),
        ("les ", "des "),
        ("la ", "de la "),
        ("l'", "de l'"),
        ("l’", "de l'"),
    ] {
        if low.starts_with(article) {
            return format!("{prefix}{}", &text[article.len()..]);
        }
    }
    format!("{}{text}", if vowel_start(text) { "d'" } else { "de " })
}
fn adjective(form: &Value, gender: &str, number: &str, before: &str) -> String {
    let key = format!("{}{}", gender, if number == "pl" { "p" } else { "" });
    if key == "m" && form["vowel_m"].is_string() && vowel_start(before) {
        return string(form, "vowel_m").into();
    }
    string(&form["text"], &key).into()
}
fn en_noun(form: &Value) -> String {
    format!(
        "{}{}",
        if flag(form, "article") { "the " } else { "" },
        string(form, "text")
    )
}
fn qualified(target: &str, emblem: &Piece, qualifier: &Piece) -> Piece {
    let mut piece = emblem.clone();
    if target == "en" {
        piece.text = format!("{} {}", qualifier.text, emblem.text);
    } else if !qualifier.loan && flag(&qualifier.form, "prenominal") && !emblem.loan {
        let adj = adjective(
            &qualifier.form,
            string(&emblem.form, "gender"),
            string(&emblem.form, "number"),
            &emblem.text,
        );
        piece.text = format!("{adj} {}", emblem.text);
        piece.form["elide"] = vowel_start(&adj).into();
    } else {
        let adj = if qualifier.loan {
            qualifier.text.clone()
        } else {
            adjective(
                &qualifier.form,
                string(&emblem.form, "gender"),
                string(&emblem.form, "number"),
                "",
            )
        };
        piece.text = format!("{} {adj}", emblem.text);
    }
    piece.form["text"] = piece.text.clone().into();
    piece
}
// Keep the reference morph.py signature: each argument is one of seven fixed slots.
#[allow(clippy::too_many_arguments)]
fn compose_target(
    target: &str,
    shape: &str,
    head: Option<Piece>,
    emblem: Option<Piece>,
    place: Option<String>,
    premod: bool,
    qualifier: Option<Piece>,
    emblem2: Option<Piece>,
    owner: Option<String>,
) -> Result<String, HistoryError> {
    let required = || invalid("naming.missing_part");
    if shape == "emblem_only" {
        return Ok(emblem.ok_or_else(required)?.text);
    }
    let head = head.ok_or_else(required)?;
    if shape == "head_place" {
        let place = place.ok_or_else(required)?;
        return Ok(if target == "fr" {
            format!("{} {}", head.text, fr_of_place(&place))
        } else {
            format!("{place} {}", head.text)
        });
    }
    if shape == "head_owner" {
        let owner = owner.ok_or_else(required)?;
        return Ok(if target == "fr" {
            format!("{} des {owner}", head.text)
        } else {
            format!("{owner} {}", head.text)
        });
    }
    let mut emblem = emblem.ok_or_else(required)?;
    if shape == "head_two_emblems" {
        let second = emblem2.ok_or_else(required)?;
        return Ok(if target == "fr" {
            let first = if emblem.loan {
                emblem.text
            } else {
                fr_of_concept(&emblem.form)
            };
            let second = if second.loan {
                second.text
            } else {
                fr_of_concept(&second.form)
            };
            format!("{} {first} et {second}", head.text)
        } else if emblem.loan && second.loan {
            format!("{} and {} {}", emblem.text, second.text, head.text)
        } else {
            format!(
                "{} of {} and {}",
                head.text,
                if emblem.loan {
                    emblem.text
                } else {
                    en_noun(&emblem.form)
                },
                if second.loan {
                    second.text
                } else {
                    en_noun(&second.form)
                }
            )
        });
    }
    let shape = if shape == "head_emblem_adjective" {
        if let Some(q) = qualifier {
            emblem = qualified(target, &emblem, &q);
        }
        "head_emblem"
    } else {
        shape
    };
    if target == "fr" {
        if emblem.loan {
            return Ok(format!("{} {}", head.text, emblem.text));
        }
        if shape == "head_emblem" {
            return Ok(format!("{} {}", head.text, fr_of_concept(&emblem.form)));
        }
        let adj = adjective(
            &emblem.form,
            string(&head.form, "gender"),
            string(&head.form, "number"),
            &head.text,
        );
        Ok(if flag(&emblem.form, "prenominal") {
            format!("{adj} {}", head.text)
        } else {
            format!("{} {adj}", head.text)
        })
    } else if emblem.loan || shape == "head_adjective" || premod {
        Ok(format!("{} {}", emblem.text, head.text))
    } else {
        Ok(format!("{} of {}", head.text, en_noun(&emblem.form)))
    }
}

fn target_text(
    pack: &Pack,
    part: &Part,
    refs: &BTreeMap<String, String>,
    target: &str,
) -> Result<String, HistoryError> {
    match part.part_type.as_str() {
        "opaque" => Ok(part.id.clone()),
        "ref" => refs
            .get(&part.id)
            .map(|s| super::unicode::nfc(s).trim().to_owned())
            .ok_or_else(|| invalid("naming.missing_ref")),
        "concept" => {
            let form = pack
                .concepts
                .get(&part.id)
                .and_then(|c| c.forms.get(target))
                .ok_or_else(|| invalid("naming.unknown_concept_or_target"))?;
            Ok(form["text"]
                .as_str()
                .unwrap_or_else(|| string(&form["text"], "m"))
                .into())
        }
        _ => Err(invalid("naming.unknown_part_type")),
    }
}
fn source_text(form: &Value, script: &str, key: Option<&str>) -> String {
    if script == "native" && form["native"].is_string() {
        return string(form, "native").into();
    }
    key.and_then(|k| form[k].as_str())
        .unwrap_or_else(|| string(form, "latin"))
        .into()
}
type Sources<'a> = Vec<(&'a str, &'a Value)>;
fn piece(
    pack: &Pack,
    tree: &Tree,
    name: &str,
    refs: &BTreeMap<String, String>,
    options: &RenderOptions,
    sources: &Sources<'_>,
) -> Result<Option<Piece>, HistoryError> {
    let Some(part) = tree.parts.get(name) else {
        return Ok(None);
    };
    let text = target_text(pack, part, refs, &options.target)?;
    if part.part_type != "concept" {
        return Ok(Some(Piece {
            text,
            form: json!({"gender":"m","number":"sg"}),
            loan: true,
        }));
    }
    let form = pack.concepts[&part.id].forms[&options.target].clone();
    let source = sources.iter().find(|(n, _)| *n == name);
    Ok(Some(Piece {
        text: source.map_or(text, |(_, s)| source_text(s, &options.script, None)),
        form,
        loan: source.is_some(),
    }))
}
fn premod(pack: &Pack, tree: &Tree) -> bool {
    let (Some(head), Some(emblem)) = (tree.parts.get("head"), tree.parts.get("emblem")) else {
        return false;
    };
    emblem.part_type == "concept"
        && flag(&pack.concepts[&emblem.id].forms["en"], "premod")
        && head
            .id
            .bytes()
            .chain(emblem.id.bytes())
            .map(u64::from)
            .sum::<u64>()
            % 2
            == 0
}
fn fill(
    profile: &Value,
    shape: &str,
    pack: &Pack,
    tree: &Tree,
    refs: &BTreeMap<String, String>,
    options: &RenderOptions,
    sources: &Sources<'_>,
) -> Result<String, HistoryError> {
    let native = options.script == "native" && sources.iter().all(|(_, s)| s["native"].is_string());
    let template = if native {
        profile["templates_native"][shape].as_str()
    } else {
        None
    }
    .or_else(|| profile["templates"][shape].as_str())
    .ok_or_else(|| invalid("naming.missing_template"))?;
    let mut rest = template;
    let mut out = String::new();
    while let Some(start) = rest.find('{') {
        let prefix = &rest[..start];
        let end = rest[start..]
            .find('}')
            .ok_or_else(|| invalid("naming.invalid_template"))?
            + start;
        let (name, key) = rest[start + 1..end]
            .split_once('.')
            .map_or((&rest[start + 1..end], None), |(n, k)| (n, Some(k)));
        let mut elide_word = None;
        if flag(profile, "elide_de") && name == "place" {
            for word in ["de", "di"] {
                let suffix = format!("{word} ");
                if prefix.ends_with(&suffix)
                    && prefix[..prefix.len() - suffix.len()]
                        .chars()
                        .next_back()
                        .is_none_or(|c| !c.is_alphanumeric() && c != '_')
                {
                    elide_word = Some((word, prefix.len() - suffix.len()));
                }
            }
        }
        if let Some((word, cut)) = elide_word {
            out.push_str(&prefix[..cut]);
            let place = target_text(pack, &tree.parts[name], refs, &options.target)?;
            let low = place.to_lowercase();
            let contraction = profile["de_articles"].as_object().and_then(|a| {
                a.iter()
                    .find(|(article, _)| low.starts_with(article.as_str()))
            });
            if let Some((article, value)) = contraction {
                out.push_str(value.as_str().unwrap_or(""));
                out.push_str(&place[article.len()..]);
            } else {
                out.push_str(&format!(
                    "{}{}",
                    if vowel_start(&place) {
                        "d'".into()
                    } else {
                        format!("{word} ")
                    },
                    place
                ));
            }
        } else {
            out.push_str(prefix);
            if let Some((_, form)) = sources.iter().find(|(n, _)| *n == name) {
                out.push_str(&source_text(form, &options.script, key));
            } else {
                out.push_str(&target_text(
                    pack,
                    tree.parts
                        .get(name)
                        .ok_or_else(|| invalid("naming.missing_part"))?,
                    refs,
                    &options.target,
                )?);
            }
        }
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    Ok(out
        .split(' ')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .into())
}
fn compose(
    pack: &Pack,
    tree: &Tree,
    refs: &BTreeMap<String, String>,
    options: &RenderOptions,
    shape: &str,
    language: &str,
) -> Result<(String, Vec<String>, Vec<String>), HistoryError> {
    let profile = &pack.languages[language];
    let concepts: Vec<_> = ["head", "emblem", "qualifier", "emblem2"]
        .into_iter()
        .filter_map(|n| {
            tree.parts
                .get(n)
                .filter(|p| p.part_type == "concept")
                .map(|p| (n, p))
        })
        .collect();
    let requested: Vec<_> = concepts
        .iter()
        .filter(|(n, _)| {
            (if *n == "head" {
                &options.head
            } else {
                &options.emblem
            }) == "source"
        })
        .collect();
    let mut sources: Sources<'_> = requested
        .iter()
        .filter_map(|(n, p)| {
            pack.concepts[&p.id]
                .forms
                .get(language)
                .map(|form| (*n, form))
        })
        .collect();
    let mut fallbacks = Vec::new();
    if requested.iter().any(|(n, _)| *n == "head") && !sources.iter().any(|(n, _)| *n == "head") {
        fallbacks.push("head:source".into());
    }
    if requested
        .iter()
        .any(|(n, _)| *n != "head" && !sources.iter().any(|(s, _)| s == n))
    {
        fallbacks.push("emblem:source".into());
        sources.retain(|(n, _)| *n == "head");
    }
    let template = profile["templates"][shape].as_str();
    let no_composition = flag(profile, "no_composition");
    let joins_parts = template.is_some_and(|t| t.matches('{').count() >= 2);
    if !concepts.is_empty() && requested.len() == concepts.len() {
        if template.is_none() || (no_composition && concepts.len() >= 2) {
            sources.clear();
            fallbacks.push("syntax:source".into());
        } else if no_composition && joins_parts {
            fallbacks.push("syntax:source".into());
        }
    }
    let mut loans = sources.iter().filter(|(n, _)| *n != "head").count();
    if no_composition && loans >= 2 {
        sources.retain(|(n, _)| *n == "head");
        loans = 0;
        fallbacks.push("syntax:source".into());
    }
    if options.script == "native" {
        fallbacks.extend(
            sources
                .iter()
                .filter(|(_, s)| !s["native"].is_string())
                .map(|(n, _)| format!("{n}:script")),
        );
    }
    let mut statuses: Vec<String> = sources
        .iter()
        .map(|(_, s)| string(s, "status").into())
        .collect();
    if !concepts.is_empty() && sources.len() == concepts.len() && !(no_composition && joins_parts) {
        if joins_parts {
            statuses.push("constructed".into());
        }
        return Ok((
            fill(profile, shape, pack, tree, refs, options, &sources)?,
            statuses,
            fallbacks,
        ));
    }
    if loans >= 2 {
        statuses.push("constructed".into());
    }
    let mut emblem = piece(pack, tree, "emblem", refs, options, &sources)?;
    let mut qualifier = piece(pack, tree, "qualifier", refs, options, &sources)?;
    if sources.iter().any(|(n, _)| *n == "qualifier") {
        let e = emblem
            .as_mut()
            .ok_or_else(|| invalid("naming.missing_emblem"))?;
        let q = qualifier
            .take()
            .ok_or_else(|| invalid("naming.missing_qualifier"))?;
        let before = template.is_some_and(|t| {
            t.find("{qualifier").unwrap_or(usize::MAX) < t.find("{emblem").unwrap_or(usize::MAX)
        });
        e.text = if before {
            format!("{} {}", q.text, e.text)
        } else {
            format!("{} {}", e.text, q.text)
        };
    }
    let part_text = |name: &str| {
        tree.parts
            .get(name)
            .map(|p| target_text(pack, p, refs, &options.target))
            .transpose()
    };
    let text = compose_target(
        &options.target,
        shape,
        piece(pack, tree, "head", refs, options, &sources)?,
        emblem,
        part_text("place")?,
        premod(pack, tree),
        qualifier,
        piece(pack, tree, "emblem2", refs, options, &sources)?,
        part_text("owner")?,
    )?;
    Ok((text, statuses, fallbacks))
}
/// Render without seed or chooser. Ref labels are supplied by the host at display
/// time; they never enter expansion. Ref labels are normalized to NFC.
pub fn render(
    pack: &Pack,
    tree: &NameTree,
    refs: &BTreeMap<String, String>,
    options: &RenderOptions,
) -> Result<RenderResult, HistoryError> {
    if !["en", "fr"].contains(&options.target.as_str())
        || !["target", "source"].contains(&options.head.as_str())
        || !["target", "source"].contains(&options.emblem.as_str())
        || !["latin", "native"].contains(&options.script.as_str())
    {
        return Err(invalid("naming.invalid_render_options"));
    }
    let tree: Tree =
        serde_json::from_value(tree.value().clone()).map_err(|_| invalid("naming.invalid_tree"))?;
    let shape = &pack
        .patterns
        .iter()
        .find(|p| p.id == tree.pattern)
        .ok_or_else(|| invalid("naming.unknown_pattern"))?
        .shape;
    let required: &[&str] = match shape.as_str() {
        "head_emblem" | "head_adjective" => &["head", "emblem"],
        "head_place" => &["head", "place"],
        "head_owner" => &["head", "owner"],
        "emblem_only" => &["emblem"],
        "head_emblem_adjective" => &["head", "emblem", "qualifier"],
        "head_two_emblems" => &["head", "emblem", "emblem2"],
        _ => return Err(invalid("naming.unknown_shape")),
    };
    if required.iter().any(|n| !tree.parts.contains_key(*n)) {
        return Err(invalid("naming.missing_part"));
    }
    for part in tree.parts.values() {
        target_text(pack, part, refs, &options.target)?;
    }
    let language = &pack
        .cultures
        .get(&tree.culture)
        .ok_or_else(|| invalid("naming.unknown_culture"))?
        .language;
    if !pack.languages.contains_key(language) {
        return Err(invalid("naming.unknown_language"));
    }
    let (mut text, mut statuses, mut fallbacks) =
        compose(pack, &tree, refs, options, shape, language)?;
    if options.gloss {
        let source = RenderOptions {
            head: "source".into(),
            emblem: "source".into(),
            ..options.clone()
        };
        let (gloss, gloss_statuses, gloss_fallbacks) =
            compose(pack, &tree, refs, &source, shape, language)?;
        if gloss != text && !gloss_fallbacks.iter().any(|f| f.ends_with(":source")) {
            text = format!("{text} ({gloss})");
            statuses.extend(gloss_statuses);
        }
        for f in gloss_fallbacks {
            if !fallbacks.contains(&f) {
                fallbacks.push(f);
            }
        }
    }
    let status = ["artistic", "constructed", "attested"]
        .into_iter()
        .find(|s| statuses.iter().any(|v| v == s))
        .unwrap_or("translated")
        .into();
    Ok(RenderResult {
        text,
        status,
        fallbacks,
    })
}
