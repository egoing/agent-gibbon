# Sentry 관측 기능 확장 검증 — 2026-11-06

## 변경 범위와 모드

오류, 안전한 앱 JS 위치, 중첩 Rust span, WebView 지연 집계, 구조화 로그·breadcrumbs, 상세 모드의 release health, 네이티브/WebView 응답 감시, 원문 없는 최대 64개 로컬 중단/복구 기록을 추가했다. SDK 0.42의 실제 지원 기능을 사용하며 replay·첨부·메모리 덤프·함수별 CPU profiler는 추가하지 않았다. 정확한 조건과 항목은 [진단 안내](../diagnostics.md)를 따른다.

일반 debug/테스트와 DSN 없는 빌드는 전송하지 않는다. 공식 release의 기존 최소 오류 수집은 유지하며 상세 기능은 명시적 진단 모드에서만 켠다. 전체 끄기가 최우선이다. 공헌자는 자신의 빌드 DSN을 제공하며, 공식 DSN 조회는 `--official`을 명시하고 저장소 쓰기 권한을 가진 경우에만 허용한다. 일반 사용자용 동의 설정 UI는 이 작업에서 구현하지 않았다.

## MacBook 구현·검증

- root `/Users/egoinglee/dev/project/agent-gibbon`, instance `dev-a5210ae16e79437d38490c8f`, binary `/Users/egoinglee/dev/project/agent-gibbon/src-tauri/target/debug/agent-gibbon`, ARM64 / Rust 1.91.1 / Node 26.0.0.
- 작업 전 main을 `257d842`에서 `e80b9bd`로 fast-forward했다. 기존 `package-lock.json` 변경, 이전 미추적 조사 문서는 스테이징에서 제외한다.
- 최종 Rust 112개, UI 73개 통과. Rustfmt와 Clippy `-D warnings`, 진단 debug 빌드 통과. Rust 전체 병렬 검사 중 기존 탐색 검사 `login_then_interactive_preserve_codex_priority_over_current_claude`가 한 차례 실패했고, `--test-threads=1` 전체 검사는 통과했다. 불안정 이력을 숨기지 않는다.
- 오류·transaction·로그·session 및 첨부 차단을 envelope 직렬화 결과의 원문 표식 부재로 검증했다. 허용된 작업명·시간·함수 정보는 유지했다. 단위 검사는 실제 네트워크에 보내지 않는다.
- WebView 테스트는 읽기 전용 Tauri API, 반환값/거절 보존, 원문·경로·오류 메시지 제외, 숨김/잠자기 지연 제외, 최대 한 개의 진행 중 heartbeat를 확인했다. 공헌자 기본 빌드의 공식 DSN 자동 조회 금지, 잘못된 DSN 비출력을 확인했다.
- 같은 진단 바이너리를 전체 끄기·일반 개발·명시적 진단 모드로 각각 재실행했다. 앞의 두 모드는 SDK·logs·trace·release health·journal 모두 꺼지고 heartbeat 0이었다. 진단 모드는 모두 켜지고 heartbeat 220 이상, 프런트 오류 0이었다. 각 모드에서 테스트 메모 갱신·숨김 갱신·inspect instance 일치를 확인했다.
- 모드별 CLI update 5회의 중앙값은 전체 끄기 293ms, 일반 debug 215ms, 진단 모드의 값은 아래 임시 결과에 보존했다. 프로세스 생성·렌더링·기존 메모 복원에 영향을 받는 작은 표본이므로 계측 자체 오버헤드나 성능 개선 수치로 사용하지 않는다.
- 최종 네이티브 확인 PID 72650과 정확한 바이너리 경로를 연결했다. 메모 `sentry-observability-20261106`의 inspect instance·본문과 native window ID 37256 / 제목 `Agent Gibbon [DEV main · a5210ae1] — sentry-observability-20261106`을 대조했다. 실제 상단 고정 버튼을 해제·복원하고 CLI 상태를 확인했다. 사용자 포커스는 false로 유지됐다.
- 초기 실환경 수집이 읽기 전용 `core.invoke`에 대한 대입 오류를 JS 파일명·행으로 검출했다. 앱 소유 `gibbonInvoke`로 바꾸고 회귀 검사를 추가한 뒤 재실행했으며 프런트 오류 0과 실제 버튼 동작을 확인했다. 초기 오류 이벤트는 합성/개발 검증 과정의 이벤트다.

## Sentry 실제 수신

- 빌드 DSN은 기존 Sentry 프로젝트의 활성 client key에서 조회해 GitHub Actions variable로 설정했다. DSN·관리 토큰 값을 소스·로그에 쓰지 않았다. GitHub/SDK 관리 인증 정보는 Cargo/GUI 실행 환경에서도 제외한다.
- `diagnostics-probe`의 event ID `00853814547d411d8dec33f3cd61f1b3` 등은 로컬 큐 flush와 별도로 Sentry API로 조회해 수신을 확인한다.
- Sentry Explore의 **spans** 데이터셋에서 `app.startup`, `ipc.open`, `ipc.wait`, `storage.load`, `window.create`, `markdown.render`, `snapshot`, `diagnostic.probe` 등의 실제 span을 확인했다. 이 프로젝트는 transaction 항목을 수신한 뒤 span으로 인덱싱하므로 기존 transactions 조회만으로 수신 여부를 판단하지 않는다.
- ingest outcome 조회에서 transaction·span·log_item이 accepted임을 확인했다. structured logs 18개, spans 363개 인덱싱 등의 값은 특정 관측 시점의 확인값이며 전체/최종 개수로 보장하지 않는다.
- `queue_flushed:true`만으로 수신했다고 기록하지 않는다. 실제 watchdog 장애/복구 알고리즘은 논리 시간 단위 검사로 검증했으며 OS 교착·GPU 장애를 강제 유발해 덤프까지 분석한 것은 아니다.

## 증거와 한계

MacBook의 `/tmp/gibbon-sentry-*.log`, `gibbon-sentry-runtime-results.json`, `gibbon-sentry-native-state.json`, `gibbon-sentry-pin-toggle.json`, `gibbon-sentry-pin-restored.json`에 명령 결과가 있다. 이 파일들은 공개 저장소에 원문으로 추가하지 않는다. GUI 창의 출처는 위 PID·native title·메모 instance로 확인했다. 끄기 설정은 기존 서버 데이터/로컬 기록을 삭제하지 않으며 서버는 네트워크 연결 IP를 볼 수 있다.

## iMac·Windows 반영

구현 커밋 push 후 같은 SHA를 두 main에 반영하고, 빌드·재시작·inspect instance·진단 상태·Sentry 실제 수신을 확인해 이 절에 후속 기록한다. 현재 이 문서의 최초 커밋에서는 원격 결과를 통과로 기록하지 않는다.
