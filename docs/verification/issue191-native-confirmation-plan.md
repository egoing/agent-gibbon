# #191 native 검증 경로 재조사·수동 확인 준비안

2026-11-15. 제품 변경·앱 재실행·사용자 새 실험 요청 없이 코드와 기존 자체 fixture 기록만 재검토했다. 기존 자동 계산 검사를 반복하지 않았다. 이 문서는 실행 준비안이며 실행 결과가 아니다.

## 결론과 진입 경로

**현재 제품에는 단축키 lease를 소유하지 않은 인스턴스에서 두 정렬을 실행할 지원 경로가 없다. 자기 트레이를 사용자가 식별해도 이 제한은 해결되지 않는다.**

| 기존 경로 | 코드 근거 | 판정 |
|---|---|---|
| 세 보조키 해제 → 포인터 오른쪽/위 | `stash::modifier_event`에서 `input_owned` 요구 → `Gesture.origin` 설정 → `sample_pointer` → `arrange_direction` | 두 정렬의 유일한 사용자 진입. 전역 소유권 없는 자기 인스턴스에서는 사용 불가 |
| 트레이 `숨김` (`stash-toggle`) | `tray.rs` → `stash::trigger` → `toggle` | 자기 앱의 숨김/복원만 실행. 방향 안내나 정렬을 시작하지 않음 |
| 메모/웹/터미널 툴바 | 이동·접기·작업 영역 확대·닫기 등의 기존 command | 명시 전체 정렬 항목 없음. 일반 열기 배치와 대체 불가 |
| 로컬 CLI·Unix socket Request | `Action`은 Open/Update/Close/Inspect/List/Diagnostics/Probe/Web | 정렬 action 없음. 임의 JSON action도 enum 역직렬화로 불가 |
| 기존 Tauri invoke | `undo_arrangement`만 등록. `arrange`는 command로 등록되지 않음 | 정렬 실행 불가. undo도 `layout-undo` 창 전용 |
| 숨김 복원 후 예약 정렬 | `restored_arrangement` | 이미 유효한 오른쪽/위 제스처에서 예약한 후속 작업일 뿐 새 진입점 아님 |

더구나 `stash::install`은 lease 획득과 입력 감지에 성공한 `owned`일 때만 `direction-guide`와 `layout-undo`를 만든다. 기존 fixture의 `Ctrl+Alt+Cmd 사용 중 · 트레이로 실행`은 숨김/복원의 대체 경로를 뜻하며 두 정렬의 대체 경로가 아니다. 비소유 인스턴스는 정렬 취소창도 없으므로 내부 arrange를 임의 호출하는 우회도 정상 제품 검증이 될 수 없다.

## 자체 캡처 fixture 상태를 분리한 재분석

출처: 기존 PID32257/instance `dev-ef9fadde855c7e2c93cbc288`에서 저장한 [CLI 원본](assets/issue191/cli-fixtures.json), 이번에 그 값을 분리한 [분석 JSON](assets/issue191/fixture-state-analysis.json). 새 앱 실행·실제 제스처는 없었다.

| 확인 관점 | 자체 fixture에서 관측한 값 | 해석·한계 |
|---|---|---|
| ID·유형 | `arrange191-image`, `note.capture={600×400}`, native label은 코드 매핑상 `note-arrange191-image` | AppNotes 이미지 fixture다. 실제 영역 캡처 생성 과정을 실행한 것이 아니며 native 제목은 CUA로 대조하지 못함 |
| 표시·전체 숨김 | `visible=true`, `stash.active=false` | 일반 표시 상태. 최소화 여부는 inspect에 없어 직접 관측하지 못함 |
| 수동 제외 | `auto_layout_excluded=false` | 이 snapshot은 제외창 재현이 아니다. 캡처 생성의 place는 코드상 manual_resize로 제외 표시하지만 이후 CLI open에서 재포함될 수 있으므로 최초 생성 상태와 혼동하지 않음 |
| entering/애니메이션 | `animating=true` | entrance가 활성. inspect에는 presenting과 transition을 구분하는 필드가 없어 둘 중 무엇인지 이 값 하나로 단정 불가 |
| 측정 예약 | `layout_pending=false`, content cap 48 | 측정 대기 중 아님. 이미지 크기는 sizing의 image state를 사용하는 코드이며 텍스트 높이48을 이미지 정렬에 사용했다고 보지 않음 |
| 현재 frame | 물리 `(2326,1080,600,422)` | 배율은 당시 inspect에 직접 포함되지 않아 실관측 배율로 확정하지 않음 |
| 일반 열기 예약 | 물리 `(3496,1746,600,422)` | 크기는 같고 위치는 다름. snapshot 당시 ‘이미 목적지와 같아 안 움직임’은 해당 일반 열기 예약에 대해서는 배제 가능 |
| 명시 정렬 이력 | `stash.last_arrangement=null` | Right/Up 실행 자체가 확인되지 않음. 위 예약을 명시 정렬 결과로 사용할 수 없음 |
| 두 계산의 차이 | Right는 열 stack, Up은 셀 중앙 배치·셀에 이미지 frame 비율 fit | 둘 다 수집된 image state를 쓰지만 서로 다른 목적지다. 한쪽 통과로 다른 쪽을 통과 판정하지 않음 |

