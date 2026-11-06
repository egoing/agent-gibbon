# 공통 토글 중복 상태 문구 제거 — 2026-11-06

[이슈 #68](https://github.com/egoing/agent-gibbon/issues/68)의 후속 사용자 피드백에 따라 버튼 옆 `표시 중`·`숨김` 문구를 제거했다. 실제 상태 조회·표시/숨김 전환·버튼의 행동 문구·실패 안내는 유지한다. [디자인 시스템](../design-system.md#메모-공통-화면-표시-도구--68-2026-11-06)을 같은 기준으로 갱신했다.

## 다른 모니터에서 복원되는 이유

현재 공통 버튼은 기존 열기 경로를 호출한다. 텍스트·이미지의 자동 배치는 `layout.rs`에서 열린 메모 자신의 `current_monitor`를 먼저 사용하며 관리 창/마우스의 모니터를 전달하지 않는다. 자동 배치 제외 창은 기존 위치를 유지한다. 터미널의 기존 창은 이동하지 않고, 새 창이면 저장된 `agent.position()`에서 생성한다. 따라서 이전 메모가 다른 모니터에 있었으면 그곳에 복원될 수 있다. 자동 배치 공간이 부족하면 다른 모니터로 배치할 수도 있다.

사용자에게 선호 기준을 질문한 뒤 현재 변경의 커밋·병합·push·앱 재시작을 승인받았다. 모니터 정책 변경은 이번 승인된 변경에 포함하지 않으며 기존 위치 복원을 유지한다.

## 변경 및 검증

- 작업 루트: `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/memo-show-common`
- 인스턴스: `dev-2420851a18a85d0606b0c157`, 개발 바이너리는 위 루트의 `src-tauri/target/debug/agent-gibbon`이다.
- 상세의 상태 span과 전용 CSS를 제거했다. 기존 테스트와 브라우저 스모크를 같은 기준으로 갱신했다. 현재 상태에 따라 버튼 하나만 표시하며 오류가 발생하면 별도 alert를 유지한다.
- `npm run test:ui`: UI 87개 통과. `node --check ui/manager.js`, `node --check scripts/manager-show-smoke.mjs`, `git diff --check`: 통과.
- `npm run dev:build`: 성공. 이번 변경에서 Rust 소스는 바꾸지 않았다. Rust 129개·Clippy 통과는 [앞선 검증](issue68-manager-toggle.md)의 결과이며 이번 문구 제거 뒤 다시 실행한 검사로 보고하지 않는다.
- Chromium 156.0.8078.4에서 실제 UI 자산과 모의 IPC로 944×480·600×360, 6개 메모 사례의 12개 조합을 확인했다. 표시→숨김→표시, 상태 span 부재, 버튼 포커스 유지·외부 상태 갱신·오류 재시도·Settings 제외를 통과했다. JavaScript 오류 0개.
- 텍스트·이미지·터미널 SVG의 중복 상태 문구를 지우고 rsvg-convert 2.62.3으로 PNG 3개를 렌더링했다. PNG를 직접 열어 버튼 한 개와 여백·잘림 없음을 확인했다. Settings 자산은 변경하지 않았다.
- [텍스트 시안](../design/manager-notes-wireframe.png), [이미지 시안](../design/manager-image-wireframe.png), [터미널 시안](../design/manager-terminal-wireframe.png).

## 실행 중 앱과 한계

같은 개발 CLI의 진행 메모 inspect에서 `ok: true`, 예상 instance, 고유 본문 표식 `ISSUE68-2420851A-20261106`을 확인했다. 사용자에 의해 숨겨진 상태는 update 시에도 유지한다. 실행 중 개발 앱은 앞선 PID 22648이며 현재 Codex 세션이 있으므로 이번 문구 제거를 반영하는 재시작은 수행하지 않았다. 새 빌드 준비와 현재 앱 반영을 구분한다.

최신 문구 제거의 native GUI·Windows·Intel 실행은 미검증이다. 이전 토글의 native 검증은 중복 문구가 있던 버전에 대한 기록이다. 커밋·push·병합은 하지 않았다.

Conf 4 (실제 UI 자산의 브라우저 검사), Risk Low (중복 문구 제거, 위치 정책과 IPC 유지).
