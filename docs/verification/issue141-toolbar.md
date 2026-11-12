# #141 접힘 드래그·아이콘 정렬·롤오버

## 요구사항과 구현

- 접힌 메모의 툴바 제목·빈 영역은 `drag_note`로 native 드래그한다. 접힌 본문·테두리 클릭 펼침과 버튼 클릭 제외는 유지한다.
- 펼침 상태도 접힘 기준 아이콘 12px·슬롯 폭 24px·높이 22px·상단 1px로 맞춘다. 펼침 행 30px의 남은 높이는 버튼 아래에 둔다. 터미널 header의 수직 중앙 정렬과 웹 버튼 최소 높이도 함께 맞춘다.
- 버튼 pointer 진입·이탈을 추적하며 포커스가 있을 때만 보완 강조를 설정하고 blur 때 제거한다. CSS hover 배경은 전경색 16% 혼합으로 구별한다. 창 활성화·native 첫 클릭 설정은 변경하지 않는다.
- 디자인 기준: [디자인 시스템](../design-system.md), [SVG](../design/toolbar-reserved-space.svg), [PNG](../design/toolbar-reserved-space.png).

## 실행 출처

- root: `/Users/egoinglee/.codex/worktrees/438f/agent-gibbon`
- branch: `codex/toolbar-fold-drag-hover`
- instance: `dev-46314bac3eb60389ce2c6b0f`
- binary: `/Users/egoinglee/.codex/worktrees/438f/agent-gibbon/src-tauri/target/debug/agent-gibbon`
- PID: `68509` (`--serve`, 정확한 실행 경로를 ps로 확인)
- 검증 메모: `progress-141-438f`. 개발 CLI inspect의 `ok:true`·instance·본문 일치 확인. native GUI 도구는 `list-windows --app pid:68509`에 빈 목록을 반환해 창 특정·GUI 조작·캡처를 수행하지 않았다.
- Node v24.21.0, rustc 1.91.1, macOS ARM64.

## 검증

- `npm run test:ui`: 183개 통과. 접힌 제목의 native drag 위임·접힘 유지·버튼 제외와 포커스별 pointer 진입/이탈·blur 강조 해제 포함.
- `cargo test --manifest-path src-tauri/Cargo.toml`: 212개 통과.
- `npm run check` (Clippy, 경고 오류 처리): 통과.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`: 통과.
- `npm run dev:build`: 통과.
- `git diff --check`: 통과.
- Chromium: `/tmp/gibbon-toolbar-browser/verify141.cjs`로 실제 UI HTML/CSS를 route한 레이아웃 전용 검사. 400×260 → 접힘 400×24에서 text/terminal/web의 토글·닫기 bounding box가 동일하다. 슬롯 24×22, SVG 12×12, y=1을 확인했다. close hover 전후 투명 배경에서 혼합색 배경으로 변함을 확인했다. Tauri IPC/native 동작은 이 검사에 포함하지 않는다.
- 캡처: `/tmp/gibbon-toolbar-browser/141-index.html.png`, `141-terminal.html.png`, `141-web.html.png` (임시 경로).
- SVG를 `rsvg-convert`로 PNG 렌더링하고 결과를 시각 확인했다.

## 미검증과 후속 절차

- 실제 macOS 접힘 드래그·물리 포인터 hover·비활성 창 첫 클릭, Intel/Windows 실제 GUI는 미검증이다.
- 사용자 커밋 승인을 받았다. 같은 브랜치에 커밋·push 후 Draft PR을 생성하고, 창 제어와 플랫폼 간 레이아웃 영향 확인을 위해 해당 SHA의 전체 플랫폼 검사를 요청한다. 실제 실행 URL·결과는 PR에 기록한다.
- 수동 확인: 메모 접기 → 제목과 빈 공간을 드래그 → 접힘 유지·위치 이동 확인 → 펼치고 같은 버튼 위치 확인 → 활성 창에서 각 버튼 hover 확인 → 다른 앱 활성화 후 hover/첫 클릭 정책 확인.
