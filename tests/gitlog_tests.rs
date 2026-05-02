use chrono::Local;
use prodstats::gitlog::{GitPushEvent, append_event, count_today, should_log_git_push};

#[test]
fn logs_successful_push_only() {
    assert!(should_log_git_push(0, &["push".into()]));
    assert!(should_log_git_push(
        0,
        &["push".into(), "origin".into(), "main".into()]
    ));
    assert!(!should_log_git_push(1, &["push".into()]));
    assert!(!should_log_git_push(0, &["status".into()]));
    assert!(!should_log_git_push(
        0,
        &["push".into(), "--dry-run".into()]
    ));
}

#[test]
fn push_event_parses_remote_and_branch() {
    let event = GitPushEvent::from_args(
        "/tmp/repo".into(),
        &["push".into(), "origin".into(), "main".into()],
    )
    .unwrap();
    assert_eq!(event.repo, "/tmp/repo");
    assert_eq!(event.remote.as_deref(), Some("origin"));
    assert_eq!(event.branch.as_deref(), Some("main"));
    assert_eq!(event.result, "success");
}

#[test]
fn duplicate_wrapper_events_are_counted_once() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("git_pushes.csv");
    let event = GitPushEvent::from_args(
        "/tmp/repo".into(),
        &["push".into(), "origin".into(), "main".into()],
    )
    .unwrap();

    append_event(&path, &event).unwrap();
    append_event(&path, &event).unwrap();

    assert_eq!(count_today(&path, Local::now()).unwrap(), 1);
}
