# #64 IPC Sentry 스레드 문맥 연결 (2026-11-07)

승인 범위는 IPC 수신 스레드 → run_on_main_thread → apply의 직접 자식 연결이다. Refs #64, 이슈 전체 완료 아님. 위험도는 사전/최종 모두 **Medium**: span 수명과 동의 세대 상태에 영향을 준다. 자동 병합 대상이 아니다.

## 환경과 기준

- 작업 루트: `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/64-sentry-thread-context`
- 개발 instance: `dev-ab5b8ab5f6f5201ea673e343`
- 개발 binary: 위 루트의 `src-tauri/target/debug/agent-gibbon`
- 최초 base: `047858207aa36f31baac3870bb1c14a03f887242` (`refs/is2/base/64` 보존)
- red: `a148774` (`refs/is2/red/64`에 full SHA 보존), PR #89.
- macOS ARM64, Node 24.21.0 / npm 11.19.0 / rustc·cargo 1.91.1. npm ci 성공.
- 원본 실행 로그: `/tmp/agent-gibbon-is2-64/`. baseline.json 및 baseline-{regression,typecheck,lint,build}.log 보존.

## TDD

기준 트리에서 npm test, cargo check --locked --all-targets, Rustfmt/Clippy, npm run dev:build 모두 exit 0. baseline 제외 없음.

red.log: 기존 Timing을 수신/처리 두 스레드에서 실행하면 transaction 1개 기대에 실제 2개로 실패했다. 설정/컴파일 오류가 아니다. 테스트만 red 커밋으로 push한 후 Draft PR을 생성했다.

Green에서는 동일 trace·직접 부모/자식 ID·자식 종료 시각 이하가 아닌 루트 종료 시각의 assertion을 유지하고 새 부모 handle 캡처/진입 호출을 추가했다. 동의 gate를 검사하는 구현에 맞춰 테스트 프로세스의 메모리 gate도 허용/복원한다. 이는 실제 앱 동의 생성이나 네트워크 전송이 아니다. 이 테스트 호출부 변경을 독립 감사 대상으로 명시한다.

## 검증 범위

- in-memory SDK transport: IPC 대기/처리/중첩 span의 trace/parent 일치.
- 대기 종료 후 callback이 남는 경우 조기 루트 전송 방지, 마지막 취소 handle 해제, 자식 → 조상 종료 순서.
- 두 수신 스레드와 같은 UI 스레드에 해당하는 목적 스레드에서 요청별 부모 분리·기존 문맥 복원·panic 복원.
- 동의 철회 중 및 재활성화 후 이전 세대 handle 거부, 동의 전/상세 비활성의 빈 handle 격리, 새 요청 복구.
- sampled/unsampled 부모의 직접 자식 결정 상속, polling root 내부 sampler 입력·최대 1% 유지.
- 기존 envelope 원문/로컬 상관 ID 제거, 동의 transport loopback 회귀 유지.

현재 추가한 5개 경계 테스트와 red 테스트의 green, 최종 5종 게이트·독립 감사·플랫폼 Actions 결과는 아래 실행 결과에 기록한다.

## 한계와 보존

실제 Sentry 서버 전송·수신, 신규 동의 생성, 다른 기기 배포, 설치 앱 교체, 공개 태그·릴리스는 실행하지 않는다. GUI 조작·캡처를 하지 않으면 GUI 출처 확인을 주장하지 않는다. 다른 스레드 전달·WebView 개별 요청·snapshot 집계 제거·metrics·태그 일괄 개선은 범위 밖이다. 기존 main 파일·프로세스·동의는 변경하지 않는다.

UI 외형·상호작용 변경이 없어 [디자인 시스템](../design-system.md)의 기준과 시안은 유지한다. 약관/개인정보 고지·외부 필드·수집 목적·전송 허용 목록은 바꾸지 않아 legal 문서를 변경하지 않는다.

## 시도 1

- 신규 테스트 PASS (telemetry 12개), 회귀 PASS (Rust 174/UI 119), 타입 PASS.
- lint FAIL: Clippy `unnecessary_unwrap`, telemetry.rs의 epoch.is_some() 뒤 unwrap 1건. baseline에는 없던 신규 정적 오류다. 테스트/규칙 완화 없이 수정 후 재검증한다.
- 원본: gate1.json, gate1-*.log. 빌드는 gate1-build.log의 완료 결과를 대조한다.

## 최종 로컬 검사와 감사 이력

