# 반복 숨김·복원 후 간격 축소 조사 — #168

## 결론과 적용 범위

**간격이 줄어드는 사용자 원증상의 직접 원인은 아직 확정하지 못했다.** 연결선 창까지 전체 숨김 스냅샷에 포함되는 이번 프로토타입의 별도 결함은 확인하고 최소 필터로 수정했다. 이를 간격 축소 원인 해결과 동일하게 취급하지 않는다.

- 기존 코드: 메모6개+연결선7개를 켜면 stash 저장 대상13개.
- 수정 코드: 연결선은 파생 표시로만 관리하고 stash 저장 대상은 메모6개. 기존 일반 메모·이미지·캡처 overlay·터미널·웹·관리 창 분류는 유지한다.
- 동일 기하의 텍스트 메모6개에서 연결 OFF/ON 비교를 수행했다. 수정 전후 각각20개의 관측 구간에서 위치·크기·간격 축소는 재현되지 않았다.
- 사용자 입력의 실제 Ctrl+Opt+Cmd/포인터 접촉 경로와 이미지·웹을 섞은 원래 환경은 격리 대조에서 재현한 것이 아니다. 기존 비동기 geometry 경합 가능성은 검토했지만 이를 원인으로 단정하거나 그 코드를 변경하지 않았다.

## 최초 사용자 인스턴스 시험의 한계와 복원 미완료

작업 루트 `/Users/egoinglee/.codex/worktrees/67c9/agent-gibbon`, instance `dev-f3d1fc901f372efdf5d35b62`, 당시 PID98381의 절대 실행 파일과 inspect.instance를 확인했다. 시작 시 전체 숨김 상태였고, 예제6개 외 이미지·웹·텍스트1개를 포함한9개가 stash에 있었다.

합성 키 입력은 전환이 불안정했고, 메뉴 AXPress도 닫힌 메뉴의 항목과 실제 표시 상태가 달라 신뢰할 수 없었다. 실제 메뉴 클릭으로 전환했으나 빠른 반복에서는 메뉴 항목 조회가 실패하여 사용자 인스턴스의 ON/OFF 인과 대조 결과로 사용하지 않았다. 이 과정에서 `relation-prototype-3`만 stash 대상에 남고 메모는 보이는 비정상 조합도 관찰했으나 그 발생 경로를 단정하지 않는다.

**시험 후 이미지·웹 창2개는 AX 최소 크기·이미지 비율 제약 때문에 시험 전 크기로 정확히 복원하지 못했다.** 사용자에게 이 한계를 알리고 기존 창 반복 시험과 임의 근사 복원을 중단했다. 위치·내용·핀·접힘을 변경하려는 시험은 아니었으며 7개 일반 메모의 위치·크기는 비교 시 일치했다. 당시9개는 원래의 전체 숨김 상태로 돌아갔다.

| 창 | 시험 전 크기(물리 px) | 시험 직후 크기(물리 px) | 당시 위치 |
|---|---:|---:|---|
| `image-1794637735188534000` | 312×102 | 108×98 | (2916,2004), 일치 |
| `web-1794637784024966000` | 380×154 | 480×320 | (2752,2036), 일치 |

[시험 후 상태 비교](assets/issue168/stash/original-window-restoration.json). 이후 사용 중인 창의 배치가 달라진 것을 과거 값으로 강제 복원하지 않았다. 최종 코드 반영 직전에는 사용자가9개를 표시한 상태(stash inactive)였고, 그 새 상태를 별도로 기록했다. 앞의 ‘시험 전’ 값과 나중의 사용자 배치를 혼동하지 않는다.

## 독립 fixture 출처와 시험 경로

기존 앱에 더 영향을 주지 않기 위해 `/private/tmp/gibbon168-stash-fixture`에 HEAD 소스를 export하고 현재 프로토타입의 main/renderer/module만 복사했다. Git index나 사용자 메모 저장소를 공유하지 않는다. 빌드 캐시는 별도 target에 APFS clone했으며, 해당 루트에서 다시 빌드한 뒤 다음 출처를 확인했다.

| 항목 | 값 |
|---|---|
| fixture root | `/private/tmp/gibbon168-stash-fixture` |
| instance | `dev-f937b703a764976ce1bac3d0` |
| binary | `/private/tmp/gibbon168-stash-fixture/src-tauri/target/debug/agent-gibbon` |
| manifest_dir | `/private/tmp/gibbon168-stash-fixture/src-tauri` |
| 필터 전 PID | 43593 |
| 필터 후 PID | 48273 |
| 검증 메모 | `fixture-stash-0` ~ `fixture-stash-5` |

해당 fixture의 `scripts/agent-gibbon-dev`만 사용했다. --instance-info.manifest_dir, proc_pidpath 실행 파일, 각 inspect.instance·고유 본문, PID 소유 native 제목을 확인했다. 실제 스크린은 macOS2배율 단일1728×1117 논리 화면이다.

