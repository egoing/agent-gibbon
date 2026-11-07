# #93 터미널 시작 화면 테마 대비 검증 — 2026-11-07

## 변경과 기준

사용자 첨부 화면의 회색 전경색 `#2c2f7e`와 고정 선택 배경 `#414e63` 조합은 Chromium 계산상 **1.3745:1**이었다. Welcome과 Toolbar에 독립 메모 디자인을 적용하고, 선택 행·버튼·명령 미리보기·제목 입력은 해당 테마 panel/foreground를 사용하도록 수정했다. 보조 문구와 placeholder도 같은 전경색을 사용한다. 선택은 안쪽 1px 테두리와 aria-pressed, 키보드 초점은 바깥 2px 테두리로 구분한다. 제목 적용 버튼의 24px 고정폭에서 생긴 세로 줄바꿈은 최소 64px로 보완했다.

컬러는 개별 저장색이 없는 터미널 기본색 `#20252e`를 사용한다. 흰색·회색·검정과 시스템 주간/야간을 지원하고 관리 창 테마와 독립적이다. 전환은 CSS 변수만 갱신하며 선택·제목 초안·DOM·PTY를 다시 만들지 않는다. 실행 후 xterm 팔레트는 기존 설정을 유지한다.

[디자인 시스템](../design-system.md) · [SVG 시안](../design/terminal-welcome.svg) · [PNG 시안](../design/terminal-welcome.png). 시안은 실제 캡처와 구분하며 rsvg-convert 렌더 후 직접 확인했다.

## 출처와 baseline

- 작업 루트 `/Users/egoinglee/.codex/worktrees/cd0d/agent-gibbon`, 브랜치 `codex/93-terminal-theme-contrast`.
- 최초 기준 `962301168cbac037310f47ca8e13fbf0bae646a8`, red `4539a6efa80bd13b344345e4ddd74bc5fa74dd42`. `refs/is2/base/93`, `refs/is2/red/93`에 보존한다.
- Node 24.21.0, Cargo/Rust 1.91.1, macOS ARM64. npm ci 성공, lockfile·의존성 변경 없음.
- `/tmp/gibbon-issue93/baseline`에 구현 전 원본 로그와 명령·exit JSON 보존. npm test, cargo check --locked --all-targets, fmt+clippy, npm run dev:build가 모두 exit0. baseline 제외 0건.

## 실행 결과

