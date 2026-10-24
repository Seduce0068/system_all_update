use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

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
    println!("  --references-only  Update only the references (/home/koppe/references)");
    println!("  --system-only      Update only the system (rustup, apt, snap, npm)");
    println!(
        "  --apps-only        Update only the applications/tools (VS Code, cargo-update, etc.)"
    );
    println!("  --install-service  Install systemd boot & resume services for automatic updates");
    println!("  --help             Show this help message");
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
            false
        }
        Err(e) => {
            eprintln!("[ERROR] ❌ Failed to start command '{}': {}", program, e);
            false
        }
    }
}

fn run_cmd_fwupd(name: &str, program: &str, args: &[&str], dry_run: bool) -> bool {
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
        Ok(s) => {
            let code = s.code().unwrap_or(-1);
            // fwupdmgr returns exit code 2 when no updates available (success, just nothing to do)
            if s.success() || code == 2 {
                if code == 2 {
                    println!(
                        "[INFO] ℹ️  {} completed: no firmware updates available.",
                        name
                    );
                } else {
                    println!("[SUCCESS] ✅ {} completed successfully.", name);
                }
                true
            } else {
                eprintln!("[ERROR] ❌ {} failed with exit code: {}", name, code);
                false
            }
        }
        Err(e) => {
            eprintln!("[ERROR] ❌ Failed to start command '{}': {}", program, e);
            false
        }
    }
}

fn update_references(dry_run: bool) {
    let ref_path = "/home/koppe/references";
    println!("\n=== 📚 Updating References ({}) ===", ref_path);

    if !Path::new(ref_path).exists() {
        println!(
            "[WARN] References path '{}' does not exist. Skipping.",
            ref_path
        );
        return;
    }

    let git_dir = format!("{}/.git", ref_path);
    if !Path::new(&git_dir).exists() {
        println!(
            "[WARN] References path '{}' is not a git repository. Skipping git pull.",
            ref_path
        );
        return;
    }

    run_cmd(
        "References Git Pull",
        "git",
        &["-C", ref_path, "pull"],
        dry_run,
    );
}

fn update_system(dry_run: bool) {
    println!("\n=== 🖥️ Updating System Components ===");

    // 1. Rustup update
    if which_command("rustup") {
        run_cmd("Rustup Update", "rustup", &["update"], dry_run);
    } else {
        println!("[INFO] rustup not found, skipping.");
    }

    // 2. Apt update & upgrade (Debian/Ubuntu)
    if which_command("apt") {
        println!("[INFO] Running apt update & upgrade...");
        let apt_success = run_cmd(
            "Apt Update",
            "sudo",
            &["apt", "-o", "DPkg::Lock::Timeout=600", "update"],
            dry_run,
        );
        if apt_success {
            run_cmd(
                "Apt Upgrade",
                "sudo",
                &["apt", "-o", "DPkg::Lock::Timeout=600", "upgrade", "-y"],
                dry_run,
            );
            // Autoremove unused dependencies
            run_cmd(
                "Apt Autoremove",
                "sudo",
                &["apt", "-o", "DPkg::Lock::Timeout=600", "autoremove", "-y"],
                dry_run,
            );
        }
    } else {
        println!("[INFO] apt package manager not found, skipping system package upgrade.");
    }

    // 3. Snap packages update
    if which_command("snap") {
        update_snaps(dry_run);
    } else {
        println!("[INFO] snap not found, skipping snap refresh.");
    }

    // 4. Flatpak packages update
    if which_command("flatpak") {
        run_cmd("Flatpak Update", "flatpak", &["update", "-y"], dry_run);
    } else {
        println!("[INFO] flatpak not found, skipping flatpak update.");
    }

    // 5. Firmware updates (fwupd)
    if which_command("fwupdmgr") {
        println!("[INFO] Running firmware updates via fwupdmgr...");
        // Use --force to avoid interactive prompt when remotes not enabled
        let _ = run_cmd(
            "Fwupd Refresh",
            "fwupdmgr",
            &["refresh", "--force"],
            dry_run,
        );
        // fwupdmgr update returns exit code 2 when no updates available (not an error)
        run_cmd_fwupd("Fwupd Update", "fwupdmgr", &["update", "-y"], dry_run);
    } else {
        println!("[INFO] fwupdmgr not found, skipping firmware updates.");
    }

    // 6. NPM global packages update
    if which_command("npm") {
        run_cmd("NPM Global Update", "npm", &["update", "-g"], dry_run);
    } else {
        println!("[INFO] npm not found, skipping global npm updates.");
    }
}

