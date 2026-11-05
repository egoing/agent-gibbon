# #57 터미널 Welcome 검증

## 출처와 기준

- 작업 루트: `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/57-terminal-welcome`
- 인스턴스: `dev-868cf91c545212324aa21ebf`
- 바이너리: 위 작업 루트의 `src-tauri/target/debug/agent-gibbon`
- base: `fb5c4f38c4183337514808d248d8234f2c45e596`, red: `cbd07472ede18ed97d45cf9f49475ea4b67fcf21`
- macOS ARM64, Node 24.21.0, npm 11.19.0, Rust/Cargo 1.91.1.
- 원본 로그: `/tmp/agent-gibbon-is2-57/`. 기존 원본 저장소의 사용자 변경은 이 작업에 포함하지 않는다.

## baseline / TDD

기준 트리의 Rust 94개·UI 53개, `cargo check --locked --manifest-path src-tauri/Cargo.toml --all-targets`, Rustfmt·Clippy, `npm run dev:build` 통과. baseline 제외 없음.

첫 baseline 테스트 컴파일 도중 테스트 초안을 추가한 것을 발견해 해당 결과를 `pre-red-test.log`로 분리했다. 테스트 초안을 제거하여 기준 트리와 diff가 없음을 확인한 뒤 baseline 테스트를 다시 실행했다. 재실행 결과만 baseline으로 사용한다. 이후 테스트만 추가해 2개의 assertion 실패를 `red.log`로 확인하고 별도 red 커밋·ref·Draft PR로 보존했다.

## is2 검증 명령

- 신규: `cargo test --locked --manifest-path src-tauri/Cargo.toml terminal::` 및 `node --test tests/terminal-welcome.test.mjs`
- 회귀: `cargo test --locked --manifest-path src-tauri/Cargo.toml` 및 `npm run test:ui`
- 타입: `cargo check --locked --manifest-path src-tauri/Cargo.toml --all-targets`, 변경 JavaScript의 `node --check`
- 린트: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`, `cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `git diff --check`
- 빌드: `npm run dev:build`; 배포 플랫폼 빌드는 GitHub Actions의 macOS ARM64·Intel 및 Windows x64 행렬에서 별도 확인.

## 구현 경계

기존 portable-pty·xterm·IME·리사이즈·로컬 통신을 재사용한다. 범용 셸은 기존 에이전트의 CLI 탐색·자동 시작과 별도 분기로 구분한다. 설치 감지는 파일 검사만 수행한다. 셸은 자체 초기화 파일을 읽을 수 있으나 앱은 명령·히스토리를 수집/저장하지 않는다.

macOS는 `proc_pidinfo(PROC_PIDVNODEPATHINFO)`로 셸의 현재 경로를 읽고, Unix PTY 전경 프로세스 그룹 및 자식 PID로 작업 여부를 판단한다. Windows는 PowerShell prompt의 제한된 경로 제어 시퀀스와 Toolhelp 프로세스 부모 관계를 사용한다. ConPTY에서는 Unix job-control API를 사용하지 않는다. 상태 조회 실패는 종료 확인으로 처리한다.

공식 API 근거: [portable-pty MasterPty](https://docs.rs/portable-pty/latest/portable_pty/trait.MasterPty.html), [Tauri Dialog](https://v2.tauri.app/plugin/dialog/).

## 실제 환경 / 진행 중

개발 CLI `inspect`의 `ok`, 인스턴스, 메모 ID를 확인했다. GUI 대상은 정확한 바이너리 경로의 PID 54693, 네이티브 제목 `Agent Gibbon [DEV 57-terminal-welcome · 868cf91c] — terminal-57001`, 창 ID 36135로 특정했다. 검증용 메타데이터는 앱 종료 후 새 ID로 만들었다. 이는 shortcut/영역 선택을 통한 신규 생성의 증거가 아니다.

최종 게이트·실환경·독립 감사 결과는 완료 후 아래에 기록한다.
