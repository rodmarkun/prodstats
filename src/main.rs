use anyhow::{Context, Result};
use chrono::{Local, Timelike};
use clap::{Parser, Subcommand};
use fs2::FileExt;
use prodstats::agents::{ProcessTable, detect_active_harnesses, read_process_table};
use prodstats::config::Config;
use prodstats::csv_store::{append_minute, read_state, stale_state, write_state_atomic};
use prodstats::display::{stale_waybar_json, waybar_json};
use prodstats::gitlog::{GitPushEvent, append_event, count_today, should_log_git_push};
use prodstats::input::{ActionCounts, start_evdev_threads};
use prodstats::install::{
    install_git_shim, install_input_access, install_omarchy, install_shell_rc,
    install_systemd_service,
};
use prodstats::paths::Paths;
use prodstats::state::{MetricsEngine, MinuteStats};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Parser)]
#[command(
    name = "prodstats",
    version,
    about = "Local productivity stats daemon and corner display"
)]
struct Cli {
    #[command(subcommand)]
    cmd: CommandKind,
}

#[derive(Subcommand)]
enum CommandKind {
    Daemon,
    Status {
        #[arg(long)]
        watch: bool,
        #[arg(long, default_value = "text")]
        format: String,
    },
    Waybar,
    Doctor,
    Install {
        #[command(subcommand)]
        target: InstallTarget,
    },
    LogGitCommand {
        exit_code: i32,
        cwd: String,
        args: Vec<String>,
    },
    Overlay {
        #[arg(long, default_value = "top-right")]
        corner: String,
    },
}

#[derive(Subcommand)]
enum InstallTarget {
    Omarchy {
        #[arg(long, default_value = "top-right")]
        corner: String,
    },
    Shell {
        #[arg(long)]
        shell: Option<String>,
    },
    Service,
    GitShim,
    InputAccess,
    All {
        #[arg(long, default_value = "top-right")]
        corner: String,
    },
}

fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();
    let cli = Cli::parse();
    match cli.cmd {
        CommandKind::Daemon => daemon(),
        CommandKind::Status { watch, format } => status(watch, &format),
        CommandKind::Waybar => waybar(),
        CommandKind::Doctor => doctor(),
        CommandKind::Install { target } => install(target),
        CommandKind::LogGitCommand {
            exit_code,
            cwd,
            args,
        } => log_git_command(exit_code, cwd, args),
        CommandKind::Overlay { corner } => overlay(&corner),
    }
}

fn daemon() -> Result<()> {
    let paths = Paths::new()?;
    fs::create_dir_all(&paths.data_dir)?;
    Config::write_default(&paths.config)?;
    let cfg = Config::load_or_default(&paths.config)?;
    let lock = OpenOptions::new()
        .create(true)
        .write(true)
        .open(&paths.lock_file)?;
    lock.try_lock_exclusive()
        .context("another prodstatsd appears to be running")?;

    let (tx, rx) = mpsc::channel::<ActionCounts>();
    let input_opened = if cfg.input.backend == "evdev" {
        start_evdev_threads(cfg.input.clone(), tx).unwrap_or(0)
    } else {
        0
    };
    eprintln!("prodstatsd started; input devices opened: {input_opened}");

    let mut engine = MetricsEngine::new(Local::now());
    engine.set_git_pushes_today_at(
        Local::now(),
        count_today(&paths.git_pushes_csv, Local::now()).unwrap_or(0),
    );
    let mut previous_table = read_process_table().unwrap_or_else(|_| ProcessTable::new(vec![]));
    let mut last = Instant::now();
    let mut minute_start = Local::now()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    let mut minute_actions = 0_u64;
    let mut minute_keys = 0_u64;
    let mut minute_mouse = 0_u64;
    let mut minute_wheel = 0_u64;
    let mut minute_agent_seconds = 0_f64;
    let mut minute_git_pushes = 0_u64;
    let mut active_agent_until: HashMap<String, chrono::DateTime<Local>> = HashMap::new();
    loop {
        let now = Local::now();
        let elapsed = last.elapsed().as_secs_f64().max(0.001);
        last = Instant::now();

        let mut counts = ActionCounts::default();
        while let Ok(c) = rx.try_recv() {
            counts.actions += c.actions;
            counts.keys += c.keys;
            counts.mouse += c.mouse;
            counts.wheel += c.wheel;
        }
        if counts.actions > 0 {
            engine.record_actions_at(now, counts.actions, counts.keys, counts.mouse, counts.wheel);
            minute_actions += counts.actions;
            minute_keys += counts.keys;
            minute_mouse += counts.mouse;
            minute_wheel += counts.wheel;
        }
        engine.record_active_input_time_at(now, elapsed, cfg.input.active_window_seconds as i64);

        let current_table = read_process_table().unwrap_or_else(|_| ProcessTable::new(vec![]));
        let raw_active = detect_active_harnesses(
            &cfg.agents.harnesses,
            &previous_table,
            &current_table,
            cfg.agents.cpu_active_threshold_ms_per_second,
            cfg.agents.io_active_threshold_bytes_per_second,
            elapsed,
        );
        previous_table = current_table;
        let keep_until = now + chrono::Duration::seconds(cfg.agents.activity_window_seconds as i64);
        for name in raw_active {
            active_agent_until.insert(name, keep_until);
        }
        active_agent_until.retain(|_, until| *until >= now);
        let mut active: Vec<String> = active_agent_until.keys().cloned().collect();
        active.sort();
        let active_count_for_minute = active.len() as f64;
        engine.record_agent_sample_at(now, active, elapsed);
        minute_agent_seconds += active_count_for_minute * elapsed;
        if cfg.git.enabled {
            let pushes = count_today(&paths.git_pushes_csv, now).unwrap_or(0);
            minute_git_pushes = minute_git_pushes
                .max(pushes.saturating_sub(engine.snapshot_at(now).git_pushes_today));
            engine.set_git_pushes_today_at(now, pushes);
        }

        let snap = engine.snapshot_at(now);
        let current_minute = now.with_second(0).unwrap().with_nanosecond(0).unwrap();
        if current_minute != minute_start {
            append_minute(
                &paths.history_dir,
                &MinuteStats {
                    timestamp_minute: minute_start,
                    actions: minute_actions,
                    key_presses: minute_keys,
                    mouse_clicks: minute_mouse,
                    wheel_events: minute_wheel,
                    apm: snap.apm,
                    active_agents: snap.active_agents,
                    agent_active_seconds: minute_agent_seconds.round() as u64,
                    git_pushes: minute_git_pushes,
                    total_actions_today: snap.total_actions_today,
                    total_agent_active_seconds_today: snap.agent_active_seconds_today,
                    total_git_pushes_today: snap.git_pushes_today,
                },
            )?;
            minute_start = current_minute;
            minute_actions = 0;
            minute_keys = 0;
            minute_mouse = 0;
            minute_wheel = 0;
            minute_agent_seconds = 0.0;
            minute_git_pushes = 0;
        }
        write_state_atomic(&paths.state_file, &snap)?;
        thread::sleep(Duration::from_millis(cfg.general.sample_interval_ms));
    }
}

