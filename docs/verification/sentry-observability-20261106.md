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
- 모드별 CLI update 5회의 중앙값은 전체 끄기 293ms, 일반 debug 215ms, 진단 모드 298ms였다. 프로세스 생성·렌더링·기존 메모 복원에 영향을 받는 작은 표본이므로 계측 자체 오버헤드나 성능 개선 수치로 사용하지 않는다.
- 커밋 전 네이티브 확인 PID 72650과 정확한 바이너리 경로를 연결했다. 메모 `sentry-observability-20261106`의 inspect instance·본문과 native window ID 37256 / 제목 `Agent Gibbon [DEV main · a5210ae1] — sentry-observability-20261106`을 대조했다. 실제 상단 고정 버튼을 해제·복원하고 CLI 상태를 확인했다. 해당 단계의 사용자 포커스는 false로 유지됐다. 커밋 후 재빌드한 최종 실행은 PID 93429, 위의 동일 binary 경로와 revision `9f7edf6b6c2e`를 다시 확인했으며, 앞 단계 GUI PID를 최종 프로세스로 간주하지 않는다.
- 초기 실환경 수집이 읽기 전용 `core.invoke`에 대한 대입 오류를 JS 파일명·행으로 검출했다. 앱 소유 `gibbonInvoke`로 바꾸고 회귀 검사를 추가한 뒤 재실행했으며 프런트 오류 0과 실제 버튼 동작을 확인했다. 초기 오류 이벤트는 합성/개발 검증 과정의 이벤트다.
- 이후 최종 실행에서 2026-11-06 01:37:24~01:37:43 KST에 별도 WebView 오류 20건이 수집됐다. Sentry API의 플랫폼 태그가 MacBook(macOS/aarch64)을 가리키며, 안전한 이벤트에는 `WebView uncaught error`만 있고 허용된 JS 파일·행은 없었다. 최종 상태의 누적 `frontend_errors:20`을 숨기지 않는다. 같은 시각의 snapshot span은 확인했으나 인과관계·원인·계측 도입 전 발생 여부는 확인하지 못했다. 따라서 MacBook을 오류 없는 상태로 판정하거나 기존 Windows 프리징의 원인이 해결됐다고 주장하지 않는다.

## Sentry 실제 수신

- 빌드 DSN은 기존 Sentry 프로젝트의 활성 client key에서 조회해 GitHub Actions variable로 설정했다. DSN·관리 토큰 값을 소스·로그에 쓰지 않았다. GitHub/SDK 관리 인증 정보는 Cargo/GUI 실행 환경에서도 제외한다.
- 커밋 전 `diagnostics-probe`의 event ID `00853814547d411d8dec33f3cd61f1b3`와 아래 세 기기의 이벤트를 로컬 큐 flush와 별도로 Sentry API로 조회해 수신을 확인했다. 모두 release `agent-gibbon@0.2.7+9f7edf6b6c2e`, environment `development`다.

| 실행 환경 | 아키텍처 | 실제 수신한 probe event ID |
| --- | --- | --- |
| MacBook | macOS / aarch64 | `ba369cb5878a4d7e944a70b49c7a7a93` |
| iMac | macOS / x86_64 | `2af49b9cf98f40519e909b4ba90c2a9b` |
| Windows | Windows / x86_64 | `24271c2024c4475d90ca0732642c7201` |
- Sentry Explore의 **spans** 데이터셋에서 `app.startup`, `ipc.open`, `ipc.wait`, `storage.load`, `window.create`, `markdown.render`, `snapshot`, `diagnostic.probe` 등의 실제 span을 확인했다. 이 프로젝트는 transaction 항목을 수신한 뒤 span으로 인덱싱하므로 기존 transactions 조회만으로 수신 여부를 판단하지 않는다.
- ingest outcome 조회에서 transaction·span·log_item이 accepted임을 확인했다. structured logs 18개, spans 363개 인덱싱 등의 값은 특정 관측 시점의 확인값이며 전체/최종 개수로 보장하지 않는다.
- `queue_flushed:true`만으로 수신했다고 기록하지 않는다. 실제 watchdog 장애/복구 알고리즘은 논리 시간 단위 검사로 검증했으며 OS 교착·GPU 장애를 강제 유발해 덤프까지 분석한 것은 아니다.

## 증거와 한계

MacBook의 `/tmp/gibbon-sentry-*.log`, `gibbon-sentry-runtime-results.json`, `gibbon-sentry-native-state.json`, `gibbon-sentry-pin-toggle.json`, `gibbon-sentry-pin-restored.json`에 명령 결과가 있다. 이 파일들은 공개 저장소에 원문으로 추가하지 않는다. GUI 창의 출처는 위 PID·native title·메모 instance로 확인했다. 끄기 설정은 기존 서버 데이터/로컬 기록을 삭제하지 않으며 서버는 네트워크 연결 IP를 볼 수 있다.

## iMac·Windows 반영

