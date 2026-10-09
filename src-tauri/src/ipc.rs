use crate::model::{Request, MAX_REQUEST};
use fs2::FileExt;
use serde_json::{json, Value};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::{
        fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
        net::{UnixListener, UnixStream},
    },
    path::PathBuf,
    time::Duration,
};

pub fn directory() -> Result<PathBuf, String> {
    // A short path avoids Darwin's 104-byte Unix socket path limit.
    let uid = unsafe { libc::geteuid() };
    let path = PathBuf::from(format!("/tmp/floatnote-{uid}"));
    let mut builder = fs::DirBuilder::new();
    builder.mode(0o700);
    match builder.create(&path) {
        Ok(()) => (),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(e) => return Err(e.to_string()),
    }
    let meta = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
    if !meta.is_dir() || meta.uid() != uid || meta.mode() & 0o077 != 0 {
        return Err("안전하지 않은 IPC 디렉터리입니다".into());
    }
    Ok(path)
}

pub struct Endpoint {
    pub listener: UnixListener,
    _lock: File,
    path: PathBuf,
}
impl Endpoint {
    pub fn bind() -> Result<Option<Self>, String> {
        let dir = directory()?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(dir.join("app.lock"))
            .map_err(|e| e.to_string())?;
        match lock.try_lock_exclusive() {
            Ok(()) => (),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(None),
            Err(e) => return Err(e.to_string()),
        }
        let path = dir.join("app.sock");
        match fs::remove_file(&path) {
            Ok(()) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.to_string()),
        }
        let listener = UnixListener::bind(&path).map_err(|e| e.to_string())?;
        Ok(Some(Self {
            listener,
            _lock: lock,
            path,
        }))
    }
}
impl Drop for Endpoint {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub fn connect() -> Result<UnixStream, String> {
    UnixStream::connect(directory()?.join("app.sock")).map_err(|e| e.to_string())
}
pub fn exchange(mut stream: UnixStream, request: &Request) -> Result<Value, String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec(request).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_REQUEST {
        return Err("요청은 1MiB 이하여야 합니다".into());
    }
    stream.write_all(&bytes).map_err(|e| e.to_string())?;
    stream
        .shutdown(std::net::Shutdown::Write)
        .map_err(|e| e.to_string())?;
    let mut response = Vec::new();
    stream
        .take(MAX_REQUEST * 8)
        .read_to_end(&mut response)
        .map_err(|e| e.to_string())?;
    serde_json::from_slice(&response).map_err(|e| e.to_string())
}
pub fn read_request(stream: &mut UnixStream) -> Result<Request, String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    stream
        .take(MAX_REQUEST + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_REQUEST {
        return Err("요청은 1MiB 이하여야 합니다".into());
    }
    serde_json::from_slice(&bytes).map_err(|_| "잘못된 JSON 요청입니다".to_string())
}
pub fn error(message: impl ToString) -> Value {
    json!({"ok": false, "error": message.to_string()})
}
