# 관계선의 메모 포커스 조건과 비텍스트 지원 조사 — #168

## 변경

같은 개발 인스턴스에 등록된 텍스트·이미지·터미널·웹 메모 중 하나라도 실제 native 포커스를 가지면 기존 관계선을 표시한다. 포커스된 메모에 연결된 선으로 한정하지 않는다. 관리창·트레이·관계선·캡처 선택창·외부 웹 child label은 독립 메모로 집계하지 않는다. 웹 child의 포커스는 등록된 native host의 포커스로 판정한다.

macOS는 전면 앱 PID도 대조한다. 비활성 앱이 key window를 기억하더라도 다른 앱에서 선이 보이지 않게 한다. Focused 이벤트는 worker를 깨우고 coalesced main-thread에서 전체 메모의 현재 포커스를 다시 조회한다. A의 blur 이벤트만으로 즉시 숨기지 않아 A→B 이벤트 순서에 따른 불필요한 표시 취소를 피한다. 250ms fallback도 유지한다.

포커스가 없으면 pending 표시 세대를 무효화하고 선을 숨기며 좌표 계산을 건너뛴다. 비동기 `relation_ready`에서도 포커스를 다시 검사한다. tick이 보지 못한 짧은 blur 중 표시 확인이 거절된 경우도 해당 pending을 무효화해 다음 포커스에서 다시 그릴 수 있게 한다. 이동·resize·stash 정지 정책과 overlay 클릭통과·비포커스 설정은 유지한다.

[디자인 시스템](../design-system.md) · [시안 SVG](../design/relation-prototype.svg) · [PNG](../design/relation-prototype.png). macOS 크기→위치 적용 순서의 직전 미커밋 복구도 보존했다.

## 연결 타입 지원표

| 연결 양끝 | 현재 구현 | 이번 확인 범위 |
|---|---|---|
| 텍스트 ↔ 텍스트 | 지원 | 기존 6개 메모·7개 관계 설정 보존. 직전 실제 화면 7개 곡선 확인과 이번 포커스 변경을 구분한다. |
| 텍스트 ↔ 이미지·캡처 | 구조상 지원 | 실제 이미지 메타데이터와 PNG를 가진 전용 메모를 생성·inspect해 AppNotes 로드를 확인. 이번 화면상의 연결은 직접 클릭 확인 대기. |
| 이미지 ↔ 이미지 | 구조상 지원 | 동일 AppNotes/geometry 경로. 이번 두 이미지 실화면 검증은 미실행. |
| 텍스트·이미지 ↔ 터미널 | 미지원 | relation geometry가 AppNotes의 양끝 ID를 요구한다. 터미널은 별도 Terminals이며 native label도 terminal-ID다. 전용 터미널 창 생성·inspect 확인, 연결 실화면 시험은 미실행. |
| 텍스트·이미지 ↔ 웹 | 미지원 | 웹은 별도 Windows 저장소와 webview-ID host, webpage-ID 외부 child를 사용한다. AppNotes/기존 note-ID resolver에 포함되지 않는다. 전용 Welcome 창 생성·inspect 확인, 연결 실화면 시험은 미실행. |
| 터미널 ↔ 웹·터미널 / 웹 ↔ 웹 | 미지원 | 동일한 양끝 등록 검사와 label 불일치. 이번 연결 실화면 시험은 미실행. |

CLI `list`에는 터미널·웹도 나오지만 그것만으로 관계 연결 지원을 뜻하지 않는다. 웹의 `managed_windows`에는 external child를 소유하는 native host의 geometry 조회 경로가 이미 있다. 최소 후속 범위는 관계 endpoint resolver를 각 등록 저장소에 맞추고, web/terminal의 이동·닫힘·숨김 이벤트에서 같은 관계 무효화 경로를 호출하도록 확장하는 것이다. 공개 API·저장구조 전체 개편은 필요하지 않으며 이번 확인 요청에서 지원을 임의 추가하지 않았다.

## 실행 출처와 검증

