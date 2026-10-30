#!/data/data/com.termux/files/usr/bin/rust-script
//! ```cargo
//! [dependencies]
//! chrono = "0.4"
//! fs2 = "0.4"
//! ```

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::process::Command;

use chrono::Local;
use fs2::FileExt;

fn main() {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let base = format!("{}/.local/share/system-all-update", home);
    let log_path = format!("{}/job.log", base);
    let lock_path = format!("{}/job.lock", base);

    if let Err(error) = fs::create_dir_all(&base) {
        eprintln!("failed to create {}: {}", base, error);
        std::process::exit(1);
    }

    let lock = match OpenOptions::new().create(true).write(true).open(&lock_path) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("failed to open {}: {}", lock_path, error);
            std::process::exit(1);
        }
    };

    if lock.try_lock_exclusive().is_err() {
        append_log(
            &log_path,
            &format!(
                "=== {} skipped: another run is active ===\n",
                Local::now().to_rfc3339()
            ),
        );
        return;
    }

    let exe = format!("{}/.local/bin/system-all-update", home);
    let output = match Command::new(&exe).output() {
        Ok(output) => output,
        Err(error) => {
            append_log(
                &log_path,
                &format!(
                    "=== {} failed to start {}: {} ===\n",
                    Local::now().to_rfc3339(),
                    exe,
                    error
                ),
            );
            std::process::exit(1);
        }
    };

    let mut entry = String::new();
    entry.push_str(&format!("=== {} ===\n", Local::now().to_rfc3339()));
    entry.push_str(&String::from_utf8_lossy(&output.stdout));
    entry.push_str(&String::from_utf8_lossy(&output.stderr));
    entry.push_str(&format!("exit={}\n", output.status.code().unwrap_or(-1)));
    append_log(&log_path, &entry);

    // 次回以降は 24 時間周期で再登録する。
    let _ = Command::new(format!("{}/.local/bin/system-all-update", home))
        .arg("--install-service")
        .status();

    if !output.status.success() {
        std::process::exit(output.status.code().unwrap_or(1));
    }
}

fn append_log(path: &str, text: &str) {
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(text.as_bytes());
        if !text.ends_with('\n') {
            let _ = file.write_all(b"\n");
        }
    }
}
