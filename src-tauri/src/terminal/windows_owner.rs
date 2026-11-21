//! Explicitly enabled local ConPTY owner. No terminal traffic is written to disk.
use super::*;
use fs2::FileExt;
use portable_pty::{Child, ChildKiller, ExitStatus};
use sha2::{Digest, Sha256};
use std::{
    collections::VecDeque,
    fs,
    io::{self, Read},
    net::Shutdown,
    os::windows::{ffi::OsStrExt, fs::MetadataExt, process::CommandExt},
    process::{Command, Stdio},
    thread,
};
use uds_windows::{UnixListener, UnixStream};
use windows_sys::Win32::{
    Foundation::*,
    Security::{Authorization::*, *},
    Storage::FileSystem::*,
    System::{JobObjects::*, Threading::*},
};

const LIMIT: usize = 65536;
const WIRE_LIMIT: u64 = 512 * 1024;
const TIMEOUT: Duration = Duration::from_secs(2);
static RUNTIME_ENABLED: AtomicBool = AtomicBool::new(false);
pub(crate) fn set_runtime_enabled(enabled: bool) {
    RUNTIME_ENABLED.store(enabled, Ordering::Release);
}
pub(crate) fn enabled() -> bool {
    RUNTIME_ENABLED.load(Ordering::Acquire)
}
fn installation_key(image: &Path) -> String {
    let text = image.to_string_lossy();
    let text = text.strip_prefix(r"\\?\").unwrap_or(&text).to_lowercase();
    format!("{:x}", Sha256::digest(text.as_bytes()))[..16].into()
}
pub(super) fn installation_namespace() -> io::Result<Option<String>> {
    if crate::instance::current().id.is_some() {
        return Ok(None);
    }
    let image = std::env::current_exe()?;
    let parent = image
        .parent()
        .ok_or_else(|| io::Error::other("owner path missing"))?;
    if image
        .file_name()
        .is_some_and(|n| n.to_string_lossy().starts_with("owner-"))
    {
        let namespace = parent
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_prefix("p-"))
            .filter(|s| s.len() == 16 && s.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or_else(|| io::Error::other("owner namespace invalid"))?;
        let base = crate::ipc::directory()
            .map_err(io::Error::other)?
            .canonicalize()?;
        if parent
            .parent()
            .ok_or_else(|| io::Error::other("owner namespace missing"))?
            .canonicalize()?
            != base
        {
            return Err(io::Error::other("owner namespace mismatch"));
        }
        return Ok(Some(namespace.into()));
    }
    Ok(Some(installation_key(&image.canonicalize()?)))
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Reconnect {
    version: u8,
    generation: String,
    owner_pid: u32,
    shell_pid: u32,
}
fn reconnect(path: &Path) -> io::Result<Reconnect> {
    let file = fs::File::open(path.with_extension("session"))?;
    let mut bytes = Vec::new();
    file.take(4097).read_to_end(&mut bytes)?;
    let value: Reconnect = serde_json::from_slice(&bytes)?;
    if bytes.len() > 4096
        || value.version != 2
        || value.owner_pid == 0
        || value.shell_pid == 0
        || !(24..=128).contains(&value.generation.len())
        || !value.generation.bytes().all(|b| b.is_ascii_alphanumeric())
    {
        return Err(io::Error::other("invalid reconnect checkpoint"));
    }
    Ok(value)
}
fn identity(value: &Value, ticket: &Reconnect) -> bool {
    value["protocol"] == 2
        && value["generation"] == ticket.generation
        && value["owner_pid"] == ticket.owner_pid
        && value["pid"] == ticket.shell_pid
}
pub(super) fn available(id: &str) -> bool {
    let Ok(path) = endpoint(id) else {
        return false;
    };
    let Ok(ticket) = reconnect(&path) else {
        return false;
    };
    exchange(
        &path,
        &json!({"op":"available","generation":ticket.generation}),
    )
    .ok()
    .is_some_and(|v| identity(&v, &ticket) && v["available"] == true)
}
pub(super) fn reconnect_count() -> usize {
    let Ok(dir) = private_directory() else {
        return 0;
    };
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|entry| {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("session") {
                return false;
            }
            let socket = path.with_extension("");
            let Ok(ticket) = reconnect(&socket) else {
                return false;
            };
            exchange(
                &socket,
                &json!({"op":"available","generation":ticket.generation}),
            )
            .ok()
            .is_some_and(|v| identity(&v, &ticket) && v["live"] == true)
        })
        .count()
}
fn file_digest(path: &Path) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut bytes = [0; 65536];
    loop {
        let n = file.read(&mut bytes)?;
        if n == 0 {
            break;
        }
        hash.update(&bytes[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
/// Versioned executable outside the installation tree. Windows can replace the
/// GUI binary while the old owner continues running its original protocol.
pub(crate) fn owner_executable() -> io::Result<PathBuf> {
    if !enabled() {
        return Err(io::Error::other("development opt-in required"));
    }
    prepare_owner_executable()
}
pub(crate) fn prepare_owner_executable() -> io::Result<PathBuf> {
    let source = std::env::current_exe()?;
    let digest = file_digest(&source)?;
    let dir = private_directory()?;
    if dir.join("000000000000").to_string_lossy().len() > 107 {
        return Err(io::Error::other(
            "ConPTY IPC path exceeds the Windows AF_UNIX limit",
        ));
    }
    let target = dir.join(format!("owner-{}.exe", &digest[..24]));
    if !target.exists() {
        let mut file = tempfile::NamedTempFile::new_in(&dir)?;
        io::copy(&mut fs::File::open(&source)?, &mut file)?;
        file.as_file().sync_all()?;
        match file.persist_noclobber(&target) {
            Ok(_) => (),
            Err(e) if e.error.kind() == io::ErrorKind::AlreadyExists => (),
            Err(e) => return Err(e.error),
        }
    }
    if fs::symlink_metadata(&target)?.file_attributes() & 0x400 != 0
        || file_digest(&target)? != digest
    {
        return Err(io::Error::other("owner executable mismatch"));
    }
    Ok(target)
}
fn wide(s: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    s.as_ref().encode_wide().chain(Some(0)).collect()
}
fn check(ok: i32) -> io::Result<()> {
    if ok == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
struct Handle(HANDLE);
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
unsafe impl Send for Handle {}

// Create atomically with a protected DACL. Existing directories must match it;
// never repair permissions on an arbitrary pre-existing object.
fn private_directory() -> io::Result<PathBuf> {
    let dir =
        crate::ipc::directory()
            .map_err(io::Error::other)?
            .join(match installation_namespace()? {
                Some(id) => format!("p-{id}"),
                None => "p".into(),
            });
    for path in dir.ancestors().filter(|p| p.exists()) {
        if fs::symlink_metadata(path)?.file_attributes() & 0x400 != 0 {
            return Err(io::Error::other("reparse path rejected"));
        }
    }
    unsafe {
        let mut token = std::ptr::null_mut();
        check(OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_QUERY,
            &mut token,
        ))?;
        let token = Handle(token);
        let mut len = 0;
        GetTokenInformation(token.0, TokenUser, std::ptr::null_mut(), 0, &mut len);
        let mut buf = vec![0usize; (len as usize).div_ceil(std::mem::size_of::<usize>())];
        check(GetTokenInformation(
            token.0,
            TokenUser,
            buf.as_mut_ptr().cast(),
            len,
            &mut len,
        ))?;
        let user = &*(buf.as_ptr().cast::<TOKEN_USER>());
        let mut sid = std::ptr::null_mut();
        check(ConvertSidToStringSidW(user.User.Sid, &mut sid))?;
        let mut n = 0;
        while *sid.add(n) != 0 {
            n += 1;
        }
        let sid_text = String::from_utf16_lossy(std::slice::from_raw_parts(sid, n));
        LocalFree(sid.cast());
        let expected = format!("D:P(A;;FA;;;{sid_text})");
        let mut sd = std::ptr::null_mut();
        check(ConvertStringSecurityDescriptorToSecurityDescriptorW(
            wide(&expected).as_ptr(),
            1,
            &mut sd,
            std::ptr::null_mut(),
        ))?;
        let sa = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: sd,
            bInheritHandle: 0,
        };
        let created = CreateDirectoryW(wide(&dir).as_ptr(), &sa);
        let error = io::Error::last_os_error();
        LocalFree(sd);
        if created == 0 && error.raw_os_error() != Some(ERROR_ALREADY_EXISTS as i32) {
            return Err(error);
        }
        let mut needed = 0;
        GetFileSecurityW(
            wide(&dir).as_ptr(),
            DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            0,
            &mut needed,
        );
        let mut data = vec![0usize; (needed as usize).div_ceil(std::mem::size_of::<usize>())];
        check(GetFileSecurityW(
            wide(&dir).as_ptr(),
            DACL_SECURITY_INFORMATION,
            data.as_mut_ptr().cast(),
            needed,
            &mut needed,
        ))?;
        let mut text = std::ptr::null_mut();
        check(ConvertSecurityDescriptorToStringSecurityDescriptorW(
            data.as_ptr().cast_mut().cast(),
            1,
            DACL_SECURITY_INFORMATION,
            &mut text,
            std::ptr::null_mut(),
        ))?;
        let mut n = 0;
        while *text.add(n) != 0 {
            n += 1;
        }
        let actual = String::from_utf16_lossy(std::slice::from_raw_parts(text, n));
        LocalFree(text.cast());
        if actual != expected || fs::symlink_metadata(&dir)?.file_attributes() & 0x400 != 0 {
            return Err(io::Error::other("private DACL required"));
        }
    }
    Ok(dir)
}
fn endpoint(id: &str) -> io::Result<PathBuf> {
    if !valid_id(id) {
        return Err(io::Error::other("invalid session"));
    }
    let hash = format!("{:x}", Sha256::digest(id.as_bytes()));
    let path = private_directory()?.join(&hash[..12]);
    if path.to_string_lossy().len() > 107 {
        return Err(io::Error::other("IPC path too long"));
    }
    Ok(path)
}
fn exchange(path: &Path, request: &Value) -> io::Result<Value> {
    let packet = serde_json::to_vec(request)?;
    let read_only = matches!(request["op"].as_str(), Some("read" | "status"));
    for attempt in 0..3 {
        let result = exchange_once(path, &packet, read_only);
        let retry = result
            .as_ref()
            .err()
            .is_some_and(|e| e.get_ref().is_some_and(|e| e.is::<EmptyReset>()))
            && read_only
            && attempt < 2;
        if !retry {
            return result;
        }
        super::verification_gate(
            || json!({"transport":"empty-reset-retry","attempt":attempt+1,"read":request["op"]=="read","possible_output_gap":request["op"]=="read"}),
        );
        if request["op"] == "read" {
            let mut status = request.clone();
            status["op"] = json!("status");
            let health = exchange(path, &status)?;
            if health["protocol"] != 2
                || health["checkpoint"] != true
                || health["durable_executable"] != true
                || !health["exit"].is_null()
            {
                return Err(io::Error::other("PTY owner health changed during recovery"));
            }
        }
        thread::sleep(Duration::from_millis(20));
    }
    unreachable!()
}
#[derive(Debug)]
struct EmptyReset;
impl std::fmt::Display for EmptyReset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PTY empty reset response")
    }
}
impl std::error::Error for EmptyReset {}
fn exchange_once(path: &Path, packet: &[u8], read_only: bool) -> io::Result<Value> {
    let mut socket = UnixStream::connect(path)?;
    socket.set_read_timeout(Some(TIMEOUT))?;
    socket.set_write_timeout(Some(TIMEOUT))?;
    socket.write_all(packet)?;
    socket.shutdown(Shutdown::Write)?;
    let mut bytes = Vec::new();
    let result = socket.take(WIRE_LIMIT).read_to_end(&mut bytes);
    decode_reply(&bytes, result, read_only)
}
fn decode_reply(bytes: &[u8], result: io::Result<usize>, read_only: bool) -> io::Result<Value> {
    if bytes.len() as u64 >= WIRE_LIMIT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "PTY response limit",
        ));
    }
    let decoded = serde_json::from_slice::<Value>(bytes);
    if let Err(error) = result {
        let complete = decoded
            .as_ref()
            .is_ok_and(|v| v.is_object() && v["ok"] == true);
        let accepted = read_only && error.kind() == io::ErrorKind::ConnectionReset && complete;
        super::verification_gate(
            || json!({"transport":"response-end-error","kind":format!("{:?}",error.kind()),"raw_os_error":error.raw_os_error(),"received_len":bytes.len(),"complete_object":complete,"accepted":accepted}),
        );
        if !accepted {
            if read_only && bytes.is_empty() && error.kind() == io::ErrorKind::ConnectionReset {
                return Err(io::Error::new(io::ErrorKind::ConnectionReset, EmptyReset));
            }
            return Err(error);
        }
        // Winsock may report reset instead of EOF after the complete response.
        // Use those original bytes once; never repeat a draining read or a write.
    }
    let value = decoded?;
    if value["ok"] != true {
        return Err(io::Error::other("PTY owner rejected request"));
    }
    Ok(value)
}

