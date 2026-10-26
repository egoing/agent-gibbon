//! Native operations on this app's HWNDs, called only on Tauri's main thread.
use tauri::WebviewWindow;

const GWL_EXSTYLE: i32 = -20;
const WS_EX_LAYERED: i32 = 0x0008_0000;
const WS_EX_NOACTIVATE: i32 = 0x0800_0000;
const LWA_ALPHA: u32 = 2;

#[link(name = "user32")]
extern "system" {
    fn GetWindowLongW(window: isize, index: i32) -> i32;
    fn SetWindowLongW(window: isize, index: i32, value: i32) -> i32;
    fn SetLayeredWindowAttributes(window: isize, key: u32, alpha: u8, flags: u32) -> i32;
    fn GetLayeredWindowAttributes(
        window: isize,
        key: *mut u32,
        alpha: *mut u8,
        flags: *mut u32,
    ) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn SetLastError(error: u32);
}

fn extended_style(handle: isize) -> Result<i32, String> {
    // GWL_EXSTYLE is a 32-bit value on both x86 and x64. Zero is valid.
    unsafe {
        SetLastError(0);
        let style = GetWindowLongW(handle, GWL_EXSTYLE);
        let error = std::io::Error::last_os_error();
        if style == 0 && error.raw_os_error() != Some(0) {
            return Err(format!("창 스타일 조회 실패: {error}"));
        }
        Ok(style)
    }
}

pub fn show_without_focus(window: &WebviewWindow) -> Result<(), String> {
    let handle = window.hwnd().map_err(|e| e.to_string())?.0 as isize;
    let focusable = extended_style(handle)? & WS_EX_NOACTIVATE == 0;
    // Keep Tao's visibility state in sync so repeated hide/show keeps working.
    // Disabling activation during show also protects against a future change to
    // Tao's focus:false/SW_SHOWNOACTIVATE implementation. Clicks work afterwards.
    if focusable {
        window.set_focusable(false).map_err(|e| e.to_string())?;
    }
    let shown = window.show().map_err(|e| e.to_string());
    let restored = if focusable {
        window.set_focusable(true).map_err(|e| e.to_string())
    } else {
        Ok(())
    };
    shown.and(restored)
}

pub fn set_opacity(window: &WebviewWindow, alpha: f64) -> Result<(), String> {
    let handle = window.hwnd().map_err(|e| e.to_string())?.0 as isize;
    let style = extended_style(handle)?;
    let alpha = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
    // Tao can replace extended styles when toggling pin/focusability. Recheck
    // instead of caching this bit, preserving all unrelated native styles.
    unsafe {
        if style & WS_EX_LAYERED == 0 {
            SetLastError(0);
            let previous = SetWindowLongW(handle, GWL_EXSTYLE, style | WS_EX_LAYERED);
            let error = std::io::Error::last_os_error();
            if previous == 0 && error.raw_os_error() != Some(0) {
                return Err(format!("창 반투명 스타일 설정 실패: {error}"));
            }
        } else {
            let (mut key, mut current, mut flags) = (0, 0, 0);
            if GetLayeredWindowAttributes(handle, &mut key, &mut current, &mut flags) != 0
                && current == alpha
                && flags == LWA_ALPHA
            {
                return Ok(());
            }
        }
        // System redirection blends the whole WebView2 host, including controls.
        // No color key or input-transparent style: normal hit testing is retained.
        if SetLayeredWindowAttributes(handle, 0, alpha, LWA_ALPHA) == 0 {
            return Err(format!(
                "창 반투명도 설정 실패: {}",
                std::io::Error::last_os_error()
            ));
        }
    }
    Ok(())
}
