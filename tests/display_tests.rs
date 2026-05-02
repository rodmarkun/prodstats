use chrono::{Local, TimeZone};
use prodstats::display::{format_compact, waybar_json};
use prodstats::state::CurrentState;

fn sample() -> CurrentState {
    CurrentState {
        timestamp: Local.with_ymd_and_hms(2026, 5, 2, 13, 24, 30).unwrap(),
        apm: 42,
        rolling_apm: 77,
        active_input_seconds_today: 600,
        active_input_human_today: "10m 0s".into(),
        active_agents: 2,
        active_agent_names: vec!["hermes".into(), "claude".into()],
        git_pushes_today: 3,
        total_actions_today: 1832,
        agent_active_seconds_today: 4280,
        agent_active_human_today: "1h 11m".into(),
        status: "ok".into(),
    }
}

#[test]
fn compact_format_contains_all_stats() {
    let text = format_compact(&sample());
    assert!(text.contains("42"));
    assert!(text.contains("2"));
    assert!(text.contains("3"));
    assert!(text.contains("1.8k"));
    assert!(text.contains("1h 11m"));
}

#[test]
fn waybar_output_is_single_line_json() {
    let out = waybar_json(&sample()).unwrap();
    assert!(!out.contains('\n'));
    let value: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(value["text"].as_str().unwrap().contains("42"));
    assert!(value["tooltip"].as_str().unwrap().contains("Avg APM: 42"));
    assert!(value["tooltip"].as_str().unwrap().contains("Now APM: 77"));
    assert_eq!(value["class"][0], "prodstats");
}
