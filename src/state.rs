use chrono::{DateTime, Datelike, Local};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CurrentState {
    pub timestamp: DateTime<Local>,
    pub apm: u64,
    pub rolling_apm: u64,
    pub active_input_seconds_today: u64,
    pub active_input_human_today: String,
    pub active_agents: usize,
    pub active_agent_names: Vec<String>,
    pub git_pushes_today: u64,
    pub total_actions_today: u64,
    pub agent_active_seconds_today: u64,
    pub agent_active_human_today: String,
    pub status: String,
}

#[derive(Debug, Clone)]
struct ActionBucket {
    timestamp: DateTime<Local>,
    actions: u64,
}

#[derive(Debug, Clone)]
pub struct MinuteStats {
    pub timestamp_minute: DateTime<Local>,
    pub actions: u64,
    pub key_presses: u64,
    pub mouse_clicks: u64,
    pub wheel_events: u64,
    pub apm: u64,
    pub active_agents: usize,
    pub agent_active_seconds: u64,
    pub git_pushes: u64,
    pub total_actions_today: u64,
    pub total_agent_active_seconds_today: u64,
    pub total_git_pushes_today: u64,
}

#[derive(Debug, Clone)]
pub struct MetricsEngine {
    day: (i32, u32, u32),
    buckets: VecDeque<ActionBucket>,
    total_actions_today: u64,
    key_presses_today: u64,
    mouse_clicks_today: u64,
    wheel_events_today: u64,
    git_pushes_today: u64,
    agent_active_seconds_today: f64,
    active_input_seconds_today: f64,
    active_agent_names: Vec<String>,
}

impl MetricsEngine {
    pub fn new(now: DateTime<Local>) -> Self {
        Self {
            day: day_tuple(now),
            buckets: VecDeque::new(),
            total_actions_today: 0,
            key_presses_today: 0,
            mouse_clicks_today: 0,
            wheel_events_today: 0,
            git_pushes_today: 0,
            agent_active_seconds_today: 0.0,
            active_input_seconds_today: 0.0,
            active_agent_names: Vec::new(),
        }
    }

    pub fn record_actions_at(
        &mut self,
        at: DateTime<Local>,
        actions: u64,
        keys: u64,
        mouse: u64,
        wheel: u64,
    ) {
        self.reset_if_new_day(at);
        self.total_actions_today += actions;
        self.key_presses_today += keys;
        self.mouse_clicks_today += mouse;
        self.wheel_events_today += wheel;
        self.buckets.push_back(ActionBucket {
            timestamp: at,
            actions,
        });
        self.prune_buckets(at);
    }

    pub fn record_git_push_at(&mut self, at: DateTime<Local>) {
        self.reset_if_new_day(at);
        self.git_pushes_today += 1;
    }

    pub fn set_git_pushes_today_at(&mut self, at: DateTime<Local>, count: u64) {
        self.reset_if_new_day(at);
        self.git_pushes_today = count;
    }

    pub fn record_active_input_time_at(
        &mut self,
        at: DateTime<Local>,
        elapsed_seconds: f64,
        active_window_seconds: i64,
    ) {
        self.reset_if_new_day(at);
        self.prune_buckets(at);
        let recently_active = self
            .buckets
            .iter()
            .any(|b| at.signed_duration_since(b.timestamp).num_seconds() < active_window_seconds);
        if recently_active {
            self.active_input_seconds_today += elapsed_seconds;
        }
    }

    pub fn record_agent_sample_at(
        &mut self,
        at: DateTime<Local>,
        active_names: Vec<String>,
        elapsed_seconds: f64,
    ) {
        self.reset_if_new_day(at);
        self.agent_active_seconds_today += active_names.len() as f64 * elapsed_seconds;
        self.active_agent_names = active_names;
    }

    pub fn snapshot_at(&mut self, at: DateTime<Local>) -> CurrentState {
        self.reset_if_new_day(at);
        self.prune_buckets(at);
        let rolling_apm = self.buckets.iter().map(|b| b.actions).sum();
        let active_input_seconds = self.active_input_seconds_today.round() as u64;
        let apm = if active_input_seconds > 0 {
            ((self.total_actions_today as f64 / self.active_input_seconds_today) * 60.0).round()
                as u64
        } else {
            0
        };
        let seconds = self.agent_active_seconds_today.round() as u64;
        CurrentState {
            timestamp: at,
            apm,
            rolling_apm,
            active_input_seconds_today: active_input_seconds,
            active_input_human_today: human_duration(active_input_seconds),
            active_agents: self.active_agent_names.len(),
            active_agent_names: self.active_agent_names.clone(),
            git_pushes_today: self.git_pushes_today,
            total_actions_today: self.total_actions_today,
            agent_active_seconds_today: seconds,
            agent_active_human_today: human_duration(seconds),
            status: "ok".into(),
        }
    }

    fn reset_if_new_day(&mut self, at: DateTime<Local>) {
        let d = day_tuple(at);
        if d != self.day {
            self.day = d;
            self.buckets.clear();
            self.total_actions_today = 0;
            self.key_presses_today = 0;
            self.mouse_clicks_today = 0;
            self.wheel_events_today = 0;
            self.git_pushes_today = 0;
            self.agent_active_seconds_today = 0.0;
            self.active_agent_names.clear();
        }
    }

    fn prune_buckets(&mut self, now: DateTime<Local>) {
        while let Some(front) = self.buckets.front() {
            if now.signed_duration_since(front.timestamp).num_seconds() >= 60 {
                self.buckets.pop_front();
            } else {
                break;
            }
        }
    }
}

pub fn human_duration(seconds: u64) -> String {
    let h = seconds / 3600;
    let m = (seconds % 3600) / 60;
    let s = seconds % 60;
    if h > 0 {
        format!("{}h {}m", h, m)
    } else if m > 0 {
        format!("{}m {}s", m, s)
    } else {
        format!("{}s", s)
    }
}

pub fn day_tuple(t: DateTime<Local>) -> (i32, u32, u32) {
    (t.year(), t.month(), t.day())
}