구현 커밋 `9f7edf6b6c2ed29d20e3c9a2b70bf59ea5c5d43b`를 main에 push한 뒤 두 기기에서 pull·진단 빌드·재실행했다. 이후 검증 문서만 바꾸는 커밋은 앱 코드 변경을 포함하지 않으므로 아래 실행 바이너리의 revision은 구현 커밋을 유지한다.

| 항목 | iMac Intel | Windows x64 |
| --- | --- | --- |
| 작업 root | `/Users/egoing/dev/project/agent-gibbon` | `C:\Users\egoin\dev\project\agent-gibbon` |
| instance | `dev-e467b83ad9082909fc66aba7` | `dev-fbc7d562e05aec4798036a95` |
| binary | 위 root의 `src-tauri/target/debug/agent-gibbon` | 위 root의 `src-tauri\target\debug\agent-gibbon.exe` |
| 최종 PID | `35487` | `6436` |
| 검증 메모 ID | `sentry-peer-imac-20261106` | `sentry-peer-windows-20261106` |
| Rust 검사 | SSH 로그인 환경에서 112개 통과 | 97개 통과 |
| UI 검사 | 73개 통과 | 72개 통과, 1개 건너뜀 |
| Clippy·진단 빌드 | 통과 | 통과 |
| 최종 inspect·진단 상태 | instance 일치, visible true, detailed true, frontend_errors 0, heartbeat 1259 | instance 일치, visible true, detailed true, frontend_errors 0, heartbeat 3276, Responding true |

iMac의 Orca 터미널에서는 Cargo PATH를 보완한 뒤 기존 탐색 검사 `zshrc_only_path_with_missing_inherited_path`가 시간 초과로 두 번 실패했다(111개 통과). 같은 검사의 단독 SSH 실행과 전체 SSH 로그인 실행은 통과했다. 환경 의존 실패를 전체 검사 통과 기록에서 제외하거나 숨기지 않는다. GUI 실행은 접근성 권한이 적용되는 Orca 터미널에서 수행했다.

iMac은 정확한 PID·binary 경로와 native window ID `17071`, 제목 `Agent Gibbon [DEV main · e467b83a] — sentry-peer-imac-20261106`, inspect의 instance·본문을 연결했다. `orca computer get-app-state`의 최신 인덱스로 상단 고정 버튼을 눌러 `always_on_top:false`가 되는 것을 확인하고 다시 true로 복원했다. 백그라운드 클릭은 `background_unavailable`로 거부되어 사전 안내 후 `--restore-window`로 앞으로 가져왔다. 이 단계는 foreground 입력이므로 검증 메모에 포커스가 갔다. 실제 버튼 반응을 확인한 것이며 모든 기능의 GUI 검사를 완료한 것은 아니다.

Windows는 Orca의 원격 desktop-control 권한이 없어 직접 GUI 조작·캡처를 수행하지 않았다. 정확한 프로세스 경로·PID, inspect 본문 표식 `WINDOWS-SENTRY-DIAGNOSTIC-20261106`, 표시 상태와 heartbeat를 확인했으며 실제 버튼 반응의 사용자 확인은 별도다. 셸을 이용한 데스크톱 제어로 권한 제한을 우회하지 않았다.

초기 두 기기의 메모가 `visible:false`였을 때 iMac의 잠금 플래그와 Windows LogonUI가 확인됐고 사용자도 화면 잠금을 확인했다. 사용자가 두 기기의 잠금을 해제한 뒤 같은 진단 앱에서 메모를 다시 열자 모두 `visible:true`가 됐다. 이 현상을 SDK로 인한 프리징으로 판정하지 않는다. 기존 Windows 프리징은 별도 원인 미확정 상태다.

원격 명령 증거는 iMac `/tmp/gibbon-sentry-deploy-20261106/`의 `rust-tests-ssh.log`, `rust-tests.log`, `ui-tests.log`, `clippy.log`, `build.log`, `inspect-unlocked.json`, `status-unlocked.json`, `probe.log`와 Windows `%TEMP%\gibbon-sentry-deploy-20261106\`의 검사·빌드 로그, `inspect-unlocked.json`에 남겼다. iMac GUI 결과는 MacBook `/tmp/gibbon-sentry-imac-current.json`, `gibbon-sentry-imac-pin-toggle.json`, `gibbon-sentry-imac-pin-restored.json`에 보관했다. 원문 로그·기기 데이터는 저장소에 추가하지 않는다.

## CI와 배포 범위

[GitHub Actions 42062289910](https://github.com/egoing/agent-gibbon/actions/runs/42062289910)의 validate와 macOS Intel·ARM64, Windows x64 검사·설치 파일 빌드가 모두 성공했다. 일반 main push이므로 publish는 건너뛰었다. 공개 릴리스나 버전 태그는 생성하지 않았다. 이번 반영 대상은 세 기기의 개발 앱이며, 일반 사용자용 상세 수집 동의 UI·프로덕션 동의 상태 저장은 포함하지 않는다.