텍스트 fixture는 `animating=false`이고 목적지 예약이 없어 일반 열기 전환이 끝난 상태였다. 이미지와 텍스트를 같은 순간의 안정 상태로 취급하면 안 된다. 다른 작업의 Right 계산 일치 근거는 참고일 뿐 이 fixture의 명시 정렬 실행 증거에 합치지 않는다.

## 팀장에게 제안하는 실행 조건

1. **지금은 재실행하거나 사용자에게 제스처를 요청하지 않는다.** 11번·20번 등 기존 앱·모니터 시험 세트를 그대로 둔다. 다른 앱 종료·lease 변경·자동 stash·새 debug 명령을 사용하지 않는다.
2. 기존 전역 소유 앱의 유지 의무가 끝나 사용자가 정상 종료하는 등, 다른 작업을 침해하지 않고 자기 인스턴스가 소유권을 얻을 수 있는 차례가 나중에 생긴 경우에만 팀장이 새 native 차례를 배정한다. 현재 앱들을 유지하는 조건이 계속되면 두 정렬 검증은 대기한다. 비소유 앱은 소유 앱이 종료돼도 설치 시 한 번 결정한 lease를 자동 재획득하지 않으므로 자기 앱도 승인된 재실행이 필요하다.
3. 그때 실행 전 `dev:info`의 root/id/binary, 실제 PID·경로, inspect.ok/instance/fixture를 다시 대조한다. 텍스트 inspect의 `stash.shortcut`이 `Ctrl+Alt+Cmd`인지 확인한다. ‘사용 중’ 또는 ‘입력 감지 권한 필요’가 붙으면 중단하며 권한 변경도 임의 실행하지 않는다.
4. 사용자가 native 제목과 고유 표식으로 자기 창을 직접 식별할 수 있어야 한다. 트레이 최상단의 정확한 브랜치 줄 `codex/mixed-note-arrangement`를 확인한다. 메모 ID·웹 제목·단색 이미지 fixture를 함께 대조한다. 브랜치 줄 하나만으로 출처를 확정하지 않는다.

## 조건 충족 후 사용자 수동 확인 순서 (아직 요청·실행하지 않음)

대상은 전용 텍스트 `arrange191-text`, 이미지 `arrange191-image`, 터미널 `terminal-1910001` Welcome, 웹 `web-arrange191` Welcome뿐이다. 본문 표식은 ‘정렬 검증 191 / 본 워크트리 전용 텍스트 fixture’, 이미지 제목은 ‘정렬 검증 191 arrange191-image’, 웹 native 제목은 `Agent Gibbon [DEV codex/mixed-note-arrangement · ef9fadde] — 웹 — web-arrange191`다. native 제목은 실행 시 다시 확인한다. 외부 URL과 셸을 실행하지 않는다.

