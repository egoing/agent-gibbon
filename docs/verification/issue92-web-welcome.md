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

커밋 승인 대기. red/attempts ref·PR은 아직 생성하지 않았다. is2 순서에 따라 테스트 red 커밋 이후 구현한다.

## 진행 표시 출처

정상 baseline 개발 빌드 이후 해당 워크트리의 `scripts/agent-gibbon-dev`만 사용했다. `list`는 앱 미실행으로 실패했고, 이후 고유 ID `web-welcome-is2-92-local`을 `open`하여 이 인스턴스를 시작했다. open/inspect의 `ok: true`, 인스턴스 ID 및 체크리스트 본문으로 통신 출처를 대조했다. GUI 조작/캡처는 수행하지 않았다. 다른 인스턴스의 기존 메모·프로세스는 변경하지 않았다.

## 남은 작업

- 테스트만 red 커밋·push, red ref 기록 및 Draft PR.
- Ctrl+Shift+4 영역 선택, 웹 Welcome, 외부 웹 창 및 CLI 구현.
- URL 검증과 외부 콘텐츠 특권 IPC 차단 검증.
- README·에이전트 스킬·[디자인 시스템](../design-system.md)과 SVG/PNG 갱신.
- 최종 5종 게이트, 실제 CLI/GUI 검증, 구현 이력 없는 독립 감사.
- 승인 후 구현 커밋·push와 `release.yml` 전체 플랫폼 검사.

현재 디자인 규칙과 시안은 변경하지 않았으며 새 UI는 미구현이다. 기존 Terminal Welcome 시안 SVG·PNG를 읽고 시각 확인했다. 커버리지 체크리스트를 대조했으며, 이번 red 단계만으로 새 파이프라인 시나리오를 검증 완료로 기록하지 않는다.
