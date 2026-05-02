use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};

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
        let parsed = parse_git_args(&repo, args)?;
        if !should_log_git_push(0, args) {
            return None;
        }
        let positional: Vec<&String> = args[parsed.command_index + 1..]
            .iter()
            .filter(|a| !a.starts_with('-'))
            .collect();
        Some(Self {
            timestamp: Local::now(),
            repo: parsed.repo,
            remote: positional.first().map(|s| (*s).clone()),
            branch: positional.get(1).map(|s| (*s).clone()),
            result: "success".into(),
            source: "shell-wrapper".into(),
        })
    }
}

pub fn should_log_git_push(exit_code: i32, args: &[String]) -> bool {
    let Some(parsed) = parse_git_args("", args) else {
        return false;
    };
    exit_code == 0
        && args.get(parsed.command_index).is_some_and(|a| a == "push")
        && !args[parsed.command_index + 1..]
            .iter()
            .any(|a| a == "--dry-run" || a == "-n")
}

#[derive(Debug)]
struct ParsedGitArgs {
    command_index: usize,
    repo: String,
}

fn parse_git_args(cwd: &str, args: &[String]) -> Option<ParsedGitArgs> {
    let mut i = 0;
    let mut repo = PathBuf::from(cwd);
    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "-C" => {
                let path = args.get(i + 1)?;
                let path = Path::new(path);
                repo = if path.is_absolute() {
                    path.into()
                } else {
                    repo.join(path)
                };
                i += 2;
            }
            "-c" | "--config-env" | "--git-dir" | "--work-tree" | "--namespace" => {
                args.get(i + 1)?;
                i += 2;
            }
            _ if arg.starts_with("--git-dir=")
                || arg.starts_with("--work-tree=")
                || arg.starts_with("--namespace=")
                || arg.starts_with("--config-env=")
                || arg.starts_with("--exec-path=")
                || arg.starts_with("-c") && arg.len() > 2 =>
            {
                i += 1;
            }
            _ if arg.starts_with('-') => {
                i += 1;
            }
            _ => {
                return Some(ParsedGitArgs {
                    command_index: i,
                    repo: repo.to_string_lossy().into_owned(),
                });
            }
        }
    }
    None
}

pub fn append_event(path: &Path, event: &GitPushEvent) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if is_recent_duplicate(path, event)? {
        return Ok(());
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

fn is_recent_duplicate(path: &Path, event: &GitPushEvent) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }
    let mut rdr = csv::Reader::from_path(path)?;
    let mut last_matching: Option<GitPushEvent> = None;
    for rec in rdr.deserialize::<GitPushEvent>() {
        let previous = rec?;
        if previous.repo == event.repo
            && previous.remote == event.remote
            && previous.branch == event.branch
            && previous.result == event.result
        {
            last_matching = Some(previous);
        }
    }
    Ok(last_matching
        .is_some_and(|previous| (event.timestamp - previous.timestamp).num_seconds().abs() <= 10))
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
