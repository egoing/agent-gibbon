# 텍스트 메모 오버레이 스크롤바

사용자가 2026-11-13 제안 시안을 승인한 뒤 구현했다. [디자인 기준](../design-system.md#텍스트-읽기-화면-여백오버레이-스크롤바--2026-11-13)을 따른다.

- 작업 루트: `/Users/egoinglee/.codex/worktrees/02ec/agent-gibbon`
- `npm run dev:info`: instance `dev-f3ab8dc6a8d44e989b85031f`, binary `/Users/egoinglee/.codex/worktrees/02ec/agent-gibbon/src-tauri/target/debug/agent-gibbon`.
- 이번 변경: `ui/overlay-scrollbar.js`·CSS 추가, index 로딩·본문 Tab 접근·scrollbar ARIA 추가, app의 자연 높이 측정 패딩을 실제 본문과 일치시킴. 다른 세션의 기존 변경 보존.
- `npm run test:ui`: 188개 통과. 추가 2개는 thumb 양 끝·최소 크기·짧은/빈 문서의 기하 검증.
- `npm run dev:build`: 성공. Rust 코드는 이번 작업에서 변경하지 않음.
- Playwright Chromium, loopback 임시 서버에서 실제 index/CSS/overlay 모듈을 로드했다. Tauri 관련 스크립트는 비워 브라우저에서 독립 검증했다. 22px 툴바·상단 패딩 0·전용 스크롤 열 0, 긴/짧은 문서 전환, End/Home, 휠, pointer thumb 드래그, 편집/이미지/접힘 제외, 144/300/600px 폭, 페이지 오류 없음을 확인했다. 서버와 브라우저는 검사 후 종료했다.
- [300×291 CSS px 브라우저 캡처](assets/overlay-scrollbar-20261113.png). 메모 기본 테마·긴 테스트 본문이며 네이티브 앱 캡처가 아니다. 검증 스크립트는 `/tmp/gibbon-overlay-browser.cjs`에 있다.
- 최초 구현 단계에서는 앱을 조작하지 않았다. 최종 병합 준비에서 개발 CLI `open`·`inspect`로 검증 메모 `overlay-final-20261113`의 `ok: true`, instance `dev-f3ab8dc6a8d44e989b85031f`, 본문 고유 표식을 확인했다. 실제 실행 경로는 위 debug binary이며 PID 41265였다. `orca computer list-windows --app pid:41265 --json`은 빈 창 목록을 반환해 네이티브 창을 특정할 수 없었다. GUI 입력·캡처를 진행하지 않았고 네이티브 UI 통과로 보고하지 않는다. 검증 메모를 `close`하여 숨김을 확인한 뒤 이번에 시작한 PID만 경로를 재확인하고 종료했다. 원래 해당 개발 앱은 실행 중이지 않았다. macOS WKWebView·Windows·물리 트랙패드 관성·스크린리더·고대비 실환경은 미검증이다. OS scrollbar 상시 표시 선호 감지는 미구현이며 CSS 대비/강제색상 환경 대응만 구현했다.
- 최종 검증: 최신 main `97a044e`를 fast-forward로 반영한 `codex/memo-overlay-scrollbar`에서 UI 188개·Chromium 검사·개발 빌드·`git diff --check` 통과. 글자 크기 CSS 변경 시 geometry를 다시 계산하는 관찰도 포함한다. 별도 전체 플랫폼 검사는 실행하지 않았다. Conf 4(브라우저), Risk Low.
- 의존성이 바뀌지 않은 lockfile 필드 순서 차이는 이번 커밋에서 제외하고 보존했다. 병합 결과와 최종 SHA는 PR에 기록한다.
