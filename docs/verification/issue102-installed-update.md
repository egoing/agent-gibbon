# #102 실제 설치 앱 업데이트 검증

> **최신 후속 결론:** v0.2.9에서 받은 공식 업데이트가 반영되어 현재 v0.2.10(revision `629f2c3958a6`, PID 80987)이 실행 중이다. 설치 전후 main·검증 메모 두 개의 기하학·가시성·핀 및 검증 본문이 일치한다. 아래 설치 보류 기록은 이전 시점의 관찰이며 영구 설치 실패로 해석하지 않는다. 자동 재시도인지 추가 사용자 버튼 동작인지는 사용자 확인과 실행 로그의 범위 내에서 구분한다.

## 범위와 출처

- 사용자 요청: 새 워크스페이스에서 작은 변경 후 정식 릴리스, 이 컴퓨터의 실제 앱 업데이트 확인.
- 작업 루트: `/Users/egoinglee/.codex/worktrees/issue-102-update-verification/agent-gibbon`
- 브랜치: `codex/102-update-verification`; 개발 ID: `dev-48fc08799f16fff8bfceaf8a`.
- 환경: macOS 15.6.1 (24G90), Apple Silicon ARM64, Node v24.21.0, Rust 1.91.1.
- 기존 실행 PID 14633은 다른 워크트리 개발 앱으로 유지한다. 정식 설치 앱이 없어 공식 v0.2.9를 최초 설치했다.
- 대상: 운영 서명 키를 포함한 정식 v0.2.9 → v0.2.10. 개발 앱이나 수동 검사 artifact를 실제 업데이트 증거로 쓰지 않는다.
- 작은 변경: 최초 수동 설치 안내의 0.2.8 URL 결함 누락을 바로잡고 0.2.10으로 버전 필드를 일치시킨다. main에 병합된 웹 창 기능도 새 릴리스에 포함된다.

## 최초 설치

`gh release download v0.2.9 --repo egoing/agent-gibbon --pattern 'Agent.Gibbon_0.2.9_aarch64.dmg' --pattern SHA256SUMS.txt --dir /tmp/agent-gibbon-102-bootstrap`.

DMG SHA-256 `cae038cd3bfaf8b106c314f13ec90c7b93cbe83e566a7082b4098c636b74330d`는 공개 자산 digest와 일치한다. hdiutil 자체 체크섬 확인 후 `/Applications/Agent Gibbon.app`에 복사했다. 설치된 앱은 없었으므로 기존 앱을 덮어쓰지 않았다.

실행 PID 64758의 경로 `/Applications/Agent Gibbon.app/Contents/MacOS/agent-gibbon`를 ps와 sample로 대조했다. diagnostics 응답은 `instance:null`, `updater.enabled:true`, `current_version:0.2.9`, 빌드 revision `1e7b30eb31a2`다. 약관은 미수락(`onboarding_required:true`), 선택 진단은 비활성이다. 동의 파일을 대신 만들거나 진단 전송을 켜지 않는다.

네이티브 관리 창은 PID 64758, window ID 45920, 제목 `Agent Gibbon — 메모 관리`로 식별했다. Orca 접근성 조회는 두 번 시간 초과했고 UI 검증 통과로 계산하지 않는다. sample의 main thread는 일반 이벤트 루프 대기여서 이 결과만으로 앱 교착을 단정하지 않는다.

## 진행 상태

- [x] 새 워크스페이스와 버전·플랫폼·설치본 출처 확인
- [x] 공식 v0.2.9 최초 설치와 updater 활성 상태 확인
- [x] Rust 206개·Node UI 169개, Rustfmt·Clippy·버전 일치·diff 공백 검사와 개발 빌드 통과
- [x] 작업 커밋·전체 플랫폼 검사와 정식 v0.2.10 공개
- [x] 실제 확인·다운로드·서명 검증·안내와 편집 보호 검증 (설치 보류 현상 발견)
- [x] 새 프로세스의 v0.2.10 실행 및 설치 전후 메모 상태 일치 확인