#[derive(Clone, Debug)]
pub(super) struct Link {
    path: PathBuf,
    attachment: String,
    active: Arc<AtomicBool>,
}
impl Link {
    pub(super) fn survives_update(&self) -> bool {
        match self.call("status", Value::Null) {
            Ok(v) => {
                let ready = v["protocol"] == 2
                    && v["checkpoint"] == true
                    && v["durable_executable"] == true
                    && v["exit"].is_null();
                if !ready {
                    super::verification_gate(
                        || json!({"remote":"status","protocol":v["protocol"],"checkpoint":v["checkpoint"],"durable_executable":v["durable_executable"],"exit":v["exit"]}),
                    );
                }
                ready
            }
            Err(error) => {
                super::verification_gate(
                    || json!({"remote":"rpc-error","kind":format!("{:?}",error.kind())}),
                );
                false
            }
        }
    }
    pub(super) fn detach(&self) {
        let _ = self.call("detach", Value::Null);
        self.active.store(false, Ordering::Release);
    }
    pub(super) fn stop(&self) {
        if self.active.load(Ordering::Acquire) {
            let _ = self.call("stop", Value::Null);
            self.active.store(false, Ordering::Release);
        }
    }
    fn call(&self, op: &str, body: Value) -> io::Result<Value> {
        if !self.active.load(Ordering::Acquire) {
            return Err(io::Error::other("detached"));
        }
        exchange(
            &self.path,
            &json!({"op":op,"attachment":self.attachment,"body":body}),
        )
    }
}
#[derive(Debug)]
pub(super) struct RemoteChild {
    link: Link,
    pid: u32,
}
impl RemoteChild {
    pub(super) fn control(&self) -> Link {
        self.link.clone()
    }
}
impl ChildKiller for RemoteChild {
    fn kill(&mut self) -> io::Result<()> {
        self.link.call("stop", Value::Null)?;
        self.link.active.store(false, Ordering::Release);
        Ok(())
    }
    fn clone_killer(&self) -> Box<dyn ChildKiller + Send + Sync> {
        Box::new(Self {
            link: self.link.clone(),
            pid: self.pid,
        })
    }
}
impl Child for RemoteChild {
    fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        if !self.link.active.load(Ordering::Acquire) {
            return Ok(Some(ExitStatus::with_exit_code(0)));
        }
        Ok(self.link.call("status", Value::Null)?["exit"]
            .as_u64()
            .map(|c| ExitStatus::with_exit_code(c as u32)))
    }
    fn wait(&mut self) -> io::Result<ExitStatus> {
        loop {
            if let Some(s) = self.try_wait()? {
                return Ok(s);
            }
            thread::sleep(lifecycle::PROBE);
        }
    }
    fn process_id(&self) -> Option<u32> {
        Some(self.pid)
    }
    fn as_raw_handle(&self) -> Option<std::os::windows::io::RawHandle> {
        None
    }
}
struct RemoteMaster(Link);
impl MasterPty for RemoteMaster {
    fn resize(&self, size: PtySize) -> anyhow::Result<()> {
        self.0
            .call("resize", json!({"cols":size.cols,"rows":size.rows}))?;
        Ok(())
    }
    fn get_size(&self) -> anyhow::Result<PtySize> {
        let v = self.0.call("status", Value::Null)?;
        Ok(PtySize {
            cols: v["cols"].as_u64().unwrap_or(80) as u16,
            rows: v["rows"].as_u64().unwrap_or(24) as u16,
            pixel_width: 0,
            pixel_height: 0,
        })
    }
    fn try_clone_reader(&self) -> anyhow::Result<Box<dyn Read + Send>> {
        Ok(Box::new(RemoteReader {
            link: self.0.clone(),
            pending: VecDeque::new(),
        }))
    }
    fn take_writer(&self) -> anyhow::Result<Box<dyn Write + Send>> {
        Ok(Box::new(RemoteWriter(self.0.clone())))
    }
}
struct RemoteWriter(Link);
impl Write for RemoteWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let n = bytes.len().min(8192);
        self.0.call("write", json!(bytes[..n]))?;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