/// Refreshes each pending snap individually so that one running application
/// cannot mask the result of the other packages.
///
/// Snapd reports snaps with running apps as errors (e.g.
/// `snap "bitwarden" has running apps`). Such snaps are deferred and reported
/// at the end instead of being treated as hard failures.
fn update_snaps(dry_run: bool) {
    println!("\n=== 📦 Updating Snap Packages ===");

    let pending = match pending_snap_refreshes() {
        Some(names) => names,
        None => {
            println!(
                "[WARN] Could not query pending snap refreshes; falling back to global refresh."
            );
            run_cmd("Snap Refresh", "sudo", &["snap", "refresh"], dry_run);
            return;
        }
    };

    if pending.is_empty() {
        println!("[INFO] ℹ️  No snap updates available.");
        return;
    }

    println!("[INFO] Pending snap refreshes: {}", pending.join(", "));

    if dry_run {
        for name in &pending {
            println!("[DRY-RUN] Would execute: sudo snap refresh {}", name);
        }
        return;
    }

    let mut refreshed: Vec<String> = Vec::new();
    let mut deferred: Vec<String> = Vec::new();
    let mut failed: Vec<String> = Vec::new();

    for name in &pending {
        println!(
            "[INFO] 🚀 Executing: Snap Refresh ({}) (sudo snap refresh {})",
            name, name
        );
        match refresh_single_snap(name) {
            SnapRefreshOutcome::Refreshed => {
                println!("[SUCCESS] ✅ {} refreshed successfully.", name);
                refreshed.push(name.clone());
            }
            SnapRefreshOutcome::Deferred => {
                println!(
                    "[WARN] ⚠️  {} was not refreshed because the app is running. Deferred.",
                    name
                );
                deferred.push(name.clone());
            }
            SnapRefreshOutcome::Failed => {
                eprintln!("[ERROR] ❌ {} could not be refreshed.", name);
                failed.push(name.clone());
            }
        }
    }

    if !deferred.is_empty() {
        println!("[INFO] ℹ️  Close the following apps, then refresh manually:");
        for name in &deferred {
            println!("  - {}", name);
        }
        println!("[HINT] sudo snap refresh {}", deferred.join(" "));
    }
    if !failed.is_empty() {
        println!("[INFO] ℹ️  Failed snaps: {}", failed.join(", "));
    }
    println!(
        "[INFO] Snap refresh summary: {} refreshed, {} deferred, {} failed.",
        refreshed.len(),
        deferred.len(),
        failed.len()
    );
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

fn refresh_single_snap(name: &str) -> SnapRefreshOutcome {
    let mut output = match capture_sudo_snap_refresh(name) {
        Ok(output) => output,
        Err(e) => {
            eprintln!(
                "[ERROR] ❌ Failed to execute 'sudo snap refresh {}': {}",
                name, e
            );
            return SnapRefreshOutcome::Failed;
        }
    };

    // Sudo may be configured with NOPASSWD for `snap` only (common on
    // systems that run this script from systemd). Try the cached/non-
    // interactive path first, and ask for a password only when sudo
    // explicitly reports that authentication is needed.
    if !output.status.success() && needs_authentication(&output) {
        println!("[INFO] 🔐 sudo authentication required for snap refresh.");
        let authenticated = Command::new("sudo")
            .arg("-v")
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !authenticated {
            eprintln!(
                "[ERROR] ❌ sudo authentication failed; cannot refresh {}.",
                name
            );
            return SnapRefreshOutcome::Failed;
        }
        output = match capture_sudo_snap_refresh(name) {
            Ok(output) => output,
            Err(e) => {
                eprintln!(
                    "[ERROR] ❌ Failed to execute 'sudo snap refresh {}': {}",
                    name, e
                );
                return SnapRefreshOutcome::Failed;
            }
        };
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    print!("{}", stdout);
    if !stdout.is_empty() && !stdout.ends_with('\n') {
        println!();
    }
    eprint!("{}", stderr);
    if !stderr.is_empty() && !stderr.ends_with('\n') {
        eprintln!();
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

fn update_apps(dry_run: bool) {
    println!("\n=== 📱 Updating Applications / Tools ===");

    // VS Code update (detect installation method)
    if which_command("code") {
        println!("[INFO] VS Code (code CLI) is available.");
        let _ = Command::new("code").arg("--version").status();

        // Detect installation method and update accordingly
        if is_installed_via_apt("code") {
            println!("[INFO] VS Code detected as apt package. Updating via apt...");
            let _ = run_cmd(
                "Apt Update (for VS Code)",
                "sudo",
                &["apt", "-o", "DPkg::Lock::Timeout=600", "update"],
                dry_run,
            );
            run_cmd(
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
            );
        } else if which_command("snap") && is_installed_via_snap("code") {
            println!("[INFO] VS Code detected as snap package. Updating via snap...");
            run_cmd(
                "VS Code Snap Refresh",
                "sudo",
                &["snap", "refresh", "code"],
                dry_run,
            );
        } else if which_command("flatpak") && is_installed_via_flatpak("com.visualstudio.code") {
            println!("[INFO] VS Code detected as flatpak. Updating via flatpak...");
            run_cmd(
                "VS Code Flatpak Update",
                "flatpak",
                &["update", "-y", "com.visualstudio.code"],
                dry_run,
            );
        } else {
            println!(
                "[INFO] VS Code installation method not detected (standalone/.deb manual). Skipping auto-update."
            );
            println!("[HINT] Update manually from https://code.visualstudio.com/");
        }
    } else {
        println!("[INFO] VS Code CLI ('code') not found in PATH.");
    }

    if which_command("cargo") {
        let output = Command::new("cargo")
            .args(["install-update", "--version"])
            .output();

        if let Ok(o) = output {
            if o.status.success() {
                run_cmd(
                    "Cargo Installed Binaries Update",
                    "cargo",
                    &["install-update", "-a"],
                    dry_run,
                );
            } else {
                println!(
                    "[INFO] cargo-update (cargo install-update) not installed. To install: cargo install cargo-update"
                );
            }
        }
    }
}

fn install_systemd_services(dry_run: bool) {
    println!("\n=== ⚙️ Installing systemd Boot & Resume Services ===");

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

fn which_command(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn is_installed_via_apt(package: &str) -> bool {
    Command::new("dpkg")
        .args(["-l", package])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn is_installed_via_snap(package: &str) -> bool {
    Command::new("snap")
        .args(["list", package])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn is_installed_via_flatpak(app_id: &str) -> bool {
    Command::new("flatpak")
        .args(["list", "--app"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains(app_id))
        .unwrap_or(false)
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

    if run_all || opts.references_only {
        update_references(opts.dry_run);
    }

    if run_all || opts.system_only {
        update_system(opts.dry_run);
    }

    if run_all || opts.apps_only {
        update_apps(opts.dry_run);
    }

    println!("\n========================================");
    println!("       🎉 All Update Tasks Done!        ");
    println!("========================================");
}
