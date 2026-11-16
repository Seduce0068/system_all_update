use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

static HAD_FAILURE: AtomicBool = AtomicBool::new(false);

struct Options {
    dry_run: bool,
    references_only: bool,
    system_only: bool,
    apps_only: bool,
    install_service: bool,
}

fn print_help() {
    println!("System All Update Rust Script");
    println!("Usage: system-all-update [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --dry-run          Show commands without executing them");
    println!("  --references-only  Update only the references (~/references)");
    println!("  --system-only      Update only system packages (pkg/apt, npm, etc.)");
    println!(
        "  --apps-only        Update only the applications/tools (VS Code, cargo-update, etc.)"
    );
    println!("  --install-service  Install automatic update service (systemd or Termux job)");
    println!("  --help             Show this help message");
    println!();
    println!("Auto-close:");
    println!("  Snaps listed in ~/.config/system-all-update/auto-close-snaps are");
    println!("  closed and refreshed when an update is blocked by running apps.");
}

fn parse_args() -> Result<Options, String> {
    let args: Vec<String> = env::args().collect();
    let mut dry_run = false;
    let mut references_only = false;
    let mut system_only = false;
    let mut apps_only = false;
    let mut install_service = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--dry-run" => dry_run = true,
            "--references-only" => references_only = true,
            "--system-only" => system_only = true,
            "--apps-only" => apps_only = true,
            "--install-service" => install_service = true,
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            other => {
                return Err(format!(
                    "Unknown argument: {}. Use --help for usage.",
                    other
                ));
            }
        }
        i += 1;
    }

    Ok(Options {
        dry_run,
        references_only,
        system_only,
        apps_only,
        install_service,
    })
}

fn run_cmd(name: &str, program: &str, args: &[&str], dry_run: bool) -> bool {
    println!(
        "[INFO] 🚀 Executing: {} ({} {})",
        name,
        program,
        args.join(" ")
    );
    if dry_run {
        println!("[DRY-RUN] Skipped execution.");
        return true;
    }

    let status = Command::new(program).args(args).status();

    match status {
        Ok(s) if s.success() => {
            println!("[SUCCESS] ✅ {} completed successfully.", name);
            true
        }
        Ok(s) => {
            eprintln!("[ERROR] ❌ {} failed with exit code: {:?}", name, s.code());
            HAD_FAILURE.store(true, Ordering::Relaxed);
            false
        }
        Err(e) => {
            eprintln!("[ERROR] ❌ Failed to start command '{}': {}", program, e);
            HAD_FAILURE.store(true, Ordering::Relaxed);
            false
        }
    }
}

/// Runs a command and captures its output for a parallel lane.
fn run_cmd_captured(
    name: &str,
    program: &str,
    args: &[&str],
    dry_run: bool,
    logs: &mut Vec<String>,
) -> bool {
    logs.push(format!(
        "[INFO] 🚀 Executing: {} ({} {})",
        name,
        program,
        args.join(" ")
    ));
    if dry_run {
        logs.push("[DRY-RUN] Skipped execution.".to_string());
        return true;
    }

    match Command::new(program).args(args).output() {
        Ok(output) => {
            if !output.stdout.is_empty() {
                logs.push(
                    String::from_utf8_lossy(&output.stdout)
                        .trim_end()
                        .to_string(),
                );
            }
            if !output.stderr.is_empty() {
                logs.push(
                    String::from_utf8_lossy(&output.stderr)
                        .trim_end()
                        .to_string(),
                );
            }
            if output.status.success() {
                logs.push(format!("[SUCCESS] ✅ {} completed successfully.", name));
                true
            } else {
                logs.push(format!(
                    "[ERROR] ❌ {} failed with exit code: {:?}",
                    name,
                    output.status.code()
                ));
                HAD_FAILURE.store(true, Ordering::Relaxed);
                false
            }
        }
        Err(e) => {
            logs.push(format!(
                "[ERROR] ❌ Failed to start command '{}': {}",
                program, e
            ));
            HAD_FAILURE.store(true, Ordering::Relaxed);
            false
        }
    }
}

/// Runs a command and captures its output without marking the overall run as
/// failed. Used for best-effort steps such as refreshing package lists, where
/// an unrelated third-party repository can return an error.
fn run_cmd_optional(
    name: &str,
    program: &str,
    args: &[&str],
    dry_run: bool,
    logs: &mut Vec<String>,
) -> bool {
    logs.push(format!(
        "[INFO] 🚀 Executing: {} ({} {})",
        name,
        program,
        args.join(" ")
    ));
    if dry_run {
        logs.push("[DRY-RUN] Skipped execution.".to_string());
        return true;
    }

    match Command::new(program).args(args).output() {
        Ok(output) => {
            if !output.stdout.is_empty() {
                logs.push(
                    String::from_utf8_lossy(&output.stdout)
                        .trim_end()
                        .to_string(),
                );
            }
            if !output.stderr.is_empty() {
                logs.push(
                    String::from_utf8_lossy(&output.stderr)
                        .trim_end()
                        .to_string(),
                );
            }
            if output.status.success() {
                logs.push(format!("[SUCCESS] ✅ {} completed successfully.", name));
                true
            } else {
                logs.push(format!(
                    "[WARN] ⚠️  {} failed with exit code: {:?} (continuing)",
                    name,
                    output.status.code()
                ));
                false
            }
        }
        Err(e) => {
            logs.push(format!(
                "[WARN] ⚠️  Failed to start command '{}': {} (continuing)",
                program, e
            ));
            false
        }
    }
}

