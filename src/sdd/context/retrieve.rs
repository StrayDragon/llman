//! PageIndex agentic retrieval.
//!
//! The chat model navigates the spec tree by calling three local tools
//! (`list_specs` / `get_document_structure` / `get_spec_content`), then emits a
//! `direct`/`related` classification. This module is LLM-client-agnostic: the
//! agentic loop is generic over a [`ChatInvoker`] trait so it can be exercised
//! with a mock client in tests (no network required). The real async-openai
//! adapter lives in [`super::chat`].

use super::tree::TreeIndex;
use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};

/// Maximum number of assistant→tool round trips before the loop forces a
/// no-tools final answer (see decision #5 in design.md).
pub const MAX_TOOL_ROUNDS: usize = 12;

/// Navigation protocol given to the chat model as the system message.
pub const SYSTEM_PROMPT: &str = "\
You are a spec retrieval agent for an SDD (spec-driven development) project.
Given a task and optional file paths, find which specs are relevant.

NAVIGATION PROTOCOL (follow this order):
1. Call list_specs() to see all available spec documents and their purposes.
2. For specs whose purpose seems relevant to the task, call get_document_structure(spec_id)\
 to see their requirement titles and, when present, the scenario ids under each requirement\
 (cheap, no full text).
3. For requirements that look relevant, call get_spec_content(spec_id, req_ids) to read\
 the full MUST/SHALL statements and, when present, the scenarios' Given/When/Then behavior\
 details.
4. Finally, output ONLY a JSON object (no other text, no code fences):
   {\"direct\": [{\"id\": \"<spec_id>\", \"reason\": \"<one sentence why this MUST be read>\"}],\
 \"related\": [{\"id\": \"<spec_id>\", \"reason\": \"<one sentence>\"}]}

CLASSIFICATION RULES:
- \"direct\" = specs whose behavior contract (any MUST/SHALL statement, command behavior,\
 exit code, output format, validation rule, or CLI surface) is affected by this change.
- \"related\" = specs that provide useful context but whose contract won't change.
- If the task changes behavior, the governing spec MUST be in \"direct\" — do NOT leave\
 \"direct\" empty just because you are unsure. An empty result means the task touches NO\
 spec behavior at all (e.g. a typo or formatting fix in a non-spec file, a pure dependency\
 version bump, or a file wholly outside every spec's scope).
- Decide \"direct\" vs \"related\" vs omit based on the requirement text you read, not on the\
 spec's purpose line alone.
- When a requirement has scenarios, read their Given/When/Then to judge precisely whether\
 the task affects that behavior — scenarios capture the exact contract better than the\
 high-level MUST/SHALL statement alone.
- Be precise: prefer fewer, well-reasoned entries over many guesses.\
";

// ---- protocol types (independent of async-openai) ---------------------------

/// A conversation message in the agentic loop.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Msg {
    System(String),
    User(String),
    Assistant {
        content: Option<String>,
        tool_calls: Vec<ToolCall>,
    },
    Tool {
        tool_call_id: String,
        content: String,
    },
}

/// A tool function schema presented to the model.
#[derive(Clone, Debug)]
pub struct ToolSchema {
    pub name: &'static str,
    pub description: &'static str,
    pub parameters: serde_json::Value,
}

/// A tool call the model wants the host to execute.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// One chat turn returned by the model.
#[derive(Clone, Debug, Default)]
pub struct ChatTurn {
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCall>,
}

/// What the model invoker must provide. Implemented by the real async-openai
/// adapter (`super::chat::OpenAiInvoker`) and by mocks in unit tests.
///
/// Uses a native `async fn` (stable on edition 2024), so no `async-trait`
/// dependency is needed; the loop is generic over `I: ChatInvoker`.
#[allow(async_fn_in_trait)]
pub trait ChatInvoker {
    /// Perform one chat completion given the conversation so far and the tools.
    async fn chat_turn(&self, messages: &[Msg], tools: &[ToolSchema]) -> Result<ChatTurn>;
}

