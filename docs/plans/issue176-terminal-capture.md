# 터미널·캡처 본문 통일 — #176

기준 커밋: main `91b1751` · 전용 브랜치 `codex/terminal-capture-spacing`

## 계획과 완료 기준

- [x] 첨부 두 화면과 디자인 시스템·툴바/활성 외곽/overlay scrollbar 시안 확인
- [x] 터미널·캡처 툴바 22px와 본문 상단 추가 여백 0 적용, 불필요한 외부 여백 제거
- [x] 터미널의 라운드 바깥 투명 배경과 텍스트 기준 overlay scrollbar 구현
- [x] Chromium에서 scrollback·alternate-screen·TUI·휠·키 입력·thumb drag·fit/resize IPC 검증
- [x] 이미지 aspect·contain·상단 간격 스타일 회귀 검증
- [ ] 실제 이미지 조작·원본 복사와 native PTY resize 확인 — 사용자 GUI 중단 요청으로 보류
- [x] 세 테마·크기·device scale 비교, 개발 CLI 출처 검증과 native 미검증 범위 기록
- [x] 디자인 시스템·연결 SVG/PNG 갱신과 렌더 확인

웹 scrollbar(팀 16)와 더블클릭(팀 17)은 제외한다. 공통 CSS·toolbar 변경 필요 시 팀장에게 알린다. GUI는 팀장 슬롯 승인 뒤에 사용하고 PID·실행 파일·instance·inspect·native 제목으로 출처를 확인한다. 기존 앱과 사용자 메모·포커스·기하는 보존한다.

계획 커밋과 Draft PR만 사전 승인 범위다. 최종 제품 커밋·push·병합은 검증 후 별도 승인을 기다린다. 전체 플랫폼 CI·배포는 실행하지 않는다.

## 현재 결과

UI 204·Rust 223·Clippy·Rustfmt·debug 빌드·Chromium 18조합 통과. [검증 기록](../verification/issue176-terminal-capture.md)을 따른다. 제품 변경은 로컬 미커밋 상태이며 계획 Draft PR만 원격에 있다. 사용자 요청에 따라 Orca 추가 실행과 새 native GUI 시험을 중단하고 검증 방식 결정 및 최종 제품 커밋·push 승인을 기다린다.

## close 승인

2026-11-15 사용자가 close를 호출하여 최종 커밋·push·병합을 승인했다. 최신 main 6d1bf73 통합 후 UI184·Rust229·xterm18조합·Welcome14조합·Milkdown 브라우저 회귀·Clippy·Rustfmt·debug 빌드가 통과했다. 미검증 native 범위는 검증 기록에 보존하며 미실행을 완료로 바꾸지 않는다. 병합과 main 동기화의 최종 근거는 이슈 #176에 기록한다.
