# 메모 관계 연결 조사 · #168

## 범위와 재현

작업 루트 `/Users/egoinglee/.codex/worktrees/67c9/agent-gibbon`, 기준 main `87aedf7961bbaa39ced7cd2debc8c852c8754f2d`, 개발 인스턴스 `dev-f3d1fc901f372efdf5d35b62`.

개발 debug 인스턴스에서만 환경변수로 활성화한다. 설치 앱·release에서는 활성화하지 않는다. 공개 CLI와 저장된 메모 스키마는 변경하지 않는다.

```sh
npm run dev:build
AGENT_GIBBON_RELATION_PROTOTYPE=1 ./scripts/agent-gibbon-dev open --id relation-source --markdown '요구사항 정리'
./scripts/agent-gibbon-dev open --id relation-target --markdown '조사용 구현'
./scripts/agent-gibbon-dev inspect --id relation-source
./scripts/agent-gibbon-dev inspect --id relation-target
python3 scripts/relation-prototype.py relation-source relation-target --label '선행 작업'
# 두 메모를 떨어뜨리면 출발 경계 → 도착 경계로 화살표를 표시한다.
python3 scripts/relation-prototype.py --clear
```

이미 실행된 앱에 환경변수는 소급 적용되지 않는다. 이 작업의 앱만 PID·실행 파일로 확인해 종료한 뒤 위 명령을 실행한다. 다른 앱이나 사용자 메모를 종료하지 않는다. 검증용 두 메모만 `close`로 숨길 수 있다.

## 조사 설계와 한계