fn status(watch: bool, format: &str) -> Result<()> {
    let paths = Paths::new()?;
    loop {
        let state = read_state(&paths.state_file).unwrap_or_else(|e| {
            stale_state(&format!("daemon not running or state unreadable: {e}"))
        });
        if format == "json" {
            println!("{}", serde_json::to_string_pretty(&state)?);
        } else {
            println!("{}", prodstats::display::format_compact(&state));
        }
        if !watch {
            break;
        }
        thread::sleep(Duration::from_secs(2));
    }
    Ok(())
}

fn waybar() -> Result<()> {
    let paths = Paths::new()?;
    match read_state(&paths.state_file) {
        Ok(state) => println!("{}", waybar_json(&state)?),
        Err(e) => println!("{}", stale_waybar_json(&e.to_string())?),
    }
    Ok(())
}

fn log_git_command(exit_code: i32, cwd: String, args: Vec<String>) -> Result<()> {
    if should_log_git_push(exit_code, &args) {
        let paths = Paths::new()?;
        if let Some(event) = GitPushEvent::from_args(cwd, &args) {
            append_event(&paths.git_pushes_csv, &event)?;
        }
    }
    Ok(())
}

fn install(target: InstallTarget) -> Result<()> {
    let exe = std::env::current_exe()?;
    Config::write_default(&Paths::new()?.config)?;
    match target {
        InstallTarget::Omarchy { corner } => print!("{}", install_omarchy(&corner)?),
        InstallTarget::Shell { shell } => println!(
            "Installed shell integration in {}",
            install_shell_rc(shell.as_deref())?.display()
        ),
        InstallTarget::Service => println!(
            "Installed systemd service at {}",
            install_systemd_service(&exe)?.display()
        ),
        InstallTarget::GitShim => println!(
            "Installed git executable shim at {}",
            install_git_shim(&exe)?.display()
        ),
        InstallTarget::InputAccess => print!("{}", install_input_access()?),
        InstallTarget::All { corner } => {
            println!(
                "Installed systemd service at {}",
                install_systemd_service(&exe)?.display()
            );
            println!(
                "Installed shell integration in {}",
                install_shell_rc(None)?.display()
            );
            println!(
                "Installed git executable shim at {}",
                install_git_shim(&exe)?.display()
            );
            print!("{}", install_omarchy(&corner)?);
        }
    }
    Ok(())
}

