# Privacy model

Prodstats is intentionally count-only.

## What is stored

- Per-minute action counts.
- Per-minute APM.
- Active agent harness count and configured harness names.
- Successful git push events: timestamp, repo path, remote, branch, result, source.
- Daily totals.

## What is not stored

- Key names.
- Characters typed.
- Clipboard contents.
- Window titles.
- Browser URLs.
- Terminal command text, except the git shell wrapper detects that a successful command was `git push`.
- Agent prompts or model output.

## Why input permissions are needed

On Wayland, normal applications cannot globally observe input. That is good for security, but it means a productivity counter needs another source.

Prodstats uses `/dev/input/event*` through evdev. This commonly requires the user to be in the `input` group or a dedicated udev rule.

Being in the input group is broad: other programs running as your user could also open raw input devices. Prodstats itself only aggregates counts, but the OS-level permission is still powerful.

If that tradeoff is unacceptable, disable input counting in config:

```toml
[input]
backend = "none"
```

Agent and git stats will still work.
