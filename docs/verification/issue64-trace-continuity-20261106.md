# #64 Trace 연결·snapshot 세분화·로컬 프로파일 검증

이 문서는 구현·로컬 검증 시점의 기록이다. 2026-11-07 사용자가 커밋·병합·push·워크스페이스 삭제를 승인했으며, 이후 통합 SHA와 전체 플랫폼 검사 결과는 연결된 PR 및 #64 진행 상태에 기록한다.

## 결과와 남은 범위

- [x] 실행 내 숫자 부모/갱신 관계로 CLI UI 큐, Rust 동기 하위 작업, WebView의 제한된 refresh/save 및 terminal worker 경로를 연결했다.
- [x] 메모 DOM 갱신·콘텐츠 측정·fit 요청·두 rAF 준비를 구분했다. 일반 저장 완료를 계측 때문에 fit/rAF 뒤로 미루지 않는다.
- [x] Sentry의 개별 `snapshot.request`와 자식 조회 span을 추가했다. 집계 `snapshot` / `measurement.origin=webview.aggregate`와 구분하고 고빈도 샘플링 최대 1%를 유지한다.
- [x] bounded/nonblocking recorder·창 소유 검증·미완료 만료·중복/늦은 단계·동시 스레드·상세 모드/동의/전송 경계 검사를 수행했다.
- [x] 이 워크스페이스의 debug native 앱에서 CLI 열기·갱신·inspect·숨김, 실제 WebView 표시와 로컬 부모·단계 관계를 확인했다.
- [x] 실제 Rust 프로세스에 sample/Time Profiler를 붙이고 함수·스레드 호출 트리를 확인했다. 관측한 recorder 저장 비용에만 버퍼링을 적용했다.
- [ ] WKWebView JavaScript & Events / Layout & Rendering 타임라인 수집, native GUI 저장 완료·터미널 시작, 장시간·다중 창 부하 p50/p95.
- [ ] Windows WebView2 Browser Profiling 및 Intel/Windows 전체 플랫폼 검사, 이번 span의 실제 Sentry 수신.
- [ ] 커밋·push·전체 플랫폼 Actions 실행. 사용자 커밋 승인 전이므로 실행하지 않았다. 이슈를 닫지 않는다.

## 환경·출처

| 항목 | 실제 값 |
| --- | --- |
| root | `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/issue-64-trace-continuity` |
| 브랜치 / 기반 HEAD | `egoing/issue-64-trace-continuity` / `f8b8f78b6e3ade70de2b1cbadc124da69fc5de6d` + 이 작업의 미커밋 변경 |
| instance | `dev-ab314a33d1bc08f8900168c2` |
| binary | 위 root의 `src-tauri/target/debug/agent-gibbon` |
| 최신 실행 PID / binary SHA-256 | `39139` / `e08fec38ac32175e97d8dc55a731b8934846fe749b18187bcd11081d8697672c` |
| 이전 실행 | PID `18502`; 버퍼링·snapshot 자식 span 추가 이전. 최신 실행 근거와 구분한다. |
| OS / CPU | macOS 15.6.1 (24G90), Apple M1 Pro, arm64 |
| 도구 | Node 24.21.0, npm 11.19.0, rustc/cargo 1.91.1, xctrace 26.0 (17C529) |
| 화면 | 주 화면 LG HDR 4K 4096×2304, 논리 2048×1152, 60Hz; 다중 화면 연결 유지 |
| 빌드 | `npm run dev:build`; unoptimized dev + debug symbols, 버전 0.2.7 |
| 메모 | `issue64-progress`(계획), `issue64-scenario`(검증 전용), 본문 표식 `ISSUE64-TRACE-20261106` |

`npm run dev:info`의 root/id/binary, 정확한 경로의 `ps` PID와 CLI `inspect`의 `ok:true`·instance·메모 ID·본문을 대조했다. 최신 실행은 최소 환경으로 개발 CLI를 시작했으며 `AGENT_GIBBON_LOCAL_TRACE=1`, `AGENT_GIBBON_TELEMETRY=off`다. diagnostics는 `terms_accepted:false`, `terms_skipped:true`, `sharing_allowed:false`, `onboarding_required:false`였다. 동의 파일·체크박스·공식 DSN을 만들거나 바꾸지 않았다.