fn run_cmd_fwupd_captured(
    name: &str,
    program: &str,
    args: &[&str],
    dry_run: bool,
    logs: &mut Vec<String>,
) -> bool {
    logs.push(format!(
        "[INFO] 🚀 Executing: {} ({} {})",
        name,
        program,
        args.join(" ")
    ));
    if dry_run {
        logs.push("[DRY-RUN] Skipped execution.".to_string());
        return true;
    }

    match Command::new(program).args(args).output() {
        Ok(output) => {
            if !output.stdout.is_empty() {
                logs.push(
                    String::from_utf8_lossy(&output.stdout)
                        .trim_end()
                        .to_string(),
                );
            }
            if !output.stderr.is_empty() {
                logs.push(
                    String::from_utf8_lossy(&output.stderr)
                        .trim_end()
                        .to_string(),
                );
            }
            let code = output.status.code().unwrap_or(-1);
            if output.status.success() || code == 2 {
                if code == 2 {
                    logs.push(format!(
                        "[INFO] ℹ️  {} completed: no firmware updates available.",
                        name
                    ));
                } else {
                    logs.push(format!("[SUCCESS] ✅ {} completed successfully.", name));
                }
                true
            } else {
                logs.push(format!(
                    "[ERROR] ❌ {} failed with exit code: {}",
                    name, code
                ));
                HAD_FAILURE.store(true, Ordering::Relaxed);
                false
            }
        }
        Err(e) => {
            logs.push(format!(
                "[ERROR] ❌ Failed to start command '{}': {}",
                program, e
            ));
            HAD_FAILURE.store(true, Ordering::Relaxed);
            false
        }
    }
}

fn print_logs(logs: &[String]) {
    for line in logs {
        println!("{}", line);
    }
}

fn update_references(dry_run: bool) {
    let mut logs = Vec::new();
    update_references_into(dry_run, &mut logs);
    print_logs(&logs);
}

fn update_references_into(dry_run: bool, logs: &mut Vec<String>) -> bool {
    let ref_path = if is_termux() {
        env::var("HOME")
            .map(|home| format!("{}/references", home))
            .unwrap_or_else(|_| "/home/koppe/references".to_string())
    } else {
        "/home/koppe/references".to_string()
    };
    let ref_path = ref_path.as_str();
    logs.push(format!("\n=== 📚 Updating References ({}) ===", ref_path));

    if !Path::new(ref_path).exists() {
        logs.push(format!(
            "[WARN] References path '{}' does not exist. Skipping.",
            ref_path
        ));
        return true;
    }

    let git_dir = format!("{}/.git", ref_path);
    if !Path::new(&git_dir).exists() {
        logs.push(format!(
            "[WARN] References path '{}' is not a git repository. Skipping git pull.",
            ref_path
        ));
        return true;
    }

    run_cmd_captured(
        "References Git Pull",
        "git",
        &["-C", ref_path, "pull"],
        dry_run,
        logs,
    )
}

fn update_system(dry_run: bool) {
    let mut logs = Vec::new();
    if is_termux() {
        update_termux_system(dry_run);
        return;
    }

    logs.push("\n=== 🖥️ Updating System Components ===".to_string());
    update_rustup_into(dry_run, &mut logs);
    update_apt_into(dry_run, &mut logs);
    update_snaps_into(dry_run, &mut logs);
    update_flatpak_into(dry_run, &mut logs);
    update_firmware_into(dry_run, &mut logs);
    update_npm_into(dry_run, &mut logs);
    print_logs(&logs);
}

fn update_rustup_into(dry_run: bool, logs: &mut Vec<String>) -> bool {
    if which_command("rustup") {
        // Nightly is intentionally excluded: only the stable toolchain is
        // updated automatically. Update nightly manually with
        // `rustup update nightly` when needed.
        run_cmd_captured(
            "Rustup Update (stable only)",
            "rustup",
            &["update", "stable"],
            dry_run,
            logs,
        )
    } else {
        logs.push("[INFO] rustup not found, skipping.".to_string());
        true
    }
}

fn update_apt_into(dry_run: bool, logs: &mut Vec<String>) -> bool {
    if !which_command("apt") {
        logs.push(
            "[INFO] apt package manager not found, skipping system package upgrade.".to_string(),
        );
        return true;
    }

    logs.push("[INFO] Running apt update & upgrade...".to_string());
    let apt_success = run_cmd_captured(
        "Apt Update",
        "sudo",
        &["apt", "-o", "DPkg::Lock::Timeout=600", "update"],
        dry_run,
        logs,
    );
    if !apt_success {
        return false;
    }

    let upgrade_ok = run_cmd_captured(
        "Apt Upgrade",
        "sudo",
        &["apt", "-o", "DPkg::Lock::Timeout=600", "upgrade", "-y"],
        dry_run,
        logs,
    );
    let autoremove_ok = run_cmd_captured(
        "Apt Autoremove",
        "sudo",
        &["apt", "-o", "DPkg::Lock::Timeout=600", "autoremove", "-y"],
        dry_run,
        logs,
    );
    upgrade_ok && autoremove_ok
}

