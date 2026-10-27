use std::{io::Write, path::Path};
use tauri::{Manager, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

// Capture the content before opening the dialog; never hold the notes lock while
// the user chooses a destination. Export does not change the note/source link.
#[tauri::command]
pub async fn export_note(
    window: WebviewWindow,
    markdown: Option<String>,
) -> Result<Option<String>, String> {
    let id = window
        .label()
        .strip_prefix("note-")
        .ok_or("메모 창이 아닙니다")?;
    let note = window
        .state::<crate::AppNotes>()
        .lock()
        .map_err(|e| e.to_string())?
        .get(id)
        .cloned()
        .ok_or("메모를 찾을 수 없습니다")?;
    let (extension, bytes) = if note.capture.is_some() {
        ("png", crate::capture::note_image_bytes(&window)?)
    } else {
        ("md", markdown.unwrap_or(note.markdown).into_bytes())
    };
    let filename = format!("{id}.{extension}");
    tauri::async_runtime::spawn_blocking(move || {
        let selected = window
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("파일로 저장")
            .add_filter(
                if extension == "png" {
                    "PNG 이미지"
                } else {
                    "Markdown"
                },
                &[extension],
            )
            .set_file_name(filename)
            .blocking_save_file();
        let Some(selected) = selected else {
            return Ok(None);
        };
        let path = selected.into_path().map_err(|e| e.to_string())?;
        write_export(&path, &bytes)?;
        Ok(Some(path.to_string_lossy().into_owned()))
    })
    .await
    .map_err(|e| e.to_string())?
}

fn write_export(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("저장할 폴더가 없습니다")?;
    let mut temp =
        tempfile::NamedTempFile::new_in(parent).map_err(|e| format!("저장 파일 생성 실패: {e}"))?;
    temp.write_all(bytes)
        .map_err(|e| format!("파일 저장 실패: {e}"))?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(path)
        .map_err(|e| format!("파일 저장 실패: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exports_utf8_and_image_bytes_and_replaces_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("한글.md");
        write_export(&path, "# 메모\n".as_bytes()).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "# 메모\n");
        let png = b"\x89PNG\r\n\x1a\n\0\xff";
        write_export(&path, png).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), png);
    }
    #[test]
    fn failed_export_preserves_destination_and_cleans_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("directory");
        std::fs::create_dir(&path).unwrap();
        assert!(write_export(&path, b"data").is_err());
        assert!(path.is_dir());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
        assert!(write_export(&dir.path().join("missing/note.md"), b"data").is_err());
    }
}
