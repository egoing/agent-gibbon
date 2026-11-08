# #102 Apple Silicon 격리 updater 검증

상태: 수정된 설치 가드로 표준 시간 설정의 수동 업데이트·재시작·A/B 창 복원을 확인했다. 300초 자동 경로는 별도 실행에서 관찰했으나 실제 무입력 300초 계측값은 남기지 못했다.

## 장비와 분리

- MacBook Pro MacBookPro18,1, Apple M1 Pro, macOS 15.6.1 (24G90), `arm64`.
- 소스 워크트리 `/Users/egoinglee/.codex/worktrees/issue-102-update-verification/agent-gibbon`, 브랜치 `codex/102-updater-diagnostics`, HEAD `a418e86215db1d4eb520029709978e39e09aaa1d`.
- 시험 소스는 HEAD의 임시 복사본 `/tmp/issue102-arm64-e2e-20261108/source`다. 수정된 `ui/updater-guard.js` SHA-256 `974dd8e07bd7eca158ba6514061f576fc3b5874651622be73e426b504205a97b`, `src-tauri/src/updater.rs` SHA-256 `85481699198e780e4d05b2d968d4fab5c12819885b8978dd340e72fab64c1d22`를 워크트리와 임시 복사본에서 대조했다.
- 이전 v0.2.10과 후보 v0.2.11을 모두 `verification-updater` feature, `GIBBON_VERIFICATION_STANDARD_TIMING=1`, 같은 임시 키와 같은 사용자 지정 bundle ID로 새로 빌드했다. 이전 바이너리 SHA-256 `887e83b39404d8e0d3b954e55a1265999bd6eef741dfdd19ba72b4c1617ef17`; 후보 바이너리 SHA-256 `c52ef074a373032d6b0a4d7620f57272dae76b6b3f66bdbebdb5f85e299c460d`; 후보 updater 자산 SHA-256 `b5735774e2f7f0197789402e16cb05bf13a8ece1ed93ea8fabc749f756168685`.
- 설치 경로 `~/Applications/Agent Gibbon Verification.app`, bundle ID `org.egoing.floatnote.verification.issue102`, IPC instance `dev-001020010200102001020010`, 데이터 경로 `~/Library/Application Support/org.egoing.floatnote.verification.issue102/notes/dev-001020010200102001020010`. 공식 설치 앱의 프로세스·데이터는 변경하지 않았다.
- 피드 서버는 `127.0.0.1:18453`에만 바인딩했다. 필수 약관만 수락했고 선택 진단 공유는 꺼져 있다. 최종 진단 상태 `terms_accepted=true`, `terms_skipped=false`, `sharing_allowed=false`.

## 장애·보호 회귀

- 피드의 후보 자산을 한 바이트 변조해 실제 다운로드를 유발했다. 앱은 서명 검증 단계에서 거부했고 `last_check_failure=download`가 기록됐다. 원본 패키지를 복구한 뒤 수동 재확인하여 v0.2.11이 다시 `ready` 상태가 됐다. 원본 패키지 SHA-256은 `30d63e267d36db0089c8b397461d14be7699032e2d0f9f183359ec033a0357de`였다.
- 후보가 준비된 상태에서 피드 서버를 중단했다. 확인은 `last_check_failure=manifest`로 실패했지만 검증된 후보의 `ready` 상태와 7,427,567바이트가 유지됐다. 로컬 서버 복구 후 manifest·패키지 요청이 다시 성공했다.
- 미저장 편집 중 수동 설치는 `protected`로 보류됐다. 편집 보고가 `editing=true`였고 저장된 A 노트는 `issue102-arm64-a-original-sentinel` 그대로였다. 이후 같은 검증에서 추가 수동 시도는 두 숨은 메모 창의 `poll_fold`, `poll_warp`, `snapshot` 요청이 pending으로 남아 `pending_request`가 됐다. 보고 epoch 2와 3에서 각 창 pending 3건이 확인됐다.
- 설치 가드 수정 후 숨은 창의 정상 background 요청을 drain하는 새 경로를 반영해 재빌드했다. 수정 빌드의 실제 수동 설치에서는 모든 창 ACK가 `true`, 각 보고 `pending=0`으로 기록됐다.

## 수정 빌드 수동 설치와 복원

