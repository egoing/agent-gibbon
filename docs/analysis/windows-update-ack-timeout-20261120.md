# Windows 0.3.5 → 0.3.7 업데이트 준비 시간 초과 원인 분석

2026-11-20, 분석 전용. 제품 수정·설치·프로세스 종료·설정 변경·외부 전송·커밋·배포 없음.

- [x] 스크린샷 직접 확인
- [x] 독립 워크트리/브랜치 및 정책 확인
- [x] 실제 설치 앱 버전·PID·읽기 전용 진단 확인
- [x] 준비 대상과 실제 응답 구현 비교
- [x] 원본 0.3.5 HTML/guard의 격리 VM 재현
- [x] 기존 관련 테스트 실행
- [x] 안전한 수정 방향·미검증 범위 기록
- [ ] 제품 수정 및 실기기 설치 회귀: 이번 분석 요청의 범위 밖

## 결론

확정: 0.3.5는 업데이트 준비 응답을 구현하지 않은 보조창까지 전체 응답 대기 대상으로 포함한다. 사용자 설치 앱에 이 보조창 네 개가 존재하고, 실제 실패 기록의 미응답 네 개와 수·정렬 위치·종류가 일치한다. 이는 이 사례의 시간 초과를 충분히 설명하는 구체적 결함이다. 실제 기록에는 label이 없으므로 과거 실패 epoch의 개별 label을 직접 관측했다고 주장하지 않는다.

원인 보조창은 direction-guide(방향 안내), layout-undo(정렬 취소하기), memo-backdrop(메모 배경), memo-overview(메모 모아보기)다. 사용자 메모나 숨긴 초안을 버리고 진행할 문제가 아니다. 응답 구현이 없으므로 타임아웃 연장은 해결책이 아니다.

## 실제 환경 근거

- 프로세스 PID 29880, 실행 파일 `C:\Users\egoin\AppData\Local\Jajan\jajan.exe`.
- 파일 버전 0.3.5, 실행 중 앱의 `diagnostics.updater.status.current_version`도 0.3.5. 다운로드 대상은 0.3.7이며 혼동하지 않음.
- 설치 앱에 `diagnostics`만 호출: `ok:true`, `instance:null`, `phase:ready`, 6,105,950/6,105,950 바이트 다운로드 완료. 조회 당시 자동 설치는 최근 입력 때문에 보류 중.
- 마지막 시도: epoch 4, outcome ack_timeout, source 0.3.5, target 0.3.7, timestamp 1795175582704 = 2026-11-20 20:53:02.704 +09:00. 첨부 스크린샷은 20:51이므로 동일 시도라고 단정하지 않음. 같은 오류의 후속 기록이다.
- 로컬 기록: `[문서 폴더]\jajan\docs\.last-update-attempt.json`; 필요한 상태 필드만 조회하고 메모 원문은 읽지 않음.
- ACK 8개 중 번호 1·2·4·5는 ack:null/report:null. 번호 3·6·8은 memo/ack:true, 번호 7은 stash/ack:true. 네 ACK 모두 pending=0, editing/saving/exporting/privacy=false. stash는 visible=false인데 정상 ACK하여 숨김 자체가 필연적 실패 원인은 아님.
- Win32 EnumWindows/GetWindowThreadProcessId/GetWindowText/IsWindowVisible로 PID 29880의 지정 보조창 제목 네 개만 읽음. 방향 안내·정렬 취소하기·메모 모아보기·메모 배경이 모두 존재하고 visible=false. 사용자 창의 제목·본문 출력 안 함. 초기 Add-Type 인자 오류 후 수정한 조회가 성공했으며 앱 동작 변경은 없음.

## 코드 근거와 인과

기준 HEAD/start/base = 5220c9402aa440e2f354255c239b177f1e1a1d88. 설치본과 맞추기 위해 v0.3.5 태그의 파일도 git show로 비교했다. 해당 updater.rs는 HEAD와 차이가 없고 guard는 자동실행 설정 busy 검사 추가만 다르다. 0.3.7의 보조창에도 같은 누락이 남아 있다.

