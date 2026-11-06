use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct Environment {
    pub home: PathBuf,
    pub paths: Vec<PathBuf>,
    pub claude_root: PathBuf,
}
impl Environment {
    pub fn current(home: PathBuf) -> Self {
        let mut paths: Vec<_> =
            std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
                .filter(|p| p.is_absolute())
                .collect();
        for suffix in [
            ".local/bin",
            ".cargo/bin",
            ".npm-global/bin",
            ".volta/bin",
            ".bun/bin",
            "AppData/Roaming/npm",
        ] {
            paths.push(home.join(suffix));
        }
        // GUI launches need not inherit a version manager's PATH. Only enumerate
        // known binary folders; do not execute login scripts or candidate agents.
        for prefix in [".nvm/versions/node", ".local/share/fnm/node-versions"] {
            if let Ok(entries) = std::fs::read_dir(home.join(prefix)) {
                for entry in entries.flatten() {
                    paths.push(entry.path().join("bin"));
                    paths.push(entry.path().join("installation/bin"));
                }
            }
        }
        for path in ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin"] {
            paths.push(path.into());
        }
        if let Some(path) = std::env::var_os("APPDATA") {
            paths.push(PathBuf::from(path).join("npm"));
        }
        let claude_root = std::env::var_os("CLAUDE_CONFIG_DIR")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute() && p.parent().is_some() && !home.starts_with(p))
            .unwrap_or_else(|| home.join(".claude"))
            .join("skills");
        Self {
            home,
            paths,
            claude_root,
        }
    }
    pub fn roots(&self) -> Vec<PathBuf> {
        let mut roots = Vec::new();
        if self.installed("codex") {
            roots.push(self.home.join(".agents/skills"));
        }
        if self.installed("claude") {
            roots.push(self.claude_root.clone());
        }
        roots.sort();
        roots.dedup();
        roots
    }
    fn installed(&self, name: &str) -> bool {
        let names = if cfg!(windows) {
            vec![
                format!("{name}.exe"),
                format!("{name}.cmd"),
                format!("{name}.bat"),
            ]
        } else {
            vec![name.to_string()]
        };
        self.paths
            .iter()
            .filter(|p| p.is_absolute())
            .any(|p| names.iter().any(|n| executable(&p.join(n))))
    }
}
fn executable(path: &Path) -> bool {
    let Ok(meta) = path.metadata() else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}
