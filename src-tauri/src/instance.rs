//! Optional development namespace; absence preserves the installed app endpoint.
use std::sync::OnceLock;

#[derive(Debug, Default)]
pub struct Instance {
    pub id: Option<String>,
    pub label: Option<String>,
}
impl Instance {
    fn parse(id: Option<String>, name: Option<String>) -> Result<Self, String> {
        let Some(id) = id else {
            return Ok(Self::default());
        };
        if id.len() != 28
            || !id.starts_with("dev-")
            || !id[4..]
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err("FLOATNOTE_INSTANCE는 dev-와 소문자 SHA-256 앞 24자리여야 합니다".into());
        }
        let name = name.unwrap_or_else(|| "worktree".into());
        let name: String = name.chars().filter(|c| !c.is_control()).take(60).collect();
        let label = Some(format!("DEV {name} · {}", &id[4..12]));
        Ok(Self {
            id: Some(id),
            label,
        })
    }
    #[cfg(any(unix, test))]
    pub fn directory(&self, uid: u32) -> std::path::PathBuf {
        let suffix = self
            .id
            .as_ref()
            .map(|id| format!("-{id}"))
            .unwrap_or_default();
        format!("/tmp/floatnote-{uid}{suffix}").into()
    }
    pub fn title(&self) -> String {
        self.label
            .as_ref()
            .map(|label| format!("Agent Gibbon [{label}]"))
            .unwrap_or_else(|| "Agent Gibbon".into())
    }
}
static INSTANCE: OnceLock<Instance> = OnceLock::new();
pub fn init() -> Result<(), String> {
    let read = |key| match std::env::var(key) {
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(_) => Err(format!("{key}는 UTF-8이어야 합니다")),
    };
    let value = Instance::parse(read("FLOATNOTE_INSTANCE")?, read("FLOATNOTE_DEV_NAME")?)?;
    INSTANCE
        .set(value)
        .map_err(|_| "인스턴스가 이미 초기화되었습니다".into())
}
pub fn current() -> &'static Instance {
    INSTANCE.get_or_init(Instance::default)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn installed_path_is_unchanged() {
        let instance = Instance::parse(None, Some("ignored".into())).unwrap();
        assert_eq!(
            instance.directory(501).to_str().unwrap(),
            "/tmp/floatnote-501"
        );
        assert_eq!(instance.title(), "Agent Gibbon");
        assert!(instance.label.is_none());
    }
    #[test]
    fn namespaces_are_short_and_distinct() {
        let a =
            Instance::parse(Some(format!("dev-{}", "a".repeat(24))), Some("same".into())).unwrap();
        let b =
            Instance::parse(Some(format!("dev-{}", "b".repeat(24))), Some("same".into())).unwrap();
        assert_ne!(a.directory(501), b.directory(501));
        assert_ne!(a.title(), b.title());
        assert!(a.directory(u32::MAX).join("app.sock").as_os_str().len() < 104);
    }
    #[test]
    fn rejects_empty_traversal_and_malformed_ids() {
        for id in [
            "",
            "../escape",
            "dev-x",
            "dev-ABCDEFGHIJKLMNOPQRSTUVWX",
            "dev-aaaaaaaaaaaaaaaaaaaaaaa/",
            "dev-😀aaaaaaaaaaaaaaaaaaaa",
        ] {
            assert!(Instance::parse(Some(id.into()), None).is_err(), "{id}");
        }
    }
}