fn update_flatpak_into(dry_run: bool, logs: &mut Vec<String>) -> bool {
    if which_command("flatpak") {
        run_cmd_captured(
            "Flatpak Update",
            "flatpak",
            &["update", "-y"],
            dry_run,
            logs,
        )
    } else {
        logs.push("[INFO] flatpak not found, skipping flatpak update.".to_string());
        true
    }
}

fn update_firmware_into(dry_run: bool, logs: &mut Vec<String>) -> bool {
    if !which_command("fwupdmgr") {
        logs.push("[INFO] fwupdmgr not found, skipping firmware updates.".to_string());
        return true;
    }

    logs.push("[INFO] Running firmware updates via fwupdmgr...".to_string());
    // Use --force to avoid interactive prompt when remotes not enabled
    let _ = run_cmd_captured(
        "Fwupd Refresh",
        "fwupdmgr",
        &["refresh", "--force"],
        dry_run,
        logs,
    );
    // fwupdmgr update returns exit code 2 when no updates available (not an error)
    run_cmd_fwupd_captured("Fwupd Update", "fwupdmgr", &["update", "-y"], dry_run, logs)
}

fn update_npm_into(dry_run: bool, logs: &mut Vec<String>) -> bool {
    if which_command("npm") {
        let ok = run_cmd_captured("NPM Global Update", "npm", &["update", "-g"], dry_run, logs);
        // DSH は next タグを使用するため、npm update -g で latest タグに
        // ダウングレードされるのを防ぐ。npm update -g 後に next タグを再インストールする。
        if ok {
            run_cmd_captured(
                "DSH Reinstall (next tag)",
                "npm",
                &["install", "-g", "@deepseek-ai/dsh@next"],
                dry_run,
                logs,
            );
        }
        ok
    } else {
        logs.push("[INFO] npm not found, skipping global npm updates.".to_string());
        true
    }
}

/// Refreshes each pending snap individually so that one running application
/// cannot mask the result of the other packages.
///
/// Snapd reports snaps with running apps as errors (e.g.
/// `snap "bitwarden" has running apps`). Such snaps are deferred and reported
/// at the end instead of being treated as hard failures.
fn update_snaps_into(dry_run: bool, logs: &mut Vec<String>) -> bool {
    logs.push("\n=== 📦 Updating Snap Packages ===".to_string());

    let pending = match pending_snap_refreshes() {
        Some(names) => names,
        None => {
            logs.push(
                "[WARN] Could not query pending snap refreshes; falling back to global refresh."
                    .to_string(),
            );
            return run_cmd_captured("Snap Refresh", "sudo", &["snap", "refresh"], dry_run, logs);
        }
    };

    if pending.is_empty() {
        logs.push("[INFO] ℹ️  No snap updates available.".to_string());
        return true;
    }

    logs.push(format!(
        "[INFO] Pending snap refreshes: {}",
        pending.join(", ")
    ));

    if dry_run {
        for name in &pending {
            logs.push(format!(
                "[DRY-RUN] Would execute: sudo snap refresh {}",
                name
            ));
        }
        return true;
    }

    let mut refreshed: Vec<String> = Vec::new();
    let mut deferred: Vec<String> = Vec::new();
    let mut failed: Vec<String> = Vec::new();
    let auto_close = auto_close_snaps();

    for name in &pending {
        logs.push(format!(
            "[INFO] 🚀 Executing: Snap Refresh ({}) (sudo snap refresh {})",
            name, name
        ));
        let mut outcome = refresh_single_snap_into(name, logs);

        if matches!(outcome, SnapRefreshOutcome::Deferred)
            && auto_close.iter().any(|allowed| allowed == name)
        {
            match close_snap_apps(name) {
                Ok(stopped) if !stopped.is_empty() => {
                    logs.push(format!(
                        "[INFO] 🔄 Closed {} ({}) and retrying refresh.",
                        name,
                        stopped.join(", ")
                    ));
                    thread::sleep(Duration::from_secs(1));
                    outcome = refresh_single_snap_into(name, logs);

                    // snapd may already be applying its own auto-refresh in
                    // the background, in which case the explicit retry can
                    // fail with a "change in progress" style error.
                    if matches!(outcome, SnapRefreshOutcome::Failed)
                        && wait_until_snap_refreshed(name, Duration::from_secs(60))
                    {
                        logs.push(format!("[INFO] ℹ️  {} was refreshed by snapd.", name));
                        outcome = SnapRefreshOutcome::Refreshed;
                    }
                }
                Ok(_) => {
                    logs.push(format!(
                        "[WARN] ⚠️  No active user scope found for {}; leaving it deferred.",
                        name
                    ));
                }
                Err(e) => {
                    logs.push(format!("[WARN] ⚠️  Could not close {}: {}", name, e));
                }
            }
        }

        match outcome {
            SnapRefreshOutcome::Refreshed => {
                logs.push(format!("[SUCCESS] ✅ {} refreshed successfully.", name));
                refreshed.push(name.clone());
            }
            SnapRefreshOutcome::Deferred => {
                logs.push(format!(
                    "[WARN] ⚠️  {} was not refreshed because the app is running. Deferred.",
                    name
                ));
                deferred.push(name.clone());
            }
            SnapRefreshOutcome::Failed => {
                logs.push(format!("[ERROR] ❌ {} could not be refreshed.", name));
                failed.push(name.clone());
            }
        }
    }

    if !deferred.is_empty() {
        logs.push("[INFO] ℹ️  Close the following apps, then refresh manually:".to_string());
        for name in &deferred {
            logs.push(format!("  - {}", name));
        }
        logs.push(format!("[HINT] sudo snap refresh {}", deferred.join(" ")));
    }
    if !failed.is_empty() {
        logs.push(format!("[INFO] ℹ️  Failed snaps: {}", failed.join(", ")));
    }
    logs.push(format!(
        "[INFO] Snap refresh summary: {} refreshed, {} deferred, {} failed.",
        refreshed.len(),
        deferred.len(),
        failed.len()
    ));

    failed.is_empty()
}

