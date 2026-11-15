# 웹 시작 화면 문구 축소 — #177 후속

- 기준: origin/main `ea0531aceff0afcddcf2e96d7ca7086c29f367d4`. 워크트리 `/Users/egoinglee/.codex/worktrees/web-mobile-chrome/agent-gibbon`, 브랜치 `codex/web-welcome-copy`. 기존 병합PR181에 추가하지 않고 현재 변경은 미커밋으로 보존한다.
- 사용자 요청: 화면의 ‘웹 주소’ 라벨·HTTP(S) 안내 문장 제거, ‘실행’ 버튼을 ‘이동’으로 변경. 입력 accessible name은 aria-label 웹 주소로 유지하고 aria-describedby는 존재하는 status만 가리킨다. 제거한 요소의 CSS는 삭제하고 입력과 버튼 사이12px를 둔다.
- 디자인 시스템 및 연결 SVG/PNG를 갱신하고 `rsvg-convert`로 렌더 후 실제 열람했다.
- `npm run test:ui`: UI180 통과.
- `scripts/web-toolbar-smoke.mjs`: Chromium 5테마×3너비144/240/600의 toolbar/접힘/핀/hover/키보드/Welcome hidden15조합 통과.
- 추가 Chromium 검사: 240×160,320×240,600×400에서 textbox accessible name 웹 주소, 이동 button 이름, 라벨/힌트 제거, status 연결, Enter 제출·실패 입력 보존·버튼 재활성화·클릭 재시도, 가로 overflow 없음 확인. 첫 추가 검사에서는 mock Error 문자열을 정확히 비교해 timeout했으며 `Error:` 접두어를 포함하는 실제 문자열을 처리하도록 검증 조건을 고친 뒤3조합 통과했다. 제품 로직 변경은 필요하지 않았다.
- [실제 자산의 Chromium 캡처](web-welcome-copy-20261115.png)와 시안은 구분한다. 캡처는 mock IPC 환경이며 native/외부 웹 탐색 증거가 아니다. 추가 작은화면 캡처: `/tmp/gibbon-welcome-copy-20261115/clean-240.png`.
- native 앱/GUI·Orca는 사용하거나 종료하지 않았다. Rust 변경·native 빌드·실환경 GUI 재검증은 수행하지 않았다. 기존177의 남은 native 검증은 이번 문구 변경으로 해결했다고 표시하지 않는다.
- 이번 문구 변경 Conf4(실제 프론트 자산 브라우저 검증), RiskLow. 최종 제품 커밋·push·새PR은 새 승인 전 대기한다.


사용자 close 후 문구 축소는 최근주소186과 함께 제품1ae87c3 및 main통합ab6ea09에 포함했다. 최신통합 Rust231/UI184·Clippy/build·Chromium 최근목록3화면/툴바15조합을 재검증했다. 최종 PR187/이슈186에서 병합 결과를 추적하며 앞선 승인대기/미커밋 표기는 당시 상태다.