struct RemoteReader {
    link: Link,
    pending: VecDeque<u8>,
}
impl Read for RemoteReader {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        while self.pending.is_empty() {
            if !self.link.active.load(Ordering::Acquire) {
                super::verification_gate(|| json!({"reader_end":"inactive-link"}));
                return Ok(0);
            }
            let v = self.link.call("read", Value::Null).inspect_err(|error| {
                super::verification_gate(
                    || json!({"reader_end":"rpc-error","kind":format!("{:?}",error.kind())}),
                );
            })?;
            self.pending = serde_json::from_value(v["bytes"].clone()).inspect_err(|error| {
                super::verification_gate(
                    || json!({"reader_end":"reply-data","kind":format!("{:?}",error.classify())}),
                );
            })?;
            if self.pending.is_empty() {
                if !v["exit"].is_null() {
                    super::verification_gate(
                        || json!({"reader_end":"child-exit","exit":v["exit"]}),
                    );
                    return Ok(0);
                }
                thread::sleep(Duration::from_millis(20));
            }
        }
        let n = out.len().min(self.pending.len());
        for byte in &mut out[..n] {
            *byte = self.pending.pop_front().unwrap();
        }
        Ok(n)
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn start(
    cwd: &Path,
    line: Option<String>,
    dimensions: PtySize,
    shell: &Path,
    id: &str,
    mut prompt: welcome::PromptState,
    attach_only: bool,
    spawned: impl FnOnce(OwnedChild),
) -> Result<(Session, PtyReader), String> {
    let path = endpoint(id).map_err(err)?;
    if !attach_only && !enabled() {
        return Err("터미널 분리를 명시적으로 활성화한 뒤 사용할 수 있습니다".into());
    }
    let ticket = if attach_only {
        Some(reconnect(&path).map_err(err)?)
    } else {
        None
    };
    let attach = json!({"generation":ticket.as_ref().map(|t| &t.generation),"op":"attach","create":!attach_only,"pid":std::process::id(),"cols":dimensions.cols,"rows":dimensions.rows,"shell":shell,"cwd":cwd,"line":line,"prompt":prompt.token});
    let mut launched = false;
    let deadline = Instant::now() + Duration::from_secs(5);
    let response = loop {
        match exchange(&path, &attach) {
            Ok(v) => break v,
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
                ) =>
            {
                if attach_only || Instant::now() >= deadline {
                    return Err("PTY 관리 프로세스 연결 시간 초과".into());
                }
                if !launched {
                    let mut command = Command::new(owner_executable().map_err(err)?);
                    command
                        .arg("--pty-owner")
                        .arg(id)
                        .env("JAJAN_POC_WINDOWS_PTY", "1")
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null());
                    // Try to escape an outer launcher job. If that job disallows
                    // breakaway, normal GUI termination is supported but killing
                    // that entire outer job remains outside this experiment.
                    command.creation_flags(0x0000_0008 | 0x0000_0200 | 0x0100_0000);
                    if command.spawn().is_err() {
                        command.creation_flags(0x0000_0008 | 0x0000_0200);
                        command.spawn().map_err(err)?;
                    }
                    launched = true;
                }
                thread::sleep(Duration::from_millis(30));
            }
            Err(e) => return Err(err(e)),
        }
    };
    if ticket.as_ref().is_some_and(|t| !identity(&response, t)) {
        return Err("재접속 소유자와 셸 식별이 일치하지 않습니다".into());
    }
    prompt.preserve_plan = attach_only;
    prompt.token = response["prompt"].as_str().ok_or("missing prompt")?.into();
    let link = Link {
        path,
        attachment: response["attachment"]
            .as_str()
            .ok_or("missing attachment")?
            .into(),
        active: Arc::new(AtomicBool::new(true)),
    };
    let child = OwnedChild::new(Box::new(RemoteChild {
        link: link.clone(),
        pid: response["pid"].as_u64().ok_or("missing pid")? as u32,
    }));
    spawned(child.clone());
    let master: Box<dyn MasterPty + Send> = Box::new(RemoteMaster(link));
    let reader = PtyReader::new(&*master)?;
    let writer = master.take_writer().map_err(err)?;
    Ok((
        Session {
            master,
            writer,
            child,
            program: "shell".into(),
            observed_command: false,
            prompt,
        },
        reader,
    ))
}

