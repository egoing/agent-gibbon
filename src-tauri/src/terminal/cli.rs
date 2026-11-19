//! CLI argv stays in a private file; only backend-generated paths enter the shell.
use super::*;
use std::fs;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Execution {
    pub terminal: String,
    pub argv: Vec<String>,
    pub request_id: Option<String>,
}
struct Record {
    execution: Execution,
    file: tempfile::NamedTempFile,
    dispatched: bool,
    failed: bool,
}
#[derive(Default)]
pub struct Requests(Mutex<BTreeMap<String, Record>>);

impl Execution {
    pub fn validate(&self) -> Result<(), String> {
        if !valid_id(&self.terminal)
            || self.argv.is_empty()
            || self.argv[0].is_empty()
            || self.argv.iter().any(|v| v.contains('\0'))
            || self
                .request_id
                .as_ref()
                .is_some_and(|v| v.is_empty() || v.len() > 128)
        {
            return Err("터미널·프로그램·요청 ID를 확인하세요".into());
        }
        Ok(())
    }
}
pub fn parse(args: &[String]) -> Result<Execution, String> {
    let terminal = args.first().ok_or("터미널 ID가 필요합니다")?.clone();
    let mut rest = &args[1..];
    let request_id = if rest.first().is_some_and(|s| s == "--request-id") {
        let id = rest.get(1).ok_or("요청 ID가 필요합니다")?.clone();
        rest = &rest[2..];
        Some(id)
    } else {
        None
    };
    let argv = match rest {
        [separator, program, tail @ ..] if separator == "--" => {
            let mut argv = vec![program.clone()];
            argv.extend_from_slice(tail);
            argv
        }
        [mode, script] if mode == "--shell" => vec!["/bin/sh".into(), "-c".into(), script.clone()],
        [choice] => match choice.as_str() {
            "git" => vec![
                "git".into(),
                "log".into(),
                "--oneline".into(),
                "--decorate".into(),
                "-20".into(),
            ],
            "htop" | "codex" | "claude" => vec![choice.clone()],
            "shell" => vec![std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into())],
            _ => return Err("프로그램과 인자는 -- 뒤에 지정하세요".into()),
        },
        _ => return Err("run ID [--request-id ID] -- PROGRAM ARGS 또는 --shell SCRIPT".into()),
    };
    let execution = Execution {
        terminal,
        argv,
        request_id,
    };
    execution.validate()?;
    Ok(execution)
}
#[cfg(not(unix))]
fn shell_line(_: &Path) -> Result<String, String> {
    Err("구조화 실행은 현재 Unix 셸에서 지원합니다".into())
}
#[cfg(unix)]
fn shell_line(file: &Path) -> Result<String, String> {
    Ok(format!(
        "{} --terminal-exec {}",
        welcome::quote(&std::env::current_exe().map_err(err)?.to_string_lossy()),
        welcome::quote(&file.to_string_lossy())
    ))
}
pub fn helper(path: &str) -> Result<i32, String> {
    #[cfg(not(unix))]
    let _ = path;
    #[cfg(unix)]
    {
        let path = Path::new(path);
        use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
        let mut file = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)
            .map_err(err)?;
        let metadata = file.metadata().map_err(err)?;
        if !metadata.is_file()
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.mode() & 0o077 != 0
        {
            return Err("안전하지 않은 실행 요청 파일입니다".into());
        }
        use std::io::Read;
        let mut text = String::new();
        file.read_to_string(&mut text).map_err(err)?;
        let execution: Execution = serde_json::from_str(&text).map_err(err)?;
        execution.validate()?;
        let code = match std::process::Command::new(&execution.argv[0])
            .args(&execution.argv[1..])
            .status()
        {
            Ok(status) => status.code().unwrap_or(1),
            Err(error) => {
                eprintln!("프로그램 실행 실패: {error}");
                127
            }
        };
        fs::write(path.with_extension("status"), code.to_string()).map_err(err)?;
        Ok(code)
    }
    #[cfg(not(unix))]
    Err("지원하지 않는 운영체제입니다".into())
}
fn response(record: &Record, token: &str, duplicate: bool) -> Value {
    let exit = fs::read_to_string(record.file.path().with_extension("status"))
        .ok()
        .and_then(|s| s.parse::<i32>().ok());
    json!({"ok":true,"request_id":record.execution.request_id,"token":token,"id":record.execution.terminal,"duplicate":duplicate,"status":if record.failed{"failed"}else if exit.is_some(){"completed"}else if record.dispatched{"dispatched"}else{"queued"},"exit_code":exit,"instance":crate::instance::current().id})
}
pub fn line(app: &tauri::AppHandle, id: &str, token: &str) -> Result<String, String> {
    let state = app.state::<Requests>();
    let mut records = state.0.lock().map_err(err)?;
    let record = records.get_mut(token).ok_or("실행 요청이 없습니다")?;
    if record.execution.terminal != id || record.dispatched {
        return Err("실행 요청이 이미 전달됐거나 대상이 다릅니다".into());
    }
    let line = shell_line(record.file.path())?;
    record.dispatched = true;
    Ok(line)
}
pub fn run(app: &tauri::AppHandle, execution: &Execution) -> Result<Value, String> {
    execution.validate()?;
    let state = app.state::<Requests>();
    let mut records = state.0.lock().map_err(err)?;
    if let Some(id) = &execution.request_id {
        if let Some((token, record)) = records
            .iter()
            .find(|(_, r)| r.execution.request_id.as_ref() == Some(id))
        {
            if &record.execution != execution {
                return Err("요청 ID와 실행 내용이 충돌합니다".into());
            }
            return Ok(response(record, token, true));
        }
    }
    let window = app
        .get_webview_window(&execution.terminal)
        .ok_or("열린 터미널이 필요합니다")?;
    let terminals = app.state::<Terminals>();
    let sessions = terminals.sessions.lock().map_err(err)?;
    let mut active = match sessions.get(&execution.terminal) {
        Some(Slot::Running { session, .. }) => Some(session.lock().map_err(err)?),
        Some(_) => return Err("터미널이 시작 중이거나 종료되었습니다".into()),
        None => None,
    };
    if active
        .as_mut()
        .is_some_and(|session| welcome::busy(session))
    {
        return Err("터미널에서 작업 또는 입력이 진행 중입니다".into());
    }
    if records
        .values()
        .any(|r| r.execution.terminal == execution.terminal && !r.dispatched)
    {
        return Err("터미널 시작 요청이 대기 중입니다".into());
    }
    let mut file = tempfile::Builder::new()
        .prefix("jajan-exec-")
        .tempfile()
        .map_err(err)?;
    serde_json::to_writer(&mut file, execution).map_err(err)?;
    file.flush().map_err(err)?;
    let token = file
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let line = shell_line(file.path())?;
    let mut record = Record {
        execution: execution.clone(),
        file,
        dispatched: false,
        failed: false,
    };
    if let Some(session) = active.as_mut() {
        session.prompt.input(format!("{line}\r").as_bytes());
        record.dispatched = true;
        records.insert(token.clone(), record);
        session
            .writer
            .write_all(format!("{line}\r").as_bytes())
            .map_err(err)?;
        session.writer.flush().map_err(err)?;
    } else {
        records.insert(token.clone(), record);
        if let Err(error) = window.eval(format!(
            "void start('shell', null, undefined, {})",
            serde_json::to_string(&token).map_err(err)?
        )) {
            records.remove(&token);
            return Err(err(error));
        }
    }
    Ok(response(records.get(&token).unwrap(), &token, false))
}
impl Drop for Record {
    fn drop(&mut self) {
        let _ = fs::remove_file(self.file.path().with_extension("status"));
    }
}

