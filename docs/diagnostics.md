# Sentry 진단

계측 코드는 오픈소스에 포함하지만 DSN과 인증 토큰, 메모·터미널 원문은 소스에 넣지 않는다. 전송용 공개 DSN은 빌드 환경에서만 주입하며 관리 API 토큰은 앱에 포함하지 않는다. 포크와 일반 개발 빌드는 기본적으로 전송하지 않는다.

## 실행 모드

| 조건 | SDK와 외부 전송 |
| --- | --- |
| 동의 기록 없음·거부·손상·이전 정책 버전 | 초기화하지 않음, 전송 없음 |
| 포크/사용자 빌드의 운영 정보 중 미확정 항목이 있음 | 선택 동의 비활성, 전송 없음 |
| DSN·개발 진단 플래그만 존재 | 동의를 대체하지 않음, 전송 없음 |
| debug 개발 인스턴스의 약관 확인 생략 | 기능 진입만 허용; 실제 수락·별도 전송 동의가 없으면 SDK 초기화·전송 없음 |
| 현재 문서에 대한 약관 및 별도 전송 동의 + 확정된 운영 정보 + 유효 DSN | 허용된 진단만 전송; 개발 빌드는 명시적 진단 실행도 필요 |
| `AGENT_GIBBON_TELEMETRY=off` | 동의가 있어도 모두 차단 |

첫 실행의 필수 약관과 선택적 전송을 구분하며 전송은 기본 해제다. [약관·개인정보 안내](legal/README.md)과 관리 창의 Privacy & Diagnostics에서 범위를 확인한다. 정책 해시는 약관·개인정보 안내·운영 정보에 연결돼 문서가 달라지면 과거 동의로 전송하지 않는다. 로컬 동의 파일은 instance별 `privacy/<instance>/consent.json`이며 계정·장치 식별자를 만들지 않는다.

debug 개발 인스턴스는 약관 확인을 기본 생략하되 동의 파일을 만들거나 변경하지 않는다. `diagnostics.privacy`는 실제 `terms_accepted`와 `terms_skipped`·`onboarding_required`를 구분한다. 사용자가 약관 검증을 요청하면 앱 시작 전에 `AGENT_GIBBON_REQUIRE_TERMS=1`을 지정한다. 설정에서 최초 진단 전송을 저장하려는 경우에는 기존 약관·선택 동의 화면에서 명시적으로 확인한다. [개발 약관 검증 기록](verification/dev-terms-skip-20261106.md)에 실행 범위와 한계를 남긴다.

SDK 초기화 전과 최종 HTTP 요청 직전에 동의를 확인한다. 전송 큐는 최대 64개 envelope, 각 직렬화 payload는 최대 512KiB이고 HTTP 요청 제한 시간은 3초다. 철회는 새 요청을 차단하고 이미 시작한 요청의 종료를 기다린 뒤 완료되며, 이전 동의 세대의 queue/log/session은 다시 동의해도 재전송되지 않는다. SDK 종료 시 flush도 이 경계를 우회하지 않는다. 과거 서버 데이터와 로컬 자료는 자동 삭제되지 않는다.

현재 공식 운영 고지는 원본 최대 30일·별도 백업/오류 누적 통계 기준으로 확정되어 첫 실행과 설정에서 진단 동의를 선택할 수 있다. 기본값은 해제이며 DSN이나 개발 플래그만으로 전송하지 않는다. 재시작 없이 설정에서 철회할 수 있고, 진단 모드 변경 이벤트는 프런트 측정 타이머와 관찰자도 정리한다.

자신의 Sentry 프로젝트를 쓰는 공헌자는 `AGENT_GIBBON_SENTRY_DSN`을 **빌드 프로세스의 환경변수**로 제공하고 다음 명령을 실행한다. `.env`는 자동으로 읽지 않는다.

```sh
npm run diagnostics:build
npm run diagnostics:open -- --id diagnostic-check --markdown '진단용 테스트'
npm run diagnostics:status
npm run diagnostics:probe
```

