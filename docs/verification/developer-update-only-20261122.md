# 개발자 업데이트 시험 전용 검증 — 2026-11-22

현재 범위의 최종 검증 기록이다. 과거 분리 통합/시간제한 기록은 당시 증거로 보존하며 이 문서를 우선 참고한다.

- 루트: `C:\Users\egoin\.codex\worktrees\5170\agent-gibbon`
- 브랜치: `codex/developer-update-test`
- 통합 기준 main: `f15d8129388f1cdeec73bf5b1313b15d79dc3be9` (PR #280)
- 실제 시험한 앱 코드: `bdf527296451d11975ad37cc92102769ee084bc7`. 이후 변경은 검증 근거와 문서 링크뿐이다.
- 범위: [업데이트 시험 #275](https://github.com/egoing/agent-gibbon/issues/275). 분리는 [#279](https://github.com/egoing/agent-gibbon/issues/279)에 유예, 과거 전환 인과 미확정은 [#278](https://github.com/egoing/agent-gibbon/issues/278)에 분리한다.

## 코드 경계

[범위 대조](developer-update-only-20261122/scope-audit.json): `terminal.rs`, `terminal/lifecycle.rs`, `terminal/persistence.rs`, `update_restore.rs`는 위 main과 동일하다. 삭제된 `windows_owner.rs`, `persistence_mode.rs`, PTY 시험 스크립트/테스트를 재도입하지 않았다. 분리 transport 재시도·수신 복구·worker/gate 진단과 opt-in 상태는 제거했다.

메뉴의 `terminal_persistence_status/enable` 호출과 상태 표시를 제거했다. 5회 클릭은 메뉴만 표시한다. 실행·시작·정리 중 일반 터미널과 숨긴 Welcome의 기존 보호를 유지한다. 서명·플랫폼·버전 바인딩, 캐시·수동 대기, 모든 창 ACK 및 편집·저장·캡처 보호를 유지한다.

공개된 복귀 가능 공식 버전 쌍이 없어 일반 빌드의 왕복 두 버튼은 비활성이다. 실제 시험은 동일 소스/데이터 형식/메뉴와 별도 서명키·제품·설치경로·고유 namespace를 가진 `verification-roundtrip` 쌍 0.3.12/0.3.13으로 수행했다. 정식 피드나 릴리스 자산은 수정하지 않았다. 실제 패키지 버전값과 signed version/파일명 바인딩을 사용했으며 서명/작업 보호 우회는 없다.

## 검사와 최종 개발 빌드

- Rust 318 통과, UI 257 통과/기존 2 건너뜀: [로그](developer-update-only-20261122/tests.log).
- Clippy all-targets 경고 없음: [로그](developer-update-only-20261122/clippy.log).
- UI 회귀: 5회 클릭/새로고침이 terminal API를 호출하지 않음, 느린 클릭 초기화, 공개 쌍 부재, 수동 준비와 설치 분리, 보호 보류 이유, 일반 캐시의 시험 설치 거부.
- 최종 `npm run dev:build` 1분9초 성공: [로그](developer-update-only-20261122/dev-build.log). [dev:info](developer-update-only-20261122/dev-info.log)와 개발 실행기를 통한 [instance-info](developer-update-only-20261122/instance-info.json)의 root/id/manifest_dir 확인. 삭제된 CLI 옵션은 [사용법 오류로 거부](developer-update-only-20261122/retired-cli.jsonl)됐다.
- 개발 binary: `C:\Users\egoin\.codex\worktrees\5170\agent-gibbon\src-tauri\target\debug\jajan.exe`, instance `dev-0e8f09996ee653e1d6855ad6`. 이 개발 GUI를 실행 중이라고 주장하지 않으며 아래 실설치는 별도 검증 release다.
- [디자인 기준](../design-system.md)과 [SVG](../design/developer-updater.svg)/[PNG](../design/developer-updater.png)에서 분리 상태 행을 제거하고 렌더링 확인.

## 전용 설치 출처

[설치 계획·소스/패키지 해시](developer-update-only-20261122/installation-plan.json), [일회성 공개키](developer-update-only-20261122/verification-public.key), [패키지 빌드 로그](developer-update-only-20261122/packages.log)를 보존했다. 비밀키는 준비 프로세스 메모리에만 생성하고 기록하지 않았다.

- 작업공간: `C:\Users\egoin\AppData\Local\Temp\jajan-roundtrip-5170-5bxQe6`
- 제품: `Jajan Roundtrip Verification 5170 efb2e8be`, identifier `app.jajan.roundtrip5170.efb2e8be`
- 실제 설치 binary: 위 작업공간의 `installed\jajan-roundtrip-5170-efb2e8be.exe`
- instance: `dev-efb2e8be996701d1c17d2a25`; 관리 창 제목 `Jajan [DEV Updater verification · efb2e8be] — 메모 관리`
- 경로가 새 작업공간 내부이며 제품 등록/대상 경로가 없고 패키지 SHA-256이 일치함을 확인한 뒤 `/S /NS /D=<전용 경로>` 최초 설치, 종료코드0. [최초 실행](developer-update-only-20261122/initial-install.json) PID10700/실제0.3.13과 [프로세스 출처](developer-update-only-20261122/gui-provenance.json) 확인.
- 실제 CLI `inspect` 응답의 위 instance, 메모 `update-only-proof-5170`과 고유 sentinel을 확인했다. Windows Computer Use의 반환 창을 정확한 exe 경로/네이티브 제목으로 선택하고 재시작마다 다시 선택했다.
- 초기 필수 약관 확인 후 [진단 상태](developer-update-only-20261122/initial-diagnostics.json)의 `sharing_allowed=false`, SDK `enabled=false`, `terms_accepted=true` 확인. 검증 앱의 자동 시작은 unavailable이며 OS에 등록하지 않았다.

## 실제 UI·설치 결과

| 검증 | 실제 결과와 근거 |
|---|---|
| 숨김 진입 | 하단 버전 5회 클릭 뒤 Privacy & Diagnostics 아래 개발자 항목 노출. 분리 행 없음. 0.3.13에서는 이전 버튼 활성/최신 버튼 비활성, 0.3.12에서는 반대. 재시작 후 다시 5회 클릭으로 진입 가능. |
| 동일 버전 준비 | 실제 현재/준비 모두0.3.13, 5,363,212 bytes, 정상 서명/플랫폼/버전·캐시 완료, ‘동일 버전 재설치 테스트’ 표시. [상태](developer-update-only-20261122/prepared.json)·[화면](developer-update-only-20261122/prepared.png). |
| 준비만 한 자동 설치 차단 | 실제 Windows 무입력349.516초(표준300초+retry15초 초과)에도 PID10700/0.3.13/ready/manual_test/캐시 유지. [측정](developer-update-only-20261122/prepared-idle-held.json)·[동일 프로세스](developer-update-only-20261122/idle-process.json). 입력 시각을 조작하지 않았다. |
| 동일 버전 재설치 | 명시적 ‘준비한 버전 설치·재시작’으로 실제 NSIS교체/재시작, PID10700→2228/실제0.3.13. [ACK](developer-update-only-20261122/after-same-diagnostics.json)는 source/target0.3.13, 창8개 모두true. [메모 비교](developer-update-only-20261122/same-comparison.json) 본문/revision/위치/크기/표시 보존. |
| 이전 호환 버전 | 실제 ‘이전 테스트 버전으로 교체’ 클릭으로 PID2228→2408, 실제0.3.13→0.3.12. [프로세스](developer-update-only-20261122/after-down-process.json)·[ACK8개true](developer-update-only-20261122/after-down-diagnostics.json)·[이전 화면](developer-update-only-20261122/previous-ready.png). |
| 재시작 후 자동 복귀 차단 | 최신 피드가0.3.13을 제공하지만 실제 무입력324.375초에도 같은PID2408/0.3.12/manual_test/roundtrip_active 유지. [측정](developer-update-only-20261122/previous-idle-held.json), [별도 시험 대기 파일](developer-update-only-20261122/previous-trial-state.json), [수동 캐시 manifest](developer-update-only-20261122/previous-cache-manifest.json). |
| 최신 복귀 | 실제 ‘최신 버전으로 복귀’ 클릭, PID2408→2492/실제0.3.12→0.3.13. [ACK8개true](developer-update-only-20261122/after-return-diagnostics.json). 최신 실행/서명된 쌍을 확인한 뒤에만 manual_test/roundtrip_active=false, phase=idle, [별도 대기 파일 제거](developer-update-only-20261122/after-return-process.json), 일반 정책 재개. |
| 양방향 데이터·안정 좌표 | 설치 전 두 시점의 animating=false/layout_pending=false와 같은 좌표를 확인했다. [하향 비교](developer-update-only-20261122/down-comparison.json)·[복귀 비교](developer-update-only-20261122/return-comparison.json): 본문/revision1/visible=true/위치(86,130)/크기634×146 전부 유지. |
| 미저장·저장 실패 보호 | 별도 시험 파일만 exclusive lock하고 편집하여 실제os error32 저장 실패/[초안 유지](developer-update-only-20261122/save-failed.png)를 만들었다. 설치 요청은 ‘편집·저장 중인 창이 있어 설치를 보류했습니다’. [ACK](developer-update-only-20261122/save-protected.json)에서 memo editing=true/ack=false, 0.3.13 유지. 잠금 해제 후 화면 ‘다시 저장’으로 [revision2 저장 복구](developer-update-only-20261122/save-recovered.json). |
| 일반 실행 터미널 보호 | 메뉴 진입 후에도 일반 native 셸 PID32596 실행, [restore_supported=false](developer-update-only-20261122/native-terminal-running.json). CLIclose로 [숨겼지만running=true](developer-update-only-20261122/native-terminal-hidden.json). 실제 이전 설치 요청은 ‘터미널 세션을 종료한 뒤 설치할 수 있습니다. 숨긴 터미널도 포함됩니다’로 보류. [상태](developer-update-only-20261122/terminal-protected.json)·[화면](developer-update-only-20261122/terminal-protected.png), GUI2492/셸32596 보존. |

[행위/PID/현재·대상 버전 로그](developer-update-only-20261122/updater-events.jsonl)와 [feed 접근 기록](developer-update-only-20261122/feed-events.jsonl)을 함께 보존했다. 명령 입력/사용자 메모 원문을 추가 수집하지 않았다. 검증 메모의 원문은 이 작업에서 생성한 sentinel과 저장 시험 문자열뿐이다.

## 실패·제한과 종료 상태

- 대기 측정의 첫 Python 실행은 Windows기본cp949 decode 오류로 결과 저장에 실패했다. UTF-8지정 후 입력 없이 즉시 재측정해349.516초/상태를 저장했다. 제품 실패가 아니다.
- UI scroll은 화면 이동이 없어 Tab 탐색을 사용했다. 저장 메모 리사이즈 뒤 한 클릭이 실제 bounds 밖이라 거부됐고, fresh screenshot으로 재관찰해 올바른 ‘다시 저장’ 버튼을 눌러 복구했다. 실패를 성공으로 처리하지 않았다.
- 의도한 보호 실패(저장 실패/미저장ACKfalse/실행터미널)는 모두 설치 보류로 확인했다. 보호 조건을 완화하지 않았다.
- macOS 실설치/UI 및 공개 정식 호환 쌍 왕복은 미실행이다. 공개 쌍이 없는 기본 빌드에서는 왕복 버튼 비활성 유지. 임의 과거 공식 버전 설치/서명 우회/사용자 설치 앱 교체는 수행하지 않았다.
- 일반 자동 업데이트의 상태/캐시 정책과 보호 회귀는 단위·통합 검사로 확인했다. 별도 일반 자동 신규버전의 무인 실설치는 이번 수동 시험에서 반복하지 않았다. 캡처·정리 중 worker 등은 main의 변경 없는 보호/기존 테스트 범위이며 이번 GUI에서 별도 재현하지 않았다.
- 사용자 앱·이전 시험 앱·기존 셸/owner는 조작하지 않았다. 현재 남은 것은 이번 전용 GUI2492/숨긴일반셸32596이며 임의 종료하지 않았다. 이 전용 앱은 보호 시험으로0.3.12패키지가 준비된 수동 대기 상태다. 자동 설치는 예약하지 않는다.
- 이번 loopback feed64342만 출처 확인 후 [종료](developer-update-only-20261122/feed-stopped.json)했다. 최종 시점 cargo/rustc/makensis 실행 없음. 공개피드/릴리스키/태그/정식설치본은 변경하지 않았다.
- PR #276은 검증 결과를 반영해 Ready 보고한다. **main 병합/릴리스는 이번 단계에서 수행하지 않으며 팀장 순서 배정을 기다린다.**

Conf **5** (Windows 전용 실제 서명 NSIS/UI 흐름), Risk **Medium** (설치/재시작과 영속 캐시 상태). 미실행 플랫폼과 공개 호환 쌍을 이 확신도에 포함하지 않는다.

## 실행

위 작업 루트에서 `scripts/jajan-dev.cmd open`으로 최신 개발 앱을 연다. 이 명령은 자동 재빌드하지 않는다. `npm run dev`는 변경 감시 실행이다. 개발 실행 파일/인스턴스는 `dev-info.log`에 기록했다. 위 격리 설치 앱과 개발 앱은 별도 인스턴스다.

새 격리 쌍을 다시 준비해야 할 때만 Node22.13이상 환경에서 `node scripts/prepare-roundtrip-verification.mjs`를 사용한다. 새 고유 작업공간/제품/instance를 만들며 설치는 자동 수행하지 않는다. 종료한 기존 feed를 다른 앱/기존 trial로 임의 대체하지 않는다.
