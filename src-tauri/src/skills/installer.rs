//! Journaled directory replacement. Rename link entries; never follow a skill
//! symlink into the user's shared source. Only destinations owned by our catalog
//! and transaction-specific siblings can be removed by recovery.
use super::{catalog, Config, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub fn exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}
fn remove(path: &Path) -> Result<()> {
    let meta = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err("스킬 파일을 확인하지 못했습니다".into()),
    };
    if meta.is_dir() {
        fs::remove_dir_all(path)
    } else {
        #[cfg(windows)]
        {
            use std::os::windows::fs::FileTypeExt;
            if meta.file_type().is_symlink_dir() {
                return fs::remove_dir(path).map_err(|_| "스킬 링크를 정리하지 못했습니다".into());
            }
        }
        fs::remove_file(path)
    }
    .map_err(|_| "스킬 파일을 정리하지 못했습니다".into())
}
pub fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let parent = path.parent().ok_or("스킬 저장 위치가 올바르지 않습니다")?;
    fs::create_dir_all(parent).map_err(|_| "스킬 저장 폴더를 만들지 못했습니다")?;
    let mut file = tempfile::NamedTempFile::new_in(parent)
        .map_err(|_| "스킬 임시 파일을 만들지 못했습니다")?;
    serde_json::to_writer(&mut file, value).map_err(|_| "스킬 설정을 기록하지 못했습니다")?;
    file.flush()
        .and_then(|_| file.as_file().sync_all())
        .map_err(|_| "스킬 설정을 저장하지 못했습니다")?;
    file.persist(path)
        .map_err(|_| "스킬 설정을 교체하지 못했습니다")?;
    Ok(())
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    dest: PathBuf,
    stage: Option<PathBuf>,
    backup: PathBuf,
    had_dest: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    before: Config,
    after: Config,
    operations: Vec<Operation>,
}
fn workspace(dest: &Path) -> Option<PathBuf> {
    Some(dest.parent()?.parent()?.join(".agent-gibbon-transactions"))
}
fn sibling(path: &Path, kind: &str) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with(&format!(".gibbon-{kind}-")))
}
fn validate(j: &Journal) -> Result<()> {
    j.before.validate()?;
    j.after.validate()?;
    for op in &j.operations {
        let id = op
            .dest
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("스킬 복구 경로 오류")?;
        if !catalog().iter().any(|s| s.id == id)
            || !op.dest.is_absolute()
            || op.dest.parent().is_none_or(|p| p.parent().is_none())
            || op.backup.parent() != workspace(&op.dest).as_deref()
            || !sibling(&op.backup, "backup")
            || op.stage.as_ref().is_some_and(|s| {
                s.parent() != workspace(&op.dest).as_deref() || !sibling(s, "stage")
            })
            || ![&j.before, &j.after].iter().any(|c| {
                c.skills
                    .get(id)
                    .is_some_and(|e| e.placements.contains(&op.dest))
            })
        {
            return Err("스킬 복구 기록의 경로가 올바르지 않습니다".into());
        }
    }
    Ok(())
}
fn rollback(ops: &[Operation]) -> Result<()> {
    for op in ops.iter().rev() {
        if exists(&op.backup) {
            remove(&op.dest)?;
            fs::rename(&op.backup, &op.dest)
                .map_err(|_| "스킬 이전 설치본을 복원하지 못했습니다")?;
        } else if !op.had_dest && op.stage.as_ref().is_some_and(|s| !exists(s)) {
            remove(&op.dest)?;
        }
        if let Some(stage) = &op.stage {
            remove(stage)?;
        }
    }
    Ok(())
}
fn cleanup(ops: &[Operation]) -> Result<()> {
    for op in ops {
        remove(&op.backup)?;
        if let Some(stage) = &op.stage {
            remove(stage)?;
        }
    }
    Ok(())
}
pub fn recover(root: &Path, current: &Config) -> Result<()> {
    let path = root.join("transaction.json");
    if !exists(&path) {
        return Ok(());
    }
    let j: Journal =
        serde_json::from_slice(&fs::read(&path).map_err(|_| "스킬 복구 기록을 읽지 못했습니다")?)
            .map_err(|_| "스킬 복구 기록이 손상되었습니다")?;
    validate(&j)?;
    if *current == j.after {
        cleanup(&j.operations)?;
    } else if *current == j.before {
        rollback(&j.operations)?;
    } else {
        return Err("스킬 설정과 복구 기록이 일치하지 않습니다".into());
    }
    remove(&path)
}
fn prepare(dest: &Path, markdown: Option<&str>) -> Result<Operation> {
    let parent = dest.parent().ok_or("스킬 설치 경로 오류")?;
    fs::create_dir_all(parent).map_err(|_| "스킬 설치 폴더를 만들지 못했습니다")?;
    // Prepared and old copies live outside the agents' discovery roots, so an
    // interrupted transaction cannot expose a second runnable SKILL.md.
    let work = workspace(dest).ok_or("스킬 작업 경로 오류")?;
    if fs::symlink_metadata(&work).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err("스킬 작업 폴더가 링크여서 사용할 수 없습니다".into());
    }
    fs::create_dir_all(&work).map_err(|_| "스킬 작업 폴더를 만들지 못했습니다")?;
    let parent = work.as_path();
    let backup = tempfile::Builder::new()
        .prefix(".gibbon-backup-")
        .tempdir_in(parent)
        .map_err(|_| "스킬 백업 경로를 만들지 못했습니다")?;
    let backup_path = backup.path().to_path_buf();
    backup.close().map_err(|_| "스킬 백업 경로 준비 실패")?;
    let stage = if let Some(markdown) = markdown {
        let stage = tempfile::Builder::new()
            .prefix(".gibbon-stage-")
            .tempdir_in(parent)
            .map_err(|_| "스킬 배포본 준비 실패")?;
        let mut file =
            fs::File::create(stage.path().join("SKILL.md")).map_err(|_| "스킬 배포본 기록 실패")?;
        file.write_all(markdown.as_bytes())
            .and_then(|_| file.sync_all())
            .map_err(|_| "스킬 배포본 기록 실패")?;
        fs::write(stage.path().join(".agent-gibbon-managed"), b"1\n")
            .map_err(|_| "스킬 소유권 기록 실패")?;
        Some(stage.keep())
    } else {
        None
    };
    Ok(Operation {
        dest: dest.into(),
        stage,
        backup: backup_path,
        had_dest: exists(dest),
    })
}
pub fn transact(
    root: &Path,
    before: &Config,
    after: &Config,
    changes: &[(PathBuf, Option<&str>)],
) -> Result<()> {
    let mut operations = Vec::new();
    for (dest, markdown) in changes {
        match prepare(dest, *markdown) {
            Ok(op) => operations.push(op),
            Err(e) => {
                cleanup(&operations)?;
                return Err(e);
            }
        }
    }
    let journal = Journal {
        before: before.clone(),
        after: after.clone(),
        operations,
    };
    validate(&journal)?;
    let journal_path = root.join("transaction.json");
    if let Err(e) = write_json(&journal_path, &journal) {
        cleanup(&journal.operations)?;
        return Err(e);
    }
    let result = (|| {
        for op in &journal.operations {
            if op.had_dest {
                fs::rename(&op.dest, &op.backup).map_err(|_| "기존 스킬을 교체하지 못했습니다")?;
            }
            if let Some(stage) = &op.stage {
                fs::rename(stage, &op.dest).map_err(|_| "새 스킬을 설치하지 못했습니다")?;
            }
        }
        write_json(&root.join("settings.json"), after)
    })();
    if let Err(e) = result {
        rollback(&journal.operations)?;
        remove(&journal_path)?;
        return Err(e);
    }
    // Persistence is the commit point. Cleanup failure must not report a failed
    // toggle with committed state. The retained journal is cleaned next time.
    if cleanup(&journal.operations).is_ok() {
        let _ = remove(&journal_path);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::Entry;
    #[test]
    fn interrupted_install_recovers_before_state_commit_and_finishes_after_commit() {
        for committed in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("state");
            fs::create_dir(&root).unwrap();
            let dest = dir.path().join("skills/agent-gibbon");
            fs::create_dir_all(&dest).unwrap();
            fs::write(dest.join("SKILL.md"), "old").unwrap();
            let op = prepare(&dest, Some("new")).unwrap();
            let before = Config::default();
            let mut after = before.clone();
            after.skills.insert(
                "agent-gibbon".into(),
                Entry {
                    enabled: true,
                    revision: "new".into(),
                    placements: vec![dest.clone()],
                },
            );
            let journal = Journal {
                before: before.clone(),
                after: after.clone(),
                operations: vec![op],
            };
            write_json(&root.join("transaction.json"), &journal).unwrap();
            let op = &journal.operations[0];
            fs::rename(&dest, &op.backup).unwrap();
            fs::rename(op.stage.as_ref().unwrap(), &dest).unwrap();
            let current = if committed { &after } else { &before };
            write_json(&root.join("settings.json"), current).unwrap();
            recover(&root, current).unwrap();
            assert_eq!(
                fs::read_to_string(dest.join("SKILL.md")).unwrap(),
                if committed { "new" } else { "old" }
            );
            assert!(!exists(&root.join("transaction.json")));
            assert!(!exists(&op.backup));
        }
    }
    #[test]
    fn interrupted_first_install_is_removed_when_choice_was_not_committed() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("skills/agent-gibbon");
        let op = prepare(&dest, Some("new")).unwrap();
        let before = Config::default();
        let mut after = before.clone();
        after.skills.insert(
            "agent-gibbon".into(),
            Entry {
                enabled: true,
                revision: "new".into(),
                placements: vec![dest.clone()],
            },
        );
        let journal = Journal {
            before: before.clone(),
            after,
            operations: vec![op],
        };
        write_json(&dir.path().join("transaction.json"), &journal).unwrap();
        fs::rename(journal.operations[0].stage.as_ref().unwrap(), &dest).unwrap();
        recover(dir.path(), &before).unwrap();
        assert!(!exists(&dest));
    }
}
