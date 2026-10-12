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
    println!("  --apps-only        Update only the applications/tools (VS Code, cargo-update, etc.)");
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
                return Err(format!("Unknown argument: {}. Use --help for usage.", other));
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
    println!("[INFO] 🚀 Executing: {} ({} {})", name, program, args.join(" "));
    if dry_run {
        println!("[DRY-RUN] Skipped execution.");
        return true;
    }

    let status = Command::new(program)
        .args(args)
        .status();

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

fn update_references(dry_run: bool) {
    let ref_path = "/home/koppe/references";
    println!("\n=== 📚 Updating References ({}) ===", ref_path);

    if !Path::new(ref_path).exists() {
        println!("[WARN] References path '{}' does not exist. Skipping.", ref_path);
        return;
    }

    let git_dir = format!("{}/.git", ref_path);
    if !Path::new(&git_dir).exists() {
        println!("[WARN] References path '{}' is not a git repository. Skipping git pull.", ref_path);
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
        let apt_success = run_cmd("Apt Update", "sudo", &["apt", "update"], dry_run);
        if apt_success {
            run_cmd("Apt Upgrade", "sudo", &["apt", "upgrade", "-y"], dry_run);
        }
    } else {
        println!("[INFO] apt package manager not found, skipping system package upgrade.");
    }

    // 3. Snap packages update
    if which_command("snap") {
        run_cmd("Snap Refresh", "sudo", &["snap", "refresh"], dry_run);
    } else {
        println!("[INFO] snap not found, skipping snap refresh.");
    }

    // 4. Flatpak packages update
    if which_command("flatpak") {
        run_cmd("Flatpak Update", "flatpak", &["update", "-y"], dry_run);
    } else {
        println!("[INFO] flatpak not found, skipping flatpak update.");
    }

    // 5. NPM global packages update
    if which_command("npm") {
        run_cmd("NPM Global Update", "npm", &["update", "-g"], dry_run);
    } else {
        println!("[INFO] npm not found, skipping global npm updates.");
    }
}

fn update_apps(dry_run: bool) {
    println!("\n=== 📱 Updating Applications / Tools ===");

    if which_command("code") {
        println!("[INFO] VS Code (code CLI) is available.");
        let _ = Command::new("code").arg("--version").status();
    } else {
        println!("[INFO] VS Code CLI ('code') not found in PATH.");
    }

    if which_command("cargo") {
        let output = Command::new("cargo")
            .args(&["install-update", "--version"])
            .output();
        
        if let Ok(o) = output {
            if o.status.success() {
                run_cmd("Cargo Installed Binaries Update", "cargo", &["install-update", "-a"], dry_run);
            } else {
                println!("[INFO] cargo-update (cargo install-update) not installed. To install: cargo install cargo-update");
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

    let boot_service = format!(
        r#"[Unit]
Description=Run system-all-update on system boot
After=network-online.target
Wants=network-online.target

[Service]
Type=oneshot
User={}
ExecStart={}

[Install]
WantedBy=multi-user.target
"#,
        current_user, exe_path
    );

    let resume_service = format!(
        r#"[Unit]
Description=Run system-all-update on resume from suspend
After=suspend.target hibernate.target hybrid-sleep.target suspend-then-hibernate.target

[Service]
Type=oneshot
User={}
ExecStart={}

[Install]
WantedBy=suspend.target hibernate.target hybrid-sleep.target suspend-then-hibernate.target
"#,
        current_user, exe_path
    );

    let boot_path = "/tmp/system-all-update-boot.service";
    let resume_path = "/tmp/system-all-update-resume.service";

    if let Err(e) = fs::write(boot_path, boot_service) {
        eprintln!("[ERROR] Failed to write temporary boot service file: {}", e);
        return;
    }
    if let Err(e) = fs::write(resume_path, resume_service) {
        eprintln!("[ERROR] Failed to write temporary resume service file: {}", e);
        return;
    }

    if dry_run {
        println!("[DRY-RUN] Would move service files to /etc/systemd/system/ and enable them.");
        return;
    }

    // Move to /etc/systemd/system/ using sudo
    if !run_cmd("Install Boot Service", "sudo", &["mv", boot_path, "/etc/systemd/system/system-all-update-boot.service"], false) {
        return;
    }
    if !run_cmd("Install Resume Service", "sudo", &["mv", resume_path, "/etc/systemd/system/system-all-update-resume.service"], false) {
        return;
    }

    // Reload daemon and enable services
    run_cmd("Systemd Daemon Reload", "sudo", &["systemctl", "daemon-reload"], false);
    run_cmd("Enable Boot Service", "sudo", &["systemctl", "enable", "system-all-update-boot.service"], false);
    run_cmd("Enable Resume Service", "sudo", &["systemctl", "enable", "system-all-update-resume.service"], false);

    println!("[SUCCESS] 🎉 systemd services successfully installed and enabled!");
}

fn which_command(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .status()
        .map(|s| s.success())
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
