# #118 업데이트 복원 검증

## 구현

- 업데이트 직전의 창 스냅샷에 외부 웹 창의 현재 HTTP(S) URL과 터미널의 고정 선택 ID만 포함한다. 터미널 입력 문자열이나 명령 히스토리는 저장하지 않는다. 이 값은 앱 데이터 디렉터리의 로컬 복원 파일에만 기록하고 진단 이벤트에는 전달하지 않는다.
- 복원 파일의 URL·선택 ID를 창 종류에 연결해 검증한다. 외부 페이지 주소는 기존 URL 정책을 통과해야 하고, 터미널 선택 ID는 `shell`, `codex`, `claude`, `git`, `htop`만 허용한다.
- 웹 Welcome/페이지 창을 기존 세션 ID로 숨긴 채 재생성하고 URL·위치·크기·핀·가시성을 복원한다. 터미널은 PTY를 재실행하지 않고 Welcome을 숨긴 상태로 준비한다. 페이지 로드 전에 재시작 안내를 등록해 선택 항목과 이전 PTY 종료 사실을 표시하며, 안내는 한 번만 소비한다.
- 스냅샷 저장 경로는 상태 기록만 수행한다. 실제 종료 전 셸을 종료하거나 웹 창을 닫지 않는다. #116에서 사용할 `check_for_restart`, `save_for_restart`, `review_pid` 인터페이스를 제공한다.

## 검증

- 작업 루트: `/Users/egoing/.codex/worktrees/102-windows-ui/agent-gibbon`; 브랜치 `codex/102-windows-ui`; 검증 시 HEAD `60907d48d4d0187b6e7d859752ccfad338c85c57` (#119 커밋 포함, #118 변경은 작업 트리).
- 환경: Intel macOS 15.6 (`x86_64`), Rust/Cargo 1.97.1, Node 24.21.0, npm 11.19.0, Tauri CLI 2.12.1. Windows Rust target은 설치되지 않아 Windows 컴파일을 로컬에서 실행하지 않았다.
- `npm test` — Rust 단위 테스트 212개와 UI 테스트 180개 통과.
- #118 핵심 회귀 테스트 — 터미널 재시작 안내/고정 ID 검사와 웹 URL·터미널 메타데이터 검증 통과.
- `npm run check` — Clippy `-D warnings` 통과.
- `npm run dev:build` — 해당 워크트리 절대 경로의 debug 개발 앱 빌드 통과. `npm run dev:info`의 루트·ID·바이너리와 `scripts/agent-gibbon-dev inspect --id task-118` 응답의 인스턴스 ID가 일치함을 확인했다.
- `docs/design/terminal-welcome.svg`를 `rsvg-convert`로 PNG 렌더링하고 선택 안내가 포함된 흰색·회색·검정 시안을 확인했다.

## 미검증 범위

- Windows 설치·실행 및 GUI, 실제 업데이트 설치 후 외부 웹 창 두 개와 터미널 두 개를 재시작해 URL·창 대응·미실행 안내를 확인하는 실기기 시나리오는 수행하지 않았다. 개발 빌드는 macOS Intel에서만 확인했다.
- #116의 업데이트 안내 UI와 확인 흐름은 별도 작업 브랜치에 있다. 이 기록은 해당 흐름이 호출하는 복원 API와 구현 상태에 한정한다.
