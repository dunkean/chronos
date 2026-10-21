//! Frozen Pecking Order resolutions (generic theme) flattened into Chronos blueprints.
//!
//! Calls the maintained Rust engine (`pecking_order::Engine::embedded().resolve`) with
//! `theme: "generic"` for a fixed list of systems, keeps the full role list of each
//! resolution and picks the head office with the engine's own rule (level 0, not
//! collective, highest importance; ties by role id). `succession` is a Chronos
//! interpretation of the engine's `authority` into a closed method list; the source
//! `selection` text is kept verbatim next to it.
//!
//! Usage: chronos-export-blueprints <output json>
//! (build after `python tools/pecking_order/snapshot.py ../pecking-order output/pecking-order-snapshot`)
use pecking_order::{Engine, Query, model::Authority};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// (blueprint kind, Pecking Order system id)
const PROFILES: [(&str, &str); 6] = [
    ("monarchy", "kingdom"),
    ("council", "free_city"),
    ("village_community", "village"),
    ("merchant_league", "city_league"),
    ("guild", "merchant_guild"),
    ("informal_collective", "mutual_aid_cell"),
];

/// Chronos closed list: hereditary | vote | appointment.
fn method(authority: Authority) -> &'static str {
    match authority {
        Authority::Hereditary => "hereditary",
        Authority::Democratic | Authority::Consensual => "vote",
        Authority::Aristocratic
        | Authority::Authoritarian
        | Authority::Theocratic
        | Authority::Military
        | Authority::Bureaucratic
        | Authority::Meritocratic
        | Authority::Personal => "appointment",
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args()
        .nth(1)
        .ok_or("usage: chronos-export-blueprints <output json>")?;
    let engine = Engine::embedded()?;
    // Digest of the knowledge base compiled into this binary (the sibling repo may change later).
    let kb_digest = format!("{:x}", Sha256::digest(serde_json::to_vec(&engine.content)?));
    let mut blueprints = vec![];
    for (kind, system) in PROFILES {
        let request =
            json!({"api_version": "2.0", "system": system, "theme": "generic", "locale": "fr"});
        let query: Query = serde_json::from_value(request.clone())?;
        let response = engine.resolve(query)?;
        let authority = serde_json::to_value(response.authority)?;
        let head = response
            .roles
            .iter()
            .filter(|r| r.level == 0 && !r.collective)
            .max_by(|a, b| a.importance.cmp(&b.importance).then(b.id.cmp(&a.id)));
        let (office, collective_only) = match head {
            Some(role) => (role, false),
            None => (
                response
                    .roles
                    .iter()
                    .max_by(|a, b| a.importance.cmp(&b.importance).then(b.id.cmp(&a.id)))
                    .ok_or("resolution without roles")?,
                true,
            ),
        };
        let roles: Vec<Value> = response
            .roles
            .iter()
            .map(|r| {
                json!({
                    "id": r.id, "function": r.function, "title_key": r.title_key,
                    "title_fr": r.title.fr, "title_en": r.title.en, "title_theme": r.title_theme,
                    "level": r.level, "importance": r.importance, "scope": r.scope,
                    "reports_to": r.reports_to, "collective": r.collective, "optional": r.optional,
                    "powers": r.powers.iter().map(|p| &p.id).collect::<Vec<_>>(),
                })
            })
            .collect();
        blueprints.push(json!({
            "id": format!("po-generic-{system}"),
            "label": response.system.label.fr,
            "label_en": response.system.label.en,
            "kind": kind,
            "office_title": office.title.fr,
            "office_title_en": office.title.en,
            "succession": if collective_only { "vote" } else { method(response.authority) },
            "source": "pecking-order",
            "role_id": office.id,
            "max_holders": if office.collective { Value::Null } else { json!(1) },
            "system_id": response.system.id,
            "theme": response.theme.id,
            "authority": authority,
            "structure": serde_json::to_value(response.structure)?,
            "size": serde_json::to_value(response.size)?,
            "selection_fr": office.selection.fr,
            "selection_en": office.selection.en,
            "office_collective": office.collective,
            "office_powers": office.powers.iter().map(|p| &p.id).collect::<Vec<_>>(),
            "content_version": response.content_version,
            "kb_digest": kb_digest,
            "api_version": response.api_version,
            "request": request,
            "roles": roles,
            "available_role_count": response.available_role_count,
        }));
    }
    let result = json!({
        "schema": "chronos-pecking-order-blueprints-v1",
        "engine": "pecking_order::Engine::embedded().resolve (Rust crate pecking-order, theme generic)",
        "kb_digest": kb_digest,
        "kb_digest_rule": "sha256 of serde_json::to_vec(&Engine::embedded().content)",
        "counts": {"systems": engine.content.systems.len(), "themes": engine.content.themes.len()},
        "succession_mapping": "Chronos interpretation of Pecking Order authority: hereditary->hereditary; democratic|consensual->vote; other authorities->appointment; collective-only offices->vote",
        "blueprints": blueprints,
    });
    if let Some(dir) = std::path::Path::new(&output).parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&output, serde_json::to_vec_pretty(&result)?)?;
    eprintln!(
        "wrote {output}: {} blueprints",
        result["blueprints"].as_array().map_or(0, Vec::len)
    );
    Ok(())
}
