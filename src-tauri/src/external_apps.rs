//! Explicitly selected external Windows desktop windows. No embedding or server.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::BTreeMap, sync::Mutex};
use tauri::Manager;
#[cfg(windows)]
mod windows;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalRequest {
    pub operation: String,
    pub app_id: Option<String>,
    pub window: Option<String>,
    pub session: Option<String>,
    pub region: Option<[i32; 4]>,
}
impl ExternalRequest {
    pub fn validate(&self) -> Result<(), String> {
        let valid = match self.operation.as_str() {
            "list" | "windows" | "sessions" | "select" => {
                self.app_id.is_none()
                    && self.window.is_none()
                    && self.session.is_none()
                    && self.region.is_none()
            }
            "open" => {
                self.app_id
                    .as_ref()
                    .is_some_and(|s| !s.is_empty() && s.len() <= 256)
                    && self.window.is_none()
                    && self.session.is_none()
                    && self.region.is_some()
            }
            "attach" => {
                self.app_id.is_none()
                    && self
                        .window
                        .as_ref()
                        .and_then(|s| s.parse::<isize>().ok())
                        .is_some_and(|h| h > 0)
                    && self.session.is_none()
                    && self.region.is_some()
            }
            "release" | "cancel" => {
                self.app_id.is_none()
                    && self.window.is_none()
                    && self.region.is_none()
                    && self
                        .session
                        .as_ref()
                        .is_some_and(|s| !s.is_empty() && s.len() <= 128)
            }
            _ => false,
        };
        if !valid {
            return Err("외부 앱 작업의 옵션 조합이 올바르지 않습니다".into());
        }
        if let Some([x, y, w, h]) = self.region {
            if w < 160
                || h < 120
                || w > 32768
                || h > 32768
                || x.checked_add(w).is_none()
                || y.checked_add(h).is_none()
            {
                return Err("영역은 최소 160×120 물리 px이며 정수 좌표여야 합니다".into());
            }
        }
        Ok(())
    }
}
pub fn parse(args: &[String]) -> Result<ExternalRequest, String> {
    let mut r = ExternalRequest {
        operation: args.first().ok_or("app 작업이 필요합니다")?.clone(),
        app_id: None,
        window: None,
        session: None,
        region: None,
    };
    let mut i = 1;
    while i < args.len() {
        let v = args.get(i + 1).ok_or("옵션 값이 없습니다")?;
        match args[i].as_str() {
            "--app-id" if r.app_id.is_none() => r.app_id = Some(v.clone()),
            "--window" if r.window.is_none() => r.window = Some(v.clone()),
            "--session" if r.session.is_none() => r.session = Some(v.clone()),
            "--region" if r.region.is_none() => {
                let values = v
                    .split(',')
                    .map(str::parse::<i32>)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| "영역은 x,y,width,height 정수입니다")?;
                r.region = Some(values.try_into().map_err(|_| "영역은 정수 네 개입니다")?);
            }
            _ => return Err(format!("알 수 없거나 중복된 app 옵션: {}", args[i])),
        }
        i += 2;
    }
    r.validate()?;
    Ok(r)
}
#[derive(Default)]
pub struct ExternalApps {
    pub bounds: Mutex<BTreeMap<String, [i32; 4]>>,
    pending_owners: Mutex<BTreeMap<String, String>>,
    #[cfg(windows)]
    inner: std::sync::Arc<Mutex<windows::State>>,
}
#[cfg(not(windows))]
const UNSUPPORTED: &str = "외부 앱 플로팅은 Windows 데스크톱에서만 지원합니다";
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

pub fn execute(app: &tauri::AppHandle, r: &ExternalRequest) -> Result<Value, String> {
    r.validate()?;
    #[cfg(windows)]
    {
        if r.operation == "select" {
            crate::capture::trigger(app, crate::capture::SelectionMode::ExternalApp);
            return Ok(json!({"ok":true,"instance":crate::instance::current().id}));
        }
        windows::execute(app, r)
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        Err(UNSUPPORTED.into())
    }
}