// ---- result types -----------------------------------------------------------

/// A spec classified into `direct` or `related`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TierEntry {
    pub id: String,
    pub reason: String,
}

/// Pageindex retrieval result (converted to JSON by `mod.rs`).
#[derive(Clone, Debug, Default)]
pub struct RetrievalOutput {
    pub direct: Vec<TierEntry>,
    pub related: Vec<TierEntry>,
    pub tool_calls: usize,
    pub truncated: bool,
}

impl RetrievalOutput {
    pub fn truncated(tool_calls: usize) -> Self {
        Self {
            direct: Vec::new(),
            related: Vec::new(),
            tool_calls,
            truncated: true,
        }
    }
}

// ---- tool schemas -----------------------------------------------------------

/// Build the three tool schemas presented to the model.
pub fn build_tool_schemas() -> Vec<ToolSchema> {
    vec![
        ToolSchema {
            name: "list_specs",
            description: "List all spec documents with metadata. Call this first to see what specs exist.",
            parameters: serde_json::json!({ "type": "object", "properties": {} }),
        },
        ToolSchema {
            name: "get_document_structure",
            description: "Get the tree structure of one spec (titles + req_ids only, no full text, to save tokens).",
            parameters: serde_json::json!({
                "type": "object",
                "properties": { "spec_id": { "type": "string" } },
                "required": ["spec_id"],
            }),
        },
        ToolSchema {
            name: "get_spec_content",
            description: "Get the full statement text of specific requirements in a spec.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "spec_id": { "type": "string" },
                    "req_ids": { "type": "array", "items": { "type": "string" } },
                },
                "required": ["spec_id"],
            }),
        },
    ]
}

// ---- tool dispatch (local, no network) --------------------------------------

/// Execute a tool call locally against the tree, returning a JSON string result.
pub fn dispatch_tool(name: &str, arguments: &str, tree: &TreeIndex) -> String {
    let args: serde_json::Value =
        serde_json::from_str(arguments).unwrap_or_else(|_| serde_json::json!({}));
    match name {
        "list_specs" => list_specs(tree),
        "get_document_structure" => {
            let spec_id = args["spec_id"].as_str().unwrap_or("");
            get_document_structure(tree, spec_id)
        }
        "get_spec_content" => {
            let spec_id = args["spec_id"].as_str().unwrap_or("");
            let req_ids: Vec<String> = args["req_ids"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            get_spec_content(tree, spec_id, &req_ids)
        }
        other => serde_json::json!({ "error": format!("unknown tool: {other}") }).to_string(),
    }
}

fn list_specs(tree: &TreeIndex) -> String {
    let arr: Vec<serde_json::Value> = tree
        .docs
        .iter()
        .map(|d| {
            serde_json::json!({
                "spec_id": d.spec_id,
                "purpose": d.purpose,
                "req_count": d.reqs.len(),
            })
        })
        .collect();
    serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string())
}

fn get_document_structure(tree: &TreeIndex, spec_id: &str) -> String {
    match tree.docs.iter().find(|d| d.spec_id == spec_id) {
        Some(d) => {
            let reqs: Vec<serde_json::Value> = d
                .reqs
                .iter()
                .map(|r| {
                    // Attach scenario ids (title only, no full text) when this req
                    // has any. Req without scenarios serializes identically to the
                    // pre-scenario shape (no `scenarios` key) — progressive.
                    let scenario_ids: Vec<&str> = d
                        .scenarios
                        .iter()
                        .filter(|s| s.req_id == r.req_id)
                        .map(|s| s.id.as_str())
                        .collect();
                    if scenario_ids.is_empty() {
                        serde_json::json!({ "req_id": r.req_id, "title": r.title })
                    } else {
                        serde_json::json!({
                            "req_id": r.req_id,
                            "title": r.title,
                            "scenarios": scenario_ids,
                        })
                    }
                })
                .collect();
            // Spec-level scenarios (req_id empty, sourced from `.feature` files).
            // Listed as ids only at the structure layer (token-saving). Omitted
            // entirely when none exist — non-BDD specs keep the old shape.
            let spec_level_ids: Vec<&str> = d
                .scenarios
                .iter()
                .filter(|s| s.req_id.is_empty())
                .map(|s| s.id.as_str())
                .collect();
            if spec_level_ids.is_empty() {
                serde_json::json!({
                    "spec_id": d.spec_id,
                    "purpose": d.purpose,
                    "reqs": reqs,
                })
            } else {
                serde_json::json!({
                    "spec_id": d.spec_id,
                    "purpose": d.purpose,
                    "reqs": reqs,
                    "scenarios": spec_level_ids,
                })
            }
            .to_string()
        }
        None => {
            serde_json::json!({ "error": format!("spec_id {spec_id:?} not found") }).to_string()
        }
    }
}

