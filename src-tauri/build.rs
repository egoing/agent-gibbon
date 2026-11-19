fn main() {
    // Native child WebViews and capture frames use the same logical row as CSS.
    let tokens = include_str!("../ui/tokens.css");
    let row_height = tokens
        .lines()
        .find_map(|line| line.trim().strip_prefix("--toolbar-row-height:"))
        .and_then(|value| value.trim().strip_suffix("px;"))
        .and_then(|pixels| pixels.parse::<u16>().ok())
        .filter(|height| *height > 0)
        .expect("--toolbar-row-height must be a positive integer in CSS pixels");
    let out_dir = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    std::fs::write(out_dir.join("toolbar_height.rs"), format!("{row_height}.0")).unwrap();
    println!("cargo:rerun-if-changed=../ui/tokens.css");
    println!("cargo:rerun-if-env-changed=TAURI_UPDATER_PUBLIC_KEY");
    println!("cargo:rerun-if-env-changed=JAJAN_SENTRY_DSN");
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_owned())
    };
    let revision = git(&["rev-parse", "--short=12", "HEAD"])
        .filter(|s| s.len() == 12 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=JAJAN_BUILD_REVISION={revision}");
    for name in ["HEAD", "refs/heads/main"] {
        if let Some(path) = git(&["rev-parse", "--git-path", name]) {
            println!("cargo:rerun-if-changed={path}");
        }
    }
    if let Some(reference) = git(&["symbolic-ref", "-q", "HEAD"]) {
        if let Some(path) = git(&["rev-parse", "--git-path", &reference]) {
            println!("cargo:rerun-if-changed={path}");
        }
    }
    #[cfg(target_os = "macos")]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rerun-if-changed=src/terminal/macos_jobs.c");
        cc::Build::new()
            .file("src/terminal/macos_jobs.c")
            .compile("terminal_jobs");
        println!("cargo:rerun-if-changed=src/capture/visibility.m");
        cc::Build::new()
            .file("src/capture/visibility.m")
            .flag("-fobjc-arc")
            .compile("capture_visibility");
        println!("cargo:rerun-if-changed=src/permission_panel.m");
        println!("cargo:rerun-if-changed=src/layout/app_windows.m");
        cc::Build::new()
            .file("src/permission_panel.m")
            .file("src/layout/app_windows.m")
            .file("src/app_activation.m")
            .flag("-fobjc-arc")
            .compile("permission_panel");
        println!("cargo:rerun-if-changed=src/app_activation.m");
    }
    tauri_build::build()
}
