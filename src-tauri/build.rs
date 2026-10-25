fn main() {
    println!("cargo:rerun-if-env-changed=AGENT_GIBBON_SENTRY_DSN");
    println!("cargo:rerun-if-env-changed=FLOATNOTE_SENTRY_DSN");
    tauri_build::build()
}