공식 저장소 쓰기 권한이 있는 유지보수자는 다음 빌드 명령을 사용할 수 있다. 실행기는 GitHub 권한을 확인한 뒤 Actions variable을 프로세스 메모리로만 읽고 DSN을 출력하지 않는다. `--official` 없이 원격 공식 DSN을 자동으로 가져오지 않는다.

```sh
npm run diagnostics:build -- --official
```

Windows는 `npm.cmd`를 사용한다. `gh api`를 지원하는 인증된 GitHub CLI가 필요하다. 설치 앱이나 다른 개발 앱이 이미 실행 중이면 새 진단 환경이 기존 프로세스에 소급 적용되지 않는다. 대상 앱의 PID·바이너리 경로를 확인해 해당 앱만 재실행한다. 일반 `scripts/agent-gibbon-dev`는 계속 현재 워크트리의 debug 바이너리만 사용한다.

`diagnostics`는 현재 서버의 로컬 진단 상태를 읽는 CLI 명령이고, `diagnostics-probe`는 명시적인 확인용 고정 이벤트·span·로그를 보낸다. probe의 `queue_flushed:true`는 서버 수신 증거가 아니므로 Sentry에서 반환 event ID를 따로 조회해야 한다.

## 상세 진단 항목

- **Tracing / spans:** 시작, IPC 대기·명령, 창 생성, Markdown 렌더, 저장·복원, 터미널 시작·쓰기·출력·리사이즈의 Rust 구간을 측정한다. 동기 중첩 작업은 같은 trace의 자식 span이 된다. CLI의 `ipc.wait` 수신 스레드에서 UI callback으로는 Sentry 부모 handle을 명시적으로 전달하므로 `ipc.open/update/close/inspect/list`와 그 동기 하위 단계도 같은 trace에 포함된다.
- **WebView 측정:** 허용된 앱 명령의 왕복 지연·성공/실패 횟수, 클릭부터 두 animation frame까지의 시간, 타이머 지연을 숫자로 집계한다. 2초 구간별 최대 지연과 횟수이며 모든 개별 호출의 분포가 아니다. `measurement.origin`의 `rust` / `webview.aggregate`로 개별 Rust 실행과 WebView 집계를 구분한다. 개별 snapshot의 Rust root는 `snapshot.request`이며 집계 transaction `snapshot`과 다른 이름이다.
- **Long tasks / long animation frames:** 해당 WebView가 PerformanceObserver 타입을 지원할 때만 지속 시간을 기록한다. 스크립트 URL·attribution·DOM은 보내지 않는다. WebView2와 WKWebView의 지원 차이를 숨기지 않는다.
- **오류 진단:** Rust panic 스택·네이티브 image 식별자, WebView 오류 종류와 허용된 앱 JS 파일명·행·열을 보낸다. 오류 문구·Promise reason·소스 문맥·로컬 절대 경로·frame 변수는 제거한다.
- **Sentry Logs / breadcrumbs:** 느린 작업 및 앱 수명의 고정 작업명·시간을 수집한다. console/stdout/셸 로그를 연결하지 않는다.
- **Release health:** 세션 통계 보유기간 확인 전 비활성화한다. SDK 자동 세션을 끄고 세션·세션 집계 envelope도 차단하며 상태의 `release_health`는 항상 false다.
- **프리징 감시:** 독립 Rust 스레드가 네이티브 UI 응답과 실제 표시 중인 WebView heartbeat를 구분한다. 시작 30초 유예 후 12초 응답 중단을 감지하고, 중단/복구 시 에피소드별 한 번 보고한다. 네이티브가 멈추면 WebView 중복 보고는 억제한다. 잠자기·프로세스 전체 일시 정지와 숨긴 창의 타이머 지연은 별도로 처리한다. 네이티브 큐에는 확인 작업을 최대 한 개만 남긴다.
- **로컬 기록:** 상세 모드의 중단/복구 고정 이벤트를 앱 데이터의 `diagnostics/<instance>/recent.json`에 최대 64개 저장한다. 창 ID·메모·명령 없이 빌드·시각·유형·지연만 남긴다. UI 스레드에서 디스크에 쓰지 않고, 큐·파일 크기를 제한하며 원자적으로 교체한다. 끄더라도 기존 로컬 기록은 자동 삭제하지 않는다.

