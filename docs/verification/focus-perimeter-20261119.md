# 포커스 메모의 연속 테두리 — 2026-11-19

## 변경과 범위

- 기존 공통 `memo-frame.css`는 포커스된 메모의 툴바 상단에만 inset shadow를 그렸다. 자식 버튼의 hover/네이티브 포인터 강조 배경은 부모의 shadow 위에 그려졌다.
- 텍스트·캡처·터미널 공통 `#memo-surface::after`에 2px 내부 테두리를 그린다. 표면 경계와 둥근 모서리를 따르며 콘텐츠·툴바(z=2)·스크롤바(z=4) 위(z=20)에 표시한다. 입력을 받지 않아 기존 클릭·툴바 드래그·resize hit area를 유지한다.
- padding·border·창 크기·이미지 object-fit·툴바 행 높이·포커스 이벤트 처리는 변경하지 않았다. 하단 배치 툴바도 표면 전체 경계를 사용한다. forced-colors는 Highlight 내부 outline을 사용한다.
- 외부 페이지의 별도 네이티브 WebView, 비활성 캡처의 그림자와 네이티브 창 설정은 변경하지 않았다.
- [디자인 시스템](../design-system.md)과 [SVG](../design/toolbar-reserved-space.svg)·[PNG](../design/toolbar-reserved-space.png)를 갱신했다. 이미지 툴바 강조 배경 위에 테두리를 표시한다. 이 그림은 설계 시안이며 앱 실행 증거가 아니다.

## 실행 근거와 한계

- 작업 루트: `/Users/egoinglee/.codex/worktrees/2319/agent-gibbon`
- 읽은 정책: 작업 루트의 `AGENTS.md`와 기본 checkout의 `/Users/egoinglee/dev/project/agent-gibbon/AGENTS.override.md`.
- HEAD: `548ce56f2ca4f001d0e8d3ae4a2794fa2dd811dc` + 이번 미커밋 변경. 커밋·push·병합·배포는 수행하지 않았다.
- 호스트: Darwin arm64, macOS 15.6.1, Node v24.21.0, pnpm 11.25.0. `pnpm install --frozen-lockfile` 성공, lockfile 변경 없음.
- 개발 인스턴스: `dev-d92bf86f537596143f065315`
- 바이너리: `/Users/egoinglee/.codex/worktrees/2319/agent-gibbon/src-tauri/target/debug/jajan`
- 바이너리 SHA-256: `9ca2e4bea35030d4a60f7b04d699e86131dd4fb5f220dcbb7cee8908188ed78a`
- 빌드: `npm run dev:build` **성공** (`Finished dev profile`, 21.68s). 위 HEAD에 이번 미커밋 제품 CSS 변경을 포함했다. 바이너리는 생성했으며 실행하거나 기존 앱을 재시작하지 않았다. 이 checkout에는 `scripts/agent-gibbon-dev`가 없으므로 실제 실행기는 `./scripts/jajan-dev open`이다. 이미 같은 개발 인스턴스가 실행 중이면 새 바이너리 적용에 정상 종료 후 재실행이 필요하다.
- 시안: `rsvg-convert`로 PNG를 생성하고 이미지를 열어 네 변·모서리 연결을 확인했다.
- GUI 사전 확인: Computer Use의 현재 Mac 앱 목록은 조회 가능하다. 지정 iMac 테스트실의 실제 시험 앱은 이 로컬 도구로 확인·조작하지 않았다. 커밋·push 승인이 없어 원격에서 받을 전체 SHA를 준비하지 않았으며 정식 시험 job을 제출하지 않았다.
- GUI 검증: **BLOCKED** — 지정 iMac의 정확한 소스·바이너리·PID·창 연결 및 Computer Use 조작이 미확인이고 원격 전달용 커밋이 준비되지 않았다. 제품 실패를 뜻하지 않는다.
- 정책에 따라 단위·통합·정적 검사·모의 브라우저 테스트는 실행하지 않았다.

## 후속 실기기 확인 기준

| 확인 | 기대 결과 | 결과 |
| --- | --- | --- |
| 포커스된 캡처의 네 변과 둥근 모서리 | 밝거나 어두운 이미지 위에서 2px 연속 테두리 | NOT RUN |
| 툴바 버튼 hover와 네이티브 포인터 강조 | 배경 위로 테두리가 연속 표시 | NOT RUN |
| 캡처 툴바 상단/하단 배치 | 두 배치 모두 메모 전체 둘레 표시 | NOT RUN |
| 포커스 전환 | 새 메모에만 표시, 비활성 그림자는 기존 동작 유지 | NOT RUN |
| 클릭·본문 입력·이미지 복사·툴바 드래그 | 기존 동작과 동일 | NOT RUN |
| 네 변과 네 모서리 리사이즈 | hit area와 조작 유지 | NOT RUN |
| 캡처 표시 크기·비율·투명 영역 | 변경 전과 동일 | NOT RUN |
| 텍스트·터미널·툴바 접힘 | 공통 표면 경계와 툴바 버튼 조작 유지 | NOT RUN |
| forced-colors | Highlight 내부 outline 표시 | NOT RUN |

Conf 1: 코드 검토와 시안 렌더 확인만 수행했으며 실행 중 제품 검증은 없다. 개발 빌드 성공은 GUI 검증을 대체하지 않는다. Risk Low: 공통 포커스 장식 CSS에 국한되며 원복 가능하다.