enum SnapRefreshOutcome {
    Refreshed,
    Deferred,
    Failed,
}

/// Returns the names of snaps with a pending refresh, or `None` when the
/// query itself failed (offline, snapd unavailable, etc.).
fn pending_snap_refreshes() -> Option<Vec<String>> {
    let output = Command::new("snap")
        .args(["refresh", "--list"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut names = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty()
            || line.starts_with("Name")
            || line.starts_with("All snaps up to date")
            || line.starts_with("No snaps")
        {
            continue;
        }
        if let Some(name) = line.split_whitespace().next() {
            names.push(name.to_string());
        }
    }
    Some(names)
}

/// Runs `sudo -n snap refresh <name>` and captures snapd's output.
fn capture_sudo_snap_refresh(name: &str) -> std::io::Result<std::process::Output> {
    Command::new("sudo")
        .args(["-n", "snap", "refresh", name])
        .output()
}

/// True when sudo refused to run because credentials were not cached.
/// Only then is an interactive `sudo -v` retry worthwhile.
fn needs_authentication(output: &std::process::Output) -> bool {
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    stderr.contains("a password is required")
        || stderr.contains("no tty present")
        || stderr.contains("a terminal is required")
        || stderr.contains("interactive authentication is required")
}

fn refresh_single_snap_into(name: &str, logs: &mut Vec<String>) -> SnapRefreshOutcome {
    let mut output = match capture_sudo_snap_refresh(name) {
        Ok(output) => output,
        Err(e) => {
            logs.push(format!(
                "[ERROR] ❌ Failed to execute 'sudo snap refresh {}': {}",
                name, e
            ));
            return SnapRefreshOutcome::Failed;
        }
    };

    // Sudo may be configured with NOPASSWD for `snap` only (common on
    // systems that run this script from systemd). Try the cached/non-
    // interactive path first, and ask for a password only when sudo
    // explicitly reports that authentication is needed.
    if !output.status.success() && needs_authentication(&output) {
        logs.push("[INFO] 🔐 sudo authentication required for snap refresh.".to_string());
        let authenticated = Command::new("sudo")
            .arg("-v")
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !authenticated {
            logs.push(format!(
                "[ERROR] ❌ sudo authentication failed; cannot refresh {}.",
                name
            ));
            return SnapRefreshOutcome::Failed;
        }
        output = match capture_sudo_snap_refresh(name) {
            Ok(output) => output,
            Err(e) => {
                logs.push(format!(
                    "[ERROR] ❌ Failed to execute 'sudo snap refresh {}': {}",
                    name, e
                ));
                return SnapRefreshOutcome::Failed;
            }
        };
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !stdout.is_empty() {
        logs.push(stdout.trim_end().to_string());
    }
    if !stderr.is_empty() {
        logs.push(stderr.trim_end().to_string());
    }

    if output.status.success() {
        return SnapRefreshOutcome::Refreshed;
    }

    let combined = format!("{}{}", stdout, stderr).to_lowercase();
    if combined.contains("has running apps") || combined.contains("is in use") {
        SnapRefreshOutcome::Deferred
    } else {
        SnapRefreshOutcome::Failed
    }
}

/// Snaps the user allowed the updater to close when a refresh is blocked.
/// The allowlist lives outside the repository so a fresh checkout never
/// terminates applications without an explicit opt-in.
fn auto_close_snaps() -> Vec<String> {
    let home = match env::var("HOME") {
        Ok(home) => home,
        Err(_) => return Vec::new(),
    };
    let path = Path::new(&home).join(".config/system-all-update/auto-close-snaps");
    match fs::read_to_string(path) {
        Ok(contents) => contents
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(str::to_string)
            .collect(),
        Err(_) => Vec::new(),
    }
}

fn current_uid() -> Option<u32> {
    let output = Command::new("id").arg("-u").output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout).trim().parse().ok()
}

fn apply_user_manager_env(command: &mut Command, uid: u32) {
    let runtime_dir = format!("/run/user/{}", uid);
    command.env("XDG_RUNTIME_DIR", &runtime_dir).env(
        "DBUS_SESSION_BUS_ADDRESS",
        format!("unix:path={}/bus", runtime_dir),
    );
}

/// Stops the systemd user scopes belonging to `name` so snapd can refresh it.
/// Returns the stopped unit names; an empty list means no scope was running.
fn close_snap_apps(name: &str) -> Result<Vec<String>, String> {
    let uid = current_uid().ok_or_else(|| "could not determine the current uid".to_string())?;
    let bus = format!("/run/user/{}/bus", uid);
    if !Path::new(&bus).exists() {
        return Err(format!("user session bus {} is not available", bus));
    }

    let pattern = format!("snap.{}.*.scope", name);
    let mut list = Command::new("systemctl");
    list.args([
        "--user",
        "list-units",
        "--type=scope",
        "--all",
        "--no-legend",
        "--plain",
        "--no-pager",
        &pattern,
    ]);
    apply_user_manager_env(&mut list, uid);
    let output = list
        .output()
        .map_err(|e| format!("failed to run systemctl: {}", e))?;
    if !output.status.success() {
        return Err(format!(
            "systemctl list-units failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let mut units = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let mut fields = line.split_whitespace();
        let unit = fields.next().unwrap_or("");
        let load = fields.next().unwrap_or("");
        let active = fields.next().unwrap_or("");
        if unit.ends_with(".scope") && load == "loaded" && active == "active" {
            units.push(unit.to_string());
        }
    }

    for unit in &units {
        println!("[INFO] 🛑 Stopping {} to release {}.", unit, name);
        let mut stop = Command::new("systemctl");
        stop.args(["--user", "stop", unit]);
        apply_user_manager_env(&mut stop, uid);
        let status = stop
            .status()
            .map_err(|e| format!("failed to stop {}: {}", unit, e))?;
        if !status.success() {
            return Err(format!("systemctl stop {} failed", unit));
        }
    }

    Ok(units)
}

fn snap_is_pending(name: &str) -> bool {
    match pending_snap_refreshes() {
        Some(pending) => pending.iter().any(|pending_name| pending_name == name),
        None => true,
    }
}

fn wait_until_snap_refreshed(name: &str, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if !snap_is_pending(name) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_secs(2));
    }
}

fn update_apps(dry_run: bool) {
    let mut logs = Vec::new();
    if is_termux() {
        update_termux_apps(dry_run);
        return;
    }

    logs.push("\n=== 📱 Updating Applications / Tools ===".to_string());
    update_vscode_into(dry_run, &mut logs, true);
    update_cargo_into(dry_run, &mut logs);
    print_logs(&logs);
}

fn update_vscode_into(dry_run: bool, logs: &mut Vec<String>, handle_flatpak: bool) -> bool {
    if !which_command("code") {
        logs.push("[INFO] VS Code CLI ('code') not found in PATH.".to_string());
        return true;
    }

    logs.push("[INFO] VS Code (code CLI) is available.".to_string());
    if let Ok(output) = Command::new("code").arg("--version").output() {
        if !output.stdout.is_empty() {
            logs.push(
                String::from_utf8_lossy(&output.stdout)
                    .trim_end()
                    .to_string(),
            );
        }
        if !output.stderr.is_empty() {
            logs.push(
                String::from_utf8_lossy(&output.stderr)
                    .trim_end()
                    .to_string(),
            );
        }
    }

    if is_installed_via_apt("code") {
        logs.push("[INFO] VS Code detected as apt package. Updating via apt...".to_string());
        let update_ok = run_cmd_optional(
            "Apt Update (for VS Code)",
            "sudo",
            &["apt", "-o", "DPkg::Lock::Timeout=600", "update"],
            dry_run,
            logs,
        );
        if !update_ok {
            logs.push(
                "[WARN] apt update failed (an unrelated repository may be unavailable); continuing with the existing package lists."
                    .to_string(),
            );
        }
        return run_cmd_captured(
            "VS Code Upgrade",
            "sudo",
            &[
                "apt",
                "-o",
                "DPkg::Lock::Timeout=600",
                "upgrade",
                "-y",
                "code",
            ],
            dry_run,
            logs,
        );
    }

    if which_command("snap") && is_installed_via_snap("code") {
        logs.push("[INFO] VS Code detected as snap package. Updating via snap...".to_string());
        return run_cmd_captured(
            "VS Code Snap Refresh",
            "sudo",
            &["snap", "refresh", "code"],
            dry_run,
            logs,
        );
    }

    if which_command("flatpak") && is_installed_via_flatpak("com.visualstudio.code") {
        if !handle_flatpak {
            logs.push(
                "[INFO] VS Code is flatpak-managed; the flatpak lane will update it.".to_string(),
            );
            return true;
        }
        logs.push("[INFO] VS Code detected as flatpak. Updating via flatpak...".to_string());
        return run_cmd_captured(
            "VS Code Flatpak Update",
            "flatpak",
            &["update", "-y", "com.visualstudio.code"],
            dry_run,
            logs,
        );
    }

    logs.push(
        "[INFO] VS Code installation method not detected (standalone/.deb manual). Skipping auto-update."
            .to_string(),
    );
    logs.push("[HINT] Update manually from https://code.visualstudio.com/".to_string());
    true
}

fn update_cargo_into(dry_run: bool, logs: &mut Vec<String>) -> bool {
    if !which_command("cargo") {
        return true;
    }

    let output = Command::new("cargo")
        .args(["install-update", "--version"])
        .output();

    let Ok(o) = output else {
        return true;
    };
    if !o.status.success() {
        logs.push(
            "[INFO] cargo-update (cargo install-update) not installed. To install: cargo install cargo-update"
                .to_string(),
        );
        return true;
    }

    let updated = run_cmd_captured(
        "Cargo Installed Binaries Update",
        "cargo",
        &["install-update", "-a"],
        dry_run,
        logs,
    );
    // A crates.io connection reset is often transient; retry exactly once.
    if !updated && !dry_run {
        logs.push(
            "[INFO] 🔁 Retrying cargo install-update once (possible transient network error)..."
                .to_string(),
        );
        return run_cmd_captured(
            "Cargo Installed Binaries Update (retry)",
            "cargo",
            &["install-update", "-a"],
            false,
            logs,
        );
    }
    updated
}

fn install_systemd_services(dry_run: bool) {
    println!("\n=== ⚙️ Installing Automatic Update Service ===");

    if is_termux() {
        install_termux_job(dry_run);
        return;
    }

    let exe_path = match env::current_exe() {
        Ok(path) => path.to_string_lossy().into_owned(),
        Err(e) => {
            eprintln!("[ERROR] Failed to get current executable path: {}", e);
            return;
        }
    };

    let current_user = env::var("USER").unwrap_or_else(|_| "koppe".to_string());
    let home_dir = env::var("HOME").unwrap_or_else(|_| format!("/home/{}", current_user));
    // systemd starts the service with a minimal PATH, so rustup/cargo in the
    // user's home would otherwise be invisible.
    let service_path = format!(
        "{home}/.cargo/bin:{home}/.local/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
        home = home_dir
    );

    let boot_service = format!(
        r#"[Unit]
Description=Run system-all-update on system boot
After=network-online.target
Wants=network-online.target

[Service]
Type=oneshot
User={user}
Environment=HOME={home}
Environment=PATH={path}
ExecStart={exe}

[Install]
WantedBy=multi-user.target
"#,
        user = current_user,
        home = home_dir,
        path = service_path,
        exe = exe_path
    );

    let resume_service = format!(
        r#"[Unit]
Description=Run system-all-update on resume from suspend
After=suspend.target hibernate.target hybrid-sleep.target suspend-then-hibernate.target

[Service]
Type=oneshot
User={user}
Environment=HOME={home}
Environment=PATH={path}
ExecStart={exe}

[Install]
WantedBy=suspend.target hibernate.target hybrid-sleep.target suspend-then-hibernate.target
"#,
        user = current_user,
        home = home_dir,
        path = service_path,
        exe = exe_path
    );

    let boot_path = "/tmp/system-all-update-boot.service";
    let resume_path = "/tmp/system-all-update-resume.service";

    if let Err(e) = fs::write(boot_path, boot_service) {
        eprintln!("[ERROR] Failed to write temporary boot service file: {}", e);
        return;
    }
    if let Err(e) = fs::write(resume_path, resume_service) {
        eprintln!(
            "[ERROR] Failed to write temporary resume service file: {}",
            e
        );
        return;
    }

    if dry_run {
        println!("[DRY-RUN] Would move service files to /etc/systemd/system/ and enable them.");
        return;
    }

    // Move to /etc/systemd/system/ using sudo
    if !run_cmd(
        "Install Boot Service",
        "sudo",
        &[
            "mv",
            boot_path,
            "/etc/systemd/system/system-all-update-boot.service",
        ],
        false,
    ) {
        return;
    }
    if !run_cmd(
        "Install Resume Service",
        "sudo",
        &[
            "mv",
            resume_path,
            "/etc/systemd/system/system-all-update-resume.service",
        ],
        false,
    ) {
        return;
    }

    // Reload daemon and enable services
    run_cmd(
        "Systemd Daemon Reload",
        "sudo",
        &["systemctl", "daemon-reload"],
        false,
    );
    run_cmd(
        "Enable Boot Service",
        "sudo",
        &["systemctl", "enable", "system-all-update-boot.service"],
        false,
    );
    run_cmd(
        "Enable Resume Service",
        "sudo",
        &["systemctl", "enable", "system-all-update-resume.service"],
        false,
    );

    println!("[SUCCESS] 🎉 systemd services successfully installed and enabled!");
}

fn is_termux() -> bool {
    cfg!(target_os = "android")
        || env::var("PREFIX")
            .map(|prefix| prefix == "/data/data/com.termux/files/usr")
            .unwrap_or(false)
}

/// Termux では sudo がなく、apt を pkg 経由で扱う。
fn update_termux_system(dry_run: bool) {
    println!("[INFO] Termux environment detected; using pkg.");

    let prefix =
        env::var("PREFIX").unwrap_or_else(|_| "/data/data/com.termux/files/usr".to_string());
    let pkg = format!("{}/bin/pkg", prefix);
    let npm = format!("{}/bin/npm", prefix);

    let updated = run_cmd(
        "Termux pkg update",
        &pkg,
        &["update", "-y", "-o", "DPkg::Lock::Timeout=600"],
        dry_run,
    );
    if updated {
        run_cmd(
            "Termux pkg upgrade",
            &pkg,
            &[
                "upgrade",
                "-y",
                "-o",
                "DPkg::Lock::Timeout=600",
                "-o",
                "Dpkg::Options::=--force-confdef",
                "-o",
                "Dpkg::Options::=--force-confold",
            ],
            dry_run,
        );
    }
    run_cmd(
        "Termux pkg autoclean",
        &pkg,
        &["autoclean", "-o", "DPkg::Lock::Timeout=600"],
        dry_run,
    );

    if Path::new(&npm).exists() {
        run_cmd("NPM Global Update", &npm, &["update", "-g"], dry_run);
        // DSH は next タグを使用するため、npm update -g で latest タグに
        // ダウングレードされるのを防ぐ。
        run_cmd(
            "DSH Reinstall (next tag)",
            &npm,
            &["install", "-g", "@deepseek-ai/dsh@next"],
            dry_run,
        );
        fix_node_cli_shebangs(dry_run);
    } else {
        println!("[INFO] npm not found, skipping global npm updates.");
    }
}

/// Termux には VS Code / snap / flatpak がないため、cargo 管理分のみ扱う。
fn update_termux_apps(dry_run: bool) {
    let home = env::var("HOME").unwrap_or_default();
    let cargo_update = format!("{}/.cargo/bin/cargo-install-update", home);

    if Path::new(&cargo_update).exists() {
        let updated = run_cmd(
            "Cargo Installed Binaries Update",
            &cargo_update,
            &["install-update", "-a"],
            dry_run,
        );
        if !updated && !dry_run {
            println!("[INFO] 🔁 Retrying cargo install-update once...");
            run_cmd(
                "Cargo Installed Binaries Update (retry)",
                &cargo_update,
                &["install-update", "-a"],
                false,
            );
        }
    } else {
        println!("[INFO] cargo-update not installed. To install: cargo install cargo-update");
    }
}

/// npm 12 が生成する `#!/usr/bin/env node` は Termux の `/usr/bin/env`
/// 不在で起動できない。既知のグローバル CLI の shebang を絶対パスへ直す。
fn fix_node_cli_shebangs(dry_run: bool) {
    let prefix = match env::var("PREFIX") {
        Ok(prefix) => prefix,
        Err(_) => return,
    };
    let node = format!("{}/bin/node", prefix);
    let names = [
        "eslint",
        "markdownlint",
        "gemini",
        "cline",
        "lsp-mcp-server",
        "mcp-server-memory",
        "zcode",
    ];

    for name in names {
        let link = format!("{}/bin/{}", prefix, name);
        let target = match fs::canonicalize(&link) {
            Ok(target) => target,
            Err(_) => continue,
        };
        let content = match fs::read_to_string(&target) {
            Ok(content) => content,
            Err(_) => continue,
        };
        if !content.starts_with("#!/usr/bin/env node") {
            continue;
        }
        if dry_run {
            println!("[DRY-RUN] Would fix shebang: {}", target.display());
            continue;
        }
        let new_content = content.replacen("#!/usr/bin/env node", &format!("#!{}", node), 1);
        match fs::write(&target, new_content) {
            Ok(_) => println!("[SUCCESS] ✅ Fixed shebang: {}", target.display()),
            Err(e) => eprintln!("[ERROR] ❌ Failed to fix {}: {}", target.display(), e),
        }
    }
}

/// Termux には systemd がないため、Android の JobScheduler を使う。
/// 実行間隔は約 24 時間で、正確な時刻指定はできない。
fn install_termux_job(dry_run: bool) {
    let home = match env::var("HOME") {
        Ok(home) => home,
        Err(_) => {
            eprintln!("[ERROR] HOME is not set.");
            return;
        }
    };
    let config_dir = format!("{}/.config/system-all-update", home);
    let script_path = format!("{}/termux-job.rs", config_dir);
    let log_dir = format!("{}/.local/share/system-all-update", home);
    let prefix =
        env::var("PREFIX").unwrap_or_else(|_| "/data/data/com.termux/files/usr".to_string());
    let scheduler = format!("{}/bin/termux-job-scheduler", prefix);
    let script = include_str!("../termux-job.rs");

    if dry_run {
        println!(
            "[DRY-RUN] Would write {} and schedule a Termux job.",
            script_path
        );
        return;
    }

    if let Err(e) = fs::create_dir_all(&config_dir) {
        eprintln!("[ERROR] Failed to create {}: {}", config_dir, e);
        return;
    }
    if let Err(e) = fs::write(&script_path, script) {
        eprintln!("[ERROR] Failed to write {}: {}", script_path, e);
        return;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755));
    }

    run_cmd(
        "Termux Job Scheduler",
        &scheduler,
        &[
            "--script",
            script_path.as_str(),
            "--job-id",
            "1",
            "--period-ms",
            "86400000",
            "--network",
            "any",
            "--battery-not-low",
            "false",
            "--persisted",
            "true",
        ],
        false,
    );

    println!("[INFO] Daily job scheduled at approximately 24h intervals.");
    println!("[INFO] Log: {}/job.log", log_dir);
}

