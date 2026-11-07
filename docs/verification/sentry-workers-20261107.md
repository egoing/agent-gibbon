# #64 추가: 터미널·캡처 worker의 Sentry 부모 연결

## 범위·출처

사용자가 “터미널, 캡처 워커까지 처리는 못해?”라고 추가 요청해 기존 PR #89에서 구현했다. 디자인 시스템에 진단 내부 구현까지 기록한 것이 적절한지도 질문했고, 화면·동의·상호작용 기준이 그대로인 이번 기술 세부는 진단/검증 문서로 모으고 앞서 추가한 디자인 시스템 문구를 제거했다.

- 루트: `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/64-sentry-thread-context`, 개발 instance `dev-ab5b8ab5f6f5201ea673e343`.
- 추가 기준: `89aac79a2723e8f1587a509b01cbc01518ebd7bf`, `refs/is2/workers-base/64` 보존. 최초 base/red/attempts 및 followup ref는 덮어쓰지 않는다.
- red: `9a47a6ed468d97c559bd1b36c85d81122a94850c`, `refs/is2/workers-red/64`. 테스트 파일만 커밋/push하고 PR을 Draft로 전환했다.
- 원본 `/tmp/agent-gibbon-is2-64/workers/`. 기준 baseline.json 및 baseline-*.log: Rust178/UI120·타입·Rustfmt/Clippy·개발 빌드 모두 PASS, 제외0.
- `red-confirmed.log`가 정본: keeps_request_parent 테스트 2개가 각각 transaction 1개 기대에 실제2개로 실패. 초기 red.log는 먼저 실행한 필터 테스트와 재실행의 로그 경로가 겹쳐 출력 일부가 섞여 사용하지 않는다. 프로세스 완료 후 별도 파일로 다시 실행해 같은 요구 실패 2개를 확인했다. 컴파일/설정 오류가 아니다.

## 구현

`TimingContext::request`는 thread-local/로컬 recorder guard를 만들지 않고 SDK 부모 handle만 생성한다. 기존 Timing의 gate/sampling/조상 생성 로직을 공통 함수로 추출해 그대로 사용한다. request handle은 Send이며 source의 async await 동안 TLS 부모를 남기지 않는다.

- terminal_start: source에서 terminal.request를 만들고 spawn_blocking closure 진입 시 설치. 기존 worker의 terminal.start 및 PTY/프로세스/입출력 연결 단계가 자식이다.
- capture trigger: 큐 중복 방지 뒤 source에서 capture.request 생성 → detached worker에서 부모 설치 → 비Windows UI callback에도 부모 전달. Windows는 기존 worker 내부 capture.prepare 및 lifecycle 잠금 순서를 유지한다.
- Windows lifecycle finish/cancel/failed: 요청 수락 시 capture.request를 생성하고 blocking worker에서 부모 설치 후 기존 처리 operation span을 시작한다.
- 장기 reader에는 Sentry lease를 넘기지 않는다. 원래 local_trace의 parent 전달은 그대로 두며, terminal.output은 별도 최대1% 루트/하위 span으로 유지한다. 사람의 화면 선택 대기 전체도 부모 수명에 묶지 않는다.

## 테스트 의미와 한계

- red의 transaction 개수/trace/직접 parent assertion은 유지하고 실제 운영 경로와 동일한 명시적 context capture/enter를 추가했다. terminal 테스트는 실제 Tauri spawn_blocking을 사용하고 capture 두-hop 테스트는 native UI 대신 별도 스레드로 UI closure 경계를 모델링했다. 실제 화면 캡처/GUI 검증이 아니다.
- request 생성 뒤 source의 별도 output 작업이 독립 루트로 남는지, worker await 완료 및 장기 reader 생존 중 request 루트가 전송되는지 in-memory SDK에서 확인한다.
- 공통 constructor의 sampled/unsampled, 철회→재활성화의 이전 handle 및 비활성 handle 거부, 미실행 요청 lease 취소 해제도 검사한다. 기존 동시 요청·panic·scope 복원·자식 종료 순서·전송 필터 회귀를 유지한다.
- 기존 IPC 테스트는 main Hub·ENABLED·전송 gate 전역을 바꾸는 대신 다른 경계 테스트와 같은 독립 hub/gate harness를 사용하도록 바꿨다. 새 Windows worker 테스트와 병렬로 실행할 때 서로의 capture에 전송이 섞이지 않게 하는 테스트 격리이며, transaction/trace/parent/종료 assertion은 삭제·약화하지 않았다. production constructor와 테스트는 동일 timing_context 엔진을 사용하며 production 전용 test 분기는 추가하지 않았다.
- `terminal.request`/`capture.request`는 기존 승인된 성능 작업명 범주의 고정 이름이다. 새 데이터 범주/전송 목적/동의는 추가하지 않고 기존 필터·메타정보·고정 클라이언트·epoch를 유지한다. 원격 Sentry 전송/동의 생성/설치/다른 기기 실행은 하지 않는다.

최신 로컬 gate·fresh 감사·최신 SHA 플랫폼 결과는 아래와 PR에 구분해 기록한다. 기존 SHA 통과를 현재 실행 증거로 재사용하지 않는다. Risk는 전체 PR 기준 Medium이며 자동 병합하지 않는다. 스킬 커버리지 표와는 추가 worker/Tauri blocking 실행 및 Medium 리뷰 경로를 대조하며, 공용 스킬 원본은 수정하지 않는다.

## 로컬 결과

최종 final.json과 final-{new,regression,typecheck,lint,build}.log: 신규 telemetry20, 전체 Rust182/UI120, cargo check --locked --all-targets, Rustfmt·Clippy(-D warnings), npm run dev:build 모두 PASS. baseline 제외0, 필수 gate SKIP0. 첫 gate도 모두 PASS였고 IPC 테스트의 전역 공유 상태를 격리한 뒤 최종 전체 gate를 다시 실행했다.

이 단계는 in-memory/Tauri blocking·모델 callback 단위/통합 검증이다. 실제 터미널/화면 캡처 GUI·worker span 원격 수신은 미실행이며, Windows cfg 경로는 최신 SHA의 전체 플랫폼 CI로 별도 확인한다. 현재 설치 앱·기존 사용자 프로세스와 동의 상태는 변경하지 않는다. fresh 감사와 최종 최신 SHA의 전체 플랫폼 실행 URL/headSha/결과는 PR #89에 기록한다.
