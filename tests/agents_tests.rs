use prodstats::agents::{AgentHarnessConfig, ProcessSample, ProcessTable, detect_active_harnesses};

fn harnesses() -> Vec<AgentHarnessConfig> {
    vec![
        AgentHarnessConfig::new("hermes", vec!["hermes"], vec!["hermes"]),
        AgentHarnessConfig::new("claude", vec!["claude"], vec!["claude"]),
    ]
}

#[test]
fn idle_harness_is_not_active() {
    let previous = ProcessTable::new(vec![ProcessSample::new(
        10, 1, "hermes", "hermes", 100, 0, 0,
    )]);
    let current = ProcessTable::new(vec![ProcessSample::new(
        10, 1, "hermes", "hermes", 100, 0, 0,
    )]);

    let active = detect_active_harnesses(&harnesses(), &previous, &current, 20, 4096, 1.0);
    assert!(active.is_empty());
}

#[test]
fn cpu_delta_marks_harness_active() {
    let previous = ProcessTable::new(vec![ProcessSample::new(
        10, 1, "hermes", "hermes", 100, 0, 0,
    )]);
    let current = ProcessTable::new(vec![ProcessSample::new(
        10, 1, "hermes", "hermes", 140, 0, 0,
    )]);

    let active = detect_active_harnesses(&harnesses(), &previous, &current, 20, 4096, 1.0);
    assert_eq!(active, vec!["hermes:10"]);
}

#[test]
fn child_activity_marks_parent_harness_active() {
    let previous = ProcessTable::new(vec![
        ProcessSample::new(10, 1, "claude", "claude", 100, 0, 0),
        ProcessSample::new(11, 10, "node", "node worker", 50, 0, 0),
    ]);
    let current = ProcessTable::new(vec![
        ProcessSample::new(10, 1, "claude", "claude", 100, 0, 0),
        ProcessSample::new(11, 10, "node", "node worker", 90, 0, 0),
    ]);

    let active = detect_active_harnesses(&harnesses(), &previous, &current, 20, 4096, 1.0);
    assert_eq!(active, vec!["claude:10"]);
}