fn job() -> io::Result<Handle> {
    unsafe {
        let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        let handle = Handle(handle);
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        check(SetInformationJobObject(
            handle.0,
            JobObjectExtendedLimitInformation,
            (&info as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
            std::mem::size_of_val(&info) as u32,
        ))?;
        // Assign owner BEFORE spawning shell, so even early shell descendants
        // belong to this job. Handle is noninheritable and only owner keeps it.
        check(AssignProcessToJobObject(handle.0, GetCurrentProcess()))?;
        Ok(handle)
    }
}
fn configure_accepted(socket: &UnixStream) -> io::Result<()> {
    socket.set_nonblocking(false)?;
    socket.set_read_timeout(Some(TIMEOUT))?;
    socket.set_write_timeout(Some(TIMEOUT))
}
fn alive(handle: &Handle) -> bool {
    unsafe { WaitForSingleObject(handle.0, 0) == WAIT_TIMEOUT }
}
fn tail(buffer: &mut VecDeque<u8>, bytes: &[u8]) {
    buffer.extend(bytes);
    if buffer.len() > LIMIT {
        buffer.drain(..buffer.len() - LIMIT);
    }
}
pub(crate) fn helper(args: &[String]) -> io::Result<()> {
    if args.len() != 1 {
        return Err(io::Error::other("invalid owner args"));
    }
    crate::instance::init().map_err(io::Error::other)?;
    if std::env::var("JAJAN_POC_WINDOWS_PTY").as_deref() != Ok("1") {
        return Err(io::Error::other("explicit owner opt-in required"));
    }
    let path = endpoint(&args[0])?;
    let executable = std::env::current_exe()?;
    let durable_executable = executable.parent().and_then(|p| p.canonicalize().ok())
        == path.parent().and_then(|p| p.canonicalize().ok())
        && executable
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with("owner-"));
    if !durable_executable {
        return Err(io::Error::other("private detached owner image required"));
    }
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path.with_extension("lock"))?;
    lock.try_lock_exclusive()?;
    match fs::remove_file(&path) {
        Ok(()) => (),
        Err(e) if e.kind() == io::ErrorKind::NotFound => (),
        Err(e) => return Err(e),
    }
    let listener = UnixListener::bind(&path)?;
    listener.set_nonblocking(true)?;
    let nonce = tempfile::Builder::new()
        .prefix("g")
        .rand_bytes(32)
        .tempfile_in(path.parent().unwrap())?;
    let generation = nonce
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    drop(nonce);
    let _job = job()?;
    let buffer = Arc::new(Mutex::new(VecDeque::new()));
    let mut master: Option<Box<dyn MasterPty + Send>> = None;
    let mut child: Option<Box<dyn Child + Send + Sync>> = None;
    let mut input: Option<mpsc::SyncSender<Vec<u8>>> = None;
    let mut attached: Option<(String, Handle)> = None;
    let mut prompt = String::new();
    let mut deadline = Instant::now() + Duration::from_secs(5);
    let lease = std::env::var("JAJAN_POC_DETACH_SECONDS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(300)
        .clamp(1, 900);
    let mut exit = None;
    loop {
        if attached.as_ref().is_some_and(|(_, p)| !alive(p)) {
            attached = None;
            deadline = Instant::now() + Duration::from_secs(lease);
        }
        if attached.is_none() && Instant::now() > deadline {
            break;
        }
        if let Some(c) = child.as_mut() {
            if let Some(status) = c.try_wait()? {
                exit = Some(status.exit_code());
            }
        }
        let (mut socket, _) = match listener.accept() {
            Ok(s) => s,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(5));
                continue;
            }
            Err(e) => return Err(e),
        };
        // Winsock accept inherits the listener's nonblocking mode. Wait for
        // the complete request instead of dropping a not-yet-written client.
        configure_accepted(&socket)?;
        let mut bytes = Vec::new();
        if Read::by_ref(&mut socket)
            .take(WIRE_LIMIT)
            .read_to_end(&mut bytes)
            .is_err()
        {
            continue;
        }
        let Ok(request) = serde_json::from_slice::<Value>(&bytes) else {
            continue;
        };
        let op = request["op"].as_str().unwrap_or("");
        let mut response = json!({"ok":false});
        let mut stop = false;
        if op == "available" && request["generation"] == generation {
            response = json!({"ok":true,"protocol":2,"generation":generation,"owner_pid":std::process::id(),"pid":child.as_ref().and_then(|c|c.process_id()),"live":child.is_some() && exit.is_none(),"available":child.is_some() && attached.is_none() && exit.is_none()});
        } else if op == "attach"
            && attached.is_none()
            && exit.is_none()
            && (child.is_none() || request["generation"] == generation)
            && (child.is_some() || request["create"] == true)
            && (child.is_none() || request["line"].is_null() || request["line"] == "")
        {
            let pid = request["pid"].as_u64().unwrap_or(0) as u32;
            let process = unsafe { OpenProcess(SYNCHRONIZE, 0, pid) };
            if !process.is_null() {
                let process = Handle(process);
                if alive(&process) {
                    if child.is_none() {
                        let dims = size(
                            request["cols"].as_u64().unwrap_or(0) as u16,
                            request["rows"].as_u64().unwrap_or(0) as u16,
                        )
                        .map_err(io::Error::other)?;
                        let pair = native_pty_system()
                            .openpty(dims)
                            .map_err(io::Error::other)?;
                        let mut reader =
                            pair.master.try_clone_reader().map_err(io::Error::other)?;
                        let mut writer = pair.master.take_writer().map_err(io::Error::other)?;
                        prompt = request["prompt"]
                            .as_str()
                            .ok_or_else(|| io::Error::other("prompt required"))?
                            .into();
                        let mut cmd = CommandBuilder::new(
                            request["shell"]
                                .as_str()
                                .ok_or_else(|| io::Error::other("shell required"))?,
                        );
                        cmd.cwd(
                            request["cwd"]
                                .as_str()
                                .ok_or_else(|| io::Error::other("cwd required"))?,
                        );
                        cmd.env("TERM", "xterm-256color");
                        cmd.env_remove("NO_COLOR");
                        cmd.args(["-NoLogo", "-NoExit", "-Command"]);
                        // Same hook as the ordinary Windows terminal. Initial
                        // command belongs ONLY to creation, never reattachment.
                        cmd.arg(format!(
                            "{}{}",
                            welcome::windows_prompt_script(&prompt),
                            request["line"].as_str().unwrap_or("")
                        ));
                        child = Some(pair.slave.spawn_command(cmd).map_err(io::Error::other)?);
                        drop(pair.slave);
                        let output = buffer.clone();
                        let (tx, rx) = mpsc::sync_channel::<Vec<u8>>(8);
                        let replies = tx.clone();
                        thread::spawn(move || {
                            let mut bytes = [0; 8192];
                            let mut cursor_query = Vec::new();
                            let mut inherited_cursor = false;
                            loop {
                                match reader.read(&mut bytes) {
                                    Ok(0) | Err(_) => break,
                                    Ok(n) => {
                                        // portable-pty requests cursor inheritance. Complete
                                        // that one initial query without a connected renderer.
                                        let mut visible = Vec::with_capacity(n);
                                        for byte in &bytes[..n] {
                                            if inherited_cursor {
                                                visible.push(*byte);
                                                continue;
                                            }
                                            cursor_query.push(*byte);
                                            while !b"\x1b[6n".starts_with(&cursor_query) {
                                                visible.push(cursor_query.remove(0));
                                            }
                                            if cursor_query == b"\x1b[6n" {
                                                let _ = replies.try_send(b"\x1b[1;1R".to_vec());
                                                cursor_query.clear();
                                                inherited_cursor = true;
                                            }
                                        }
                                        tail(&mut output.lock().unwrap(), &visible);
                                    }
                                }
                            }
                        });
                        thread::spawn(move || {
                            while let Ok(bytes) = rx.recv() {
                                if writer
                                    .write_all(&bytes)
                                    .and_then(|_| writer.flush())
                                    .is_err()
                                {
                                    break;
                                }
                            }
                        });
                        input = Some(tx);
                        master = Some(pair.master);
                    }
                    let dimensions = size(
                        request["cols"].as_u64().unwrap_or(0) as u16,
                        request["rows"].as_u64().unwrap_or(0) as u16,
                    )
                    .map_err(io::Error::other)?;
                    master
                        .as_ref()
                        .unwrap()
                        .resize(dimensions)
                        .map_err(io::Error::other)?;
                    // Unique random capability held in memory, never logged.
                    let attachment = tempfile::Builder::new()
                        .prefix("a")
                        .rand_bytes(24)
                        .tempfile_in(path.parent().unwrap())?;
                    let key = attachment
                        .path()
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .into_owned();
                    drop(attachment);
                    let ticket = Reconnect {
                        version: 2,
                        generation: generation.clone(),
                        owner_pid: std::process::id(),
                        shell_pid: child
                            .as_ref()
                            .and_then(|c| c.process_id())
                            .ok_or_else(|| io::Error::other("shell pid missing"))?,
                    };
                    let mut checkpoint = tempfile::NamedTempFile::new_in(path.parent().unwrap())?;
                    serde_json::to_writer(&mut checkpoint, &ticket)?;
                    checkpoint.as_file().sync_all()?;
                    checkpoint
                        .persist(path.with_extension("session"))
                        .map_err(io::Error::other)?;
                    response = json!({"ok":true,"protocol":2,"generation":generation,"owner_pid":ticket.owner_pid,"attachment":key,"pid":ticket.shell_pid,"prompt":prompt});
                    attached = Some((key, process));
                }
            }
        } else if attached
            .as_ref()
            .is_some_and(|(key, _)| Some(key.as_str()) == request["attachment"].as_str())
        {
            let m = master.as_ref().unwrap();
            let dims = m.get_size().map_err(io::Error::other)?;
            response = json!({"ok":true,"exit":exit,"cols":dims.cols,"rows":dims.rows,"protocol":2,"checkpoint":reconnect(&path).is_ok_and(|t|t.generation == generation && t.owner_pid == std::process::id() && Some(t.shell_pid) == child.as_ref().and_then(|c|c.process_id())),"durable_executable":durable_executable});
            match op {
                "status" => (),
                "read" => {
                    let bytes: Vec<_> = buffer.lock().unwrap().drain(..).collect();
                    response["bytes"] = json!(bytes);
                }
                "write" => {
                    let data = serde_json::from_value::<Vec<u8>>(request["body"].clone());
                    if !data.is_ok_and(|v| {
                        v.len() <= 8192 && input.as_ref().unwrap().try_send(v).is_ok()
                    }) {
                        response["ok"] = json!(false);
                    }
                }
                "resize" => {
                    let body = &request["body"];
                    let result = size(
                        body["cols"].as_u64().unwrap_or(0) as u16,
                        body["rows"].as_u64().unwrap_or(0) as u16,
                    )
                    .and_then(|s| m.resize(s).map_err(err));
                    response["ok"] = json!(result.is_ok());
                }
                "detach" => {
                    attached = None;
                    deadline = Instant::now() + Duration::from_secs(lease);
                }
                "stop" => stop = true,
                _ => response["ok"] = json!(false),
            }
        }
        if socket.write_all(&serde_json::to_vec(&response)?).is_ok() {
            let _ = socket.shutdown(Shutdown::Write);
        }
        if stop {
            break;
        }
    }
    let _ = fs::remove_file(path.with_extension("session"));
    let _ = fs::remove_file(path);
    // Exiting the owner closes the job handle, terminating only this owner and
    // its descendants. Avoid synchronous ClosePseudoConsole teardown here.
    std::process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepted_nonblocking_listener_waits_for_complete_delayed_request() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s");
        let listener = UnixListener::bind(&path).unwrap();
        listener.set_nonblocking(true).unwrap();
        let server = thread::spawn(move || {
            let (mut socket, _) = loop {
                match listener.accept() {
                    Ok(s) => break s,
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(1))
                    }
                    Err(e) => panic!("{e}"),
                }
            };
            configure_accepted(&socket).unwrap();
            let mut bytes = Vec::new();
            socket.read_to_end(&mut bytes).unwrap();
            assert_eq!(
                serde_json::from_slice::<Value>(&bytes).unwrap(),
                json!({"op":"status"})
            );
        });
        let mut client = UnixStream::connect(&path).unwrap();
        thread::sleep(Duration::from_millis(80));
        client.write_all(br#"{"op":"#).unwrap();
        thread::sleep(Duration::from_millis(30));
        client.write_all(br#""status"}"#).unwrap();
        client.shutdown(Shutdown::Write).unwrap();
        server.join().unwrap();
    }
    #[test]
    fn completed_reset_reply_preserves_original_bytes_without_reissuing_requests() {
        let bytes = br#"{"ok":true,"bytes":[65,66,67],"exit":null}"#;
        let reset = || Err(io::Error::from(io::ErrorKind::ConnectionReset));
        assert_eq!(
            decode_reply(bytes, reset(), true).unwrap()["bytes"],
            json!([65, 66, 67])
        );
        assert!(decode_reply(bytes, reset(), false).is_err());
        for rejected in [
            b"".as_slice(),
            br#"{"ok":true"#,
            br#"{"ok":false}"#,
            br#"{"ok":true}{}"#,
            b"true",
        ] {
            assert!(decode_reply(rejected, reset(), true).is_err());
        }
        assert!(decode_reply(bytes, Err(io::Error::from(io::ErrorKind::TimedOut)), true).is_err());
        assert!(decode_reply(
            &vec![b' '; WIRE_LIMIT as usize],
            Ok(WIRE_LIMIT as usize),
            true
        )
        .is_err());
    }
    #[test]
    fn reconnect_requires_bounded_valid_checkpoint_and_exact_identity() {
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("abc");
        assert!(reconnect(&socket).is_err());
        let good = json!({"version":2,"generation":"g123456789012345678901234567890123","owner_pid":42,"shell_pid":43});
        fs::write(
            socket.with_extension("session"),
            serde_json::to_vec(&good).unwrap(),
        )
        .unwrap();
        let ticket = reconnect(&socket).unwrap();
        let mut response =
            json!({"protocol":2,"generation":ticket.generation,"owner_pid":42,"pid":43});
        assert!(identity(&response, &ticket));
        response["pid"] = json!(44);
        assert!(!identity(&response, &ticket));
        for value in [
            json!({}),
            json!({"version":1}),
            json!({"version":2,"generation":"short","owner_pid":42,"shell_pid":43}),
        ] {
            fs::write(
                socket.with_extension("session"),
                serde_json::to_vec(&value).unwrap(),
            )
            .unwrap();
            assert!(reconnect(&socket).is_err());
        }
    }
    #[test]
    fn installed_paths_keep_updates_together_but_other_installations_apart() {
        assert_eq!(
            installation_key(Path::new(r"C:\Apps\Jajan\jajan.exe")),
            installation_key(Path::new(r"\\?\c:\apps\jajan\JAJAN.EXE"))
        );
        assert_ne!(
            installation_key(Path::new(r"C:\Apps\Jajan\jajan.exe")),
            installation_key(Path::new(r"C:\Apps\Other\jajan.exe"))
        );
        assert_ne!(
            installation_key(Path::new(r"C:\Apps\Jajan\jajan.exe")),
            installation_key(Path::new(r"C:\Apps\Jajan\other.exe"))
        );
    }
    #[test]
    fn update_waits_for_removed_sessions_and_pending_closes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s");
        let listener = UnixListener::bind(&path).unwrap();
        let server = thread::spawn(move || loop {
            let (mut stream, _) = listener.accept().unwrap();
            let mut bytes = Vec::new();
            stream.read_to_end(&mut bytes).unwrap();
            let request: Value = serde_json::from_slice(&bytes).unwrap();
            stream
                .write_all(
                    b"{\"ok\":true,\"protocol\":2,\"checkpoint\":true,\"durable_executable\":true,\"exit\":null}",
                )
                .unwrap();
            if request["op"] == "stop" {
                break;
            }
        });
        let link = Link {
            path,
            attachment: "test-only".into(),
            active: Arc::new(AtomicBool::new(true)),
        };
        let child = OwnedChild::new(Box::new(RemoteChild {
            link: link.clone(),
            pid: std::process::id(),
        }));
        let state = Terminals::default();
        state
            .records
            .restore(vec![serde_json::from_value(
                json!({"id":"terminal-1","bounds":{"x":0.,"y":0.,"width":720.,"height":460.}}),
            )
            .unwrap()])
            .unwrap();
        let supervisor = state.workers.enter();
        let reader = state.workers.enter();
        let (sender, _events) = mpsc::channel();
        state.sessions.lock().unwrap().insert(
            "terminal-1".into(),
            Slot::Running {
                token: 1,
                control: Control {
                    sender,
                    cancelled: Arc::new(AtomicBool::new(false)),
                },
                session: Arc::new(Mutex::new(Session {
                    master: Box::new(RemoteMaster(link)),
                    writer: Box::new(io::sink()),
                    child,
                    program: "shell".into(),
                    observed_command: false,
                    prompt: welcome::PromptState::new(),
                })),
            },
        );
        assert!(state
            .detached_ready_for_update(&state.sessions.lock().unwrap())
            .unwrap());
        let old_cleanup = state.workers.enter();
        assert!(!state
            .detached_ready_for_update(&state.sessions.lock().unwrap())
            .unwrap());
        drop(old_cleanup);
        state.closing.lock().unwrap().insert("terminal-2".into());
        assert!(!state
            .detached_ready_for_update(&state.sessions.lock().unwrap())
            .unwrap());
        state.closing.lock().unwrap().clear();
        let saving = state.close_saves.enter();
        assert!(!state
            .detached_ready_for_update(&state.sessions.lock().unwrap())
            .unwrap());
        drop(saving);
        assert!(state
            .detached_ready_for_update(&state.sessions.lock().unwrap())
            .unwrap());
        drop(reader);
        assert!(!state
            .detached_ready_for_update(&state.sessions.lock().unwrap())
            .unwrap());
        drop(supervisor);
        drop(state);
        server.join().unwrap();
    }
    #[test]
    fn update_requires_a_live_compatible_owner_outside_the_gui_binary() {
        for (response, expected) in [
            (
                json!({"ok":true,"protocol":2,"checkpoint":true,"durable_executable":true,"exit":null}),
                true,
            ),
            (
                json!({"ok":true,"protocol":2,"checkpoint":true,"durable_executable":false,"exit":null}),
                false,
            ),
            (
                json!({"ok":true,"protocol":3,"checkpoint":true,"durable_executable":true,"exit":null}),
                false,
            ),
            (
                json!({"ok":true,"protocol":2,"checkpoint":true,"durable_executable":true,"exit":0}),
                false,
            ),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("s");
            let listener = UnixListener::bind(&path).unwrap();
            let server = thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                let mut bytes = Vec::new();
                stream.read_to_end(&mut bytes).unwrap();
                let request: Value = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(request["op"], "status");
                stream
                    .write_all(&serde_json::to_vec(&response).unwrap())
                    .unwrap();
            });
            let link = Link {
                path,
                attachment: "test-only".into(),
                active: Arc::new(AtomicBool::new(true)),
            };
            assert_eq!(link.survives_update(), expected);
            server.join().unwrap();
        }
    }
    fn supervisor_requests(quit: bool, exit: Value) -> Vec<String> {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s");
        let listener = UnixListener::bind(&path).unwrap();
        let server = thread::spawn(move || {
            let mut operations = Vec::new();
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().unwrap();
                let mut bytes = Vec::new();
                stream.read_to_end(&mut bytes).unwrap();
                let request: Value = serde_json::from_slice(&bytes).unwrap();
                operations.push(request["op"].as_str().unwrap().to_owned());
                stream
                    .write_all(&serde_json::to_vec(&json!({"ok":true,"exit":exit})).unwrap())
                    .unwrap();
            }
            operations
        });
        let link = Link {
            path,
            attachment: "test-only".into(),
            active: Arc::new(AtomicBool::new(true)),
        };
        let active = link.active.clone();
        let child = OwnedChild::new(Box::new(RemoteChild {
            link,
            pid: std::process::id(),
        }));
        let (tx, rx) = mpsc::channel();
        let (done, saved) = mpsc::channel();
        drop(done);
        tx.send(if quit {
            Event::Quit {
                saved,
                until: Instant::now(),
            }
        } else {
            Event::Stop
        })
        .unwrap();
        lifecycle::supervise(child, rx, |_, _| panic!("quit is not a natural shell exit"));
        assert!(!active.load(Ordering::Acquire));
        server.join().unwrap()
    }
    #[test]
    fn normal_quit_supervisor_detaches_without_stopping_remote_shell() {
        for exit in [Value::Null, json!(7)] {
            assert_eq!(supervisor_requests(true, exit), ["status", "detach"]);
        }
    }
    #[test]
    fn observed_shell_exit_keeps_control_for_explicit_owned_job_cleanup() {
        assert_eq!(supervisor_requests(false, json!(7)), ["status", "stop"]);
    }
    #[test]
    fn disconnected_output_has_a_bounded_tail() {
        let mut b = VecDeque::new();
        tail(&mut b, &vec![b'a'; LIMIT]);
        tail(&mut b, b"last");
        assert_eq!(b.len(), LIMIT);
        assert_eq!(b.iter().rev().take(4).copied().collect::<Vec<_>>(), b"tsal");
    }
}