| 위치 | 관찰 |
|---|---|
| src-tauri/src/updater.rs:929 | managed_windows의 모든 label을 준비 대상으로 수집 |
| src-tauri/src/web.rs managed_windows | app.webview_windows 전체와 웹 host 병합; 보조창 필터 없음 |
| src-tauri/src/update_policy.rs:83 | prepare가 각 label을 None으로 기록 |
| src-tauri/src/update_policy.rs:116 | 모두 Some(true)여야 ready |
| src-tauri/src/updater.rs:942 | 3초 안에 ready가 아니면 동일한 한글 오류 반환 |
| src-tauri/src/stash.rs:1034 | 입력 훅 소유 시 overview 두 창, layout-undo, direction-guide 생성; 이후 숨긴 채 유지 |
| src-tauri/src/layout/overview.rs:124 | memo-backdrop/memo-overview 숨김 생성 |
| ui/direction-guide.html, ui/layout-undo.html, ui/memo-backdrop.html, ui/memo-overview.html | updater-guard.js가 없음. 연결 스크립트에도 prepare/ACK 구현 없음 |
| ui/updater-guard.js:69 | guard가 있는 창만 prepare를 듣고 updater_ack를 보냄 |
| src-tauri/src/telemetry/watchdog.rs kind | 알려진 prefix 외에는 memo로 분류, 네 보조창 모두 memo로 오표현 |
| src-tauri/src/updater.rs:886 | BTreeMap 정렬 순으로 number를 기록하지만 label은 기록하지 않음 |

네 미응답 위치는 label 알파벳 순 direction-guide / layout-undo / main / memo-backdrop / memo-overview / note-* / stash-indicator / updater-notice와 정확히 맞는다. 과거 label별 직접 기록이 없다는 한계는 유지한다.

## 격리 재현과 검증

작업 root `C:\Users\egoin\.codex\worktrees\update-ack-analysis\agent-gibbon`, branch `codex/update-ack-analysis`. create_worktree 전용 도구 사용 후 브랜치 생성. AGENTS.override.md 자동 복사 확인. 원본 checkout 및 다른 작업 파일 변경 없음.

1. PowerShell stdin → Node VM에서 `git show v0.3.5:...`로 원본 HTML 7개와 guard를 읽어, 각 HTML에 guard가 있을 때만 실행했다. mock IPC에서 epoch 4의 prepare/release 전달.
2. 보조창 네 개는 listener 없음/ACK 없음. index·stash·updater-notice는 listener 있음/safe:true ACK. 네 누락 명칭과 ACK 개수에 대한 assert 통과. WebView2나 실제 설치 E2E가 아니라 코드 실행 수준의 재현이며 원본 파일에 테스트 훅을 남기지 않았다.
3. `node --test tests/updater-guard.test.mjs tests/updater-release.test.mjs`: 21/21 통과. busy 거절, pending 드레인, stale epoch, release 후 명령 재개, ACK 거절 은폐, 릴리스 검증을 확인했다. 이 기존 테스트들은 모든 실제 HTML의 참여 여부를 검사하지 않아 결함을 놓친다.
4. Node v20.11.0은 package engines >=22.13보다 낮음. 위 순수 Node 테스트는 실제 통과했지만 전체 지원 환경 빌드 검증으로 간주하지 않는다. Rust·WebView2·설치 E2E 미실행.

## 다른 가설과 실패 후 상태

- dirty/저장/내보내기 거절은 safe:false와 별도 사유를 만든다. 이번 기록은 false가 하나도 없고 네 null이므로 관측된 직접 원인이 아니다.
- 웹 host/remote child 세션 및 터미널은 safe_native/update_restore의 사전 차단 대상. 이번 기록은 해당 kind가 없으며 보조창 누락만으로 충분히 설명된다. 이 경계를 완화하면 안 된다.
- 숨김 timer throttling: 일반 위험은 있으나 이번 네 창은 listener 자체가 없어 timer 이전 단계에서 실패한다. 직접 WebView2 지연 실험은 하지 않음.
- listener 등록 비동기 타이밍, activity 콜백 예외, ACK IPC 실패는 추가 null 원인이 될 수 있고 catch가 오류를 숨긴다. 이번 사례에서 이를 증명하는 증거는 없음.
- 동시 요청: status가 ready일 때만 preparing으로 전환하며 frozen/activities/epoch 검사도 존재. stale epoch의 ACK와 release는 거절한다. 이번 실패의 직접 증거 없음.
- 시간 초과 시 updater.rs:1000~1022에서 phase를 ready로 되돌리고 진단을 기록한 뒤 release_epoch → updater-release를 보낸다. JS release는 입력 차단과 frozenEpoch를 해제한다. 실제 조회에서도 ready다. 다만 emit 오류를 무시하며 release 수신 ACK는 없으므로 모든 프론트엔드 해제를 실기기에서 직접 확인했다고 할 수 없다.
- 화면 하단 경고는 freeze 중 반복 snapshot/poll 요청이 guard에서 거절되어 app.js의 failure→notify로 표시되는 경로다. updater-release는 이 알림을 지우지 않는다. 비편집 상태는 알림 타이머로 지워질 수 있고 editing=true이면 persistent가 될 수 있다. 따라서 경고가 남았다는 사실만으로 네 창 모두 아직 동결됐다고 판단하면 안 된다. 이번 개별 경고의 실제 지속시간은 미측정.

