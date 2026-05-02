use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHarnessConfig {
    pub name: String,
    pub process_names: Vec<String>,
    pub cmdline_regex: Vec<String>,
    #[serde(default)]
    pub exclude_cmdline_regex: Vec<String>,
}

impl AgentHarnessConfig {
    pub fn new(name: &str, process_names: Vec<&str>, cmdline_regex: Vec<&str>) -> Self {
        Self {
            name: name.into(),
            process_names: process_names.into_iter().map(String::from).collect(),
            cmdline_regex: cmdline_regex.into_iter().map(String::from).collect(),
            exclude_cmdline_regex: Vec::new(),
        }
    }
}

impl Default for AgentHarnessConfig {
    fn default() -> Self {
        Self::new("hermes", vec!["hermes"], vec!["(^|/)hermes( |$)"])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessSample {
    pub pid: i32,
    pub ppid: i32,
    pub comm: String,
    pub cmdline: String,
    pub cpu_ms: u64,
    pub read_bytes: u64,
    pub write_bytes: u64,
}

impl ProcessSample {
    pub fn new(
        pid: i32,
        ppid: i32,
        comm: &str,
        cmdline: &str,
        cpu_ms: u64,
        read_bytes: u64,
        write_bytes: u64,
    ) -> Self {
        Self {
            pid,
            ppid,
            comm: comm.into(),
            cmdline: cmdline.into(),
            cpu_ms,
            read_bytes,
            write_bytes,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProcessTable {
    by_pid: HashMap<i32, ProcessSample>,
    children: HashMap<i32, Vec<i32>>,
}

impl ProcessTable {
    pub fn new(samples: Vec<ProcessSample>) -> Self {
        let mut by_pid = HashMap::new();
        let mut children: HashMap<i32, Vec<i32>> = HashMap::new();
        for s in samples {
            children.entry(s.ppid).or_default().push(s.pid);
            by_pid.insert(s.pid, s);
        }
        Self { by_pid, children }
    }

    pub fn samples(&self) -> impl Iterator<Item = &ProcessSample> {
        self.by_pid.values()
    }
    pub fn get(&self, pid: i32) -> Option<&ProcessSample> {
        self.by_pid.get(&pid)
    }

    pub fn descendants_inclusive(&self, root: i32) -> Vec<i32> {
        let mut out = Vec::new();
        let mut stack = vec![root];
        while let Some(pid) = stack.pop() {
            out.push(pid);
            if let Some(kids) = self.children.get(&pid) {
                stack.extend(kids);
            }
        }
        out
    }
}

pub fn default_harnesses() -> Vec<AgentHarnessConfig> {
    vec![
        AgentHarnessConfig::new(
            "hermes",
            vec!["hermes"],
            vec!["(^|/)hermes( |$)", "hermes.*agent"],
        ),
        AgentHarnessConfig::new(
            "claude",
            vec!["claude"],
            vec!["(^|/)claude( |$)", "claude-code"],
        ),
        AgentHarnessConfig::new(
            "codex",
            vec!["codex"],
            vec!["(^|/)codex( |$)", "openai-codex"],
        ),
        AgentHarnessConfig::new("opencode", vec!["opencode"], vec!["(^|/)opencode( |$)"]),
        AgentHarnessConfig::new("aider", vec!["aider"], vec!["(^|/)aider( |$)"]),
        AgentHarnessConfig::new("goose", vec!["goose"], vec!["(^|/)goose( |$)"]),
        AgentHarnessConfig::new("gemini", vec!["gemini"], vec!["(^|/)gemini( |$)"]),
        AgentHarnessConfig::new("amp", vec!["amp"], vec!["(^|/)amp( |$)"]),
    ]
}

pub fn detect_active_harnesses(
    harnesses: &[AgentHarnessConfig],
    previous: &ProcessTable,
    current: &ProcessTable,
    cpu_threshold_ms_per_second: u64,
    io_threshold_bytes_per_second: u64,
    elapsed_seconds: f64,
) -> Vec<String> {
    let mut active = Vec::new();
    let mut seen_roots = HashSet::new();
    for h in harnesses {
        let cpu_threshold_ms =
            ((cpu_threshold_ms_per_second as f64) * elapsed_seconds).ceil() as u64;
        let io_threshold_bytes =
            ((io_threshold_bytes_per_second as f64) * elapsed_seconds).ceil() as u64;
        let regexes: Vec<Regex> = h
            .cmdline_regex
            .iter()
            .filter_map(|r| Regex::new(r).ok())
            .collect();
        let exclude_regexes: Vec<Regex> = h
            .exclude_cmdline_regex
            .iter()
            .filter_map(|r| Regex::new(r).ok())
            .collect();
        for proc in current.samples() {
            if !seen_roots.insert((h.name.clone(), proc.pid)) {
                continue;
            }
            let name_match = h.process_names.iter().any(|n| n == &proc.comm);
            let cmd_match = regexes.iter().any(|r| r.is_match(&proc.cmdline));
            if !(name_match || cmd_match) {
                continue;
            }
            if is_excluded_root(h, proc, &exclude_regexes) {
                continue;
            }
            let mut root_active = false;
            for pid in current.descendants_inclusive(proc.pid) {
                let Some(cur) = current.get(pid) else {
                    continue;
                };
                let Some(prev) = previous.get(pid) else {
                    root_active = true;
                    break;
                };
                let cpu_delta = cur.cpu_ms.saturating_sub(prev.cpu_ms);
                let io_delta = cur.read_bytes.saturating_sub(prev.read_bytes)
                    + cur.write_bytes.saturating_sub(prev.write_bytes);
                if cpu_delta >= cpu_threshold_ms || io_delta >= io_threshold_bytes {
                    root_active = true;
                    break;
                }
            }
            if root_active {
                active.push(format!("{}:{}", h.name, proc.pid));
            }
        }
    }
    active
}

fn is_excluded_root(
    h: &AgentHarnessConfig,
    proc: &ProcessSample,
    exclude_regexes: &[Regex],
) -> bool {
    if exclude_regexes.iter().any(|r| r.is_match(&proc.cmdline)) {
        return true;
    }
    match h.name.as_str() {
        "hermes" => {
            proc.cmdline.contains(" dashboard ")
                || proc.cmdline.contains(" hermes dashboard")
                || proc.cmdline.contains("hermes_cli.main gateway run")
                || proc.cmdline.contains(" tui_gateway.slash_worker")
        }
        _ => false,
    }
}

pub fn read_process_table() -> anyhow::Result<ProcessTable> {
    let ticks_per_second = procfs::ticks_per_second() as u64;
    let mut samples = Vec::new();
    for pr in procfs::process::all_processes()? {
        let Ok(process) = pr else {
            continue;
        };
        let Ok(stat) = process.stat() else {
            continue;
        };
        let cmdline = process
            .cmdline()
            .map(|v| v.join(" "))
            .unwrap_or_else(|_| stat.comm.clone());
        let io = process.io().ok();
        let cpu_ticks = stat.utime + stat.stime;
        let cpu_ms = cpu_ticks.saturating_mul(1000) / ticks_per_second;
        samples.push(ProcessSample::new(
            process.pid,
            stat.ppid,
            &stat.comm,
            &cmdline,
            cpu_ms,
            io.as_ref().map(|i| i.read_bytes).unwrap_or(0),
            io.as_ref().map(|i| i.write_bytes).unwrap_or(0),
        ));
    }
    Ok(ProcessTable::new(samples))
}
