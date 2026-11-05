# Windows 터미널 영역 선택 후 프리징 — 2026-11-06

## 재발 증거

사용자가 Sentry 진단 빌드에서도 프리징을 보고해 재시작 전에 조사했다. 대상 root는 `C:\Users\egoin\dev\project\agent-gibbon`, instance는 `dev-fbc7d562e05aec4798036a95`, 실행 파일은 해당 root의 `src-tauri\target\debug\agent-gibbon.exe`, PID는 `6436`, 구현 revision은 `9f7edf6b6c2e`였다.

- `Get-Process`의 `Responding:false`, CLI `diagnostics`의 응답 시간 초과를 확인했다.
- 로컬 `diagnostics/<instance>/recent.json`에 `Native UI stalled`, `duration_ms:12003.0162`, 시각 `1793897037`이 남았다.
- Sentry API로 event `5054be99284d4e7da05d89ff18941307`의 수신을 확인했다. 시각은 **2026-11-06 01:43:57 KST**, release는 `agent-gibbon@0.2.7+9f7edf6b6c2e`, 플랫폼은 Windows/x86_64, window.kind는 native였다.
- 따라서 이번 재발은 숨김 상태나 화면 잠금만으로 설명하지 않는다. 앞선 배포 직후 정상 응답 확인은 이 후속 프리징이 없다는 보장이 아니었다.

## 멈춘 위치

Windows의 `GetThreadContext`·`StackWalk64`와 같은 빌드의 로컬 PDB로 살아 있는 프로세스의 함수 스택을 읽었다. 스레드는 읽는 동안만 일시 정지하고 finally에서 재개했다. 메모리 덤프·화면·함수 인자·지역 변수는 저장하거나 Sentry로 보내지 않았다.

메인 스레드 TID `24176`의 스택은 다음 호출 관계를 보였다. 실제 아래쪽 프레임부터 읽은 순서다.

```text
capture::finish_capture
  terminal::welcome::create
    terminal::open
      WebviewWindowBuilder::build
        tauri_runtime_wry::create_webview
          wry::webview2::InnerWebView::create_controller
            webview2_com::wait_with_pump
              DestroyWindow / handle_event_loop
                WebviewWrapper::drop
                  Mutex<WebContextStore>::lock
                    WaitOnAddress
```

현재 `tauri-runtime-wry 2.12.1` 소스에서 `create_webview`는 WebContextStore 잠금을 잡고 WebView2 controller를 만든다. `WebviewWrapper::drop`도 같은 잠금을 잡는다. 동기 WebView IPC 콜백 안에서 영역 선택 화면의 삭제를 요청한 직후 새 터미널을 만들면서, controller 초기화의 메시지 펌프가 삭제 이벤트를 처리해 같은 잠금으로 재진입한 교착을 확인했다. Sentry 전송 스레드는 수신 채널을 기다리고 있었고 watchdog 스레드는 별도로 살아 있었다. 이 관측으로 모든 과거 프리징의 원인까지 동일하다고 단정하지 않는다.

