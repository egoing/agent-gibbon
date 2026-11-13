# 일반 시작 창 복원 검증 — #150

## 대상과 출처

- 작업 루트: `/Users/egoinglee/.codex/worktrees/af67/agent-gibbon`
- 브랜치: `codex/startup-window-restore`, 기준 main `9904f10770d552b8f90ceb8073415d9875c09062`
- 계획 커밋: `0dca6cd`. 이후 사용자에게 #150 최종 커밋·push·병합 승인을 받아 제품 구현과 이 검증 기록을 PR #151로 제출했다. 최종 구현 SHA와 병합 SHA는 PR 및 이슈의 완료 기록에서 확인한다.
- 개발 인스턴스: `dev-31e614327480dc0b4df74402`
- 실행 파일: `/Users/egoinglee/.codex/worktrees/af67/agent-gibbon/src-tauri/target/debug/agent-gibbon`
- `npm run dev:info`로 root/id/binary를 확인하고 같은 루트의 `scripts/agent-gibbon-dev inspect`에서 `ok:true`·instance·검증 ID·본문을 대조했다. [최초 응답](startup-window-restore/instance-proof.json), [최종 응답](startup-window-restore/final-visible.json).
- 환경: macOS ARM64 (`uname -m: arm64`), Node v24.21.0, Cargo/Rust 1.91.1. 설치 앱·기본 체크아웃·다른 워크트리 앱은 변경/종료하지 않았다.

## 구현 동작

