fn main() {
    println!("cargo:rerun-if-env-changed=AGENT_GIBBON_SENTRY_DSN");
    tauri_build::build()
}
