# position 워크스페이스 미커밋 변경 인계

원본 워크스페이스 폐기 후에도 필요한 파일은 새 워크스페이스와 `/Users/egoinglee/orca/handoffs/position-20261104-155506`에 남는다. 커밋은 하지 않았다.

## 반영 내역

- README, docs/design-system.md, docs/verification.md, scripts/entrance-direction-smoke.py, docs/design/placement-zero-gap.svg/png: 원본과 동일하게 복사했다.
- main.rs의 가장자리 gap=0 변경: 새 구현에서 place_new_window가 layout.rs로 대체되었고 작업 영역을 축소하지 않는 동등 동작이 이미 존재함을 확인했다. 옛 무작위 배치 함수를 복원하지 않았다.
- warp.rs: 0px 인수가 이미 반영되어 있으며 새 제외 플래그 코드를 보존했다.
- placement.rs: 기존 zero_gap_fills_exact_horizontal_and_vertical_slots 회귀 테스트를 새 stack API에 맞게 이식했다. 원본 파일 전문은 백업에 있다.
- 새 에이전트의 entrance.rs, main.rs, sizing.rs, warp.rs, ui/app.js, layout.rs, package-lock.json 변경을 보존했다.

## 후속 작업

사용자가 이 인계를 위해 에이전트를 일시 중지했다. 재개할 때 이 문서를 읽고 현재 코드를 기준으로 계속한다. 이전 안내의 ‘원래 워크스페이스에서 읽어서 가져오라’는 지시는 본 인계로 대체한다. 원래 경로가 삭제돼도 의존하지 말 것.

원본 문서는 이전 0px 최소 간격 단계의 기록이다. 특히 무작위 y 유지 설명은 새 요구와 다르므로 새 구현에 맞게 README/디자인 시스템/시안을 갱신하고, 과거 실환경 검증을 새 코드 검증으로 재사용하지 않는다. 새 에이전트 대화에서 추가된 사용자 요청인 재배치 전환 애니메이션도 유지한다.

최종 요구: Undo/Redo·작업 이력 없이 메모별 자동 배치 제외 플래그만 사용. 마지막 사용자 기하학 작업이면 제외, 후속 비기하학 작업이면 포함. 일반 생성/숨김 복원 때 오른쪽 위부터 아래→왼쪽 열로 0px 정렬. 높이는 기존 min/max 안에서 사용 가능 높이를 고려해 계산. 영역 지정 생성은 트리거/대상 제외. 일반적인 여러 메모 열기와 제외/복귀 및 전환 동작을 새 워크스페이스 개발 앱에서 검증한다.

## 보존 자료

- 백업: `/Users/egoinglee/orca/handoffs/position-20261104-155506`
- source/: 원본 미커밋 파일 9개 전문, source.patch: tracked diff
- destination-before/: 이식 전 새 에이전트 변경 전문, destination-before.patch
- manifest.json: SHA-256 대조용 목록
- evidence/: 원본 실환경 검증 JSON·부분 캡처·실행 스크립트. 원본 검증 출처는 position이며 새 구현의 증거가 아니다.

## 이관 검증

복사 파일 SHA-256 원본 일치, 기존 에이전트 변경 보존, git diff --check 통과. 새 워크스페이스에서 cargo test --manifest-path src-tauri/Cargo.toml zero_gap_fills_exact_horizontal_and_vertical_slots 통과(1 passed). 전체 기능 및 실제 앱 재검증은 재개 후 수행할 것.
