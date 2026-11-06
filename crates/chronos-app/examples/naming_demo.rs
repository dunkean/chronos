//! L5 evaluation on the actual two-domain demo and pinned Lineage Lab worker.
//! Run with CHRONOS_PYTHON and CHRONOS_LINEAGE_SRC configured, from the repo root.
use chronos_app::v2::lineage_worker::WorkerSettings;
use chronos_app::v2::orchestrator::{Orchestrator, WorldInputs};
use chronos_app::v2::store::Store;
use chronos_core::v2::model::{ConductClass, EntityNature};
use chronos_core::v2::naming::{self, render, RenderOptions};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = root.join("output/naming-l5-evaluation");
    let inputs = WorldInputs::load(
        &root.join("fixtures/social-v2/region-demo.world.json"),
        &root.join("fixtures/social-v2/region-demo.atlas.json"),
        &root.join("docs/design/chronos-social-v2/culture-bindings.v1.json"),
    )?;
    let settings = WorkerSettings::resolve(None, None, &root, out.join(".cache"));
    let world = Orchestrator::open(inputs, settings, Store::new(&out))?;
    let mut names: BTreeMap<ConductClass, Vec<serde_json::Value>> = BTreeMap::new();
    let mut fallbacks = 0;
    let mut groups = 0;
    for domain in world.domain_ids() {
        let packet = world.social_packet(&domain)?;
        for group in &packet.groups {
            groups += 1;
            let Some(tree) = &group.entity.name_tree else {
                fallbacks += 1;
                continue;
            };
            if group.entity.nature != EntityNature::OrganizedGroup {
                continue;
            }
            let sample = names.entry(group.name.tone).or_default();
            // Emblematic names need no provisional geographic label in this report.
            if sample.len() >= 2
                || tree.value()["parts"]
                    .as_object()
                    .unwrap()
                    .values()
                    .any(|p| p["type"] == "ref")
            {
                continue;
            }
            let refs = BTreeMap::new();
            let en = render(
                naming::frozen_pack(),
                tree,
                &refs,
                &RenderOptions::default(),
            )?;
            let fr = render(
                naming::frozen_pack(),
                tree,
                &refs,
                &RenderOptions {
                    target: "fr".into(),
                    ..RenderOptions::default()
                },
            )?;
            sample.push(json!({"id":group.entity.id,"archetype":group.entity.archetype_id,"tone":group.name.tone,"culture":tree.value()["culture"],"en":en.text,"fr":fr.text}));
        }
    }
    let report = json!({"world_seed":11,"canon_id":world.canon().canon_id,"runtime_fingerprint":world.runtime_fingerprint(),"generation":naming::generation_fingerprint(),"rendering":naming::rendering_fingerprint(),"groups":groups,"fallbacks":fallbacks,"samples":names.values().flatten().collect::<Vec<_>>()});
    std::fs::write(out.join("names.json"), serde_json::to_vec_pretty(&report)?)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
