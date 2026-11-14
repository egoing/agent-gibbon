# 이동·숨김 중 관계선 계산 정지 — #168 후속

## 구현 상태와 규칙

사용자가 제안한 ‘움직일 때 선 숨김/계산 정지, 멈추면 계산 후 표시’를 채택했다. **코드·빌드·단위 검증 완료, 현재 실행 앱의 적용은 전체 숨김 해제 후 재시작 대기 중**이다. 실제 드래그·숨김/복원의 GUI 자동 입력 검증은 하지 않았다.

- 해당 메모의 drag/resize 시작 경로와 Moved/Resized/ScaleFactorChanged/Destroyed 이벤트에서 관련된 선만 즉시 숨긴다. 다른 메모 관계는 숨기지 않는다.
- 마지막 이벤트부터 **150ms** 추가 변화가 없을 때 위치·크기 조회와 곡선 계산을 수행한다. 150ms는 조사 구현의 초기 선택값이며 사용자가 확정한 수치가 아니다. 계속 움직이면 대기 시간을 다시 시작한다.
- 일시 정지된 관계는 window/rect 조회와 곡선 계산 분기보다 먼저 건너뛴다. 관련 없는 정지 관계도 위치가 바뀌지 않으면 곡선을 다시 계산하지 않는다. 250ms 보조 확인은 가시성/관계 파일에 남아 있다.
- 전체 숨김 진입/복원 시작 시 모든 선을 먼저 숨긴다. `stash.active`는 숨김 완료 후도 true이므로 숨김 유지와 복원 애니메이션 전체 구간에 위치 계산을 하지 않는다. inactive로 바뀌면150ms 안정화를 새로 시작한다.
- 개별 CLI close/메모 숨김도 선을 먼저 숨긴다. 창 삭제·같은 ID 재생성/관계 설정 교체에서 이전 표시 요청이 다시 선을 살리지 않도록 세대 번호를 무효화한다.
- RAF 좌표 보간을 제거했다. 안정된 곡선은 한 번 SVG에 반영하고, one-shot microtask에서 내부 `relation_ready(generation)`으로 알린다. 서버는 실제 호출 창 label·현재 pending 세대·motion/stash 상태를 검사한 뒤 해당 연결선만 비포커스로 표시한다. 이 내부 확인은 공개 CLI/메모 저장 스키마 확대가 아니다.
- 숨긴 WebView가 프레임 콜백을 계속 실행한다고 가정하지 않도록, 재표시를 RAF에 의존시키지 않았다. DOM 반영 확인이며 GPU가 특정 프레임을 합성한 것까지 계측한 것은 아니다.
- 실제 native 연결창의 위치/크기가 비동기로 보정되면 해당 파생창의 이벤트를 따로 합쳐 실제 viewport로 다시 그린다. 파생창 이벤트를 메모 이동으로 처리하거나 자기 재배치 루프를 만들지 않는다.

이미 전송한 이전 DOM 갱신 작업까지 취소한 것은 아니다. 이동 중에는 **새 좌표 계산/전송을 중단**하고 예전 표시 확인은 폐기해 화면에 다시 나타나지 못하게 한다. 가시성 확인·파일 읽기·이벤트 처리까지 모두 중단하는 것은 아니다.

## 검증

작업 루트 `/Users/egoinglee/.codex/worktrees/67c9/agent-gibbon`, instance `dev-f3d1fc901f372efdf5d35b62`. 기존 source/main 통합이나 무관한 lockfile 변경은 포함하지 않았다.

- 최신 Rust 전체 **230개** 통과. 추가 회귀는150ms 경계·연속 이벤트의 기한 연장·관계없는 양끝에 미치는 영향 없음·dirty timestamp 소진·stash 전구간 정지·이전 세대/취소된 표시 확인 거부를 검증한다.
- UI 단위 **206개** 통과. 기존 보간 시험4개를 정지 후 정적 렌더 시험3개로 교체해 전체 개수가1개 줄었다. 새 시험은 숨긴 WebView의 RAF 없이 DOM 반영 확인, 최신 세대만 확인 전송, 방향/identity/좌표 교체 시 잔존 애니메이션 없음·같은 데이터의 재그리기/텍스트 재측정 억제를 검증한다. DOM은 Node VM mock이며 실브라우저/실앱 드래그 시험이 아니다.
- Clippy all-targets `-D warnings`, Rustfmt·diff 공백 검사 통과. `CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 npm run dev:build` 성공.
- [Rust](assets/issue168/motion-pause/rust.log) · [UI](assets/issue168/motion-pause/ui.log) · [Clippy](assets/issue168/motion-pause/clippy.log) · [빌드](assets/issue168/motion-pause/build.log) · [소스 해시](assets/issue168/motion-pause/source-hashes.json).
- [업데이트 SVG](../design/relation-prototype.svg) · [PNG](../design/relation-prototype.png)는 이동 시작/전체 숨김/정지 후 재표시의3단계를 설명한다. 실제 앱 캡처가 아니다.

## 현재 앱 반영 대기 이유

현재 앱은 이전 테스트 창을 다시 열며 CLI로 시작한 `--serve` PID69025다. watcher가 없으므로 빌드만으로 실행 중인 코드가 바뀌지 않는다. 적용 전6개 표시·미저장 편집 없음·stash inactive를 확인하고 기하/관계 JSON을 기록했으나, 실제 재시작 직전에는 **전체 숨김 세션이 active**였다. 이 상태에서 종료하면 일괄 복원 예약이 없어질 수 있어 종료하지 않았다. 사용자에게 Ctrl+Opt+Cmd로 복원한 뒤 알려 달라고 요청했다.

프로젝트의 ‘사용자 작업의 포커스, 창 위치, 숨김 상태를 보존한다’ 기준을 따른 보류이며 미저장 초안이나 숨김 예약을 자동 폐기하지 않았다. 기존 창 크기 변경·새 GUI 자동 입력·Orca/제거된 computer-use 스킬 사용은 하지 않았다.

Conf3: 단위·정적·빌드 검증. 실제 앱 적용과 사용자의 드래그/resize/전체 숨김 시험은 남았다. CPU/전력은 새로 측정하지 않았으며 기존 프로토타입 CPU 표본을 이번 변경의 이득으로 사용하지 않는다. RiskLow(개발 전용 파생선 표시 정책), 이전 조사에서 생긴 이미지·웹2개 크기 복원 미완료는 별도 조사 기록에서 계속 유지한다. 최종 커밋·push·병합 승인은 대기 중이다.
