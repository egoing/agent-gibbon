# 터미널·캡처 본문 통일 — #176

기준 커밋: main `91b1751` · 전용 브랜치 `codex/terminal-capture-spacing`

## 계획과 완료 기준

- [x] 첨부 두 화면과 디자인 시스템·툴바/활성 외곽/overlay scrollbar 시안 확인
- [ ] 터미널·캡처 툴바 22px와 본문 상단 추가 여백 0 적용, 불필요한 외부 여백 제거
- [ ] 터미널의 라운드 바깥 투명 배경과 텍스트 기준 overlay scrollbar 구현
- [ ] scrollback·alternate-screen·TUI·휠·키 입력·thumb drag·fit/PTY resize 검증
- [ ] 이미지 aspect·기존 조작·원본 복사 회귀 검증
- [ ] 세 테마·크기·배율 비교와 출처를 확인한 개발 앱 검증 기록
- [ ] 디자인 시스템·연결 SVG/PNG 갱신과 렌더 확인

웹 scrollbar(팀 16)와 더블클릭(팀 17)은 제외한다. 공통 CSS·toolbar 변경 필요 시 팀장에게 알린다. GUI는 팀장 슬롯 승인 뒤에 사용하고 PID·실행 파일·instance·inspect·native 제목으로 출처를 확인한다. 기존 앱과 사용자 메모·포커스·기하는 보존한다.

계획 커밋과 Draft PR만 사전 승인 범위다. 최종 제품 커밋·push·병합은 검증 후 별도 승인을 기다린다. 전체 플랫폼 CI·배포는 실행하지 않는다.
