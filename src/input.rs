use crate::config::InputConfig;
use anyhow::{Context, Result};
use evdev::{Device, EventSummary, KeyCode, RelativeAxisCode};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, Default)]
pub struct ActionCounts {
    pub actions: u64,
    pub keys: u64,
    pub mouse: u64,
    pub wheel: u64,
}

pub fn discover_devices() -> Vec<PathBuf> {
    WalkDir::new("/dev/input")
        .max_depth(1)
        .into_iter()
        .filter_map(Result::ok)
        .map(|e| e.path().to_path_buf())
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("event"))
        })
        .collect()
}

pub fn classify_event(summary: EventSummary, config: &InputConfig) -> ActionCounts {
    match summary {
        EventSummary::Key(_, key, value)
            if value == 1 || (value == 2 && config.count_key_repeats) =>
        {
            if is_mouse_button(key) {
                if config.count_mouse_buttons {
                    ActionCounts {
                        actions: 1,
                        mouse: 1,
                        ..Default::default()
                    }
                } else {
                    ActionCounts::default()
                }
            } else if config.count_keys {
                ActionCounts {
                    actions: 1,
                    keys: 1,
                    ..Default::default()
                }
            } else {
                ActionCounts::default()
            }
        }
        EventSummary::RelativeAxis(_, axis, value)
            if value != 0
                && matches!(
                    axis,
                    RelativeAxisCode::REL_WHEEL
                        | RelativeAxisCode::REL_HWHEEL
                        | RelativeAxisCode::REL_WHEEL_HI_RES
                        | RelativeAxisCode::REL_HWHEEL_HI_RES
                ) =>
        {
            if config.count_wheel {
                ActionCounts {
                    actions: 1,
                    wheel: 1,
                    ..Default::default()
                }
            } else {
                ActionCounts::default()
            }
        }
        EventSummary::RelativeAxis(_, axis, value)
            if value != 0 && matches!(axis, RelativeAxisCode::REL_X | RelativeAxisCode::REL_Y) =>
        {
            if config.count_pointer_motion {
                ActionCounts {
                    actions: 1,
                    ..Default::default()
                }
            } else {
                ActionCounts::default()
            }
        }
        _ => ActionCounts::default(),
    }
}

fn is_mouse_button(key: KeyCode) -> bool {
    (KeyCode::BTN_LEFT.0..=KeyCode::BTN_TASK.0).contains(&key.0)
}

pub fn start_evdev_threads(config: InputConfig, tx: Sender<ActionCounts>) -> Result<usize> {
    let devices = discover_devices();
    let mut opened = 0;
    for path in devices {
        let Ok(mut dev) = Device::open(&path) else {
            continue;
        };
        opened += 1;
        let tx = tx.clone();
        let cfg = config.clone();
        thread::Builder::new()
            .name(format!("prodstats-input-{}", path.display()))
            .spawn(move || {
                loop {
                    match dev.fetch_events() {
                        Ok(events) => {
                            for ev in events {
                                let c = classify_event(ev.destructure(), &cfg);
                                if c.actions > 0 {
                                    let _ = tx.send(c);
                                }
                            }
                        }
                        Err(_) => thread::sleep(Duration::from_millis(200)),
                    }
                }
            })
            .context("spawn input thread")?;
    }
    Ok(opened)
}
