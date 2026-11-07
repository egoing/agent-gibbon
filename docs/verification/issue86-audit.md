# 이슈 #86 독립 코드·정직성 감사

감사 대상: `27e5321049ad42b2e75848221ac27f9990df3055`.
최초 BASE: `48041c5e3dcdb6db98e136d0098ab30a6343666a`, RED: `edad8f93ef5badb881322c3a68703573344d0c43`.
스쿼시 전 감사 이력: `refs/is2/attempts/86` = `3009e0b619fdbfc5ed38f065ad7b5ef05ef9e7f7`.
PR 기준 upstream: `047858207aa36f31baac3870bb1c14a03f887242`.
프로젝트 코드 변경 없음. 처음 감사할 때 구현 이력을 상속하지 않고 이슈 본문·전체 diff를 읽었으며 이후 수정마다 증분 재감사했다.

## 최종 코드 판정

현재 확인된 미해결 기능 차단 결함 없음. 외부 창 상태를 변경하므로 Risk Medium 유지. 최신 Windows 실제 제품 관측은 아래 최종 증거 감사에서 확인했다. 전체 플랫폼 CI는 아직 미요청이므로 완료로 판단하지 않는다.

앞서 발견한 직접 attach 선택기 닫기 경합, 동일 프로세스 HWND 재사용 식별 누락, 저널 I/O 이후 출처 재검사 누락, 열거 이후 감시 등록 누락, 바로가기 WorkingDirectory 누락, 동기 SetWindowPlacement로 인한 외부 앱 정지 시 복원 블로킹, 텍스트 버튼의 24px 아이콘 폭 상속은 모두 코드상 보완을 확인했다.

창 inventory는 snapshot 전에 감시 등록한다. 배치 시 저널 기록 후·claim 후 identity/epoch/property를 반복 확인하고 변경 전 실패는 geometry를 건드리지 않고 claim/저널을 정리한다. 복원은 process creation/path와 고유 window property를 검사한다. 복원용 WINDOWPLACEMENT에는 WPF_ASYNCWINDOWPLACEMENT를 적용하고 관측 timeout 이후 미복원 상태를 보존한다. 저널은 인스턴스별 Store 아래에 있고 창 lease는 인스턴스 간 공유 잠금이다.

Win32 검사·변경 사이의 잔여 TOCTOU는 사라지지 않았으며 README와 디자인 시스템에서 명시한다. WinEvent ACK를 OS 원자성 또는 모든 이벤트 전달 완료의 증명으로 표현하지 않는다. 혼합 DPI·UAC/권한 경계·모니터 분리·전체 z-order 보존은 지원 검증으로 확대할 수 없다.

## 통합·충돌 해결

`3009e0b..27e5321` 차이는 upstream #84의 관리 창 활성화 및 관련 문서·증거 7개 파일이다. #86 PR 자체의 scope 이탈로 판단하지 않는다. #86의 external_apps.rs/windows.rs, capture.rs, windows_messages.rs, stash_input.rs, external-apps.css/html/js 8개 파일을 git object에서 직접 비교하여 byte 동일함을 확인했다.

충돌 해결 후 디자인 시스템의 #86 절은 attempts와 그대로 같고 #84 절은 upstream과 그대로 같다. 양쪽 확정 기준과 검증 한계가 모두 보존됐다. manager.rs는 upstream #84 변경에 이번 Request.external 기본값 한 줄을 더한 PR diff이며 외부 앱 창 처리 변경은 없다.

## 정직성 3항목

| 항목 | 판정 | 근거 |
|---|---|---|
| 테스트 완화 | 미발동 | RED CLI 계약 테스트 유지, 기존 테스트 기대값·timeout·lint 변경 없음. Windows 테스트 모듈은 위치만 이동했고 strip한 본문 SHA256 `15170e786625962f0a5aeb794b5868701fa9abffb0bc98597509f89850ae43d2`와 source-integrity.json 일치. Windows UI 1 SKIP은 기존 POSIX fixture 제한이며 신규 skip 아님. |
| 퇴행 | 미발동(실행한 범위) | 최신 제품 SHA에서 Mac 9개 명령 exit0 및 Windows 178개 Rust/123개 UI·기존1skip, 실제 필수4종·9개 UI 관측을 원본 대조했다. 미검증 환경·실제 경합·트레이/동시 설치 인스턴스는 별도 보류하며 전체 플랫폼 CI는 아직 미요청이다. |
| 스코프 이탈 | 미발동 | 네이티브 창 선택/배치/고정/복원과 로컬 저널·lease에 한정. WebView 임베딩·새 서버·클라우드/Sentry 추가 없음. upstream #84는 PR 기준에서 제외됨. |

## 증거 검토

PoC 성공 run-20261107-012815의 원시 관측·C# 외부 앱 소스·캡처를 대조했다. 활성 일반 B 위의 비활성 A, 실제 문서 입력과 clicks=1, 복원 화면을 직접 확인했다. manifest 41개 파일 해시 모두 일치. 실패 네 실행은 CLOSED로 보존됐고 source SHA 37dbfed/High→High/125% 단일 화면 한계를 명시한다. PoC가 제품 구현 전에 실행됐으며 제품 검증과 구분돼 있다.

Windows escalation source.diff를 독립 대조해 테스트 본문 및 나머지 Rust(빈 줄 제외) 불변을 확인했다. 실제 headless Chromium 4개 viewport의 before FAIL→after PASS 측정과 480×460/240×240 PNG를 열어 확인했다. HTML/CSS 실제 레이아웃·Range·scroll 접근성 검증이며 실제 Tauri/WebView2 실행 증거는 아니다. raw test178/0ignored, UI123/기존1skip, fmt/check/Clippy/dev-build exit0와 출력 요약을 대조했다.