pub fn create(app: &tauri::AppHandle, bounds: crate::capture::ImageNote) -> Result<(), String> {
    #[cfg(not(windows))]
    {
        let _ = (app, bounds);
        Err(UNSUPPORTED.into())
    }
    #[cfg(windows)]
    {
        let id = format!(
            "external-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(err)?
                .as_nanos()
        );
        let region = [
            bounds.x.round() as i32,
            bounds.y.round() as i32,
            bounds.width.round() as i32,
            bounds.height.round() as i32,
        ];
        create_region(app, &id, region)
    }
}
#[cfg(windows)]
pub fn create_region(app: &tauri::AppHandle, id: &str, region: [i32; 4]) -> Result<(), String> {
    ExternalRequest {
        operation: "attach".into(),
        app_id: None,
        window: Some("1".into()),
        session: None,
        region: Some(region),
    }
    .validate()?;
    app.state::<ExternalApps>()
        .bounds
        .lock()
        .map_err(err)?
        .insert(id.into(), region);
    let result = tauri::WebviewWindowBuilder::new(
        app,
        id,
        tauri::WebviewUrl::App("external-apps.html".into()),
    )
    .title(format!("{} — {id}", crate::instance::current().title()))
    .decorations(true)
    .inner_size(480.0, 460.0)
    .min_inner_size(300.0, 240.0)
    .position(0.0, 0.0)
    .visible(false)
    .focused(false)
    .build()
    .map_err(err);
    match result {
        Ok(w) => {
            w.set_position(tauri::PhysicalPosition::new(region[0], region[1]))
                .map_err(err)?;
            w.show().map_err(err)?;
            w.set_focus().map_err(err)?;
            Ok(())
        }
        Err(e) => {
            app.state::<ExternalApps>()
                .bounds
                .lock()
                .map_err(err)?
                .remove(id);
            Err(e)
        }
    }
}
#[tauri::command]
pub fn external_welcome(window: tauri::WebviewWindow) -> Result<Value, String> {
    let state = window.state::<ExternalApps>();
    let bounds = state
        .bounds
        .lock()
        .map_err(err)?
        .get(window.label())
        .copied()
        .ok_or("외부 앱 선택 창이 아닙니다")?;
    Ok(
        json!({"ok":true,"region":bounds,"instance":crate::instance::current().id,"supported":cfg!(windows)}),
    )
}
#[tauri::command]
pub async fn external_request(
    window: tauri::WebviewWindow,
    mut request: ExternalRequest,
) -> Result<Value, String> {
    if !window
        .state::<ExternalApps>()
        .bounds
        .lock()
        .map_err(err)?
        .contains_key(window.label())
    {
        return Err("외부 앱 선택 창이 아닙니다".into());
    }
    if matches!(request.operation.as_str(), "attach" | "open") {
        request.region = window
            .state::<ExternalApps>()
            .bounds
            .lock()
            .map_err(err)?
            .get(window.label())
            .copied();
    }
    let app = window.app_handle().clone();
    let owner = window.label().to_string();
    tauri::async_runtime::spawn_blocking(move || {
        if !app
            .state::<ExternalApps>()
            .bounds
            .lock()
            .map_err(err)?
            .contains_key(&owner)
        {
            return Err("선택 창이 닫혀 작업을 취소했습니다".into());
        }
        let result = execute(&app, &request)?;
        if let Some(launch) = result
            .get("launch")
            .or_else(|| result.get("session"))
            .and_then(Value::as_str)
        {
            app.state::<ExternalApps>()
                .pending_owners
                .lock()
                .map_err(err)?
                .insert(owner.clone(), launch.into());
            if !app
                .state::<ExternalApps>()
                .bounds
                .lock()
                .map_err(err)?
                .contains_key(&owner)
            {
                let _ = execute(
                    &app,
                    &ExternalRequest {
                        operation: if launch.starts_with("launch-") {
                            "cancel"
                        } else {
                            "release"
                        }
                        .into(),
                        session: Some(launch.into()),
                        app_id: None,
                        window: None,
                        region: None,
                    },
                );
            }
        }
        Ok(result)
    })
    .await
    .map_err(err)?
}
pub fn shutdown(app: &tauri::AppHandle) {
    #[cfg(windows)]
    windows::shutdown(app);
    #[cfg(not(windows))]
    let _ = app;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn region_validation_rejects_overflow_and_tiny_rects() {
        for rect in [
            [0, 0, 159, 120],
            [0, 0, 160, 119],
            [i32::MAX, 0, 640, 480],
            [0, 0, 32769, 400],
        ] {
            assert!(parse(
                &[
                    "attach",
                    "--window",
                    "42",
                    "--region",
                    &format!("{},{},{},{}", rect[0], rect[1], rect[2], rect[3])
                ]
                .map(String::from)
            )
            .is_err());
        }
        assert!(parse(
            &["attach", "--window", "42", "--region", "-1200,50,640,480"].map(String::from)
        )
        .is_ok());
    }
}

pub fn picker_closed(app: &tauri::AppHandle, label: &str) {
    if let Ok(mut bounds) = app.state::<ExternalApps>().bounds.lock() {
        bounds.remove(label);
    }
    let pending = app
        .state::<ExternalApps>()
        .pending_owners
        .lock()
        .ok()
        .and_then(|mut p| p.remove(label));
    if let Some(key) = pending {
        let app = app.clone();
        std::thread::spawn(move || {
            let _ = execute(
                &app,
                &ExternalRequest {
                    operation: if key.starts_with("launch-") {
                        "cancel"
                    } else {
                        "release"
                    }
                    .into(),
                    session: Some(key),
                    app_id: None,
                    window: None,
                    region: None,
                },
            );
        });
    }
}
pub fn release_all(app: &tauri::AppHandle) -> Result<(), String> {
    #[cfg(windows)]
    {
        windows::release_all(app)
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        Err(UNSUPPORTED.into())
    }
}

#[tauri::command]
pub fn external_complete(window: tauri::WebviewWindow) -> Result<(), String> {
    if !window
        .state::<ExternalApps>()
        .bounds
        .lock()
        .map_err(err)?
        .contains_key(window.label())
    {
        return Err("외부 앱 선택 창이 아닙니다".into());
    }
    window
        .state::<ExternalApps>()
        .pending_owners
        .lock()
        .map_err(err)?
        .remove(window.label());
    Ok(())
}

pub fn recover(app: &tauri::AppHandle) {
    #[cfg(windows)]
    {
        let app = app.clone();
        std::thread::spawn(move || {
            if let Err(e) = windows::initialize(&app) {
                eprintln!("외부 창 복원 기록 확인 실패: {e}");
            }
        });
    }
    #[cfg(not(windows))]
    let _ = app;
}
