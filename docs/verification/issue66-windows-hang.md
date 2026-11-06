# Windows 반복 행 조사 — #66, 2026-11-06

## 현재 판단 — 10:47 KST 후속

**이번 캡처 드래그 후 정지는 Notes 잠금과 UI 메뉴 갱신 사이의 교착으로 확인했다.** 정지한 동일 PID의 UI/작업자 호출 스택과 해당 빌드 소스를 대조했다. 트레이 갱신에서 잠금을 해제하도록 수정한 개발 빌드로 computer use의 실제 포인터 드래그 5회를 완료했다. 아래 초기 조사 기록은 당시의 한계이며, 최신 근거는 문서 마지막의 **실제 캡처 드래그 정지와 수정 검증** 절을 따른다. 모든 과거 장시간 행의 원인이 동일하다고 일반화하지 않는다.

## 초기 판단 — 후속 스택 수집 전

**원인 미확정, 해결 완료 아님.** 이슈에 기록된 장시간 실행 중 정지를 이번 세션에서 재현하지 못했다. 멈춘 프로세스가 없어 대기/호출 스택도 수집하지 못했다. 기존 [영역 선택 후 교착 기록](windows-freeze-20261106.md)은 별개 실행의 근거이며 #66의 정지 원인으로 재사용하지 않는다. Sentry 이벤트는 이번 작업에서 다시 조회하지 않았으며 이슈 본문의 관측과 구분한다.

## 출처

- root: `C:\Users\egoin\orca\workspaces\agent-gibbon\windows-native-ui`
- instance: `dev-68a1c4f96c3312982312ca04`
- binary: 위 root의 `src-tauri\target\debug\agent-gibbon.exe`
- 기반 HEAD: `6cc7f91c65d77611a677e0860a6627935e562ccc` + 이번 미커밋 변경. 빌드 표식만으로 소스 일치를 단정하지 않는다.
- Node `v20.11.0`, npm `10.2.4`, rustc/cargo `1.99.1`, Rust host `x86_64-pc-windows-msvc`. Tauri `2.12.2`, runtime-wry `2.12.1`, Wry `0.57.0`을 실제 빌드 로그/설치 소스로 확인했다.
- 시작 전 `Get-CimInstance Win32_Process`·`Get-Process`에서 실행 중인 `agent-gibbon.exe`가 없었다. 다른 인스턴스를 종료하지 않았다.

## 코드 점검과 변경

`snapshot`은 `AppNotes` guard를 보유한 채 `inspect_window`를 호출했다. 이 내부에는 `sync_opacity` → 포인터/창 조회, Feedback·Stash·Sizing 등 상태 조회, 여러 네이티브 getter가 있다. 설치된 runtime-wry `src/lib.rs`의 `getter!`는 메시지 전송 후 `recv()`로 대기한다. 따라서 상태 잠금 안의 네이티브 조회를 조사 대상으로 삼을 근거는 있지만 실제 #66 스택이 없으므로 이것만으로 교착 원인을 확정할 수 없다. [Tauri WebviewWindow API](https://docs.rs/tauri/2.12.2/tauri/webview/struct.WebviewWindow.html)와 로컬 런타임 구현을 함께 확인했다.

이번 변경은 `snapshot`의 Note를 잠금 안에서 복사하고 native 조회 전에 guard를 해제한다. 명령의 동기/비동기 실행 정책, 사용자 표시·위치·숨김 규칙은 바꾸지 않는다. 이는 잠금 범위 축소이며 원인 확인 후의 검증된 수정으로 주장하지 않는다. `save_note`의 Notes → Feedback, list/배치/터미널/캡처의 다른 상태 잠금과 네이티브 API 경로까지 전체 잠금 순서를 증명한 것은 아니다. 특히 `save_note`는 Notes를 잡고 저장·Feedback을 갱신하며 이번 변경 범위 밖이다.

