# 본문·Feedback 통합 활성 외곽 — 2026-11-13

최종 시각 수치는 문서 끝의 **후속 시각 조정 — 2px·안쪽 라운드·구분선 약화**를 따른다. 아래 3px 기록은 그 이전 통합 구조 검증이다.

## 원인과 구조 변경

초기 구현은 본문 내부 `#memo-surface::after`와 별도 Feedback 창의 기존 1px 경계가 각각 그려지는 구조였다. 표면 내부 좌표와 고정 라운드 클립 때문에 선이 창 가장자리에서 떨어져 보이고, Feedback 연결 면에도 본문의 하단 라운드·테두리가 남았다.

- 공유 `ui/memo-frame.css`의 **viewport `body::after`**가 외곽을 소유한다. `position:fixed; inset:0; box-sizing:border-box`의 3px border를 안쪽으로 그린다. 본문·Feedback 콘텐츠의 기존 공간과 창 geometry에는 참여하지 않는다.
- 아래 부착 시 본문은 위·좌·우 선, Feedback은 좌·우·아래 선을 그린다. 위 부착 시 반대로 그린다. 연결 면의 두 모서리 반경은 0이고, 전체 최외곽만 4px 라운드다. 기존 SVG 외곽선 DOM·경로 계산을 제거했다.
- Feedback 자체의 상시 경계는 투명하게 유지하고, 연결부에는 좌우 3px 여백의 기존 1px 구분선 하나만 남긴다. 편집 입력칸 자체의 내부 경계·버튼 키보드 포커스 표시는 유지한다.
- 본문 또는 **표시 중인 Feedback 중 하나의 native 포커스**를 공유한다. Rust `memo_frame_focus`가 두 창의 상태를 읽고, native focus·Feedback 표시/숨김 시 두 창으로 같은 결과를 보낸다. 숨긴 Feedback의 focus는 무시한다. JS는 이벤트 구독 이후 초기 상태를 읽고, 더 최신 이벤트가 오면 초기 응답을 버린다. 기존 `allow-is-focused` 프런트엔드 권한은 더 이상 필요 없어 제거했다.

[디자인 기준](../design-system.md) · [네 조합 시안 SVG](../design/memo-focus.svg) · [PNG](../design/memo-focus.png) · [위/아래 부착 시안](../design/feedback-drag.png)

## 실행 출처

- 작업 루트: `/Users/egoinglee/.codex/worktrees/02ec/agent-gibbon`
- `npm run dev:info`의 instance: `dev-f3ab8dc6a8d44e989b85031f`
- 실행 파일: `/Users/egoinglee/.codex/worktrees/02ec/agent-gibbon/src-tauri/target/debug/agent-gibbon`
- macOS ARM64, Node v26.0.0, Rust 1.91.1. 모든 메모 명령은 해당 워크트리 `scripts/agent-gibbon-dev`로 실행했다.
- 네 조합 검증: **PID77792**, native window **67303**, 제목 `Agent Gibbon [DEV detached@f343dbe · f3ab8dc6] — frame-02ec-proof`. `inspect`에서 `ok:true`, instance·ID 및 고유 본문 `FRAME-02EC-20261113 외곽 통합 검증`을 확인했다. Feedback native window **67313**은 응답의 `feedback_surface.window_id`와 CGWindow PID/geometry로 연결했다.
- 최종 불필요 SVG 제거 후 `npm run dev:build` 성공 → 직접 시작한 PID77792만 종료 → **PID83253**으로 재시작했다. 기존 proof의 `inspect`와 native window **67341** 제목·고유 본문을 다시 대조하고 최종 캡처했다. [최종 빌드 출처](assets/memo-frame-20261113/final-build.json).
- 설치 앱과 다른 워크트리의 프로세스는 종료하지 않았다. 같은 워크트리의 동시 Ctrl 입력 변경과 빌드/GUI 사용 순서를 조율하고 해당 변경을 보존했다.

## 실제 네 조합

| 상태 | 본문 native focused | Feedback 표시 | 본문 물리 좌표·크기 | 확인 |
|---|---:|---:|---|---|
| Feedback 없음 · 비활성 | false | false | `(2200,600)`, `720×320` | 외곽선 없음 |
| Feedback 없음 · 활성 | true | false | 동일 | 최외곽 3px 안쪽 선 |
| Feedback 있음 · 활성 | true | true | 동일 | 본문·바의 연속된 외곽, 연결 면의 선·라운드 없음 |
| Feedback 있음 · 비활성 | false | true | 동일 | 본문과 바 모두 외곽선 없음 |

- Feedback 물리 크기는 `720×58`, 위치는 `(2200,920)`으로 본문 하단과 간격 0이다. 포커스 전환 전후 두 창의 크기·위치가 같다.
- 추가로 Feedback 저장 버튼 위 mouse-down 후 바깥 mouse-up으로 동작을 취소해 **본문 focused:false / Feedback focused:true**를 확인했다. 이 상태에서도 두 창의 공유 외곽이 활성으로 유지된다. 저장·취소로 본문을 바꾸지 않았다.
- 실제 native 마우스 입력으로 활성화하고 CLI 상태를 읽은 뒤 PID/창 ID를 지정한 Orca ScreenCaptureKit 캡처를 받았다. `get-app-state`의 background 클릭만으로 활성화를 가정하지 않았다.
- ScreenCaptureKit의 부모 캡처는 연결된 자식 창도 포함해 요청 이미지 크기에 맞춰 보여 준다. 따라서 캡처의 합성 이미지 크기를 네이티브 창 geometry로 해석하지 않았다. geometry는 위 CLI/CGWindow 값으로 검증했다. Feedback 창 ID 직접 `get-app-state`는 `window_not_found`여서 부모·자식이 함께 보이는 부모 캡처를 사용했다.