- 시도 1 fresh 감사: 테스트 완화 미발동, 스코프 이탈 미발동. 신규 Clippy 실패를 퇴행 발동으로 분류했다. 테스트의 context capture/enter 및 메모리 gate 호출 추가는 운영 경로와 일치하는 정당한 변경으로 판단했다.
- is2에 따라 격리 작업자 1회 escalation 실시. `if let (true, Some(epoch))` 두 줄 수정의 의미 보존을 확인했고 fmt·Clippy·telemetry 12개 PASS. 로그: escalation-{fmt,clippy,telemetry,review}.log.
- gate2: 신규/회귀/타입/린트/개발 빌드 모두 PASS. 이후 늦은 로그가 새 클라이언트로 재바인딩되는 경합을 막기 위해 시작 당시 hub/client를 span과 함께 보존했다. 이 보완에는 목적 스레드 client/scope가 루트 종료에 섞이지 않는 테스트를 추가했다.
- gate3: 신규 telemetry 13개 PASS, 전체 회귀 Rust 175/UI 119 PASS, cargo check --locked --all-targets PASS, Rustfmt·Clippy(-D warnings) PASS, npm run dev:build PASS. 원본: gate3.json, gate3-*.log. baseline 제외 0, 핵심 게이트 SKIP 0.
- 최초 퇴행 감지 이력을 clean으로 바꾸지 않는다. 최종 fresh 감사 결과는 PR에 기록한다. 검증 수준은 기능 자체의 in-memory/loopback 단위·통합 수준(Conf 3)이며, is2 정직성 의심 발동 보정 적용 시 최종 Conf 2다. 사전/최종 Risk Medium, 사람 리뷰 대기.

## 자체 native CLI 확인

- 자체 개발 인스턴스의 최초 프로세스 PID 72569를 정확한 binary `.../64-sentry-thread-context/src-tauri/target/debug/agent-gibbon --serve`와 대조한 뒤 그 PID만 종료하고 개발 CLI로 재시작했다. 새 PID 86514.
- gate3 빌드 binary SHA256 `d27c4342809208212e646f9fda058da6dd4a7c899e6708a3629eae62b0f71f67`. 빌드 내장 revision은 `3c9c2010d006`이며 당시 미커밋 보완 소스까지 포함한 gate3 빌드다. 내장 revision만으로 해당 보완 검증을 주장하지 않는다.
- `inspect --id is2-64-sentry-context`의 ok/instance/메모 본문을 확인했다. `update` 후 다시 inspect하여 위치·크기·가시성·포커스·핀·본문 유지 PASS. diagnostics의 sharing_allowed=false, terms_accepted=false, terms_skipped=true, enabled=false 확인. 동의 대리 생성·외부 전송 없음.
- **부가 확인 실패:** 재시작 전후 자체 진행 메모의 위치 보존 assertion은 실패했다(2026,1560 → 1748,494). 재시작/생성 시 재배치가 있었으며 baseline에서 이 assertion을 실행하지 않아 이번 변경의 회귀 여부는 미확정이다. 이는 update/inspect 상태 보존 PASS와 구분한다. 다른 사용자 메모나 앱은 조작하지 않았다. 재시작 위치 보존을 이번 기능의 통과 결과로 주장하지 않는다.
- 원본: native-before.json, native-after.json, native.json, native-server.log. GUI 조작/캡처와 native Sentry 활성 trace는 미실행.

## 전체 플랫폼

최종 squash 커밋을 push한 뒤 release.yml workflow_dispatch를 요청한다. 최신 headSha·macOS ARM64/Intel·Windows x64 결과와 실행 URL은 PR에서 갱신한다. 태그·공개 배포·설치본 교체는 하지 않는다.

## is2 커버리지 대조

현재 스킬 표의 Medium 백엔드, 기본 자동 제출, 충돌/감사 감지 경로와 이번 실행을 대조했다. 실제 실행은 기존 워크스페이스 인수·TDD·Clippy 퇴행 감지·1회 escalation·fresh 감사·Medium 리뷰 대기 경로다. 스킬 원본 유지보수는 요청 범위가 아니므로 공용 표를 수정하지 않으며, 이 기록과 PR을 이번 실사용 근거로 남긴다. 에이전트는 Codex(현재 환경, 세부 모델 버전 미노출)다.

## 최종 fresh 감사

`6ad8537bd72b1c3b26477c0d8efa0d2d29c82e73` 고정 커밋에 대해 구현 이력 미상속 검토자가 최초 base/red 전체 diff와 원본 로그를 대조했다. 테스트 완화·스코프 이탈 미발동, 기존 Clippy 퇴행 발동 이력 유지/최종 차단 없음, 결과 보고 적합, 새 설계 차단 결함 없음, Risk Medium으로 판정했다. 감사자는 테스트를 재실행하지 않고 보존 로그와 SDK/transport 원본을 확인했다.

늦은 `log_for` 검사 직후 철회·재활성화 경합을 강제로 발생시키는 테스트는 없다. 시작 client 고정 및 기존 transport epoch 구조를 코드로 검토했고, 원래 hub/client 보존·목적 스레드 문맥 분리와 기존 loopback 세대 차단은 각각 실행 검증했다. 이 한계는 실제 경합 재현 성공과 구분한다. 최종 squash는 감사한 소스·테스트를 그대로 유지하며, 감사 기록 문구만 추가한다.
