use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn shell_snippet() -> &'static str {
    r#"
# prodstats git push tracking
if command -v prodstats >/dev/null 2>&1; then
  git() {
    command git "$@"
    local rc=$?
    prodstats log-git-command "$rc" "$PWD" "$@" >/dev/null 2>&1 || true
    return "$rc"
  }
fi
"#
}

pub fn install_shell_rc(shell_name: Option<&str>) -> Result<PathBuf> {
    let home = dirs::home_dir().context("home dir not found")?;
    let rc = match shell_name.unwrap_or("") {
        "bash" => home.join(".bashrc"),
        "zsh" => home.join(".zshrc"),
        _ => {
            let shell = std::env::var("SHELL").unwrap_or_default();
            if shell.contains("zsh") {
                home.join(".zshrc")
            } else {
                home.join(".bashrc")
            }
        }
    };
    let old = fs::read_to_string(&rc).unwrap_or_default();
    if !old.contains("prodstats git push tracking") {
        fs::write(&rc, format!("{}\n{}", old, shell_snippet()))?;
    }
    Ok(rc)
}

pub fn systemd_service(binary: &Path) -> String {
    format!(
        "[Unit]\nDescription=Prodstats productivity stats daemon\nAfter=graphical-session.target\n\n[Service]\nExecStart={} daemon\nRestart=on-failure\nRestartSec=3\n\n[Install]\nWantedBy=default.target\n",
        binary.display()
    )
}

pub fn install_systemd_service(binary: &Path) -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .context("config dir not found")?
        .join("systemd/user");
    fs::create_dir_all(&dir)?;
    let path = dir.join("prodstats.service");
    fs::write(&path, systemd_service(binary))?;
    let _ = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .status();
    let _ = Command::new("systemctl")
        .args(["--user", "enable", "--now", "prodstats.service"])
        .status();
    Ok(path)
}

pub fn install_input_access() -> Result<String> {
    let user = std::env::var("USER").context("USER environment variable not set")?;
    let quoted_user = shell_quote(&user);
    let cmd = format!(
        "usermod -aG input {quoted_user} && setfacl -m u:{quoted_user}:rw /dev/input/event*"
    );
    let status = if is_root() {
        Command::new("sh").args(["-c", &cmd]).status()?
    } else if command_exists("pkexec") {
        Command::new("pkexec").args(["sh", "-c", &cmd]).status()?
    } else {
        Command::new("sudo").args(["sh", "-c", &cmd]).status()?
    };
    if !status.success() {
        anyhow::bail!(
            "failed to grant input access. Run manually: sudo usermod -aG input '$USER' && sudo setfacl -m u:$USER:rw /dev/input/event*"
        );
    }
    Ok(format!(
        "Granted current-session ACL on /dev/input/event* and added {user} to the input group. Restart prodstats now; log out/in later so group membership persists without ACLs.\n"
    ))
}

fn command_exists(name: &str) -> bool {
    Command::new("sh")
        .args([
            "-c",
            &format!("command -v {} >/dev/null 2>&1", shell_quote(name)),
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn is_root() -> bool {
    Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .is_some_and(|s| s.trim() == "0")
}

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

pub fn waybar_module_snippet() -> &'static str {
    r#""custom/prodstats": {
  "exec": "prodstats waybar",
  "return-type": "json",
  "interval": 2,
  "tooltip": true,
  "on-click": "xdg-terminal-exec prodstats status",
  "on-click-right": "xdg-terminal-exec prodstats doctor"
}"#
}

pub fn install_omarchy(corner: &str) -> Result<String> {
    let config_dir = dirs::config_dir().context("config dir not found")?;
    let waybar = config_dir.join("waybar/config.jsonc");
    let css = config_dir.join("waybar/style.css");
    let mut msg = String::new();
    if waybar.exists() {
        let backup = waybar.with_extension("jsonc.prodstats.bak");
        if !backup.exists() {
            fs::copy(&waybar, &backup)?;
        }
        let text = fs::read_to_string(&waybar)?;
        if !text.contains("custom/prodstats") {
            fs::write(&waybar, patch_waybar_text(&text, corner))?;
            msg.push_str(&format!(
                "Patched {} (backup {})\n",
                waybar.display(),
                backup.display()
            ));
        } else {
            msg.push_str("Waybar already contains custom/prodstats\n");
        }
    } else {
        msg.push_str(&format!(
            "Waybar config not found at {}. Add this module manually:\n{}\n",
            waybar.display(),
            waybar_module_snippet()
        ));
    }
    if css.exists() {
        let old = fs::read_to_string(&css)?;
        if !old.contains("#custom-prodstats") {
            fs::write(
                &css,
                format!(
                    "{}\n\n#custom-prodstats {{\n  padding: 0 8px;\n  margin: 0 3px;\n  border-radius: 8px;\n}}\n#custom-prodstats.stale {{ color: #f38ba8; }}\n",
                    old
                ),
            )?;
        }
    }
    let _ = Command::new("omarchy-restart-waybar")
        .status()
        .or_else(|_| Command::new("pkill").arg("waybar").status());
    msg.push_str(&format!("Requested corner: {corner}. For true floating overlay, run: prodstats overlay --corner {corner}\n"));
    Ok(msg)
}

fn patch_waybar_text(text: &str, corner: &str) -> String {
    let target = match corner {
        "top-left" | "left" => "modules-left",
        "top-center" | "center" => "modules-center",
        _ => "modules-right",
    };
    let mut out = text.to_string();
    let key = format!("\"{}\": [", target);
    if let Some(idx) = out.find(&key) {
        let insert = idx + key.len();
        out.insert_str(insert, "\"custom/prodstats\", ");
    }
    if let Some(pos) = out.rfind('}') {
        out.insert_str(pos, &format!(",\n  {}\n", waybar_module_snippet()));
    }
    out
}