native GUI 출처는 PID 39139의 window **42471**, 제목 `Agent Gibbon [DEV egoing/issue-64-trace-continuity · ab314a33] — issue64-scenario`와 CLI 본문을 대조했다. Orca의 list-windows는 빈 목록을 반환했지만 get-app-state의 AX와 캡처가 같은 PID·window·제목을 반환했다. `final-gui.json`의 `result.screenshot.path`에 캡처 경로가 있으며 실제 이미지를 열어 Markdown 제목·체크리스트·변경 확인 Feedback 렌더링을 확인했다. 이전 PID 18502의 window 42400에서는 편집 진입·검증용 원문 AXSetValue/readback까지만 확인했다. Feedback 42404의 선택 실패 때문에 저장 버튼 완료를 통과로 기록하지 않는다. 기존 사용자 창을 foreground로 가져오거나 포커스를 넘기지 않았다.

이전 PID 18502만 binary 절대 경로를 재대조해 종료하고 최신 빌드를 실행했다. 설치 앱·다른 워크스페이스를 종료하지 않았다. 검증 메모는 마지막에 CLI close/inspect로 `visible:false`를 확인했다. 진행 메모는 같은 ID로 갱신하며 자동 숨김/삭제하지 않는다.

## 검사 명령과 결과

- `cargo test --manifest-path src-tauri/Cargo.toml`: 최종 **163개 통과**(마지막 추가는 recorder 단계 폭주/만료 검사이며 native 동작 변경 없음).
- `npm run test:ui`: **100개 통과**.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`, `npm run check`(Clippy all-targets, warnings deny), `npm run dev:build`, `npm run version:check`(0.2.7), `git diff --check`: 통과.
- 로컬/모의 검사는 원격 수신과 다르다. Sentry SDK의 in-memory transport에서 snapshot root와 자식 10종의 실제 protocol span `trace_id`·`parent_span_id`를 검사하고 최종 privacy envelope 필터 뒤에도 관계가 남는 것을 확인했다. 모의 검사에서만 capture client를 쓰며 앱의 동의 상태나 전송 gate를 우회하지 않는다.
- 기존 테스트는 동의 전 SDK/전송 차단, 철회 시 진행 요청 종료 대기·새 요청 차단, 이전 세대 큐 재생 금지, 프런트 상세 모드 해제·재활성화 시 오래된 callback/집계 제거를 포함한다. 새 검사는 로컬 opt-in off의 flow/인자 부재, 동시 flow ID 분리, 숨김·2초 제한, 임의 operation 배제, 원문/오류 미직렬화, rAF가 앱 결과를 지연하지 않는 것, fit 관찰 뒤 frame 종료를 포함한다.
- `parent/cause/local_flow/local_mutation`은 Sentry envelope에 남지 않고 새 `webview.aggregate` 고정 표식만 허용된다. profile·attachment·session 등 envelope 항목의 허용 범위를 넓히지 않았다.

## 대표 native 실행

증거 폴더는 로컬 `/tmp/gibbon-issue64-20261106/`이며 저장소에 원본 로그·화면·프로파일을 추가하지 않는다. 앱 local recorder는 앱 데이터 `diagnostics/dev-ab314a33d1bc08f8900168c2/local-stages.json`이다. 이 파일은 재시작 때 덮어쓰므로 실행 중 수집본을 별도 보관했다.

실행 명령은 모두 해당 root의 `./scripts/agent-gibbon-dev`를 썼다.

1. `list`로 기존 메모를 조회했다. 초기 앱 미실행 오류를 확인하고 개발 빌드 뒤 고유 진행 메모를 open했다.
2. `open --id issue64-scenario --title ISSUE64-LOCAL-ONLY --markdown ...`, `inspect --id issue64-scenario`의 출처·본문·visible·focused를 대조했다.
3. 제목과 표식 한 개 + 40행 체크리스트를 1.1초 간격으로 **8회 update**했다. CLI 응답의 ok·instance를 매번 확인했다. 두 실행 모두 2개 메모 창, 같은 입력 규모를 사용했다. 처음 open/자동 배치와 이미 보이는 update는 별개 동작이다.
4. collector는 recorder 파일을 100ms 간격으로 읽어 호출/단계/단조 시각으로 중복 제거했다. 최신 실행에서 **7100개 stage**, `lost` 최댓값 **0**, 마지막 관측 active **0**을 확인했다. 파일 snapshot이므로 모든 500ms 미만 호출의 개별 발생을 보존한다고 보장하지 않는다.
5. 샘플 flow **2844 → snapshot.request 2845 → snapshot 2846**, snapshot의 `cause:2801`이 실제 갱신 작업을 가리켰다. `note.state.*`는 2845의 자식이다. 이 실행에서 notes_wait/copy, opacity/feedback, position/size, visibility/focus/pin, response를 구분했다. `final-flow-summary.json`에 호출 관계를 보관했다.
6. 같은 flow에서 `dom duration_us:0`, `measure:2000`, `frame_ready:31000`; 다른 flow 4444에서 measure 3000, frame_ready 24999를 관측했다. 0은 WebView 시계 해상도/반올림 범위이며 CPU 비용이 없다는 뜻이 아니다. `elapsed_us`는 Rust 수신 시각이고 `duration_us`는 WebView가 잰 동기 경과 시간이다. 단일 샘플을 p50/p95·GPU 완료로 해석하지 않는다.

최종 GUI 근거는 `final-gui.json`, CLI 근거는 `final-scenario-inspect.json`, `scenario-closed-inspect.json`, `final-diagnostics.json`, 단계는 `scenario-stages.json`이다. 조기 구현의 `initial-local-stages.json`, `before-buffer-stages.json`은 이전 PID 근거이며 최신 native 변경의 근거로 대체하지 않는다.

## 실제 로컬 프로파일과 해석

`sample 18502 5 1 -file .../native.sample.txt`로 살아 있는 정확한 native PID의 5초 스레드 호출 트리를 얻었다. 같은 시기 CLI update workload를 수행했다. sample은 대기 스레드도 보이므로 샘플 수 전체를 CPU 시간으로 해석하지 않는다.

`xcrun xctrace record --template 'Time Profiler' --attach PID --time-limit 20s --output .../scenario.trace`와 최신 PID 39139의 **15초** recording을 수행하고, `xctrace export`의 `time-profile` table을 파싱했다. 첫 초기 12초 녹화는 도구 실행 확인용으로 별도다. 타깃 PID·호출 트리·Rust 심볼이 실제로 포함된 것을 확인했다.

- 이전 20초 표: 3076개 row, `local_trace::persist` inclusive sample 957개, leaf `write` 874개. JSON 직렬화의 작은 토큰마다 파일 write를 하는 recorder 경로가 두드러졌다.
- 관측 뒤 `BufWriter`로 동일한 원자 교체 파일 저장을 버퍼링했다. 최종 15초 표: 1840개 row, `local_trace::persist` inclusive sample 60개. `final-time-profile.xml`과 요약 JSON을 로컬에 보관했다.
- 녹화 길이·GUI 편집 포함 여부·snapshot 계측 수가 다르므로 **957→60을 개선율로 계산하지 않는다**. 이 결과는 관측한 저장 경로를 바꾼 뒤 호출 트리를 다시 확인한 증거다. 현재 사용자가 체감하는 지연의 원인 확정, Markdown의 최적화 필요성, 최적화 빌드의 성능 수치, recorder 전체 부하 p50/p95를 입증하지 않는다.
- main thread의 `sync_opacity`, `feedback_window::layout_feedback`, snapshot 조회도 실제 심볼로 관측했다. inclusive sample은 중첩되므로 합산해서 100% 비율을 만들지 않는다. UI 이벤트/잠금/IPC 대기는 해당 trace stage의 wall time과 대조하고 CPU 샘플만으로 대기 시간을 추정하지 않는다.

Instruments 원본/TOC에는 프로세스 환경변수·장치 정보가 포함될 수 있다. 전체 TOC·원본을 출력/업로드하지 않는다. 이번 원본 `.trace`와 register export는 삭제했고 환경 영역을 제거한 목차, 환경·레지스터가 없는 `time-profile` 표와 요약만 남겼다. 후속 녹화는 최소 환경으로 시작했다. 증거 폴더 권한은 0700이다. 이 기록이 원본 프로파일 외부 공유 승인은 아니다.

## 재현 가능한 분석 절차

### Rust/macOS

1. 다른 인스턴스를 유지하고 `npm run dev:info`, binary/PID, CLI inspect/GUI 제목을 먼저 대조한다. 로컬 trace를 원하면 새 실행 전에 `AGENT_GIBBON_LOCAL_TRACE=1 AGENT_GIBBON_TELEMETRY=off`를 설정한다. 이미 실행 중인 앱에는 소급되지 않는다.
2. 이번 결과는 debug 실행이다. 최적화 비교는 별도로 `CARGO_PROFILE_DEV_OPT_LEVEL=2 CARGO_PROFILE_DEV_DEBUG=2 npm run dev:build`로 심볼을 보존한 격리 debug 실행기를 사용한다. 현재 실행을 정확히 종료한 뒤 재빌드하고 `scripts/agent-gibbon-dev`로만 실행한다. 이 최적화 빌드는 이번에 수행하지 않았다. 비교 양쪽에 같은 build mode·compiler·화면·창 수·본문/행 수·반복 간격을 기록한다.
3. Instruments Time Profiler에서 해당 PID에 attach하고 recording을 시작한 뒤 위 8회 갱신/열기·숨김/명시적 저장/터미널 shell 시작을 각각 구분해서 수행한다. 자동 모델 실행·기존 터미널 작업을 재현용으로 사용하지 않는다. 원격 Sentry를 켜려면 별도 실제 전송 동의가 필요하다.
4. CLI는 `xcrun xctrace record --template 'Time Profiler' --attach PID --time-limit 15s --output /로컬/경로/run.trace`다. 녹화 완료와 저장 완료를 기다린 뒤 export한다. 설치 도구는 `--help` 대신 인자 없는 하위 명령에서 사용법을 출력했으므로 현재 CLI 계약을 확인한다.
5. main/worker/reader/recorder 스레드를 나누고 함수 inclusive/self sample을 본다. `note.state.notes_wait` wall time이 큰데 CPU 샘플이 적으면 잠금/큐 대기를 의심하는 식으로 해석하며 단독 증거로 교착을 확정하지 않는다. sample/Time Profiler는 WebView JS 함수나 GPU 페인트 완료 증거가 아니다.

### WKWebView/macOS

debug Tauri 2.12.2는 devtools를 제공하며 설치된 Rust 소스의 `open_devtools` 문서에서 debug 또는 devtools feature 조건을 확인했다. Safari의 개발자 도구를 허용하고, 출처가 확인된 검증 메모의 WebView만 Inspect한다. 앱 이름만으로 대상 WKWebView를 선택하지 말고 제목/고유 표식과 inspect 응답을 대조한다.

Timelines에서 Screenshots/Memory/네트워크 수집 등 필요 없는 항목은 끄고 **JavaScript & Events**, **Layout & Rendering**만 선택한다. recording을 시작하고 같은 40행/8회 갱신을 실행한 뒤 멈춘다. refresh/fitContent의 JS 호출 트리와 DOM/layout/style/paint 구간을 선택해 local stage의 같은 재현 시간 범위와 대조한다. `frame_ready` 표식 자체는 paint를 입증하지 않는다. 타깃 WebContent PID와 recording 시간 범위, inspector 활성화 여부, export 경로를 함께 기록한다.

이번에는 native GUI 표시까지 확인했지만 Safari Inspector에 연결해 JS/렌더링 recording을 만들지는 않았다. 따라서 WKWebView 함수별 비용·레이아웃 병목은 미검증이다. [WebKit Inspector 활성화](https://webkit.org/web-inspector/enabling-web-inspector/), [Timelines의 호출 트리/렌더링/내보내기 정의](https://webkit.org/web-inspector/timelines-tab/)를 따른다.

### Windows Browser Profiling

Windows 런타임이 없어 **미검증**이다. [Sentry Browser Profiling 공식 문서](https://docs.sentry.io/platforms/javascript/profiling/)를 이번에 다시 읽었으며 beta, Chromium JS Self-Profiling API, `@sentry/browser` 10.27.0 이상, 응답 `Document-Policy: js-profiling` 조건을 확인했다. Chromium 엔진 이름만으로 WebView2/Tauri 지원을 확정하지 않는다. WKWebView에는 이 Chromium 전용 경로를 적용하지 않는다.

후속 Windows 검증은 정확한 WebView2 버전·보안 context·`Profiler` API 존재·실제 document policy와 profile 시작/중지·최종 서버 수신을 작은 테스트로 나눠 확인한다. API 없는 경우 WebView2 DevTools Performance/WPR 등 로컬 대안을 검토한다. 현재 브라우저 SDK/profile envelope를 설치·허용하지 않았으며 기존 Rust envelope 필터는 profile을 차단한다. 도입하려면 동의·샘플링·URL/경로/스크립트/원문 경계를 별도 설계해야 한다.

## 연결의 한계와 후속 검증

WebView 시작 전 Rust에 도달하는 첫 대기, Tauri의 반환값 최종 직렬화/전달, 모든 UI callback/타이머·배치 애니메이션 완료, 터미널 첫 출력의 WebView 파싱까지 모두 하나의 원격 trace로 잇지는 않는다. 명시적으로 전달한 실행 내 로컬 관계와 동기 Rust span을 구분한다. 다음 rAF가 안 오면 60초 만료하고, lost/버퍼 회전/flush 지연/강제 종료에서는 미완료 목록이 완전하지 않다.

다중 flow/스레드의 잘못된 부모 연결은 자동 검사로 확인했으나 native 다중 창 동시 터미널 시작은 미검증이다. snapshot 자식 span의 원격 실제 수신, Windows/Intel native, WK 타임라인과 최적화 프로파일, 전체 플랫폼 Actions는 후속 항목으로 유지한다. [디자인 시스템](../design-system.md)의 외형·위치·숨김·포커스 기준은 변경하지 않았다.