- 작업 루트: `/Users/egoinglee/.codex/worktrees/67c9/agent-gibbon`.
- instance: `dev-f3d1fc901f372efdf5d35b62`.
- 실제 바이너리: 위 루트의 `src-tauri/target/debug/agent-gibbon`, PID78429, `AGENT_GIBBON_RELATION_PROTOTYPE=1`을 지정한 개발 CLI `--serve`로 실행.
- 전용 메모: `relation-focus-20261115`, `image-202611150011`, `terminal-202611150011`, `web-relation-20261115`. 각 inspect의 ok/instance/visible/type을 대조했다. 이미지·터미널 fixture는 앱 종료 중 신규 검증 파일만 작성한 저장 fixture이며, 캡처 UI나 터미널 생성 UI를 통과했다는 증거는 아니다. 터미널 shell은 시작하지 않았다.
- 기존 7개 관계에 시험용 이미지↔텍스트 관계 하나만 추가했다. 원본 JSON과 재시작 직전 모든 창 inspect는 `/tmp/gibbon168-motion/focus-original-relations.json`, `focus-before.json`에 임시 보존했다. 기존 메모의 과거 위치를 강제로 복원하지 않았다.
- 현재 전용 텍스트·이미지·웹 inspect가 모두 focused=false이고 동일 PID의 native 관계선 표시 수가 0임을 확인했다. 외부 웹 페이지 본문/터미널 본문·툴바의 포커스 검증과 구분한다.
- Rust 관계 단위 7개, 관계 렌더 UI 3개, Clippy all-targets `-D warnings`, 개발 빌드 통과. Rustfmt와 diff 공백 검사 통과. 원시 로그는 `/tmp/gibbon168-motion/focus-{rust,clippy,build}.log`의 임시 파일이다.
- CUA가 debug 바이너리 절대 경로 및 실행파일 이름을 `Invalid app`으로 거부했다. 설치 앱이나 다른 OS 입력기로 대체하지 않았다. 따라서 자동 실제 클릭·물리 드래그·A→B 빠른 전환·관리창 전환·전체 숨김 반복·웹 external child/터미널 본문 포커스 검증은 미실행이다.
- 사용자에게 검증 텍스트/이미지 클릭 후 다른 앱 전환을 요청했다. 이 직접 확인 전에는 표시 복귀·깜박임 부재·클릭통과 전체를 실환경 통과로 기록하지 않는다. 검증용 창과 추가 관계는 이 확인을 위해 유지 중이다.

Conf3(단위·정적·빌드), 무포커스 native 숨김만 실제 환경 확인. RiskLow(개발 전용 파생 표시 조건). Windows·Intel 실환경은 미검증이며 제품 커밋·push·병합은 하지 않았다.

## 최신 main 병합 — 2026-11-15

사용자 지시에 따라 `origin/main`의 `307f7fd`를 현재 관계선 브랜치에 병합했다. 병합 커밋은 `e30b7fe`다. 디자인 시스템의 양쪽 추가 내용을 보존했고, stash 대상 제외는 main의 방향 안내창·layout-undo와 관계선 파생창을 모두 제외하도록 합쳤다. 관련 제외 회귀에 두 창을 추가했다.

병합 전 미커밋 관계선 작업은 임시 보관 후 복원했으며 이 작업들은 여전히 미커밋이다. 기존 package-lock.json 원본은 현재 untracked 파일로 보존했다. 이전 untracked pnpm-lock.yaml은 최신 main의 Milkdown 의존성이 포함된 잠금 파일과 달라 덮어쓰지 않았고, 원본을 `/tmp/gibbon-main-merge-20261115-183639/before-pnpm-lock.yaml`에 보존했다. 전체 원본 tar·patch와 안전용 stash도 유지했다.

검사 대상은 병합 커밋에 기존 미커밋 작업을 복원한 현재 작업 트리다. `pnpm install --frozen-lockfile`, Rust 전체 242개·UI 전체 192개, Clippy all-targets `-D warnings`, Rustfmt, 개발 빌드가 통과했다. 원시 로그는 위 임시 백업 디렉터리에 있다. 실행 앱은 재시작하지 않았으며 새 병합 상태의 실제 GUI·Windows·Intel 검증은 하지 않았다. 최신 전체 숨김 진입은 main의 방향 선택 방식으로 바뀌었으므로 앞선 단축키 설명은 과거 검증 조건으로 해석해야 한다. 병합 후 main으로 역병합하거나 push하지 않았다.