fn doctor() -> Result<()> {
    let paths = Paths::new()?;
    println!("prodstats doctor");
    println!(
        "config: {} {}",
        paths.config.display(),
        if paths.config.exists() {
            "ok"
        } else {
            "missing; run prodstats install service"
        }
    );
    println!(
        "state: {} {}",
        paths.state_file.display(),
        if paths.state_file.exists() {
            "ok"
        } else {
            "missing; daemon may not be running"
        }
    );
    let input_readable = prodstats::input::discover_devices()
        .into_iter()
        .any(|p| OpenOptions::new().read(true).open(p).is_ok());
    println!(
        "input access: {}",
        if input_readable {
            "ok"
        } else {
            "not available. On Omarchy/Arch: sudo usermod -aG input $USER, then log out/in"
        }
    );
    let svc = Command::new("systemctl")
        .args(["--user", "is-active", "prodstats.service"])
        .output();
    println!(
        "systemd service: {}",
        svc.ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_else(|| "unknown".into())
            .trim()
    );
    let shell_snippet = "prodstats git push tracking";
    let shell_files: Vec<String> = dirs::home_dir()
        .map(|home| vec![home.join(".bashrc"), home.join(".zshrc")])
        .unwrap_or_default()
        .into_iter()
        .filter(|path| {
            fs::read_to_string(path)
                .map(|text| text.contains(shell_snippet))
                .unwrap_or(false)
        })
        .map(|path| path.display().to_string())
        .collect();
    println!(
        "git shell integration: {}",
        if shell_files.is_empty() {
            "not found; run prodstats install shell".to_string()
        } else {
            format!("ok ({})", shell_files.join(", "))
        }
    );
    let shim = dirs::home_dir()
        .map(|home| home.join(".local/bin/git"))
        .filter(|path| {
            fs::read_to_string(path)
                .map(|text| text.contains("prodstats git executable shim"))
                .unwrap_or(false)
        });
    println!(
        "git executable shim: {}",
        shim.map(|path| format!("ok ({})", path.display()))
            .unwrap_or_else(|| "not found; run prodstats install git-shim".to_string())
    );
    Ok(())
}

fn overlay(corner: &str) -> Result<()> {
    let script = r#"#!/usr/bin/env python3
import json, subprocess, sys
try:
    import gi
    gi.require_version('Gtk', '4.0')
    gi.require_version('Gtk4LayerShell', '1.0')
    from gi.repository import Gtk, GLib, Gtk4LayerShell as LayerShell
except Exception as e:
    print('prodstats overlay requires python-gobject, gtk4, and gtk4-layer-shell:', e, file=sys.stderr)
    print('Omarchy fallback: use `prodstats install omarchy --corner __CORNER__` for the Waybar corner module.', file=sys.stderr)
    sys.exit(2)

CORNER = '__CORNER__'

def read_text():
    try:
        out = subprocess.check_output(['prodstats', 'waybar'], text=True, timeout=1)
        return json.loads(out).get('text', 'prodstats')
    except Exception as e:
        return '󰅙 prodstats'

class App(Gtk.Application):
    def do_activate(self):
        win = Gtk.ApplicationWindow(application=self)
        win.set_decorated(False)
        win.set_resizable(False)
        LayerShell.init_for_window(win)
        LayerShell.set_layer(win, LayerShell.Layer.TOP)
        LayerShell.set_namespace(win, 'prodstats')
        for edge in [LayerShell.Edge.TOP, LayerShell.Edge.BOTTOM, LayerShell.Edge.LEFT, LayerShell.Edge.RIGHT]:
            LayerShell.set_anchor(win, edge, False)
        if 'bottom' in CORNER: LayerShell.set_anchor(win, LayerShell.Edge.BOTTOM, True)
        else: LayerShell.set_anchor(win, LayerShell.Edge.TOP, True)
        if 'left' in CORNER: LayerShell.set_anchor(win, LayerShell.Edge.LEFT, True)
        else: LayerShell.set_anchor(win, LayerShell.Edge.RIGHT, True)
        LayerShell.set_margin(win, LayerShell.Edge.TOP, 8)
        LayerShell.set_margin(win, LayerShell.Edge.BOTTOM, 8)
        LayerShell.set_margin(win, LayerShell.Edge.LEFT, 8)
        LayerShell.set_margin(win, LayerShell.Edge.RIGHT, 8)
        self.label = Gtk.Label(label=read_text())
        self.label.set_margin_top(6); self.label.set_margin_bottom(6); self.label.set_margin_start(10); self.label.set_margin_end(10)
        box = Gtk.Box(); box.append(self.label)
        box.add_css_class('prodstats-overlay')
        win.set_child(box)
        css = Gtk.CssProvider(); css.load_from_data(b'.prodstats-overlay { background: alpha(#11111b, .88); color: #cdd6f4; border-radius: 10px; font-weight: 600; }')
        Gtk.StyleContext.add_provider_for_display(win.get_display(), css, Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION)
        GLib.timeout_add_seconds(2, self.refresh)
        win.present()
    def refresh(self):
        self.label.set_text(read_text())
        return True

App(application_id='local.prodstats.overlay').run(sys.argv)
"#.replace("__CORNER__", corner);
    let path = Paths::new()?.data_dir.join(format!("overlay-{corner}.py"));
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(&path, script)?;
    let mut perms = fs::metadata(&path)?.permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms)?;
    }
    println!(
        "Experimental layer-shell overlay written to {}",
        path.display()
    );
    println!("Run it with: {}", path.display());
    println!(
        "If gtk4-layer-shell is unavailable, use the stable Omarchy Waybar integration: prodstats install omarchy --corner {corner}"
    );
    Ok(())
}
