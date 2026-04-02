use crate::sdd::spec::fence::{extract_single_fence_payload, replace_single_fence_payload};
use crate::sdd::spec::ir::{
    DeltaOpEntry, DeltaSpecDoc, MainSpecDoc, RequirementEntry, ScenarioEntry,
};
use anyhow::{Result, anyhow};
use serde_yaml::{Mapping, Value};
use yamlpatch::{Op, Patch, apply_yaml_patches};

pub enum OverlayResult {
    NoChanges,
    Patched { yaml: String },
}

pub fn overlay_main_spec_yaml(
    original_yaml: &str,
    old: &MainSpecDoc,
    new: &MainSpecDoc,
) -> Result<OverlayResult> {
    let patches = plan_main_spec_patches(old, new)?;
    apply_patches(original_yaml, patches)
}

pub fn overlay_delta_spec_yaml(
    original_yaml: &str,
    old: &DeltaSpecDoc,
    new: &DeltaSpecDoc,
) -> Result<OverlayResult> {
    let patches = plan_delta_spec_patches(old, new)?;
    apply_patches(original_yaml, patches)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YamlWriteBackMode {
    LosslessOverlay,
    FencedRewrite,
}

pub struct YamlWriteBack {
    pub content: String,
    pub mode: YamlWriteBackMode,
}

pub fn update_main_spec_markdown_with_overlay_or_fallback(
    original_markdown: &str,
    old: &MainSpecDoc,
    new: &MainSpecDoc,
    fallback_payload: &str,
) -> Result<YamlWriteBack> {
    let original_payload = extract_single_fence_payload(original_markdown, "yaml")?.payload;
    match overlay_main_spec_yaml(&original_payload, old, new) {
        Ok(OverlayResult::NoChanges) => Ok(YamlWriteBack {
            content: original_markdown.to_string(),
            mode: YamlWriteBackMode::LosslessOverlay,
        }),
        Ok(OverlayResult::Patched { yaml }) => Ok(YamlWriteBack {
            content: replace_single_fence_payload(original_markdown, "yaml", &yaml)?,
            mode: YamlWriteBackMode::LosslessOverlay,
        }),
        Err(_) => Ok(YamlWriteBack {
            content: replace_single_fence_payload(original_markdown, "yaml", fallback_payload)?,
            mode: YamlWriteBackMode::FencedRewrite,
        }),
    }
}

pub fn update_delta_spec_markdown_with_overlay_or_fallback(
    original_markdown: &str,
    old: &DeltaSpecDoc,
    new: &DeltaSpecDoc,
    fallback_payload: &str,
) -> Result<YamlWriteBack> {
    let original_payload = extract_single_fence_payload(original_markdown, "yaml")?.payload;
    match overlay_delta_spec_yaml(&original_payload, old, new) {
        Ok(OverlayResult::NoChanges) => Ok(YamlWriteBack {
            content: original_markdown.to_string(),
            mode: YamlWriteBackMode::LosslessOverlay,
        }),
        Ok(OverlayResult::Patched { yaml }) => Ok(YamlWriteBack {
            content: replace_single_fence_payload(original_markdown, "yaml", &yaml)?,
            mode: YamlWriteBackMode::LosslessOverlay,
        }),
        Err(_) => Ok(YamlWriteBack {
            content: replace_single_fence_payload(original_markdown, "yaml", fallback_payload)?,
            mode: YamlWriteBackMode::FencedRewrite,
        }),
    }
}

fn apply_patches(original_yaml: &str, patches: Vec<Patch<'static>>) -> Result<OverlayResult> {
    if patches.is_empty() {
        return Ok(OverlayResult::NoChanges);
    }

    let doc = yamlpath::Document::new(original_yaml)
        .map_err(|err| anyhow!("failed to parse YAML for patching: {err}"))?;
    let updated = apply_yaml_patches(&doc, &patches)
        .map_err(|err| anyhow!("failed to apply YAML patches: {err}"))?;
    Ok(OverlayResult::Patched {
        yaml: updated.source().to_string(),
    })
}

fn plan_main_spec_patches(old: &MainSpecDoc, new: &MainSpecDoc) -> Result<Vec<Patch<'static>>> {
    let mut patches: Vec<Patch<'static>> = Vec::new();

    if old.kind != new.kind {
        patches.push(Patch {
            route: yamlpath::route!("kind"),
            operation: Op::Replace(Value::String(new.kind.clone())),
        });
    }
    if old.name != new.name {
        patches.push(Patch {
            route: yamlpath::route!("name"),
            operation: Op::Replace(Value::String(new.name.clone())),
        });
    }
    if old.purpose != new.purpose {
        patches.push(Patch {
            route: yamlpath::route!("purpose"),
            operation: Op::Replace(Value::String(new.purpose.clone())),
        });
    }

    // requirements: keyed by req_id
    let mut old_req_index: std::collections::HashMap<&str, usize> =
        std::collections::HashMap::new();
    for (idx, req) in old.requirements.iter().enumerate() {
        old_req_index.insert(req.req_id.as_str(), idx);
    }
    let mut new_req_by_id: std::collections::HashMap<&str, &RequirementEntry> =
        std::collections::HashMap::new();
    for req in &new.requirements {
        new_req_by_id.insert(req.req_id.as_str(), req);
    }

    // replace fields first (no index shifts)
    for old_req in &old.requirements {
        let Some(new_req) = new_req_by_id.get(old_req.req_id.as_str()).copied() else {
            continue;
        };
        let idx = old_req_index[old_req.req_id.as_str()];
        if old_req.title != new_req.title {
            patches.push(Patch {
                route: yamlpath::route!("requirements", idx, "title"),
                operation: Op::Replace(Value::String(new_req.title.clone())),
            });
        }
        if old_req.statement != new_req.statement {
            patches.push(Patch {
                route: yamlpath::route!("requirements", idx, "statement"),
                operation: Op::Replace(Value::String(new_req.statement.clone())),
            });
        }
    }

    // removals: reverse order
    let mut req_remove_indices = old
        .requirements
        .iter()
        .enumerate()
        .filter_map(|(idx, req)| {
            if new_req_by_id.contains_key(req.req_id.as_str()) {
                None
            } else {
                Some(idx)
            }
        })
        .collect::<Vec<_>>();
    req_remove_indices.sort_unstable_by(|a, b| b.cmp(a));
    for idx in req_remove_indices {
        patches.push(Patch {
            route: yamlpath::route!("requirements", idx),
            operation: Op::Remove,
        });
    }

    // appends
    for req in &new.requirements {
        if old_req_index.contains_key(req.req_id.as_str()) {
            continue;
        }
        patches.push(Patch {
            route: yamlpath::route!("requirements"),
            operation: Op::Append {
                value: requirement_to_yaml_value(req),
            },
        });
    }

    // scenarios: keyed by (req_id, id)
    let mut old_scenario_index: std::collections::HashMap<(&str, &str), usize> =
        std::collections::HashMap::new();
    for (idx, scenario) in old.scenarios.iter().enumerate() {
        old_scenario_index.insert((scenario.req_id.as_str(), scenario.id.as_str()), idx);
    }
    let mut new_scenario_by_key: std::collections::HashMap<(&str, &str), &ScenarioEntry> =
        std::collections::HashMap::new();
    for scenario in &new.scenarios {
        new_scenario_by_key.insert((scenario.req_id.as_str(), scenario.id.as_str()), scenario);
    }

    for old_scenario in &old.scenarios {
        let key = (old_scenario.req_id.as_str(), old_scenario.id.as_str());
        let Some(new_scenario) = new_scenario_by_key.get(&key).copied() else {
            continue;
        };
        let idx = old_scenario_index[&key];
        if old_scenario.given != new_scenario.given {
            patches.push(Patch {
                route: yamlpath::route!("scenarios", idx, "given"),
                operation: Op::Replace(Value::String(new_scenario.given.clone())),
            });
        }
        if old_scenario.when_ != new_scenario.when_ {
            patches.push(Patch {
                route: yamlpath::route!("scenarios", idx, "when"),
                operation: Op::Replace(Value::String(new_scenario.when_.clone())),
            });
        }
        if old_scenario.then_ != new_scenario.then_ {
            patches.push(Patch {
                route: yamlpath::route!("scenarios", idx, "then"),
                operation: Op::Replace(Value::String(new_scenario.then_.clone())),
            });
        }
    }

    // removals: reverse order
    let mut scenario_remove_indices = old
        .scenarios
        .iter()
        .enumerate()
        .filter_map(|(idx, scenario)| {
            let key = (scenario.req_id.as_str(), scenario.id.as_str());
            if new_scenario_by_key.contains_key(&key) {
                None
            } else {
                Some(idx)
            }
        })
        .collect::<Vec<_>>();
    scenario_remove_indices.sort_unstable_by(|a, b| b.cmp(a));
    for idx in scenario_remove_indices {
        patches.push(Patch {
            route: yamlpath::route!("scenarios", idx),
            operation: Op::Remove,
        });
    }

    // appends
    for scenario in &new.scenarios {
        let key = (scenario.req_id.as_str(), scenario.id.as_str());
        if old_scenario_index.contains_key(&key) {
            continue;
        }
        patches.push(Patch {
            route: yamlpath::route!("scenarios"),
            operation: Op::Append {
                value: scenario_to_yaml_value(scenario),
            },
        });
    }

    Ok(patches)
}

