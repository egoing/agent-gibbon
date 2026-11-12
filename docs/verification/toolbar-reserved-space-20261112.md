# 툴바 전용 행 검증 — #135

## 작업 출처

- 작업 루트 `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/toolbar-reserved-space`
- 브랜치 `codex/toolbar-reserved-space`, 초기 계획 커밋 `02f7ead130f19ff89c9f59763300ac7de80be948`, 구현 미커밋.
- 이슈 #135, Draft PR #136.
- 개발 인스턴스 `dev-b02fb5c1ff3578daf9f82f6d`, 바이너리는 위 작업 루트의 `src-tauri/target/debug/agent-gibbon`.
- 최종 재빌드 후 PID 91330. 텍스트 검증 메모 `toolbar-135-progress`, 웹 `web-toolbar-135`. 해당 루트 개발 CLI inspect 응답의 `ok`, instance, 메모 ID·본문 또는 URL을 확인했다.
- Orca workspace `da5d2674-a5c4-4f3d-99cb-5f69bdbabe9c::/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/toolbar-reserved-space`. 전환·활성화 완료.

## 변경 기준

[디자인 시스템](../design-system.md)의 #135 절과 [시안](../design/toolbar-reserved-space.png)을 따른다. 툴바 두 상태에서 본문을 표시하며 전용 행의 초기 높이는 30px다. 웹은 외부 child WebView를 30px 아래에 배치한다. Tauri가 다중 WebView 호스트를 일반 webview_windows 조회에서 제외하므로 앱 전체 창 열거와 숨김 복원 조회를 보완했다.

