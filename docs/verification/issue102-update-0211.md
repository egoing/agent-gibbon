# #102 두 번째 실제 업데이트: v0.2.10 → v0.2.11

## 목적과 출처

사용자가 패치 버전을 하나 올려 실제 업데이트를 다시 검증하도록 요청했다. 기능 변경 없이 npm·Cargo·Tauri와 lockfile의 앱 버전을 0.2.11로 올린다.

- 작업 루트: `/Users/egoinglee/.codex/worktrees/issue-102-update-verification/agent-gibbon`
- 브랜치: `codex/102-update-0211`
- 환경: macOS ARM64, 정식 설치 경로 `/Applications/Agent Gibbon.app/Contents/MacOS/agent-gibbon`.
- 시작 설치본: v0.2.10, revision `629f2c3958a6`, PID 81210, `instance:null`, updater enabled, phase idle.
- 기존 개발 진행 메모 인스턴스는 실행 중이지 않아 재표시하지 않고 대화로 진행 상황을 안내한다.
- 로컬 검사: Rust 206개·Node UI 169개, Rustfmt·Clippy·버전 일치·diff 공백 검사 통과. 원본 `/tmp/issue102-0211-local-tests.log`, `/tmp/issue102-0211-clippy.log`.

## 관찰 방법과 진행

앱 진단을 반복 조회하면 그 조회 자체가 설치 예약과 겹칠 수 있어, 상시 관찰은 앱 호출 없이 Info.plist·실제 프로세스 PID·공개 업데이트 캐시의 버전/크기를 읽는다. 최초 diagnostics로 실행 버전을 확인했고, 설치 후 새 프로세스가 뜨면 다시 diagnostics와 창 상태를 대조한다. 다운로드 후 자동 idle 설치 경로를 관찰하며, 실제 앱의 작업 보호를 우회하지 않는다.

모니터 `/tmp/issue102-0211-passive-monitor.py`, 기록 `/tmp/issue102-0211-passive-monitor.jsonl`. 시작 기록 2026-11-07 21:38:24 KST의 설치/캐시 버전은 모두 0.2.10, 캐시는 8,656,542바이트다.

- [x] v0.2.10 설치본의 버전·revision·PID 확인
- [x] v0.2.11 버전 변경과 로컬 검사
- [ ] 세 플랫폼 검사와 정식 v0.2.11 배포
- [ ] 실제 앱에서 새 버전 확인·다운로드·서명 검증
- [ ] 설치와 프로세스 교체를 연속 관찰
- [ ] v0.2.11 실행 및 메모 내용·창 상태 대조

현재 이 문서 작성 시점에는 새 릴리스와 실제 업데이트를 완료하지 않았다. [이전 실증](issue102-installed-update.md), [디자인 기준](../design-system.md)을 따른다. Intel·Windows 실제 업데이트와 모든 보호 조건 검증은 이 Mac의 결과와 구분한다.
