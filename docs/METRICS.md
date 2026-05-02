# Metrics definitions

## APM

APM means actions per minute.

The visible/main APM is StarCraft-style average active APM:

```text
total_actions_today / active_input_seconds_today * 60
```

`rolling_apm` / "Now APM" is the short-term current-rate metric: actions in the recent rolling 60-second window.

Default action classes:

- keyboard key press
- mouse button press
- wheel scroll event

Default exclusions:

- held-key auto-repeat events
- key releases
- mouse movement
- window focus changes

Config:

```toml
[input]
count_keys = true
count_mouse_buttons = true
count_wheel = true
count_pointer_motion = false
count_key_repeats = false
active_window_seconds = 60
```

## Active agent harnesses

An agent harness is a configured root process such as Hermes, Claude, Codex, OpenCode, Aider, Goose, Gemini, or Amp.

A harness is considered active when its root process or one of its descendants has recent measurable CPU or disk I/O activity above threshold.

This is a heuristic. It means "recently doing local work", not "semantically thinking".

Config:

```toml
[agents]
activity_window_seconds = 5
cpu_active_threshold_ms_per_second = 20
io_active_threshold_bytes_per_second = 65536
```

## Agent active time

Every daemon sample adds:

```text
number_of_active_harnesses * elapsed_wall_seconds
```

So if Hermes and Claude are active for the same 60 seconds, agent active time increases by 120 seconds.

## Git pushes today

V1 counts successful `git push` commands, including common global-option forms such as `git -C /repo push`, through:

- a `~/.local/bin/git` executable shim, which catches non-interactive agent/automation pushes when `~/.local/bin` appears before `/usr/bin` in `PATH`
- an interactive shell function fallback installed in `~/.bashrc` or `~/.zshrc`

It does not count:

- GUI git clients
- programs calling `/usr/bin/git` directly while bypassing `PATH`
- failed pushes
- dry-run pushes

The shell wrapper logs after `git push` returns with exit code 0.