[없음·비활성](assets/memo-frame-20261113/plain-inactive-memo.png) · [없음·활성](assets/memo-frame-20261113/plain-active-memo.png) · [있음·활성](assets/memo-frame-20261113/joined-active-memo.png) · [있음·비활성](assets/memo-frame-20261113/joined-inactive-memo.png) · [Feedback 단독 포커스](assets/memo-frame-20261113/joined-feedback-focus-memo.png)

각 캡처 옆 같은 이름의 JSON에 PID·창 제목·CLI instance·geometry·포커스 상태를 보관했다. [최종 재시작 후 복원된 기존 proof](assets/memo-frame-20261113/final-build-restored-proof.png)도 확인했다.

## 자동 검증과 상태 보존

- 개발 빌드 성공, UI **185/185**, Rust **215/215**, Clippy `-D warnings` 통과. Rustfmt·JS 구문·`git diff --check`도 통과했다.
- 회귀 검사는 공유 native 포커스(숨긴 바의 stale focus 제외), 초기 비동기 응답 역전 방지, 자기 메모의 연결 면만 변경하고 숨길 때 복원하는 동작을 포함한다. 기존 SVG 경로를 기대하던 검사는 현재 연결 상태 규칙으로 갱신했다.
- 시안 SVG 두 개를 `rsvg-convert`로 PNG 렌더링하고 직접 열어 확인했다.
- 재시작 전에 열려 있던 main/proof/progress/ctrl-only 메모·관리 창의 geometry와 두 편집 초안을 임시 AX 스냅샷으로 보존했다. 최종 복원 후 **모든 대상의 위치·크기와 두 초안 문자열이 최초 값과 동일함을 assert**로 확인했다. 사용자 초안은 저장소/검증 자료에 기록하지 않았다.
- 새 `frame-02ec-proof`만 숨긴 상태다. 사용자가 편집 중이던 진행 메모의 본문·초안은 덮어쓰지 않고 `status` 메타데이터에 후속 검증 완료 결과를 남겼다. 기존 창 표시 상태와 이전 앱 포커스를 복원했다. 검증 GUI 해제 후 동시 입력 작업 세션에 순서를 넘겼다.

## 한계

실화면 검증은 macOS ARM64 텍스트 메모·편집 Feedback·현재 검정 테마다. 위 부착·접힘·이미지·터미널·다른 테마·Windows·Intel 실화면은 별도 미검증이다. 공유 CSS의 위/아래 규칙과 디자인 시안은 갱신했지만 미실행 플랫폼이나 상태를 통과로 간주하지 않는다. 전체 플랫폼 빌드·설치·배포 및 커밋·푸시·병합은 하지 않았다.

**Conf 5**(기재한 macOS 실앱 범위), **Risk Low**.


## 후속 시각 조정 — 2px·안쪽 라운드·구분선 약화

- 사용자 추가 요청에 따라 `--memo-focus-width`를 3px에서 **2px**로 줄였다. 바깥 반경 4px에서 안쪽 반경이 **2px**로 남아 테두리 안팎이 모두 둥글다. 연결 면의 반경 0과 통합 외곽 구조는 유지한다.
- Feedback 구분선의 불투명도를 **50% → 25%**로 낮췄다. 오류 구분선에도 같은 불투명도가 적용된다. 시안 세 개의 SVG·PNG와 디자인 기준을 함께 갱신하고 렌더링을 확인했다.
- UI **185/185**와 개발 빌드 통과. 변경은 CSS 토큰과 구분선 스타일에 국한되어 Rust 검사는 추가 실행하지 않았다.
- 최종 재시작 PID **92340**, 기존 전용 proof `focus-02ec-proof`, native window **67415**와 제목·본문·CLI instance `dev-f3ab8dc6a8d44e989b85031f`를 대조했다. `focused:true`, 크기 `600×114`에서 [2px 실앱 캡처](assets/memo-frame-20261113/refine-2px.png)를 확인했다. Feedback이 위에 연결된 상태이며 안팎의 둥근 모서리와 흐려진 구분선을 확인했다.
- 이번 재시작 직전 상태를 새로 보존했다. 기존 메모·관리 창의 위치·크기, 접힌 Ctrl 검증 메모, 편집 중이던 **세 초안**을 복원하고 최초 AX 값과 일치함을 확인했다. 이전 앱 포커스를 복원했다. 커밋·배포하지 않았다.


## main 통합 전 최종 검증

- 입력 제스처 PR #146·호버 격리 PR #147이 병합된 `origin/main` (`acda6685121b6ede5f805ee5cddb67536d066c55`)을 기준으로 포커스 변경만 스테이징했다.
- 공유 작업 트리의 overlay scrollbar 구현·제안·검증 자료와 lockfile 변경은 제외했다. 공용 `ui/index.html`과 디자인 시스템 문서는 index blob을 부분 생성하여 해당 변경을 working tree에 보존했다.
- 스테이징된 전체 파일을 임시 스냅샷으로 추출한 뒤 UI **186/186** 통과. Rust 소스는 index와 working tree가 동일한 상태에서 **216/216**, Clippy `-D warnings`, Rustfmt 통과. staged diff 공백 검사도 통과했다.
- 앞서 기록한 개발 빌드·실앱 증거를 유지하고, 커밋 준비 과정에서 앱을 재시작하거나 창·초안을 조작하지 않았다. 전체 플랫폼 빌드·설치·배포는 실행하지 않았다.