| 단계 | 사용자가 한 번 수행할 행동 | 에이전트 관측·기대 결과 |
|---|---|---|
| 안정·기준 | 고유 네 창을 확인하고 전환이 멈출 때까지 대기 | 두 text/image inspect의 animating=false, layout_pending=false. 네 창의 position/size/visible 기준값 저장. terminal running=false/pid=null, web welcome=true/url=null 유지 |
| 캡처 수동 제외 | 자기 단색 이미지 창 툴바를 빈 위치로 한 번 이동 | image.auto_layout_excluded=true, visible=true 확인. 다음 목표와 우연히 같은 위치가 되지 않도록 직전 frame 기록 |
| 오른쪽 정렬 | 자기 입력 소유권 확인 뒤 세 보조키를 누르고 모두 놓은 다음 포인터를 오른쪽 40 논리px 이상 이동 | text.stash.last_arrangement가 `Right`, 이미지 포함 네 창이 같은 모니터 작업 영역의 오른쪽 stack으로 이동. 안정 후 position/size 확인. 제외 flag가 true여도 명시 정렬에는 참여해야 함 |
| 5초 취소 | 표시된 ‘취소하기’를 5초 안에 클릭 | 직전 네 창 frame과 복원값 비교. 첫 실행은 실수 방지를 위해 이 단계까지만 진행하고 결과 저장 |
| 바둑판 | 다시 안정 기준 저장 → 같은 보조키 해제 후 위로 이동 | last_arrangement=`Up`, 네 창이 셀 중앙으로 분산, 이미지 프레임 비율 유지. 화면 부족 오류는 누락 성공으로 처리하지 않음 |
| 취소 만료 | 위 정렬에서 취소를 누르지 않고 5초 경과 | 정렬 위치 유지·취소창 사라짐. Right의 기존 이력과 혼동하지 않음 |
| 숨김 제외 | 개발 CLI close로 자기 image fixture만 숨김 → 같은 Up 수행 | image.visible=false 유지, 나머지 세 창만 이동. 원래 숨긴 이미지가 새로 열리지 않아야 함 |
| 명령 취소 | 안정 상태에서 보조키 해제 후 왼쪽 또는 Esc | 네 창 frame과 last_arrangement 변화 없음. 직전 정렬 이력은 누적값이므로 초기화됐다고 가정하지 않음 |

5초 취소 검증 중 에이전트가 장시간 추가 작업을 시키지 않는다. 사용자가 취소를 누른 뒤 읽기 전용 inspect로 복원 frame을 수집한다. 위치·크기가 이미 같았으면 그 창의 이동 성공 증거로 쓰지 않고, 해당 창만 다음 실행 전 사용자가 수동으로 이동하여 구분한다. Undo 실행 후 숨긴 창을 다시 열지 않는 회귀는 별도 작은 단계로 분리한다.

관측 명령은 해당 루트의 `scripts/agent-gibbon-dev inspect --id <위 ID>`만 사용한다. text/image의 `ok,instance,id,note.capture,visible,animating,layout_pending,auto_layout_excluded,folded,layout_destination,position,size,stash.last_arrangement,stash.active`, web의 Welcome/URL·frame, terminal의 running/pid/PTY·frame을 저장한다. inspect는 type별 필드가 다르므로 웹·터미널에 없는 필드를 false로 채우지 않는다. 현재 inspect는 scale/작업 영역/내부 image_size/최소화 상태를 직접 반환하지 않으므로 그 필드의 실제 관측이 없으면 보고서에도 미확인으로 둔다.

필요하면 기존 지원 `AGENT_GIBBON_ENTRANCE_TRACE`를 **자기 앱의 승인된 시작 전** 전용 `/tmp/agent-gibbon-191-entrance.jsonl`로 지정할 수 있다. 기존 trace는 instance/PID/window/generation과 destination의 scale·프레임 진행을 기록하고 본문은 기록하지 않는다. 스냅샷·사용자 행동 시각과 대조하되 trace만으로 입력 주체를 추정하지 않는다. 새 제품 명령/권한은 추가하지 않는다. 이 환경변수 설정·재실행은 아직 하지 않았다.

## 제출 범위

- 이미 수정·자동 검사 완료: 웹·터미널 수집 누락 및 크기·생성 순서 처리.
- 미완료: 캡처 누락 자체 재현·native 두 정렬·5초 취소·숨김 회귀·web child/실행 PTY fit.
- 따라서 현시점에서 전체 해결·이슈 종료·제품 최종 커밋 승인을 요청하지 않는다. 미완료 기준을 별도 명시한 검토용 결과를 유지한다.
- 재조사는 문서/기존 JSON 재분석만 했으며 제품 소스와 기존 미커밋 변경을 보존했다. Conf 3 / Risk Medium 판정 유지.