종료되지 않은 작업도 다음 조사에서 구분할 수 있게 별도 opt-in 로컬 recorder를 추가했다. 메모 snapshot 단계와 CLI의 UI 실행 큐 대기를 구분하고, 미종료 호출은 이력이 순환해도 별도로 보관한다. 고정 스키마·수량 제한·비차단 기록·원자적 worker 저장과 한계는 [진단 안내](../diagnostics.md#windows-행-조사용-로컬-단계-기록--66)를 따른다. 전송 동의나 Sentry 경로는 변경하지 않았다.

## 이번 실행

- `npm run dev:info`로 위 출처를 기록하고 `npm run dev:build` 후 해당 root의 `.\scripts\agent-gibbon-dev.cmd`만 사용했다.
- 실행 환경은 `AGENT_GIBBON_LOCAL_TRACE=1`, `AGENT_GIBBON_TELEMETRY=off`. `open --id issue66-20261106 --markdown ISSUE66-LOCAL-TRACE-20261106`은 `ok:false`, `onboarding_required:true`, 예상 instance를 반환했다. 메모 생성 성공이나 창 출처 확인으로 간주하지 않는다.
- 이번에 직접 시작한 PID `26516`, 정확한 binary 경로를 CIM으로 확인했다. `diagnostics`는 `ok:true`, 예상 instance, `enabled:false`, `detailed:false`, `terms_accepted:false`, `sharing_allowed:false`를 반환했다. 이는 진단 CLI 응답 근거이며 메모 `inspect` 출처 검증을 대신하지 않는다.
- 2026-11-06 10:15:40 KST의 읽기 전용 점검: Responding true, 1087.7ms 동안 CPU 증가 0ms, 열거한 해당 PID의 창 8개(표시 2개) 모두 WM_NULL 응답 성공. 네이티브 TID `25860`. 로컬 증거: `%TEMP%\gibbon-hang-20261106-101538\process.json`. 창 제목/본문·화면을 수집하지 않아 UI 기능의 시각 검증으로 보고하지 않는다.
- `%APPDATA%\org.egoing.floatnote\diagnostics\dev-68a1c4f96c3312982312ca04\local-stages.json`에 PID `26516`, 빌드 `6cc7f91c65d7`, IPC 2건의 `ui_queue` → `ui_execute` → `complete`, `lost:0`, `active:{}`를 확인했다. 초기 2건의 큐 대기는 약 1.129ms / 0.073ms였으며 일반 부하 성능 분포를 뜻하지 않는다.
- 약관을 자동 수락하거나 consent 파일을 조작하지 않았다. 첫 실행 약관 수락 여부를 사용자에게 요청했다. 메모 `inspect`, 1/5/10개 창, 숨김 갱신, 터미널/캡처/배치 반복, 실제 포커스·화면 반응은 수락 전 미검증이다.

## 검사와 남은 범위

- 최종 Rust 전체 **112개 통과**(recorder 검사 4개 포함), UI **79개 통과·1개 POSIX 전용 제외**. Rustfmt·Clippy `--all-targets -- -D warnings`·`git diff --check` 및 Windows 개발 빌드 통과. 미종료 작업 저장 검사는 앱 시작 후 추가한 `cfg(test)` 코드이며 PID 26516의 실행 결과를 해당 테스트 자체의 실행 근거로 사용하지 않는다. 실행 이후 런타임 동작 코드는 변경하지 않았다.
- recorder 검사는 완료 없이 차단된 호출을 먼저 저장하는 경로, 최근 이력 순환 이후 active 보존, 용량 제한, recorder mutex 경합 시 대기하지 않는 경로, Windows 원자 파일 교체와 고정 필드를 다룬다. 인위적으로 앱 UI를 교착시킨 실제 회귀 검증은 아니다.
- 재현 전후 동일 Windows 시나리오 비교, 실제 정지 스택·최소 재현, WebView2 자식 프로세스·GPU 상태, 장시간 실행, OS 잠금/해제·절전 복귀, 여러 화면/혼합 DPI, macOS Intel/ARM 실행·빌드, 설치 파일 빌드·설치는 수행하지 않았다. 이슈 완료 조건은 남아 있다.
- 다음 실제 정지에서는 재시작 전 위 점검 파일과 local-stages 사본의 PID·시각을 맞추고 WinDbg/WPR 등의 호출 스택을 대조한다. `active` 또는 WM_NULL 실패만으로 특정 잠금을 원인으로 확정하지 않는다. 덤프·원문 자료는 임의 업로드하지 않는다.

[디자인 시스템](../design-system.md)의 기존 사용자 작업 보존 기준을 유지하며 외형·시안은 변경하지 않았다.

## Computer use 재현 조사 — 2026-11-06 10:20–10:22 KST

사용자 요청으로 `orca-computer-use`를 사용했다. `orca skills get computer-use --json`의 현재 버전 안내와 `orca computer capabilities --json`을 확인했다. binary·PID·instance는 위와 동일하며 `diagnostics`에서 여전히 `terms_accepted:false`, `sharing_allowed:false`였다. 앱을 재시작하거나 동의를 대신 수락하지 않았다.

- `orca computer list-windows --app pid:26516 --json`에서 HWND `2492548`, 네이티브 제목 `Agent Gibbon [DEV egoing/windows-native-ui · 68a1c4f9] — 메모 관리`를 특정했다. 이후 모든 GUI 호출에 같은 PID와 `--window-id 2492548`을 지정했다. 위치 `(192,192)`, 크기 `1198×647`의 동일 창을 확인했다. 메모 inspect 성공 근거는 아직 없으므로 이는 관리 창만의 출처다.
- `get-app-state`에서 첫 실행 화면을 확인했다. `Read Terms` 클릭 뒤 문서 제목·본문·`Close document`가 나타났으며 실제 캡처도 열어 문서 표시를 확인했다. 닫은 뒤 원래 동의 화면이 다시 나타났다. 도구의 action verification은 `unverified/accessibility_action_unasserted`였으나, 반환된 트리와 문서 캡처로 상태 전환을 별도 확인했다.
- 약관·개인정보 문서를 번갈아 **6회** 열고 닫았다(각 3회). 매 동작 직전에 새 트리에서 버튼 인덱스를 찾고, 열기 후 `Close document` 생성과 닫기 후 제거를 확인했다. 각 열기/닫기 두 computer-use 호출의 합산 소요는 `2682, 2769, 2665, 2775, 2667, 2782ms`였다. 이 수치는 도구 호출·트리 수집 비용을 포함하므로 앱 자체 지연으로 해석하지 않는다.
- 반복 직후 `diagnostics`는 `ok:true`, 예상 instance로 응답했다. `%TEMP%\gibbon-hang-20261106-102155\process.json`에 네이티브 점검을 남겼다. 이 조사에서 hang을 관측하지 않았다. 상세 watchdog는 전송 정책에 따라 꺼져 있어 watchdog 이벤트 부재를 통과 근거로 사용하지 않는다.
- 캡처: `%TEMP%\orca-computer-use\fc9fb679-ca65-4b6b-90c1-06da188f1ba9-screenshot.png`(실제 약관 문서), `%TEMP%\orca-computer-use\756806d4-2acb-4942-a5e3-039ef47e5e0b-screenshot.png`(반복 이후 첫 실행 화면). 둘 다 위 PID·HWND를 지정해 얻었다. 저장소에 화면 원본을 추가하거나 외부에 업로드하지 않았다.
- **미검증:** 사용자 약관 수락을 요청했으며 아직 응답 대기 상태다. 메모·캡처·영역 터미널 생성은 첫 실행 동의 전 차단되므로 이번 조사에서 실행하지 못했다. 이번 결과는 동의 전 문서 UI에 한정되며 #66의 최소 재현 실패나 전체 해결을 의미하지 않는다. Windows 작업 특성상 포커스가 이동할 수 있음을 사전에 안내했으며 창 이동·크기 조절·다른 인스턴스 종료는 하지 않았다.

## 실제 캡처 드래그 정지와 수정 검증 — 10:39–10:47 KST

사용자가 직접 캡처를 드래그하면 바로 멈춘다고 추가 제보했다. **사용자가 재현한 입력**과 **이번 에이전트가 computer use로 수행한 수정 후 입력**을 구분한다. 단순히 재시작해서 회복한 것을 수정 검증으로 취급하지 않는다.

### 수정 전: 살아 있는 프로세스에서 얻은 증거

- 동일 root·instance·binary의 PID `26516`이 실제 멈춰 있었다. 약관 대기 때의 과거 결과를 재사용하지 않고 다시 조회했다. `Get-Process`의 Responding false, 개발 CLI `diagnostics`의 `ok:false`/응답 시간 초과, 해당 PID의 HWND 10개 모두 WM_NULL 응답 실패를 확인했다. `%TEMP%\gibbon-hang-20261106-104003\process.json`에 남겼다.
- computer use `list-windows --app pid:26516`은 캡처 메모 `image-1793929144214790500`, HWND `14813050`, 위치 `(665,352)`, 크기 `426×349`를 식별했다. 기존 UI 스레드 TID는 `25860`이었다.
- `local-stages.json`에는 `lost:0`, 호출 38의 `snapshot/notes_wait`(실행 시작 후 1406098172µs)와 호출 39의 `ipc/ui_queue`가 남아 있었다. 이전 완료 snapshot은 호출 37이었다. 이로써 현재 동의 후의 메모 요청이 실행됐으나 UI 큐가 진행하지 못하는 상황을 구분했다.
- 재시작 전에 `GetThreadContext`/`StackWalk64`와 해당 binary의 로컬 PDB로 UI·작업자 스택을 수집했다. 각 스레드는 조회하는 동안만 정지하고 `finally`에서 재개했다. 함수 인자·변수·메모리 덤프·화면은 수집하거나 업로드하지 않았다. 기존 `%TEMP%\gibbon-live-stacks-main.py`, `gibbon-live-stacks.py`를 읽고 로컬 심벌만 쓰는 경로임을 확인해 사용했다.

```text
UI TID 25860:
  WebView2 IPC callback → agent_gibbon::snapshot
    → Mutex<BTreeMap<String, Note>>::lock → WaitOnAddress

worker TID 33420 (tokio-rt-worker):
  capture::finish_capture → capture::finish_selection → capture::create_note
    → show_without_focus → tray::sync_visibility
      → IconMenuItem::set_icon → UI 실행 완료 채널 recv
```

기존 `sync_visibility`는 `TrayMenu` 잠금을 잡고, 각 항목에서 `AppNotes` 잠금을 잡은 상태로 `set_icon`을 호출했다. 설치된 Tauri `2.12.2`의 `src/menu/icon.rs`와 `run_item_main_thread!`에서 이 호출이 UI 실행을 동기 대기하는 구현임을 확인했다. 따라서 작업자는 Notes를 보유하고 UI를 기다리며, UI는 snapshot을 처리하려고 Notes를 기다린다. 이 두 스택과 lexical guard 범위로 **이번 실행의 순환 대기**를 설명할 수 있다. 이전 WebContextStore 재진입 교착과는 잠금·호출 위치가 다르다.

재현 조건은 캡처 단축키 → 영역 드래그/해제 → 이미지 메모 생성 및 표시 → 트레이 아이콘 갱신과 새 메모의 snapshot 요청이 겹치는 경로다. 경합 순서에 영향을 받으므로 모든 환경·입력마다 반드시 멈춘다고 주장하지 않는다.

### 국소 수정과 자동 검사

- `tray.rs`에서 소유한 메뉴 메타데이터(ID·최대 30자 제목·캡처 여부)만 Notes 잠금 안에서 복사한다. 본문·HTML·이미지 데이터는 복사하지 않는다. 이후 메뉴 생성·아이콘 갱신·가시성 조회에서는 Notes guard를 보유하지 않는다.
- `sync_visibility`는 메뉴 handle을 복제하고 TrayMenu guard도 네이티브 호출 전에 해제한다. `refresh`에서 교체된 이전 메뉴의 destructor도 TrayMenu 잠금 밖에서 실행한다. 관측된 Notes 교착과 같은 방식의 다른 잠금 재진입을 방지한다.
- 작업자가 메뉴 API 완료를 기다릴 때 UI의 snapshot이 Notes를 획득할 수 있는 회귀 검사와 메뉴 메타데이터의 소유권/제목 제한 검사를 추가했다. Rust **114개**, Rustfmt, Clippy `--all-targets -- -D warnings`, Windows 개발 빌드 통과. 이 작업에서 UI 소스는 바꾸지 않아 앞선 UI **79개 통과/1개 제외** 결과와 구분한다.
- 증거 확보 후 정확한 경로를 재대조해 이번 검증에서 직접 시작한 PID 26516만 종료했다. 새 빌드는 같은 root의 개발 CLI로 시작했다. 다른 인스턴스·설치 앱은 종료하지 않았다. 실행 중 바이너리를 덮어쓰지 않았다.

### 수정 후: 실제 computer-use 드래그

새 PID `12644`, 동일 binary·instance를 CIM 및 CLI로 확인했다. `diagnostics`의 `terms_accepted:true`를 관측했으며 에이전트가 동의 파일/체크박스를 조작하지 않았다. 외부 진단 전송은 계속 꺼져 있었다.

검증 메모 `issue66-gui-20261106`의 `inspect`에서 `ok:true`, 예상 instance, 본문 `ISSUE66-GUI-20261106-CAPTURE-REGRESSION`을 확인했다. computer use에서 같은 PID의 HWND `1051054`와 해당 ID가 포함된 네이티브 제목·본문도 대조했다. 새로 선택하는 Overlay/이미지 창마다 PID와 HWND·제목을 다시 확인했다.

`orca computer hotkey ... --key Ctrl+Shift+1 --restore-window`로 Overlay를 생성하고, `get-app-state`의 영역 선택 안내를 확인한 뒤 `orca computer drag`로 실제 pointer down/move/up을 보냈다. WebView에 직접 `finish_capture`를 호출하는 방식으로 대체하지 않았다. 처음에는 포커스 부족 오류가 나 foreground 전환을 안내한 뒤 한 번 재시도했다.

| 회차 | 새 이미지 메모 ID | Overlay HWND | 생성 뒤 표시 창 수 |
| --- | --- | --- | --- |
| 1 | `image-1793929409848633400` | 1376930 | 2 |
| 2 | `image-1793929460261545400` | 3016818 | 3 |
| 3 | `image-1793929470321802400` | 1441916 | 4 |
| 4 | `image-1793929480600071400` | 6357226 | 5 |
| 5 | `image-1793929490678643700` | 2361516 | 6 |

- 1회차는 Overlay 내 `(984,488)→(1340,565)`, 나머지는 `(1555,30)→(1885,100)` 드래그였다. Overlay 보고 scale은 1이었다. 새 메모마다 해당 root의 CLI로 `ok:true`, instance 일치, capture 메타데이터, visible true를 확인했다. 겹쳐 배치된 창은 모두 가시 상태였지만 모든 픽셀이 동시에 노출된 것은 아니다.
- 2–5회차의 drag 도구 호출+list+inspect 합산 소요는 `3411,3468,3188,3568ms`였다. 도구 실행 비용을 포함하므로 앱 자체 지연이나 p95로 보고하지 않는다. 실제 드래그 5회는 약 10:43:29–10:44:50 KST에 수행했다. 수정 전의 첫 사용자 드래그 정지에 비해 수정 후 같은 이미지 생성 경로 5회에서는 hang이 관측되지 않았다.
- 마지막 이미지의 `상단 고정` 버튼을 computer use로 누른 뒤 inspect의 `always_on_top:false`로 상태 변경을 확인했다. 겹친 앞 창 때문에 이후 해당 HWND가 도구에서 찾히지 않는 경우는 CLI가 계속 응답하고 다른 창을 찾았으므로 hang으로 분류하지 않았다. 창을 다시 선택한 뒤 4회차 이미지의 `메모 창 닫기`를 누르고 inspect에서 `closed:true`, `visible:false`를 확인했다.
- 최종 캡처 이미지의 실제 화면을 열어 렌더링과 Toolbar 표시를 확인했다. 캡처 경로: `%TEMP%\orca-computer-use\390b2787-98e8-4799-8770-a3c7c2337d53-screenshot.png`, PID 12644/이미지 5/ HWND `2427052`와 연결된다. 검사 중 화면 원본을 저장소에 추가하거나 외부 업로드하지 않았다.
- 테스트로 만든 나머지 메모는 CLI close로 숨겼다. 사용자 원래 캡처 `image-1793929144214790500`은 다시 표시하고, 자동 배치를 중단한 뒤 DPI 인식 Win32 위치 복원으로 이전 물리 `(665,352),426×349`를 맞췄다. `inspect`의 위치·크기·visible true·animating false로 확인했다. 캡처 원본 PNG나 본문은 바꾸지 않았다. 기록하지 못한 다른 창의 원래 기하학·미저장 초안까지 복구했다고 주장하지 않는다.
- 10:47:13 최종 네이티브 점검: PID 12644, Responding true, HWND 13개 모두 WM_NULL 응답 성공. 1054.0ms 동안 CPU 증가 140.625ms. local trace `lost:0`, `active` 0개, history 256개. 전송 정책으로 상세 watchdog는 계속 꺼져 있어 watchdog 이벤트 부재는 해결 근거에 포함하지 않는다.

### 증거 보관과 한계

로컬 `%TEMP%\gibbon-issue66-live-26516\`에 `main-stack.jsonl`, `worker-stacks.jsonl`, `local-stages-before-restart.json`, `local-stages-after-fix.json`, `binary-before.json`, `fixed-source-binary-hashes.json`을 보관했다. 위 root의 실제 main/tray/capture/local_trace 소스와 수정 binary의 SHA-256을 기록했으며, 동일한 기반 revision만으로 소스 일치를 주장하지 않는다. 네이티브 상태는 `%TEMP%\gibbon-hang-20261106-104003\process.json`(전), `...-104331`, `...-104515`, `...-104613`, `...-104713`(후)의 `process.json`에 남았다.

이번 **이미지 캡처 생성 경로의 교착은 확인·수정·짧은 실제 반복 검증을 완료**했다. 수정 전 자동화 반복으로 재현 빈도를 측정하지 않았고, 수정 후 사용자의 물리 입력 재확인, 수시간 실행, 잠금/해제·절전, 텍스트/터미널 선택 반복, 다중 모니터·다른 DPI, macOS ARM/Intel 회귀 및 설치 파일 빌드는 미검증이다. #66에 열거된 과거 모든 정지의 해결이나 장시간 안정성을 보장하지 않는다.

## main 통합 검사와 작업 종료

- 수정 커밋 `f0a0874`를 작성하고 원격 main `2f7ee58`의 개인정보 동의·관리 창 변경을 `65326d6`에서 충돌 없이 통합했다. 이 통합은 앞선 PID 12644의 실행 뒤에 수행했으므로 과거 드래그 화면을 통합본의 새로운 GUI 실행 근거로 사용하지 않는다.
- 같은 작업 root에서 통합본 Rust **115개**, UI **80개 통과/1개 POSIX 전용 제외**, Rustfmt·Clippy·버전 일관성 검사(`0.2.7`)·Windows 개발 빌드와 diff 검사를 통과했다. 이 단계에서는 앱을 다시 실행하거나 새 개인정보 정책에 대신 동의하지 않았으며, 통합본 native GUI 반복·다른 플랫폼·설치 빌드는 미검증이다.
- 사용자의 커밋·병합·푸시·워크스페이스 종료 요청에 따라, 직접 시작했던 PID 12644의 binary 절대 경로를 재대조해 종료했다. 원래 캡처의 저장 파일과 로컬 조사 증거는 워크스페이스 밖의 앱 데이터·TEMP에 유지한다. 종료를 기능상 `close`와 혼동하지 않는다.
