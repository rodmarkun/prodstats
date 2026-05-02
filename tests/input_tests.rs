use evdev::{EventSummary, KeyCode, KeyEvent};
use prodstats::{config::Config, input::classify_event};

#[test]
fn held_key_repeat_is_not_counted_by_default() {
    let config = Config::default();
    let counts = classify_event(
        EventSummary::from(KeyEvent::new(KeyCode::KEY_A, 2)),
        &config.input,
    );

    assert_eq!(counts.actions, 0);
    assert_eq!(counts.keys, 0);
}

#[test]
fn initial_key_press_is_counted() {
    let config = Config::default();
    let counts = classify_event(
        EventSummary::from(KeyEvent::new(KeyCode::KEY_A, 1)),
        &config.input,
    );

    assert_eq!(counts.actions, 1);
    assert_eq!(counts.keys, 1);
}

#[test]
fn held_key_repeat_can_be_opted_in() {
    let mut config = Config::default().input;
    config.count_key_repeats = true;

    let counts = classify_event(
        EventSummary::from(KeyEvent::new(KeyCode::KEY_A, 2)),
        &config,
    );

    assert_eq!(counts.actions, 1);
    assert_eq!(counts.keys, 1);
}