- fresh 시작 전 격리 설치 번들을 v0.2.10으로 교체하고 `.update-cache`, `.update-notice.json`, 이전 `.last-update-attempt.json`을 각각 임시 백업했다. 버전별 알림 중복 방지 기록을 제거해 후보가 준비되면 알림이 다시 만들어지게 했다. 동일한 검증 데이터와 A/B 노트 본문은 유지했다.
- 수동 버튼 직전 기록 [fixed-run-preclick.json](</tmp/issue102-arm64-e2e-20261108/fixed-run-preclick.json>): 시각 2026-11-08 19:53:20 KST, OS `HIDIdleTime=159.10초`(설정 임계값 300초), 현재 v0.2.10, 후보 v0.2.11 `ready`. 이 값은 자동 설치 조건보다 짧다.
- 테스트 번들 PID 50952의 실제 AX 창 제목 `Agent Gibbon · 업데이트`에서 활성화된 `업데이트하고 재시작` 버튼을 PID 귀속 AXPress로 실행했다. 직후 v0.2.10 프로세스가 종료되고 v0.2.11 프로세스 PID 52627이 시작됐다. 설치된 실행 파일 해시가 수정 후보 바이너리 해시와 같고 진단 상태는 `current_version=0.2.11`, `phase=idle`, `message=최신 버전입니다`다.
- 설치 준비 기록 [`.last-update-attempt.json`](</Users/egoinglee/Library/Application%20Support/org.egoing.floatnote.verification.issue102/notes/dev-001020010200102001020010/docs/.last-update-attempt.json>)은 epoch 1, `prepared`, source `0.2.10`, target `0.2.11`이다. 가시·숨김 메모 창과 stash를 포함한 5개 응답이 전부 ACK됐으며 pending 요청은 없다.
- 직전 A/B 스냅샷과 재시작 뒤 스냅샷은 [fixed-run-pre-a.json](</tmp/issue102-arm64-e2e-20261108/fixed-run-pre-a.json>), [fixed-run-pre-b.json](</tmp/issue102-arm64-e2e-20261108/fixed-run-pre-b.json>), [fixed-run-post-a.json](</tmp/issue102-arm64-e2e-20261108/fixed-run-post-a.json>), [fixed-run-post-b.json](</tmp/issue102-arm64-e2e-20261108/fixed-run-post-b.json>)이다. 각 노트의 본문·위치·크기·표시 여부·상단 고정·접힘 상태가 전후 일치했다. A는 `(1638,1024)`, `600×96`, 표시, 상단 고정, 펼침 상태와 원본 sentinel 본문을 유지했다. B는 `(1748,494)`, `600×480`, 숨김, 상단 고정, 펼침 상태와 숨김 sentinel 본문을 유지했다. A의 `animating`만 설치 직전 `true`에서 재시작 뒤 `false`로 끝났다.
- 설치 후 진단 스냅샷은 [fixed-run-post-diagnostics.json](</tmp/issue102-arm64-e2e-20261108/fixed-run-post-diagnostics.json>)이며, UI 접근 후 시험 앱 창 제목은 `Agent Gibbon [DEV Updater verification · 00102001] — main`으로 확인했다.

## 자동 대기와 범위

- 별도 fresh v0.2.10 실행에서는 기다리는 동안 v0.2.11로 자동 교체된 사례가 한 번 있었다. 300초 임계값 및 앱의 “5분 동안 조작하지 않으면 자동 설치” 안내와 일치하지만, 그 교체 순간의 `HIDIdleTime`을 기록하지 못했으므로 정확한 300초 실증으로 세지 않았다.
- 앞선 수동 시도에서는 직전 입력부터 71.44초, 수정 빌드의 성공 클릭 직전에는 159.10초를 계측했다. 두 값 모두 300초 미만이다. 처음 실행했던 약 100초 모니터에서는 연속 300초 무입력이 발생하지 않았다.
- 운영 앱 화면에서 관찰된 `웹 창 세션` 보류 안내는 제목에 DEV 표시가 없어 검증 앱 창으로 귀속하지 않았다. 최종 시험은 사용자 지정 bundle ID와 정확한 검증 PID로 열린 updater 알림에서 수행했다.
- 결과 파일과 서명 키는 로컬 `/tmp/issue102-arm64-e2e-20261108`에만 두었다. 공개·커밋·배포하지 않았다. 로컬 테스트 피드 종료와 임시 자원 정리는 최종 결과 기록 후 수행한다.

공통 업데이트 UX 기준은 [디자인 시스템](../design-system.md), 전체 계획은 [issue 102 실행 체크리스트](issue102-live-checklist.md)를 참조한다.
