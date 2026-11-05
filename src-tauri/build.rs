fn main() {
    println!("cargo:rerun-if-env-changed=AGENT_GIBBON_SENTRY_DSN");
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
    println!("cargo:rustc-env=AGENT_GIBBON_BUILD_REVISION={revision}");
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
    tauri_build::build()
}
