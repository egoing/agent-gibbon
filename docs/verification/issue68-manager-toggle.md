# 공통 메모 표시 상태와 토글 — 2026-11-06

[이슈 #68](https://github.com/egoing/agent-gibbon/issues/68)의 공통 표시 도구에 사용자가 요청한 표시 상태 조회·표시/숨김 전환을 추가했다. 이전 표시 전용 버전의 [네이티브 검증 기록](issue68-manager-show.md)은 이번 토글의 실행 근거로 재사용하지 않는다. UI 기준은 [디자인 시스템](../design-system.md#메모-공통-화면-표시-도구--68-2026-11-06)을 따른다.

## 동작

- 선택한 메모의 실제 네이티브 창 상태를 읽는다. 창이 없거나 숨겨졌거나 최소화됐으면 `숨김`, 표시 중이면 `표시 중`을 버튼 옆에 표시한다. Notes 잠금은 native 창 조회 전에 해제한다.
- `숨김`에서는 `화면에 띄우기`, `표시 중`에서는 `화면에서 숨기기`를 제공한다. 숨기기는 기존 CLI close와 같은 창 숨김이며 메모 삭제·창 종료·PTY 종료가 아니다. 공통 도구는 제목/본문 위에 하나만 있고 Settings에는 없다.
- 버튼의 의미에 맞는 명시적 목표 상태를 서버에 보낸다. 화면을 읽은 직후 다른 곳에서 상태가 바뀌어도 서버에서 다시 반전시키지 않는다. 결과 상태도 native 창에서 다시 읽어 반환한다.
- 외부 CLI·툴바에서 바뀐 상태를 기존 2초 조회·관리 창 포커스·새로고침으로 반영한다. 상태만 바뀔 때 상세 DOM을 보존해 버튼 포커스와 오류 안내를 유지한다. 실행 중 중복 요청을 막고 요청 전 시작된 오래된 조회는 완료 상태를 덮어쓰지 못한다.
- 실패는 상태를 성공으로 바꾸지 않고 공통 도구의 alert로 표시한다. 다른 메모 선택 뒤 도착한 실패는 새 상세에 표시하지 않는다.

## 출처

- 작업 루트: `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/memo-show-common`
- 브랜치: `feat/memo-show-common`
- `npm run dev:info`의 root는 위 루트, ID는 `dev-2420851a18a85d0606b0c157`, binary는 위 루트의 `src-tauri/target/debug/agent-gibbon`이다.
- 도구 환경은 같은 세션에서 확인한 macOS ARM64, Node v24.21.0, npm 11.19.0, rustc/cargo 1.91.1, tauri-cli 2.12.1이다.
- 기존 미커밋 #68 구현 위에 후속 변경했다. 원본 프로젝트 폴더는 수정·복사하지 않았다. 커밋·push·병합은 하지 않았다.

## 통과한 검증

| 명령/범위 | 결과 |
|---|---|
| `npm test` | Rust 129개·UI 87개 통과 |
| `npm run check` | 전체 target Clippy, 경고 오류 처리 통과 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | 통과 |
| `node --check ui/manager.js` 및 `node --check scripts/manager-show-smoke.mjs` | 통과 |
| `npm run dev:build` | 현재 워크스페이스의 debug 바이너리 빌드 성공 |
| `git diff --check` | 통과 |

추가 UI 회귀 검사는 네 유형의 표시→숨김→표시, 외부 상태 변경 뒤 동일한 도구 DOM 유지, 숨김 실패 시 원래 상태·오류 유지, 토글 전 시작한 조회 응답의 역전 방지를 확인한다. 기존 선택·Settings·빈 목록·중복 요청·선택 전환 검사도 유지한다.

### Chromium

```sh
PLAYWRIGHT_MODULE=/tmp/agent-gibbon-is2-57/browser-tools/node_modules/playwright/index.mjs node scripts/manager-show-smoke.mjs
```

- Chromium 156.0.8078.4에서 실제 UI 자산과 모의 네이티브/약관 응답을 사용했다. 실제 동의 파일은 변경하지 않았다.
- 944×480·600×360에서 제목 있는 텍스트·제목 없는 텍스트·빈 텍스트·이미지·범용 터미널·에이전트 12개 조합을 검사했다. 각각 표시→숨김→표시의 명령·목표 상태·버튼 문구·현재 상태를 확인했다.
- 2초 조회를 거친 뒤에도 버튼의 키보드 포커스 유지, 외부 상태 변경 반영, 실패 후 원래 상태 유지·오류 표시·재시도, Settings·빈 목록의 도구 제외를 확인했다. JavaScript 오류 0개.
- [표시 중인 텍스트](issue68-manager-toggle/browser-text-944.png), [표시 중인 터미널](issue68-manager-toggle/browser-terminal-600.png), [실패와 숨김 상태](issue68-manager-toggle/browser-error-600.png)는 브라우저 모의 데이터 기반 캡처다. 실제 IPC·창·PTY 검증이 아니다.

### 디자인

텍스트 시안은 `화면에 띄우기 · 숨김`, 이미지·터미널 시안은 `화면에서 숨기기 · 표시 중`을 표현한다. SVG 3개를 rsvg-convert 2.62.3으로 PNG로 렌더링하고 직접 열어 배치·문구·잘림 없음을 확인했다. Settings 자산은 변경하지 않았다.

[텍스트 SVG](../design/manager-notes-wireframe.svg) · [PNG](../design/manager-notes-wireframe.png), [이미지 SVG](../design/manager-image-wireframe.svg) · [PNG](../design/manager-image-wireframe.png), [터미널 SVG](../design/manager-terminal-wireframe.svg) · [PNG](../design/manager-terminal-wireframe.png).

## 사용자 승인 재시작과 실제 검증

사용자가 `재실행`을 지시해 실행 중인 Codex 세션 종료를 수반하는 개발 앱 재시작을 승인했다. 이 검증은 위 최초 브라우저 검사 이후 같은 워크스페이스에서 수행했다.

### 실행 출처

- `npm run dev:build`를 다시 실행해 성공을 확인했다. 기존 `ps`·`lsof -a -p 51335 -d txt -Fn`의 실제 바이너리 절대 경로가 이 작업 루트와 일치함을 확인했다.
- 재시작 전 같은 개발 CLI로 모든 항목을 inspect해 `ok: true`·예상 instance를 확인하고, 표시 상태·본문·실제 위치·크기와 PID별 CGWindow 목록을 `/tmp/gibbon-issue68-toggle-restart-before.json`에 기록했다. 앱의 포커스 대상은 기존 PID 51335였다.
- `NSRunningApplication(processIdentifier: 51335).terminate()`가 true를 반환했고 기존 앱과 Codex PID 65873의 종료를 확인했다. 다른 인스턴스는 종료하지 않았다. 강제 종료는 사용하지 않았다.
- 같은 루트의 `./scripts/agent-gibbon-dev open --id issue68-progress-2420851a`로 새 개발 앱을 실행했다. 새 PID는 **22648**, 실제 실행 파일은 위 root의 `src-tauri/target/debug/agent-gibbon`이다. `ps`와 `lsof -a -p 22648 -d txt -Fn`으로 대조했다.
- 새 서버의 inspect가 `ok: true`, instance `dev-2420851a18a85d0606b0c157`, 메모 ID `issue68-progress-2420851a`, 본문 표식 `ISSUE68-2420851A-20261106`을 반환했다. 같은 표식을 관리 창의 상세에서도 대조했다.
- `orca computer list-windows --app pid:22648 --json`의 관리 창 ID **39218**과 정확한 native 제목으로 GUI 대상을 특정했다. 새 PID의 CGWindow 목록에서 메모 ID를 포함한 텍스트 창 39201, 이미지 창 39200, 에이전트 창 39212의 native 제목도 확인했다.

### 통과한 실제 동작

PID·정확한 창 제목을 지정하는 저장소의 `scripts/design-system-ax.swift`로 목록 선택과 공통 버튼의 AXPress를 실행하고, 각 단계의 실제 개발 CLI inspect 및 접근성 트리를 재조회해 효과를 확인했다. AXPress 호출 성공 자체를 동작 성공으로 간주하지 않았다.

| 유형 | 메모 ID | 실제 전환 | 결과 |
|---|---|---|---|
| 텍스트 | `issue68-progress-2420851a` | 표시→숨김→표시 | 버튼 한 개·문구·실제 상태 일치 |
| 이미지 | `image-1793936326662915000` | 표시→숨김→표시 | 버튼 한 개·문구·실제 상태 일치 |
| 에이전트 | `agent-1793936218271001000` | 표시→숨김→표시 | 토글 전후 세션 PID 23059·running 유지 |
| 범용 터미널 | `terminal-1793936350225402000` | 숨김→표시→숨김 | 창이 없던 상태에서 Welcome 재생성·표시·숨김 확인 |

- 각 유형의 목록 선택만으로는 원래 표시 상태가 바뀌지 않았다. 버튼 실행 뒤 실제 visible과 `표시 중`/`숨김`·행동 문구를 대조했다. 각 토글의 전후 전면 앱 PID가 동일했다.
- 첫 자동 갱신 검사는 가려진/비활성 관리 창에서 수행돼 버튼 갱신이 멈추는 실패가 있었다. 새로고침 시 실제 상태가 반영됐다. 이 실패를 통과로 기록하지 않았다. 관리 창에는 `!document.hidden`일 때만 정기 조회하는 조건이 있으므로, 표시된 관리 창의 동작과 구분했다. 앱 재시작 전 포커스가 관리 앱이었던 점에 맞춰 `orca computer get-app-state --app pid:22648 --window-id 39218 --restore-window --json`으로 관리 창을 화면 앞으로 복원한 뒤 재검증했다.
- 표시된 관리 창에서 외부 개발 CLI close는 약 **1.64초**, open은 약 **1.76초** 뒤 별도 새로고침 없이 올바른 버튼으로 바뀌었다. 최초의 즉시 상태 단정 검사도 주기 조회·표시 애니메이션 완료를 기다리는 검사로 다시 수행했다. 네 유형의 최종 검사는 모두 통과했다. 가려진 창에서 실시간 정기 갱신을 보장한다고 주장하지 않는다.
- Settings의 실제 접근성 트리에 토글이 없음을 확인하고 모든 메모 페이지로 복귀했다.
- 재시작 전 보이던 4개 메모는 다시 열고, 숨겨져 있던 범용 터미널은 검증 뒤 숨김으로 돌렸다. 저장된 native 좌표를 사용해 보이던 메모 4개와 관리 창 위치·크기를 복원했다. 이후 CLI inspect로 모든 항목의 표시 상태와 보이는 메모의 물리 위치·크기가 재시작 전과 같음을 확인했다. 진행 메모 외 텍스트/이미지의 본문·제목·상태·색·연결 파일도 일치했다.
- 관리 창은 원래 native 위치 `{683,396}`, 크기 `944×480`으로 복원했다. 개발 앱과 관리 창은 실행·표시 상태로 유지했다. 진행 메모는 같은 ID의 update로 갱신했고 자동으로 숨기거나 삭제하지 않았다.
- 이전 Codex 세션 PID 65873은 승인된 앱 종료 때 끝났다. 다시 연 에이전트는 **새 세션 PID 23059**다. 새 세션은 이번 표시/숨김 동안 유지됐지만, 재시작 전 대화/PTY를 이어받았다고 주장하지 않는다. 모델에 작업이나 프롬프트를 보내지 않았다.

### 캡처와 실행 요약

- [실제 표시 중 상태](issue68-manager-toggle/native-shown.png), [실제 숨김 상태](issue68-manager-toggle/native-hidden.png). 두 캡처는 새 PID 22648의 관리 창 39218·scale 2다. 공통 도구가 제목 위에 있고 버튼 옆 상태가 구별됨을 PNG에서 확인했다.
- 원본 캡처 경로는 각각 `/var/folders/ds/zm2jvd217ss_q5jy2bjcbb9c0000gn/T/orca-computer-use/f5499d62-434d-49d4-871b-3de44debef89-screenshot.png`, `/var/folders/ds/zm2jvd217ss_q5jy2bjcbb9c0000gn/T/orca-computer-use/c7c8625e-6e39-4888-8164-bf16cc2d6b59-screenshot.png`이다.
- [native-results.json](issue68-manager-toggle/native-results.json)에 실행 PID·instance·창 ID·유형별 전환·토글 전후 세션 PID·복원 상태를 남겼다. 이 ID들은 이번 실행의 출처이며 향후 세션에서도 유효하다는 뜻이 아니다.
- 이번 후속 작업은 실행 검증·문서 갱신이며 앱 코드는 추가 변경하지 않았다. 따라서 위 Rust 129개·UI 87개·Clippy·Rustfmt·브라우저 통과 결과의 대상 코드와 동일하다. 재빌드와 마지막 `git diff --check`를 수행했다.

## 남은 검증과 상태

- Native 실패 강제 재현, 최소화된 창의 복원, Windows·Intel 실행, 두 워크트리 격리 스모크는 미실행이다. 범용 터미널은 Welcome 상태로 검사했으며 실제 셸 입력·한글 IME는 이번 범위 밖이다.
- 가려진 관리 창의 정기 조회는 검증 실패/제약으로 남긴다. 표시된 관리 창의 2초 주기 반영과 새로고침은 확인했다.
- 사용자 승인 없이 커밋·push·병합하지 않았다. 진행 메모에는 완료 결과·미검증 범위·커밋 승인 대기를 남겼다. 설치 앱과 다른 워크스페이스의 메모·프로세스는 변경하지 않았다.

Conf 5 (macOS ARM64 최신 개발 앱에서 네 유형의 실제 표시/숨김·상태 반영·복원 확인), Risk Low (관리 창의 국소 표시/숨김 도구, 기존 열기·숨기기 경로 재사용). 위 미검증/제약 범위는 Conf 5의 대상이 아니다.
