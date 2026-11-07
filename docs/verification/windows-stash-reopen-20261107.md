# Windows 전체 숨김 직후 재표시 보정

## 원인과 변경

`stash::place_indicator`가 일반 메모의 `show_without_focus`를 호출해 `sync_opacity`에서 Indicator의 네이티브 불투명도를 읽었다. Tao 0.37.1은 `set_ignore_cursor_events(true)`에 `WS_EX_TRANSPARENT | WS_EX_LAYERED`를 설정하지만 `SetLayeredWindowAttributes`를 호출하지 않는다. 이 창에 대한 `GetLayeredWindowAttributes` 실패가 `tick`으로 전파되면 오류 복구의 `toggle`이 숨긴 창을 곧 복원한다.

[Microsoft API 문서](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getlayeredwindowattributes)는 사전 `SetLayeredWindowAttributes` 호출을 요구한다. 로컬 Tao 소스 `platform_impl/windows/window_state.rs`의 플래그 처리와 Windows 네이티브 테스트로 해당 조건을 확인했다. 이 기록은 사용자가 보고한 전체 앱 증상을 수정 전 GUI에서 직접 재현했다고 주장하지 않는다.

Indicator를 기존 `show_frame` 경로로 표시해 메모 투명도 조회/설정을 피한다. Windows에서는 기존 비활성 표시·포커스 복원 함수를 사용한다. CSS 투명도, 키 감지, 전환 시간, 오류 복구 정책은 유지한다. 시각 기준은 [디자인 시스템](../design-system.md)을 따르며 외형·배치 변경은 없다.

## 작업 출처와 로컬 검사

- root: `C:/Users/egoin/dev/project/agent-gibbon`
- `npm run dev:info`의 id: `dev-fbc7d562e05aec4798036a95`
- binary: `C:/Users/egoin/dev/project/agent-gibbon/src-tauri/target/debug/agent-gibbon.exe`
- Node 20.11.0, npm 10.2.4, Rust/Cargo 1.99.1, Windows x64.
- `cargo test --manifest-path src-tauri/Cargo.toml`: 174개 통과. 추가 네이티브 테스트는 클릭 통과용 layered 창의 조회 실패와 명시적 alpha 설정 후 조회 성공을 확인한다.
- `node scripts/test-ui.mjs`: 118개 통과, 1개 건너뜀. 최초 샌드박스 실행은 자식 프로세스 `EPERM`으로 실패했고 권한 확장 재실행에서 통과했다.
- Rustfmt, Clippy(`--all-targets -- -D warnings`), `npm run dev:build`, `git diff --check`: 통과.

## 개발 CLI 실행

기본 IPC 경로는 시작 시 `os error 1920`이 발생했다. 해당 경로의 기존 파일을 삭제하지 않고 검증 전용 `LOCALAPPDATA=<root>/.tmp/stash`로 같은 개발 실행기를 사용했다. 이 대체 경로의 성공을 기본 경로 정상 동작으로 간주하지 않는다.

수정 빌드 완료 후 `node scripts/dev.mjs cli open|close|update|inspect --id win-stash-20261107`을 순차 실행했다. `inspect`의 `ok: true`, 예상 instance, ID 및 `win-stash-20261107 숨김 갱신 유지` 본문이 일치했다. 숨김 뒤 갱신에도 `visible: false`, `focused: false`, 위치 `(1545, 960)`과 크기 `(375, 60)`을 유지했다. 해당 개발 바이너리의 PID는 23048이었다. 앞선 수정 전 실행 PID 24208의 CLI 결과는 수정 빌드 검증으로 재사용하지 않았다.

설치 검증 앱이 입력 lease를 점유하여 `stash.shortcut`은 `Ctrl+Alt+Win 사용 중 · 트레이로 실행`이었다. 설치 앱은 종료하지 않았다. 단축키나 트레이로 실제 전체 숨김/복원을 수행하지 않았으므로 해당 GUI 동작, 아이콘 회피, 실제 전환 중 포커스 및 macOS 실행은 미검증이다. GUI 캡처는 생성하지 않았다.

## 전체 플랫폼 검사

전역 창 표시 경로 변경이므로 프로젝트 지침에 따라 작업 브랜치의 수동 Release 검사를 요청한다. 실행 URL·대상 SHA·결과는 작업 결과에서 별도로 보고하며, 요청/대기와 통과를 구분한다. 태그·릴리스 공개는 수행하지 않는다.
