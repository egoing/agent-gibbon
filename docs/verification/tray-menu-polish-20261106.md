# #76 트레이 목록·단축키 표시 검증 — 2026-11-06

> 이 문서는 전용 에이전트 기능 제거 전의 검증 이력이다. 현재 동작·최신 검사·출처는 [에이전트 기능 제거 기록](remove-agent-20261106.md)을 따른다.

작업 루트: `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/tray-menu-polish`  
브랜치: `tray-menu-polish`, 시작 커밋: `9f015a6b9198aae3c7765fdf53988d330aafc8c0`  
인스턴스: `dev-1fd207d59c82431a32969a4a`  
바이너리: 위 루트의 `src-tauri/target/debug/agent-gibbon`  
최종 실행 PID: **80881**, SHA-256: `63fda88ef436dd0ae470a4de936e75259c9a1efd9275183f8b81c345ee25820e`

커밋 전 로컬 변경의 검증이다. 배포·push·PR·전체 플랫폼 CI는 실행하지 않았다. 기준·정렬·키 표기의 정의는 [디자인 시스템](../design-system.md#최근-트레이-목록표시용-단축키--76-2026-11-06), 외형은 [SVG](../design/tray-memo-list.svg) · [PNG](../design/tray-memo-list.png)를 따른다.

## 구현 범위

- 네 유형을 같은 아이콘 목록에 통합하고 Tabler `terminal-2`를 터미널·에이전트에 적용했다. macOS 제목·아이콘은 실제 창 가시성에 따라 100%/45%, 숨김 항목도 선택 가능하다. PTY running과 visible은 별도다. 터미널 CLI close도 메뉴를 교체하지 않고 상태만 동기화한다.
- 표시 제목의 ID·유형 래퍼를 제거하고 빈 제목은 유형별 기본 제목으로 표시한다. 사용자 제목의 다른 괄호와 실제 저장·이벤트 ID는 유지한다. 트레이·관리 목록에 Unicode 30자 제한을 적용한다. 관리 창의 기존 첫 Markdown 블록 제목 정책은 유지한다.
- 최근 30개와 `더보기(남은 개수)`, 브랜치명 최상단·메모 관리 다음 줄을 구현했다. 더보기는 기존 관리 창이 설정에 있어도 모든 메모로 전환한다. 네이티브 창 제목과 CLI의 인스턴스 해시는 유지한다.
- 메모의 기존 갱신 시각과 영역 생성 ID의 시각, 터미널의 경로·선택적 제목 갱신 시각으로 정렬한다. 처음 실환경에서 발견한 capture `updated_at=0`도 생성 시각 보완값으로 처리하며 저장 데이터는 일괄 변경하지 않는다. 동률은 이벤트 ID 사전순이다.
- 표시용 단축키 변환만 추가했다. 캡처 등록 문자열·키 입력·CLI `stash.shortcut` 원시값을 변경하지 않는다. 트레이·전체 접어두기 말풍선·메모 복사/저장 Tooltip을 점검했다. 기존 stash/region/tray smoke의 화면 기대 문자열도 갱신했다.
- Notes·Terminals 잠금은 메뉴·창 API 전에 풀며 기존 교착 회귀 검사를 유지했다. 기존 메모 inspect/update는 열린 메뉴를 교체하지 않는다. 트레이 Enter 때 metadata를 다시 수집한다.

## 자동 검사

환경: macOS ARM64, Node **v24.21.0**, cargo **1.91.1**, rustc **1.91.1**, rsvg-convert **2.62.3**. 인계 메모의 Node 버전과 달라 현재 실행 도구 출력 기준으로 기록했다.

| 명령 | 실제 결과 |
|---|---|
| `npm test` | Rust **157/157**, UI **96/96** 통과 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | 통과 |
| `npm run check` | Clippy all-targets, `-D warnings` 통과 |
| `npm run dev:build` | 해당 워크트리 debug 빌드 통과 |
| `python3 -m py_compile ...` | 변경한 tray/region/stash smoke 스크립트 구문 검사 통과 |
| `git diff --check` | 통과 |
| `rsvg-convert -z 2 ...` | SVG→PNG 렌더링 후 육안 확인 |

[검사 요약](tray-menu-polish-20261106/validation.txt). 0/30/31개, 혼합 유형·동률 경계·입력 순서 반전, 본문 갱신 정렬, legacy 시각 fallback·저장값 불변, ID 표시 제거·기본 제목·Unicode 제한, 네 유형 아이콘 알파와 이벤트 라우팅, 두 OS 키 이름·availability 설명·원시 문자열, 기존 Notes 잠금 교착을 검사한다. 관리 UI는 31개 전체 목록·설정에서 목록으로 전환도 검사한다.

## 네이티브 실행

모든 CLI 호출은 이 루트의 `scripts/agent-gibbon-dev`를 사용했다. `dev:info`의 root/id/binary, 실제 프로세스 comm 절대 경로, inspect의 `ok/instance/id/markdown`을 대조했다. GUI에서는 PID에 속한 `Agent Gibbon [DEV tray-menu-polish · 1fd207d5] — tray76-proof-1fd207` 창과 고유 본문을 확인했다. 재빌드 뒤에는 자기 검증 앱만 종료하고 다시 실행하여 PID·출처를 재확인했다.

### 최종 빌드 — PID 80881

[최종 원시 증거](tray-menu-polish-20261106/final-report.json), [메뉴 캡처](tray-menu-polish-20261106/final-menu-hidden.png), [전체 관리 목록 AX](tray-menu-polish-20261106/manager-more.txt).

- 네이티브 메뉴의 실제 위치를 PID로 다시 조회했다(재시작 후 x 좌표 변경). 메뉴는 x=1160/y=25, 313×972 논리 px, 해당 PID의 CG layer 101 창으로 연결하고 그 범위만 캡처했다. 처음 재시작 전 좌표로 보낸 클릭은 검증으로 인정하지 않고 새 좌표로 다시 수행했다.
- 31개 저장 항목 중 네 유형을 합친 30개와 `더보기(1)`을 확인했다. 브랜치명·메모 관리 순서, 기본 `터미널` 제목, ID·유형 접두사 미노출, 숨긴 터미널·에이전트·캡처와 보이는 텍스트의 아이콘·제목 대비를 육안 확인했다.
- 직접 시작한 셸 **PID 81644**에 대해 visible=true/running=true → CLI close로 visible=false/running=true → 트레이 선택으로 visible=true/running=true와 **동일 PID**를 확인했다. 특히 열린 메뉴 중 close 뒤에도 해당 항목이 enabled로 남고 메뉴 선택이 동작했다.
- 설정을 선택한 실제 관리 창에서 더보기를 눌러 모든 메모로 돌아오고 31개 항목이 남는 것을 확인했다. 관리 창은 같은 PID의 네이티브 창 **40790**이다.
- 검증 셸은 정확한 PID·부모 PID(80881)·실행 파일(`/bin/zsh`)을 대조한 뒤 SIGHUP으로 정리하고 running=false·visible=false를 확인했다. GUI 도구의 종료 버튼 호출은 background_unavailable 또는 효과 미확인이라 성공으로 집계하지 않았다.

### 앞선 경계 실행 — PID 70625

`python3 scripts/tray-menu-polish-smoke.py` 통과. [실행 보고](tray-menu-polish-20261106/native-report.json), [열린 메뉴의 inspect/update 후 캡처](tray-menu-polish-20261106/menu-background-update.png), [31개 캡처](tray-menu-polish-20261106/menu31.png).

이 실행은 capture 생성 시각 fallback 보완 뒤, 최종 터미널 CLI close 동기화·빈 터미널 제목 기본값 보완 전 빌드다. 최종 빌드의 증거와 구분한다. 고유 검증 메모를 실제 CLI로 생성해 30개에 더보기 없음 → 31개에 30개+더보기(1), 기존 메모 본문만 갱신한 뒤 다음 포인터 진입·메뉴 열기에서 맨 위 정렬, 숨김 항목 복원, 동일 PID 관리 창 연결을 확인했다. 추가한 26개 검증 메모는 숨긴 상태로 남겼으며 다른 인스턴스를 정리하지 않았다. 이 스크립트의 정확한 30/31개 네이티브 경계는 시작 시 저장 항목이 30개 이하인 개발 인스턴스를 요구한다.

그보다 앞선 PID 54349에서는 실행 중인 에이전트도 hidden→visible에서 PTY PID 56566이 유지되는 것을 확인했다. 이후 재시작에서는 이 앱의 검증용 PTY만 종료됐다. 이 과거 실행을 최종 빌드 전체 동작의 증거로 대체하지 않는다.

## 한계·후속 검증

- **Windows 제목의 45% 알파·자동 테마 연동은 기존 미구현 범위다.** 이번 공통 아이콘 알파·클릭 가능 상태·제목 정리·개수·키 이름 변환은 구현했지만 Windows 실환경을 검증하지 않았다. 이 이슈의 모든 OS 제목 알파 요구까지 완료했다고 주장하지 않는다. macOS Intel·Windows 설치 실행·다크 모드 전환·키보드만으로 트레이 열기도 미검증이다.
- 0개는 순수 로직 검사로 검증했다. 실제 앱은 기본 main 메모를 가지므로 사용자 데이터를 지워 네이티브 0개를 만들지 않았다. 최종 빌드의 30개 경계는 자동 검사, 네이티브 31개는 최종 실행으로 검증했다.
- metadata 재수집은 트레이 포인터 진입에 연결했다([Tauri Enter API](https://docs.rs/tauri/2.12.5/tauri/tray/enum.TrayIconEvent.html#variant.Enter), 설치된 crate 소스 확인). 포인터를 아이콘에 둔 채 키보드만으로 재열기하는 경로의 최신 정렬은 보장·검증하지 않았다. Enter로 열린 메뉴를 재진입하는 특수 조작의 장시간 회귀도 별도다.
- 네이티브 검사에서 실제 키 입력 조합 자체와 stash 전환을 재실행하지 않았다. 입력 코드는 변경하지 않았으며 UI/단위 회귀와 CLI 원시값 보존을 확인했다. 다른 stash/region smoke는 기대 문자열과 안전한 PID/개발 CLI를 갱신하고 구문 검사를 했으며 전체 GUI 시나리오를 모두 재실행하지 않았다.
- 전체 플랫폼 workflow_dispatch는 커밋·push 승인이 없어 미실행이다. 승인 뒤 최신 main 통합·관련 검사·작업 브랜치 push 후 최신 SHA로 요청해야 한다. 공개 배포·자동 병합은 범위 밖이다.

검증한 macOS 경로 **Conf 5**, 실패 영향 **Risk Low**(로컬 표시·복구 가능한 선택 metadata 추가). 플랫폼 전체에 Conf 5를 적용하지 않는다. 커밋하지 않은 상태로 검토를 요청한다.
