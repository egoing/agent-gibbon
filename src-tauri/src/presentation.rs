//! Display-only labels. Never feed these strings back into IDs or key registration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Note,
    Capture,
    Terminal,
}
impl Kind {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Terminal)
    }
    fn default_title(self) -> &'static str {
        match self {
            Self::Note => "메모",
            Self::Capture => "화면 캡처",
            Self::Terminal => "터미널",
        }
    }
}
pub fn list_title(raw: &str, id: &str, kind: Kind) -> String {
    let clean: String = raw.chars().filter(|c| !c.is_control()).collect();
    let mut title = clean.trim();
    // Strip only known presentation wrappers and this entry's exact legacy ID.
    // Arbitrary parentheses and identifiers inside a user's title stay intact.
    for prefix in ["[텍스트]", "[이미지]", "[터미널]", "[에이전트]"] {
        if let Some(rest) = title.strip_prefix(prefix) {
            title = rest.trim();
        }
    }
    let suffix = format!(" ({id})");
    title = title.strip_suffix(&suffix).unwrap_or(title).trim();
    if title.is_empty() || title == id {
        title = kind.default_title();
    }
    title.chars().take(30).collect()
}
pub fn shortcut(raw: &str) -> String {
    shortcut_for(raw, cfg!(target_os = "macos"))
}
fn shortcut_for(raw: &str, mac: bool) -> String {
    // Keep availability explanations verbatim; transform only shortcut tokens.
    raw.split_once(' ').map_or_else(
        || chord(raw, mac),
        |(keys, explanation)| format!("{} {explanation}", chord(keys, mac)),
    )
}
fn chord(raw: &str, mac: bool) -> String {
    raw.split('+')
        .map(|part| match part {
            "Ctrl" | "Control" => "Ctrl",
            "Alt" | "Option" => {
                if mac {
                    "Opt"
                } else {
                    "Alt"
                }
            }
            "Shift" => "⇧",
            "Cmd" | "Command" | "Win" | "Super" => {
                if mac {
                    "⌘"
                } else {
                    "Win"
                }
            }
            _ => part,
        })
        .collect::<Vec<_>>()
        .join(" ")
}
// Tray-only compact key labels. Raw registration strings and other UI labels stay unchanged.
pub fn tray_shortcut(raw: &str) -> String {
    tray_shortcut_for(raw, cfg!(target_os = "macos"))
}
fn tray_shortcut_for(raw: &str, mac: bool) -> String {
    // Availability belongs to diagnostics/CLI; the tray shows keys only.
    let keys = raw.split_whitespace().next().unwrap_or("");
    keys.split('+')
        .map(|key| {
            let key = match key {
                "Control" | "Ctrl" => "Ctrl",
                "Option" | "Alt" if mac => "Opt",
                "Command" | "Cmd" | "Super" | "Win" if mac => "⌘",
                "Shift" => "⇧",
                other => other,
            };
            key.to_owned()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn labels_hide_only_internal_wrappers_and_bound_unicode_titles() {
        assert_eq!(
            list_title("[터미널] 작업 (terminal-42)", "terminal-42", Kind::Terminal),
            "작업"
        );
        assert_eq!(
            list_title("작업 (다른 값)", "id", Kind::Note),
            "작업 (다른 값)"
        );
        for kind in [Kind::Note, Kind::Capture, Kind::Terminal] {
            assert_eq!(list_title("  ", "internal", kind), kind.default_title());
            assert_eq!(
                list_title("internal", "internal", kind),
                kind.default_title()
            );
        }
        assert_eq!(
            list_title(&"가".repeat(40), "id", Kind::Note),
            "가".repeat(30)
        );
    }
    #[test]
    fn display_keys_follow_os_without_changing_raw_contract() {
        let raw = "Ctrl+Shift+1";
        assert_eq!(shortcut_for(raw, true), "Ctrl ⇧ 1");
        assert_eq!(shortcut_for(raw, false), "Ctrl ⇧ 1");
        assert_eq!(shortcut_for("Ctrl+Alt+Cmd", true), "Ctrl Opt ⌘");
        assert_eq!(shortcut_for("Ctrl+Alt+Win", false), "Ctrl Alt Win");
        assert_eq!(
            shortcut_for("Ctrl+Alt+Cmd 사용 중 · 트레이로 실행", true),
            "Ctrl Opt ⌘ 사용 중 · 트레이로 실행"
        );
        assert_eq!(raw, "Ctrl+Shift+1");
    }
    #[test]
    fn tray_labels_show_only_keys_and_other_surfaces_keep_explanations() {
        assert_eq!(tray_shortcut_for("Ctrl+Shift+1", true), "Ctrl ⇧ 1");
        assert_eq!(
            tray_shortcut_for("Ctrl+Alt+Cmd 사용 중 · 트레이로 실행", true),
            "Ctrl Opt ⌘"
        );
        assert_eq!(tray_shortcut_for("Ctrl+Alt+Win", false), "Ctrl Alt Win");
        assert_eq!(
            tray_shortcut_for("Ctrl+Alt+Win 입력 감지 권한 필요 · 트레이로 실행", false),
            "Ctrl Alt Win"
        );
        assert_eq!(
            shortcut_for("Ctrl+Alt+Cmd 사용 중 · 트레이로 실행", true),
            "Ctrl Opt ⌘ 사용 중 · 트레이로 실행"
        );
        assert_eq!(shortcut_for("Ctrl+Alt+Cmd", true), "Ctrl Opt ⌘");
    }
}
