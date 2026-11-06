# 관리 창 공통 화면 표시 도구 — #68 (2026-11-06)

[이슈 #68](https://github.com/egoing/agent-gibbon/issues/68)의 구현과 이번 워크스페이스에서 수행한 검증을 기록한다. UI 기준은 [디자인 시스템](../design-system.md#메모-공통-화면-표시-도구--68-2026-11-06)을 따른다.

## 변경

- 텍스트·캡처 이미지·범용 터미널·에이전트의 상세 맨 위에 공통 `화면에 띄우기` 버튼 하나를 둔다. 기존 터미널 본문 안의 표시 버튼은 제거한다. 텍스트는 별도 제목을 만들지 않고 원본 HTML을 보존한다.
- `manager_show`는 관리 창만 호출할 수 있고, 기존 메모·터미널인지 확인한 뒤 기존 명시적 열기 경로를 사용한다. 일반 조회·목록 선택은 창을 열지 않는다. Settings·선택 전·빈 목록에는 버튼이 없다.
- 실행 중 중복 클릭을 막고 실패를 상세 도구 영역의 alert로 표시한다. 오류 원문은 노출하지 않으며 재시도할 수 있다. 이전 선택의 늦은 실패는 새 상세에 표시하지 않는다.
- 텍스트·이미지 SVG/PNG를 갱신하고 터미널 SVG/PNG를 추가했다. Settings 자산은 변경하지 않았다. 새 서버·저장 형식·진단 동의 정책은 추가하지 않았다.

## 실행 출처

- 작업 루트: `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/memo-show-common`
- 브랜치: `feat/memo-show-common`
- `npm run dev:info`의 `root`: 위 작업 루트와 일치.
- 인스턴스 ID: `dev-2420851a18a85d0606b0c157`
- 바이너리: `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/memo-show-common/src-tauri/target/debug/agent-gibbon`
- 환경: macOS ARM64, Node v24.21.0, npm 11.19.0, rustc/cargo 1.91.1, tauri-cli 2.12.1, rsvg-convert 2.62.3.
- 시작 시 `git status --short`는 비어 있었다. 원본 `/Users/egoinglee/dev/project/agent-gibbon`은 수정하거나 복사하지 않았다. 커밋·push·병합은 수행하지 않았다.

## 통과한 검사

| 명령/범위 | 결과 |
|---|---|
| `npm ci` | 성공, 취약점 0개 |
| `npm test` | Rust 129개, UI 84개 통과 |
| `npm run check` | Clippy 전체 target, 경고 오류 처리 통과 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | 통과 |
| `node --check ui/manager.js` 및 `node --check scripts/manager-show-smoke.mjs` | 통과 |
| `npm run version:check` | 0.2.7 일치, 버전 변경 없음 |
| `npm run dev:build` | 이 워크스페이스의 debug 바이너리 빌드 성공 |
| `git diff --check` | 통과 |

UI 회귀 검사는 공통 버튼의 위치·선택한 ID 전달·선택만 할 때 호출 없음·빈 목록·Settings 복귀·오류 유지와 재시도·중복 요청 억제·선택 전환 후 늦은 실패 무시를 확인한다. 기존 Rust 테스트의 통과는 새 명령의 실제 네이티브 실행 검증을 뜻하지 않는다.

## 브라우저 및 시안

실행 명령:

```sh
PLAYWRIGHT_MODULE=/tmp/agent-gibbon-is2-57/browser-tools/node_modules/playwright/index.mjs node scripts/manager-show-smoke.mjs
```

- Chromium 156.0.8078.4에서 실제 `manager.html`, `manager.css`, `manager.js`를 사용했다. 네이티브 호출과 이전 약관 수락 상태만 모의 응답으로 제공했다. 실제 앱의 동의 기록은 변경하지 않았다.
- 944×480, 600×360에서 제목 있는 텍스트·제목 없는 텍스트·빈 텍스트·이미지·범용 터미널·에이전트 6개 사례, 총 12개 조합을 검사했다.
- 각 사례의 버튼 하나·제목/본문보다 앞선 위치·선택만 할 때 실행 없음·키보드 Enter 실행·선택한 ID 전달·가로 넘침 없음을 확인했다. 텍스트 제목 중복 없음, Settings·빈 목록에서 버튼 없음, 오류 표시·주기적 조회 뒤 오류 유지·재시도를 확인했다. JavaScript 오류 0개.
- [텍스트 944×480](issue68-manager-show/browser-text-944.png), [터미널 600×360](issue68-manager-show/browser-terminal-600.png), [오류 600×360](issue68-manager-show/browser-error-600.png)는 모의 데이터 기반 브라우저 캡처다. 실제 플로팅 창의 복원·위치·포커스 검증이 아니다. 이미지 응답은 배치 검사용 1px PNG다.
- 텍스트·이미지·터미널 SVG를 각각 `rsvg-convert 입력.svg -o 출력.png`로 렌더링하고 PNG를 열어 버튼 위치와 잘림 없음을 확인했다. 기존 Settings SVG/PNG도 읽어 공통 도구가 없는 범위를 확인했다. [텍스트 PNG](../design/manager-notes-wireframe.png), [이미지 PNG](../design/manager-image-wireframe.png), [터미널 PNG](../design/manager-terminal-wireframe.png).

## 실제 개발 CLI와 네이티브 검증

### 최초 동의 차단 및 해소

1. 이 워크스페이스의 `./scripts/agent-gibbon-dev list`는 처음에 앱 미실행 오류를 반환했다. `open --id issue68-progress-2420851a --file -`로 진행 체크리스트 표시를 시도하자 앱이 시작됐지만 `ok: false`, `onboarding_required: true`, 예상 인스턴스를 반환했다. 같은 CLI의 `inspect`도 차단됐다.
2. 사용자에게 새 개발 앱에서 약관을 직접 확인·수락해 달라고 요청했다. 이후 실제 `diagnostics`의 `terms_accepted: true`를 확인하고 다시 진행했다. 에이전트가 동의 파일을 복사·작성하거나 선택적 진단 설정을 조작하지 않았다.
3. 같은 개발 CLI로 진행 메모를 생성했다. `inspect`에서 `ok: true`, instance `dev-2420851a18a85d0606b0c157`, ID `issue68-progress-2420851a`, 본문 표식 `ISSUE68-2420851A-20261106`을 대조했다. 이후 진행 상황은 같은 메모의 `update`로 갱신한다. 설치 앱으로 대체하지 않았다. 음성 안내 스킬도 적용했다.

### GUI 출처와 조작

- `ps -p 51335 -o pid=,comm=`과 `lsof -a -p 51335 -d txt -Fn`으로 실제 실행 파일이 위 작업 루트의 debug 바이너리임을 확인했다.
- 관리 창은 PID 51335의 native ID 39025, 제목 `Agent Gibbon [DEV feat/memo-show-common · 2420851a] — 메모 관리`로 특정했다. `orca computer get-app-state --app pid:51335 --window-id 39025 --json`의 상세 본문에서도 위 고유 표식을 대조했다. 관리 창 위치는 사용자 조작으로 바뀌었으며 초기 위치로 강제 복원하지 않았다.
- `swift scripts/auto-layout-native.swift 51335`의 실제 CGWindow 목록에서 아래 메모 ID가 포함된 native 창 제목과 ID를 확인했다. CLI 가시성만으로 화면 출처를 판단하지 않았다.
- `orca computer click`의 AXPress 성공 표시는 단독 성공 근거로 사용하지 않고, 반환된 상세 내용과 개발 CLI 상태를 다시 확인했다. 관리 창 숨김/재표시 뒤 provider의 `background_unavailable`(창 일치 식별 실패)이 한 번 발생했다. 창 ID·PID를 다시 조회하고, 저장소의 PID·정확한 창 제목 기반 `scripts/design-system-ax.swift`로 배경 AXPress를 수행한 뒤 상태를 대조했다. 포커스를 강제로 가져오는 재시도는 하지 않았다.
- 이미지와 범용 터미널은 이 PID의 트레이에서 각각 영역 캡처·영역 터미널을 선택해 만들었다. 트레이 AXPress만으로 메뉴가 열리지 않는 경우 기존 native 입력 helper로 실제 트레이 좌표를 눌렀다. 영역 선택·트레이 클릭은 `scripts/macos-input.swift`로 수행했고 helper가 매 동작 후 포인터를 원위치로 복원했다. 영역 생성 자체는 이번 공통 버튼의 회귀 검사와 구분한다.

| 유형 | 검증 메모 ID | 실제 native 창 ID |
|---|---|---|
| 텍스트/진행 메모 | `issue68-progress-2420851a` | 39053 |
| 이미지 | `image-1793936326662915000` | 39106 |
| 에이전트 | `agent-1793936218271001000` | 39058 |
| 범용 터미널 | `terminal-1793936350225402000` | 39112 |

### 확인한 동작

- 네 유형 모두 관리 상세에 공통 버튼 하나가 제목/본문보다 먼저 표시됐다. 텍스트의 원본 Markdown 제목은 하나만 보이며 별도 제목을 추가하지 않았다. 이미지의 실제 미리보기도 표시됐다. 터미널의 삭제 버튼은 본문 동작으로 남았다.
- 같은 개발 CLI의 `close`로 검증 메모를 숨기고, 목록을 선택한 뒤 `inspect`로 여전히 숨김임을 확인했다. 텍스트는 선택된 상태에서 숨기고 버튼을 실행했으며 나머지 세 유형은 숨긴 후 다른 목록에서 새로 선택했다. `화면에 띄우기` 클릭 뒤 올바른 ID가 `visible: true`가 됐다.
- 텍스트와 두 터미널의 숨김 복원에서 위치·크기가 유지됐다. 이미지는 기존 명시적 열기 경로의 자동 배치가 적용돼 물리 위치가 `{-2300,1400}`에서 `{-400,500}`으로 바뀌었고 크기는 `400×240`으로 유지됐다. 목록 선택 때에는 위치가 바뀌지 않았다. 고정 위치 복원을 새로 보장했다고 주장하지 않는다.
- 네 유형 모두 **이미 보이는 상태에서 다시 버튼을 누른 전후** 위치·크기와 전면 앱 PID가 일치했다. 텍스트·이미지의 pin 값도 유지됐다. 터미널은 `inspect`에 pin 필드가 없어 같은 필드의 검증으로 주장하지 않는다. 검증 도중 helper가 그 없는 필드를 읽어 중단된 부분은 실제 응답 스키마에 맞춰 다시 실행해 통과했다.
- 에이전트는 자동 시작된 Codex 프로세스 PID 65873을 숨김 복원·재표시 전후 동일하게 유지했다. 모델에 작업이나 프롬프트를 전송하지 않았다. 범용 터미널은 Welcome 상태(`running: false`)로 검사했으며 셸 작업은 시작하지 않았다.
- Settings로 전환한 실제 접근성 트리에서 Settings 제목이 있고 `화면에 띄우기`가 없음을 확인했다. 모든 메모 페이지로 복귀할 수 있었다.
- [네이티브 텍스트 화면](issue68-manager-show/native-text.png), [네이티브 터미널 상세](issue68-manager-show/native-terminal.png)는 창 ID 39025·scale 2의 캡처다. 각각 원본은 `/var/folders/ds/zm2jvd217ss_q5jy2bjcbb9c0000gn/T/orca-computer-use/c698b321-16e7-4636-9e87-cb2721c4b7df-screenshot.png`, `/var/folders/ds/zm2jvd217ss_q5jy2bjcbb9c0000gn/T/orca-computer-use/8fbb9410-1845-415b-ab65-a3156b69d985-screenshot.png`이다. 네이티브 화면에서 버튼 위치·제목 중복 없음을 시각 확인했다.
- 반복 표시의 ID·위치·크기·전면 앱 보존 요약은 [native-summary.json](issue68-manager-show/native-summary.json)에 남겼다. 해당 실행의 테스트 PID는 역사적 근거이며 이후 세션까지 살아 있음을 보장하지 않는다.

## 미검증과 종료 상태

- 실제 native IPC 실패를 강제로 만들어 alert를 표시하는 검사는 미실행이다. 오류·재시도·늦은 실패 무시는 단위 및 Chromium 검사에서 확인했다.
- 창을 종료한 후 새 WebView로 재생성하는 경로, 범용 터미널의 실제 셸 실행·인증·한글 입력은 이번 검증에 포함하지 않았다.
- Windows·Intel 실행과 두 워크트리 격리 스모크는 미실행이다. 통신·개발 실행기 자체는 이번 변경 범위가 아니다.
- 커밋 승인은 아직 받지 않았다. 새 개발 앱과 검증 메모는 검토할 수 있도록 남겼고, 설치 앱·다른 워크스페이스의 프로세스나 메모는 변경하지 않았다. 진행 메모의 미검증 항목과 승인 대기 상태를 유지한다.

Conf 5 (macOS ARM64 개발 앱에서 지원 네 유형의 실제 CLI·버튼·복원·반복 표시 확인), Risk Low (국소적인 관리 도구 추가, 기존 열기 경로 재사용). 위 미검증 범위는 Conf 5의 대상이 아니다.
