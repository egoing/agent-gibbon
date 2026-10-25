fn main() {
    println!("cargo:rerun-if-env-changed=FLOATNOTE_SENTRY_DSN");
    tauri_build::build()
}
