# 메모 자동 배치의 오른쪽 하단 시작 — 2026-11-06

## 변경 기준

새 일반 메모·숨김 복원의 시작점을 화면 작업 영역의 오른쪽 하단으로 옮겼다. 열기 순서대로 위로 쌓고, 공간이 차면 왼쪽 열로 이어진다. 오른쪽 열 정렬·열기 순서·높이 상한·수동 배치 제외·420ms 전환은 유지한다. 화면이 가득 찼을 때는 최소 겹침 후보 중 오른쪽 하단부터 확인한다.

[디자인 시스템](../design-system.md#메모-자동-배치--확정-2026-11-04), [시안 SVG](../design/placement-zero-gap.svg), [시안 PNG](../design/placement-zero-gap.png), README와 앱에 포함하는 사용 스킬을 함께 갱신했다. 시안은 설명용이며 실행 화면과 구분한다.

## 작업 출처

- 사용자 요청으로 Orca 전용 워크스페이스에 이전했다. 표시 이름은 `bottom-right-memo-placement`, 브랜치는 `egoing/bottom-right-memo-placement`다. 생성 후 방향 정정으로 표시 이름·브랜치를 바꿨으며 실제 디렉터리는 생성 당시 경로를 유지한다.
- 작업 루트: `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/bottom-left-memo-placement`.
- 기준 커밋: `30a39e9354b0c7298048d7e9b98af65b29c92012` (`origin/main`). 이전 패치의 stable patch-id 일치를 확인하고 원래 `/Users/egoinglee/dev/project/agent-gibbon`의 이번 변경은 제거했다. 원래 작업 폴더의 소스 변경 없음도 확인했다. 마지막에 원래 폴더의 개발 바이너리도 원래 소스 기준으로 재빌드했으며 앱을 재시작하지 않았다.
- `npm run dev:info`: root는 위 경로, instance는 `dev-dab6365756cd9981d7d0ff67`, binary는 위 루트의 `src-tauri/target/debug/agent-gibbon`.
- 환경: macOS ARM64, Node v24.21.0, cargo/rustc 1.91.1, tauri-cli 2.12.1, rsvg-convert 2.62.3.

## 로컬 검사

| 실행 | 결과 |
|---|---|
| `npm test` | Rust 156개·UI 95개 통과 |
| `npm run dev:build` | 워크스페이스의 debug 바이너리 빌드 성공 |
| `npm run check` | Clippy all-targets, warnings-as-errors 통과 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | 통과 |
| SVG → PNG 렌더링 | rsvg-convert로 생성 후 위로 쌓임·왼쪽 열 전환·문구·잘림 육안 확인 |

순수 배치 검사에는 음수 화면 원점, 장애물 아래·위 공간 탐색, 0px 접합, 짧은 메모의 높이 제한, 정수 나머지 배분, 고정 비율 이미지·접힌 메모, 공간 부족 및 최소 겹침 우선을 포함한다. 기존 열기 순서 실환경 스크립트의 비교 방향도 오른쪽 열부터 위로 진행하도록 갱신했다.

## 실제 실행 검증

`python3 /tmp/agent-gibbon-bottom-right-smoke.py`로 이 워크스페이스의 개발 CLI만 호출했다. 최종 실행 결과는 [출처·창 식별·실제 프레임 JSON](bottom-right-placement-live.json)에 정리했다. 전체 임시 원본 기록은 `/tmp/agent-gibbon-bottom-right-runtime/live.json`이다.

- 최종 실행 PID는 **58138**이다. `ps -p 58138 -o comm=`의 실행 파일 경로를 위 바이너리와 대조하고, `inspect`의 `ok: true`·예상 instance·메모 ID와 고유 본문 `right-bottom-9a1f79`를 확인했다.
- GUI 캡처 전에 같은 PID의 CGWindow 목록에서 `Agent Gibbon [DEV egoing/bottom-right-memo-placement · dab63657] — <메모 ID>` 제목을 정확히 찾았다. 해당 window-id로 `orca computer get-app-state`를 호출하고 반환된 PID·제목·본문 표식을 대조했다. 세 캡처의 window-id·CLI 실제 위치·크기는 JSON의 captures에 남겼다.
- 짧은 첫 메모 `right-bottom-9a1f79-origin`의 native 오른쪽·아래 경계가 OS 작업 영역과 일치했다. 실제 물리 위치는 JSON을 따르며, 300 논리 px 너비와 콘텐츠 높이만 사용함을 확인했다. [첫 메모 화면](bottom-right-placement-origin.png).
- 긴 일반 메모 6개를 ID 역순으로 열어 오른쪽 하단→위→왼쪽 열 순서, 가로·세로 0px 간격, 최종 겹침 없음, 높이 상한과 포커스 비활성 상태를 확인했다. [첫 메모 화면](bottom-right-placement-six-first.png). 개별 캡처는 메모 표면만 보여 주며 전체 배치는 JSON의 sequential 프레임 대조가 근거다.
- 이미 보이는 메모의 다시 열기·내용 갱신에서 모든 기존 위치·크기를 유지했다. 숨김 갱신의 위치·크기·숨김 보존과 명시적 복원 시 마지막 순서 합류를 확인했다. 이어 두 메모를 빠르게 열어 최종 순서·겹침 없음을 확인했다. [연속 열기 마지막 메모](bottom-right-placement-rapid-last.png).
- 전체 실환경 검사 **5개 통과**. 전후 frontmost PID는 모두 **54349**였고 메모 자체는 포커스를 받지 않았다. 테스트 메모는 모두 숨겼고 포인터를 복원했다. 다른 워크스페이스의 앱은 종료·재시작·수정하지 않았다.

### 재시도와 구분

오른쪽 하단 첫 시도에서는 검증 스크립트가 포인터를 2px만 이동해 앱의 기존 8px 이동 임계값을 넘지 못했고, 자동 이동 완료 대기에서 실패했다. 앱 로직을 바꾸지 않고 검증 입력만 40px로 고친 뒤 위 5개 검사가 통과했다. 첫 실패 기록은 `/tmp/agent-gibbon-bottom-right-runtime/attempt1-threshold-too-small.json`에 보존했다.

방향 정정 전 왼쪽 하단 임시 빌드의 기존 `open-order-smoke.py` 실행도 포커스 동일성 검사에서 중단됐으며, 최종 동작의 통과 근거로 사용하지 않았다. 최종 오른쪽 하단 검증에서는 해당 12창·수동 드래그 스크립트 전체를 재실행하지 않았다. 수동 장애물·이미지 비율·접힘·공간 부족 분기는 순수 배치 단위 검사로만 확인했다.

## 한계

Windows·Intel·서로 다른 배율의 복수 모니터 GUI 검증은 아직 실행하지 않았다. 로컬 검사는 해당 플랫폼 전체 CI를 대신하지 않는다. 이 워크스페이스의 프로젝트 지침상 창 제어 변경은 커밋·push 후 전체 플랫폼 수동 검사가 필요하며, 사용자 커밋 승인 전이므로 아직 실행하지 않았다. 태그·공개 배포는 수행하지 않았다.

Conf 5 (macOS ARM64 실제 개발 인스턴스의 핵심 배치·복원 확인), Risk Low (기본 배치 방향의 국소적이며 되돌릴 수 있는 변경).

## 통합 진행 — 사용자 승인 후

사용자가 이 워크스페이스의 커밋·병합·push·이슈 기록·종료를 승인했다. 배치 변경과 스포트라이트 보정을 별도 커밋으로 보존한 뒤 같은 작업 브랜치에서 전체 플랫폼 검사를 요청한다. 앞의 승인 대기 설명은 최초 로컬 검증 시점의 상태이며, 원격 검사 결과와 대상 SHA는 후속 통합 기록으로 구분한다.