fn which_command(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn is_installed_via_apt(package: &str) -> bool {
    Command::new("dpkg")
        .args(["-l", package])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn is_installed_via_snap(package: &str) -> bool {
    Command::new("snap")
        .args(["list", package])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn is_installed_via_flatpak(app_id: &str) -> bool {
    Command::new("flatpak")
        .args(["list", "--app"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains(app_id))
        .unwrap_or(false)
}

struct LaneResult {
    name: &'static str,
    logs: Vec<String>,
    failed: bool,
}

/// Caches sudo credentials once so parallel lanes do not race on a prompt.
fn ensure_sudo(dry_run: bool) {
    if dry_run || is_termux() {
        return;
    }

    let cached = Command::new("sudo")
        .args(["-n", "true"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if cached {
        return;
    }

    println!("[INFO] 🔐 sudo authentication is required once before parallel updates.");
    let _ = Command::new("sudo").arg("-v").status();
}

fn run_all_parallel(dry_run: bool) {
    ensure_sudo(dry_run);

    let (system_lane, user_lane, misc_lane) = thread::scope(|scope| {
        let system_lane = scope.spawn(|| {
            let mut logs = Vec::new();
            logs.push("\n=== 🖥️ System Packages Lane ===".to_string());
            let apt_ok = update_apt_into(dry_run, &mut logs);
            let vscode_ok = update_vscode_into(dry_run, &mut logs, false);
            let snap_ok = update_snaps_into(dry_run, &mut logs);
            LaneResult {
                name: "System packages",
                logs,
                failed: !(apt_ok && vscode_ok && snap_ok),
            }
        });

        let user_lane = scope.spawn(|| {
            let mut logs = Vec::new();
            logs.push("\n=== 👤 User Space Lane ===".to_string());
            let rustup_ok = update_rustup_into(dry_run, &mut logs);
            let npm_ok = update_npm_into(dry_run, &mut logs);
            let cargo_ok = update_cargo_into(dry_run, &mut logs);
            LaneResult {
                name: "User space",
                logs,
                failed: !(rustup_ok && npm_ok && cargo_ok),
            }
        });

        let misc_lane = scope.spawn(|| {
            let mut logs = Vec::new();
            logs.push("\n=== 📚 References & Flatpak Lane ===".to_string());
            let refs_ok = update_references_into(dry_run, &mut logs);
            let flatpak_ok = update_flatpak_into(dry_run, &mut logs);
            LaneResult {
                name: "References & flatpak",
                logs,
                failed: !(refs_ok && flatpak_ok),
            }
        });

        (
            system_lane.join().unwrap(),
            user_lane.join().unwrap(),
            misc_lane.join().unwrap(),
        )
    });

    for lane in [system_lane, user_lane, misc_lane] {
        let marker = if lane.failed { "⚠️" } else { "✅" };
        println!("\n=== {} Lane: {} ===", marker, lane.name);
        print_logs(&lane.logs);
        if lane.failed {
            eprintln!("[ERROR] ❌ Lane '{}' reported failures.", lane.name);
        }
    }

    // Firmware updates run alone: they may require a reboot and must not
    // overlap with other package managers.
    let mut firmware_logs = Vec::new();
    firmware_logs.push("\n=== ⚙️ Firmware Updates (single lane) ===".to_string());
    update_firmware_into(dry_run, &mut firmware_logs);
    print_logs(&firmware_logs);
}

fn main() {
    let opts = match parse_args() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("[ERROR] {}", e);
            std::process::exit(1);
        }
    };

    println!("========================================");
    println!("   System, References & Apps Updater    ");
    println!("========================================");

    if opts.install_service {
        install_systemd_services(opts.dry_run);
        return;
    }

    let run_all = !opts.references_only && !opts.system_only && !opts.apps_only;

    if run_all && !is_termux() {
        run_all_parallel(opts.dry_run);
    } else {
        if run_all || opts.references_only {
            update_references(opts.dry_run);
        }

        if run_all || opts.system_only {
            update_system(opts.dry_run);
        }

        if run_all || opts.apps_only {
            update_apps(opts.dry_run);
        }
    }

    println!("\n========================================");
    println!("       🎉 All Update Tasks Done!        ");
    println!("========================================");

    if HAD_FAILURE.load(Ordering::Relaxed) {
        eprintln!("[WARN] ⚠️  Some update tasks failed. See the log above.");
        std::process::exit(1);
    }
}
