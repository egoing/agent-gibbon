# #15 · 취소한 초안 복구 검증 (2026-11-16)

기준 main `22dc7de7309f8d47772bb0cf1f2c88768f742ef8`, 전용 브랜치 `codex/issue15-cancelled-draft`, 계획 커밋 `41c19a36574052c3585e111975001f9201808856`. 이 독립 검증을 수행할 당시 [PR #210](https://github.com/egoing/agent-gibbon/pull/210)에는 계획 커밋만 있었다. [#15](https://github.com/egoing/agent-gibbon/issues/15)의 분리 편집기와 현행 [#174](issue174-milkdown.md)의 차이: 현재 정상 입력은 600ms 자동저장하며, 실패하면 ‘변경 취소’가 나타난다. Esc는 저장 뒤 숨기기다.

## 재현과 검증

독립 Chromium에서 해당 worktree의 index.html, 실제 Milkdown 번들, app.js, 내부 Feedback과 버튼 이벤트를 사용했다. 최종 23개 시나리오가 모두 통과했고 브라우저 오류와 외부 요청은 0이었다. Tauri 호출·저장소 응답·가시성은 대역이다. 같은 테스트에서 저장 실패·응답 지연·외부 revision을 재현할 수 있게 했다. 시스템 전체 키보드 또는 사용자의 메모를 사용하지 않았다.

| 확인 경로 | 결과 |
|---|---|
| 기존 main: 입력 → 저장 실패 → 변경 취소 → Cmd+Z | ‘원문 잃어버릴 초안’이 ‘원문’으로 바뀌고 undo 후에도 원문. 복구 버튼 0개. |
| 복구 클릭·키보드, undo/redo, 명시 저장 | 복구 직후 dirty, undo는 현재 원문, redo는 복구 내용. 같은 revision에서는 명시 저장과 입력 유휴 후 자동저장 모두 성공했다. 144px 창에서 실제 Tab → Enter로 복구했다. |
| 취소 직전·직후 외부 변경, 색만 바뀐 revision | 복구 본문이 남고 최신 외부 원문은 유지된다. 알려진 충돌은 600ms 이후에도 자동 쓰기 없이 대기하고, 명시 재시도도 최초 revision으로 거절된다. |
| 충돌 복구 후 undo/redo·새 입력 | redo는 원래 CAS를 유지한다. undo로 이력을 모두 되감은 뒤 시작한 새 입력만 최신 기준을 쓴다. 본문을 손으로 최신 원문과 같게 맞춰도 undo 이력에 옛 초안이 남는 동안 쓰기 기준을 바꾸지 않는다. |
| 최신 수신본이 초안과 같은 문서 | 명시 복구는 후보만 소비한다. 더티 상태, 저장 요청, revision 증가는 없다. |
| 재시도 실패·다시 취소·늦은 복구 이벤트 | 원문과 초안을 보존한다. 새 dirty 입력은 이전 후보를 폐기하며, 수신된 오래된 버튼 세대번호가 새 후보·새 입력을 바꾸지 않는다. |
| 숨김·표시, Esc, 별도 WebView | 숨김 복원은 후보를 유지하지만 자동 복구하지 않는다. Esc는 성공 시 저장하고 숨기며 실패하면 현재 편집을 유지한다. 별도 페이지 사이에 후보가 공유되지 않는다. |
| 원문 보호, 합성 IME 조합, 저장·내보내기 중 | 복구를 막고 기존 본문을 유지한다. 조합·원문 보호가 해제된 뒤에만 다시 복구할 수 있다. 저장 응답과 겹친 후속 입력을 유지한다. |

### ProseMirror 플러그인 변경과 redo

첫 검토 패치에서는 충돌 초안을 복구한 뒤 undo로 최신 원문(revision 2)에 도달하면, `addToHistory: false`인 변경이 redo 한 건을 보존하면서 초안의 기원(revision 1)을 지울 수 있었다. 직접 문서를 바꾼 경우와 비이력 부모 트랜잭션에 플러그인이 문서 변경을 덧붙인 경우를 재현했다. 둘 다 redo 후 600ms 자동저장이 revision 2로 성공하여 fixture 저장소의 최신 원문을 초안으로 바꿨다.

설치된 Milkdown 7.23.0의 CommonMark preset은 제목 ID 동기화에 직접 비이력 변경을, 목록 번호 동기화에 비이력 추가 변경을 사용한다. 두 회귀 시험은 제품 편집기 소스를 같은 라이브러리로 묶고 테스트 페이지에서만 EditorView에 접근해 트랜잭션을 주입한다. 실제 두 플러그인의 특정 키 입력 시퀀스로 증상을 재현했다는 뜻은 아니다.

수정 후 두 경로에서 redo로 초안이 돌아와도 750ms 동안 자동 쓰기가 없고, 명시 저장은 원래 revision 1로 거절되어 외부 원문(revision 2)이 남는다. undo 뒤 일반 타이핑으로 redo를 버린 경우에는 최신 기준으로 저장한다.

실행: `pnpm install --frozen-lockfile`, `npm run build:editor` 후 Playwright를 사용할 수 있는 환경에서 `node scripts/cancel-draft-browser.mjs`. 선택 환경변수: `PLAYWRIGHT_MODULE`(모듈 경로), `PLAYWRIGHT_CHROMIUM_EXECUTABLE`(headless 실행 파일), `GIBBON_SMOKE_OUTPUT`(결과 폴더). 원인 재현은 수정 전 제품에서 같은 스크립트의 `GIBBON_BASELINE=1` 모드. 기존 전체 milkdown-browser 스크립트는 #15와 무관한 옛 툴바 mousedown 드래그 기대값을 포함한다. 현재 pointerdown·4px 이동 계약을 바꾸거나 해당 전체 스크립트를 통과로 집계하지 않았다.

로컬 macOS arm64, Node 24.21.0, pnpm 11.25.0, Rust/Cargo 1.91.1. `npm test`: Rust 267개, UI 198개 통과. rustfmt 검사, Clippy(all targets, warnings denied), 편집기 번들 및 `npm run dev:build` 성공. `./scripts/agent-gibbon-dev --instance-info`로 전용 debug 바이너리 식별만 확인했다. 작업 루트 `/Users/egoinglee/.codex/worktrees/a15c/agent-gibbon`, instance `dev-746209b8f15ed327664fcfa9`, binary `src-tauri/target/debug/agent-gibbon`. 개발 앱은 실행하지 않았으므로 native inspect, PID·창 식별, WKWebView, 실물 한글 IME·키보드·포커스, Windows·Intel, 설치·배포 검증은 하지 않았다. #202의 실물 입력 장애에 관한 판정이 아니다.

[디자인 시스템](../design-system.md) · [설명용 시안](../design/milkdown-editor.png). SVG를 PNG로 렌더해 본문·버튼·문구를 육안 확인했다. Conf 4 / Risk Medium(초안·쓰기 revision 관리).