## 부하·개인정보 경계

### Windows 행 조사용 로컬 단계 기록 — #66

`AGENT_GIBBON_LOCAL_TRACE=1`을 **앱 시작 전** 설정하면 Sentry와 독립적인 로컬 단계 기록을 켠다. 기본은 꺼짐이며 외부 전송 동의·SDK·watchdog를 활성화하지 않는다. `AGENT_GIBBON_TELEMETRY=off`와 함께 사용할 수 있다. 이는 원인 확정이나 행 해결 표시가 아니다.

```powershell
$env:AGENT_GIBBON_LOCAL_TRACE = '1'
$env:AGENT_GIBBON_TELEMETRY = 'off'
.\scripts\agent-gibbon-dev.cmd open --id issue66-test --markdown 'LOCAL-TRACE-TEST'
.\scripts\agent-gibbon-dev.cmd diagnostics
powershell -NoProfile -File scripts/windows-hang-evidence.ps1
```

이미 실행 중인 앱에는 환경변수가 소급 적용되지 않는다. 기존 사용자 프로세스는 임의 종료하지 않는다. debug 개발 인스턴스의 기본 생략은 동의 기록을 만들지 않으며, 약관 검증 모드와 배포 앱에서는 사용자가 검토하고 점검 도구가 대신 동의하지 않는다.

- 파일: Tauri 앱 데이터의 `diagnostics/<instance>/local-stages.json`. Windows에서는 `%APPDATA%\org.egoing.floatnote\diagnostics\<instance>\local-stages.json`이다. 재시작 시 이전 실행 파일을 덮어쓰므로 멈춘 앱을 재시작하기 **전에** 사본을 남긴다.
- 내용: 실행 PID·시작 시각·빌드 표식, 실행 내 숫자 호출 번호, 고정 작업/단계 이름, 프로세스 시작 후 단조 증가 마이크로초만 기록한다. 창 ID·메모·제목·경로·명령·출력·화면·장치 식별자를 입력받지 않으며 Sentry 경로와 연결하지 않는다.
- 메모 `snapshot`: Rust command 진입(`dispatch`), `notes_wait`, `notes_copy`, 잠금 해제 후 `opacity`·`feedback`·`position`·`size`·`visibility`·`focus`·`pin`, `response`, `complete`/`aborted`를 기록한다. `opacity`와 `feedback`은 여러 하위 조회를 포함하며 단일 Win32 함수 이름을 뜻하지 않는다. `response`도 다른 상태 잠금과 JSON 값 생성을 포함한다. Tauri가 반환값을 직렬화·전달하는 시간은 포함하지 않는다.
- CLI: 고정 작업명 `ipc`로 UI 실행 큐 진입(`ui_queue`)·실행(`ui_execute`)·종료를 기록한다. 메모 `inspect`의 내부 조회는 별도 호출 번호로 기록한다. 이 큐·작업 관계는 아래 #64의 실행 내 부모 연결로 보강한다. 모든 비동기 callback에 자동 전파되지는 않는다.
- 완료되지 않은 호출은 최근 이력과 별도로 최대 64개 유지하고, 단계 이력은 최대 256개 유지한다. 기록은 `try_lock`으로 UI를 기다리게 하지 않으며 경합·용량 초과 횟수를 `lost`로 표시한다. `lost > 0`이면 시작·진행·완료가 누락되어 `active`가 불완전하거나 이미 끝난 호출이 남을 수 있다. 이 상태만으로 교착을 확정하지 않는다.
- 별도 worker가 최대 약 500ms 주기로 작은 스냅샷을 원자 교체한다. 애플리케이션 잠금이나 네이티브 창 API를 호출하지 않는다. 파일 I/O 지연·실패·프로세스 정지·강제 종료 때 마지막 단계 보존을 보장하지 않는다. 파일 부재·수정 시각·PID·시작 시각을 반드시 확인한다. 실제 CPU/디스크 부하의 p50/p95 검증은 아직 없다.
- `scripts/windows-hang-evidence.ps1`은 현재 워크트리 binary와 일치하는 단일 PID만 선택한다. 로컬 임시 폴더에 CPU 변화, 스레드 상태/대기 이유, HWND/TID/가시성·WM_NULL 응답을 저장한다. 창 제목·본문을 읽거나 포커스·배치를 바꾸지 않는다. 호출 스택·WebView2 자식 상태·메모리 덤프를 얻는 도구는 아니며, `Responding`/WM_NULL 성공만으로 WebView 렌더링 성공을 보장하지 않는다. 대상이 없거나 여러 개면 다른 앱으로 대체하지 않고 실패한다.