이 기록 작성 시점에는 업데이트 전 과정 성공을 확인하지 않았다. Intel Mac·Windows의 실제 설치 업데이트, 자동 idle 설치와 버튼 설치 양쪽, 오프라인·변조·중복 요청의 실앱 검증은 별도 범위이며 미실행 항목을 구분한다. 디자인 기준은 [디자인 시스템](../design-system.md)의 자동 업데이트 절을 따른다.

로컬 로그: `/tmp/issue102-local-test.log`, `/tmp/issue102-ui.log`, `/tmp/issue102-clippy.log`. 실제 배포 전이라 원격 검사·실제 업데이트 성공으로 확대 해석하지 않는다.

개발 CLI 확인: `npm run dev:info`의 root/id/binary와 `scripts/agent-gibbon-dev inspect --id issue-102-release-progress` 응답의 `ok:true`, `instance:dev-48fc08799f16fff8bfceaf8a`, 메모 ID·본문을 대조했다. 개발 빌드 로그는 `/tmp/issue102-dev-build.log`이며 진행 메모를 설치 앱과 분리했다.

## 사용자 확인 후 사전 상태

사용자의 약관 확인 완료와 커밋·push·릴리스 계속 승인을 받았다. 설치 앱 diagnostics에서 `terms_accepted:true`, `terms_skipped:false`, `onboarding_required:false`, `updater.enabled:true`, 현재 버전 0.2.9, phase idle(최신 버전)을 확인했다. `sharing_allowed:true`도 관찰했으며 이는 사용자가 앱에서 선택한 상태다. 에이전트는 동의 설정을 변경하지 않았다.

## 정식 릴리스 — 완료