- 로컬 IPC 디렉터리의 `relation-prototype.json`을 원자적으로 교체한다. 최대 8관계·80자 레이블, 자기 자신·잘못된 ID·제어문자 제외. Python 보조 도구는 한 관계 생성/전체 제거만 제공한다.
- 텍스트·이미지 메모의 동일 인스턴스 ID만 조회한다. 터미널·웹 및 임의 native 창 label은 연결하지 않는다.
- 100ms마다 UI 스레드에서 실제 물리 위치·크기를 읽는다. 물리 좌표의 사각형 경계와 중심 간 직선의 교점을 연결한다. 음수 좌표와 수직·수평 연결을 계산한다. overlay 자체 배율로 글자/선 크기를 보정한다. 혼합 배율 경계에서의 실제 native 좌표 체계는 미검증이다.
- 이동·resize·접힘은 현재 창 경계를 따른다. 숨김·최소화·전체 접어두기·겹침·삭제된 창에서는 overlay를 파괴한다. 다시 나타나면 재생성한다. 레이블은 중간점에 위치하며 다른 메모/관계와 충돌 회피는 하지 않는다.
- 관계의 소유자는 앱 프로세스다. 메모를 native parent로 지정하지 않으므로 한쪽 부모의 최소화나 수명에 자동 종속되지 않는다. 실제 창 존재·가시성으로 관리한다. 관계 파일은 안정 ID를 참조하므로 같은 ID를 다시 열면 연결을 재생성한다. 앱 시작 때 관계 파일을 제거하여 재시작 복원은 하지 않는다.
- 투명·비포커스·비활성 시작·클릭통과·always-on-top overlay. 다른 앱이 메모를 가린 경우에도 선이 위에 남을 수 있다. Space별 가림/전체화면 정책은 제품화 전 별도 결정이 필요하다. 설치 앱/다른 인스턴스의 메모는 관여하지 않는다.
- Tauri [WebviewWindowBuilder](https://docs.rs/tauri/latest/tauri/webview/struct.WebviewWindowBuilder.html)와 [클릭 통과 API](https://docs.rs/tauri/latest/tauri/webview/struct.WebviewWindow.html#method.set_ignore_cursor_events)를 확인했다. 지원 API 확인은 실제 플랫폼 검증과 구분한다.

## 검증 기록

- arm64 macOS, Rust 1.91.1, Node v24.21.0 확인.
- SVG→PNG `rsvg-convert` 렌더와 이미지 육안 확인 완료. 시안은 실제 앱 캡처가 아니다.
- 첫 Rust 검사: 의존성 빌드 중 디스크 공간 소진으로 실패. 이번 워크트리 `cargo clean` 후 디버그 심볼·증분 없이 재시도했으나 의존성 컴파일에서 다시 공간 부족으로 실패했다. 제품 모듈의 컴파일 성공 여부는 확인되지 않았다.
- UI 자동 검사 203개 통과. Rustfmt와 `git diff --check` 통과. Rust 테스트는 실행 단계에 도달하지 못했다.
- 공간 조율 뒤 최신 코드로 개발 빌드·Rust 전체 225개·Clippy all-targets `-D warnings`·Rustfmt·UI 203개·공백 검사 통과. 저용량 환경변수는 빌드 실행에만 지정했으며 lockfile·빌드 설정을 변경하지 않았다.
- 실제 앱 PID `98455`, 실행 파일은 위 작업 루트의 `src-tauri/target/debug/agent-gibbon --serve`. 검증 ID `relation-92c17cf6-a`, `relation-df7f4460-b`; native 제목은 `Agent Gibbon [DEV codex/relation-overlay-prototype · f3d1fc90] — <ID>`. 두 `inspect`의 `ok/instance/id/본문`과 PID 소유 native 창 제목을 대조했다. 재빌드 후 자기 PID `95381`만 종료하고 최신 바이너리로 재시작하여 출처를 다시 확인했다.
- `AGENT_GIBBON_RELATION_PROTOTYPE=1 python3 scripts/relation-prototype-smoke.py` 통과. 생성 전후 두 메모의 포커스 유지, 이동·resize 추적, overlay의 실제 native 사각형 안으로 마우스 클릭 후 아래 메모 포커스 획득, 접힘·펼침, 한쪽 숨김/복원, 겹침/분리, native 닫기/같은 ID 재생성, 관계 제거를 확인했다. 첫 harness는 잘못된 버튼 이름으로 중단됐고 실제 접근성 이름으로 고친 최신 실행이 통과했다.
- 마지막 검증 후 생성한 메모만 숨기고 관계를 비웠다. 원래 foreground PID와 복원 후 PID 일치를 확인했으며 입력 도구는 포인터를 복원했다. 기존 설치 앱과 다른 워크트리 창은 조작하지 않았다.
- [native 증거 JSON](assets/issue168/native-evidence.json)에는 root·instance·binary·각 단계 PID/창 제목/기하 정보와 포커스 복원 결과가 있다. [화살표 창 캡처](assets/issue168/relation-native.png), [출발 메모](assets/issue168/memo-0.png), [도착 메모](assets/issue168/memo-1.png)는 최신 실행의 해당 창만 개별 캡처했다. 다른 앱이 포함된 화면 캡처는 저장소에 넣지 않았다.
- 실제 장치는 1728×1117 논리 좌표·2배율 단일 모니터였다. 다중 모니터/혼합 배율·모니터 제거·다른 Space·전체화면·다른 앱 가림·Windows/Linux 실환경은 미검증이다. 전체 플랫폼 CI·설치/배포는 실행하지 않았다.
- Conf **5**: 위 단일 macOS 환경의 검증 항목만 해당. Risk **Low**: 명시 활성화한 개발 인스턴스의 국소 overlay이며 파일 제거·앱 재시작으로 해제 가능. 제품화 완료나 모든 플랫폼 지원을 뜻하지 않는다.


자동 검증 스크립트는 기존 Swift AX/창 목록/마우스 입력 도구를 임시 디렉터리에 빌드한다. macOS 접근성·화면 캡처·입력 권한이 필요하며 거부 시 성공으로 처리하지 않는다. 관계 생성 후 앱 종료/재시작을 통한 영구 복원은 제공하지 않는다.

## 사용자 요청 후속: 곡선과 이동 갱신 개선

아래 내용이 초기 직선·100ms 반복 갱신 설계를 대체한다. 아직 개발 전용 조사 기능이며 공개 API/영구 저장/연결 편집 UI는 추가하지 않았다.

- 서로 마주 보는 변의 중심에 접점을 배치하고 cubic Bézier 곡선으로 연결한다. 도착 방향을 따르는 화살촉과 곡선 중간의 수평 레이블을 표시한다. 겹치는 메모의 연결은 숨긴다. 다른 메모를 피하는 경로 탐색은 하지 않는다.
- 메모 이동·resize·배율·삭제 이벤트를 크기 1의 채널로 합쳐 최대 약 60Hz로 처리한다. 네이티브 이벤트가 없는 가시성·관계 파일 변경은 250ms 보조 확인으로 감지한다. 실제 프레임레이트가 60fps임을 실측했다는 뜻은 아니다.
- 현재 곡선 주변에 이동 여유를 둔 native overlay 영역을 확보하고, 범위 안의 이동은 native 위치·크기를 바꾸지 않는다. 범위를 벗어나거나 긴 레이블에 여유가 더 필요하면 영역을 다시 잡는다. 좌표계가 바뀔 때는 즉시 새 좌표로 그린다.
- geometry·레이블·배율이 같으면 WebView eval을 보내지 않는다. 관계 JSON은 변경된 바이트만 파싱한다. 일반 갱신에는 requestAnimationFrame에서 약 24ms 시정수로 좌표를 보간하고, 수렴하면 프레임 예약을 중단한다. 레이블 크기도 문구/배율이 바뀔 때만 측정한다. 정지 상태의 **재그리기**를 없앤 것이며 저빈도 상태 확인이나 기존 메모 기능의 폴링이 모두 없어지는 것은 아니다.
- 초기 페이지 로딩 완료 뒤에만 첫 곡선을 전송한다. 숨김·삭제·겹침으로 제거된 연결은 필요할 때 다시 생성한다. 클릭 통과·포커스 비활성·설치 앱/다른 인스턴스 분리는 유지한다.

### 최신 검증

- 같은 작업 루트·instance `dev-f3d1fc901f372efdf5d35b62`, 사용자가 시작한 watcher의 새 PID `55569`. `proc_pidpath`의 절대 실행 파일을 `dev:info.binary`와 대조하고 두 검증 메모의 inspect.instance·본문 및 PID 소유 창 제목을 확인했다. watcher가 자동 재빌드했으며 사용자 프로세스를 수동 종료하지 않았다.
- 최신 Rust 전체 **226개**, UI **205개**, Clippy all-targets `-D warnings`, Rustfmt, diff 공백 검사 통과. Rust에는 방향/음수·수직·겹침·접촉 경계와 native 영역 유지 검증을 포함한다. JS에는 보간 수렴 후 RAF 중단, 빠른 목표 합치기, 좌표계 변경 시 잔존 애니메이션 취소, 텍스트 재측정 억제를 검증하는 2개 테스트를 추가했다.
- `GIBBON_RELATION_SMOKE_OUTPUT=/tmp/gibbon168-smooth/native python3 scripts/relation-prototype-smoke.py` 성공. 이동 시 overlay native 영역이 유지되는 것과 resize·실제 마우스 클릭 통과·접힘/펼침·숨김/복원·겹침/분리·native 닫기/동일 ID 재생성·관계 제거를 확인했다. 검증 ID `relation-08dbaf70-a`, `relation-c4703bdb-b`.
- 관계 JSON은 원자적으로 백업/복원했다. 검증 메모만 숨겼고 사용자 관계 `relation-098c6c3b-a → relation-17e6ae18-b`의 `참고` 레이블을 복원했다. 원래 foreground PID와 검증 후 PID **80374** 일치 확인. 사용자 메모 위치·크기·표시 상태는 건드리지 않았으며 watcher와 개발 앱은 유지했다.
- [최신 native 증거](assets/issue168/smooth/native-evidence.json) · [실제 곡선 창 캡처](assets/issue168/smooth/curve-native.png). [업데이트 SVG](../design/relation-prototype.svg)와 PNG는 렌더 후 이미지 확인 완료. 실제 캡처와 설명 시안은 구분한다.

### CPU 비교

`python3 scripts/relation-performance.py`는 별도 검증 메모 2개만 이동한다. 같은 300×180 논리 크기와 출발 좌표, 동일한 5초 60Hz AX 위치 변경 요청 300회, 같은 관계 문구를 사용했다. macOS `proc_pid_rusage`의 누적 user+system CPU 시간 차이를 wall time으로 나눴다. 수치는 **한 코어 100% 기준**이며 앱 Rust 프로세스와 관계 생성 직후 새로 생긴 overlay WebContent 1개만 합산했다.

| 구간 | 이전 직선/100ms | 새 곡선/이벤트·RAF |
|---|---:|---:|
| 정지 5초 | 0.37% | 1.06% |
| 이동 요청 300회 | 0.50% | 0.97% |

[이전 원시 표본](assets/issue168/smooth/before-performance.json) · [이후 원시 표본](assets/issue168/smooth/after-performance.json) · [소스 해시](assets/issue168/smooth/source-hashes.json).

이 표본에서 측정 대상의 사용량은 약 1코어의 1% 수준이었다. **CPU 절감이나 전체 앱/시스템 비용 감소가 입증된 것은 아니다.** 새 방식은 더 자주 움직임을 반영해 이동 중 비용이 늘 수 있다. 각 1회·짧은 표본이고, 백그라운드 빌드 등 주변 부하를 통제하지 않았다. 이전/이후 실행 사이 검증용 숨김 메모 WebView 수도 달라져 Rust 호스트 비용에 영향을 줄 수 있다. 메모 본문 WebContents·GPU·WindowServer·전력·고주사율·장시간 드래그·8개 관계 부하는 측정하지 않았다. 곡선 native 영역을 넓게 유지하는 비용도 전체 GPU/합성 비용 실측으로 확인한 것은 아니다. 최초 baseline cleanup의 AppleScript 포커스 복원 명령은 실패했지만 foreground PID가 원래와 같은 80374인 것을 확인했고, 최신 자동 검증은 native AppKit 복원과 PID 일치 검증을 통과했다.

Conf **5**는 위 단일 macOS 실행 항목에 한정한다. Risk **Low**. 혼합 배율·다중 모니터·다른 Space·Windows/Linux와 장애물 회피는 계속 미검증/미구현이다. 최종 커밋·push·병합 승인 대기를 유지하며 무관한 package-lock.json 변경은 보존한다.

### 정지 상태 스케줄링 검토

추가 정적 검토에서 연결 렌더러에는 상시 interval이 없고, requestAnimationFrame은 수렴 시 다시 예약하지 않는 것을 확인했다(자동 테스트 포함). Rust의 250ms 보조 상태 확인은 의도적으로 남아 있다. 연결되지 않은 메모의 이동/resize 이벤트도 상태 확인 스레드를 깨울 수 있으나 payload가 같으면 그리기·native 배치를 하지 않는다. 양끝 메모만 알림 대상으로 제한하는 것은 후속 최적화 여지이며, 위 CPU 차이의 원인으로 실증한 것은 아니다. 실제 드래그 프레임률·목표 위치까지의 지연을 계측하지 않았으므로 CPU 표본만으로 버벅임 해결이나 프레임 성능 개선을 확정하지 않는다.

### 최종 — 연결 양끝 이벤트 제한과 반복 표본

위 정적 검토에서 확인한 후속 여지를 이번 작업에서 추가 적용했다. 관계 설정이 바뀌면 양끝 native label 집합을 갱신하고, 해당 메모의 이동/resize/배율/삭제 이벤트만 확인 스레드를 깨운다. 관련 없는 메모 이벤트는 큐에 넣지 않는다. 연결창 페이지 완료 알림과 250ms 보조 확인은 유지한다.

최신 watcher PID는 **70081**이며 사용자 프로세스를 수동 종료하지 않았다. 전체 Rust226·Clippy all-targets·Rustfmt 통과. 아래 표본은 끝점 이벤트 제한까지 반영한 코드에서 같은 검증 메모·같은 5초 300회 AX 이동/5초 정지 구간을 **3회씩** 반복한 결과다. [원시 표본](assets/issue168/smooth/final-performance.json).

| 구간 | 반복1 | 반복2 | 반복3 |
|---|---:|---:|---:|
| 정지 | 0.243% | 0.245% | 0.231% |
| 이동 300회 | 0.547% | 0.588% | 0.564% |

측정 대상은 계속 Rust 앱+관계 WebContent 하나, 한 코어 100% 기준이다. 정지시 관계 WebContent CPU는 각 5초 동안 약 0.000008~0.000182초였고, Rust의 보조 상태 확인과 기존 앱 기능은 남는다. 앞의 미필터 단일 표본보다 낮아졌지만 주변 부하·숨김 메모 수가 통제되지 않았으므로 차이 전부를 이벤트 필터 효과로 귀속하지 않는다. 최초 직선 baseline도 1회뿐이므로 통계적으로 신뢰할 수 있는 이전/이후 성능 차이를 확정하지 않는다. 이동 중 더 자주 그리는 비용과 추적 지연은 별개의 지표이며, 실제 fps·입력에서 표시까지 지연·GPU/전력은 여전히 미측정이다.

최종 코드의 native 14단계 회귀도 PID70081에서 다시 통과했다. 검증 ID `relation-2db85ab4-a`, `relation-5363db23-b`; [최종 출처/회귀 증거](assets/issue168/smooth/final-native-evidence.json) · [최종 곡선 캡처](assets/issue168/smooth/final-curve-native.png). 관계 JSON 복원과 최종 회귀의 foreground PID70081 전후 일치는 확인했다. 이후 사용자 메모를 과거 좌표와 비교한 추가 검사는 실패했다. 현재 좌표는 `(1144,332)`, `(2294,684)`이고 크기600×96·가시성·자동배치제외는 유지된다. 검증 스크립트는 검증 ID만 직접 이동했지만 실행 중 사용자 조작과 다른 요인의 영향을 분리하지 못했으므로 원위치 보존을 완료로 단정하지 않는다. 현재 사용 중인 배치를 과거 값으로 강제 복원하지 않았다. watcher는 계속 실행 중이다.

## 이동 중 다른 메모에 붙어 보이는 현상 후속

사용자는 A에 붙어야 할 선이 이동 중 잠깐 B에 연결되는 현상을 보고했다. 정확한 사용자 화면을 직접 캡처해 원증상 전체를 재현한 것은 아니다. 코드 검토에서는 모든 제어점과 양 끝점을 약24ms 시정수로 보간하므로 빠른 이동·포트 전환에서 끝점이 이전 위치를 지나갈 수 있음을 확인했다. surface 순번을 재사용할 때도 출발/도착 identity를 renderer에 전달하지 않았다.

### 변경과 검증

- 양 끝점은 draw 갱신을 받으면 최신 native 좌표에 즉시 고정한다. source 제어점은 source 끝점 이동량만큼, target 제어점은 target 끝점 이동량만큼 함께 이동한다. 남은 곡률 변화만 RAF에서 보간하며 끝점은 RAF 중에도 고정한다.
- Rust payload에 `(from,to)` identity를 포함한다. identity·포트 방향·native 좌표계가 달라지면 큐에 남아 있는 이전 애니메이션을 취소하고 새 곡선을 즉시 그린다. 단순 평행 이동은 전체 곡선을 즉시 옮기고 추가 애니메이션을 만들지 않는다.
- UI **207개**, Rust **226개**, Clippy all-targets `-D warnings`, Rustfmt·공백 검사 통과. JS 회귀4개는 이동 직후/매 RAF의 끝점 일치, 평행 이동의 잔존 프레임 없음, 포트 방향 전환과 관계 identity 교체 시 즉시 취소, 빠른 목표 합치기/좌표계 변경을 검증한다.
- 최신 watcher PID **87479**, instance `dev-f3d1fc901f372efdf5d35b62`. 절대 binary를 proc_pidpath로 확인하고 검증 메모 `relation-b791c8e2-a`, `relation-0841f153-b`의 inspect.instance·본문과 PID 소유 native 제목을 확인했다.
- `GIBBON_RELATION_SMOKE_OUTPUT=/tmp/gibbon168-anchor-native python3 scripts/relation-prototype-smoke.py`의 **18단계** 통과. 아래/오른쪽 방향 전환 2회와 관계 source/target 반전·복원, 기존 이동·resize·클릭 통과·접힘·숨김/복원·겹침·닫기/재생성을 포함한다. 관계 JSON은 원자적으로 복원하고 검증 메모만 숨겼다. foreground PID 전후 일치를 확인했으며 사용자 watcher는 수동 종료하지 않았다. 사용 중인 사용자 메모 배치를 과거 좌표로 강제 복원하지 않았다.
- [native 출처·회귀 증거](assets/issue168/anchors/native-evidence.json) · [최신 곡선 캡처](assets/issue168/anchors/curve-native.png) · [소스 해시](assets/issue168/anchors/source-hashes.json). 정지 상태 외형이 같으므로 기존 디자인 SVG/PNG는 변경하지 않았다.

최신 native 위치가 renderer에 전달되기까지의 OS/IPC 지연이나 다른 메모 위를 곡선이 지나가는 장애물 회피까지 해결한 것은 아니다. 원증상 전체가 사라졌다고 보장하거나 이전 CPU 표본을 이 수정의 재측정 결과로 취급하지 않는다. Conf5는 위 native 검사에 한정하며 RiskLow, 최종 커밋·push·병합 승인은 계속 대기한다.

## OS 위치 보정으로 분리되는 선: 원인과 직접 픽셀 검증

사용자 첨부 화면에 메모와 선 끝이 분리된 모습을 확인했다. 이전의 보간 수정과 ‘연결선 창 생성/native 회귀 통과’만으로 원증상이 해결됐다고 볼 수 없었다. 실제 창 좌표와 SVG 끝점의 픽셀 일치 검사가 누락된 한계를 인정한다.

### 재현과 원인

동일 워크트리/instance의 사용자 메모 `relation-098c6c3b-a`, `relation-17e6ae18-b`와 PID92198의 창을 대조했다. 실제 메모 경계의 연결점은 전역 논리 좌표 `(503,237)`과 `(232,650)`이었다. native 연결창은 X=-56pt, **Y=38pt**에 있었다. 여유 공간을 포함한 요청 원점은 물리 Y=-102px(2배율에서 -51pt) 근처였지만, macOS가 메뉴 막대 아래 38pt로 옮겼다. 요청 원점을 SVG 기준으로 쓰면서 약89pt/178px 오차가 발생했다. 요청 높이와 실제 높이도 달라질 수 있었다.

`python3 scripts/relation-endpoint-pixels.py 92198 relation-098c6c3b-a relation-17e6ae18-b --output /tmp/gibbon168-clamp/before.png --expect-detached`에서 두 끝점 모두 **stroke_present=false**를 확인했다. [수정 전 결과](assets/issue168/native-origin/before.json) · [연결선 창 단독 캡처](assets/issue168/native-origin/before.png). 사용자 첨부 원본이나 다른 앱의 내용은 저장소에 넣지 않았다.

### 수정

- reserve 계산용 요청 bounds는 캐시해 이동 중 native 재배치를 억제한다.
- `set_position`과 `set_size` 이후 실제 `outer_position`과 `outer_size`를 다시 읽는다. resize가 위치에 미치는 보정까지 포함한 실제 viewport를 SVG width/height·끝점 오프셋 계산에 사용한다.
- 실제 viewport 변화 시 이전 보간을 취소한다. 요청 bounds와 실제 bounds를 구분해 OS clamp가 다음 확인마다 재배치로 이어지거나 원점 오차가 누적되지 않도록 했다.
- 물리 좌표를 한 번만 실제 원점에서 빼고, SVG viewBox의 실제 물리 크기와 native 배율을 사용한다. 전역 음수 좌표는 0으로 잘라내지 않는다.

### 최신 직접 검증

- watcher가 자동 재빌드한 **PID98381**의 실행 파일을 proc_pidpath로 `dev:info.binary`와 대조하고 inspect.instance·본문/메모 ID와 PID 소유 native 제목을 확인했다. 같은 사용자 메모와 같은 화면 끝점에서 수정 후 **양쪽 stroke_present=true**를 확인했다. [수정 후 결과](assets/issue168/native-origin/after.json) · [연결선 단독 캡처](assets/issue168/native-origin/after.png).
- 픽셀 검사는 PID 소유 두 메모의 실제 native 경계에서 기대 접점을 구하고, 연결선 창을 단독 캡처한 RGBA 이미지의 해당 화면 좌표 주변4px에 실제 선 색/alpha가 존재하는지 확인한다. native 논리 좌표와 캡처 픽셀 크기의 비율을 사용한다. 캡처 전후 메모 경계가 바뀌면 검사를 실패 처리한다. 첫 스크립트 실행에서는 픽셀 변수명이 target 사각형 변수를 덮어써 안정성 assertion이 실패했고, 변수명을 고친 재실행에서 위 재현을 확인했다.
- 최신 Rust **227개**, UI **207개**, Clippy all-targets `-D warnings`, Rustfmt·공백 검사 통과. 새 Rust 회귀는 요청 음수 Y가 실제 +76px로 보정된 경우, 음수 X/Y viewport와 resize 이후의 좌표 변환 및 반복 계산의 무누적을 검증한다.
- `GIBBON_RELATION_SMOKE_OUTPUT=/tmp/gibbon168-clamp/native python3 scripts/relation-prototype-smoke.py`의 기존 **18단계** 통과에 더해 실제 픽셀 검사를 포함했다. 검증 ID `relation-e96e3575-a`, `relation-95177f33-b`.
- [resize 후 픽셀 일치](assets/issue168/native-origin/endpoints-resize.json), [아래로 방향 전환 후](assets/issue168/native-origin/endpoints-80-600.json), [오른쪽으로 전환 후](assets/issue168/native-origin/endpoints-700-310.json) 모두 양 끝점이 선 픽셀과 일치했다. 각 단계의 native창·전체기하 및 포커스 근거는 [native 증거](assets/issue168/native-origin/native-evidence.json), 소스 출처는 [해시](assets/issue168/native-origin/source-hashes.json)를 따른다.
- 기존 사용자 관계 JSON을 원자 복원했고 검증 메모만 숨겼다. foreground PID 전후 일치 확인. 사용자 메모를 강제로 과거 위치로 옮기지 않았으며 사용자 watcher/앱은 유지했다.

위 증거는 단일 macOS2배율·상단 메뉴 막대 보정에 대한 실제 재현/수정 확인이다. 혼합 배율·모니터 제거·여러 Space·다른 플랫폼과 전체 GPU/CPU 재측정은 하지 않았다. 요청 frame 유지와 실제 원점 재확인으로 반복 재배치를 피하지만, 모든 OS의 위치 보정 정책을 검증한 것은 아니다. Conf5/RiskLow는 기록한 범위에 한정하며 최종 커밋·push·병합은 미승인 상태다.
