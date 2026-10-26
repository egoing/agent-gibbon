use crate::model::{Request, MAX_REQUEST};
use fs2::FileExt;
use serde_json::{json, Value};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
    time::Duration,
};

#[cfg(unix)]
use std::os::unix::{
    fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    net::{UnixListener, UnixStream},
};
#[cfg(windows)]
use uds_windows::{UnixListener, UnixStream};

#[cfg(unix)]
pub fn directory() -> Result<PathBuf, String> {
    // A short path avoids Darwin's 104-byte Unix socket path limit.
    let uid = unsafe { libc::geteuid() };
    let path = crate::instance::current().directory(uid);
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

#[cfg(windows)]
pub fn directory() -> Result<PathBuf, String> {
    let root = std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA is unavailable")?;
    let path = PathBuf::from(root).join("AgentGibbon").join(
        crate::instance::current()
            .id
            .as_deref()
            .unwrap_or("installed"),
    );
    fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    if fs::symlink_metadata(&path)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_symlink()
    {
        return Err("IPC directory cannot be a symbolic link".into());
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
        Self::bind_in(directory()?)
    }
    fn bind_in(dir: PathBuf) -> Result<Option<Self>, String> {
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
        let lock = options
            .open(dir.join("app.lock"))
            .map_err(|e| e.to_string())?;
        match lock.try_lock_exclusive() {
            Ok(()) => (),
            Err(e) if e.raw_os_error() == fs2::lock_contended_error().raw_os_error() => {
                return Ok(None)
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_locks_and_stale_socket_recovery() {
        let root =
            std::env::temp_dir().join(format!("floatnote-endpoint-test-{}", std::process::id()));
        let a = root.join("a");
        let b = root.join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        let first = Endpoint::bind_in(a.clone()).unwrap().unwrap();
        let second = Endpoint::bind_in(b.clone()).unwrap().unwrap();
        assert!(Endpoint::bind_in(a.clone()).unwrap().is_none());
        assert!(UnixStream::connect(a.join("app.sock")).is_ok());
        assert!(UnixStream::connect(b.join("app.sock")).is_ok());
        drop(first);
        assert!(!a.join("app.sock").exists());
        assert!(UnixStream::connect(b.join("app.sock")).is_ok());
        // A crashed process can leave its socket but cannot retain the flock.
        let stale = UnixListener::bind(a.join("app.sock")).unwrap();
        drop(stale);
        let restarted = Endpoint::bind_in(a).unwrap().unwrap();
        drop(restarted);
        drop(second);
        fs::remove_dir_all(root).unwrap();
    }
}
