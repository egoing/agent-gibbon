# #84 트레이 메모 관리 첫 선택 — 2026-11-07

## 확인된 현상과 원인

macOS ARM64에서 Finder를 활성화한 뒤 실제 개발 트레이의 `메모 관리`를 한 번 선택했다. 최초 생성과 최소화 복원은 창이 생성·복원되고도 Finder가 활성 앱으로 남았고, 다음 선택은 개발 앱을 활성화했다. 닫기 버튼으로 숨긴 뒤 복원, 이미 열린 창의 재선택은 한 번에 활성화했다. 창이 아예 생성되지 않은 현상과 구분한다.

| 시나리오 | 수정 전 한 번 선택 | 수정 후 |
|---|---|---|
| 최초 생성 | 창 존재, 활성 앱은 Finder (FAIL, 재실행에서는 PASS도 관측) | 표시·활성화 PASS |
| 이미 열린 창·다른 앱 활성 | 표시·활성화 PASS | 표시·활성화 PASS |
| 닫기로 숨긴 뒤 복원 | 표시·활성화 PASS | 표시·활성화 PASS |
| 최소화·다른 앱 활성 | 최소화 해제, 활성 앱은 Finder (FAIL) | 표시·활성화 PASS |

코드로 확인한 두 경로 차이:

- 최초 생성은 `builder.build()` 후 종료했고 기존 창에만 명시적인 `set_focus()`가 있었다. 잠금된 Tao 0.37.1의 생성은 `makeKeyAndOrderFront`를 호출하지만, 명시적 포커스 경로의 앱 활성화와는 다르다.
- 기존 창은 `unminimize → show → set_focus` 순서였다. Tao의 macOS `set_focus`는 `isMiniaturized`가 true이면 아무 작업도 하지 않는다. 비동기 최소화 해제 중에는 창 복원과 앱 활성화가 분리될 수 있다.

실제로 측정한 것은 선택 후 창 생성·복원·main 상태와 활성 PID다. 최소화 해제 순간의 내부 플래그를 계측한 것은 아니므로 애니메이션 중 guard가 실행된 정확한 시각은 의존성 코드에 근거한 원인 분석이다. 트레이 선택 후 창이 생성되므로 해당 재현에서 메뉴 이벤트 유실은 관측하지 않았다. 모든 간헐적 상황의 원인이 이것뿐이라고 단정하지 않는다.

## 최소 수정

관리 창의 최초 생성과 기존 창 표시가 같은 명시적 포커스 함수를 사용한다. macOS에서는 최소화 애니메이션의 guard와 별개로 현재 앱을 활성화한다. 모든 창 활성화 옵션·타이머·지연 재시도를 쓰지 않는다. 일반 메모의 포커스 보존, 창 위치·크기·숨김·핀, 전역 실행 정책·트레이 이벤트 처리·약관·저장 데이터는 변경하지 않는다.