다중 WebView API의 플랫폼 제약은 [Tauri 공식 문서](https://docs.rs/tauri/2.12.2/tauri/window/struct.Window.html#method.add_child)와 설치된 tauri 2.12.2 소스로 확인했다. 외부 페이지의 IPC 거부, URL 검사·팝업·다운로드 차단을 유지한다.

## 실행 결과

| 실행 | 결과 |
|---|---|
| `npm run test:ui` | 179개 통과, 새 툴바 상태 회귀와 자동 크기 계산 검사 포함 |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 211개 통과 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | 통과 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 불필요한 참조 보정 후 통과 |
| `npm run dev:build` | 최종 빌드 통과 |
| `git diff --check` | 통과 |
| SVG → PNG `rsvg-convert` | 렌더·시각 확인 완료 |
| Chromium 실제 UI 파일 18개 조합 | 텍스트·터미널·웹 Welcome × 너비 144/300/720 × 툴바 두 상태. 행 높이 30px, 본문 시작 좌표, 제목 표시 상태, 닫기 버튼 범위 통과 |

브라우저 검증은 `/tmp/gibbon-toolbar-browser/verify.cjs`, 결과 `/tmp/gibbon-toolbar-browser-results.json`, 캡처 `/tmp/gibbon-toolbar-index.html.png`, `/tmp/gibbon-toolbar-terminal.html.png`, `/tmp/gibbon-toolbar-web.html.png`에 남겼다. 브라우저는 native IPC·PTY·외부 child WebView의 실행 검증을 대신하지 않는다.

## 실제 macOS 개발 앱

- `scripts/agent-gibbon-dev`로 검증 메모와 `https://example.com` 웹 창을 생성·inspect했다. 웹은 최종 재시작 후에도 정확한 instance와 현재 URL·visible 상태를 반환했다.
- PID 91330의 실제 창 제목에 `toolbar-135-progress`가 포함됨을 AX snapshot에서 확인했다. 세션 `toolbar-135`에서 펼치기 버튼을 눌러 도구 버튼들이 다시 표시되고 동일 본문이 유지되는 것을 전후 tree에서 확인했다.
- 실제 접힘 화면 캡처 `/var/folders/ds/zm2jvd217ss_q5jy2bjcbb9c0000gn/T/orca-computer-use/1513020b-9dd4-40f1-8115-8770a40c7ede-screenshot.png`를 시각 확인했다. 제목·펼치기·닫기와 본문이 겹치지 않는다. 전후 상태는 `/tmp/gibbon-toolbar-native-state.json`, `/tmp/gibbon-toolbar-native-action.json`이다.
- Orca의 list-windows는 빈 목록을 반환했으나 get-app-state는 PID와 네이티브 제목을 포함한 AX tree·캡처를 반환했다. 다른 워크트리로 대체하지 않았다.

## 남은 범위

- 이미지·터미널·웹의 모든 실제 GUI 조작, 웹 Welcome→외부 콘텐츠 전환과 resize·핀·숨김 복원 종단 회귀는 미검증이다.
- Windows·Intel 실행, 전체 플랫폼 workflow는 미실행이다. 최종 구현 커밋 승인·push 후 해당 SHA에 전체 검사를 요청해야 한다.
- 툴바 초기 상태는 펼침으로 구현했다. 툴바 접힘 상태의 디스크 영속 저장은 이번 구현에 포함하지 않았다.
- 웹 닫기는 기존 세션 숨김 계약을 유지한다. 종료 변경은 #119 범위다.

## 사용자 피드백 반영

이전의 본문 유지 해석과 별도 툴바 상태를 폐기했다. 접으면 본문을 숨기고 창 높이를 32px로 줄이며 펼치면 이전 크기를 복원한다. 공통 토글과 닫기는 오른쪽 동일 슬롯에 둔다. 이전 18개 조합 검사는 수정 전 결과이므로 이번 동작의 근거로 재사용하지 않는다.

수정 후 UI 179개·Rust 211개·Clippy를 통과했다. native PID 86679, 동일 인스턴스의 `toolbar-135-progress`에서 folded=true·실제 높이 64 physical px(배율 2, 32 logical px)를 inspect로 확인했다. 이후 Orca가 on-screen window를 찾지 못해 같은 실행에서 펼침 재검증을 완료하지 못했다. 새 실행에서 추가 검증한다.

추가 실행 PID 90620에서도 공통 툴바 버튼으로 folded=true·32 logical px 접힘을 확인했다. 수정된 Chromium 18개 조합은 두 상태의 토글 x좌표 동일, 접힌 본문 비표시를 모두 통과했다. GUI 도구의 window_not_found로 실제 펼침 재검증은 보류한다.

## 시스템 툴팁 전환

사용자 요청에 따라 메모 내부의 `#tooltip` 렌더링과 창 경계 내 강제 좌표 계산을 제거했다. 공통 툴바 버튼은 WebView 기본 `title` 툴팁을 사용하며 `aria-label` 변경에 맞춰 제목을 갱신한다. 접기·펼치기 버튼 문구의 회귀 검사를 포함한 UI 179개와 개발 빌드가 통과했다. SVG·PNG 시안을 갱신하고 렌더를 시각 확인했다. macOS·Windows의 실제 native 툴팁 출현·화면 경계 배치는 실행 검증 전이며, 기존 실행 중 개발 앱은 사용자 창 상태 보존을 위해 강제로 재시작하지 않았다.

## 접힌 메모의 피드백 차단

접힌 동안 tip 예외를 제거하고 native 레이아웃에서 모든 피드백의 표시를 차단한다. 접기 시 기존 바를 즉시 숨기며, 펼침 시 피드백 발행 상태를 재평가한다. 이전 클라이언트가 `folded_tip=true`를 보내는 경우와 오류·버튼이 있는 경우도 접힘 상태에서 거부하는 회귀 검사를 추가했다. Rust 212개·Clippy·개발 빌드 통과. 기존 시안의 접힌 상태에 피드백이 없는 배치를 그대로 적용했으며 실제 앱 GUI는 이번 변경으로 재검증하지 않았다.

## Codex 작업 공간으로 이전 후 검증

사용자 지시에 따라 현재 작업 루트는 `/Users/egoinglee/.codex/worktrees/toolbar-reserved-space/agent-gibbon`이다. 두 폴더의 HEAD `02f7ead130f19ff89c9f59763300ac7de80be948` 일치와 구현 파일 31개의 바이트 일치를 확인했다. 기존 `package-lock.json` 변경은 그대로 보존했다. `codex/toolbar-reserved-space` 브랜치를 현재 폴더로 옮겼으며 Orca는 같은 HEAD에서 detached 상태의 복구 사본으로 유지한다.

현재 루트에서 UI 179개·Rust 212개, Rustfmt·Clippy·개발 빌드와 Chromium 18개 조합을 다시 통과했다. 실행 로그는 `/tmp/gibbon-codex-ui.log`, `/tmp/gibbon-codex-rust.log`, `/tmp/gibbon-codex-clippy.log`, `/tmp/gibbon-codex-build.log`이다. 브라우저 검증은 `/tmp/gibbon-toolbar-browser/verify-codex.cjs`로 실행했다.

`npm run dev:info`의 인스턴스는 `dev-babaa0322f6fa3a26987397f`이며 바이너리는 현재 루트의 `src-tauri/target/debug/agent-gibbon`이다. 기존 개발 실행기가 변경을 감지해 재빌드한 PID 49525의 cwd·실행 파일 절대 경로를 lsof로 확인했다. 검증 메모 `toolbar-135-codex`를 이 루트의 개발 CLI로 생성·inspect하여 `ok:true`, instance 일치와 본문 고유 표식을 확인했다. 출처 응답은 `/tmp/gibbon-codex-note-inspect.json`이다.

native AX 조회는 같은 PID의 사용자 `main` 창을 반환했으므로 해당 창에 조작을 수행하지 않았다. 검증 메모의 전체 native GUI·시스템 툴팁 출현과 Windows·Intel 실행은 남아 있다. 기존 실행기의 DEV 브랜치 배지에는 실행기 시작 당시 detached 값이 남아 있으며, 출처 판정은 루트·인스턴스·바이너리·PID를 사용했다.

## Orca 폴더 정리와 남은 검증

사용자의 명시적인 삭제 요청에 따라 원본 Orca의 변경 파일이 모두 현재 Codex 폴더에 동일하게 보존됐음을 다시 대조했다. 이후 `orca worktree rm --worktree path:/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/toolbar-reserved-space --force --json`의 `removed:true`, 실제 경로 부재와 `git worktree list`의 등록 제거를 확인했다. 현재 브랜치와 구현은 Codex 폴더에 보존돼 있다.

추가 native GUI 연결을 시도했으나 CUA 앱 목록에 unbundled 개발 앱이 없으며 앱 이름과 정확한 바이너리 경로 모두 `Invalid app`으로 거부됐다. 설치 앱으로 대체하지 않았다. 실제 native 펼침 재검증과 시스템 툴팁 출현 검증은 이 연결 제한으로 남아 있다. 기존 자동 검사·빌드와 CLI 출처 검증 결과는 위 Codex 이전 후 기록을 따른다.

최종 확인에서 native `folded` CSS 상태까지 함께 적용하는 브라우저 18개 조합을 추가 실행해 접힌 본문 비표시와 토글 x좌표 유지를 확인했다. 사용자 승인에 따라 기존 `package-lock.json` 변경은 JSON 내용이 완전히 같고 속성 순서만 다름을 대조한 뒤 되돌렸다. 창 제어·다중 WebView 및 Tauri feature 변경의 플랫폼 간 영향을 확인하기 위해 승인된 작업 커밋을 push한 뒤 `release.yml` 수동 검사를 요청한다.

## 펼침 상태의 피드백 복구

사용자가 실제 앱에서 시스템 툴팁과 접힌 상태의 피드백 숨김은 정상이나 펼친 상태의 피드백이 보이지 않는다고 보고했다. 공통 툴바 토글이 기존 메모 토글 경로를 우회해 펼침 안내를 발행하지 않는 문제를 확인하고, 메모에서는 기존 토글을 호출하도록 연결했다. 피드백 발행 서명에는 접힘 상태도 포함해 진행 중인 이전 발행이 끝난 뒤 펼친 상태를 다시 발행하도록 보완했다.

기존 메모의 정상 토글 경로와 공통 툴바의 위임을 검증하는 회귀 검사를 추가했다. UI 180개와 개발 빌드, diff 공백 검사를 통과했다. 최초 전체 플랫폼 CI `43202459858`은 초기 구현 SHA `68aedd0`의 결과이며 이 수정의 검증으로 재사용하지 않는다. 새 커밋을 push한 뒤 새 SHA의 전체 검사를 요청한다. 사용자에게 펼침 상태의 실제 피드백 재확인을 요청했다.

## 최종 높이·피드백 드래그·구분선

사용자가 실제 앱에서 펼침 피드백 복구까지 정상이라고 확인했다. macOS 스티커의 접힌 막대를 CUA로 캡처해 현재 배율의 24 device px, 약 12 logical px 기준으로 확인한 뒤, 사용자 후속 요청에 따라 최종 접힘 높이는 그 두 배인 24px, 제목은 12px로 확정했다. 테두리 안의 22px 행 중앙에 제목을 배치하고 토글·닫기의 가로 슬롯은 유지했다. 사용자가 최종 크기도 정상이라고 확인했다.

피드백 안내 문구와 빈 영역의 왼쪽 마우스 입력은 기존 `entrance::drag_note` 경로로 부모 메모를 이동한다. 버튼·링크·입력 컨트롤과 보조 버튼은 드래그를 시작하지 않는다. 피드백이 위에 붙으면 아래 경계, 아래에 붙으면 위 경계에 글자색 1px 선을 불투명도 50%로 표시하며 왼쪽 여백은 3px다. 오류 안내는 오류 글자색을 따른다.

사용자가 피드백 드래그 중 잠시 어긋나 보이는 현상을 보고했다. 설치된 tao 0.37.1의 macOS 구현에서 `addChildWindow` 연결을 확인했다. macOS에서 피드백 위에 왼쪽 버튼을 누른 동안 수동 위치 보정을 생략해 AppKit 자식 창 이동과 중복되지 않도록 작은 보정을 적용했다. native 드래그의 시각적 개선 효과는 자동 재현하지 못했으며 사용자 요청에 따라 추가적인 복잡한 드래그 구현은 도입하지 않았다.

최신 로컬 UI 181개·Rust 212개, Rustfmt·Clippy·개발 빌드 통과. 브라우저 검증 `/tmp/gibbon-toolbar-browser/verify-collapsed-24.cjs`의 텍스트·이미지·터미널·웹 × 세 너비 × 두 상태 24개 조합에서 제목 12px·수직 중앙·버튼 위치·본문 접힘을 확인했다. `/tmp/gibbon-toolbar-browser/verify-feedback.cjs`의 12개 조합에서 구분선 1px·opacity 0.5·왼쪽 3px·위아래 위치·오류 색상을 확인했다. 결과는 `/tmp/gibbon-feedback-browser-results.json`이다. 두 SVG·PNG 시안을 렌더링하고 확인했다.

초기 구현 SHA `68aedd0`의 전체 검사 `43202459858`은 성공했다. 펼침 피드백 SHA `c5e418f`의 실행 `43203147871`은 후속 변경으로 대체하며 최신 높이·드래그 변경의 검증으로 재사용하지 않는다. 새 커밋 SHA의 전체 플랫폼 검사를 요청한다.

## 피드백 여백·hover 최종 조정

사용자 요청으로 구분선의 오른쪽에도 3px 여백을 추가했다. 피드백 바의 사방 패딩은 본문과 같은 3px로 통합하고 `--memo-padding` 토큰을 공유한다. 위·아래 부착 시 추가하던 6px padding override를 제거했다. 툴바 rollover의 outline을 제거하고 배경색만 바꾸며 키보드 focus-visible의 2px 표시는 유지한다. 웹 툴바에도 같은 hover 배경을 반영했다.

UI 181개·개발 빌드 통과. `/tmp/gibbon-toolbar-browser/verify-feedback.cjs`의 12개 조합에서 좌우 3px·본문과 같은 사방 패딩을 확인했다. `/tmp/gibbon-toolbar-browser/verify-padding-hover.cjs`의 24개 조합에서 hover outline/border 없음·배경 존재, Tab으로 닫기에 이동했을 때 2px 포커스 표시를 확인했다. 시안 SVG·PNG를 재렌더링하고 시각 확인했다. 이 후속 변경의 최신 SHA에 전체 플랫폼 CI를 재요청한다.
