# 관계선 연결 UI 최종 검토 자료 — 2026-11-15

## 검토 대상

사용자 요청의 ‘각 메모 툴바 연결 버튼 → 포인터를 따라가는 선 → 대상 롤오버 미리보기 → 대상 클릭 → 레이블/화살표 설정’을 구현한 로컬 작업 트리다. 레이블은 선택이며, 한쪽 화살표는 나중에 클릭한 대상을 향한다. 양쪽과 화살표 없음도 선택할 수 있다.

- 루트: `/Users/egoinglee/.codex/worktrees/67c9/agent-gibbon`.
- 브랜치: `codex/relation-overlay-prototype`.
- 통합 HEAD: `269eaea`(부모 `0eb4c12`, `f6a08ca`). 모니터 복구 main과 권한 메뉴 main이 모두 포함된다.
- 연결 UI와 추가 통합 방어 코드는 미커밋·미스테이징이다. 기존 [Draft PR169](https://github.com/egoing/agent-gibbon/pull/169)에 최신 로컬 UI가 모두 올라갔다고 보지 않는다. 제품 최종 커밋·push·병합은 승인 전이다.

## 사용자 동작과 현재 지원

| 항목 | 구현 상태 |
|---|---|
| 텍스트·이미지/캡처·터미널·웹 | 등록된 native host의 툴바와 양끝 resolver 지원. 웹 external child는 별도 대상/IPC caller가 아니다. |
| 툴바 버튼 | Tabler plug-connected, 기존12px 아이콘·24×22 슬롯·툴팁·overflow·키보드 접근 경로 사용. |
| 롤오버 | 화면별 비포커스 입력 표면에서 포인터 곡선과 대상 경계 미리보기. 호버만으로 저장하지 않음. |
| 클릭 | 클릭 순간 geometry를 다시 판정. 자기 자신·겹쳐 모호한 대상 거부. 설정창에서 최종 저장. |
| 설정 | 빈 레이블 허용. end=클릭 대상, both=양쪽, none=화살표 없음. 기존 JSON은 end 기본값. |
| 재연결 | 같은 두 메모의 기존 관계를 양끝 순서와 무관하게 갱신. 다른 관계 보존. |
| 취소·상태 변화 | 출발 메모 Esc/화면 취소·설정 취소·native 설정창 닫기. 대상/출발 숨김·닫힘·stash 진입 정리. |
| 오래된 응답 | picker 고유 세대와 설정 저장·취소/출발 Esc 세대를 대조. 현재 연결 이외 요청 거부. |
| 표시 정책 | 평소 메모 하나라도 포커스일 때 전체 선 표시. 이동/숨김 중 관련 선 정지·150ms 안정화 후 재표시 유지. |
| 저장 | UI 저장은 인스턴스 Store/.relations.json과 live 관계 JSON을 각각 원자 교체. startup은 live를 보존하고 없으면 저장본으로 복구. 두 파일을 하나의 트랜잭션으로 묶지는 않음. |
| 제한 | 개발·설치·release에서 기본 활성화. 연결용 환경변수·debug/instance gate 없음.16관계/16KiB/레이블80자. 레이어·부모 병합·연결 삭제/편집 목록 등의 범용 도구는 추가하지 않음. |

외부 webpage-*의 앱 IPC 거부는 main handler에서 계속 적용한다. 새 picker capability는 로컬 preview 이벤트 수신만 허용한다. 권한 메뉴 완료 marker와 관계 저장 파일은 별도이며, 연결 모드는 화면 캡처 begin을 호출하지 않아 캡처 권한 메뉴 재노출을 유발하지 않는다.

## 변경 파일 묶음

| 묶음 | 주요 파일 |
|---|---|
| 연결 입력·저장·표시 | src-tauri/src/relation_editor.rs(신규), relation_prototype.rs, main.rs, stash.rs, terminal.rs, web.rs |
| 모니터 통합 방어 | src-tauri/src/monitor_layout.rs의 보조 창 manual/native-event 제외와 회귀 검사 |
| 권한 경계 | src-tauri/capabilities/relation-picker.json(신규), main의 기존 external child 차단 보존 |
| 툴바·미리보기·설정 | ui/relation-toolbar.js, relation-pick.html/css/js, relation-settings.html/css/js(신규); index.html·terminal.html·web.html 로드; relation-prototype.html/js의 arrow 설정 |
| 라이선스 | ui/licenses/tabler-icons.txt(신규) |
| 검사 | tests/relation-editor.test.mjs(신규), relation-render.test.mjs, Rust 내부 target/upsert/monitor 분류 검사 |
| 문서·시안 | docs/design-system.md, docs/design/relation-editor.svg/png(신규), 기존 relation-prototype.svg/png, 연결 작업 verification 문서 |

실제 파일별 해시와 git 상태는 원본 백업과 후속 최종 manifest를 대조한다. 무관한 untracked package-lock.json은 원본 그대로 보존하며 제품 커밋 후보가 아니다.

## 검증 근거와 남은 한계

57231 모니터 main 통합 상태에서 Rust258·UI196 전체 검사, Clippy·Rustfmt·개발 빌드를 통과했다. f6a08ca 권한 메뉴 증분은 해당 버전의 별도 대상 검사·컴파일 결과로 기록하며 이전 전체258/196을 새 버전 전체 검사로 재표기하지 않는다. 새 결과는 [구현 검증 기록](relation-editor-20261115.md)의 권한 메뉴 통합 후속을 따른다.

설정폼의 실제 HTML/CSS/JS를 독립 headless Chrome/CDP380×330 밝음/어둠으로 렌더해 입력 포커스와 버튼의 viewport 수용을 확인했다. 곡선·hover 테두리도 실제 SVG로 렌더했다. IPC는 mock이므로 native hit-test나 사용자 연결 완료의 증거가 아니다. [시안 SVG](../design/relation-editor.svg)·[PNG](../design/relation-editor.png)도 설명용이다.

이전에 적용한 개발 앱 PID81735의 시작 기록에서는 기존12관계가 바이트까지 동일하고 저장된44프레임이 inspect와 일치했다. 이 결과는 그 당시 구빌드에 한정된다. 이번 두 main 통합에서는 사용자 앱·PTY·기하·관계를 조작하거나 자동 재시작하지 않았다. 최신 소스와 새 binary가 현재 실행 앱에 반영됐다고 보지 않는다.

남은 native 확인은 버튼 실제 표시/첫 클릭, 비포커스 picker와 source Esc, 다중 화면 포인터/대상 경계, 설정 저장 후 방향·포커스, 실제 재시작 저장 복구, 대상 닫힘·숨김과 전체 숨김의 사용자 흐름이다. Windows·Intel·혼합 DPI도 미검증이다. CUA의 debug앱 선택 제한을 다른 입력 기술로 우회하지 않았다. 최신 앱 반영/사용자 클릭 검증은 별도 차례가 필요하다.

Conf3 / RiskMedium: 자동·headless 검증을 마쳤으나 포인터 입력 표면·로컬 관계 저장·포커스의 전체 native 흐름은 확인이 남아 있다.

[디자인 시스템](../design-system.md) · [세부 구현·검증](relation-editor-20261115.md) · [종류별 예제](relation-mixed-demo-20261115.md).


사용자 후속 요청으로 제품 기본 기능 전환을 준비 중이다. 환경변수 및 개발 전용 활성화 gate를 제거한 최신 소스는 Rust259·UI197·Clippy와 macOS release 앱 번들 빌드를 통과했다. 이전 통합 검증과 구분하며 아직 설치 앱 교체·main 병합·공식 공개는 수행하지 않았다. 실제 배포 대상 확인이 필요하다. 후속3항목은 #198/#199/#200에 기록됐고 이번 구현에 포함되지 않는다.