[Tauri의 WebviewWindowBuilder 문서](https://docs.rs/tauri/2.12.2/tauri/webview/struct.WebviewWindowBuilder.html)는 Windows의 동기 command/event handler에서 창을 만들면 교착할 수 있으며 async command 또는 별도 스레드를 쓰도록 안내한다. Win32 스택 조회는 [StackWalk64](https://learn.microsoft.com/en-us/windows/win32/api/dbghelp/nf-dbghelp-stackwalk64), [GetThreadContext](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getthreadcontext)의 계약을 따랐다.

## 수정과 확인 범위

- Windows에서만 `finish_capture`에 Tauri의 async command 실행 정책을 적용했다. WebView2의 IPC 콜백을 반환한 뒤 작업자에서 삭제·창 생성 요청을 보낸다. macOS의 AppKit 실행 정책은 유지한다.
- 초기 비동기 수정 후 사용자가 터미널 생성·버튼 반응을 한 번 확인했으나 재발을 보고했다. 로컬 journal에는 revision `ebfc93515476`의 두 번째 `Native UI stalled`가 시각 `1793897747`(01:55:47 KST)에 기록됐고, Windows Application 이벤트 1001의 `EventName:AppHangB1`, `P1:agent-gibbon.exe`를 확인했다. 수집 시점에는 해당 프로세스가 사라져 두 번째 중단의 스택은 얻지 못했다. 첫 수정만으로 해결됐다고 판정하지 않는다.
- 추가 수정에서는 Windows에서 모든 Capture Overlay의 `WindowEvent::Destroyed`를 수신한 뒤 새 WebView 생성을 요청한다. 단순 destroy 요청의 반환을 실제 종료로 간주하지 않는다. 대기는 UI 스레드 밖에서 수행하고, 전체 5초 제한 안에 확인되지 않으면 새 창 생성을 진행하지 않는다.
- 사용자 요청에 따라 Astra 독립 감사를 한 번 수행했다. 감사는 실제 스택 분석과 실행 검증을 대체하지 않는 코드 감사다. `Destroyed` listener 자체는 runtime destructor보다 앞서 호출되지만 worker의 후속 생성 메시지는 Tao 이벤트 핸들러가 파괴 처리를 마친 뒤 실행된다는 순서를 확인했다. 종료 완료 여부를 listener 시점 하나로만 해석하지 않는다.
- 감사에서 발견한 관리 창의 `manager_terminal(open)` 동기 WebView 생성 경로도 Windows async command로 분리했다. 이 경로는 동일 위험을 가진 소스 경로이며 두 번째 중단의 실제 원인으로 확정한 것은 아니다.
- Windows 캡처 시작·완료·명시적 취소·초기화 실패 정리는 blocking worker에서 같은 RAII mutex로 직렬화한다. UI callback은 이 mutex를 기다리지 않는다. 초기화 중 도착한 실패 정리도 버리지 않고 작업자로 넘기며, 시작 실패·시간 초과·오류 반환에서 guard가 해제된다. Overlay label에 세대 번호를 포함해 이전 선택의 늦은 취소가 새로운 선택을 지우지 않게 했다. macOS는 기존 main-thread 실행 정책을 유지한다.
- Windows window getter가 UI 스레드를 기다릴 수 있으므로 `scale_factor` 조회를 Captures 잠금 밖으로 옮겼다. 이 때문에 캡처 준비·취소가 UI 스레드에서 같은 상태 잠금을 기다리는 별도 교착도 만들지 않도록 했다.
- 사용자가 터미널만 선택한 사각형과 다른 곳에 생성되는 현상도 보고했다. Capture의 Windows 원점은 물리 desktop 좌표인 반면 기존 terminal builder.position은 논리 좌표로 해석했다. 새 터미널 메타데이터에 물리 좌표 여부를 보관하고, 숨긴 창을 올바른 좌표계로 배치한 뒤 대상 모니터 DPI로 크기를 설정한다. 이동·크기 변경 이후 저장도 같은 규칙을 적용한다. 기존 필드 없는 메타데이터는 논리 좌표로 복원해 호환성을 유지한다. macOS 원점은 기존 논리 좌표를 유지한다.
- 외형·크기·선택한 영역·사용자 포커스에 관한 디자인 기준은 변경하지 않았다. [디자인 시스템](../design-system.md)의 기존 기준을 적용한다.
- MacBook에서 Rustfmt, 최종 Rust 전체 116개, Clippy `--all-targets -- -D warnings`가 통과했다. 새 회귀 검사는 여러 창의 종료 확인을 모두 기다리는 순서, 종료 확인 누락·기한 초과 시 진행 차단, 음수 desktop 원점, 과거 논리 좌표와 새 물리 좌표 메타데이터 복원을 검증한다. Windows에서는 100%·125%·150%·200% scale 변환 시 물리 원점이 추가로 배율 적용되지 않는 것을 확인한다.
- Windows에서 최종 수정 소스의 진단 빌드, Rust 전체 101개, Clippy가 통과했다. 전달한 `capture.rs`·`terminal.rs`·`terminal/welcome.rs`의 SHA-256을 줄바꿈 정규화 후 MacBook과 대조해 모두 일치했다. 멈춘 PID의 실행 경로를 대조하고 해당 프로세스만 종료한 뒤 진단 앱을 다시 실행했다.
- 수정 직후 Windows PID `14228`, 같은 binary와 instance를 확인했다. 메모 `sentry-peer-windows-20261106`의 본문 `WINDOWS-FREEZE-FIX-20261106`, visible true, focused false, Responding true, frontend_errors 0, heartbeat 증가를 확인했다. 이 실행은 문서 커밋 `ebfc93515476` 위의 미커밋 수정 빌드이므로 revision만으로 원격 소스 변경을 식별하지 않는다. 실제 전달한 `capture.rs`와 빌드 로그를 함께 근거로 남긴다.
- 원격 desktop-control 권한이 없어 Ctrl+Shift+3 → 영역 선택 → 새 터미널 버튼 반응의 직접 GUI 재현은 사용자 확인을 요청했다. 앞선 1회 확인 뒤 재발한 경위를 보존하고, 종료 대기와 좌표 저장 호환을 포함한 최종 수정 앱(PID `20796`)에서 위치와 2~3회 생성·닫기의 사용자 확인을 다시 요청했다. 답변 전에는 최종 실환경 회귀 검증을 완료했다고 기록하지 않는다.
- 최종 재실행 후 프로세스 Responding true, frontend_errors 0, heartbeat 605까지 증가를 확인했다. 이 관측만으로 사용자 조작에 따른 재발이 없다고 판정하지 않는다. 현재 수정은 MacBook 소스와 Windows 개발 바이너리에 반영됐으며 커밋·push·iMac 반영은 수행하지 않았다.
- 위 PID `20796`의 실행 증거는 독립 감사 후 추가한 직렬화·관리 창 보강 전 바이너리다. 해당 소스 보강에 이 PID의 과거 실행 결과를 재사용하지 않는다. 새 빌드·재시작 결과와 최종 반복 동작은 후속 기록으로 구분한다.
- Astra 지적을 반영한 최종 소스의 MacBook Rust 전체 116개와 Windows 전체 102개, 양쪽 Clippy가 통과했다. Windows 작업자 실행·실패 뒤 lifecycle guard 해제·후속 정리 실행 검사도 포함했다. 전달한 네 파일(`capture.rs`, `terminal.rs`, `terminal/welcome.rs`, `manager.rs`)의 정규화 SHA-256이 일치했다. Windows 증거는 `%TEMP%\gibbon-freeze-astra-audit-20261106\`에 보관했다. 반복 GUI 회귀와 관리 창 복원 실행은 원격 desktop-control 권한이 없어 별도 확인 대기 상태다.

증거는 Windows `%TEMP%\gibbon-live-stacks-6436.jsonl`, `gibbon-live-stacks-main-6436.json`, `gibbon-freeze-fix-20261106\`와 `gibbon-freeze-fix-final-20261106\`의 중단 기록·빌드·검사 로그, MacBook `/tmp/gibbon-win-freeze-*.json`, `/tmp/gibbon-win-main-stack-response.json`, `/tmp/gibbon-freeze-fix-final-tests.log`, `/tmp/gibbon-freeze-fix-final-clippy.log`에 남겼다. 원문 로그는 저장소에 추가하지 않았다.
