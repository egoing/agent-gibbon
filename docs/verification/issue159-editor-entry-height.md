# #159 편집 진입 높이 확보 — 2026-11-14

## 결과와 상태

작은 텍스트 메모의 편집 진입에 자동 높이 상한의 절반까지 한 번 확대하는 경로를 구현했다. 자동 상한 420 논리 px는 `sizing::AUTO_HEIGHT_MAX` 한 곳에서 측정 cap과 진입 높이 계산에 공유한다. 이미 210px 이상이면 프레임과 진행 중인 등장 애니메이션을 변경하지 않는다. 작업 영역이 목표보다 낮으면 영역 높이까지만 늘리고 기존 큰 창은 줄이지 않는다.

사용자의 `close` 호출로 최종 커밋·push·병합·이슈 정리가 승인됐다. 제품 구현은 `f43326e`, 최신 main `d1c3e8a` 통합은 `a8bea00`이다. [PR #162](https://github.com/egoing/agent-gibbon/pull/162) · [이슈 #159](https://github.com/egoing/agent-gibbon/issues/159). 초기 접근성 실패 후 재검증 결과는 다음 절이 우선하며, 아래 초기 기록은 당시 출처와 한계를 보존한다.

## 최종 main 통합과 close 검증

- 최신 main `d1c3e8a`의 내부 Feedback·저장 명칭·편집 상단 여백·overlay scrollbar를 모두 보존했다. 자동 높이 상한은 그대로 420 논리 px이며 진입 목표는 **내부 Feedback을 포함한 메모 전체 210px**이다. UI/SVG/PNG·native 검증 스크립트를 이에 맞췄다. 외부 Feedback의 위/아래 부착 설명은 초기 실행 기록에만 해당한다.
- 통합 코드 `a8bea00`: `npm run test` Rust **223개**·UI **203개**, `CARGO_INCREMENTAL=0 npm run check`, Rustfmt, `CARGO_INCREMENTAL=0 npm run dev:build`, `git diff --check` 통과. Rust 개수 감소는 최신 main의 외부 Feedback 제거 반영이다. 저장·취소·충돌·내보내기 계약의 최신 main 테스트도 함께 통과했다.
- `scripts/editor-entry-height-browser.mjs`: Chromium 158.0.8127.18에서 144×32, 300×48/209/210/280 다섯 조합 통과. 최종 내부 Feedback에서 textarea 높이 **155px**, 큰 창은 **225px**. 너비·포커스·초안·수동 축소·숨김 복원·재진입·저장 확인, JS 오류 0개. **native 프레임 적용은 모의 처리**다. `/tmp/gibbon159-close-browser.log` 및 `/tmp/gibbon159-close-browser/`에 결과를 보존했다.
- Clippy와 debug 링크 도중 디스크 부족으로 각각 실패했다. 이번 워크트리의 `target/debug/incremental`과 이번 패키지의 오래된 `agent_gibbon-*.rcgu.o`만 정리하고 증분을 꺼서 재실행해 통과했다. 소스·다른 워크트리·사용자 앱은 제거하지 않았다. 로그: `/tmp/gibbon159-close-test.log`, `-clippy.log`, `-build.log`.

### 실제 실행 앱의 핵심 높이·저장 검증

사용자가 실행 유지하도록 요청한 앱을 재시작하지 않았다. 실행 중인 **통합 전 빌드**에서 변경된 핵심 함수를 검증했으며, 최종 통합 앱 전체의 native 검증이라고 주장하지 않는다. `git diff f43326e -- src-tauri/src/sizing.rs ui/app.js`로 `AUTO_HEIGHT_MAX`·`editor_frame`·`enter_note_editor` 및 진입 호출/초기 가시화 대기 내용이 유지됨을 대조했다. 최종 main에서 추가된 내부 Feedback·저장 문구·내보내기 숨김은 별도 브라우저/자동 회귀 근거로 검증했다.

- root `/Users/egoinglee/.codex/worktrees/b3f2/agent-gibbon`, instance `dev-cca5a7a82225235b7519e45c`, 실제 실행 PID **49254**, 절대 debug binary와 `--serve`를 `ps`로 대조했다. 재선택한 고유 메모 `editor-height-4d270df1`의 `inspect` ok/instance/id/본문과 native 제목 `Agent Gibbon [DEV codex/editor-entry-height · cca5a7a8] — editor-height-4d270df1`를 PID-bound AX에서 대조했다. 배율 2.0.
- 접근성 조회가 복구되어 `GIBBON_SMOKE_OUTPUT=/tmp/gibbon159-close-native-final python3 scripts/editor-entry-height-smoke.py`가 **15개 상태 기록을 포함해 종료 코드 0으로 통과**했다.

| 시작 논리 크기 | 편집 진입 뒤 논리 크기 | 실제 textarea 높이 |
|---|---|---|
| 144×32 | 144×210 | 172px |
| 300×48 | 300×210 | 172px |
| 300×209 | 300×210 | 172px |
| 300×210 | 300×210 | 172px |
| 300×280 | 300×280 | 242px |

너비·x·경계 밖이 아닌 y·핀·포커스 보존과 입력 가능 영역을 확인했다. 화면 하단에서는 같은 x로 필요한 만큼만 위로 이동했다. 이후 수동 96px 축소를 폴링이 되돌리지 않았고, 숨김→CLI open 뒤 초안이 유지됐다. CLI 재열기는 기존 자연 콘텐츠 자동 배치로 57px가 됐으며 편집 진입을 재호출해 210px로 강제하지 않았다. 원본 파일 외부 변경 시 충돌과 초안·외부 파일 보존, 접기→펼침→다시 편집 시 210px 확보, 다시 읽은 원본의 정상 저장까지 통과했다.

초기 native 스크립트 재시도 두 건은 실패했다. 첫째는 CLI 재열기가 수동 96px를 유지할 것이라는 테스트의 잘못된 가정, 둘째는 저장 버튼의 접근성 이름에서 ` · Command S`를 빠뜨린 테스트 도구 문제였다. 제품 코드는 바꾸지 않고 스크립트를 보정한 뒤 최종 전체 시나리오를 통과했다. 실패 로그 `/tmp/gibbon159-close-native.log`, `/tmp/gibbon159-close-native2.log`와 최종 `/tmp/gibbon159-close-native-final.log`, 해당 폴더 `evidence.json`을 구분해 보존했다.

[실제 native 메모 캡처](assets/issue159-native-editor-height.png)는 PID·메모 제목을 대조한 CG 창을 `screencapture -l`로 캡처한 **통합 전 유지 앱**의 화면이다. 최종 내부 Feedback 화면이나 전체 플랫폼 실행 증거가 아니다. 사용자 확인창 `editor-height-user-check`의 본문·표시·위치·크기는 전후 inspect가 같음을 확인했고, 사용자 앱은 종료하지 않았다. 검증용 메모만 숨겼다.

최종 통합 UI 전체의 native 실행, 초기 영역 생성의 native 자동 편집, Windows·Intel·혼합 DPI·전체 플랫폼 CI·설치·배포는 미실행이다. **Conf 4 / Risk Low**: 최신 통합의 자동·브라우저 검증과 같은 핵심 변경 함수의 실제 환경 실행을 확보했으며, 실행 빌드별 한계를 분리한다.

## 초기 준비 시점의 구현·검증 기록

## 구현 범위

- 프론트엔드의 실제 `beginEditing`에서 revision과 함께 한 번 요청한다. 이미지와 이미 편집 중인 상태는 제외한다. 초기 영역 메모의 `edit_on_open`은 첫 가시·펼침 snapshot을 기다린 뒤 진입하며, 숨김 snapshot에서 진입 의도를 소모하지 않는다. 이후 폴링·입력·숨김 복원·접기/펼침은 재요청하지 않는다.
- native 명령은 main/note- 창, 현재 revision의 비이미지 메모, 표시·펼침 상태만 허용한다. 호출한 자기 창만 변경하며 외부 페이지 IPC 거부를 유지한다.
- 실제로 확대할 때만 기존 등장 애니메이션을 취소하고, 기존 동기식 native 프레임 경로를 재사용한다. 너비/x를 보존하고 가능한 y를 유지하되 화면 하단 초과분만 위로 보정한다. 영구 최소 크기·전체 reflow·Feedback resize는 추가하지 않는다.
- 저장·취소·연결 원본 충돌 보호는 기존 경로 그대로다. 크기 변경 실패는 기존 오류 안내를 사용하고 초안·편집 상태를 지우지 않는다.
- [디자인 시스템](../design-system.md#편집-진입-높이-확보--159-2026-11-14) · [시안 SVG](../design/editor-entry-height.svg) · [PNG](../design/editor-entry-height.png). SVG→PNG 렌더링과 시각 확인 완료. 시안은 실제 앱 캡처가 아니다.

## 실행한 검사

| 범위 | 명령/근거 | 결과 |
|---|---|---|
| Rust 순수 정책·기존 회귀 | `cargo test --manifest-path src-tauri/Cargo.toml` | 224개 통과 |
| UI 진입·저장 계약 | `npm run test:ui` | 201개 통과 |
| 정적 검사 | `cargo fmt --check --manifest-path src-tauri/Cargo.toml`, `npm run check`, `git diff --check` | 통과 |
| 개발 빌드 | `npm run dev:build` | macOS ARM64 debug 빌드 통과 |
| 실제 UI 자산 브라우저 | `PLAYWRIGHT_MODULE=/tmp/gibbon-playwright/node_modules/playwright/index.mjs node scripts/editor-entry-height-browser.mjs` | Chromium 158.0.8127.18, 5조합 통과. **native 크기 적용은 모의 처리** |
| #16 Feedback 회귀 | 같은 모듈 설정으로 `node scripts/editor-save-smoke.mjs` | 144/300px 버튼 접근·단축키·포커스 보존 통과 |
| native 검증 준비 | `swiftc scripts/design-system-ax.swift`, `python3 -m py_compile scripts/editor-entry-height-smoke.py` | helper 컴파일·Python 구문 검사 통과, 실제 전체 시나리오는 미통과 |

Rust 기하 테스트는 scale 1/1.25/1.5/2의 임계 미만·동일·초과, 하단, 음수 모니터 좌표, 작업 영역이 목표보다 낮은 경우를 확인했다. 이는 OS API 적용·혼합 DPI 실환경 검증을 대체하지 않는다.

UI 테스트는 진입당 한 번 호출, 초기 자동 편집의 숨김/접힘 대기, 이미지 제외, 폴링·입력·접기·복원 중 초안 보존, 확대 실패, 빠른 취소를 확인했다. 브라우저에서는 144×32, 300×48/209/210/280에서 모의 높이를 적용한 뒤 textarea 높이 172px(큰 창은 242px), 포커스, 수동 96px 축소 유지, 숨김/복원, 취소 후 재진입, 저장을 확인했다. 화면 캡처는 `/tmp/gibbon159-browser/*-editor.png`, 세부 결과는 `/tmp/gibbon159-browser-final.log`에 있다. 브라우저 오류 0개. Feedback은 별도 브라우저 회귀이며 네이티브 위/아래 부착 증거는 아니다.

## 실제 개발 앱 출처와 실패 기록

- 작업 루트: `/Users/egoinglee/.codex/worktrees/b3f2/agent-gibbon`
- 브랜치: `codex/editor-entry-height`, 기반 main `53ab3a710fb54f4702de2499fdc22b9dd8506087`
- `npm run dev:info`: instance `dev-cca5a7a82225235b7519e45c`, binary `/Users/egoinglee/.codex/worktrees/b3f2/agent-gibbon/src-tauri/target/debug/agent-gibbon`
- 도구: Node v24.21.0 / Rust·Cargo 1.91.1 / Tauri CLI 2.12.1 / macOS ARM64, 관측 화면 배율 2.0. 검증 중 화면 구성이 3→2→3으로 바뀌었다.
- `scripts/agent-gibbon-dev`만 사용했다. 첫 PID20938은 `ps` 절대 binary+`--serve`로 확인했다. 로그 확인용 PID22032/22696/23373은 `lsof -p PID -a -d txt`로 실제 절대 실행 파일을 대조한 뒤 재시작했다. 최종 PID24718도 동일 경로와 instance를 다시 확인했다.
- 첫 메모 `editor-height-736bdecd`, 최종 메모 `editor-height-c11826b9`: `inspect`의 ok/instance/id/고유 Markdown을 대조했다. 별도 진단 메모 `editor-height-debug`도 본 작업에서만 만들었다. 검사 원문·원본 파일은 임시 테스트 파일이다.
- 초기 `open`은 `ok:true`지만 `visible:false`. 일시적 오류 로그에서 `available_monitors: Ok([])`와 기존 reflow의 `배치할 화면이 너무 작습니다` 반환을 확인했다. 로컬 `caffeinate -u -t 5` 후 같은 CLI open을 재시도하자 모니터와 work_area가 복구되고 `visible:true`·600×98 물리 px(300×49 논리)가 됐다. 이 절전 상태 실패를 일반 open 통과로 숨기지 않는다. 표시 코드 변경은 하지 않았고 **임시 진단 로그는 모두 제거했다.**
- 최종 PID24718의 CG 창 번호69995와 bounds(300×49 논리), inspect의 물리 위치·크기가 일치했다. CG 제목은 제공되지 않았고 AX에서 예상 네이티브 제목 `Agent Gibbon [DEV codex/editor-entry-height · cca5a7a8] — editor-height-c11826b9`를 식별하지 못했다. `AXWindows`는 실제 창 대신 앱 자신을 반환했다. `AXIsProcessTrusted()`는 true였다.
- `screencapture -x -l 69995`는 `could not create image from window`로 실패했다. 실제 앱 캡처는 확보하지 못했다.
- `computer-use` 스킬의 Orca 경로를 적용했다. runtime 미실행으로 `orca open --json` 후 capabilities의 background 지원을 확인하고 `orca computer get-app-state --app pid:24718 --window-id 69995 --json`을 실행했다. 결과는 **permission_denied**: visible windows는 있으나 AX가 1500ms 동안 막혀 접근성 창을 얻지 못했다. `orca computer permissions --json`은 accessibility/screenshots granted를 반환했으나 실제 접근 거부와 모순돼 통과로 간주하지 않았다. 권한 설정 토글·강제 포커스 전환은 하지 않았다.
- 최종 inspect 예: 물리 위치 x672/y540, 크기600×98, instance 일치·visible true·animating false. **이는 편집 버튼 실행 전 표시/출처의 부분 근거일 뿐, 편집 확대 성공 증거가 아니다.** native 시나리오 `scripts/editor-entry-height-smoke.py`는 AX 대상 창을 찾지 못해 실패했다.
- `/tmp/gibbon159-native.log`, `/tmp/gibbon159-native/evidence.json`, `/tmp/gibbon159-native/windows.json`, `/tmp/gibbon159-orca-permissions.json`, `/tmp/gibbon159-devruntime3.log`에 실패·부분 근거를 보존했다. 기존 타인 캡처를 사용하지 않았다.

## 초기 정리와 당시 남은 검증

본 작업 메모 세 개를 숨기고 `list`로 모두 visible false를 확인했다. PID24718은 `lsof`로 자기 binary를 재확인한 뒤 종료했다. 다른 워크트리 앱·사용자 watch·설치 앱을 종료하지 않았다. 팀장에게 GUI 슬롯 해제를 보고했다. 임시 테스트 원본은 정리됐고 메모 자체는 숨김 상태로 남아 있다.

실제 OS에서 작은/경계/큰 창 편집 전후 높이·너비·x/y·포커스·핀·초안·Feedback 위/아래 부착·수동 축소·숨김 복원·충돌 후 재시도·저장/취소 확인이 남는다. 초기 영역 생성의 native 자동 편집도 미검증이다. Windows·Intel·혼합 DPI·물리 화면 해제 및 전체 플랫폼 CI·설치·릴리스는 미실행이다.

**Conf 3**: Rust/UI 자동 테스트와 실제 UI 자산의 모의 크기 브라우저 검증을 통과했으나 native 적용을 검증하지 못했다. **Risk Low**: 편집 진입 시 자기 텍스트 창의 일회성 프레임 변경이며 파일 저장·인증·외부 전송은 변경하지 않는다. 접근성 환경 복구 후 위 native 회귀를 통과하기 전 실제 동작 완료로 보고하지 않는다.