pub fn status(app: &tauri::AppHandle, token: &str) -> Result<Value, String> {
    let state = app.state::<Requests>();
    let records = state.0.lock().map_err(err)?;
    let record = records
        .get(token)
        .ok_or("실행 요청을 찾을 수 없습니다. 앱 재시작 여부를 확인하세요")?;
    Ok(response(record, token, false))
}
pub fn failed(app: &tauri::AppHandle, token: &str) {
    if let Ok(mut records) = app.state::<Requests>().0.lock() {
        if let Some(record) = records.get_mut(token) {
            record.failed = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_optional_request_id_and_literal_arguments() {
        let args = [
            "terminal-123",
            "--request-id",
            "one",
            "--",
            "printf",
            "%s",
            "a b",
            "$(touch nope)",
            ";",
            "'",
        ];
        let execution = parse(&args.map(String::from)).unwrap();
        assert_eq!(execution.argv, &args[4..]);
        assert_eq!(execution.request_id.as_deref(), Some("one"));
        assert!(
            parse(&["terminal-123", "--", "du", "-sh", "."].map(String::from))
                .unwrap()
                .request_id
                .is_none()
        );
        assert!(parse(&["web-1", "--", "du"].map(String::from)).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn helper_passes_argv_without_shell_evaluation_and_records_status() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        let execution = Execution {
            terminal: "terminal-123".into(),
            request_id: None,
            argv: vec![
                "/bin/sh".into(),
                "-c".into(),
                r#"test "$1" = '$(exit 99); a b'"#.into(),
                "test".into(),
                "$(exit 99); a b".into(),
            ],
        };
        serde_json::to_writer(&mut file, &execution).unwrap();
        assert_eq!(helper(file.path().to_str().unwrap()).unwrap(), 0);
        assert_eq!(
            fs::read_to_string(file.path().with_extension("status")).unwrap(),
            "0"
        );
        fs::remove_file(file.path().with_extension("status")).unwrap();
    }
}
