# #64 후속: snapshot 요약 trace 제거·자식 span 메타정보

## 범위와 출처

- 사용자가 기존 PR #89에 snapshot 요약 집계 문제 해결을 추가 승인했다. 메타정보/다른 스레드 항목 질문에 구현 상태를 설명하고 범위를 확인하는 질문을 제시했다. 권장 기본안(요약 제거 + 고정 span 메타정보)을 고지한 뒤 진행했다. 다른 스레드 경계 전체 연결은 확대하지 않았다.
- 기존 구현은 `d28ca3ff2ed852a6f3a9a8af8eccb5d749978abd`로 이미 커밋·push된 상태였으며 미커밋 변경이 없었다. 원래 base/red/attempts ref와 해당 SHA 전체 플랫폼 PASS 증거는 보존한다. 이 과거 PASS를 후속 변경의 PASS로 사용하지 않는다.
- 후속 기준은 `refs/is2/followup-base/64`, red는 `refs/is2/followup-red/64`로 분리 보존한다. 최초 is2 ref는 덮어쓰지 않는다. 기존 squash를 다시 최초 base까지 squash하지 않는다.
- 작업 루트 `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/64-sentry-thread-context`, 개발 instance `dev-ab5b8ab5f6f5201ea673e343`. 같은 개발 CLI만 사용하고 main/설치 앱/다른 워크트리/동의 상태는 변경하지 않는다.
- 원본 로그 `/tmp/agent-gibbon-is2-64/followup/`. baseline-{regression,typecheck,lint,build}.log 및 baseline.json: 기존 Rust175/UI119, 타입, fmt/Clippy, 개발 빌드 모두 exit0. 제외 없음.

## TDD와 경계

- red-ui.log: snapshot 호출의 결과·reject를 유지하면서 heartbeat 측정 배열 0개 기대에 실제 1개로 실패.
- red-rust.log: 최종 envelope의 snapshot 요약 차단 기대에 snapshot이 남아 실패. 자식 span app.os 기대에 None으로 실패. 설정/컴파일 실패가 아니다.
- 테스트만 red 커밋 `15b0a9371f6df4b16c59a65969d9d63b4a5cc171`으로 push하고 기존 PR을 Draft로 전환했다.
- 프런트 snapshot 계측 제거 + 백엔드 구버전 snapshot 샘플 무시 + 최종 허용목록 차단으로 방어한다. bounded legacy 입력 유효성 검사는 유지해 watchdog heartbeat가 중단되지 않게 한다. 개별 snapshot과 다른 작업의 WebView 집계는 유지한다.
- root/child의 OS·아키텍처·빌드 태그는 전송 직전 빌드 상수로 덮어쓴다. 원문·다른 태그·로컬 상관값은 기존대로 제거한다.
- 기존 `aggregate_tag_survives_but_local_correlation_never_leaves_envelope`의 fixture 작업명을 snapshot에서 save_note로 변경했다. 새 요구로 금지된 요약 대신 계속 허용되는 집계의 provenance/로컬 ID 제거를 검증하기 위한 정당한 fixture 변경이며 assertion을 삭제·약화하지 않았다. red의 새 assertion은 그대로 유지한다.
- green 후 구버전 snapshot의 transaction 미생성과 다른 측정의 계속 생성을 in-memory SDK transport에서 추가 확인한다. remote 진단 전송은 하지 않는다.

## 남은 연결 경계

CLI 수신 → UI callback의 explicit 부모 전달은 기존 구현으로 유지한다. 터미널 시작 worker, 캡처 worker/UI callback, 장기 output reader, WebView 개별 왕복 부모 연결은 미구현 범위로 남긴다. 기존 local_trace 연결을 Sentry 연결로 보고하지 않는다. 장기 reader의 부모 lease를 수명 전체 유지하는 방식은 루트 종료를 무기한 늦출 수 있어 적용하지 않는다.

## 디자인·동의

[디자인 시스템](../design-system.md)에 후속 적용 상태를 반영한다. 외형/배치 변경이 없어 시안 변경 없음. docs/legal/README에 기존 수집 범주 유지와 snapshot 왕복 요약 관측 감소를 기록하고 정책 본문/해시·동의 선택은 유지한다. 원격 Sentry 수신, 기기 필터 검색, 네이티브 진단 활성화는 이번 실행 증거에 포함하지 않는다.

## 로컬 게이트 결과

후속 gate.json: 신규 telemetry Rust16/UI7, 전체 회귀 Rust178/UI120, cargo check --locked --all-targets, Rustfmt·Clippy(-D warnings), npm run dev:build 모두 exit0. 원본 gate-{new,regression,typecheck,lint,build}.log. 후속 baseline 제외 0, 게이트 SKIP 0. green의 신규 red assertion은 그대로 유지했다.

변경 소스는 in-memory SDK envelope와 VM에서 검증했다. 기존 프로세스의 실행을 최신 후속 코드의 native 증거로 재사용하지 않는다. 후속 native 활성 진단·GUI/실제 Sentry 수신은 미실행이다. 미공개 자체 개발 바이너리만 빌드했으며 설치 앱·다른 기기·main·동의 상태를 변경하지 않았다.

최신 후속 커밋의 fresh 감사와 release.yml workflow_dispatch 전체 플랫폼 결과는 PR #89에 갱신한다. 위험도는 기존 연결 구현을 포함해 Medium을 유지하며 자동 병합하지 않는다. 원래 감사의 Clippy 퇴행 감지·해소 이력도 유지한다.
