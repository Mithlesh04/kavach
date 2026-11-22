// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! The settings of agents and `ask` calls as a run uses them (spec 8.4,
//! 8.5, decisions 0056 and 0060), read from what the checker found.

use crate::{AgentConfig, AgentMode, Compiled, Config};

/// The model an `ask` uses when neither it nor its agent names one
/// (spec 8.4): the smallest current one.
pub const DEFAULT_MODEL: &str = "claude-haiku-5-5";

/// The most tokens an answer may take when its `Model` doesn't say.
pub const DEFAULT_MAX_TOKENS: u32 = 4096;

impl Compiled {
    /// Agent `name`'s settings as a run uses them (spec 8.5, decision
    /// 0060): its `settings`, with what survives a restart on in a
    /// project, in `agents/<Name>/` beside `kavach.toml` (or `[agents]
    /// folder`, or its own `folder`). `fresh` (`kavach test`, a dry run)
    /// keeps no state and no journal.
    pub fn agent_config(&self, name: &str, fresh: bool) -> AgentConfig {
        let program = std::path::Path::new(&self.path);
        let config = Config::for_file(program).unwrap_or_default();
        let setup = self.agent_settings.get(name).cloned().unwrap_or_default();
        // Paths are relative to the folder of `kavach.toml`, or the
        // program's own without one.
        let base = (config.root.clone()).unwrap_or_else(|| {
            program
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_default()
        });
        let folder = match (&setup.folder, &config.root) {
            (Some(f), _) => Some(base.join(f)),
            (None, Some(root)) => {
                let all = config.agents.folder.as_deref().unwrap_or("agents");
                Some(root.join(all).join(name))
            }
            (None, None) => None,
        };
        let path = |p: std::path::PathBuf| p.to_string_lossy().to_string();
        let kept = |on: Option<bool>, file: &str| match (fresh, on) {
            (true, _) | (_, Some(false)) => None,
            _ => folder.as_ref().map(|f| path(f.join(file))),
        };
        let audit = setup.audit.as_ref();
        AgentConfig {
            mode: match setup.mode.as_deref() {
                Some("observe") => AgentMode::Observe,
                Some("suggest") => AgentMode::Suggest,
                _ => AgentMode::Enforce,
            },
            audit_log: audit.map(|a| match &a.file {
                Some(f) => path(base.join(f)),
                None => path(folder.clone().unwrap_or(base.clone()).join("audit.log")),
            }),
            state: kept(setup.state, "state.json"),
            journal: kept(setup.journal, "journal"),
            audit_format: audit.and_then(|a| a.json.then(|| "json".to_string())),
            audit_otlp: audit.is_some_and(|a| a.otlp),
            under_way: setup.under_way.unwrap_or_else(|| "Average".into()),
        }
    }

    /// The project's agents folder, where `agents.waiting()` and
    /// `agents.answer` look (spec 14.16): `agents/` beside `kavach.toml`,
    /// or `[agents] folder`. `None` outside a project, and when `fresh`.
    pub fn agents_folder(&self, fresh: bool) -> Option<String> {
        let config = Config::for_file(std::path::Path::new(&self.path)).unwrap_or_default();
        let all = config.agents.folder.as_deref().unwrap_or("agents");
        let root = config.root.filter(|_| !fresh)?;
        Some(root.join(all).to_string_lossy().to_string())
    }

    /// The model an `ask` at `call` uses, inside `agent`'s handlers or
    /// outside any: its own `model:`, else its agent's, else the default
    /// (spec 8.4, decision 0056).
    pub fn ask_model(&self, call: (u32, u32), agent: Option<&str>) -> kavach_checker::AiModel {
        self.ask_models
            .get(&call)
            .or_else(|| agent.and_then(|a| self.agent_models.get(a)))
            .cloned()
            .unwrap_or(kavach_checker::AiModel {
                name: DEFAULT_MODEL.to_string(),
                ..Default::default()
            })
    }
}

/// A model's settings as the runtime takes them (`kavach_runtime_net::ai::
/// Request::new`): the interpreter and native code send the same request.
pub fn model_settings(m: &kavach_checker::AiModel) -> kavach_runtime_core::json::Tree {
    use kavach_runtime_core::json::Tree;
    let text = |s: &str| Tree::Text(s.to_string());
    let max_tokens = m.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS);
    let mut fields = vec![
        ("model".to_string(), text(&m.name)),
        ("max_tokens".to_string(), Tree::Int(i64::from(max_tokens))),
    ];
    let mut add = |k: &str, v: Option<Tree>| {
        if let Some(v) = v {
            fields.push((k.to_string(), v));
        }
    };
    add("effort", m.effort.as_deref().map(text));
    add("thinking", m.thinking.map(Tree::Bool));
    add("instructions", m.instructions.as_deref().map(text));
    let stop = (!m.stop.is_empty()).then(|| Tree::Array(m.stop.iter().map(|s| text(s)).collect()));
    add("stop", stop);
    add("timeout_ms", m.timeout_ms.map(|n| Tree::Int(n as i64)));
    add("retries", m.retries.map(|n| Tree::Int(i64::from(n))));
    add("api_url", m.api_url.as_deref().map(text));
    add("api_key_env", m.api_key_env.as_deref().map(text));
    add("extra", m.extra.as_deref().map(text));
    // The provider's type, for its gate (decision 0140).
    add("provider", m.provider.as_ref().map(|(_, name)| text(name)));
    Tree::Object(fields)
}

/// Where a model's price is kept (`Compiled::ai_prices`): where its
/// provider is written (`None`: `Anthropic()`), and its name.
pub type AiPriceKey = (Option<(u32, u32)>, String);

/// The key of `m`'s price.
pub fn price_key(m: &kavach_checker::AiModel) -> AiPriceKey {
    (m.provider.as_ref().map(|(at, _)| *at), m.name.clone())
}

impl crate::Compiled {
    /// `m`'s settings, with its price when the program's build worked one
    /// out (spec 8.4, *Prices*): what an `ask` sends the runtime.
    pub fn priced_settings(&self, m: &kavach_checker::AiModel) -> kavach_runtime_core::json::Tree {
        use kavach_runtime_core::json::Tree;
        let mut settings = model_settings(m);
        if let (Tree::Object(fields), Some(price)) = (&mut settings, self.price_of(m)) {
            fields.push(("price".to_string(), price.clone()));
        }
        settings
    }

    /// `m`'s price as the build worked it out, if it did.
    pub fn price_of(
        &self,
        m: &kavach_checker::AiModel,
    ) -> Option<&kavach_runtime_core::json::Tree> {
        self.ai_prices.get()?.get(&price_key(m))
    }

    /// The models whose prices the build works out: each one the program
    /// writes, and the default.
    pub fn priced_models(&self) -> Vec<kavach_checker::AiModel> {
        let default = kavach_checker::AiModel {
            name: DEFAULT_MODEL.to_string(),
            ..Default::default()
        };
        let written = self.ai_written.iter().map(|(_, m)| m.clone());
        let mut out: Vec<kavach_checker::AiModel> = Vec::new();
        for m in std::iter::once(default).chain(written) {
            if !out.iter().any(|o| price_key(o) == price_key(&m)) {
                out.push(m);
            }
        }
        out
    }
}
