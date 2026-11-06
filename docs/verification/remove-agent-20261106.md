# 전용 에이전트 기능 제거 — #76 후속, 2026-11-06

사용자 요청에 따라 트레이 표시와 전용 에이전트 실행 기능을 함께 제거했다. 기존 트레이 개선 변경 위에 적용했으며 커밋·push·배포는 하지 않았다. [디자인 시스템](../design-system.md#전용-에이전트-기능-제거--76-후속-2026-11-06) · [현재 SVG](../design/tray-memo-list.svg) · [PNG](../design/tray-memo-list.png).

## 변경 범위

- `영역 에이전트 터미널` 트레이 생성 명령, 전용 영역 선택 모드, 저장된 전용 에이전트의 목록·관리·복원을 제거했다. 최근 30개 집계도 메모·캡처·일반 터미널 세 유형만 포함한다.
- `agent` CLI 명령과 IPC action, 자동 Codex/Claude 탐색·캐시·자동 실행 모듈, 전용 WebView 자동 시작·숨김 IPC·창 이벤트 처리를 제거했다. 일반 터미널은 항상 Welcome → 사용자 선택 → 실행으로 시작한다. 일반 터미널에서 명시적으로 고르는 추천 앱·셸과 메모 관리용 스킬은 유지한다.
- 기존 `kind=agent`, `agent-숫자` 기록은 로드하지 않는다. 메타데이터·작업 폴더를 변경·삭제하지 않는다. 예전 ID의 open/update/inspect/close는 거절하여 일반 메모로 오인하거나 `index.md`를 덮어쓰지 못하게 했다. `agent-notes` 같은 일반 메모 ID는 허용한다.
- 옛 에이전트 메타데이터가 없어도 예약된 프로젝트 폴더의 `index.md`를 메모로 읽지 않는다. 실제 메타데이터가 있는 경우와 없는 경우 모두 파일 보존·반복 로드를 검사했다. 일반 터미널의 저장 위치·ID·입력 조합·PTY 동작은 유지한다.
- README와 현재 디자인 기준·트레이 SVG/PNG를 갱신했다. 과거 에이전트 시안·검증 자료는 현재 기능과 구분하여 이력으로 보관한다. 이전 트레이 기록의 에이전트 관련 결과는 현재 구현의 증거로 재사용하지 않는다.

## 자동 검사

환경: macOS ARM64, Node v24.21.0, rustc/cargo 1.91.1, rsvg-convert 2.62.3.

| 명령 | 최종 결과 |
|---|---|
| `npm test` | Rust **150/150**, UI **97/97** 통과 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | 통과 |
| `npm run check` | Clippy all-targets, `-D warnings` 통과 |
| `npm run dev:build` | 해당 워크트리 debug 빌드 통과 |
| `python3 -m py_compile scripts/tray-menu-polish-smoke.py` | 통과 |
| `git diff --check` | 통과 |
| SVG→PNG 렌더링 | 전용 생성 행·에이전트 목록 행 제거 후 육안 확인 |

[명령 결과 요약](remove-agent-20261106/validation.txt). 전용 탐색·자동 실행 코드와 그 전용 시험은 함께 삭제했다. 일반 터미널의 폴더 선택·명시적 Run·명령 종료 뒤 같은 PTY 복귀·한글 입력 처리·busy/자식 정리 검사는 유지했다. 새 회귀는 다음을 검증한다.

- CLI `agent`, IPC의 제거된 action, 이전 숫자 에이전트 ID가 거절된다.
- kind가 없는 옛 메타데이터와 명시적 `kind=agent`는 제외하고 일반 터미널만 읽는다. 두 번 로드해도 원래 메타데이터와 프로젝트 `index.md` 바이트가 같다.
- 전용 메타데이터가 없는 프로젝트 폴더도 보존하며 정상 메모를 계속 로드한다.
- 지원하지 않는 옛 UI 모드는 PTY를 자동 실행하지 않는다. 관리 UI도 오래된 agent 행을 표시하지 않는다.
- 세 유형의 0/30/31개·동률 정렬, 이벤트 라우팅·아이콘 알파·표시용 키 이름과 기존 Notes 잠금 회귀를 유지한다.

첫 Clippy 실행에서 삭제한 모드의 남은 import/변수를 검출해 제거한 뒤 위 최종 검사를 통과했다. 검사를 약화하거나 warning을 허용하지 않았다.

## 최종 빌드 실제 확인

- 루트: `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/tray-menu-polish`
- 브랜치: `tray-menu-polish`, 시작 SHA: `9f015a6b9198aae3c7765fdf53988d330aafc8c0`
- instance: `dev-1fd207d59c82431a32969a4a`
- binary: 위 루트의 `src-tauri/target/debug/agent-gibbon`
- **최종 PID 63665**, SHA-256: `251c356e5657ebd0790a4990382f5f18c20fff2a254fc081c49b51c7c26d03db`
- 고유 검증 메모: `tray76-removal-final-20261106`, 본문 `검증 표식: tray76-removal-final-20261106`

`npm run dev:info`, 해당 개발 CLI inspect의 ok/instance/id/본문, 실제 프로세스 comm 절대 경로와 PID, 같은 PID의 고유 메모 ID를 포함한 네이티브 창 제목을 대조했다. 개발 명령은 모두 이 루트의 `scripts/agent-gibbon-dev`만 사용했다. 기존 진행 메모는 숨김 상태를 보존하고 최종 출처 확인에 별도 검증 메모를 사용했다.

[최종 실행 원시 기록](remove-agent-20261106/final-restart.json) · [최종 실제 메뉴](remove-agent-20261106/menu-final-restart.png) · [관리 창 AX](remove-agent-20261106/manager-final-restart.txt).

- 최근 목록에서 전용 생성 항목·저장된 에이전트가 사라졌고 실제 CLI list도 kind=agent/이전 숫자 ID를 포함하지 않았다. 최종 저장 목록 32개에서 30개+`더보기(2)`가 나오고 같은 PID의 관리 창 **41418**에는 32개 전체가 나왔다.
- 설치 앱·원래 main·다른 워크트리를 종료하지 않았다. 각 재빌드 후 자신이 시작한 정확한 개발 PID만 종료·재실행하고 출처를 다시 확인했다.
- `agent` 명령과 옛 실제 에이전트 ID의 open이 exit 1/ok=false로 거절됐다. 이전 전용 메타데이터의 SHA-256은 처음 읽은 값과 같았다. 최종 재시작 후 자동 실행된 PTY도 없었다.
- 메뉴는 PID가 일치하는 CG layer 101 창의 범위만 캡처했다. 최종 기록에 해당 bounds를 보관했다. SVG/PNG 시안과 실제 캡처는 구분한다.

### 일반 터미널과 30/31개 실환경 회귀

같은 작업 루트·인스턴스의 앞선 **PID 31185**(마지막 orphan-project 읽기 방어 추가 전 빌드)에서 일반 터미널 회귀를 수행했다. 전용 기능 제거·일반 터미널 실행 코드는 최종 빌드와 같지만 이 기록을 최종 PID의 실행 기록으로 대신하지 않는다.

[실행 기록](remove-agent-20261106/native-final.json) · [실제 메뉴](remove-agent-20261106/menu-final.png) · [30개](remove-agent-20261106/menu30-no-agent.png) · [31개](remove-agent-20261106/menu31-no-agent.png).

- 기존 실제 전용 에이전트 메타데이터는 디스크에 남아 있지만 목록에서는 빠져 처음 30개가 됐다. 더보기 없음 → 자기 검증 메모 1개 생성 후 31개·더보기(1), 전체 관리 목록 31개를 확인했다.
- 일반 터미널 `terminal-1793961625951752000`은 자동 실행 없이 Welcome으로 열렸다. 실제 셸 선택·실행 버튼을 누른 뒤 **PTY PID 52052**, running=true를 확인했다. CLI 숨김 후 visible=false/running=true, 트레이 복원 후 visible=true/running=true와 같은 PID를 확인했다.
- 자기 검증 셸의 정확한 PID·부모 PID 31185를 대조해 SIGHUP으로 정리하고 running=false·visible=false를 확인했다. 다른 에이전트나 PTY를 종료하지 않았다.
- 동일 데이터로 경계 스크립트를 다시 실행할 때 이미 31개라 ‘시작 30개 이하’ 전제에서 중단됐다([재실행 문맥](remove-agent-20261106/rerun-precondition.json)). 재실행을 통과로 보고하지 않으며, 첫 실행의 30/31개 캡처와 이후 독립 검사를 구분했다. 일시적인 AX 시간 초과는 이후 정상 스냅샷을 새로 읽은 뒤 실제 상태로 재검증했다.

## 한계와 상태

- 실제 실행은 macOS ARM64다. Windows·Intel 실행 및 전체 플랫폼 workflow_dispatch는 미실행이다. 커밋·push 승인 후 최신 SHA로 전체 플랫폼 검사가 필요하다. 공개 배포·자동 병합은 승인받지 않았다.
- 원래 요청에서 남긴 Windows 메뉴 제목 알파·키보드만의 재열기 정렬 한계는 별도이며 이번 제거 작업에서 바꾸지 않았다.
- 예전 전용 프로젝트·메타데이터를 영구 삭제하거나 설치 앱의 에이전트 8개를 정리하는 운영 작업은 하지 않았다. 사용자가 삭제한 전용 기능은 새 빌드에서 접근·실행할 수 없다.

검증한 macOS 경로 **Conf 5**. 의도적인 CLI/action 제거·복원 대상 변경이므로 **Risk Medium**이며, 원래 파일은 보존한다. 커밋 승인 전 상태다.
