#![cfg(all(windows, debug_assertions))]
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    net::Shutdown,
    os::windows::process::CommandExt,
    path::PathBuf,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use uds_windows::UnixStream;

fn test_binary() -> PathBuf {
    std::env::var_os("JAJAN_PTY_TEST_BINARY")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_jajan")))
}
fn owner_program(instance: &str) -> PathBuf {
    if let Some(owner) = std::env::var_os("JAJAN_PTY_TEST_OWNER_BINARY") {
        let owner = PathBuf::from(owner);
        assert_eq!(
            Sha256::digest(std::fs::read(&owner).unwrap()),
            Sha256::digest(std::fs::read(test_binary()).unwrap())
        );
        return owner;
    }
    let result = Command::new(test_binary())
        .arg("--poc-owner-info")
        .env("JAJAN_INSTANCE", instance)
        .env("JAJAN_POC_WINDOWS_PTY", "1")
        .output()
        .unwrap();
    assert!(result.status.success());
    let value: Value = serde_json::from_slice(&result.stdout).unwrap();
    let owner = PathBuf::from(value["owner"].as_str().unwrap());
    assert_ne!(owner.parent(), test_binary().parent());
    assert_eq!(
        Sha256::digest(std::fs::read(&owner).unwrap()),
        Sha256::digest(std::fs::read(test_binary()).unwrap())
    );
    owner
}
fn instance() -> String {
    if let Ok(instance) = std::env::var("JAJAN_PTY_TEST_INSTANCE") {
        return instance;
    }
    let root =
        std::fs::canonicalize(PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap()).unwrap();
    let text = root.to_string_lossy();
    let root = text.strip_prefix(r"\\?\").unwrap_or(&text);
    format!(
        "dev-{}",
        &format!("{:x}", Sha256::digest(root.as_bytes()))[..24]
    )
}

#[test]
#[ignore = "explicit isolated GUI fixture; holds owner until stop marker or 900 seconds"]
fn seed_isolated_gui_session() {
    let generated = format!(
        "terminal-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let id = generated.as_str();
    let instance = instance();
    let hash = format!("{:x}", Sha256::digest(id.as_bytes()));
    let path = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap())
        .join("Jajan")
        .join(&instance)
        .join("p")
        .join(&hash[..12]);
    let mut command = Command::new(owner_program(&instance));
    command
        .args(["--pty-owner", id])
        .env("JAJAN_INSTANCE", &instance)
        .env("JAJAN_POC_WINDOWS_PTY", "1")
        .env("JAJAN_POC_DETACH_SECONDS", "900")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(0x08000208);
    let owner = Owner(command.spawn().unwrap());
    let deadline = Instant::now() + Duration::from_secs(5);
    while UnixStream::connect(&path).is_err() {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(25));
    }
    let shell = PathBuf::from(std::env::var_os("SystemRoot").unwrap())
        .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let result = rpc(
        &path,
        json!({"op":"attach","create":true,"pid":std::process::id(),"shell":shell,"cwd":env!("CARGO_MANIFEST_DIR"),"cols":80,"rows":24,"prompt":"gui-probe","line":"$global:jajanProbe=731; while($true){ [Console]::WriteLine(('GUI_PROBE_'+$PID+'_'+$global:jajanProbe)); Start-Sleep 2 }"}),
    );
    assert_eq!(result["ok"], true);
    let key = result["attachment"].as_str().unwrap();
    let pid = result["pid"].as_u64().unwrap();
    output(&path, key, &format!("GUI_PROBE_{pid}_731"));
    call(&path, key, "detach", Value::Null);
    let root = PathBuf::from(std::env::var_os("APPDATA").unwrap())
        .join(std::env::var("JAJAN_PTY_TEST_APP_ID").unwrap_or_else(|_| "app.jajan".into()))
        .join("notes")
        .join(&instance)
        .join("docs")
        .join(".jajan-terminals");
    std::fs::create_dir_all(&root).unwrap();
    let fixture = root.join(format!("{id}.json"));
    assert!(!fixture.exists(), "fixture already exists");
    std::fs::write(fixture,serde_json::to_vec(&json!({"id":id,"kind":"terminal","physical_position":true,"bounds":{"x":200,"y":200,"width":720,"height":460},"last_path":env!("CARGO_MANIFEST_DIR"),"recent_paths":[],"path_updated":0,"updated_at":0,"title":"Windows PTY isolated probe","auto_restore":false})).unwrap()).unwrap();
    println!(
        "GUI fixture {id} owner={} shell={pid} instance={instance}",
        owner.0.id()
    );
    let stop = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.tmp/windows-pty-gui-stop");
    let until = Instant::now() + Duration::from_secs(900);
    while !stop.exists() && Instant::now() < until {
        thread::sleep(Duration::from_millis(200));
    }
    drop(owner);
}