| 게이트 | 명령 | 결과 |
|---|---|---|
| 신규 | `node --test tests/terminal-theme.test.mjs tests/terminal-appearance.test.mjs` | 16 PASS |
| 회귀 | `npm test` | Rust 182 / UI 136 PASS |
| 타입 | `cargo check --locked --manifest-path src-tauri/Cargo.toml --all-targets` | PASS |
| 린트 | `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` 및 `cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | PASS |
| 빌드 | `npm run dev:build` | PASS, 현재 워크트리 debug 빌드 |
| 기존 Welcome | `node scripts/terminal-welcome-smoke.mjs` | Chromium PASS: 선택→실행·명령·폴더·제목·오류·좁은창 회귀 |
| 신규 브라우저 | `node scripts/terminal-theme-smoke.mjs` | Chromium 14개 테마/크기 조합 PASS |
| 문서/패치 | rsvg-convert 렌더, PNG 확인, `git diff --check` | PASS |

원본 final 로그는 `/tmp/gibbon-issue93/final`, 신규 browser 원본과 캡처는 `/tmp/gibbon-issue93/browser-final`에 있다. 브라우저 두 명령은 `PLAYWRIGHT_MODULE=/tmp/agent-gibbon-is2-57/browser-tools/node_modules/playwright/index.mjs`를 사용했다. 기존 설치 도구만 재사용했으며 과거 실행 결과를 이번 증거로 사용하지 않았다.

신규 브라우저는 실제 HTML/CSS/JS/xterm을 요청 가로채기로 로드하며 Tauri IPC만 대체한다. 서버·외부 요청 없이 720×700/320×240, 흰색/회색/검정/컬러/시스템 주간·야간, 고정 회색의 야간 유지, 별도 같은 origin 문서의 storage 전파·media 변경을 검사했다. 각 상태에서 실제 computedStyle 대비가 4.5:1 이상이며 가로 넘침이 없다. 제목 초안·선택·DOM 동일성, 키보드 명령 미리보기 focus ring, Run 1회, xterm options.theme 유지도 확인했다. 테스트는 사용자의 실제 셸을 실행하지 않는다.

[현재 회색 실제 브라우저 캡처](issue93-terminal-theme/browser-gray.png) · [좁은 검정 화면](issue93-terminal-theme/browser-small-black.png). [출처·색상별 계산 결과](issue93-terminal-theme/provenance.json).

## red와 테스트 정정

신규 CSS 대비 테스트는 구현 전에 흰색·회색·시스템 주간·여러 컬러에서 실패했으며 실제 Chromium 선택 행도 1.3745:1로 실패했다. 원본 `/tmp/gibbon-issue93/red-unit.log`, `red-browser.log`에 보존했다. 구현 후 대비·테마 검사들은 통과했다.

추가 xterm 팔레트 assertion은 처음 `.xterm-screen`의 computed background를 설정값으로 오인해 transparent로 실패했고, 첫 정정의 `.xterm-viewport`도 별도 검정 배경이므로 설정값을 대변하지 못했다. 최종적으로 실제 `terminal.options.theme` 전체가 기존 `{background:'#20252e', foreground:'#f0f0f0'}`인지 검사한다. 잘못된 측정 대상 정정이며 대비·테마·상태 보존 assertion은 삭제하거나 약화하지 않았다. 이전 실패 원본은 `green-browser.log`, `green-browser-retry1.log`, 최종 통과는 `green-browser-retry2.log`와 final/theme-browser.log다. 독립 감사에서 이 정정의 정당성을 별도로 판단해야 한다.

## 실제 개발 앱 검증과 한계

- `npm run dev:info`의 root·id·binary를 확인했다. id는 `dev-347f6deb27841f629bb0e49e`, binary는 위 작업 루트의 `src-tauri/target/debug/agent-gibbon`이다.
- 개발 CLI로 진행 메모 `is2-93-theme-cd0d`를 만들고 inspect의 ok=true·예상 instance·본문을 확인했다. 이후 갱신만 사용한다. 기존 설치 앱과 다른 워크트리 앱은 조작/종료하지 않았다.
- 실제 Welcome은 앱 종료 상태에서 전용 테스트 메타데이터 `terminal-93001`만 준비하고 개발 CLI로 열었다. 첫 메타데이터 경로 착오로 생성된 일반 테스트 메모는 자체 프로세스를 종료한 뒤 `/tmp/gibbon-issue93/accidental-test-note`로 보존·이동하고 올바른 docs/.agent-gibbon-terminals 경로로 정정했다. 사용자 메모는 변경하지 않았다.
- 재빌드·재시작 후 PID70200의 실행 파일 절대 경로와 CLI inspect의 instance·id·kind=terminal을 재확인했다. PID에 속한 네이티브 제목 `Agent Gibbon [DEV codex/93-terminal-theme-contrast · 347f6deb] — terminal-93001`, CG 창45094, 720×700 기하와 AX 제목을 대조했다.
- AX에서 실제 Welcome의 폴더·추천·미리보기를 읽고 Codex 선택 후 명령 미리보기가 codex 경로로 바뀌는 것을 확인했다. inspect상 running=false이며 PTY를 실행하지 않았다. 창 위치·크기는 선택 전후 동일했다. 원래 foreground PID53662를 복원했다.
- `screencapture -x -l 45094`는 `could not create image from window`로 실패했다. **네이티브 색상·픽셀 검증 및 실제 앱 내 테마 전환은 미검증**이며 브라우저 통과로 대체 주장하지 않는다. 실제 OS 테마·사용자 저장 설정을 바꾸지 않았다.
- 약관 수락·진단 전송 동의를 만들지 않았고 diagnostics에서 terms_accepted=false, terms_skipped=true, sharing_allowed=false를 확인했다.
- Windows/Intel 실앱·release 설치·스크린 리더는 미검증. 원격 전체 플랫폼 결과는 해당 PR의 검증 SHA별 실행 기록을 따른다. 자동 병합과 독립 감사는 작업자 검증과 별도 단계다.

위험도 예상 Low: 프런트엔드의 테마 표시와 컨트롤 표현에 한정되며 저장 계약·실행 명령·네이티브 창 제어·진단 전송 경계는 변경하지 않았다. 최종 diff와 독립 감사로 재평가한다.