fn plan_delta_spec_patches(old: &DeltaSpecDoc, new: &DeltaSpecDoc) -> Result<Vec<Patch<'static>>> {
    let mut patches: Vec<Patch<'static>> = Vec::new();

    if old.kind != new.kind {
        patches.push(Patch {
            route: yamlpath::route!("kind"),
            operation: Op::Replace(Value::String(new.kind.clone())),
        });
    }

    // ops: keyed by req_id
    let mut old_op_index: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for (idx, op) in old.ops.iter().enumerate() {
        old_op_index.insert(op.req_id.as_str(), idx);
    }
    let mut new_op_by_id: std::collections::HashMap<&str, &DeltaOpEntry> =
        std::collections::HashMap::new();
    for op in &new.ops {
        new_op_by_id.insert(op.req_id.as_str(), op);
    }

    for old_op in &old.ops {
        let Some(new_op) = new_op_by_id.get(old_op.req_id.as_str()).copied() else {
            continue;
        };
        let idx = old_op_index[old_op.req_id.as_str()];

        if old_op.op != new_op.op {
            patches.push(Patch {
                route: yamlpath::route!("ops", idx, "op"),
                operation: Op::Replace(Value::String(new_op.op.clone())),
            });
        }
        if old_op.title != new_op.title {
            patches.push(Patch {
                route: yamlpath::route!("ops", idx, "title"),
                operation: Op::Replace(option_string_to_yaml_value(&new_op.title)),
            });
        }
        if old_op.statement != new_op.statement {
            patches.push(Patch {
                route: yamlpath::route!("ops", idx, "statement"),
                operation: Op::Replace(option_string_to_yaml_value(&new_op.statement)),
            });
        }
        if old_op.from != new_op.from {
            patches.push(Patch {
                route: yamlpath::route!("ops", idx, "from"),
                operation: Op::Replace(option_string_to_yaml_value(&new_op.from)),
            });
        }
        if old_op.to != new_op.to {
            patches.push(Patch {
                route: yamlpath::route!("ops", idx, "to"),
                operation: Op::Replace(option_string_to_yaml_value(&new_op.to)),
            });
        }
        if old_op.name != new_op.name {
            patches.push(Patch {
                route: yamlpath::route!("ops", idx, "name"),
                operation: Op::Replace(option_string_to_yaml_value(&new_op.name)),
            });
        }
    }

    // removals: reverse order
    let mut op_remove_indices = old
        .ops
        .iter()
        .enumerate()
        .filter_map(|(idx, op)| {
            if new_op_by_id.contains_key(op.req_id.as_str()) {
                None
            } else {
                Some(idx)
            }
        })
        .collect::<Vec<_>>();
    op_remove_indices.sort_unstable_by(|a, b| b.cmp(a));
    for idx in op_remove_indices {
        patches.push(Patch {
            route: yamlpath::route!("ops", idx),
            operation: Op::Remove,
        });
    }

    // appends
    for op in &new.ops {
        if old_op_index.contains_key(op.req_id.as_str()) {
            continue;
        }
        patches.push(Patch {
            route: yamlpath::route!("ops"),
            operation: Op::Append {
                value: op_to_yaml_value(op),
            },
        });
    }

    // op_scenarios: keyed by (req_id, id)
    let mut old_scenario_index: std::collections::HashMap<(&str, &str), usize> =
        std::collections::HashMap::new();
    for (idx, scenario) in old.op_scenarios.iter().enumerate() {
        old_scenario_index.insert((scenario.req_id.as_str(), scenario.id.as_str()), idx);
    }
    let mut new_scenario_by_key: std::collections::HashMap<(&str, &str), &ScenarioEntry> =
        std::collections::HashMap::new();
    for scenario in &new.op_scenarios {
        new_scenario_by_key.insert((scenario.req_id.as_str(), scenario.id.as_str()), scenario);
    }

    for old_scenario in &old.op_scenarios {
        let key = (old_scenario.req_id.as_str(), old_scenario.id.as_str());
        let Some(new_scenario) = new_scenario_by_key.get(&key).copied() else {
            continue;
        };
        let idx = old_scenario_index[&key];
        if old_scenario.given != new_scenario.given {
            patches.push(Patch {
                route: yamlpath::route!("op_scenarios", idx, "given"),
                operation: Op::Replace(Value::String(new_scenario.given.clone())),
            });
        }
        if old_scenario.when_ != new_scenario.when_ {
            patches.push(Patch {
                route: yamlpath::route!("op_scenarios", idx, "when"),
                operation: Op::Replace(Value::String(new_scenario.when_.clone())),
            });
        }
        if old_scenario.then_ != new_scenario.then_ {
            patches.push(Patch {
                route: yamlpath::route!("op_scenarios", idx, "then"),
                operation: Op::Replace(Value::String(new_scenario.then_.clone())),
            });
        }
    }

    // removals: reverse order
    let mut scenario_remove_indices = old
        .op_scenarios
        .iter()
        .enumerate()
        .filter_map(|(idx, scenario)| {
            let key = (scenario.req_id.as_str(), scenario.id.as_str());
            if new_scenario_by_key.contains_key(&key) {
                None
            } else {
                Some(idx)
            }
        })
        .collect::<Vec<_>>();
    scenario_remove_indices.sort_unstable_by(|a, b| b.cmp(a));
    for idx in scenario_remove_indices {
        patches.push(Patch {
            route: yamlpath::route!("op_scenarios", idx),
            operation: Op::Remove,
        });
    }

    // appends
    for scenario in &new.op_scenarios {
        let key = (scenario.req_id.as_str(), scenario.id.as_str());
        if old_scenario_index.contains_key(&key) {
            continue;
        }
        patches.push(Patch {
            route: yamlpath::route!("op_scenarios"),
            operation: Op::Append {
                value: scenario_to_yaml_value(scenario),
            },
        });
    }

    Ok(patches)
}

