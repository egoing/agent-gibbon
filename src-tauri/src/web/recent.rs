//! Local, instance-isolated recent addresses. Opening a session records the
//! requested URL, not redirects or a claim that the remote page loaded.
use std::{
    fs,
    io::{Read, Write},
    path::Path,
    sync::Mutex,
};
const FILE: &str = ".web-recent.json";
const LIMIT: usize = 10;
const MAX_BYTES: u64 = 100 * 1024;
static SERIAL: Mutex<()> = Mutex::new(());
fn normalize(values: Vec<String>) -> Vec<String> {
    let mut result = Vec::new();
    for value in values {
        if let Ok(url) = super::validate_url(&value) {
            let url = url.to_string();
            if !result.contains(&url) {
                result.push(url);
            }
            if result.len() == LIMIT {
                break;
            }
        }
    }
    result
}
fn read(root: &Path) -> Result<Vec<String>, String> {
    let file = match fs::File::open(root.join(FILE)) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => return Err("최근 주소를 읽을 수 없습니다".into()),
    };
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "최근 주소를 읽을 수 없습니다")?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("최근 주소 파일이 너무 큽니다".into());
    }
    let urls = serde_json::from_slice(&bytes).map_err(|_| "최근 주소 파일을 읽을 수 없습니다")?;
    Ok(normalize(urls))
}
fn write(root: &Path, urls: &[String]) -> Result<(), String> {
    let save = || -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all(root)?;
        let mut temp = tempfile::NamedTempFile::new_in(root)?;
        temp.write_all(&serde_json::to_vec(urls)?)?;
        temp.as_file().sync_all()?;
        temp.persist(root.join(FILE))?;
        Ok(())
    };
    save().map_err(|_| "최근 주소를 저장할 수 없습니다".into())
}
pub fn list(root: &Path) -> Result<Vec<String>, String> {
    let _guard = SERIAL.lock().map_err(|_| "최근 주소를 읽을 수 없습니다")?;
    read(root)
}
pub fn remember(root: &Path, url: &str) -> Result<(), String> {
    let url = super::validate_url(url)?.to_string();
    let _guard = SERIAL
        .lock()
        .map_err(|_| "최근 주소를 저장할 수 없습니다")?;
    let mut urls = read(root)?;
    urls.retain(|old| old != &url);
    urls.insert(0, url);
    urls.truncate(LIMIT);
    write(root, &urls)
}
pub fn remove(root: &Path, url: &str) -> Result<Vec<String>, String> {
    let url = super::validate_url(url)?.to_string();
    let _guard = SERIAL
        .lock()
        .map_err(|_| "최근 주소를 저장할 수 없습니다")?;
    let mut urls = read(root)?;
    urls.retain(|old| old != &url);
    write(root, &urls)?;
    Ok(urls)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recent_addresses_survive_reload_deduplicate_limit_and_delete() {
        let root = tempfile::tempdir().unwrap();
        assert!(list(root.path()).unwrap().is_empty());
        for i in 0..12 {
            remember(root.path(), &format!("https://example.com/{i}")).unwrap();
        }
        remember(root.path(), "https://example.com/5").unwrap();
        let urls = list(root.path()).unwrap();
        assert_eq!(urls.len(), 10);
        assert_eq!(urls[0], "https://example.com/5");
        assert_eq!(urls.iter().filter(|u| u.ends_with("/5")).count(), 1);
        remove(root.path(), &urls[0]).unwrap();
        assert_eq!(list(root.path()).unwrap().len(), 9);
        assert!(!list(root.path()).unwrap().contains(&urls[0]));
        assert!(list(tempfile::tempdir().unwrap().path())
            .unwrap()
            .is_empty());
    }
    #[test]
    fn invalid_records_never_reach_the_ui_and_corruption_is_preserved() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join(FILE), r#"["file:///tmp/x","https://u:p@example.com/","https://tauri.localhost/","https://example.com","https://example.com/"]"#).unwrap();
        assert_eq!(list(root.path()).unwrap(), vec!["https://example.com/"]);
        assert!(remember(root.path(), "javascript:1").is_err());
        fs::write(root.path().join(FILE), "broken").unwrap();
        assert!(remember(root.path(), "https://example.com/").is_err());
        assert_eq!(
            fs::read_to_string(root.path().join(FILE)).unwrap(),
            "broken"
        );
    }
}