fn get_spec_content(tree: &TreeIndex, spec_id: &str, req_ids: &[String]) -> String {
    match tree.docs.iter().find(|d| d.spec_id == spec_id) {
        Some(d) => {
            let want: std::collections::HashSet<&str> =
                req_ids.iter().map(|s| s.as_str()).collect();
            let mut result: Vec<serde_json::Value> = d
                .reqs
                .iter()
                .filter(|r| want.contains(r.req_id.as_str()))
                .map(|r| {
                    // Attach matching scenarios' full Given/When/Then when present.
                    // A req without scenarios serializes identically to the
                    // pre-scenario shape (no `scenarios` key) — progressive.
                    let scenarios: Vec<serde_json::Value> = d
                        .scenarios
                        .iter()
                        .filter(|s| s.req_id == r.req_id)
                        .map(|s| {
                            serde_json::json!({
                                "id": s.id,
                                "given": s.given,
                                "when": s.when_,
                                "then": s.then_,
                            })
                        })
                        .collect();
                    if scenarios.is_empty() {
                        serde_json::json!({ "req_id": r.req_id, "statement": r.statement })
                    } else {
                        serde_json::json!({
                            "req_id": r.req_id,
                            "statement": r.statement,
                            "scenarios": scenarios,
                        })
                    }
                })
                .collect();
            // Spec-level scenarios (req_id empty, sourced from `.feature` files):
            // append one extra entry so the agent can read behavior details that
            // live outside any requirement. Omitted when none exist — progressive.
            let spec_level: Vec<serde_json::Value> = d
                .scenarios
                .iter()
                .filter(|s| s.req_id.is_empty())
                .map(|s| {
                    serde_json::json!({
                        "id": s.id,
                        "given": s.given,
                        "when": s.when_,
                        "then": s.then_,
                    })
                })
                .collect();
            if !spec_level.is_empty() {
                result.push(serde_json::json!({
                    "req_id": "",
                    "statement": "",
                    "scenarios": spec_level,
                }));
            }
            serde_json::to_string(&result).unwrap_or_else(|_| "[]".to_string())
        }
        None => {
            serde_json::json!({ "error": format!("spec_id {spec_id:?} not found") }).to_string()
        }
    }
}

// ---- agentic loop -----------------------------------------------------------