[현재 출처·검증·미해결 범위](verification/issue66-windows-hang.md)를 따른다.

### 단계별 trace/span 이름

운영 이름과 레이블은 국제화와 쿼리 재사용을 위해 고정된 영어 이름을 사용한다. 동적 메모 제목·ID·경로·실행 명령을 이름에 붙이지 않는다. 같은 Rust 동기 호출 흐름의 하위 구간은 부모 trace의 자식 span이며, IPC 수신 → UI callback 경계만 명시적 부모 handle로 연결한다. 다른 UI callback·thread·WebView 집계 사이에 자동으로 trace가 이어진다고 가정하지 않는다.

| 흐름 | 영어 작업 이름 | 해석 |
| --- | --- | --- |
| 영역 선택 준비 | `capture.prepare`, `capture.screen.enumerate`, `capture.screen.capture`, `capture.overlay.create`, `capture.preview.copy` | 모니터 열거, 화면 이미지 획득, Overlay WebView 생성, 프런트 전달용 픽셀 복사를 분리 |
| 선택 완료·정리 | `capture.finish`, `capture.lifecycle.wait`, `capture.selection.validate`, `capture.image.encode`, `capture.cleanup.request`, `capture.cleanup.wait` | 작업자 직렬화 대기, 좌표 검증, 이미지 모드의 PNG 인코딩, 파괴 요청·이벤트 대기를 구분 |
| 선택 취소·실패 | `capture.cancel`, `capture.failed` | 사용자의 취소와 초기화 실패 정리를 구분 |
| 터미널 창 | `terminal.window.open`, `terminal.window.create`, `terminal.window.position`, `terminal.window.show` | 새 WebView 생성, 모니터 좌표·DPI 배치, 표시를 분리; 기존 창 재열기는 생성 span이 없음 |
| 관리 창 | `manager.terminal.open`, `manager.terminal.delete` | 터미널 복원·삭제에서 시작한 흐름을 별도 root로 식별 |
| 세션 시작 | `terminal.start`, `terminal.command.prepare`, `terminal.discovery`, `terminal.discovery.resolve`, `terminal.pty.create`, `terminal.process.spawn`, `terminal.io.attach` | 명령 준비, 탐색 전체와 실제 resolve, PTY 할당, 자식 프로세스 spawn, reader/writer 연결을 분리 |
| 입력·출력 | `terminal.write`, `terminal.session.lock`, `terminal.input.write`, `terminal.output`, `terminal.output.filter`, `terminal.output.dispatch` | 세션 잠금 대기, 실제 PTY 쓰기·flush, 출력 제어 표식 처리, IPC 전달을 구분; 원문은 수집하지 않음 |
| 표시된 WebView의 초기 준비 | `terminal.ui.ready`, `terminal.first_output`, `terminal.output.parse` | 페이지 시작부터 Welcome/초기 세션 준비, 시작 요청부터 첫 출력 수신, xterm write callback까지의 파싱·큐 처리 시간. 실제 픽셀 페인트나 GPU 프로파일을 뜻하지 않음 |

