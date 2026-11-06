//! Optional bundled instructions: local installation only, never agent execution.
mod environment;
mod installer;
#[cfg(test)]
mod tests;
use environment::Environment;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use tauri::Manager;
type Result<T> = std::result::Result<T, String>;

struct Skill {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    markdown: &'static str,
}
fn catalog() -> [Skill; 2] {
    [
        Skill { id: "agent-gibbon", name: "메모 관리", description: "Agent Gibbon의 메모 생성·조회·갱신·숨기기를 사용합니다. 기존 창의 위치와 포커스, 숨김 상태를 유지합니다.", markdown: include_str!("../../.agents/skills/agent-gibbon/SKILL.md") },
        Skill { id: "agent-gibbon-progress", name: "작업 진행 표시", description: "작업 계획과 진행상황을 플로팅 체크리스트 메모로 표시합니다. 실제로 끝난 단계와 미검증 범위를 구분하며 같은 메모를 갱신합니다.", markdown: include_str!("../../bundled-skills/agent-gibbon-progress/SKILL.md") },
    ]
}
fn revision() -> String {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    hash.update(env!("CARGO_PKG_VERSION"));
    for s in catalog() {
        hash.update(s.id);
        hash.update(s.markdown);
    }
    format!("{:x}", hash.finalize())
}
#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    enabled: bool,
    revision: String,
    placements: Vec<PathBuf>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    format: u8,
    skills: BTreeMap<String, Entry>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            format: 1,
            skills: BTreeMap::new(),
        }
    }
}
impl Config {
    fn validate(&self) -> Result<()> {
        if self.format != 1 {
            return Err("지원하지 않는 스킬 설정 형식입니다".into());
        }
        for (id, entry) in &self.skills {
            if !catalog().iter().any(|s| s.id == id)
                || entry.placements.len() > 16
                || entry.placements.iter().any(|p| {
                    !p.is_absolute()
                        || p.components().any(|c| {
                            matches!(
                                c,
                                std::path::Component::ParentDir | std::path::Component::CurDir
                            )
                        })
                        || p.file_name().and_then(|n| n.to_str()) != Some(id.as_str())
                        || p.parent().is_none_or(|p| p.parent().is_none())
                })
            {
                return Err("스킬 설정의 항목이나 경로가 올바르지 않습니다".into());
            }
        }
        Ok(())
    }
}
fn load(root: &Path) -> Result<Config> {
    match fs::read(root.join("settings.json")) {
        Ok(bytes) => {
            if bytes.len() > 65536 {
                return Err("스킬 설정 파일이 너무 큽니다".into());
            }
            let config: Config = serde_json::from_slice(&bytes)
                .map_err(|_| "스킬 설정을 읽지 못했습니다. 기존 파일은 보존했습니다")?;
            config.validate()?;
            Ok(config)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(_) => Err("스킬 설정 파일을 읽지 못했습니다".into()),
    }
}
#[derive(Serialize)]
struct SkillView {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    enabled: bool,
}
#[derive(Serialize)]
pub struct Snapshot {
    skills: Vec<SkillView>,
    feedback: Option<String>,
}
fn snapshot(config: &Config, feedback: Option<String>) -> Snapshot {
    Snapshot {
        skills: catalog()
            .iter()
            .map(|s| SkillView {
                id: s.id,
                name: s.name,
                description: s.description,
                enabled: config.skills.get(s.id).is_some_and(|e| e.enabled),
            })
            .collect(),
        feedback,
    }
}
struct Service {
    root: PathBuf,
    environment: Environment,
    revision: String,
}
impl Service {
    fn locked<T>(&self, f: impl FnOnce(Config) -> Result<T>) -> Result<T> {
        fs::create_dir_all(&self.root).map_err(|_| "스킬 설정 폴더를 만들지 못했습니다")?;
        let lock = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.root.join("settings.lock"))
            .map_err(|_| "스킬 설정 잠금을 만들지 못했습니다")?;
        lock.try_lock_exclusive()
            .map_err(|_| "다른 스킬 변경을 처리 중입니다. 잠시 후 다시 시도하세요")?;
        let config = load(&self.root)?;
        installer::recover(&self.root, &config)?;
        let result = f(config);
        let _ = FileExt::unlock(&lock);
        result
    }
    fn destinations(&self, id: &str) -> Result<Vec<PathBuf>> {
        let mut result = Vec::new();
        for root in self.environment.roots() {
            fs::create_dir_all(&root).map_err(|_| "에이전트의 스킬 폴더에 쓸 수 없습니다")?;
            // The root may be an intentional shared directory. Canonicalize it,
            // but never canonicalize the skill entry that will be replaced.
            let root =
                fs::canonicalize(root).map_err(|_| "에이전트 스킬 폴더를 확인하지 못했습니다")?;
            result.push(root.join(id));
        }
        result.sort();
        result.dedup();
        Ok(result)
    }
    fn set(&self, id: &str, enabled: bool) -> Result<Snapshot> {
        let skill = catalog()
            .into_iter()
            .find(|s| s.id == id)
            .ok_or("알 수 없는 스킬입니다")?;
        self.locked(|before| {
            let mut after = before.clone();
            let old = before.skills.get(id).cloned().unwrap_or_default();
            let mut changes = Vec::new();
            if enabled {
                let mut placements = self.destinations(id)?;
                if placements.is_empty() {
                    return Err("사용 가능한 Codex 또는 Claude Code를 찾지 못했습니다".into());
                }
                // Retain receipts for copies whose agent was removed meanwhile,
                // so Off can still disable exactly those app-managed copies.
                for p in old.placements {
                    if !placements.contains(&p) {
                        if installer::exists(&p) && !owned(&p) {
                            return Err("이전 스킬 설치본의 소유권을 확인하지 못했습니다".into());
                        }
                        placements.push(p);
                    }
                }
                for p in &placements {
                    changes.push((p.clone(), Some(skill.markdown)));
                }
                after.skills.insert(
                    id.into(),
                    Entry {
                        enabled: true,
                        revision: self.revision.clone(),
                        placements,
                    },
                );
            } else {
                for p in &old.placements {
                    if installer::exists(p) {
                        if !owned(p) {
                            return Err(
                                "스킬 설치본의 소유권을 확인할 수 없어 해제하지 못했습니다".into(),
                            );
                        }
                        changes.push((p.clone(), None));
                    }
                }
                after.skills.insert(id.into(), Entry::default());
            }
            installer::transact(&self.root, &before, &after, &changes)?;
            Ok(snapshot(&after, None))
        })
    }
    fn synchronize(&self, before: &Config) -> Result<Config> {
        let mut after = before.clone();
        let mut changes = Vec::new();
        for skill in catalog() {
            let Some(entry) = before.skills.get(skill.id).filter(|e| e.enabled) else {
                continue;
            };
            let mut placements = self.destinations(skill.id)?;
            for p in &entry.placements {
                if !placements.contains(p) {
                    if installer::exists(p) && !owned(p) {
                        return Err("이전 스킬 설치본의 소유권을 확인하지 못했습니다".into());
                    }
                    placements.push(p.clone());
                }
            }
            if placements.is_empty() {
                return Err("사용 가능한 Codex 또는 Claude Code를 찾지 못했습니다".into());
            }
            let upgrade = entry.revision != self.revision;
            for p in &placements {
                if upgrade
                    || !entry.placements.contains(p)
                    || !installer::exists(&p.join("SKILL.md"))
                {
                    changes.push((p.clone(), Some(skill.markdown)));
                }
            }
            after.skills.insert(
                skill.id.into(),
                Entry {
                    enabled: true,
                    revision: self.revision.clone(),
                    placements,
                },
            );
        }
        if after != *before || !changes.is_empty() {
            installer::transact(&self.root, before, &after, &changes)?;
        }
        Ok(after)
    }
    fn list(&self) -> Result<Snapshot> {
        self.locked(|before| match self.synchronize(&before) {
            Ok(after) => Ok(snapshot(&after, None)),
            Err(error) => Ok(snapshot(&before, Some(error))),
        })
    }
}
fn owned(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| m.is_dir() && !m.file_type().is_symlink())
        && fs::symlink_metadata(path.join(".agent-gibbon-managed"))
            .is_ok_and(|m| m.is_file() && !m.file_type().is_symlink())
        && fs::read(path.join(".agent-gibbon-managed")).is_ok_and(|v| v == b"1\n")
}
fn service(app: &tauri::AppHandle) -> Result<Service> {
    let home = app
        .path()
        .home_dir()
        .map_err(|_| "홈 폴더를 찾지 못했습니다")?;
    let root = app
        .path()
        .app_data_dir()
        .map_err(|_| "스킬 저장 위치를 찾지 못했습니다")?
        .join("skills");
    // Debug-only fixture boundary for native verification. Release ignores it.
    #[cfg(debug_assertions)]
    if let Some(fixture) = std::env::var_os("AGENT_GIBBON_SKILLS_TEST_ROOT") {
        let fixture = PathBuf::from(fixture);
        if !fixture.is_absolute() {
            return Err("스킬 검증 경로는 절대 경로여야 합니다".into());
        }
        return Ok(Service {
            root: fixture.join("state"),
            environment: Environment {
                home: fixture.join("home"),
                paths: vec![fixture.join("bin")],
                claude_root: fixture.join("home/.claude/skills"),
            },
            revision: revision(),
        });
    }
    Ok(Service {
        root,
        environment: Environment::current(home),
        revision: revision(),
    })
}
fn allowed(window: &tauri::WebviewWindow) -> Result<()> {
    if window.label() != crate::manager::LABEL || !crate::privacy::accepted(window.app_handle()) {
        return Err("약관 확인 후 관리 창에서만 스킬을 설정할 수 있습니다".into());
    }
    Ok(())
}
#[tauri::command]
pub async fn skills_list(window: tauri::WebviewWindow) -> Result<Snapshot> {
    allowed(&window)?;
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || service(&app)?.list())
        .await
        .map_err(|_| "스킬 조회 작업이 중단되었습니다")?
}
#[tauri::command]
pub async fn skills_set(
    window: tauri::WebviewWindow,
    id: String,
    enabled: bool,
) -> Result<Snapshot> {
    allowed(&window)?;
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || service(&app)?.set(&id, enabled))
        .await
        .map_err(|_| "스킬 변경 작업이 중단되었습니다")?
}
pub fn synchronize(app: tauri::AppHandle) {
    if !crate::privacy::accepted(&app) {
        return;
    }
    tauri::async_runtime::spawn_blocking(move || {
        let _ = service(&app).and_then(|s| s.list());
    });
}
