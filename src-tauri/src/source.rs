use std::{
    fs,
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

static SERIAL: AtomicU64 = AtomicU64::new(0);

// Keep the old file intact on validation or write failures. Only a complete
// replacement is renamed over the source, in the same directory/filesystem.
pub fn save(path: &str, expected: &str, replacement: &str) -> Result<(), String> {
    let path = Path::new(path);
    let check = || -> Result<fs::Metadata, String> {
        let metadata =
            fs::symlink_metadata(path).map_err(|e| format!("원본 파일 확인 실패: {e}"))?;
        if !metadata.is_file() || fs::read(path).map_err(|e| e.to_string())? != expected.as_bytes()
        {
            return Err("원본 파일이 외부에서 변경되었습니다. 작성 내용을 복사한 뒤 파일을 다시 열어 주세요.".into());
        }
        if metadata.permissions().readonly() {
            return Err("원본 파일이 읽기 전용입니다".into());
        }
        Ok(metadata)
    };
    let metadata = check()?;
    let parent = path.parent().ok_or("원본 파일의 디렉터리가 없습니다")?;
    let temporary = parent.join(format!(
        ".floatnote-{}-{}.tmp",
        std::process::id(),
        SERIAL.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)
        .map_err(|e| format!("저장 파일 생성 실패: {e}"))?;
    let result = (|| {
        file.write_all(replacement.as_bytes())
            .map_err(|e| e.to_string())?;
        file.set_permissions(metadata.permissions())
            .map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        check()?;
        fs::rename(&temporary, path).map_err(|e| format!("원본 파일 저장 실패: {e}"))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
