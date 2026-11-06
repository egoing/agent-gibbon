use super::*;
use std::io::Write;

struct Fixture {
    dir: tempfile::TempDir,
    service: Service,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        let bin = dir.path().join("bin");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&bin).unwrap();
        let service = Service {
            root: dir.path().join("state"),
            environment: Environment {
                home: home.clone(),
                paths: vec![bin],
                claude_root: home.join(".claude/skills"),
            },
            revision: "release-1".into(),
        };
        Self { dir, service }
    }
    fn agent(&self, name: &str) {
        let path = self.service.environment.paths[0].join(if cfg!(windows) {
            format!("{name}.cmd")
        } else {
            name.into()
        });
        fs::write(&path, "not a runnable program").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
    fn installed(&self, id: &str) -> PathBuf {
        self.service
            .environment
            .home
            .join(".agents/skills")
            .join(id)
    }
    fn config(&self) -> Config {
        load(&self.service.root).unwrap()
    }
}
#[test]
fn default_is_off_and_enabling_installs_all_detected_agents_without_execution() {
    let f = Fixture::new();
    assert!(f.service.list().unwrap().skills.iter().all(|s| !s.enabled));
    assert!(f.service.set("agent-gibbon", true).is_err());
    f.agent("codex");
    f.agent("claude");
    let snapshot = f.service.set("agent-gibbon", true).unwrap();
    assert!(snapshot.skills[0].enabled);
    assert!(!snapshot.skills[1].enabled);
    for root in [
        f.installed("agent-gibbon"),
        f.service.environment.claude_root.join("agent-gibbon"),
    ] {
        assert_eq!(
            fs::read_to_string(root.join("SKILL.md")).unwrap(),
            catalog()[0].markdown
        );
        assert!(owned(&root));
    }
    assert_eq!(f.config().skills["agent-gibbon"].placements.len(), 2);
}
#[test]
fn explicit_on_overwrites_local_changes_but_ordinary_start_does_not() {
    let f = Fixture::new();
    f.agent("codex");
    f.service.set("agent-gibbon", true).unwrap();
    let path = f.installed("agent-gibbon").join("SKILL.md");
    fs::write(&path, "local edits").unwrap();
    f.service.list().unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "local edits");
    f.service.set("agent-gibbon", true).unwrap();
    assert_eq!(fs::read_to_string(path).unwrap(), catalog()[0].markdown);
}
#[test]
fn upgrade_overwrites_only_on_skills_preserves_choices_and_retries_failures() {
    let mut f = Fixture::new();
    f.agent("codex");
    f.service.set("agent-gibbon", true).unwrap();
    let off = f.installed("agent-gibbon-progress");
    fs::create_dir_all(&off).unwrap();
    fs::write(off.join("SKILL.md"), "user owned off skill").unwrap();
    fs::write(f.installed("agent-gibbon").join("SKILL.md"), "local edits").unwrap();
    f.service.revision = "release-2".into();
    let root = f.service.environment.home.join(".agents/skills");
    let moved = f.service.environment.home.join("old-root");
    fs::rename(&root, &moved).unwrap();
    fs::write(&root, "blocked").unwrap();
    let failed = f.service.list().unwrap();
    assert!(failed.skills[0].enabled);
    assert!(failed.feedback.is_some());
    assert_eq!(f.config().skills["agent-gibbon"].revision, "release-1");
    fs::remove_file(&root).unwrap();
    fs::rename(moved, &root).unwrap();
    assert!(f.service.list().unwrap().feedback.is_none());
    assert_eq!(f.config().skills["agent-gibbon"].revision, "release-2");
    assert_eq!(
        fs::read_to_string(f.installed("agent-gibbon").join("SKILL.md")).unwrap(),
        catalog()[0].markdown
    );
    assert_eq!(
        fs::read_to_string(off.join("SKILL.md")).unwrap(),
        "user owned off skill"
    );
}
#[test]
fn off_removes_only_receipted_copies_and_never_reactivates_on_upgrade() {
    let mut f = Fixture::new();
    f.agent("codex");
    f.service.set("agent-gibbon", true).unwrap();
    f.service.set("agent-gibbon-progress", true).unwrap();
    f.service.set("agent-gibbon", false).unwrap();
    assert!(!installer::exists(&f.installed("agent-gibbon")));
    assert!(f
        .installed("agent-gibbon-progress")
        .join("SKILL.md")
        .exists());
    f.service.revision = "release-2".into();
    f.service.list().unwrap();
    assert!(!f.config().skills["agent-gibbon"].enabled);
    assert!(!installer::exists(&f.installed("agent-gibbon")));
    assert!(f.config().skills["agent-gibbon-progress"].enabled);
}
#[test]
fn new_agent_receives_on_skills_on_next_reconciliation() {
    let f = Fixture::new();
    f.agent("codex");
    f.service.set("agent-gibbon", true).unwrap();
    f.agent("claude");
    f.service.list().unwrap();
    assert!(f
        .service
        .environment
        .claude_root
        .join("agent-gibbon/SKILL.md")
        .exists());
    assert_eq!(f.config().skills["agent-gibbon"].placements.len(), 2);
}
#[test]
fn corrupt_state_and_unowned_off_fail_without_deleting_user_data() {
    let f = Fixture::new();
    f.agent("codex");
    f.service.set("agent-gibbon", true).unwrap();
    fs::remove_file(f.installed("agent-gibbon").join(".agent-gibbon-managed")).unwrap();
    assert!(f.service.set("agent-gibbon", false).is_err());
    assert!(f.config().skills["agent-gibbon"].enabled);
    let before = fs::read(f.installed("agent-gibbon").join("SKILL.md")).unwrap();
    fs::write(f.service.root.join("settings.json"), b"bad config").unwrap();
    assert!(f.service.list().is_err());
    assert_eq!(
        fs::read(f.installed("agent-gibbon").join("SKILL.md")).unwrap(),
        before
    );
    assert_eq!(
        fs::read(f.service.root.join("settings.json")).unwrap(),
        b"bad config"
    );
}
#[test]
fn state_write_failure_rolls_back_every_agent_copy() {
    let f = Fixture::new();
    f.agent("codex");
    f.agent("claude");
    f.service.set("agent-gibbon", true).unwrap();
    let before = f.config();
    let mut after = before.clone();
    after.skills.get_mut("agent-gibbon").unwrap().revision = "new".into();
    let state = f.service.root.join("settings.json");
    fs::remove_file(&state).unwrap();
    fs::create_dir(&state).unwrap();
    let dests = &before.skills["agent-gibbon"].placements;
    for p in dests {
        fs::write(p.join("SKILL.md"), "keep my edits").unwrap();
    }
    let changes: Vec<_> = dests
        .iter()
        .map(|p| (p.clone(), Some("new bundle")))
        .collect();
    assert!(installer::transact(&f.service.root, &before, &after, &changes).is_err());
    for p in dests {
        assert_eq!(
            fs::read_to_string(p.join("SKILL.md")).unwrap(),
            "keep my edits"
        );
    }
    assert!(!f.service.root.join("transaction.json").exists());
}
#[cfg(unix)]
#[test]
fn on_replaces_symlink_entry_without_modifying_its_shared_source() {
    let f = Fixture::new();
    f.agent("codex");
    let source = f.dir.path().join("shared-skill");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("SKILL.md"), "shared original").unwrap();
    let dest = f.installed("agent-gibbon");
    fs::create_dir_all(dest.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&source, &dest).unwrap();
    f.service.set("agent-gibbon", true).unwrap();
    assert!(!fs::symlink_metadata(&dest)
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(
        fs::read_to_string(source.join("SKILL.md")).unwrap(),
        "shared original"
    );
    f.service.set("agent-gibbon", false).unwrap();
    assert_eq!(
        fs::read_to_string(source.join("SKILL.md")).unwrap(),
        "shared original"
    );
}
#[test]
fn cross_instance_lock_refuses_parallel_changes() {
    let f = Fixture::new();
    fs::create_dir_all(&f.service.root).unwrap();
    let mut lock = fs::File::create(f.service.root.join("settings.lock")).unwrap();
    lock.write_all(b"").unwrap();
    lock.lock_exclusive().unwrap();
    assert!(f.service.list().is_err());
}
#[test]
fn catalog_is_real_self_contained_resources_and_does_not_ship_example_capabilities() {
    for s in catalog() {
        assert!(s.markdown.starts_with("---\n"));
        assert!(s.markdown.contains(&format!("name: {}", s.id)));
        assert!(!s.markdown.contains("TODO"));
        assert!(!s.markdown.contains("AGENT_GIBBON_SKILLS_TEST_ROOT"));
    }
    assert!(catalog()[0].markdown.contains("워크플로우"));
}
