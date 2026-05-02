use chrono::{Duration, Local, TimeZone};
use prodstats::state::{CurrentState, MetricsEngine};

#[test]
fn rolling_apm_counts_last_sixty_seconds_only() {
    let start = Local.with_ymd_and_hms(2026, 5, 2, 13, 0, 0).unwrap();
    let mut engine = MetricsEngine::new(start);

    engine.record_actions_at(start, 10, 8, 1, 1);
    engine.record_actions_at(start + Duration::seconds(30), 5, 5, 0, 0);
    assert_eq!(
        engine
            .snapshot_at(start + Duration::seconds(30))
            .rolling_apm,
        15
    );

    assert_eq!(
        engine
            .snapshot_at(start + Duration::seconds(61))
            .rolling_apm,
        5
    );
}

#[test]
fn average_apm_uses_active_input_time() {
    let start = Local.with_ymd_and_hms(2026, 5, 2, 13, 0, 0).unwrap();
    let mut engine = MetricsEngine::new(start);

    engine.record_actions_at(start, 10, 10, 0, 0);
    engine.record_active_input_time_at(start, 10.0, 60);
    let snap = engine.snapshot_at(start + Duration::seconds(10));
    assert_eq!(snap.apm, 60);
    assert_eq!(snap.active_input_seconds_today, 10);
}

#[test]
fn active_agent_seconds_sum_concurrent_harnesses() {
    let start = Local.with_ymd_and_hms(2026, 5, 2, 13, 0, 0).unwrap();
    let mut engine = MetricsEngine::new(start);

    engine.record_agent_sample_at(start, vec!["hermes".into(), "claude".into()], 1.0);
    engine.record_agent_sample_at(start + Duration::seconds(1), vec!["hermes".into()], 1.0);

    let snap = engine.snapshot_at(start + Duration::seconds(2));
    assert_eq!(snap.active_agents, 1);
    assert_eq!(snap.active_agent_names, vec!["hermes"]);
    assert_eq!(snap.agent_active_seconds_today, 3);
}

#[test]
fn local_midnight_resets_daily_totals() {
    let start = Local.with_ymd_and_hms(2026, 5, 2, 23, 59, 59).unwrap();
    let mut engine = MetricsEngine::new(start);

    engine.record_actions_at(start, 7, 7, 0, 0);
    let next_day = start + Duration::seconds(2);
    engine.record_actions_at(next_day, 3, 3, 0, 0);

    let snap = engine.snapshot_at(next_day);
    assert_eq!(snap.total_actions_today, 3);
}

#[test]
fn state_json_round_trips() {
    let s = CurrentState {
        timestamp: Local.with_ymd_and_hms(2026, 5, 2, 13, 24, 30).unwrap(),
        apm: 37,
        rolling_apm: 42,
        active_input_seconds_today: 600,
        active_input_human_today: "10m 0s".into(),
        active_agents: 2,
        active_agent_names: vec!["hermes".into(), "claude".into()],
        git_pushes_today: 3,
        total_actions_today: 1832,
        agent_active_seconds_today: 4280,
        agent_active_human_today: "1h 11m".into(),
        status: "ok".into(),
    };
    let json = serde_json::to_string(&s).unwrap();
    let decoded: CurrentState = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.apm, 37);
    assert_eq!(decoded.active_agent_names, vec!["hermes", "claude"]);
}
