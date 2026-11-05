# #57 Welcome 구성 개선 토론 — 2026-11-05

> **후속 결정:** 사용자가 확정한 [최신 와이어프레임](../design-system.md#사용자-확정-와이어프레임--2026-11-05)이 아래 합의를 대체한다. 최근 폴더 목록·텍스트 선택 행·별도 실행·상시 읽기 전용 명령 미리보기가 현재 기준이며 이 문서는 이전 논의의 역사로 보존한다.

## 성격과 범위

사용자의 전문가 토론 요청에 따라 UX/정보구조 검토자, 접근성 검토자, Argon이 독립 제안과 상호 반론을 제출하고 주 작업자가 합의안을 구현했다. 역할에 따른 설계 검토이며 실제 사용자를 모집한 실증조사, 사용성 점수 측정, 전문가 자격 인증을 의미하지 않는다. UX 검토자는 실제 UI 파일과 디자인 시스템을 읽었으며 Argon 1차는 제공된 요구사항만으로 분석했다.

대상은 `ui/terminal.html`, `ui/terminal.css`, `ui/terminal.js`의 시작 화면 구성이다. 기존 에이전트 자동 시작, PTY 수명, 숨김·종료, 설치 감지, resume 제외 정책은 변경하지 않는다. 합의된 시각 기준의 원본은 [디자인 시스템](../design-system.md#terminal-welcome--57-2026-11-05)이다.

## 제안·반론·합의

| 관점 | 초기 제안 | 반론과 조정 | 최종 채택 |
|---|---|---|---|
| UX/정보구조 | 큰 Welcome 축소, 작업 위치→실행→설정 순서, 제목 폼 접기, 단일 명령 details | 셸을 보조 행에 두면 범용 터미널 진입이 묻힐 수 있음. 긴 경로 전체 줄바꿈은 작은 창의 실행 영역을 밀어냄 | ‘작업 시작’과 항상 보이는 셸 열기, 작업 위치 행, 두 열 앱 버튼, 하단 details. 경로 원문은 두 줄 높이 안에서 스크롤 |
| 접근성 | 작은 제목·홈 설명·제목 details, 단일 클릭 실행 유지, 시작 상태·실패 초점 복원·명시 실행 후 입력 초점 | 카드 선택 후 별도 실행은 Tab 이동과 스크롤을 늘림. 명령 확인을 Tooltip에만 맡기면 키보드 접근성이 약함. 초점 변경은 IME 회귀 검증 필요 | 앱 버튼 자체가 실행, 명령 본문 details, 명시적 focus-visible, 상태 안내, 조건부 성공 초점/실패 복원 |
| Argon 1차 | 위치 우선, 에이전트/셸과 보조 도구 분리, 인라인 실제 명령, 타이틀바 제목 편집, 작은 창 설명 축약 | 절대경로 명령·다중 그룹·상시 동작 설명은 작은 창 높이를 늘림. 타이틀바 편집은 드래그와 충돌 가능. 설명 전체 숨김은 앱 역할 이해를 약화 | 역할 설명이 있는 동일한 두 열 앱 버튼, 명령 단일 details, 본문 제목 details, 좁은 화면 패딩 축소로 절충 |
| Argon 2차 | 통합안 수용, 단일 명령 확인·홈 위치 설명·초점 복원·Enter 적용 권고 | 모든 앱과 details를 320×240에서 항상 무스크롤로 보장하는 것은 긴 경로 등 가변 상태와 충돌 | 실행 우선 배치와 필요 시 스크롤. 제목 Enter 제출 유지. 무스크롤 보장은 하지 않음 |

Argon 원문은 작업 당시 `/tmp/gibbon-welcome-review/argon-round1.txt`, `argon-round2.txt`에 저장했다. 임시 파일은 영구 문서의 의존성이 아니며 위 표가 토론의 보존용 요약이다. Argon이 예시로 제안한 셸 명령/옵션이나 첫 답변의 Git 명령은 실제 구현 명령의 근거로 사용하지 않는다. 명령 details는 백엔드가 반환한 설치 앱의 실제 명령만 표시한다. 셸 실제 명령 추가 표시는 이번 변경에 포함하지 않았다. 빈 제목 제출은 기존 사용자 제목 유지가 아니라 기본 제목 복원이다.

## 시안과 검증 경계

[SVG](../design/terminal-welcome.svg)와 [PNG](../design/terminal-welcome.png)는 합의된 구성의 설명용 시안이다. 기록 경로, 경로 없는 시작, 설치 앱 없음의 세 상태를 보여 주며 실제 앱 캡처가 아니다. SVG를 `rsvg-convert`로 PNG로 렌더링해 텍스트·버튼·배치의 겹침을 시각 확인했다.

이번 토론이나 시안 검토로 기존 is2 게이트 실패와 독립 감사의 퇴행 신호를 해소했다고 판단하지 않는다. [기존 검증 기록](issue57-terminal-welcome.md)의 부분 범위 탐색 테스트 실패(최종 Clippy는 통과), 자동 병합 제외 및 macOS 최종 네이티브 GUI·Windows 실앱 미검증을 그대로 유지한다. 이번 UI 실행 검증 결과는 주 작업자의 해당 검증 기록을 따르며 시안 렌더를 네이티브 검증으로 대체하지 않는다.


## 후속 구현 검증

- 작업 루트: `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/57-terminal-welcome`.
- `npm run test:ui`: 64/64 통과. `node --check ui/terminal.js`, `git diff --check` 통과.
- `npm run dev:build`: 개발 바이너리 빌드 통과. 실행 중인 앱은 재시작하지 않았으며 현재 native 화면이 최신 빌드라고 주장하지 않는다.
- `PLAYWRIGHT_MODULE=/tmp/agent-gibbon-is2-57/browser-tools/node_modules/playwright/index.mjs node scripts/terminal-welcome-smoke.mjs`: 720×460·320×240 통과. 실제 HTML/CSS/xterm 자산을 사용하되 Tauri 호출은 모의 처리했다. 앱 4개/없음, 긴 경로, 키보드 details, 제목, 앱·셸 실행, 실패 재시도와 닫기 요청을 확인했다.
- 독립 코드 리뷰에서 늦은 실패 응답이 사용자가 툴바로 옮긴 포커스를 되돌리는 문제를 발견했다. Welcome 복원 전에 현재 포커스를 확인하도록 수정하고, 지연 Promise 실패 후 핀 버튼 포커스가 유지되는지 두 크기에서 재검증했다.
- 시작 화면 [720×460](issue57-welcome-browser.png) · [320×240](issue57-welcome-browser-small.png)을 직접 시각 확인했다. 네이티브 화면·실제 PTY·VoiceOver 검증 증거는 아니다.
- 사용자 보고 입력 문제는 별도 [한글 조합 표시 #62](https://github.com/egoing/agent-gibbon/issues/62), [보조키+방향키 #63](https://github.com/egoing/agent-gibbon/issues/63)로 등록했다. 이번 구성 변경에서 해결했다고 주장하지 않는다.