/// Run the pageindex agentic retrieval loop.
///
/// The loop asks the model to navigate the tree via tools until it returns a
/// final `direct`/`related` JSON answer, or until [`MAX_TOOL_ROUNDS`] is reached
/// (in which case the result is marked truncated).
pub async fn retrieve<I: ChatInvoker>(
    invoker: &I,
    tree: &TreeIndex,
    task: &str,
    paths: &[String],
) -> Result<RetrievalOutput> {
    let tools = build_tool_schemas();
    let user = if paths.is_empty() {
        format!("Task: {task}")
    } else {
        format!("Task: {task}\nFile paths involved: {}", paths.join(", "))
    };
    let mut messages: Vec<Msg> = vec![Msg::System(SYSTEM_PROMPT.to_string()), Msg::User(user)];

    let mut tool_calls_total = 0usize;
    let debug = std::env::var("LLMAN_SDD_INDEX_DEBUG").is_ok();
    if debug {
        eprintln!("[pageindex] task={task:?} paths={paths:?}");
    }
    for _round in 0..MAX_TOOL_ROUNDS {
        let turn = invoker.chat_turn(&messages, &tools).await?;
        if debug {
            eprintln!(
                "[pageindex] turn: content={:?} tool_calls={{{}}}",
                turn.content.as_deref().unwrap_or(""),
                turn.tool_calls
                    .iter()
                    .map(|tc| format!("{}({})", tc.name, tc.arguments))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        if turn.tool_calls.is_empty() {
            let mut out = parse_final_answer(turn.content.as_deref().unwrap_or(""))?;
            out.tool_calls = tool_calls_total;
            if debug {
                eprintln!(
                    "[pageindex] final: direct={:?} related={:?}",
                    out.direct, out.related
                );
            }
            return Ok(out);
        }
        // Echo the assistant's tool calls back into the conversation.
        messages.push(Msg::Assistant {
            content: turn.content.clone(),
            tool_calls: turn.tool_calls.clone(),
        });
        for tc in &turn.tool_calls {
            tool_calls_total += 1;
            let result = dispatch_tool(&tc.name, &tc.arguments, tree);
            messages.push(Msg::Tool {
                tool_call_id: tc.id.clone(),
                content: result,
            });
        }
    }

    // Round limit reached: instead of discarding everything the model already
    // read, force one more turn with NO tools so it must emit a direct/related
    // answer from what it has gathered. This salvages thorough-but-slow models.
    let empty_tools: Vec<ToolSchema> = Vec::new();
    let salvage_prompt = Msg::User(
        "You have used all available tool calls. Based ONLY on what you have read so far, \
         output the final direct/related JSON now. Do not request any more tools."
            .to_string(),
    );
    messages.push(salvage_prompt);
    let turn = invoker.chat_turn(&messages, &empty_tools).await?;
    if debug {
        eprintln!(
            "[pageindex] salvage turn (no tools): content={:?}",
            turn.content.as_deref().unwrap_or("")
        );
    }
    match parse_final_answer(turn.content.as_deref().unwrap_or("")) {
        Ok(mut out) => {
            out.tool_calls = tool_calls_total;
            out.truncated = true;
            if debug {
                eprintln!(
                    "[pageindex] final (salvaged, truncated): direct={:?} related={:?}",
                    out.direct, out.related
                );
            }
            return Ok(out);
        }
        Err(e) => {
            if debug {
                eprintln!("[pageindex] salvage parse failed: {e}");
            }
        }
    }

    Ok(RetrievalOutput::truncated(tool_calls_total))
}

// ---- final answer parsing ---------------------------------------------------

/// Parse the model's final `direct`/`related` JSON answer out of its content.
pub fn parse_final_answer(content: &str) -> Result<RetrievalOutput> {
    let json_str = extract_json_object(content)
        .ok_or_else(|| anyhow::anyhow!("model did not return a JSON object; got: {content}"))?;
    #[derive(Deserialize)]
    struct Answer {
        #[serde(default)]
        direct: Vec<TierEntry>,
        #[serde(default)]
        related: Vec<TierEntry>,
    }
    let ans: Answer =
        serde_json::from_str(&json_str).context("failed to parse direct/related JSON")?;
    Ok(RetrievalOutput {
        direct: ans.direct,
        related: ans.related,
        tool_calls: 0,
        truncated: false,
    })
}

/// Extract the outermost JSON object from text (handles ```json fences and
/// surrounding prose).
fn extract_json_object(content: &str) -> Option<String> {
    let start = content.find('{')?;
    let end = content.rfind('}')?;
    if end < start {
        return None;
    }
    Some(content[start..=end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdd::spec::ir::{MainSpecDoc, RequirementEntry, ScenarioEntry};
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn build_test_tree() -> TreeIndex {
        let mk_req = |rid: &str, title: &str, stmt: &str| RequirementEntry {
            req_id: rid.to_string(),
            title: title.to_string(),
            statement: stmt.to_string(),
        };
        let docs = crate::sdd::context::tree::build_docs(&[
            (
                "sdd-workflow".to_string(),
                MainSpecDoc {
                    kind: "llman.sdd.spec".into(),
                    name: "sdd-workflow".into(),
                    purpose: "Define the SDD workflow.".into(),
                    valid_scope: vec![],
                    requirements: vec![
                        mk_req("r1", "Init", "`llman sdd init` MUST create dirs."),
                        mk_req(
                            "r12",
                            "Validate",
                            "`llman sdd validate` MUST check format and exit non-zero on error.",
                        ),
                    ],
                    scenarios: vec![],
                },
            ),
            (
                "cli".to_string(),
                MainSpecDoc {
                    kind: "llman.sdd.spec".into(),
                    name: "cli".into(),
                    purpose: "CLI surface.".into(),
                    valid_scope: vec![],
                    requirements: vec![mk_req("r1", "Commands", "MUST expose subcommands.")],
                    scenarios: vec![],
                },
            ),
        ]);
        TreeIndex::new(docs, "hash".into(), "ts".into(), "model".into())
    }

    #[test]
    fn test_list_specs_tool() {
        let tree = build_test_tree();
        let out: serde_json::Value =
            serde_json::from_str(&dispatch_tool("list_specs", "{}", &tree)).unwrap();
        let arr = out.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert!(arr.iter().any(|v| v["spec_id"] == "sdd-workflow"));
    }

    #[test]
    fn test_get_document_structure_strips_statement() {
        let tree = build_test_tree();
        let out: serde_json::Value = serde_json::from_str(&dispatch_tool(
            "get_document_structure",
            r#"{"spec_id":"sdd-workflow"}"#,
            &tree,
        ))
        .unwrap();
        assert_eq!(out["spec_id"], "sdd-workflow");
        let reqs = out["reqs"].as_array().unwrap();
        assert_eq!(reqs.len(), 2);
        // No statement text (token-saving), only req_id + title.
        assert!(reqs[0]["title"].as_str().is_some());
        assert!(reqs[0].get("statement").is_none());
    }

    #[test]
    fn test_get_spec_content_returns_statements() {
        let tree = build_test_tree();
        let out: serde_json::Value = serde_json::from_str(&dispatch_tool(
            "get_spec_content",
            r#"{"spec_id":"sdd-workflow","req_ids":["r12"]}"#,
            &tree,
        ))
        .unwrap();
        let arr = out.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["req_id"], "r12");
        assert!(arr[0]["statement"].as_str().unwrap().contains("MUST"));
    }

    #[test]
    fn test_unknown_tool_errors() {
        let tree = build_test_tree();
        let out: serde_json::Value =
            serde_json::from_str(&dispatch_tool("bogus", "{}", &tree)).unwrap();
        assert!(out["error"].as_str().unwrap().contains("unknown tool"));
    }

    #[test]
    fn test_parse_final_answer_plain_and_fenced() {
        let plain = r#"{"direct":[{"id":"sdd-workflow","reason":"x"}],"related":[]}"#;
        let out = parse_final_answer(plain).unwrap();
        assert_eq!(out.direct.len(), 1);
        assert_eq!(out.direct[0].id, "sdd-workflow");

        let fenced = "Here you go:\n```json\n{\"direct\":[],\"related\":[]}\n```\n";
        let out2 = parse_final_answer(fenced).unwrap();
        assert!(out2.direct.is_empty());
    }

    /// Build a tree where one spec carries scenarios under r1 (for scenario
    /// exposure tests). The other spec stays scenario-free to exercise the
    /// progressive (no-scenarios) path.
    fn build_test_tree_with_scenarios() -> TreeIndex {
        let mk_req = |rid: &str, title: &str, stmt: &str| RequirementEntry {
            req_id: rid.to_string(),
            title: title.to_string(),
            statement: stmt.to_string(),
        };
        let mk_scenario =
            |req_id: &str, id: &str, given: &str, when: &str, then: &str| ScenarioEntry {
                req_id: req_id.to_string(),
                id: id.to_string(),
                given: given.to_string(),
                when_: when.to_string(),
                then_: then.to_string(),
                feature: true,
            };
        let docs = crate::sdd::context::tree::build_docs(&[
            (
                "bdd-spec".to_string(),
                MainSpecDoc {
                    kind: "llman.sdd.spec".into(),
                    name: "bdd-spec".into(),
                    purpose: "A spec with scenarios.".into(),
                    valid_scope: vec![],
                    requirements: vec![mk_req("r1", "Behavior", "MUST do the thing.")],
                    scenarios: vec![
                        mk_scenario("r1", "happy", "a precondition", "an action", "an outcome"),
                        mk_scenario("r1", "error", "an error case", "an action", "an error"),
                    ],
                },
            ),
            (
                "plain-spec".to_string(),
                MainSpecDoc {
                    kind: "llman.sdd.spec".into(),
                    name: "plain-spec".into(),
                    purpose: "A spec without scenarios.".into(),
                    valid_scope: vec![],
                    requirements: vec![mk_req("r1", "Plain", "MUST exist.")],
                    scenarios: vec![],
                },
            ),
        ]);
        TreeIndex::new(docs, "hash".into(), "ts".into(), "model".into())
    }

    #[test]
    fn test_get_document_structure_includes_scenario_titles() {
        let tree = build_test_tree_with_scenarios();
        let out: serde_json::Value = serde_json::from_str(&dispatch_tool(
            "get_document_structure",
            r#"{"spec_id":"bdd-spec"}"#,
            &tree,
        ))
        .unwrap();
        let reqs = out["reqs"].as_array().unwrap();
        assert_eq!(reqs.len(), 1);
        let scenarios = reqs[0]["scenarios"].as_array().unwrap();
        // Only ids, no full text (token-saving at the structure layer).
        assert_eq!(scenarios.len(), 2);
        assert!(scenarios.iter().any(|v| v == "happy"));
        assert!(scenarios.iter().any(|v| v == "error"));
    }

    #[test]
    fn test_get_spec_content_includes_scenario_full_text() {
        let tree = build_test_tree_with_scenarios();
        let out: serde_json::Value = serde_json::from_str(&dispatch_tool(
            "get_spec_content",
            r#"{"spec_id":"bdd-spec","req_ids":["r1"]}"#,
            &tree,
        ))
        .unwrap();
        let arr = out.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        let scenarios = arr[0]["scenarios"].as_array().unwrap();
        assert_eq!(scenarios.len(), 2);
        // Full Given/When/Then text is present at the content layer.
        let happy = scenarios.iter().find(|v| v["id"] == "happy").unwrap();
        assert_eq!(happy["given"], "a precondition");
        assert_eq!(happy["when"], "an action");
        assert_eq!(happy["then"], "an outcome");
    }

    #[test]
    fn test_get_spec_content_no_scenarios_legacy() {
        // A spec with no scenarios MUST produce output identical to the
        // pre-scenario shape: no `scenarios` key on the entry at all.
        let tree = build_test_tree_with_scenarios();
        let out: serde_json::Value = serde_json::from_str(&dispatch_tool(
            "get_spec_content",
            r#"{"spec_id":"plain-spec","req_ids":["r1"]}"#,
            &tree,
        ))
        .unwrap();
        let arr = out.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert!(
            arr[0].get("scenarios").is_none(),
            "no scenarios key when empty"
        );
        assert_eq!(arr[0]["req_id"], "r1");
    }

    /// A mock invoker that returns a tool-call for every turn up to a cutoff,
    /// then a final answer turn. This lets truncation/salvage tests drive the
    /// loop deterministically regardless of `MAX_TOOL_ROUNDS`.
    struct ScriptedInvoker {
        turns: Vec<ChatTurn>,
        idx: AtomicUsize,
    }

    impl ScriptedInvoker {
        /// Replays `turns` in order; calls past the end repeat the LAST turn.
        fn new(turns: Vec<ChatTurn>) -> Self {
            Self {
                turns,
                idx: AtomicUsize::new(0),
            }
        }
    }

    impl ChatInvoker for ScriptedInvoker {
        async fn chat_turn(&self, _messages: &[Msg], _tools: &[ToolSchema]) -> Result<ChatTurn> {
            let i = self.idx.fetch_add(1, Ordering::SeqCst);
            let last = self.turns.len().saturating_sub(1);
            Ok(self.turns[i.min(last)].clone())
        }
    }

    /// Drive a future on a minimal current-thread runtime (no tokio "macros"
    /// feature required).
    fn block_on<F: std::future::Future>(fut: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(fut)
    }

    fn tool_call(id: &str, name: &str, args: &str) -> ChatTurn {
        ChatTurn {
            content: None,
            tool_calls: vec![ToolCall {
                id: id.to_string(),
                name: name.to_string(),
                arguments: args.to_string(),
            }],
        }
    }

    #[test]
    fn test_agentic_loop_navigates_then_classifies() {
        let tree = build_test_tree();
        let invoker = ScriptedInvoker::new(vec![
            tool_call("a", "list_specs", "{}"),
            tool_call("b", "get_document_structure", r#"{"spec_id":"sdd-workflow"}"#),
            tool_call("c", "get_spec_content", r#"{"spec_id":"sdd-workflow","req_ids":["r12"]}"#),
            ChatTurn {
                content: Some(
                    r#"{"direct":[{"id":"sdd-workflow","reason":"validate exit code lives here"}],"related":[{"id":"cli","reason":"cli surface"}]}"#.to_string(),
                ),
                tool_calls: vec![],
            },
        ]);

        let out = block_on(retrieve(&invoker, &tree, "fix validate exit code", &[])).unwrap();
        assert!(!out.truncated);
        assert_eq!(out.tool_calls, 3);
        assert_eq!(out.direct.len(), 1);
        assert_eq!(out.direct[0].id, "sdd-workflow");
        assert_eq!(out.related.len(), 1);
        assert_eq!(out.related[0].id, "cli");
    }

    #[test]
    fn test_agentic_loop_truncates_after_max_rounds() {
        let tree = build_test_tree();
        // Always request a tool call → never reaches a final answer; salvage also
        // gets a (tool-call) turn back, so the result stays empty + truncated.
        let invoker = ScriptedInvoker::new(vec![tool_call("x", "list_specs", "{}")]);
        let out = block_on(retrieve(&invoker, &tree, "task", &[])).unwrap();
        assert!(out.truncated, "loop must stop at MAX_TOOL_ROUNDS");
        assert_eq!(out.tool_calls, MAX_TOOL_ROUNDS);
        assert!(out.direct.is_empty(), "salvage turn had no JSON → empty");
    }

    #[test]
    fn test_agentic_loop_salvages_partial_answer_on_truncation() {
        let tree = build_test_tree();
        // Build a script with exactly MAX_TOOL_ROUNDS tool-call turns followed by
        // one final answer turn. The loop consumes all MAX tool rounds, then the
        // forced no-tools salvage call reads the answer turn.
        let mut turns: Vec<ChatTurn> = (0..MAX_TOOL_ROUNDS)
            .map(|_| tool_call("x", "list_specs", "{}"))
            .collect();
        turns.push(ChatTurn {
            content: Some(
                r#"{"direct":[{"id":"sdd-workflow","reason":"salvaged"}],"related":[]}"#
                    .to_string(),
            ),
            tool_calls: vec![],
        });
        let invoker = ScriptedInvoker::new(turns);
        let out = block_on(retrieve(&invoker, &tree, "task", &[])).unwrap();
        assert!(out.truncated, "hit the round limit");
        assert_eq!(out.tool_calls, MAX_TOOL_ROUNDS);
        assert_eq!(out.direct.len(), 1, "salvage answer preserved");
        assert_eq!(out.direct[0].id, "sdd-workflow");
    }
}