## 안전한 수정 권고와 필요한 검증

1. 창 역할과 업데이트 참여 방식을 명시하는 단일 규약을 둔다. 보조창도 준비/해제 프로토콜을 구현하거나, 네이티브에서 사용자 데이터·작업이 없는 것으로 보장된 역할만 명시적으로 처리한다. 미응답·숨김·알 수 없는 창을 포괄적으로 제외하는 방식은 금지한다.
2. guard만 HTML에 추가하면 보조 UI가 직접 호출하는 core.invoke가 pending/input 규약을 우회할 수 있다. layout/overview/undo 변경 같은 작업도 같은 보호 경계와 명령 추적을 거치게 설계해야 한다. 새 listener 준비 완료와 현재 epoch 재조회/해제 복구도 검토한다.
3. 익명 진단의 memo 기본값을 세분화하고, 고정된 보조창 역할·listener 등록/ACK 실패 단계·경과시간을 기록한다. 사용자 메모 ID·제목·본문은 전송하지 않는다.
4. 설치 준비 상태는 일반 오류 알림과 구분하고, release 때 그 상태 알림만 해제한다. 실제 저장 실패나 초안 경고까지 일괄 삭제하지 않는다.
5. 네 보조창이 생성된 기본 Windows 시작 상태/숨김·표시 각각에서 전체 준비가 완료되는 테스트, guard/역할 규약과 HTML 연결 검사, dirty/IME/저장/내보내기/캡처/터미널/웹 세션의 차단, listener 미준비/ACK 실패/해제 실패/중복 요청/재시도에 대한 회귀를 추가한다.
6. 실제 설치는 docs/releases.md 설치 보호 실기기 시험에 따라 전용 검증 앱·bundle/store/feed·일회성 키에서만 검증한다. 실사용 0.3.5 앱을 분석 목적으로 종료하거나 강제 설치하지 않는다. 이 결함은 기존 실행 앱의 준비 경로에 있어 새 버전 공개만으로 기존 앱의 ACK 코드가 바뀌지 않는다는 점도 복구 계획에 반영해야 한다.

## 현재 상태와 한계

분석 완료, 제품 해결 미실행. Conf 3(실제 실패 진단/현재 네 보조창 확인 + 격리 VM 재현 + 관련 테스트 21개 통과), Risk Low(이번 읽기 전용 조사·문서만); 향후 수정은 저장/설치 상태 경계 영향으로 Medium. 실제 설치 재현·완전한 label별 당시 매핑·모든 창의 해제 동작은 미검증이다.

president office.py는 배정 지침대로 Windows fcntl 미지원이라 실행·우회·가짜 등록하지 않았다. 등록 불가와 실제 분석 착수를 구분한다. agent-gibbon 스킬 경로에 파일이 없어 이 문서 체크리스트와 대화로 진행 표시했다. voice-guide도 fcntl import 실패로 음성 안내 불가, 화면 안내로 진행했다.

문서만 추가하여 dev:build는 실행하지 않았다. `npm run dev:info` 결과 ID는 `dev-064b569be88189ba4c163c22`, binary는 작업 root의 `src-tauri\target\debug\jajan.exe`. 이 워크트리에서 앱을 빌드·실행하지 않았으며 곧바로 실행 가능한 바이너리를 준비했다고 주장하지 않는다. 향후 개발 실기기 검증 시 이 root에서 `npm run dev:build` 후 `scripts/jajan-dev.cmd open`을 사용하고 PID·inspect.instance를 다시 대조해야 한다.

## 후속 확인: 사용자 메모창을 모두 닫은 뒤

사용자가 모든 창을 닫았는데도 같은 오류가 발생한다고 보고한 후 설치 앱 diagnostics를 다시 읽었다. epoch 5, timestamp 1795176527327, source 0.3.5/target 0.3.7, outcome ack_timeout이다. 준비 대상은 이전 8개에서 6개로 줄었지만 미응답은 여전히 4개(number 1~4, memo/null), stash와 남은 memo 종류 창은 safe:true였다. 사용자 메모창이 없어져도 네 보조창의 미응답이 남는다는 원인 분석과 일치한다. 과거처럼 label 자체는 진단에 없어 미응답별 label을 직접 관측한 것은 아니다. 사용자 앱 조작·설치·종료는 수행하지 않았다.

## 수정 작업 후속
사용자가 해결을 요청한 뒤 네 보조창의 준비·해제 응답과 명령 보호를 구현했다. 최신 범위·검증·미검증 사항은 [수정 검증 기록](../verification/updater-auxiliary-ack-20261120.md)을 따른다. 위 분석 시점의 ‘제품 수정 미실행’은 당시 상태이며 현재는 미커밋 구현과 개발 빌드까지 완료했다.

