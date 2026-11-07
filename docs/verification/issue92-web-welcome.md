# #92 웹 Welcome — 진행 기록

## 출처와 범위

- 작업 루트: `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/92-web-welcome`
- 브랜치: `92-web-welcome`, base ref: `refs/is2/base/92`
- 최초 기준 SHA: `962301168cbac037310f47ca8e13fbf0bae646a8` (기존 ref 보존)
- 개발 인스턴스: `dev-70d4bfcea466ab2bbe492b65`
- 개발 바이너리: 해당 루트의 `src-tauri/target/debug/agent-gibbon`
- 환경: macOS ARM64, Node v24.21.0, npm 11.19.0, Rust/Cargo 1.91.1.
- 원문 및 확정 범위: [이슈 #92](https://github.com/egoing/agent-gibbon/issues/92).
- 위험도: Medium. 외부 웹 콘텐츠와 창 상태 관리. 자동 병합 보류.

## 구현 전 baseline (2026-11-07)

변경 없는 기준 트리에서 순서대로 실행했다. 원본 로그와 명령·종료 코드는 [baseline 폴더](issue92-web-welcome/baseline)에 보존한다.

| 항목 | 명령 | 결과 |
|---|---|---|
| 회귀 | `npm test` | PASS: Rust 182개, UI 120개 |
| 타입 | `cargo check --locked --manifest-path src-tauri/Cargo.toml --all-targets` | PASS |
| 린트 | `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` 및 `cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | PASS |
| 빌드 | `npm run dev:build` | PASS |

baseline 제외 0건. 개발 바이너리 빌드이며 배포 설치 파일/전체 플랫폼 검사와 구분한다.

## TDD red

`src-tauri/src/main.rs`의 기존 테스트 모듈에 테스트만 추가했다. 신규 production 구현은 없다.

- URL 없는 `web --id web-review` Welcome 요청.
- `web --id web-review --url URL`: HTTPS/HTTP와 loopback 개발 페이지.
- 위험 스킴·내부 origin·자격증명·제어문자 URL의 연결 이전 거절.
- 메모 내용 옵션·중복 URL·메모/터미널 ID 충돌 거절.

명령: `cargo test --locked --manifest-path src-tauri/Cargo.toml tests::web_cli_ -- --nocapture`.
실행 결과: exit 101, 1 PASS / 3 FAIL. `web` 명령 부재로 유효 요청이 usage 오류를 반환하며 URL 검증 오류도 제공하지 않아 실패한다. 컴파일/설정 실패가 아니다. [원본 red 로그](issue92-web-welcome/red.log), [명령·종료 코드](issue92-web-welcome/red.json).

초기 red 단계에서는 커밋 승인을 기다렸다. 이후 사용자 승인으로 테스트만 `6e7766f4db4db405289c3e19b81bcc4664acce5b`에 커밋하고 `refs/is2/red/92`에 기록해 push했다. 조사·로그는 `45fe5e5`에 별도 커밋했다. [Draft PR #96](https://github.com/egoing/agent-gibbon/pull/96)을 생성했다. attempts ref는 아직 미생성이다.

## 진행 표시 출처

정상 baseline 개발 빌드 이후 해당 워크트리의 `scripts/agent-gibbon-dev`만 사용했다. `list`는 앱 미실행으로 실패했고, 이후 고유 ID `web-welcome-is2-92-local`을 `open`하여 이 인스턴스를 시작했다. open/inspect의 `ok: true`, 인스턴스 ID 및 체크리스트 본문으로 통신 출처를 대조했다. 이 초기 단계에서는 GUI 조작/캡처를 수행하지 않았다. 이후 GUI 출처는 아래 최종 기록을 따른다. 다른 인스턴스의 기존 메모·프로세스는 변경하지 않았다.

## 최종 구현과 검증

- Ctrl+Shift+4 및 트레이의 영역 웹 창을 추가했다. 기존 1/2/3 단축키는 유지한다.
- 로컬 Welcome URL 입력·명시적 실행, 별도 외부 WebView, `web [--id web-ID] [--url URL]`, `open` 복원·`close` 숨김·`inspect`/`list` 조회를 구현했다.
- 외부 창 label의 앱 invoke를 명시적으로 거절하며 capability를 부여하지 않는다. HTTP(S)·자격증명·제어문자·내부 origin 정책은 입력과 이후 탐색 모두에 적용한다. 팝업·다운로드는 차단한다.
- 웹 창은 세션만 유지하며 새 서버·주소 이력 저장·의존성 추가는 없다. 일반 메모와 충돌하는 ID는 거절한다.
- README·기본 에이전트 스킬·[디자인 시스템](../design-system.md#web-welcome--92-2026-11-07)·[SVG](../design/web-welcome.svg)·[PNG](../design/web-welcome.png)를 갱신했다. SVG→PNG 렌더와 시각 확인을 완료했다.

### 수정 후 로컬 게이트

[최종 원본 로그와 명령·종료 코드](issue92-web-welcome/final)는 macOS 스레드 충돌 수정 이후에 실행했다. `implementation/` 로그는 초기 구현 시점이며 최종 증거로 대체하지 않는다.

| 항목 | 결과 |
|---|---|
| 신규 테스트 | RED Rust 4개 green, URL 정책 1개, UI 제출/실패 재시도 2개 PASS |
| 전체 회귀 | Rust 187개, UI 122개 PASS |
| 타입 | cargo check --locked --all-targets PASS |
| 린트 | Rustfmt 및 Clippy -D warnings PASS |
| 빌드 | npm run dev:build PASS |

baseline 제외 0건. 배포 설치 파일·전체 플랫폼 CI는 아직 실행하지 않았다.

### 브라우저

`PUPPETEER_MODULE=/tmp/gibbon-issue77/tools/node_modules/puppeteer-core/lib/puppeteer/puppeteer-core.js WEB_OUTPUT=docs/verification/issue92-web-welcome/browser node scripts/web-welcome-smoke.mjs`.

실제 자산의 720×460/320×240/240×160에서 버튼·Enter 제출, 오류 후 입력 보존·재시도, 가로 넘침 없음을 확인했다. 서버 없이 요청 interception을 사용하고 IPC는 mock이다. [보고서와 캡처](issue92-web-welcome/browser). 초기 스모크 도구의 favicon 처리 누락과 Puppeteer locator API 오용을 수정한 뒤 실행한 결과이며 제품 실패로 분류하지 않는다.

### 네이티브 macOS ARM64

[출처 JSON](issue92-web-welcome/native/provenance.json): 개발 root와 instance는 위와 같고, PID 32277의 실제 text executable을 `lsof -n -a -p 32277 -d txt -Fn`으로 대조했다. 앱은 이 워크트리 개발 CLI로 시작했다. 다른 인스턴스를 종료하지 않았다.

- 초기 GUI 실행에서 PID 26365/28251이 SIGTRAP으로 종료됐다. OS 진단은 tokio worker의 `NSWindow::orderFrontRegardless`에 대해 `Must only be used from the main thread`를 기록했다. [개인정보 없는 원인 요약](issue92-web-welcome/native/first-native-failure.json).
- `web_run`은 async command로 유지하고 창 생성·표시·포커스·이전 창 파괴 전체를 main으로 dispatch해 수정했다. 수정 후 PID 32277의 Welcome 창 45261에서 URL을 입력하고 실행 버튼을 눌렀다. 동일 ID `web-92-fixed`의 외부 창 45292에서 Example Domain 본문을 실제 렌더링했다. [AX/창 출처 원본](issue92-web-welcome/native/welcome-run-state.json)·[캡처](issue92-web-welcome/native/welcome-run-result.png)·[inspect](issue92-web-welcome/native/web-92-fixed-inspect.json).
- 배경 URL 입력은 값이 반영되지 않아 미검증으로 처리했다. 시스템 알림이 전경을 차지한 한 차례 입력은 도구가 거절했다. 이후 알림이 사라진 상태에서 명시적 foreground 입력과 AX 실행 뒤 inspect·새 창 본문으로 실제 결과를 확인했다. 도구 호출 수락만으로 성공 판정하지 않았다.
- [CLI 스모크 원본](issue92-web-welcome/native/cli-smoke.json): remote URL 실행, close→web 복원, 페이지·위치·크기 유지와 비포커스 상태, 위험 URL 거절 후 현재 페이지 보존을 확인했다.
- 합성 Ctrl+Shift+4 입력 후 같은 PID의 `웹 창 영역 선택 — Escape로 취소` 오버레이 45304/45305/45306이 세 모니터에 나타났다. 이후 선택으로 `web-1794033707011305000` Welcome이 생성된 것은 [inspect](issue92-web-welcome/native/web-1794033707011305000-inspect.json)로 확인했다. 합성 드래그의 기대 좌표와 결과 좌표가 달라 정확한 좌표·모니터/DPI 종단 검증은 완료로 판정하지 않는다. 실제 물리 키와 Escape 취소는 별도 미검증이다.
- 외부 공격 페이지 검증용 `tests/fixtures/web-ipc-probe.html`을 준비했으나 외부 echo 응답이 text/plain이어서 스크립트가 실행되지 않았다. 이를 IPC 차단 통과로 보고하지 않는다. 이후 응답 형식 설정 시도도 HTTP 400으로 실패했다. 런타임 공격 검증은 미완료이며 URL 단위 테스트·명시적 invoke 거부 코드·capability 및 독립 검토 결과와 구분한다. 새 서버는 실행하지 않았다.

### 독립 감사

구현 대화를 상속하지 않은 별도 검토자 `audit_web`가 최초 base와 red부터 전체 diff, 테스트, 최종 로그 및 native AX/PNG를 읽었다. RED 4개와 JS 2개도 직접 재실행했다.

- 테스트 완화: 미발동. RED·기존 테스트 삭제/skip/assertion 완화 없음.
- 기존 동작 퇴행: 미발동. baseline Rust182/UI120 → 최종 Rust187/UI122 통과. 초기 신규 기능 GUI 충돌과 구분한다.
- 스코프 이탈: 미발동. 앱·CLI·문서·시안·검증에 한정.
- 도움말 불일치와 Welcome의 임의 HTTP localhost origin 허용 지적을 반영했다. 새 확정 차단 결함은 발견하지 못했다.

## 현재 상태와 남은 작업

조사와 red 커밋 이후 구현을 검증했다. 2026-11-07 close 요청으로 구현 커밋·push와 PR 마무리가 승인되었다.

- 구현 커밋을 PR에 반영한 뒤 `release.yml --ref 92-web-welcome` 전체 플랫폼 검사.
- 후속 #98: 실제 외부 공격 스크립트의 IPC 시도.
- 후속 #99: Windows·Intel 실제 GUI, 물리 단축키/정확한 영역 좌표·DPI 및 Escape.
- Medium 위험도로 자동 병합하지 않는다. 워크트리와 base/red ref를 보존한다.

is2 커버리지 체크리스트를 대조했다. 이번 실행은 아직 커밋·전체 플랫폼 검사 및 최종 PR 제출까지 완료되지 않았으므로 새 파이프라인 시나리오의 검증 완료로 공용 스킬 기록을 변경하지 않는다.

## 최신 main 통합 — 2026-11-07

main의 자동 업데이트와 터미널 변경을 통합하면서 `main.rs`에서 웹 창과 업데이트 안내 창의 닫기 처리를 모두 보존했다. 통합 후 Rust 205개·UI 169개, Rustfmt·Clippy 및 개발 빌드를 통과했다. 기존 코드의 독립 변경을 덮어쓰지 않았다.

새 업데이트 준비의 전체 창 응답 요구와 웹 창의 세션 전용 수명이 충돌한다. 웹 창이 보이거나 숨겨진 동안 설치를 사전에 보류하고, 재시작하면 웹 세션이 끝난다는 안내를 추가했다. 앱 IPC를 외부 콘텐츠에 부여하지 않는다. 이 분기의 단위 테스트는 통과했으며 실제 업데이트 UI·설치 경로는 [후속 #100](https://github.com/egoing/agent-gibbon/issues/100)에서 검증한다.

통합 커밋의 전체 플랫폼 CI와 실제 실행 결과는 PR과 이슈 최종 기록을 따른다. 이 절은 이전 독립 앱 검증의 대체 실행 증거가 아니다.
