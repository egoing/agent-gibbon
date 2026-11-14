# #166 툴바 Tab 포커스 테두리

## 출처와 변경

- 작업 루트: `/Users/egoinglee/.codex/worktrees/fd96/agent-gibbon`
- 브랜치: `codex/toolbar-focus-opacity`, 기준 `d1c3e8a`, 계획 커밋 `654a29e`.
- `npm run dev:info`: instance `dev-35d850d465b26f927bd60d16`, binary `/Users/egoinglee/.codex/worktrees/fd96/agent-gibbon/src-tauri/target/debug/agent-gibbon`.
- 공통 `#controls button:focus-visible` 선만 전경색 28% 알파로 변경. 2px 두께·-2px offset·포커스 배경·아이콘·키보드 이동 유지. forced-colors는 시스템 Highlight로 복귀한다.
- Node v24.21.0, npm 11.19.0, rustc 1.91.1, Chromium 158.0.8127.18. 기존 package-lock 변경 없음.

## 실행한 검사

- `npm run test:ui`: 198/198 통과.
- `PLAYWRIGHT_MODULE=/tmp/gibbon-playwright/node_modules/playwright/index.mjs GIBBON_SCREENSHOTS=/tmp/gibbon166-render node scripts/toolbar-focus-smoke.mjs`: 텍스트·이미지·터미널·웹 × 밝음·회색·검정 12조합 통과. 실제 HTML/CSS를 사용하고 JS/IPC를 로드하지 않는 브라우저 렌더 검사다. Tab/Shift+Tab, 28% 계산색, 선 두께·offset·버튼 bounds, forced-colors의 비투명 시스템 색을 확인했다.
- 같은 PLAYWRIGHT_MODULE로 `node scripts/toolbar-layout-smoke.mjs`: 6조합 통과. `node scripts/web-toolbar-smoke.mjs`: 15조합 통과. 기존 Tauri mock 회귀 검사다.
- `git diff --check`: 통과.
- SVG 두 파일을 `rsvg-convert`로 PNG 렌더하고 이미지 확인. [세 테마 설명 시안](../design/toolbar-focus.png)에서 100%와 28%를 비교했다. [실제 Chromium 세 테마 캡처](issue166-toolbar-focus/chromium-themes.png)에서도 포커스 배경과 선이 식별되며 원본보다 선 강조가 줄었다. 28% 선만으로 접근성 대비 기준을 충족한다고 주장하지 않는다.

## 초기 실패와 재검증

- `npm run dev:build`: 의존성 컴파일 중 `tauri-utils`에서 `No space left on device`로 실패. 로그 `/tmp/gibbon166-build.log`. 가용 디스크가 209MiB까지 감소해 추가 빌드를 중단하고 이번 워크트리에서 생성한 target 산출물만 cargo clean으로 정리했다.
- 초기 실패 후 디스크 여유가 28GiB로 회복되어 `npm run dev:build`를 다시 실행해 성공했다(로그 `/tmp/gibbon166-close-build.log`). 최신 main `87aedf7`을 일반 merge로 통합한 `16543dd`에서도 개발 빌드가 8.65초에 성공했다(로그 `/tmp/gibbon166-integrated-build.log`).
- 통합 후 `npm run test:ui`: 203/203 통과. 포커스 12조합·toolbar 6조합·web 15조합 재실행 통과. `scripts/editor-entry-height-browser.mjs`의 편집 높이 5조합 회귀도 통과했다. 해당 로그는 `/tmp/gibbon166-integrated-{ui,focus,toolbar,web,editor}.log`에 있다.
- 실제 CLI `open/update/inspect/close --id focus166-fd96-native`를 실행해 `ok: true`, 응답 instance `dev-35d850d465b26f927bd60d16`, 고유 본문 `포커스 검증 fd96 166`의 일치를 확인했다. PID 90375의 실행 경로는 위 fd96 debug binary의 `--serve`였다. CLI 원본 응답은 `/tmp/gibbon166-native-{open,update,inspect-final,close}.json`이다.
- **실앱 GUI 미검증:** Orca가 실행되지 않아 runtime을 사용할 수 없었고 CUA가 debug 바이너리 경로/프로세스명을 `Invalid app`으로 거부했다. 설치 앱이나 다른 워크트리로 대체하지 않았다. 팀장이 현재 브라우저 근거와 CLI 출처를 구분하고 이 국소 CSS 변경의 병합을 진행하도록 확인했다. 실제 native Tab 화면·Windows 실제 고대비는 통과로 보고하지 않는다.
- 직접 만든 검증 메모만 CLI close로 숨기고 정확한 경로를 재대조한 PID 90375만 SIGTERM으로 종료했다. 사용자 앱·창 위치·포커스는 조작하지 않았다. GUI 슬롯을 반환했다.
- 원격 전체 플랫폼 검사는 실행하지 않았다. 릴리스가 아닌 국소 CSS 변경이므로 전체 플랫폼 워크플로는 요청하지 않았다. PR 필수 status check는 등록돼 있지 않다.
- 사용자의 close 호출로 제품 커밋·push·병합 및 main 동기화를 승인받았다. 팀장에게 GUI/main 슬롯을 배정받아 초기 디스크 실패 후 재검증을 완료했다.

Conf 4: 실제 CSS의 브라우저 렌더·키보드 검사 통과, native 검증 한계는 위와 같다. Risk Low: 공통 툴바의 국소적 시각 변경이며 쉽게 복원 가능하다.