fn requirement_to_yaml_value(req: &RequirementEntry) -> Value {
    let mut mapping = Mapping::new();
    mapping.insert(
        Value::String("req_id".to_string()),
        Value::String(req.req_id.clone()),
    );
    mapping.insert(
        Value::String("title".to_string()),
        Value::String(req.title.clone()),
    );
    mapping.insert(
        Value::String("statement".to_string()),
        Value::String(req.statement.clone()),
    );
    Value::Mapping(mapping)
}

fn scenario_to_yaml_value(scenario: &ScenarioEntry) -> Value {
    let mut mapping = Mapping::new();
    mapping.insert(
        Value::String("req_id".to_string()),
        Value::String(scenario.req_id.clone()),
    );
    mapping.insert(
        Value::String("id".to_string()),
        Value::String(scenario.id.clone()),
    );
    mapping.insert(
        Value::String("given".to_string()),
        Value::String(scenario.given.clone()),
    );
    mapping.insert(
        Value::String("when".to_string()),
        Value::String(scenario.when_.clone()),
    );
    mapping.insert(
        Value::String("then".to_string()),
        Value::String(scenario.then_.clone()),
    );
    Value::Mapping(mapping)
}

fn op_to_yaml_value(op: &DeltaOpEntry) -> Value {
    let mut mapping = Mapping::new();
    mapping.insert(
        Value::String("op".to_string()),
        Value::String(op.op.clone()),
    );
    mapping.insert(
        Value::String("req_id".to_string()),
        Value::String(op.req_id.clone()),
    );
    mapping.insert(
        Value::String("title".to_string()),
        option_string_to_yaml_value(&op.title),
    );
    mapping.insert(
        Value::String("statement".to_string()),
        option_string_to_yaml_value(&op.statement),
    );
    mapping.insert(
        Value::String("from".to_string()),
        option_string_to_yaml_value(&op.from),
    );
    mapping.insert(
        Value::String("to".to_string()),
        option_string_to_yaml_value(&op.to),
    );
    mapping.insert(
        Value::String("name".to_string()),
        option_string_to_yaml_value(&op.name),
    );
    Value::Mapping(mapping)
}