`window.create`, `terminal.window.create`, `terminal.process.spawn`, `capture.cleanup.request`, `capture.cleanup.wait`에는 시작 시 고정 stage 로그를 남긴다. 구간이 영원히 끝나지 않아 span이 전송되지 않아도 마지막 시작 지점을 찾기 위한 로그이며 기존 분당 120개 한도 안에서만 전송한다. 현재 Rust trace가 있으면 structured log의 `trace_id`로 연결한다. 로그/trace 샘플링이 다르므로 모든 로그에 인덱싱된 trace가 반드시 존재하는 것은 아니다.

Sentry로 보내는 WebView 측정은 기존 2초 집계 경로를 유지하며, 상세 모드가 꺼지면 `gibbonMeasure` 자체를 설치하지 않는다. 숨긴 창의 측정과 임의 이름·무한값은 제외한다. 이 확장은 #19/#64의 단계별 계측 부분이며, 1·5·10개 창의 p50/p95 부하 기준과 Rust·WebView 함수별 로컬 프로파일링 완료를 의미하지 않는다.

진단 모드의 일반 trace 기본 샘플링은 100%, 높은 빈도의 snapshot/inspect/list/터미널 쓰기·출력·조회는 최대 1%다. `AGENT_GIBBON_TRACE_SAMPLE_RATE`를 0~1로 지정하면 일반 구간을 낮출 수 있다. 시작·probe는 상세 모드에서 항상 측정한다. 오류·진단 로그는 trace 샘플링과 별도로 제한한다: 분당 WebView 오류 20개, 작업 로그 120개, 중단/복구 알림 10개, probe 2개. 이는 프로세스별 한도이며 여러 앱의 합산 한도는 아니다. Sentry 측 요금·프로젝트 할당량도 별도로 적용된다.

전송 경계는 SDK의 `before_send`만으로 보호하지 않는다. 오류·transaction·로그 envelope를 허용 목록으로 재구성하고 session·session 집계와 알 수 없는 항목, 첨부파일·replay를 차단한다. 메모 본문·제목·상태·연결 경로, 명령/출력/입력 키, clipboard, DOM, 화면, 사용자명·장치명, IP 필드는 보내지 않는다. 네트워크 서버는 연결 IP 주소를 볼 수 있다.

세션 리플레이, 화면 녹화, 메모 첨부, 네이티브 메모리 덤프, 함수별 CPU sampling profiling은 이 Rust/Tauri 구성에 추가하지 않았다. 지원하지 않는 SDK 옵션을 켰다고 주장하지 않으며, OS 수준 교착·GPU 문제의 최종 원인은 별도 현장 도구가 필요할 수 있다. 강제 종료·오프라인·OS 자체 정지는 모든 이벤트의 보존/전송을 보장하지 않는다.

## 빌드와 운영

Actions의 `AGENT_GIBBON_SENTRY_DSN` secret/variable을 공식 release 빌드에서만 주입한다. 기본 수집 모드와 자세한 공지 내용은 [README](../README.md)의 Sentry 절을 따른다. 이번 변경은 공개 버전 태그나 설치 앱 자동 업데이트를 수행하지 않는다.

현재 동의·철회·loopback 전송 경계와 macOS 무전송 검증은 [동의 검증 기록](verification/privacy-consent-20261106.md)을 따른다. [이전 Sentry 수신 검증](verification/sentry-observability-20261106.md)은 동의 정책 변경 전의 기록이며, 현재 동의 정책 빌드의 실제 전송 근거로 사용하지 않는다.