공식 API: [Tauri set_focus](https://docs.rs/tauri/2.12.2/tauri/webview/struct.WebviewWindow.html#method.set_focus). 구체적인 동작은 로컬 lockfile에 해당하는 Tao 0.37.1 `platform_impl/macos/window.rs`의 생성·`set_focus`·`set_minimized`와 `util/async.rs`의 `set_focus`를 대조했다.

## 실행 출처와 TDD

- root: `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/84-tray-manager-first-click`
- instance: `dev-93d04163fc73ff36c6926322`
- binary: 위 root의 `src-tauri/target/debug/agent-gibbon`
- base: `37213fd0733f8c28f3891772031e85df051d21db`
- red: `bb0cc766f803a32843b5902e1dfb75d654409a5a` (`refs/is2/red/84`)
- 환경: macOS ARM64, Node v24.21.0, rustc/cargo 1.91.1, npm ci 수행.
- 모든 CLI는 해당 root의 `scripts/agent-gibbon-dev`를 사용했다. `inspect.ok`, instance, 고유 ID·본문과 실제 프로세스 실행 경로·PID·동일 PID의 네이티브 창 제목을 대조했다.
- 최초 조사 PID 99368: 첫 선택은 Finder PID 732, 두 번째 선택은 99368로 활성화. 최소화 복원도 Finder로 남았다. 기존 창 재선택·숨김 복원은 활성화했다.
- RED PID 2759: 최초 생성·최소화 복원 2건 FAIL, 기존 창·숨김 복원 PASS. 테스트의 assertion 실패(exit 1)이며 실행 환경 오류가 아니다.
- 첫 GREEN 시도 PID 6476: 최초 생성 PASS 뒤 다음 메뉴 준비에서 빈 AX 메뉴를 읽어 실패했다. `23996d7`에 실패 시도를 보존했다. 기존 트레이 검증 스크립트와 같이 포인터 진입·메뉴 refresh를 마우스 누르기 전에 전달하도록 준비 절차만 보강했다. 선택 횟수·창 활성화 assertion·관찰 시간은 완화하지 않았다.
- 재시도 PID 14389: 네 시나리오 PASS, 두 테스트 메모의 위치·크기·가시성·핀 보존 PASS.
- 최종 테스트의 수정 전 재대조 PID 16617: 최소화 복원 FAIL, 나머지 PASS. 최초 생성은 실행에 따라 PASS/FAIL이 달라 간헐적이다. 재대조 실행기의 “두 항목 모두 실패” 기대는 충족하지 않았으나 테스트 자체는 실제 활성화 assertion으로 RED(exit 1)를 유지했다. 원본 RED의 최초 생성 실패를 매 실행 재현한 것처럼 보고하지 않는다.
- 최종 GREEN PID 19674: 같은 최종 테스트의 네 시나리오와 메모 상태 보존 모두 PASS(exit 0). 각 선택 뒤 0.05/0.2/0.5/1초 간격의 네 snapshot을 기록하고 마지막 상태에서 창 존재·onscreen·main·최소화 해제·활성 PID 일치를 요구했다. 포커스 요청을 재시도하거나 메뉴를 두 번 선택하지 않았다.

테스트 실행: `python3 scripts/manager-open-smoke.py --pid <이 작업이 시작한 fresh dev PID> --report-dir <고유 경로>`.
새 관리 창이 아직 없는 프로세스에서 실행하며 테스트 스크립트는 앱을 종료하지 않는다. 테스트가 만든 메모만 숨기고 이전 활성 앱과 포인터를 복원한다. 재시작은 실행 파일 경로를 확인한 이 작업 소유 PID만 종료했다.

원본 로그·JSON: `/tmp/is2-agent-gibbon-84/`의 `baseline-*.log`, `baseline.json`, `investigation.json`, `red.log`, `red/report.json`, `green.log`, `green/report.json`, `retry1.log`, `retry1/report.json`, `gate-*.log`, `gates.json`, `replay-*`. 과거 다른 워크트리 결과를 이번 증거로 사용하지 않았다.

[PID·명령·활성화 snapshot 기록](issue84-manager-open/evidence.json) · [실제 관리 창 캡처](issue84-manager-open/manager.png). 캡처는 PID 19674의 네이티브 창 제목과 window ID 43732를 확인하고 `orca computer get-app-state --app pid:19674 --window-id 43732 --json`으로 얻었다. 캡처 전에 동일 프로세스·instance·고유 메모를 재확인했다. 캡처 자체는 다른 앱으로 포커스를 복원한 뒤의 외형이며, 활성화 통과 근거는 JSON의 시나리오별 활성 PID다.

## 게이트와 한계

기준 트리의 Rust 163·UI 116, cargo check --all-targets, Rustfmt/Clippy -D warnings, 개발 빌드는 모두 PASS였다. baseline 제외 0건.
최초 구현의 같은 회귀·타입·린트와 `npm run build`(macOS .app 생성)는 PASS. 네이티브 검증은 위 재시도와 최종 대조 결과를 구분한다. 서명 공증·설치 검증은 수행하지 않았다.

독립 감사(`d100e641025c74114f3ee76dc7ad9cc5b95ad5ca`, 구현 대화 이력 미상속)는 테스트 완화·퇴행·스코프 이탈 모두 미발동, Risk Low로 판정했다. 감사자는 원본 로그·diff를 대조했으며 GUI를 직접 재실행했다고 주장하지 않았다. 원격 main 전진 후 재배치된 최종 SHA의 게이트·재감사·전체 플랫폼 검사 URL/headSha/플랫폼별 결과는 [PR #85](https://github.com/egoing/agent-gibbon/pull/85)에 SHA별로 기록한다. 이 문서의 앞선 PID 실행을 재배치 후 실행 증거로 대신하지 않는다.

아래 추가 Windows 검증과 별개로 Intel의 실제 UI, 다른 Spaces/전체화면 앱·Stage Manager·모든 OS 버전은 미검증이다. 원격 빌드 성공을 네이티브 UI 검증으로 대신하지 않는다.

## 추가 Windows 실앱 검증

사용자가 지정한 Windows Orca 터미널 `term_e027f9f7-0ca3-4fb5-94fb-682fea2a7d84`의 에이전트가 별도 테스트 워크트리에서 `ce9ab3e7e39aae92a68ccf5c72ffb79a4b7f5fc6`를 빌드·실행했다. 이 추가 기록은 제품 코드·테스트를 변경하지 않는다.

- OS: Windows 11 Home x64, 10.0.26200.
- root: `C:/Users/egoin/orca/workspaces/agent-gibbon/verify-84-ce9ab3e-win`
- instance: `dev-acef1e9fb1b11b355a0ec313`; 고유 검증 메모: `verify84-visible-ce9ab3e`, `verify84-hidden-ce9ab3e`.
- binary: 위 root의 `src-tauri/target/debug/agent-gibbon.exe`
- 실제 관리 창: PID 24280, HWND 6160490, 네이티브 제목 `Agent Gibbon [DEV egoing/verify-84-ce9ab3e-win · acef1e9f] — 메모 관리`.
- `npm ci`, `npm run dev:build`, 해당 워크트리 개발 CLI의 open/close/inspect는 exit 0. CLI instance·고유 메모와 실제 바이너리 PID·네이티브 창 제목을 대조했다.
- 도구: Rust/Cargo 1.99.1, Node 20.11.0, npm 10.2.4, Tauri CLI 2.12.1, Orca 1.4.250, WebView2 156.0.4314.58, 단일 1920×1080 화면.
- 최초 생성·닫기로 숨긴 뒤 복원·최소화 복원·다른 앱 활성 상태 각각 트레이의 `메모 관리` **한 번 선택**으로 표시 및 foreground HWND/PID 일치 PASS. 최소화 준비는 `IsIconic=true`로 확인했다. 두 검증 메모의 위치·크기·가시성·핀 보존 PASS.
- Orca GUI 조회는 exit 1이어서 해당 도구 성공으로 기록하지 않았다. Windows UI Automation, 실제 마우스 입력, Win32 창/foreground 상태 및 캡처를 대조했다.
- 각 선택 뒤 약 1.9초 동안 30개 네이티브 상태 표본을 관찰했다. 네이티브 팝업 항목은 UI Automation에 노출되지 않아 소유 PID를 확인한 HMENU를 읽었다. 장시간 반복·스트레스, 핀 해제(두 메모 모두 pinned=true), 사용자 지정 크기, Windows 10/ARM64·다중 모니터·혼합 DPI는 미검증이다. 해당 Windows 에이전트는 전체 테스트 suite·릴리스 패키징을 수행하지 않았고 별도의 원격 CI 결과와 구분한다.
- 원본 보고서·JSON·캡처·명령 로그·SHA256 목록은 Windows의 `C:/Users/egoin/AppData/Local/AgentGibbonValidation/issue84-ce9ab3e-20261107/`에 보존한다(`report.txt`, `summary.json`, `scenario1-after.png` 등, `artifacts-sha256.csv`). 사용자 main과 기존 앱을 보존하고 이 검증에서 시작한 PID만 종료한 뒤 원래 활성 창을 복원했다.

이 결과는 수정 후 Windows 동작 검증이며 Windows에서 수정 전 증상을 재현했다고 주장하지 않는다. 문서 보강 전 코드 SHA의 실제 실행과 문서 보강 후 최종 CI를 구분하고 [PR #85](https://github.com/egoing/agent-gibbon/pull/85)에 기록한다.

`is2` 커버리지 체크리스트와 대조했다. 이번 실행은 기존 Low 백엔드 및 검증 준비 실패 후 재시도 축에 해당하며, 새 위험도·충돌·정직성 의심·롤백 시나리오를 검증한 것으로 표시하지 않는다. 사용 에이전트는 Codex(현재 세션, 상세 모델 버전 미노출), 실사용 이슈 #84 / PR #85다.

[디자인 시스템](../design-system.md)의 관리 창 열기 적용 기록에 연결한다. 외형·배치·문구가 바뀌지 않아 기존 SVG/PNG는 유지한다.
