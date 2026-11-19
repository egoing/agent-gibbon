# 닫기 버튼 오른쪽 끝 강조

- 작업 루트: `/Users/egoinglee/.codex/worktrees/709b/agent-gibbon`
- 기준 HEAD: `952a0dbbcde3242051bf582d0a5eb73a202aa2c3`; 이번 CSS·디자인 자료는 미커밋 변경이다.
- 인스턴스: `dev-f742052197e209049ae78575`
- 바이너리: `/Users/egoinglee/.codex/worktrees/709b/agent-gibbon/src-tauri/target/debug/jajan`

## 구현

공통 툴바의 오른쪽 3px 패딩을 닫기 버튼 너비에 포함하고 오른쪽 버튼 라운드를 제거했다. 버튼 내부 오른쪽 패딩으로 아이콘 정렬을 보존한다. 기존 표면의 모서리 clip과 z-index 20의 pointer-transparent 활성 테두리를 그대로 사용한다. 내부 버튼 반경·키보드 outline과 resize 영역은 변경하지 않았다. 실제 기준은 [디자인 시스템](../design-system.md#닫기-버튼의-오른쪽-끝-강조--2026-11-19)을 따른다.

## 최종 개발 빌드

- 작업 Mac arm64 / Node v24.21.0 / pnpm 11.25.0.
- `pnpm install --frozen-lockfile` 완료(lockfile 변경 없음).
- `npm run dev:build` **성공** (`Finished dev profile`, 종료 코드 0). 위 HEAD와 이번 미커밋 변경을 포함했다.
- 빌드 후 이 워크트리의 개발 앱 실행 프로세스는 발견하지 않았다. 다른 인스턴스는 종료하지 않았다.

## 확인과 한계

- 설명용 SVG를 `rsvg-convert`로 PNG 렌더링하고 직접 열어 상단·하단·접힌 창의 활성/비활성 배치와 선의 연속성을 확인했다. 제품 GUI 실행 검증이 아니다.
- 지정 iMac 실기기 GUI 시험: **BLOCKED**. 정식 시험 요청에는 승인된 커밋·push와 원격 전체 SHA가 필요하지만 현재 요청은 구현·자료·개발 빌드 범위이며 해당 승인이 없다. 실제 시험 앱의 Computer Use 조작 가능 여부도 아직 확인하지 않았다.
- 진행 메모는 개발 앱 미실행으로 생성하지 못했다(`list`: ok:false). 앱을 임의로 시작하지 않고 대화로 진행을 안내했다.
- 단위·통합·정적·모의 브라우저 검사는 로컬 정책에 따라 실행하지 않았다.
- 후속 GUI 확인: 텍스트/이미지/터미널 공통 툴바의 활성·비활성 hover와 native pointer 강조, 키보드 초점, 좁은 창 paging, 접힘, 이미지 하단 툴바, 오른쪽/모서리 resize 우선권. 웹 별도 native frame도 오른쪽 끝 배경을 확인한다.
- Conf 1 / Risk Low. 빌드 성공은 실제 GUI 통과로 간주하지 않는다.

## 사용자 확인 및 병합 승인

2026-11-19 이 워크트리의 개발 앱을 실행했고 CLI에서 예상 인스턴스와 visible:true를 확인했다. 사용자가 화면 확인 후 병합을 승인했다. 이 사용자 확인은 지정 iMac의 Computer Use 시험 통과를 의미하지 않으며, 위 GUI 미실행 범위는 유지한다.
