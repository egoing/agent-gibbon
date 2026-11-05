# Sentry 진단

계측 코드는 오픈소스에 포함하지만 DSN과 인증 토큰, 메모·터미널 원문은 소스에 넣지 않는다. 전송용 공개 DSN은 빌드 환경에서만 주입하며 관리 API 토큰은 앱에 포함하지 않는다. 포크와 일반 개발 빌드는 기본적으로 전송하지 않는다.

## 실행 모드

| 빌드·실행 조건 | 오류 | 상세 진단 |
| --- | --- | --- |
| DSN 없는 소스 빌드 | 꺼짐 | 꺼짐 |
| 일반 debug 또는 테스트 | 꺼짐 | 꺼짐 |
| DSN 있는 공식 release | 기존 최소 오류 수집 | 기본 꺼짐 |
| DSN 포함 빌드 + 명시적 `AGENT_GIBBON_DIAGNOSTICS=1` | 켜짐 | 켜짐 |
| `AGENT_GIBBON_TELEMETRY=off` | 모두 꺼짐 | 모두 꺼짐 |

일반 사용자용 동의 설정 UI는 아직 없다. 설치·실행만으로 상세 진단을 활성화하지 않는다. 진단 모드는 운영자가 실행 환경에서 명시적으로 켜며 이번 세 개발환경 배포도 이 모드다. 수집 끄기는 해당 환경변수를 설정하고 앱을 완전히 재실행해야 적용된다. 이미 전송한 데이터는 이 설정으로 삭제되지 않는다.

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

- **Tracing / spans:** 시작, IPC 대기·명령, 창 생성, Markdown 렌더, 저장·복원, 터미널 시작·쓰기·출력·리사이즈의 Rust 구간을 측정한다. 동기 중첩 작업은 같은 trace의 자식 span이 된다.
- **WebView 측정:** 허용된 앱 명령의 왕복 지연·성공/실패 횟수, 클릭부터 두 animation frame까지의 시간, 타이머 지연을 숫자로 집계한다. 2초 구간별 최대 지연과 횟수이며 모든 개별 호출의 분포가 아니다. `measurement.origin`으로 Rust/WebView를 구분한다.
- **Long tasks / long animation frames:** 해당 WebView가 PerformanceObserver 타입을 지원할 때만 지속 시간을 기록한다. 스크립트 URL·attribution·DOM은 보내지 않는다. WebView2와 WKWebView의 지원 차이를 숨기지 않는다.
- **오류 진단:** Rust panic 스택·네이티브 image 식별자, WebView 오류 종류와 허용된 앱 JS 파일명·행·열을 보낸다. 오류 문구·Promise reason·소스 문맥·로컬 절대 경로·frame 변수는 제거한다.
- **Sentry Logs / breadcrumbs:** 느린 작업 및 앱 수명의 고정 작업명·시간을 수집한다. console/stdout/셸 로그를 연결하지 않는다.
- **Release health:** 상세 모드에서만 익명 세션의 시작·종료·오류 상태를 활성화한다. 버전+커밋, OS·CPU 아키텍처, production/development로 비교한다. 사용자·장치 식별자는 제거한다.
- **프리징 감시:** 독립 Rust 스레드가 네이티브 UI 응답과 실제 표시 중인 WebView heartbeat를 구분한다. 시작 30초 유예 후 12초 응답 중단을 감지하고, 중단/복구 시 에피소드별 한 번 보고한다. 네이티브가 멈추면 WebView 중복 보고는 억제한다. 잠자기·프로세스 전체 일시 정지와 숨긴 창의 타이머 지연은 별도로 처리한다. 네이티브 큐에는 확인 작업을 최대 한 개만 남긴다.
- **로컬 기록:** 상세 모드의 중단/복구 고정 이벤트를 앱 데이터의 `diagnostics/<instance>/recent.json`에 최대 64개 저장한다. 창 ID·메모·명령 없이 빌드·시각·유형·지연만 남긴다. UI 스레드에서 디스크에 쓰지 않고, 큐·파일 크기를 제한하며 원자적으로 교체한다. 끄더라도 기존 로컬 기록은 자동 삭제하지 않는다.

