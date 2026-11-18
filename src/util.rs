use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Get the user's home directory.
pub fn home_dir() -> PathBuf {
    dirs_home().unwrap_or_else(|| PathBuf::from("/home/rvs"))
}

fn dirs_home() -> Option<PathBuf> {
    if let Ok(sudo_user) = std::env::var("SUDO_USER")
        && !sudo_user.is_empty()
        && sudo_user != "root"
    {
        let user_home = PathBuf::from(format!("/home/{}", sudo_user));
        if user_home.exists() {
            return Some(user_home);
        }
    }
    std::env::var_os("HOME").map(PathBuf::from)
}

/// Reads the user's LINUX_PASSWORD from ~/.credentials if it exists and is enabled in config.
pub fn get_sudo_password() -> Option<String> {
    let cfg = crate::config::UserStateConfig::load();
    if !cfg.use_credentials_file {
        return None;
    }

    let cred_path = home_dir().join(".credentials");
    if let Ok(content) = fs::read_to_string(&cred_path) {
        for line in content.lines() {
            let line = line.trim();
            let clean_line = line.strip_prefix("export ").unwrap_or(line);
            if clean_line.starts_with("LINUX_PASSWORD=") {
                let pass = clean_line
                    .trim_start_matches("LINUX_PASSWORD=")
                    .trim_matches('"')
                    .trim_matches('\'');
                if !pass.is_empty() {
                    return Some(pass.to_string());
                }
            }
        }
    }
    None
}

/// Executes a command with elevated privileges using:
/// 1. Direct execution if already root (euid == 0)
/// 2. Passwordless sudo (`sudo -n`) - fastest, zero prompts when installed via sudoers
/// 3. Native Omarchy desktop polkit prompt (`pkexec`)
/// 4. Optional ~/.credentials fallback (if enabled by config)
/// 5. Fallback to `run0` or interactive `sudo`
pub fn run_elevated(cmd: &str, args: &[&str]) -> Result<String> {
    // 1. If we are already root
    if unsafe { libc::geteuid() } == 0 {
        let output = Command::new(cmd)
            .args(args)
            .output()
            .with_context(|| format!("Failed to run root command: {} {:?}", cmd, args))?;
        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).to_string());
        } else {
            let err = String::from_utf8_lossy(&output.stderr).to_string();
            anyhow::bail!("Command failed ({}): {}", output.status, err);
        }
    }

    // 2. Try passwordless sudo directly for this exact command
    let nopasswd_sudo = Command::new("sudo").arg("-n").arg(cmd).args(args).output();

    if let Ok(res) = nopasswd_sudo
        && res.status.success()
    {
        return Ok(String::from_utf8_lossy(&res.stdout).to_string());
    }

    // 3. Try native Omarchy graphical authentication window via pkexec if available
    if which::which("pkexec").is_ok()
        && (std::env::var_os("WAYLAND_DISPLAY").is_some() || std::env::var_os("DISPLAY").is_some())
        && let Ok(output) = Command::new("pkexec").arg(cmd).args(args).output()
        && output.status.success()
    {
        return Ok(String::from_utf8_lossy(&output.stdout).to_string());
    }

    // 4. Try sudo with credentials password ONLY IF explicitly enabled
    if let Some(pass) = get_sudo_password() {
        use std::io::Write;
        let mut child = Command::new("sudo")
            .args(["-S", cmd])
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to spawn sudo -S {} {:?}", cmd, args))?;

        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(pass.as_bytes());
            let _ = stdin.write_all(b"\n");
        }

        let output = child
            .wait_with_output()
            .with_context(|| format!("Failed waiting for sudo -S {} {:?}", cmd, args))?;

        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).to_string());
        } else {
            let err = String::from_utf8_lossy(&output.stderr).to_string();
            anyhow::bail!("Sudo execution failed: {}", err);
        }
    }

    // 5. Fallback to run0 if available
    if which::which("run0").is_ok() {
        let output = Command::new("run0")
            .arg(cmd)
            .args(args)
            .output()
            .with_context(|| format!("Failed to run run0 {} {:?}", cmd, args))?;
        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).to_string());
        }
    }

    // 6. Interactive sudo (terminal fallback)
    if let Ok(res) = Command::new("sudo").arg(cmd).args(args).output()
        && res.status.success()
    {
        return Ok(String::from_utf8_lossy(&res.stdout).to_string());
    }

    anyhow::bail!(
        "No privilege escalation mechanism succeeded for: {} {:?}",
        cmd,
        args
    );
}

/// Helper to read Omarchy theme accent color from disk
pub fn get_omarchy_accent() -> String {
    let colors_path = home_dir().join(".local/state/omarchy/current/theme/colors.toml");
    if let Ok(content) = fs::read_to_string(&colors_path) {
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("accent =") || line.starts_with("accent=") {
                let val = line
                    .split('=')
                    .nth(1)
                    .unwrap_or("")
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'');
                if !val.is_empty() {
                    return val.to_string();
                }
            }
        }
    }
    // Fallback Omarchy blue accent
    "#7aa2f7".to_string()
}
