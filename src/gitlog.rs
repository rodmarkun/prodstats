use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GitPushEvent {
    pub timestamp: DateTime<Local>,
    pub repo: String,
    pub remote: Option<String>,
    pub branch: Option<String>,
    pub result: String,
    pub source: String,
}

impl GitPushEvent {
    pub fn from_args(repo: String, args: &[String]) -> Option<Self> {
        if !should_log_git_push(0, args) {
            return None;
        }
        let positional: Vec<&String> = args
            .iter()
            .skip(1)
            .filter(|a| !a.starts_with('-'))
            .collect();
        Some(Self {
            timestamp: Local::now(),
            repo,
            remote: positional.first().map(|s| (*s).clone()),
            branch: positional.get(1).map(|s| (*s).clone()),
            result: "success".into(),
            source: "shell-wrapper".into(),
        })
    }
}

pub fn should_log_git_push(exit_code: i32, args: &[String]) -> bool {
    exit_code == 0
        && args.first().is_some_and(|a| a == "push")
        && !args.iter().any(|a| a == "--dry-run" || a == "-n")
}

pub fn append_event(path: &Path, event: &GitPushEvent) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let exists = path.exists();
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("open {}", path.display()))?;
    let mut writer = csv::WriterBuilder::new()
        .has_headers(!exists)
        .from_writer(file);
    writer.serialize(event)?;
    writer.flush()?;
    Ok(())
}

pub fn count_today(path: &Path, now: DateTime<Local>) -> Result<u64> {
    if !path.exists() {
        return Ok(0);
    }
    let mut rdr = csv::Reader::from_path(path)?;
    let mut count = 0;
    for rec in rdr.deserialize::<GitPushEvent>() {
        let event = rec?;
        if event.timestamp.date_naive() == now.date_naive() && event.result == "success" {
            count += 1;
        }
    }
    Ok(count)
}