공식 근거: [Sentry Rust SDK](https://docs.rs/sentry/0.42.0/sentry/), [ClientOptions](https://docs.rs/sentry-core/0.42.0/sentry_core/struct.ClientOptions.html), [Sentry envelope](https://develop.sentry.dev/sdk/envelopes/), [Sentry spans 조회](https://docs.sentry.io/api/explore/query-explore-events-in-table-format/).

## 동의 선택 활성화 — 2026-11-06

원본 30일·백업 생성 후 30/90일·오류 누적 횟수 별도 고지를 반영했다. 개인정보 안내 2026-11-06.3의 정책 해시 변경으로 과거 동의는 자동 승계하지 않는다. Release Health 세션은 SDK와 전송 경계에서 차단한다. 현재 검증은 [동의 활성화 기록](verification/privacy-choice-enabled-20261106.md)을 따른다.


### #64 로컬 흐름 연결과 개별 snapshot

`AGENT_GIBBON_LOCAL_TRACE=1`일 때만 `local-stages.json` format 2를 기록한다. 이 로컬 opt-in은 전송 동의와 독립적이며, 꺼져 있으면 새 WebView 흐름을 시작하거나 상관 ID를 붙이지 않는다. 기존 Sentry 상세 모드·동의·철회·전송 필터 경계는 유지한다. 로컬 단계/상관 ID를 heartbeat나 Sentry envelope에 넣지 않는다.

- `call`은 프로세스 실행 내 발급 번호, `parent`는 명시적으로 전달한 상위 실행 번호, `cause`는 snapshot이 복사한 메모의 마지막 CLI/저장 갱신 실행 번호다. 이름·창 ID·본문·경로·명령을 파일에 넣지 않는다. 메모의 갱신 번호는 메모리에서만 유지하고 저장 파일/일반 메모 JSON에는 직렬화하지 않는다. 중간 갱신을 건너뛴 snapshot은 실제 복사한 최신 갱신만 가리킨다.
- CLI의 `ipc.wait → ipc(UI queue/execute) → ipc.open/update/inspect → Markdown/저장/창 조회`를 연결한다. WebView의 `note.refresh → snapshot.request → 조회 단계`와 `note.save → save_note → Markdown/저장`, `webview.terminal.start → terminal.worker(queue/execute) → terminal.start → PTY/spawn/attach`도 연결한다. 터미널 reader는 각 출력 처리 시 시작 작업 번호를 명시적으로 받으며, reader의 대기 전체를 활성 span으로 유지하지 않는다.
- WebView flow 번호는 Rust가 발급하고 호출 창에만 사용 가능하다. 창 소유 표식은 검증을 위해 메모리에만 두며 파일에 직렬화하지 않는다. 개별 `note.refresh` flow는 창당 최대 2초에 한 번으로 제한한다. 나머지 snapshot의 기존 로컬 단계는 계속 남지만 부모 WebView flow가 없을 수 있다. 샘플링 밖에서 발생한 DOM 변경도 빠질 수 있다.
- 메모 `dom` / `measure`는 각각 HTML 대입과 복제 DOM의 콘텐츠 크기 측정에 걸린 WebView 단조 시계의 동기 경과 시간(`duration_us`)이다. `fit`은 Rust 요청 직전 표식이며 `window.fit`에서 네이티브 요청 처리를 구분한다. `frame_ready`는 두 rAF callback 준비까지의 경과 시간으로 픽셀 페인트/GPU 완료가 아니다. 프런트 단계의 `elapsed_us`는 Rust 수신 시각이므로 IPC 전달 지연을 포함한다. 두 시계를 하나의 정확한 CPU 시간선으로 해석하지 않는다.
- 기존 active 64개/history 256개·try_lock·lost·약 500ms flush 한도를 유지한다. WebView flow도 최대 64개, flow당 32회 단계, 60초 만료를 둔다. 종료·실패·만료 뒤 늦은 WebView 단계는 무시한다. 숨김/종료로 rAF가 오지 않으면 `expired`이며 실제 작업 실패나 hang 확정이 아니다. `lost`가 있으면 끝난 호출이 active에 남거나 부모 이력이 사라질 수 있다. buffer 회전과 강제 종료의 마지막 단계 보존을 보장하지 않는다.
- JSON 저장은 worker에서 버퍼링해 작은 토큰마다 파일 write를 호출하지 않는다. 실제 프로파일에서 관측한 recorder 비용에만 대응한 변경이며, 앱 전반의 속도 개선 수치나 recorder p50/p95를 주장하지 않는다.

Sentry 개별 Rust snapshot root `snapshot.request`는 기존 고빈도 작업과 같이 최대 1% 샘플링이다. 자식은 `note.state.notes_wait`, `notes_copy`, `opacity`, `feedback`, `position`, `size`, `visibility`, `focus`, `pin`, `response`다(각 이름의 앞에 `note.state.`). 잠금 획득 대기와 잠금 안 복사를 구분하고 네이티브 조회 전에 Notes 잠금을 해제한다. `opacity`/`feedback`은 각각 여러 관련 조회를 포함한다. `response`는 응답 Value 구성과 남은 상태 조회를 포함하며 Tauri의 최종 직렬화·IPC 전달은 포함하지 않는다. 개별 실행 자식은 같은 trace지만 WebView의 집계 transaction과 원격으로 이어지지는 않는다. 프로파일이나 샘플링 수치를 이유로 고빈도 전송을 100%로 올리지 않는다.

[대표 실행·로컬 프로파일링 절차와 현재 검증](verification/issue64-trace-continuity-20261106.md)을 따른다. 원격 수신, Windows Browser Profiling, WKWebView 함수/렌더링 프로파일은 각각 별도 실행 근거가 필요하다.

### IPC 스레드 부모 전달 — #64

- trace ID를 복사해 transaction을 재시작하지 않고 SDK 부모 handle의 직접 자식 span을 만든다. 샘플링 결정을 그대로 상속한다. inspect/list 요청은 `ipc.wait` 루트 생성부터 최대 1%를 적용하므로 자식 연결이 폴링 전송량을 높이지 않는다. 이 분류는 내부 sampler 입력이며 외부 필드로 추가하지 않는다.
- 부모 handle은 callback이 처리되거나 폐기될 때까지 조상을 유지한다. 수신 대기가 먼저 끝난 경우에도 자식이 끝난 뒤 transaction을 끝내며, callback 취소·큐 등록 실패 시 마지막 handle 해제로 정리한다. 영구 정지·프로세스 종료는 미완료 trace 손실 가능성이 있다.
- UI callback은 현재 스레드의 기존 문맥을 잠시 교체하고 정상 반환·오류·panic 시 복원한다. 동시 요청은 각각의 부모를 사용한다. 로컬 recorder의 scope/parent/cause ID는 전달 handle에 포함하지 않으며 전송 필터도 유지한다.
- handle은 기존 전송 gate의 동의 세대에 묶인다. 철회·재활성화 후 이전 handle로 span·작업 로그를 시작하거나 완료 전송하지 않는다. span 종료와 작업 로그는 시작 당시 SDK hub/client를 유지해 검사 직후 철회·재활성화가 경합해도 새 클라이언트로 갈아타지 않는다. 전송 직전의 기존 transport 세대 검사도 유지한다. 동의 전/상세 비활성 상태에서 캡처한 빈 문맥 역시 새 활성 세대의 루트로 승격하지 않는다.
- UI의 5초 실행 기한·6초 수신 제한과 snapshot 상태 읽기, 2초 WebView 집계는 유지한다. 터미널의 장기 output worker를 부모 수명에 묶거나 WebView 개별 요청 프로토콜을 신설하지 않는다. 다른 스레드 경계는 후속 범위다.
- 검증 근거와 미실행 범위는 [스레드 문맥 검증 기록](verification/sentry-thread-context-20261107.md)을 따른다. in-memory/loopback 테스트는 실제 Sentry 서버 수신 증거가 아니다.
