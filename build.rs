use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=src/main.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");

    let profile = env::var("PROFILE").unwrap_or_default();
    if profile != "release" {
        return;
    }

    let out_dir = env::var("OUT_DIR").unwrap();

    // Find target/release directory by walking up from OUT_DIR
    let mut target_dir = PathBuf::from(&out_dir);
    loop {
        if target_dir
            .file_name()
            .map(|s| s == "release")
            .unwrap_or(false)
        {
            break;
        }
        if !target_dir.pop() {
            eprintln!("[build.rs] ERROR: Could not find release directory from OUT_DIR");
            return;
        }
    }
    target_dir.pop();
    target_dir.push("release");

    let binary_name =
        env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "system-all-update".to_string());
    let binary_path = target_dir.join(&binary_name);

    let home = env::var("HOME").expect("HOME not set");
    let dest_dir = PathBuf::from(home).join(".local").join("bin");
    let dest_path = dest_dir.join(&binary_name);

    if let Err(e) = fs::create_dir_all(&dest_dir) {
        eprintln!(
            "[build.rs] Warning: Failed to create {}: {}",
            dest_dir.display(),
            e
        );
        return;
    }

    if binary_path.exists() {
        match fs::copy(&binary_path, &dest_path) {
            Ok(_) => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = fs::metadata(&dest_path).unwrap().permissions();
                    perms.set_mode(0o755);
                    fs::set_permissions(&dest_path, perms).ok();
                }
                println!("[build.rs] ✅ Installed to {}", dest_path.display());
            }
            Err(e) => {
                eprintln!("[build.rs] Warning: Failed to copy binary: {}", e);
            }
        }
    } else {
        eprintln!(
            "[build.rs] Warning: Binary not found at {}",
            binary_path.display()
        );
    }
}
