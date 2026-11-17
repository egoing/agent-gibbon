# 메모 드래그 중 반복 호출 감소 검증

작업 Mac(Darwin 24.6.0, arm64)의 워크트리 `/Users/egoinglee/.codex/worktrees/memo-drag-performance/agent-gibbon`에서 검증했다. 브랜치는 `codex/memo-drag-performance`, HEAD는 `d5be57ae36eb47e07802e185a2f282e9ce2442c3`이다. 기존 미커밋 모아보기 변경 10개 파일이 있는 상태에서 제품 소스 두 파일을 편집했다.

| 변경 경로 | 줄인 작업 | 유지한 동작 |
|---|---|---|
| `src-tauri/src/monitor_layout.rs` | 이동·크기 변경 시 해당 창의 수동 이동 표식이 이미 있으면 모니터 열거·정렬, 창 조회, 표식 재삽입을 생략한다. | 새로 표시될 창은 기존 모니터 전환 검사를 거친다. 저널은 마우스를 놓은 뒤 기존 규칙으로 창별 표식을 소비한다. |
| `src-tauri/src/relation_prototype.rs` | 활성 드래그 창의 이동을 무시하는 기존 분기에서 관계선 UI 갱신 요청도 생략한다. | 32ms 주기의 기존 해제 확인과 드래그 시작·실패 처리, 다른 끝점 이동, 크기 변경, 잠금 경합 시 갱신 요청은 유지한다. |

## 실행 결과

- Rust 단위 테스트는 단일 Cargo 작업으로 관계선 해제·재진입·리사이즈·준비 확인 4개와 모니터 안정화·창별 이동·클릭 후 복원 3개를 선별해 7개 모두 통과했다. 302개는 필터에서 제외했다. 로그는 `/tmp/agent-gibbon-memo-drag-performance-rust-20261117.log`에 있다.
- Node v26.0.0으로 `node --test tests/entrance-drag.test.mjs tests/feedback-drag.test.mjs tests/relation-render.test.mjs`를 실행해 10개 모두 통과했다.
- 편집한 두 Rust 파일의 `rustfmt --edition 2021 --check`와 `git diff --check`가 통과했다.
- `npm run dev:build` 성공. 인스턴스는 `dev-64cba1ae3ddc3d357302d98b`, 바이너리는 이 워크트리의 `src-tauri/target/debug/agent-gibbon`, SHA-256은 `57f49c0c031de16c26b9ece574d94a97e0c4c6e95c9eb38fecc48b8a7c88ac3c`이다. 빌드 로그는 `/tmp/agent-gibbon-memo-drag-performance-20261117.log`에 있다.

실기기 드래그·포커스·초안·캡처·터미널·웹 GUI 시험과 프레임 시간 측정은 수행하지 않았다. 코드에서 중복 작업 생략 경로를 확인했으며 실제 입력 지연 개선 폭은 미확인이다. Conf 3, Risk Low.
