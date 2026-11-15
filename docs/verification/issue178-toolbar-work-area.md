# #178 툴바 더블클릭 확대·복원 검증

## 대상과 출처

- 루트: `/Users/egoinglee/.codex/worktrees/toolbar-double-click/agent-gibbon`
- 브랜치: `codex/toolbar-double-click`, 기준 `91b1751`, 계획 커밋 `b96c85a`. 제품 변경은 승인 전 미커밋이다.
- `pnpm dev:info`: instance `dev-02fb6da00054817952960148`, binary `/Users/egoinglee/.codex/worktrees/toolbar-double-click/agent-gibbon/src-tauri/target/debug/agent-gibbon`.
- Node v26.0.0, pnpm 11.25.0, rustc 1.91.1, macOS ARM64. lockfile Tauri 2.12.2.
- 이슈 [#178](https://github.com/egoing/agent-gibbon/issues/178), 계획 Draft PR [#180](https://github.com/egoing/agent-gibbon/pull/180).

## 구현과 판단

공통 toolbar의 즉시 native drag를 제거하고 실제 왼쪽 포인터 이동 4px 후 `chrome_drag`를 호출한다. 빈 공간의 stationary 더블클릭만 `chrome_work_area`로 전달한다. 버튼·제목·입력은 제외하고 빠른 수직 이동도 window pointermove에서 이어받는다. 본문 drag는 기존 경로를 유지한다.

[Tauri 2.12.2 공식 문서](https://docs.rs/tauri/2.12.2/tauri/webview/struct.WebviewWindow.html)의 maximize/unmaximize·fullscreen·current_monitor·outer_position/outer_size API를 확인했다. 이번 요구의 작업 영역 정확한 기하와 이미지 비율·접힘 복원을 함께 관리하기 위해 native maximize 상태 대신 기존 entrance::place와 current_monitor.work_area를 사용한다. 기존 AppKit 프레임 변환/Windows outer-size 경로를 재사용하며 별도 Space 전체화면 API는 호출하지 않는다.

창별 실행 중 snapshot에 기존 프레임의 물리 좌표·크기·해당 배율·접힘 펼친 크기·자동 배치 제외 여부를 저장한다. 핀·포커스 API는 호출하지 않는다. 이미지 aspect는 확대 동안 해제하고 복원 시 돌린다. 접힘에서 확대하면 일시 펼치고 복원 시 다시 접는다. 기존 resize 이벤트가 PTY fit과 웹 child bounds를 처리한다. 내용 갱신/숨김 재열기가 확대 상태를 자동 배치로 해제하지 않도록 layout mark를 보호한다.

확대 이전 snapshot은 실행 중에만 보존한다. 앱 재시작 후의 확대 이전 프레임 복원은 구현 범위가 아니며 기존 시작 복원은 마지막 현재 기하를 따른다.

## 실행한 검사

- `pnpm test`: Rust 223개, UI 207개 통과. 최종 제품 변경에 대한 마지막 결과는 아래 보강 기록을 따른다.
- `pnpm check`: Clippy all-targets `-D warnings` 통과.
- `cargo fmt --manifest-path src-tauri/Cargo.toml`: 적용. `git diff --check`: 통과.
- `pnpm dev:build`: 현재 워크트리 debug 빌드 통과.
- `PLAYWRIGHT_MODULE=/tmp/gibbon-playwright/node_modules/playwright/index.mjs node scripts/toolbar-work-area-smoke.mjs`: Chromium 실제 mouse.click/dblclick/down/move/up로 텍스트·이미지·터미널·웹 4종 통과. single click에서 이동 없음, dblclick 1회 확대 명령, 작은 이동 미작동/4px 이상 drag 1회, 버튼·제목 확대 제외 확인. 실제 HTML/CSS/공통 toolbar.js를 사용하며 unrelated JS를 제외하고 native IPC를 mock했다. OS 창 확대 검증이 아니다.
- `tests/toolbar-work-area.test.mjs`: pending toggle 중복 차단, 실패 안내 후 재시도, pointer release/cancel, 비주버튼, interactive descendants 등을 확인한다. 웹 전용 drag 검사는 공통 검사로 옮겼다.
- `rsvg-convert docs/design/toolbar-work-area.svg -o docs/design/toolbar-work-area.png`: 렌더 후 실제 이미지 육안 확인. 기존 toolbar-reserved-space SVG/PNG를 참고했다. 신규 시안은 설명용이며 실제 앱 캡처가 아니다.

## 개발 CLI와 GUI

`./scripts/agent-gibbon-dev open/inspect --id issue178-text`에서 `ok: true`, instance 일치와 `ISSUE178-02fb6da0` 본문을 확인했다. 첫 실행 PID 51655의 실행 파일 절대 경로가 위 binary와 일치했다. 애니메이션 종료 뒤 위치 (3496,2208), 크기 600×96 물리 px, folded=false, always_on_top=true를 확인했다.

최초 GUI 시도는 Orca runtime_unavailable, CUA는 debug binary/path/PID 선택 Invalid app로 입력을 실행하지 못했다. 설치 앱으로 대체하지 않았다. 이후 팀장이 지정한 공식 `orca open --json`→`orca computer get-app-state --app pid:77231 --json` 경로로 연결했다. 최신 빌드 뒤 이전 자체 PID 51655만 종료하고 전용 CLI로 재기동했다. 재기동 PID 77231, 동일 binary/instance, native title `Agent Gibbon [DEV codex/toolbar-double-click · 02fb6da0] — issue178-text`, native window ID 73633, 화면/CLI의 고유 본문을 대조했다.

- 초기 기하: (3496,2208), 600×96 physical px, 배율2. [원본 캡처](assets/issue178/initial.png).
- `orca computer click --app pid:77231 --window-id 73633 --x 50 --y 10 --click-count 2 --json`: [확대 캡처](assets/issue178/zoomed.png), CLI 기하 (0,50), 4096×2254, pin=true, excluded=true. AX는 logical (0,25), 2048×1127. [CLI 결과](assets/issue178/after.json).
- 같은 빈 공간을 다시 double-click: 위치 (3496,2208), 600×96, folded=false, pin=true, excluded=false로 정확 복원. [복원 캡처](assets/issue178/restored.png) · [CLI](assets/issue178/restored.json).
- 첫 좌표 선택(215,10)은 실제 warp 버튼이었다. 확대하지 않고 해당 버튼의 ‘최초 위치가 아직 준비되지 않았습니다’ 오류가 표시됐다. 이후 screenshot scale2를 적용해 실제 빈 영역(50,10)으로 바로잡았다. 사용자 메모가 아닌 전용 메모에서만 발생한 시험 오류다.
- background drag는 성공 응답이지만 CLI 위치가 그대로여서 통과로 판정하지 않았다. 전경 전환을 고지하고 `orca computer drag --app pid:77231 --window-id 73633 --from-x 50 --from-y 10 --to-x 75 --to-y -45 --restore-window --json` 후 CLI (3540,2098), 600×96, excluded=true로 실제 이동 확인. [캡처](assets/issue178/dragged.png) · [CLI](assets/issue178/dragged-front.json). 명령 자체의 synthetic verification은 unverified였으나 독립 CLI 기하 변화로 검증했다.
- 추가 GUI 사용 중단 지시 직후 새 입력을 중단하고 자체 issue178-text를 CLI close해 visible=false 확인 후 정확 PID77231만 종료했다. Orca와 다른 앱은 종료하지 않았다. 사전 포인터 좌표를 확보하지 않아 원좌표 복원은 확인하지 못했으며 전경 입력 뒤 기존 사용자 포커스의 정확 복원도 별도 실증하지 못했다. 기존 사용자 메모의 기하는 조작하지 않았다.

### 최종 보강 검사

빠른 수직 drag와 확대 중 자동 배치 보호, 기존 최소 크기를 변경하지 않도록 보강한 최종 코드에서 UI207/Rust223, Chromium4종, Clippy, dev build가 통과했다. `git diff --check` 통과. 원격 PR은 승인된 계획 커밋만 포함하고 제품 코드는 미커밋이다.

## 미검증과 판정

실제 image aspect·PTY fit·외부 WebView 경계·숨김 재표시·접힌 상태 왕복, 혼합 DPI/음수 좌표/모니터 해제 및 Windows·Intel macOS는 미검증이다. 코드 검토·browser mock 결과를 실제 native 통과로 대체하지 않는다. 전체 플랫폼 CI·배포는 요청 범위에서 제외했다.

Conf 5 (macOS 텍스트 메모 native 확대·복원·이동 실제 확인; 나머지 유형은 browser IPC mock/테스트 수준), Risk Medium (공통 창 기하·접힘·상태 관리). native 검증 후 범위에 맞춰 갱신한다.

## 2026-11-15 웹 창 더블클릭 실패 후속 조사

사용자가 웹 창에서 확대되지 않는다고 보고했다. 읽기 전용 프로세스 조회에서 17번 전용 binary PID47995와 web-mobile-chrome 전용 binary PID49985가 동시에 실행 중이었다. 16번 소스에는 이번 chrome_work_area/dblclick 구현이 없었고, 최초 17번 CLI list에는 웹 창이 없었다. 이 사실만으로 사용자가 조작한 창의 소속이나 실패 원인을 확정하지 않는다.

자체 CLI `web --id web-issue178-double-click`로 전용 Welcome을 열었다. `inspect`에서 instance `dev-02fb6da00054817952960148`, native title `Agent Gibbon [DEV codex/toolbar-double-click · 02fb6da0] — 웹 — web-issue178-double-click`, visible=true, focused=false, 기하 (1448,414)1200×800 physical px를 확인했다. 사용자가 이 창의 툴바 빈 공간을 더블클릭한 수동 시험 결과를 “커집니다”로 확인했다. 이는 출처를 확인한 웹 Welcome의 확대에 대한 사용자 확인이며, 복원·외부 페이지 탐색 뒤 확대까지 확인한 것은 아니다. 기존 창/프로세스의 재시작·종료·Orca GUI는 수행하지 않았다.

브라우저 회귀를 보강해 실제 web.js·web-chrome.js·toolbar.js 및 appearance 모듈을 함께 로드한 Welcome 초기화와 URL 제출 뒤 외부 콘텐츠 iframe fixture가 공존하는 경우 모두 빈 공간 double-click이 chrome_work_area를 1회 전달함을 확인했다. 총 기본4종+full-web2경로 통과, pageerror 없음. iframe은 OS native child WebView가 아니며 native 크기 변경과 IPC는 mock이므로 이번 사용자 실패의 원인 해결 또는 실제 웹 확대 통과로 기록하지 않는다. 제품 동작은 아직 변경하지 않았다.

후속 사용자 확인: 전용 웹 Welcome은 더블클릭으로 확대됨을 확인했다. 최초 실패 창의 소속과 실패 원인은 여전히 미확정이며 다른 인스턴스 사용으로 단정하지 않는다. 이번 후속에서는 제품 코드 변경 없이 회귀 검사와 실행 범위 기록을 보강했다.

## close 승인과 최신 main 통합 — 2026-11-15

사용자의 명시적인 close 호출로 최종 제품 커밋/push/병합을 승인받았다. 기존 승인 대기·미커밋 표기는 당시 상태의 기록이다. main `93f027f`를 통합하며 디자인 문서는 양쪽 기준을 보존하고 이미지 툴바 상태 필드와 이번 확대 snapshot을 함께 유지했다. 본문 선택을 위해 main에서 제한한 document mousedown handler는 제거하고 공통 toolbar의 실제 포인터 이동 처리로 일원화했다. 관련 회귀는 공통 toolbar 검사 및 메모 본문에 즉시 mousedown drag를 등록하지 않는 검사로 확인했다.

최신 통합에서 Rust234/UI189, Clippy, Rustfmt, debug 개발 빌드를 수행했다. Chromium 기본4종 및 웹 전체 스크립트2경로를 다시 확인했다. 이전 native 텍스트 실증과 웹 Welcome 사용자 확인은 이전 코드 버전의 증거이며 최신 통합 뒤 native 실환경 재검증은 하지 않았다. 다른 앱·사용자 창을 재시작하거나 조작하지 않았고 전체 플랫폼 CI·릴리스는 실행하지 않았다. 최신 통합의 Conf는3, Chromium 이벤트 처리 범위는4, 과거 native 실증은 해당 버전·유형에 한정한5이며 Risk는Medium이다.
