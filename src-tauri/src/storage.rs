//! One atomically replaced Markdown document per note, isolated by instance.
use crate::model::{Action, Note, Request};
use serde::{Deserialize, Serialize};
#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

fn entry_type_is_directory(path: &Path) -> Result<bool, String> {
    fs::symlink_metadata(path)
        .map(|m| m.is_dir())
        .map_err(|e| e.to_string())
}
pub struct Store(pub PathBuf);
#[derive(Serialize, Deserialize)]
struct Metadata {
    #[serde(default)]
    capture: Option<crate::capture::ImageNote>,
    title: String,
    color: String,
    status: String,
    source_path: Option<String>,
    updated_at: u64,
    revision: u64,
}
static SERIAL: AtomicU64 = AtomicU64::new(0);
const PREFIX: &str = "<!-- agent-gibbon-v1\n";
impl Store {
    pub fn new(root: PathBuf, instance: Option<&str>) -> Self {
        Self(match instance {
            Some(id) => root.join("notes").join(id).join("docs"),
            None => root.join("agent-gibbon").join("docs"),
        })
    }
    pub fn path(&self, id: &str) -> PathBuf {
        // Escape uppercase only: readable IDs without case collisions on macOS.
        let reserved = cfg!(windows)
            && matches!(
                id.to_ascii_lowercase().as_str(),
                "con"
                    | "prn"
                    | "aux"
                    | "nul"
                    | "com1"
                    | "com2"
                    | "com3"
                    | "com4"
                    | "com5"
                    | "com6"
                    | "com7"
                    | "com8"
                    | "com9"
                    | "lpt1"
                    | "lpt2"
                    | "lpt3"
                    | "lpt4"
                    | "lpt5"
                    | "lpt6"
                    | "lpt7"
                    | "lpt8"
                    | "lpt9"
            );
        let name: String = id
            .bytes()
            .map(|b| {
                if b.is_ascii_uppercase() || reserved {
                    format!("~{b:02x}")
                } else {
                    char::from(b).to_string()
                }
            })
            .collect();
        self.0.join(name).join("index.md")
    }
    pub fn save(&self, id: &str, note: &Note) -> Result<(), String> {
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let path = self.path(id);
            let mut builder = fs::DirBuilder::new();
            builder.recursive(true);
            #[cfg(unix)]
            builder.mode(0o700);
            builder.create(path.parent().ok_or("메모 폴더가 없습니다")?)?;
            let meta = Metadata {
                capture: note.capture.clone(),
                title: note.title.clone(),
                color: note.color.clone(),
                status: note.status.clone(),
                source_path: note.source_path.clone(),
                updated_at: note.updated_at,
                revision: note.revision,
            };
            let metadata = serde_json::to_string(&meta)?
                .replace('<', "\\u003c")
                .replace('>', "\\u003e");
            let temp = path.with_extension(format!(
                "{}-{}.tmp",
                std::process::id(),
                SERIAL.fetch_add(1, Ordering::Relaxed)
            ));
            let mut options = fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            options.mode(0o600);
            let mut file = options.open(&temp)?;
            let result = (|| -> Result<(), Box<dyn std::error::Error>> {
                write!(file, "{PREFIX}{metadata}\n-->\n{}", note.markdown)?;
                file.sync_all()?;
                fs::rename(&temp, &path)?;
                Ok(())
            })();
            if result.is_err() {
                let _ = fs::remove_file(temp);
            }
            result
        })();
        result.map_err(|e| format!("메모 저장 실패: {e}"))
    }
    pub fn save_edit(
        &self,
        id: &str,
        note: &mut Note,
        markdown: String,
        revision: u64,
    ) -> Result<(), String> {
        let mut next = note.clone();
        next.save_edit(markdown, revision)?;
        if let Err(error) = self.save(id, &next) {
            if let Some(path) = &note.source_path {
                crate::source::save(path, &next.markdown, &note.markdown)
                    .map_err(|rollback| format!("{error}; 원본 복구 실패: {rollback}"))?;
            }
            return Err(error);
        }
        *note = next;
        Ok(())
    }
    pub fn load(&self) -> Result<BTreeMap<String, Note>, String> {
        let mut notes = BTreeMap::new();
        if !self.0.exists() {
            return Ok(notes);
        }
        for entry in fs::read_dir(&self.0).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if !entry_type_is_directory(&path)? {
                continue;
            }
            let path = path.join("index.md");
            if !path.exists() {
                continue;
            }
            let (id, note) = Self::read(&path, false)
                .map_err(|e| format!("메모 복원 실패 ({}): {e}", path.display()))?;
            notes.insert(id, note);
        }
        Ok(notes)
    }
    pub fn migrate_from(&self, legacy: &Path) -> Result<(), String> {
        if !legacy.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(legacy).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }
            let (id, note) = Self::read(&path, true)?;
            if !self.path(&id).exists() {
                self.save(&id, &note)?;
            }
        }
        Ok(())
    }
    fn read(path: &Path, legacy: bool) -> Result<(String, Note), String> {
        let name = if legacy {
            path.file_stem()
        } else {
            path.parent().and_then(Path::file_name)
        }
        .and_then(|s| s.to_str())
        .ok_or("잘못된 파일명")?;
        if !name.is_ascii() {
            return Err("잘못된 파일명".into());
        }
        let id = if legacy {
            if name.len() % 2 != 0 {
                return Err("잘못된 파일명".into());
            }
            let bytes = (0..name.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&name[i..i + 2], 16))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            String::from_utf8(bytes).map_err(|e| e.to_string())?
        } else {
            let mut id = String::new();
            let mut chars = name.chars();
            while let Some(c) = chars.next() {
                if c == '~' {
                    let hex: String = chars.by_ref().take(2).collect();
                    if hex.len() != 2 {
                        return Err("잘못된 폴더명".into());
                    }
                    let b = u8::from_str_radix(&hex, 16).map_err(|e| e.to_string())?;
                    if !b.is_ascii_alphanumeric() {
                        return Err("잘못된 폴더명".into());
                    }
                    id.push(char::from(b));
                } else {
                    id.push(c);
                }
            }
            id
        };
        let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
        let (header, markdown) = text
            .strip_prefix(PREFIX)
            .and_then(|s| s.split_once("\n-->\n"))
            .ok_or("저장 형식을 확인하세요")?;
        let meta: Metadata = serde_json::from_str(header).map_err(|e| e.to_string())?;
        let mut note = Note::default();
        note.apply(&Request {
            action: Action::Open,
            id: Some(id.clone()),
            color: Some(meta.color),
            title: Some(meta.title),
            status: Some(meta.status),
            source_path: meta.source_path,
            markdown: Some(markdown.into()),
        })?;
        note.capture = meta.capture;
        note.updated_at = meta.updated_at;
        note.revision = meta.revision.max(1);
        Ok((id, note))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windows_reserved_ids_roundtrip_without_case_collisions() {
        let root = std::env::temp_dir().join(format!("gibbon-reserved-{}", std::process::id()));
        let store = Store(root.clone());
        let ids = ["con", "CON", "Con", "aux", "nul", "prn", "com1", "lpt9"];
        for id in ids {
            let note = Note {
                markdown: id.into(),
                ..Note::default()
            };
            store.save(id, &note).unwrap();
            store.save(id, &note).unwrap();
        }
        let notes = store.load().unwrap();
        assert_eq!(notes.len(), ids.len());
        for id in ids {
            assert_eq!(notes[id].markdown, id);
        }
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn roundtrip_and_isolation() {
        let root = std::env::temp_dir().join(format!("gibbon-store-{}", std::process::id()));
        let store = Store::new(root.clone(), None);
        let mut note = Note {
            markdown: "# 한글\n- [x] 저장\n<!-- 본문 -->\n".into(),
            title: "제목 -->\n".into(),
            ..Note::default()
        };
        store.save("Ab", &note).unwrap();
        assert_eq!(store.path("ab"), root.join("agent-gibbon/docs/ab/index.md"));
        assert_ne!(store.path("Ab"), store.path("ab"));
        fs::write(
            store.path("Ab").parent().unwrap().join("asset.png"),
            b"untouched",
        )
        .unwrap();
        note.markdown = String::new();
        store.save("ab", &note).unwrap();
        let legacy = root.join("legacy");
        fs::create_dir_all(&legacy).unwrap();
        fs::copy(store.path("Ab"), legacy.join("4162.md")).unwrap();
        let migrated = Store(root.join("migrated"));
        migrated.migrate_from(&legacy).unwrap();
        assert_eq!(
            migrated.load().unwrap()["Ab"].markdown,
            "# 한글\n- [x] 저장\n<!-- 본문 -->\n"
        );
        migrated.save("Ab", &note).unwrap();
        migrated.migrate_from(&legacy).unwrap();
        assert_eq!(migrated.load().unwrap()["Ab"].markdown, "");
        assert!(legacy.join("4162.md").exists());
        note.capture = Some(crate::capture::ImageNote {
            x: -200.0,
            y: 50.0,
            width: 320.0,
            height: 180.0,
        });
        store.save("image", &note).unwrap();
        let image = store
            .load()
            .unwrap()
            .remove("image")
            .unwrap()
            .capture
            .unwrap();
        assert_eq!(
            (image.x, image.y, image.width, image.height),
            (-200.0, 50.0, 320.0, 180.0)
        );
        fs::remove_file(store.path("image")).unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.len(), 2);
        assert!(loaded["Ab"].markdown.starts_with("# 한글"));
        assert_eq!(loaded["Ab"].title, note.title);
        assert_eq!(loaded["ab"].markdown, "");
        assert!(Store::new(root.clone(), Some("dev-test"))
            .load()
            .unwrap()
            .is_empty());
        fs::write(store.path("Ab"), "broken").unwrap();
        assert!(store.load().is_err());
        fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod failure_tests {
    use super::*;
    #[test]
    fn edit_persists_and_failed_store_rolls_back_source_and_memory() {
        let root = std::env::temp_dir().join(format!("gibbon-edit-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let source = root.join("source.md");
        fs::write(&source, "old").unwrap();
        let mut note = Note {
            markdown: "old".into(),
            source_path: Some(source.to_str().unwrap().into()),
            ..Note::default()
        };
        let store = Store::new(root.clone(), None);
        store.save_edit("edit", &mut note, "new".into(), 0).unwrap();
        assert_eq!(store.load().unwrap()["edit"].markdown, "new");
        assert_eq!(fs::read_to_string(&source).unwrap(), "new");
        assert!(store
            .save_edit("edit", &mut note, "stale".into(), 0)
            .is_err());
        let blocked = root.join("blocked");
        fs::write(&blocked, "file").unwrap();
        assert!(Store(blocked)
            .save_edit("edit", &mut note, "failed".into(), 1)
            .is_err());
        assert_eq!(note.markdown, "new");
        assert_eq!(fs::read_to_string(&source).unwrap(), "new");
        fs::remove_dir_all(root).unwrap();
    }
}
