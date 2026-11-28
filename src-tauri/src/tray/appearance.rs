//! Windows popup menus use the OS app theme, independently of the manager page.
#[cfg(windows)]
pub(super) fn foreground() -> [u8; 3] {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};
    #[repr(C)]
    struct HighContrast {
        size: u32,
        flags: u32,
        scheme: *mut u16,
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn SystemParametersInfoW(
            action: u32,
            param: u32,
            value: *mut HighContrast,
            flags: u32,
        ) -> i32;
        fn GetSysColor(index: i32) -> u32;
    }
    let mut contrast = HighContrast {
        size: std::mem::size_of::<HighContrast>() as u32,
        flags: 0,
        scheme: std::ptr::null_mut(),
    };
    // Read only: respect the user's contrast menu text color before app theme.
    if unsafe { SystemParametersInfoW(0x0042, contrast.size, &mut contrast, 0) } != 0
        && contrast.flags & 1 != 0
    {
        let color = unsafe { GetSysColor(7) }; // COLOR_MENUTEXT, COLORREF = 0x00bbggrr
        return [color as u8, (color >> 8) as u8, (color >> 16) as u8];
    }
    let dark = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize")
        .and_then(|key| key.get_value::<u32, _>("AppsUseLightTheme"))
        .is_ok_and(|light| light == 0);
    if dark {
        [240, 240, 240]
    } else {
        [32, 32, 32]
    }
}

pub(super) fn pixels(rgba: &mut [u8], visible: bool, foreground: Option<[u8; 3]>) {
    for pixel in rgba.as_chunks_mut::<4>().0 {
        if let Some(color) = foreground {
            pixel[..3].copy_from_slice(&color);
        }
        if !visible {
            pixel[3] = (u16::from(pixel[3]) * 45 / 100) as u8;
        }
    }
}

pub(super) fn title(title: &str, visible: bool) -> String {
    if cfg!(windows) {
        format!(
            "{title}\t{}",
            crate::language::tr(if visible { "표시 중" } else { "숨김" })
        )
    } else {
        title.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_tint_preserves_antialiasing_and_distinguishes_hidden_icons() {
        for color in [[240, 240, 240], [32, 32, 32], [255, 255, 0]] {
            let mut visible = [0, 0, 0, 255, 0, 0, 0, 128, 0, 0, 0, 0];
            pixels(&mut visible, true, Some(color));
            assert_eq!(&visible[..3], &color);
            assert_eq!(visible[3], 255);
            assert_eq!(visible[7], 128);
            assert_eq!(visible[11], 0);
            pixels(&mut visible, false, Some(color));
            assert_eq!(visible[3], 114);
            assert_eq!(visible[7], 57);
            assert_eq!(visible[11], 0);
        }
    }

    #[test]
    fn mac_template_rgb_is_preserved_and_windows_state_is_explicit() {
        let mut rgba = [12, 34, 56, 255];
        pixels(&mut rgba, false, None);
        assert_eq!(rgba, [12, 34, 56, 114]);
        if cfg!(windows) {
            assert_eq!(title("메모", true), "메모\t표시 중");
            assert_eq!(title("메모", false), "메모\t숨김");
        } else {
            assert_eq!(title("메모", false), "메모");
        }
    }
}