- PR [#103](https://github.com/egoing/agent-gibbon/pull/103), 작업 커밋 `7facb76edec5822a9ca3dc36bc43d794f389bc09`.
- [수동 전체 검사 42358525160](https://github.com/egoing/agent-gibbon/actions/runs/42358525160): 해당 작업 SHA의 macOS ARM64·Intel·Windows x64 테스트·Rustfmt·Clippy·설치 파일 생성·서명 자산 조립 모두 success. 완성 artifact 40,294,682바이트와 플랫폼 artifact 세 개를 확인했다. Intel 캐시 저장이 오래 걸렸으며 완료 전에는 전체 성공으로 기록하지 않았다.
- 병합·태그 SHA `629f2c3958a623ca66b866e99a59b8a307a1903f`. `/Users/egoinglee/dev/project/agent-gibbon`의 main을 fast-forward하고 origin/main과 SHA 일치를 확인했다. 기존 미추적 `docs/verification/sentry-dev-deploy-20261107.md`는 보존했다.
- [v0.2.10 정식 릴리스](https://github.com/egoing/agent-gibbon/releases/tag/v0.2.10), [태그 실행 42359683524](https://github.com/egoing/agent-gibbon/actions/runs/42359683524): 위 병합 SHA의 세 플랫폼 검사·운영 키 서명·assemble·publish가 모두 success.
- `/tmp/issue102-public-assets`에 실제 공개 자산을 내려받아 모든 SHA256SUMS 항목을 검증했다. 저장소의 공개 키와 플랫폼별 `.pub`를 대조하고 `prepareAssets`로 실제 Ed25519 payload·trusted comment 서명 및 서명된 버전/파일명을 검증했다. 재생성 manifest/checksum은 공개본과 정확히 일치했다. manifest에 있는 세 플랫폼 updater URL은 모두 HEAD HTTP 200이었다.
- 원본 근거: `/tmp/issue102-ci-dispatch-result.json`, `/tmp/issue102-release-run.json`, `/tmp/issue102-release-metadata.json`, `/tmp/issue102-public-assets/`.

## 실제 앱 검증 — 잠금 해제 대기

설치 앱의 네이티브 화면은 Orca 접근성 조회 실패 후 Codex CUA로 조회·조작할 수 있었다. 업데이트 설정에서 `현재 버전 0.2.9`, `최신 버전입니다`, 비활성 설치 버튼을 확인했다. 이는 새 릴리스 공개 전의 정상 상태다.

`issue102-visible-20261107` 메모의 네이티브 창 제목과 화면 본문의 `issue102-production-029-to-0210` 표식을 CLI inspect(`instance:null`)와 대조했다. 편집 화면에 `미저장 보호 검증용 초안: issue102-unsaved`를 입력했다. CUA 인라인 캡처로 초안을 확인했다. 이 초안은 검증용이며 기존 사용자 메모의 내용을 바꾸지 않았다. 비교용 숨김 메모는 `issue102-hidden-20261107`, 본문 표식은 `issue102-hidden-production-20261107`이다. 원본 inspect는 `/tmp/issue102-visible-before.json`, `/tmp/issue102-hidden-before.json`에 있다. 편집 후 숨김·다시 열기를 수행했으므로 실제 설치 직전에 기하학 상태를 다시 기록해야 한다.

정식 릴리스 공개 후 CUA가 **Mac 잠김, 자동 잠금 해제 불가**를 보고했다. 사용자에게 직접 잠금 해제 및 설치 앱의 메모 관리 열기를 요청했다. CLI diagnostics도 `앱 응답 시간이 초과되었습니다`로 실패했다(`/tmp/issue102-installed-before-update.json`). 이를 업데이트 실패나 앱 교착의 확정 증거로 확대하지 않는다. GUI 잠금을 우회하거나 동의·업데이트 캐시를 조작하지 않았다.

**남은 실행:** 잠금 해제 후 실제 앱의 지금 확인 → 다운로드·서명 검증 → 미저장 편집 중 설치 보류 → 검증용 초안 정리 → 설치·재시작 → 실제 실행 버전/빌드 revision과 메모 내용·위치·크기·고정·숨김 대조. 안내창 포커스와 반복 억제, 터미널 등 다른 보호 조건·자동 idle 설치·다른 플랫폼 실제 업데이트는 이 단계의 성공만으로 완료 처리하지 않는다. #102는 열어 둔다.

현재 확신도는 로컬·CI 자동 검사 범위 Conf 3, 실제 업데이트 전 과정은 미검증이다. 실제 앱 교체·재시작의 영향 위험도는 Medium이다.

## 잠금 해제 후 실제 앱 결과 — 설치 보류 재현

사용자가 설치 앱의 메모 관리를 열었다고 알려 준 뒤, 같은 설치 프로세스 PID 64758·공식 v0.2.9·revision `1e7b30eb31a2`·`instance:null`를 다시 확인했다. 앞서 잠금 중 실패한 CLI도 다시 정상 응답했다. 따라서 잠금 중의 시간 초과만으로 교착을 확정하지 않는다.

1. 메모 관리 → 설정 → 자동 업데이트에서 **지금 확인**을 눌렀다. 실제 앱이 v0.2.10을 발견하고 8,656,542바이트를 다운로드했다. diagnostics는 `downloaded == total == 8656542`, `phase:ready`, `version:0.2.10`이었다(`/tmp/issue102-update-download.json`). 코드의 ready 전환은 서명 검증과 캐시 저장 이후이므로 다운로드·운영 서명 검증 완료의 실제 실행 근거다.
2. `Agent Gibbon · 업데이트 준비 완료` 창에 새 버전 0.2.10, 설치 보류 사유, **나중에**·**업데이트하고 재시작** 버튼을 확인했다. 처음 관찰한 사유는 접힌 메모/크기 조정이었다. 검증 메모의 inspect에서도 `folded:true`를 확인했다. 정확한 표시 직후의 OS 포커스는 사용자 조작과 분리하지 못해 비포커스 요건 전체 통과로 처리하지 않는다.
3. 안내의 설치 버튼을 누르자 **편집·저장 중인 창이 있어 설치를 보류했습니다**가 표시됐고 PID 64758은 유지됐다. 나중에로 안내를 닫고 검증 메모를 펼쳤을 때 `issue102-unsaved` 미저장 초안이 그대로 남아 있었다. 이는 초안을 강제 종료하지 않은 실제 보호 근거다.
4. Escape로 검증용 초안만 취소했다. 원래 본문과 활성 편집 버튼을 확인하고, 펼쳐진 상태에서 메모 관리의 설치 버튼을 눌렀으나 같은 편집·저장 보류가 발생했다. 잠시 뒤 재시도하고 프라이버시 패널 상태를 확인한 뒤 돌아와도 같았다. 프라이버시 설정 자체는 변경하지 않았다.
5. 검증 메모의 **메모 창 닫기**로 해당 WebView를 종료하고 CLI open으로 새 창을 만들었다. 원래 본문이 유지됐고 편집하지 않은 새 창·`folded:false`였지만 설치는 다시 같은 메시지로 보류됐다. 기본 main도 원문 표시 상태와 활성 편집 버튼을 확인했다. **초안 정리 이후 최소 네 번의 설치 요청이 동일하게 보류됐다.** 강제 종료·보호 조건 우회·수동 새 버전 덮어쓰기를 실행하지 않았다.
6. 최종 diagnostics는 현재 버전 0.2.9, 대상 0.2.10, phase ready였으며, 설치 프로세스 PID 64758과 revision도 그대로였다(`/tmp/issue102-final-installed-diagnostics.json`). 따라서 **실제 설치·새 프로세스 재시작·v0.2.10 실행은 미완료**다. 실패 범위를 다운로드/서명 검증으로 확대하지 않는다.

### 원인과 한계

실행 근거로 확정할 수 있는 것은 설치 사전 검사에서 하나 이상의 WebView가 `safe:false`를 응답해 설치가 거절됐다는 범위다. Rust의 refused 분기와 UI 문자열이 대응한다. 어느 창/조건이 false를 보냈는지는 현재 diagnostics가 기록하지 않으므로 확인하지 못했다.

코드상 프런트엔드 가드는 편집·저장뿐 아니라 진행 중 IPC와 프라이버시의 미저장 상태도 함께 검사한다. 일반 메모·Feedback의 주기적 조회가 진행 중 IPC로 계산될 가능성을 조사 후보로 확인했지만, 이번 실행에서 그 후보를 원인으로 입증하지 않았다. 원인 확정 없이 가드를 완화하거나 설치를 강제하지 않는다. 차단 창과 이유를 관찰할 수 있는 안전한 진단을 추가해 별도 재현하는 후속 조사가 필요하다.

`/tmp/issue102-final-before-install.json`은 창 재생성 후 설치 직전 main·검증 메모 두 개의 위치·크기·보임·핀·펼침과 검증 본문 기준이다. 설치 자체가 되지 않아 재시작 복원 통과로 계산하지 않는다. GUI 캡처와 AX 증거는 대화의 CUA 인라인 결과에 있으며, 해당 도구는 로컬 캡처 경로를 반환하지 않았다. 기존 Orca 캡처를 현재 실증 자료로 대신 사용하지 않았다.

당시 판정(아래 후속 결과로 갱신): **그 시점에는 다운로드·서명 검증·준비 안내·미저장 초안 보호를 확인했으나 설치 요청이 보류되어 새 버전 실행은 확인하지 못했다.** #102를 닫지 않는다. 안내창 반복 억제의 장시간 확인, 전 과정의 포커스 보존, PTY·다른 보호 조건, 자동 idle 설치, 오프라인·변조·중복 요청 및 Intel·Windows의 실제 업데이트는 미검증이다.

확신도: **Conf 5 (macOS ARM64 설치 보류 현상과 초안 보존 실증에 한함)**. 위험도: **Medium (앱 교체·재시작과 작업 상태에 영향)**. 전체 업데이트 성공에 Conf 5를 부여한 것이 아니다.

## 후속 조사 — v0.2.10 실행과 메모 복원 확인

사용자의 후속 조사 요청 직후 설치 앱을 다시 조회했다. 이전 설치 요청 당시와 달리 `current_version:0.2.10`, `phase:idle`, 메시지 `최신 버전입니다`, revision `629f2c3958a6`, `instance:null`이었다. 앱 내 설정의 자동 업데이트 패널도 현재 버전 0.2.10과 비활성 설치 버튼을 표시했다. `/Applications/Agent Gibbon.app/Contents/Info.plist`의 버전도 0.2.10이다.

실행 파일은 동일한 정식 설치 경로이며, 새 PID는 **80987**, 프로세스 시작은 **2026-11-07 19:48:41 KST**다. 이전 PID 64758은 더 이상 실행되지 않는다. 이전 v0.2.9 diagnostics 파일의 시각은 19:44:17 KST였으므로, 앞선 보고 이후 새 버전으로 전환된 것이 확인된다. 운영 키로 검증한 공식 v0.2.10 ARM64 updater와 설치 앱에 남은 다운로드 캐시 `package`의 SHA-256도 정확히 일치한다.

설치 직전 기록 `/tmp/issue102-final-before-install.json`과 새 프로세스의 inspect를 대조했다.

| 메모 | 위치 | 크기 | 보임 | 상단 고정 | 본문 |
|---|---|---|---|---|---|
| main | (3496, 2208) 일치 | 600×96 일치 | true 유지 | true 유지 | 기존 본문 수집·변경 없이 기하학만 비교 |
| issue102-visible-20261107 | (3496, 2078) 일치 | 600×130 일치 | true 유지 | true 유지 | 저장된 Markdown 정확히 일치 |
| issue102-hidden-20261107 | (3496, 1982) 일치 | 600×96 일치 | false 유지 | true 유지 | 저장된 Markdown 정확히 일치 |

세 창 모두 folded=false(펼침)로 일치했다. 업데이트 복원 snapshot `.update-windows.json`은 남아 있지 않았다. 새 프로세스의 가시성과 좌표가 모두 보존된 결과는 앱의 updater 창 복원 경로와 부합한다. 실시간으로 교체 순간을 캡처한 것은 아니며, 자동 idle 재시도와 추가 사용자 설치 버튼 동작의 구분은 사용자에게 확인 요청했다. 이 문서 작성 시점에는 그 답을 받지 못했다.

원본 근거: `/tmp/issue102-followup-installed-diagnostics.json`, `/tmp/issue102-followup-after-update.json`, 위 설치 전 기준 파일 및 대화의 CUA 자동 업데이트 패널 결과. 실제 앱에서 **다운로드·검증 완료 → 뒤이은 새 버전 실행·프로세스 교체 및 저장 메모/창 상태 보존**을 확인했다. 따라서 앞선 보류를 영구적인 업데이트 실패로 결론 내리지 않는다.

### 보류 메시지의 범위 재현

같은 `ui/updater-guard.js`를 VM에 올리고 편집 상태는 false로 고정했다. 읽기 전용 `snapshot` 요청을 완료시키지 않은 상태에서 prepare 이벤트를 보내면 `updater_ack.safe:false`가 나왔고, release 후 그 조회를 완료시킨 다음 prepare를 보내면 `safe:true`가 나왔다. 두 assertion을 통과한 원본은 `/tmp/issue102-transient-readonly-repro.json`이다. 운영 앱/캐시/동의를 변경하지 않은 제어 실험이다.

이는 **편집·저장 중이라는 UI 문구가 실제 편집만 뜻하지 않고, 진행 중 조회 IPC에도 대응할 수 있음**을 재현한다. 실제 과거 요청에서 어느 창의 어떤 조건이 false였는지는 현재 로그가 남기지 않아 복원할 수 없었다. 이 제어 실험만으로 당시 원인을 확정하지 않는다. 다만 이후 성공 사실과 함께, 재시도 가능한 일시 보류와 영구 설치 결함을 구별해야 한다는 근거가 된다.

현재 범위의 결론은 macOS ARM64의 실제 새 버전 실행과 메모 복원 성공이다. 최초 안내의 OS 포커스·장시간 반복 억제, 버튼 설치 성공과 자동 idle 설치 성공 각각의 독립 실증, PTY 등 다른 보호 조건, 오프라인/변조/중복 요청 및 Intel·Windows의 실제 업데이트는 남아 있어 #102 전체를 닫지 않는다. 확신도 **Conf 5 (이 Mac의 v0.2.10 실행·메모 복원)**, 위험도 **Medium**. 새 배포나 보호 로직 변경은 이 후속 확인에 필요하지 않았다.
