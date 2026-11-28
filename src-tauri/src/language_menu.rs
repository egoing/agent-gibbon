//! Keep the native Edit actions, key equivalents and AppKit window/help roles.
use crate::language::tr;
use tauri::menu::{
    AboutMetadata, Menu, PredefinedMenuItem as Item, Submenu, HELP_SUBMENU_ID, WINDOW_SUBMENU_ID,
};

pub fn install(app: &tauri::AppHandle) -> tauri::Result<()> {
    let about = AboutMetadata {
        name: Some(app.package_info().name.clone()),
        version: Some(app.package_info().version.to_string()),
        copyright: app.config().bundle.copyright.clone(),
        authors: app.config().bundle.publisher.clone().map(|name| vec![name]),
        ..Default::default()
    };
    let application = Submenu::with_items(
        app,
        "Jajan",
        true,
        &[
            &Item::about(app, Some(tr("Jajan 정보")), Some(about))?,
            &Item::separator(app)?,
            &Item::services(app, Some(tr("서비스")))?,
            &Item::separator(app)?,
            &Item::hide(app, Some(tr("Jajan 숨기기")))?,
            &Item::hide_others(app, Some(tr("다른 앱 숨기기")))?,
            &Item::separator(app)?,
            &Item::quit(app, Some(tr("Jajan 종료")))?,
        ],
    )?;
    let file = Submenu::with_items(
        app,
        tr("파일"),
        true,
        &[&Item::close_window(app, Some(tr("창 닫기")))?],
    )?;
    let edit = Submenu::with_items(
        app,
        tr("편집"),
        true,
        &[
            &Item::undo(app, Some(tr("실행 취소")))?,
            &Item::redo(app, Some(tr("다시 실행")))?,
            &Item::separator(app)?,
            &Item::cut(app, Some(tr("잘라내기")))?,
            &Item::copy(app, Some(tr("복사")))?,
            &Item::paste(app, Some(tr("붙여넣기")))?,
            &Item::select_all(app, Some(tr("모두 선택")))?,
        ],
    )?;
    let view = Submenu::with_items(
        app,
        tr("보기"),
        true,
        &[&Item::fullscreen(app, Some(tr("전체 화면 전환")))?],
    )?;
    let windows = Submenu::with_id_and_items(
        app,
        WINDOW_SUBMENU_ID,
        tr("윈도우"),
        true,
        &[
            &Item::minimize(app, Some(tr("최소화")))?,
            &Item::maximize(app, Some(tr("확대")))?,
            &Item::separator(app)?,
            &Item::close_window(app, Some(tr("창 닫기")))?,
        ],
    )?;
    let help = Submenu::with_id_and_items(app, HELP_SUBMENU_ID, tr("도움말"), true, &[])?;
    app.set_menu(Menu::with_items(
        app,
        &[&application, &file, &edit, &view, &windows, &help],
    )?)?;
    Ok(())
}