임시 fixture에만 로컬 명령 파일 observer를 두고 UI main thread의 **실제 `crate::stash::toggle`**을 호출했다. 별도 숨김 알고리즘을 복제하지 않았다. 내부 saved rect와 실제 native rect를 함께 기록했다. 전역 키는 보내지 않았고 사용자 앱의 shortcut lease를 탈취하지 않았다. 이 observer·명령 파일은 제품 소스/API에 넣지 않았다. [fixture driver 참고](assets/issue168/stash/fixture-test.py.txt) · [fixture observer 참고](assets/issue168/stash/fixture-hook.rs.txt).

### 동일 조건 대조

- 두 모드 모두 같은6개 메모를300×180 논리 크기와 같은3열×2행 좌표로 설정하고 실제 기하가 일치하는지 assert한 뒤 시작했다.
- OFF는 관계0개, ON은 같은 메모 사이 관계7개다. 원래 다른 사용자 창은 fixture에 없다.
- 모드별 정상 완료 뒤 hide/restore5쌍과, 60ms 간격8회 반전을5묶음 수행했다. 후자는 완료하지 않은420ms 전환을 반대 방향으로 바꾸는 구간이다.
- 수정 전20관측: OFF/ON 모두 위치·크기 차이0. ON 시작 saved 목록에 메모6개+연결선7개=13개를 확인했다.
- fixture에서 필터만 바꾸고 재빌드한 수정 후20관측: OFF/ON 모두 위치·크기 차이0, ON saved 대상이 메모6개로 줄었다.
- [필터 전 원시 기록](assets/issue168/stash/before-filter.json) · [필터 후 원시 기록](assets/issue168/stash/after-filter.json). 각 cycle의 before/started/hidden/after와 저장 rect·실제 rect를 포함한다.

초기 fixture 시도는 최초 자동 배치가 겹쳐 두 모드의 높이가 달랐고, 다음 재실행은 observer sequence 번호가 재사용되어 실패했다. 이 두 실행은 유효한 동일 조건 대조로 사용하지 않았다. 위 링크는 초기 기하 assert와 sequence 연속성을 수정한 최종 대조다.

**한계:** main-thread observer가 타이밍을 바꿀 수 있고, 실제 modifier·hover 입력 경로를 우회했다. 텍스트6개만 대조했으며 이미지·웹/고부하·다중 모니터·서로 다른 배율을 섞은 원래 조건과 동일하지 않다. 재현 실패를 ‘사용자 현상 없음’ 또는 ‘연결선 영향 없음’의 증명으로 보고하지 않는다. 이전 CPU 시험도 이 문제의 원인 증거가 아니다.

## 제품에 반영한 최소 수정

stash 스냅샷에 넣기 전에 `relation-prototype-*`와 기존 indicator를 제외한다. 렌더러는 각 메모의 stash 소유/가시성을 따라 연결선을 숨기고 재생성한다. 파생 연결선을 독립적으로 압축·복원하지 않는다. 다른 창 유형은 기존 분류를 유지하도록 회귀 검사를 추가했다.

이미 생긴 malformed stash 세션은 재컴파일한 코드로 메모리만 유지한 채 hot patch되는 방식이 아니다. 적용 시 watcher가 새 프로세스를 시작하므로 기존 임시 stash session은 사라진다. 적용 직전에 stash inactive/9개 표시 상태를 확인했으며, 오래된 session을 새 제품 API로 조작하거나 사용자 창을 파괴하는 정리 경로는 추가하지 않았다.

- 제품 Rust 전체 **228개**, Clippy all-targets `-D warnings`, Rustfmt·diff 공백 검사 통과. UI207개는 앞선 연결점 수정 이후 변경이 없으며 이 조사에서는 Rust 분류만 변경했다.
- 사용자가 실행한 watcher는 수동 종료하지 않았다. 재빌드 후 **PID52141**의 절대 실행 파일과 inspect.instance를 다시 확인했다. 코드 반영 직전의9개 상태와 반영 직후 위치·크기·가시성·접힘·핀은 전부 같았다. 관계 JSON7개도 복원했다. [반영 전후 비교](assets/issue168/stash/filter-application-state.json).
- 이 반영 직전/직후 일치는 앞서 실패한 이미지·웹2개 크기의 ‘시험 전 복원 완료’를 뜻하지 않는다.
- fixture6개는 숨겼고 fixture PID48273만 절대 경로 검증 후 종료했다. 원래 사용자 앱은 유지했다.

Conf5: 기록한 fixture의 대상 혼입과 분류 수정 효과. **사용자 간격 축소 원인은 미확정**이다. RiskMedium: 조사 과정에서 기존 창 크기 상태에 영향이 있었고2개 복원이 미완료다. 최종 커밋·push·병합은 승인 대기이며 무관한 lockfile 변경은 보존했다.
