//! Local, persistent colors for app-owned terminal/web chrome. Content is untouched.
use std::path::Path;
pub const PALETTE: [&str; 5] = ["#bdd657", "#db704f", "#6baa92", "#e5e4d2", "#4d4d4b"];
pub const LEGACY: &str = "#20252e";
pub fn random() -> String {
    use std::hash::{BuildHasher, Hasher};
    // RandomState uses a fresh OS-seeded key. This is visual variety, not security.
    let mut hash = std::collections::hash_map::RandomState::new().build_hasher();
    hash.write_u32(std::process::id());
    PALETTE[hash.finish() as usize % PALETTE.len()].into()
}
fn path(root: &Path, id: &str) -> Result<std::path::PathBuf, String> {
    if id.is_empty()
        || !id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
    {
        return Err("잘못된 메모 색상 ID".into());
    }
    Ok(root.join(".jajan-colors").join(format!("{id}.json")))
}
pub fn read(root: &Path, id: &str) -> Result<String, String> {
    let path = path(root, id)?;
    match std::fs::read(path) {
        Ok(bytes) => {
            let color: String = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            if valid(&color) {
                Ok(color)
            } else {
                Err("저장된 메모 색상이 잘못되었습니다".into())
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(LEGACY.into()),
        Err(e) => Err(e.to_string()),
    }
}
fn valid(color: &str) -> bool {
    color.len() == 7
        && color.starts_with('#')
        && color.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
}
pub fn write(root: &Path, id: &str, color: &str) -> Result<(), String> {
    use std::io::Write;
    if !valid(color) {
        return Err("색상은 #RRGGBB 형식이어야 합니다".into());
    }
    let path = path(root, id)?;
    let dir = path.parent().unwrap();
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut file = tempfile::NamedTempFile::new_in(dir).map_err(|e| e.to_string())?;
    file.write_all(&serde_json::to_vec(&color.to_ascii_lowercase()).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    file.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}
pub fn create(root: &Path, id: &str) -> Result<(), String> {
    if !path(root, id)?.exists() {
        write(root, id, &random())?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saved_colors_survive_recreation_and_legacy_is_not_recolored() {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(read(root.path(), "web-old").unwrap(), LEGACY);
        create(root.path(), "web-new").unwrap();
        assert!(PALETTE.contains(&read(root.path(), "web-new").unwrap().as_str()));
        write(root.path(), "web-new", "#ABCDEF").unwrap();
        create(root.path(), "web-new").unwrap();
        assert_eq!(read(root.path(), "web-new").unwrap(), "#abcdef");
        assert!(write(root.path(), "../bad", "#ffffff").is_err());
        assert!(write(root.path(), "web-new", "red").is_err());
    }
}