[디자인 시스템](../design-system.md#앱-시작-시-창-복원--150)에 사용자 동작을 관리한다. 일반 journal은 Store의 인스턴스별 docs 아래 `.session-windows.json`에 tempfile + sync + 원자적 교체로 저장한다(Unix tempfile 기본 0600). 1초 checkpoint는 동시에 한 개만 main thread에 예약한다. 정상 종료는 창 파괴 전 `RunEvent::ExitRequested`에서 저장한다. 진행 중인 등장·크기 측정·stash는 checkpoint를 미룬다.

업데이트 일회성 snapshot이 먼저 성공하면 일반 journal은 적용하지 않는다. 이 작업은 기존 업데이트의 웹·터미널·접힌 창 보류 기준을 완화하지 않는다. #118의 URL 재열기·PTY 자동실행 금지 정책을 일반 시작에 준용한 범위다.

메모 내용은 기존 open/update/save_edit의 동기 Store 저장을 그대로 사용한다. 배치 checkpoint가 같은 bytes로 생략돼도 내용 저장은 별도로 완료된다. 매 창 이동마다 전체 메모 내용을 다시 쓰지 않는다. 터미널의 원시 입력·출력·히스토리를 파일에 저장하지 않으며, 메모리 버퍼도 고정 안전 명령의 접두어만 유지한다. 안전 문자열 이외의 입력은 즉시 버리고 마지막 명령 표시를 비운다. 새 PTY를 명시 시작하면 이전 실행의 명령 표시도 초기화한다.

## 로컬 검사

| 검사 | 결과 |
|---|---|
| `cargo test --manifest-path src-tauri/Cargo.toml` | 220 통과 |
| `npm run test:ui` | 188 통과 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 통과 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 통과 |
| `npm run dev:build` | 통과 |
| `git diff --check` | 통과 |
| `rsvg-convert docs/design/terminal-restore.svg -o docs/design/terminal-restore.png` | 렌더 후 육안 확인 |

추가 Rust 검사는 숨긴 접힘 웹 snapshot 왕복, 악성 URL·잘못된 ID·잘못된 크기·중복·버전 거부, 비밀을 포함할 수 있는 입력·escape 편집 제외, 두 화면에 걸친 위치 유지와 가장 가까운 화면 보정을 다룬다.

## 실제 실행

CLI는 전부 이 루트의 `./scripts/agent-gibbon-dev`를 사용했다. 처음 `open --id restore-visible --markdown STARTUP-RESTORE-AF67-visible-v1`, 별도 메모 두 개, `web --id web-restore-a --url https://example.com/`, `web --id web-restore-b --url https://example.org/`를 생성했다. CLI 등장 애니메이션이 포인터 이동을 기다리는 동안 journal을 쓰지 않는 것을 확인했고, 해당 메모를 close한 뒤 실제 journal 생성과 숨김 기록을 확인했다. 내용만 v2로 update한 메모의 최신 본문도 재시작 후 유지됐다.

| 실행 | 근거와 결과 |
|---|---|
| 강제 종료 후 시작 | 정확한 경로의 자체 PID 6800·8026만 SIGKILL. 고정 배치/접힘/웹/터미널 [fixture](startup-window-restore/seed.json)를 앱이 정지한 상태에서 주입하고 `--serve` 재실행. [보이는 메모](startup-window-restore/restored-visible.json) 620×340, [숨긴 메모](startup-window-restore/restored-hidden.json) 숨김, [접힌 메모](startup-window-restore/restored-folded.json) 600×48/접힘 및 위치 단언 통과. 이는 UI로 해당 배치를 만든 E2E와 구분한다. |
| 웹 두 개 | [웹 A](startup-window-restore/restored-web-a.json) 표시·example.com, [웹 B](startup-window-restore/restored-web-b.json) 숨김·example.org, 위치·크기 유지. 이전 URL로 네트워크 탐색하며 로그인/페이지 내부 상태를 복원하지 않는다. |
| 터미널 두 개 | [A](startup-window-restore/restored-terminal-a.json) 표시·git status, [B](startup-window-restore/restored-terminal-b.json) 숨김·명령 저장 제외. 모두 running:false/pid:null. 명령값은 fixture로 주입했고 자유 입력→파일 기록의 native 종단 검증으로 계산하지 않는다. 안전 명령 입력 필터는 Rust 검사로 확인했다. |
| 닫힌 메모 | [closed:true,visible:false](startup-window-restore/restored-closed.json). 일반 무인자 시작에서도 닫힘 상태 유지. |
| 이미지 | 초기 fixture에는 `capture.png`가 없어 파일 오류가 보였으므로 **내용 복원 통과로 계산하지 않았다**. 유효한 청색/황색 [원본 fixture PNG](startup-window-restore/image-fixture.png)를 실제 저장 경로에 만든 뒤 재시작해 [원래 이미지 표시](startup-window-restore/image-native.png)를 육안 확인했다. [최종 inspect](startup-window-restore/final-image.json). |
| 정상 종료 | 자체 PID 8623에 `NSRunningApplication(processIdentifier:8623).terminate()` 사용. [terminate:true·terminated:true 및 frontmost 92046 전후 동일](startup-window-restore/normal-quit.txt). 종료 후 이미지의 펼친 크기와 closed_notes가 journal에 남았다. SIGTERM을 정상 종료로 취급하지 않았다. |
| 실제 PTY 정리 | 자체 PID13794의 terminal-15001에서 셸을 명시 실행하여 child PID14672/running:true 확인 후 앱 정상 종료. child PID 소멸 확인. 최종 PID15786에서 [running:false,pid:null](startup-window-restore/final-terminal.json), 명령 자동실행 없음. 이 시행 중 frontmost가 ChatGPT92046→Keynote6218로 바뀌어 포커스 보존 통과 근거로 사용하지 않았다([기록](startup-window-restore/live-pty-normal-quit.txt)). 사용자 작업을 덮어쓰는 강제 포커스 복원은 하지 않았다. |
| 업데이트 우선 | 일반 journal은 visible:true지만 업데이트 fixture는 visible:false·900,200·500×400. 재시작 후 [업데이트 값](startup-window-restore/update-priority.json) 적용, 일회성 파일 소비, 일반 웹 snapshot 미적용 확인. 실제 업데이트 설치는 실행하지 않았다. |
| 손상 파일 | `{deliberately-broken-fixture`를 정지한 자체 인스턴스에 넣고 시작. 해석 실패 안내·checkpoint 중지, 주기 이후 및 정상 종료 뒤에도 bytes 동일 유지 [확인](startup-window-restore/corrupt-file-preserved.txt). 이후 정상 fixture로 복구했다. |

최종 빌드 재시작 PID15786, [보이는 메모](startup-window-restore/final-visible.json)·[이미지](startup-window-restore/final-image.json)·[닫힌 메모](startup-window-restore/final-closed.json)·[터미널](startup-window-restore/final-terminal.json)·[웹](startup-window-restore/final-web.json)의 instance와 상태를 다시 확인했다.

## GUI 출처와 한계

CUA는 unbundled debug 경로를 `Invalid app`으로 거부했다. 팀리더가 예약한 GUI 슬롯에서 computer-use 스킬과 `/usr/local/bin/orca`의 PID 지정 백그라운드 경로를 사용했다. `list-windows`는 빈 목록이었지만 `get-app-state`는 실제 창 제목·ID·캡처를 반환했다. CoreGraphics로 해당 PID의 창 번호만 읽고 각 번호의 native 제목을 확인한 뒤 목표 ID를 선택했다.

- 최신 빌드 PID15786의 실제 바이너리 경로를 ps로 재확인하고 inspect instance를 대조했다. 최신 image-native.json 및 terminal-native.json의 native 제목에 검증 ID가 포함된다. 각 JSON에는 PID·창 ID·좌표·캡처 출처가 있다. 이미지와 터미널 캡처는 사용자 화면 전체가 아닌 검증 창만 포함한다.
- 이전 PID8623의 restore-image/68127에서 AXPress 후 inspect folded:true 확인. CmdQ는 window_not_found로 실패했으므로 정상 종료 성공 근거로 사용하지 않았다. 이후 명시된 NSRunningApplication 대안을 사용했다.
- Windows·Intel GUI, 실제 모니터 분리·혼합 DPI, 실제 updater 설치, 자유 입력의 native 키보드→PTY→journal 종단, 비정상 종료 직전 1초 미만 배치 손실은 별도 검증하지 않았다. 물리 모니터 분리의 대체 근거는 순수 보정 함수 테스트다.
- 미저장 편집 초안·PTY 화면/프로세스·셸 이력·웹 로그인/탭 내부 상태·글자 크기를 새로 영속화하지 않는다. 접힘과 크기는 지원한다. 새 기능은 설치 앱에 아직 배포되지 않았다.

Conf: 5 (이 Mac의 실제 개발 앱 재시작/정상 종료/CLI/native 일부 확인; 위 미실행 범위 제외). Risk: Medium (로컬 세션 상태 및 시작 동작 변경, 원본 저장 내용과 복구 파일은 보존).

검증 종료 후 이번에 시작한 최종 PID15786만 정상 종료했다([정리 결과](startup-window-restore/cleanup.txt)). 테스트 데이터와 미커밋 구현은 전용 워크트리에 보존한다. GUI 슬롯 사용을 마쳤으며 다른 사용자 앱은 종료하지 않았다.

## 병합 승인 후 재검증

2026-11-13 최신 origin/main `9904f10` 기준으로 #150만 독립 제출한다. 이전 검증 이후 제품 변경은 없으며 Rust 220개·UI 188개·Clippy·Rustfmt·개발 빌드와 diff 공백 검사를 다시 수행했다. 실제 native 검증은 위에서 기록한 동일 제품 코드의 실행 근거이며 이번 제출 단계에 GUI를 재조작하지 않았다. 전체 플랫폼 CI와 배포는 실행하지 않았다.

af67 및 기본 main 체크아웃에서 발견한 `package-lock.json` 변경은 의존성 추가가 아닌 필드 정렬이며 이번 기능의 변경이 아니다. 두 파일의 재개 시 SHA256은 `45c183f012d19d8493392c0faab2edec85702759d9977ee2f7265d74128023c7`로 같았고, 스테이징에서 제외하고 그대로 보존한다.
