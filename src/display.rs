use crate::state::CurrentState;
use anyhow::Result;
use serde_json::json;

#[derive(Debug, Clone)]
pub struct Icons {
    pub apm: String,
    pub agents: String,
    pub git_pushes: String,
    pub actions: String,
    pub agent_time: String,
}

impl Default for Icons {
    fn default() -> Self {
        Self {
            apm: "󰘳".into(),
            agents: "󰚩".into(),
            git_pushes: "󰊢".into(),
            actions: "󰐌".into(),
            agent_time: "󱎫".into(),
        }
    }
}

pub fn format_compact(state: &CurrentState) -> String {
    format_compact_with_icons(state, &Icons::default())
}

pub fn format_compact_with_icons(state: &CurrentState, icons: &Icons) -> String {
    format!(
        "{} {}  {} {}  {} {}  {} {}  {} {}",
        icons.apm,
        state.apm,
        icons.agents,
        state.active_agents,
        icons.git_pushes,
        state.git_pushes_today,
        icons.actions,
        compact_number(state.total_actions_today),
        icons.agent_time,
        state.agent_active_human_today
    )
}

pub fn waybar_json(state: &CurrentState) -> Result<String> {
    let tooltip = format!(
        "Avg APM: {}\rNow APM: {}\rActive input time today: {}\rActive agents: {}{}\rGit pushes today: {}\rActions today: {}\rAgent active time today: {}\rUpdated: {}",
        state.apm,
        state.rolling_apm,
        state.active_input_human_today,
        state.active_agents,
        if state.active_agent_names.is_empty() {
            String::new()
        } else {
            format!("\r  - {}", state.active_agent_names.join("\r  - "))
        },
        state.git_pushes_today,
        state.total_actions_today,
        state.agent_active_human_today,
        state.timestamp.format("%H:%M:%S")
    );
    let class = if state.status == "ok" {
        vec!["prodstats", "active"]
    } else {
        vec!["prodstats", "stale"]
    };
    Ok(serde_json::to_string(&json!({
        "text": format_compact(state),
        "tooltip": tooltip,
        "class": class,
        "percentage": state.apm.min(100),
    }))?)
}

pub fn stale_waybar_json(reason: &str) -> Result<String> {
    Ok(serde_json::to_string(&json!({
        "text": "󰅙 prodstats",
        "tooltip": format!("Prodstats unavailable: {reason}\rRun: prodstats doctor"),
        "class": ["prodstats", "stale"],
        "percentage": 0,
    }))?)
}

fn compact_number(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}m", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