초기 Windows 실패 E0282, Clippy ABI/테스트 모듈 배치 실패는 SHA별 시도 기록에 보존되며 결과를 감추거나 검사를 완화한 근거는 없다. 초기에는 원시 PowerShell 로그의 trailing whitespace/EOF blank 때문에 `git diff --check`가 exit2였다. 이후 증거 폴더에 국소 .gitattributes로 원시 log의 whitespace만 제외했고 최신 작업트리 diff --check는 exit0이다. 소스 lint/test 규칙은 불변이며 원시 증거는 수정하지 않았다.

## 후속 확인

최종 증거·문서 커밋에 대한 release.yml 전체 플랫폼 URL/headSha/결과는 아직 미요청이며 수신 후 추가 감사해야 한다. 아래에 명시한 미검증 범위를 완료 처리하지 않는다.


## 최신 Windows 제품 원본 증거 최종 감사

제품 SHA `27e5321049ad42b2e75848221ac27f9990df3055`, 증거 전용 원격 커밋 `0355f52ffc6bb7c0cae2f4e1a29d999cfc54f173`에서 복원된 docs/verification/issue86-product-windows를 검토했다. 현재 제품 src-tauri/ui/tests 미커밋 diff는 없다.

- sha256.json의 Windows 상대 경로를 PureWindowsPath로 해석하여 460개 파일을 직접 해시 계산했으며 불일치 0개였다.
- 27e5321-gates의 source-sha, git-status(증거 폴더만 untracked), dev-info, cargo-test/check/Clippy/dev-build/UI 원본을 확인했다. Rust178/ignored0, UI123/기존1skip, fmt/check/Clippy/dev-build 성공과 출처가 일치한다. UTF-16 원본도 BOM에 맞춰 해석했다. Mac rebased/results.json의 SHA 일치와 9개 명령 exit0도 확인했다.
- 022338 raw: PID23952의 정확한 debug exe, inspect ok/instance/고유 본문 및 native title/HWND 확인. A PID19100/HWND3017072의 실제 rect=[220,180,860,600]. B PID22240/HWND9177576 실제 제목줄 클릭 뒤 foreground=B, 겹침 root=A. 외부 문서 PRODUCT_86_KEYBOARD와 button-click/실제 clicks=1 캡처를 대조했다. release 후 rect/topmost/visible/placement가 최초값과 일치한다. 최소크기 실패 rollback과 닫힌 inventory 거절도 관측했다. topmost crop은 A 내부만 보여 B 활성 자체의 시각 증명은 아니며 foreground+hit-test 원본 관측과 결합하여 판단했다.
- 022353 raw: Start Menu의 전용 .lnk→신규 프로세스 연결, Launch-events.tsv 실제 cwd, 기존 창 재선택 반환, 지연 실행 취소 후 topmost 없음/세션 없음, 12초 timeout failed와 뒤늦은 비고정 창을 대조했다.
- 022543 raw: 저널 경로·claim 기록, 정확한 자기 PID 비정상 종료와 재시작 후 inspect/PID/경로 재확인, 실제 복원값 일치 확인. claim 제거 모의는 변경된 창 상태를 그대로 유지하고 기록만 폐기한 것으로 확인했다. WM_QUIT 정상종료 복원도 확인했다. 실제 HWND 재사용과 실제 트레이 종료로 확대하지 않는다.
- 022602 raw 및 UI 결과 파일: 자기 PID26864의 overlay가 실제 Ctrl+Shift+4 뒤 생겼고 CLI fallback 발생 없이 소유 확인. Escape/작은 영역 유지/정상 재드래그, 80·97·81×42 물리px의 한 줄 버튼, 선택만으로 세션 없음, 기존 UIA/UIB 재선택, 명시적 실행 후 UIA 세션, 실제 해제 버튼 뒤 세션 제거·원래 placement 복구, 취소 버튼 뒤 선택기 제거를 대조했다. 검색 일부는 type-text readback 실패 후 set-value fallback이며 외부 앱 키보드 입력 증거와 구분한다. selected PNG를 직접 열어 최종 실제 WebView2 버튼·행을 확인했다.
- final-cleanup UTF-16 원본에서 foreground1967202 동일, cursor[1618,908] 동일, 자기 창 종료를 확인했다. existing-before/after의 타인 Gibbon은 둘 다 빈 배열(0→0)이다.

상위 issue86.md와 제품 README는 0→0을 동시 설치 인스턴스 보존 PASS로 표현하지 않는다. 실제 tray 해제/종료, 재활용 HWND/PID, 직접 attach/닫기 의도적 경합, 무응답 타사 앱, 혼합 DPI/복수 모니터/음수 원점/Medium/UAC 및 기본 설치 인스턴스와 동시 격리는 미검증으로 명시됐다. 모의 guard와 실제 HWND 재사용, WM_QUIT와 트레이 GUI를 구분한다. 현재 문서에서 검증을 부풀린 근거는 발견하지 못했다.

최종 판정은 테스트 완화 미발동 / 퇴행 미발동(실행한 범위) / 스코프 이탈 미발동이다. 새 기능 차단 결함 없음. 필수 Windows 제품4종·실제 Welcome9종의 증거 수준은 실환경 관측(Conf5, 해당 범위), 위험도는 Medium 유지. 전체 플랫폼 CI와 미실행 조건까지 완료로 확대하지 않는다.