## 부하·개인정보 경계

### 단계별 trace/span 이름

운영 이름과 레이블은 국제화와 쿼리 재사용을 위해 고정된 영어 이름을 사용한다. 동적 메모 제목·ID·경로·실행 명령을 이름에 붙이지 않는다. 같은 Rust 동기 호출 흐름의 하위 구간은 부모 trace의 자식 span이며, 서로 다른 UI callback·thread·WebView 집계 사이에 자동으로 trace가 이어진다고 가정하지 않는다.

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

WebView 측정은 기존 2초 집계 경로로만 보내며, 상세 모드가 꺼지면 `gibbonMeasure` 자체를 설치하지 않는다. 숨긴 창의 측정과 임의 이름·무한값은 제외한다. 이 확장은 #19/#64의 단계별 계측 부분이며, 1·5·10개 창의 p50/p95 부하 기준과 Rust·WebView 함수별 로컬 프로파일링 완료를 의미하지 않는다.

진단 모드의 일반 trace 기본 샘플링은 100%, 높은 빈도의 snapshot/inspect/list/터미널 쓰기·출력·조회는 최대 1%다. `AGENT_GIBBON_TRACE_SAMPLE_RATE`를 0~1로 지정하면 일반 구간을 낮출 수 있다. 시작·probe는 상세 모드에서 항상 측정한다. 오류·진단 로그는 trace 샘플링과 별도로 제한한다: 분당 WebView 오류 20개, 작업 로그 120개, 중단/복구 알림 10개, probe 2개. 이는 프로세스별 한도이며 여러 앱의 합산 한도는 아니다. Sentry 측 요금·프로젝트 할당량도 별도로 적용된다.

전송 경계는 SDK의 `before_send`만으로 보호하지 않는다. 오류·transaction·로그·session envelope를 모두 허용 목록으로 재구성하고 알 수 없는 항목, 첨부파일·replay를 차단한다. 메모 본문·제목·상태·연결 경로, 명령/출력/입력 키, clipboard, DOM, 화면, 사용자명·장치명, IP 필드는 보내지 않는다. 네트워크 서버는 연결 IP 주소를 볼 수 있다.

세션 리플레이, 화면 녹화, 메모 첨부, 네이티브 메모리 덤프, 함수별 CPU sampling profiling은 이 Rust/Tauri 구성에 추가하지 않았다. 지원하지 않는 SDK 옵션을 켰다고 주장하지 않으며, OS 수준 교착·GPU 문제의 최종 원인은 별도 현장 도구가 필요할 수 있다. 강제 종료·오프라인·OS 자체 정지는 모든 이벤트의 보존/전송을 보장하지 않는다.

## 빌드와 운영

Actions의 `AGENT_GIBBON_SENTRY_DSN` secret/variable을 공식 release 빌드에서만 주입한다. 기본 수집 모드와 자세한 공지 내용은 [README](../README.md)의 Sentry 절을 따른다. 이번 변경은 공개 버전 태그나 설치 앱 자동 업데이트를 수행하지 않는다.

검증은 네트워크 없는 오류/trace/log/session 필터·모드 경계·watchdog 상태 검사와 실제 앱에서 명시적인 probe 수신 검사를 구분한다. 실제 수신 결과는 [검증 기록](verification/sentry-observability-20261106.md)에 남긴다.

공식 근거: [Sentry Rust SDK](https://docs.rs/sentry/0.42.0/sentry/), [ClientOptions](https://docs.rs/sentry-core/0.42.0/sentry_core/struct.ClientOptions.html), [Sentry envelope](https://develop.sentry.dev/sdk/envelopes/), [Sentry spans 조회](https://docs.sentry.io/api/explore/query-explore-events-in-table-format/).
