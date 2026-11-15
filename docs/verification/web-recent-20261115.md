# 웹 최근 주소 목록 — #186 · 2026-11-15

## 구현과 보존 범위

사용자가 웹 시작 화면에서 최근 연 주소를 목록으로 쉽게 이동하고 삭제하도록 요청했다. 최대10개·중복최신순·재시작보존은 이번 구현 선택이다. HTTP(S) 요청 주소를 normalize해 기록하고 자동 복원/redirect/로딩 성공 여부와 구분한다. 해당 instance Store 루트의 `.web-recent.json`에 NamedTempFile·sync·persist로 원자 저장한다. 삭제는 목록만 바꾸고 웹 창을 닫지 않는다. 저장 실패는 이미 열린 세션의 성공을 되돌리지 않으며 create 응답의 `recent_saved:false`로 표시한다.

`web_recent`은 실제 등록된 Welcome 소유 `webview-*`에서만 조회/삭제하며 외부 `webpage-*`의 기존 IPC 거부를 유지한다. 새 capability/외부 bridge·eval 권한은 추가하지 않았다. JSON 로드시 HTTP(S)/무자격증명/내부도메인 거부 규칙을 재적용하고 유효한 주소만 textContent로 표시한다. 읽기·삭제 실패 시 목록을 보존하고 상태 안내를 제공한다.

이전 미커밋 문구 축소(라벨·설명 제거, 이동 버튼, 접근성 이름 웹 주소/status 연결)는 보존했다. 최근주소는 제목+목록+행별 삭제로 구현하고 빈 목록은 숨긴다. 긴 주소는 말줄임/전체 tooltip·접근성이름을 제공한다. 최대10개가 작은창을 넘으면 본문 스크롤을 사용한다. 삭제 후 인접 삭제버튼/마지막 삭제 후 입력으로 포커스를 복원한다.

## 출처와 검사

- 작업 루트 `/Users/egoinglee/.codex/worktrees/web-mobile-chrome/agent-gibbon`, `codex/web-welcome-copy`, HEAD 기준 `ea0531a`, 제품 변경 미커밋.
- Rust226·UI180 통과. 최근주소 신규 Rust2개에서 파일 재조회·중복·최신순·10개 제한·삭제·루트 격리·거부 URL 정제·손상파일 보존을 확인했다.
- Clippy all-targets -D warnings 및 개발 빌드 통과. native 앱을 열거나 종료하지 않았다.
- `scripts/web-recent-smoke.mjs`: 실제 frontend 자산+mock native IPC로 240×160,320×240,600×400의 접근성이름/라벨제거/목록·Enter즉시이동·연결오류 입력보존·삭제실패 보존·삭제포커스·페이지reload후 목록유지·빈목록 숨김·가로overflow 없음을 확인했다. 실제 filesystem 재시작 보존 근거는 Rust 파일 재조회 검사이며 mock reload와 구분한다.
- 최초 브라우저 검사에서 새 버튼이 삭제중 disabled인 시점에 focus를 호출해 복원에 실패했다. 버튼 재활성화 뒤 포커스를 복원하도록 제품 코드를 고치고3조합을 다시 통과했다.
- 기존 web-toolbar smoke의 web_recent 응답 mock을 갱신하고 5테마×3너비15조합을 확인했다. 기존 web-welcome smoke도 조회 호출을 탐색 호출 수에 섞지 않도록 mock을 갱신했다.
- SVG를 rsvg-convert로 PNG 렌더하고 실제 열람했다. [브라우저600 캡처](web-recent/browser-600.png) · [240 캡처](web-recent/browser-240.png). native 캡처가 아니며 mock 환경이다.

## 최신 main 통합과 미검증

요청에 따라 fetch한 origin/main은 `6d1bf73`이다. `git merge --ff-only origin/main`은 기존 미커밋 `docs/design-system.md` 덮어쓰기 보호로 중단됐다. 자동 stash/reset/제품 커밋 없이 모든 기존/신규 변경을 보존했다. 최종 승인 후 커밋→최신 main 통합→관련 재검증이 필요하며 현재 결과를6d1bf73 검증으로 보고하지 않는다.

Orca/제거 스킬·native GUI를 사용하지 않았고 기존 사용자 앱·창·포커스를 조작하거나 종료하지 않았다. native Welcome 연결·실제 앱 재시작·외부페이지 신규 명령거부의 실환경 재검증과 Windows/Intel·전체 플랫폼 CI/배포는 미실행이다. 기존177의 미완료 검증도 이번 작업으로 완료 처리하지 않는다.

Conf4(브라우저 상호작용·Rust 저장 통합 검사), RiskLow(로컬 최근주소 목록, 삭제로 열린세션에 영향 없음). 최종 커밋·push·PR·병합은 새 승인 대기다.
