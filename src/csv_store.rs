use crate::state::{CurrentState, MinuteStats};
use anyhow::Result;
use chrono::{Datelike, Local};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

pub fn write_state_atomic(path: &Path, state: &CurrentState) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let data = serde_json::to_vec_pretty(state)?;
    {
        let mut f = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&tmp)?;
        f.write_all(&data)?;
        f.sync_all()?;
    }
    fs::rename(tmp, path)?;
    Ok(())
}

pub fn read_state(path: &Path) -> Result<CurrentState> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

pub fn append_minute(history_dir: &Path, row: &MinuteStats) -> Result<()> {
    fs::create_dir_all(history_dir)?;
    let path = history_dir.join(format!(
        "{:04}-{:02}.csv",
        row.timestamp_minute.year(),
        row.timestamp_minute.month()
    ));
    let exists = path.exists();
    let file = OpenOptions::new().create(true).append(true).open(path)?;
    let mut w = csv::WriterBuilder::new()
        .has_headers(!exists)
        .from_writer(file);
    w.write_record(&[
        row.timestamp_minute.to_rfc3339(),
        row.actions.to_string(),
        row.key_presses.to_string(),
        row.mouse_clicks.to_string(),
        row.wheel_events.to_string(),
        row.apm.to_string(),
        row.active_agents.to_string(),
        row.agent_active_seconds.to_string(),
        row.git_pushes.to_string(),
        row.total_actions_today.to_string(),
        row.total_agent_active_seconds_today.to_string(),
        row.total_git_pushes_today.to_string(),
    ])?;
    w.flush()?;
    Ok(())
}

pub fn stale_state(reason: &str) -> CurrentState {
    CurrentState {
        timestamp: Local::now(),
        apm: 0,
        rolling_apm: 0,
        active_input_seconds_today: 0,
        active_input_human_today: "0s".into(),
        active_agents: 0,
        active_agent_names: vec![],
        git_pushes_today: 0,
        total_actions_today: 0,
        agent_active_seconds_today: 0,
        agent_active_human_today: "0s".into(),
        status: reason.into(),
    }
}