fn option_string_to_yaml_value(value: &Option<String>) -> Value {
    match value {
        Some(value) => Value::String(value.clone()),
        None => Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_preserves_comments_and_appends_to_block_sequences() {
        let original = r#"kind: llman.sdd.spec
name: sample
purpose: Purpose
requirements:
  # top comment
  - req_id: r1 # inline comment
    title: Old title
    statement: System MUST do old.
scenarios:
  - req_id: r1
    id: baseline
    given: '' # given comment
    when: old when
    then: old then
"#;

        let old = MainSpecDoc {
            kind: "llman.sdd.spec".to_string(),
            name: "sample".to_string(),
            purpose: "Purpose".to_string(),
            requirements: vec![RequirementEntry {
                req_id: "r1".to_string(),
                title: "Old title".to_string(),
                statement: "System MUST do old.".to_string(),
            }],
            scenarios: vec![ScenarioEntry {
                req_id: "r1".to_string(),
                id: "baseline".to_string(),
                given: "".to_string(),
                when_: "old when".to_string(),
                then_: "old then".to_string(),
            }],
        };

        let new = MainSpecDoc {
            kind: "llman.sdd.spec".to_string(),
            name: "sample".to_string(),
            purpose: "Purpose".to_string(),
            requirements: vec![RequirementEntry {
                req_id: "r1".to_string(),
                title: "New title".to_string(),
                statement: "System MUST do new.".to_string(),
            }],
            scenarios: vec![
                ScenarioEntry {
                    req_id: "r1".to_string(),
                    id: "baseline".to_string(),
                    given: "".to_string(),
                    when_: "old when".to_string(),
                    then_: "new then".to_string(),
                },
                ScenarioEntry {
                    req_id: "r1".to_string(),
                    id: "happy".to_string(),
                    given: "".to_string(),
                    when_: "new when".to_string(),
                    then_: "new then".to_string(),
                },
            ],
        };

        let out = overlay_main_spec_yaml(original, &old, &new).expect("overlay");
        match out {
            OverlayResult::NoChanges => panic!("expected patched YAML"),
            OverlayResult::Patched { yaml } => {
                // Comments outside the patched features should survive.
                assert!(yaml.contains("# top comment"));
                assert!(yaml.contains("# inline comment"));
                assert!(yaml.contains("# given comment"));
                assert!(yaml.contains("title: New title"));
                assert!(yaml.contains("statement: System MUST do new."));
                assert!(yaml.contains("id: happy"));
            }
        }
    }

    #[test]
    fn overlay_delta_preserves_comments_and_appends_to_block_sequences() {
        let original = r#"kind: llman.sdd.delta
ops:
  # ops comment
  - op: add_requirement
    req_id: r1
    title: Old title
    statement: System MUST do old.
    from: null
    to: null
    name: null
op_scenarios:
  - req_id: r1
    id: baseline
    given: '' # comment
    when: old when
    then: old then
"#;

        let old = DeltaSpecDoc {
            kind: "llman.sdd.delta".to_string(),
            ops: vec![DeltaOpEntry {
                op: "add_requirement".to_string(),
                req_id: "r1".to_string(),
                title: Some("Old title".to_string()),
                statement: Some("System MUST do old.".to_string()),
                from: None,
                to: None,
                name: None,
            }],
            op_scenarios: vec![ScenarioEntry {
                req_id: "r1".to_string(),
                id: "baseline".to_string(),
                given: "".to_string(),
                when_: "old when".to_string(),
                then_: "old then".to_string(),
            }],
        };

        let new = DeltaSpecDoc {
            kind: "llman.sdd.delta".to_string(),
            ops: vec![DeltaOpEntry {
                op: "add_requirement".to_string(),
                req_id: "r1".to_string(),
                title: Some("Old title".to_string()),
                statement: Some("System MUST do new.".to_string()),
                from: None,
                to: None,
                name: None,
            }],
            op_scenarios: vec![
                ScenarioEntry {
                    req_id: "r1".to_string(),
                    id: "baseline".to_string(),
                    given: "".to_string(),
                    when_: "old when".to_string(),
                    then_: "old then".to_string(),
                },
                ScenarioEntry {
                    req_id: "r1".to_string(),
                    id: "happy".to_string(),
                    given: "".to_string(),
                    when_: "new when".to_string(),
                    then_: "new then".to_string(),
                },
            ],
        };

        let out = overlay_delta_spec_yaml(original, &old, &new).expect("overlay");
        match out {
            OverlayResult::NoChanges => panic!("expected patched YAML"),
            OverlayResult::Patched { yaml } => {
                assert!(yaml.contains("# ops comment"));
                assert!(yaml.contains("statement: System MUST do new."));
                assert!(yaml.contains("# comment"));
                assert!(yaml.contains("id: happy"));
            }
        }
    }
}