struct Owner(Child);
impl Drop for Owner {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn rpc(path: &PathBuf, request: Value) -> Value {
    let mut socket = UnixStream::connect(path).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    socket
        .write_all(&serde_json::to_vec(&request).unwrap())
        .unwrap();
    socket.shutdown(Shutdown::Write).unwrap();
    let mut bytes = Vec::new();
    socket.read_to_end(&mut bytes).unwrap();
    serde_json::from_slice(&bytes).unwrap()
}
fn call(path: &PathBuf, key: &str, op: &str, body: Value) -> Value {
    let value = rpc(path, json!({"attachment":key,"op":op,"body":body}));
    assert_eq!(value["ok"], true, "{op}");
    value
}
fn output(path: &PathBuf, key: &str, needle: &str) -> String {
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut bytes = Vec::new();
    loop {
        let v = call(path, key, "read", Value::Null);
        bytes.extend(serde_json::from_value::<Vec<u8>>(v["bytes"].clone()).unwrap());
        let text = String::from_utf8_lossy(&bytes).into_owned();
        if text.contains(needle) {
            return text;
        }
        assert!(
            Instant::now() < deadline,
            "missing fixed test marker {needle}"
        );
        thread::sleep(Duration::from_millis(50));
    }
}
fn write(path: &PathBuf, key: &str, text: &str) {
    call(path, key, "write", json!(text.as_bytes()));
}
fn live(pid: u32) -> bool {
    use windows_sys::Win32::{Foundation::*, System::Threading::*};
    unsafe {
        let h = OpenProcess(0x00100000, 0, pid);
        if h.is_null() {
            return false;
        }
        let alive = WaitForSingleObject(h, 0) == WAIT_TIMEOUT;
        CloseHandle(h);
        alive
    }
}

#[test]
fn real_conpty_survives_detach_client_death_flood_and_stops_owned_job() {
    let id = "terminal-90120261121";
    let instance = instance();
    let hash = format!("{:x}", Sha256::digest(id.as_bytes()));
    let path = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap())
        .join("Jajan")
        .join(&instance)
        .join("p")
        .join(&hash[..12]);
    let mut command = Command::new(owner_program(&instance));
    command
        .args(["--pty-owner", id])
        .env("JAJAN_INSTANCE", &instance)
        .env("JAJAN_POC_WINDOWS_PTY", "1")
        .env("JAJAN_POC_DETACH_SECONDS", "30")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(0x08000000);
    let mut owner = Owner(command.spawn().unwrap());
    let deadline = Instant::now() + Duration::from_secs(5);
    while UnixStream::connect(&path).is_err() {
        assert!(owner.0.try_wait().unwrap().is_none(), "owner exited");
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(25));
    }
    let shell = PathBuf::from(std::env::var_os("SystemRoot").unwrap())
        .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let generation = std::cell::RefCell::new(serde_json::Value::Null);
    let attach = |pid, create, line: &str| json!({"generation":*generation.borrow(),"op":"attach","pid":pid,"create":create,"shell":shell,"cwd":env!("CARGO_MANIFEST_DIR"),"cols":80,"rows":24,"prompt":"fixed-probe","line":line});
    let a = rpc(
        &path,
        attach(
            std::process::id(),
            true,
            "$global:jajanProbe=731; [Console]::WriteLine('CREATED_' + $PID)",
        ),
    );
    assert_eq!(a["ok"], true);
    let pid = a["pid"].as_u64().unwrap() as u32;
    *generation.borrow_mut() = a["generation"].clone();
    let key = a["attachment"].as_str().unwrap();
    let status = call(&path, key, "status", Value::Null);
    assert_eq!(status["protocol"], 2);
    assert_eq!(status["durable_executable"], true);
    output(&path, key, &format!("CREATED_{pid}"));
    assert_eq!(
        rpc(&path, attach(std::process::id(), false, ""))["ok"],
        false,
        "duplicate attachment"
    );
    assert_eq!(
        rpc(&path, json!({"op":"stop","attachment":"wrong"}))["ok"],
        false
    );
    let duplicate = Owner(command.spawn().unwrap());
    thread::sleep(Duration::from_millis(250));
    assert!(live(pid));
    drop(duplicate);
    write(&path,key,"1..30000 | ForEach-Object { [Console]::WriteLine('xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx') }; [Console]::WriteLine('FLOOD_DONE')\r");
    call(&path, key, "detach", Value::Null);
    assert!(live(pid));
    thread::sleep(Duration::from_secs(3));
    let b = rpc(&path, attach(std::process::id(), false, ""));
    assert_eq!(b["pid"], json!(pid));
    let key = b["attachment"].as_str().unwrap();
    output(&path, key, "FLOOD_DONE");
    write(
        &path,
        key,
        "[Console]::WriteLine(('STATE_' + $PID + '_' + $global:jajanProbe))\r",
    );
    output(&path, key, &format!("STATE_{pid}_731"));
    call(&path, key, "resize", json!({"cols":101,"rows":33}));
    let status = call(&path, key, "status", Value::Null);
    assert_eq!(status["cols"], 101);
    assert_eq!(status["rows"], 33);
    write(&path,key,"[Console]::WriteLine(('SIZE_' + [Console]::WindowWidth + '_' + [Console]::WindowHeight))\r");
    output(&path, key, "SIZE_101_33");
    call(&path, key, "detach", Value::Null);
    let mut client = Command::new(&shell)
        .args(["-NoLogo", "-NoProfile", "-Command", "Start-Sleep 30"])
        .creation_flags(0x08000000)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let c = rpc(&path, attach(client.id(), false, ""));
    assert_eq!(c["pid"], json!(pid));
    client.kill().unwrap();
    client.wait().unwrap();
    thread::sleep(Duration::from_millis(100));
    let d = rpc(&path, attach(std::process::id(), false, ""));
    assert_eq!(d["pid"], json!(pid));
    let key = d["attachment"].as_str().unwrap();
    write(&path,key,"[Console]::WriteLine(('ALIVE_' + $PID + '_' + $global:jajanProbe)); $job=Start-Process -FilePath $PSHOME/powershell.exe -ArgumentList '-NoProfile','-Command','Start-Sleep 60' -WindowStyle Hidden -PassThru; [Console]::WriteLine(('CHILD_' + $job.Id))\r");
    let text = output(&path, key, "CHILD_");
    // Echo can contain the prefix too, so continue until a concrete child PID.
    let mut all = text;
    let deadline = Instant::now() + Duration::from_secs(10);
    let child_pid = loop {
        if let Some(value) = all.lines().find_map(|line| {
            line.trim()
                .strip_prefix("CHILD_")
                .and_then(|s| s.parse::<u32>().ok())
        }) {
            break value;
        }
        assert!(Instant::now() < deadline);
        let v = call(&path, key, "read", Value::Null);
        all.push_str(&String::from_utf8_lossy(
            &serde_json::from_value::<Vec<u8>>(v["bytes"].clone()).unwrap(),
        ));
        thread::sleep(Duration::from_millis(50));
    };
    assert!(all.contains(&format!("ALIVE_{pid}_731")));
    assert!(live(child_pid));
    call(&path, key, "stop", Value::Null);
    owner.0.wait().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while live(pid) || live(child_pid) {
        assert!(Instant::now() < deadline, "owned job survived");
        thread::sleep(Duration::from_millis(30));
    }
    println!("PASS owner={} shell={pid} descendant={child_pid}: detach, 1.9MiB disconnected output, same variable/PID, resize, duplicate owner/client, unauthorized token, forced client death, owned job cleanup",owner.0.id());
    command.env("JAJAN_POC_DETACH_SECONDS", "1");
    let mut fresh = Owner(command.spawn().unwrap());
    let deadline = Instant::now() + Duration::from_secs(5);
    while UnixStream::connect(&path).is_err() {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(25));
    }
    assert_eq!(
        rpc(&path, attach(std::process::id(), false, ""))["ok"],
        false
    );
    assert_eq!(
        rpc(
            &path,
            json!({"op":"available","generation":*generation.borrow()})
        )["ok"],
        false
    );
    let created = rpc(&path, attach(std::process::id(), true, ""));
    assert_eq!(created["ok"], true);
    let leased_pid = created["pid"].as_u64().unwrap() as u32;
    call(
        &path,
        created["attachment"].as_str().unwrap(),
        "detach",
        Value::Null,
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    while fresh.0.try_wait().unwrap().is_none() || live(leased_pid) {
        assert!(Instant::now() < deadline, "lease cleanup timed out");
        thread::sleep(Duration::from_millis(30));
    }
    println!(
        "PASS attach-only never creates a shell; detached lease expiry cleans owner and shell"
    );
}
