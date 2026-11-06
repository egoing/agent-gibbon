# #68 통합 검증과 관리 창 포커스 보정 — 2026-11-06

사용자가 커밋·main 병합·push·개발 앱 재시작을 승인했다. 이후 보고한 ‘화면에 띄우기 시 관리 창이 사라짐’도 같은 변경에서 처리한다. 모니터 선택 정책은 기존 위치 복원을 유지하며 별도의 정책 변경을 추가하지 않는다.

## 통합

- 작업 루트: `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/memo-show-common`
- 브랜치: `feat/memo-show-common`, instance `dev-2420851a18a85d0606b0c157`.
- 원본 `/Users/egoinglee/dev/project/agent-gibbon`은 수정·복사하지 않았고 원본 main 작업 폴더의 미커밋 변경도 그대로 두었다. 원격 PR 병합으로 main에 반영한다.
- 구현 커밋 `5a654e2`에 공통 도구·토글·중복 상태 문구 제거·검증 자산을 담았다. 이후 `origin/main`의 설정 사이드바 변경 `24f1ea3`을 통합했다. 네 파일의 충돌에서 설정 탐색·초안 보존과 공통 토글을 모두 유지했으며 통합 커밋은 `91a93b5`다.
- 포커스 보정과 이 기록은 위 통합본에 대한 후속 변경이다. PR과 원격 반영 결과는 작업 완료 응답에서 확인한다. 버전·릴리스 태그·설치 앱은 변경하지 않는다.

## 관리 창이 가려지는 원인과 보정

재시작 후 첫 이미지 표시에 재현했다. 관리 창은 native 창 목록에 계속 있었지만 전면 앱이 개발 앱 PID 23012에서 이전 앱 PID 16242로 바뀌었다. 실제 닫기·숨김이 아니라 다른 앱이 앞으로 와서 가려진 현상이다.

복원된 WebView가 최초 표시 전에 앱 시작 당시의 외부 앱 포커스 기록을 보관하고 있었다. 관리 창을 사용 중인 명시적 열기에서도 그 오래된 기록이 남아 첫 표시 때 이전 앱을 활성화했다. 관리 창의 표시 명령에서 **선택한 메모의 이전 복원 기록만 제거**하고 기존 열기 경로가 현재 작업의 포커스 보존 정보를 새로 기록하도록 보정했다. 일반 CLI와 다른 메모의 대기 기록은 바꾸지 않는다. 지연 중 사용자가 다른 앱으로 옮긴 경우를 존중하는 기존 restore 검사도 유지한다.

## 자동 검사

- 최신 통합·포커스 보정 소스의 `npm test`: **Rust 129개·UI 88개 통과**.
- `npm run check`: 전체 target Clippy, 경고 오류 처리 통과.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`, `git diff --check`: 통과.
- `npm run version:check`: 0.2.7 일치.
- `npm run dev:build`: 성공. 실제 바이너리는 위 작업 루트의 `src-tauri/target/debug/agent-gibbon`이다.
- 통합 UI의 `scripts/manager-show-smoke.mjs`: Chromium 156.0.8078.4, 944×480·600×360의 12개 조합 통과, JS 오류 0개. 포커스 보정은 이후 Rust만 바뀌었으며 동일한 UI 자산이다.
- 설정 사이드바 회귀 검사를 함께 유지해 UI 수가 87개에서 88개가 됐다. 새 UI·IPC 동작만 있고 테스트용 원본 앱 변경이나 별도 서버는 없다.

## 최신 개발 앱과 native 실행 출처

- 최초 재실행 시 이전 PID 22648과 세션 PID 23059는 이미 종료돼 있었다. 새 통합 앱 PID 23012에서 사용자 보고를 재현했다.
- 보정 후 PID 23012만 `NSRunningApplication.terminate()`로 정상 종료하고 같은 개발 CLI로 다시 실행했다. 최종 검사 앱 PID는 **35913**, 관리 창 native ID는 **39537**이다. 명령·실행 파일은 위 루트의 debug 바이너리와 일치한다.
- `inspect --id issue68-progress-2420851a`에서 `ok:true`, 예상 instance와 ID, 고유 본문 `ISSUE68-2420851A-20261106`을 확인했다. 관리 창에서도 같은 내용을 대조했다. 재시작은 `scripts/agent-gibbon-dev open`으로 수행했으며 설치 앱이나 다른 워크스페이스로 대체하지 않았다.
- GUI는 PID·정확한 native 제목을 지정하는 `scripts/design-system-ax.swift`와 `orca computer ... --app pid:35913 --window-id 39537`로 확인했다. 효과는 명령 응답만으로 판단하지 않고 실제 전면 앱 PID·native 창 목록·CLI 가시성·버튼 문구를 재조회했다.

### 실제 확인

- 재시작 후 첫 이미지 표시에서 전면 앱이 PID 35913으로 유지됐고 관리 창도 표시 상태였다. 수정 전 23012→16242 전환과 구분해 기록한다.
- 현재 남아 있는 텍스트·이미지·에이전트의 표시→숨김→표시에서 관리 창·전면 앱을 유지하고 상태에 맞는 버튼 하나만 표시했다. 에이전트의 새 세션 PID는 토글 전후 같았다. `표시 중`·`숨김` 별도 텍스트는 없었다.
- 초기 검사에서 다른 메모도 모두 숨김일 것으로 가정한 준비 단계가 중단돼 실제 상태를 다시 읽고 숨김을 준비한 뒤 검사했다. 이 중단을 앱 동작 성공으로 보고하지 않으며 최종 검사만 위 통과 결과다.
- 범용 터미널 항목은 이 실행 전에 목록에서 제거돼 재생성하지 않았다. 범용 터미널은 앞선 native 토글 기록과 이번 통합 Chromium 검사 범위이며, 최종 포커스 보정 후 native 범용 터미널 재검사는 수행하지 않았다.
- 최신 main의 Settings 사이드바에서 스킬 진입점과 Privacy & Diagnostics 전환을 확인하고 메모로 복귀했다. 진단 동의 값은 변경하지 않았다.
- 재시작 전 snapshot의 표시/숨김 상태와 보이던 메모·관리 창의 native 위치·크기를 복원하고 CLI로 대조했다. 사용자가 숨긴 main 메모는 계속 숨김이며 진행 메모는 같은 ID로 갱신한다. 이전 세션은 재시작 때 종료됐고 새 세션을 이전 대화의 연속으로 보고하지 않는다.
- [포커스 회귀 요약](issue68-delivery/focus-regression.json), [최신 관리 창](issue68-delivery/manager-focus.png). 캡처는 PID 35913·창 39537·scale 2이고 원본은 `/var/folders/ds/zm2jvd217ss_q5jy2bjcbb9c0000gn/T/orca-computer-use/eba269c4-f48c-4468-b42f-5f4050ee2172-screenshot.png`다. PNG에서 제목 위 버튼 하나, 중복 상태 문구 없음을 확인했다.

## 한계

Windows·Intel의 native 실행, 최소화 복원, 강제 오류 경로, 두 워크트리 격리 스모크는 미실행이다. 가려진 관리 창의 정기 상태 갱신 제약은 [이전 기록](issue68-manager-toggle.md)에 구분했다. 다른 모니터에 기존 위치로 복원되는 정책은 유지한다.

Conf 5 (최신 macOS ARM64 앱의 재현·포커스 회귀·공통 도구 확인), Risk Low (관리 창에서 명시적으로 표시할 때 선택 메모의 오래된 복원 기록만 제거). 미검증 범위에는 Conf 5를 적용하지 않는다.
