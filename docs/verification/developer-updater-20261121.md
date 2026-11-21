# 숨김 개발자 업데이트 시험 — 2026-11-21

이 문서는 최초 준비/동일 버전 시험 당시의 기록이다. 이후 승인된 버전 왕복·터미널 분리 연동·병합 진행 상태는 [후속 기록](developer-roundtrip-20261121.md)을 따른다.

구현·검증 전용 작업이다. 커밋·push·병합·릴리스는 실행하지 않았다.

## 출처와 환경

- 루트: `C:\Users\egoin\.codex\worktrees\5170\agent-gibbon`
- 브랜치: `codex/developer-update-test`
- fetch 후 기준 origin/main과 HEAD: `974197adbc34f2f72872eb4e6262e270db90ef2d`
- 기존 PTY 워크트리 `04fb`와 기본 체크아웃의 변경은 읽거나 복사하지 않았으며 작업에 포함하지 않았다.
- Cargo 1.99.1, Tauri CLI 2.12.1. 시스템 Node 20.11.0은 프로젝트 요구(22.13 이상)보다 낮아서 UI 검증에는 임시 폴더의 공식 Node 22.13.1을 사용했다. 의존성은 고정 pnpm lockfile로 설치했다.
- 개발 인스턴스: `dev-0e8f09996ee653e1d6855ad6`, debug 실행 파일: `C:\Users\egoin\.codex\worktrees\5170\agent-gibbon\src-tauri\target\debug\jajan.exe`.
- 최종 debug 실행 파일 SHA-256: `FB137EA85CE321D0F4F9D1AA27C64769B31132DAE3F936838E0B038E2D331D45`. 개발 CLI inspect에서 `ok=true`, 예상 instance, 메모 ID `developer-update-proof-5170`, 고유 검증 본문을 대조했다. 최종 PID·실행 경로·해시와 sentinel 대조는 [최종 개발 앱 증거](developer-updater-20261121/final-dev-probe.json)에 기록하고, 확인 후 이 작업에서 시작한 debug 프로세스만 종료했다.
- 별도 release 검증 앱: `src-tauri/target/release/jajan.exe`, identifier `app.jajan.developer-update-test`, instance `dev-001020010200102001020010`. 공식 공개키와 공식 최신 피드로 빌드했다. 검증 전용 표준 타이밍(5분 무입력/15분 확인/15초 재시도)을 사용했다.
- release UI 검증 실행 파일 SHA-256: `AC4D50AF418648D8F53DBA3FFACD5D0B4008A1F689D9B2E2F4E52F7FC6B8B8F5`, 검증 시작 PID 25108. 실제 설치 앱 PID 20272의 경로는 `C:\Users\egoin\AppData\Local\Jajan\jajan.exe`였으며 종료하거나 업데이트하지 않았다. UI 검증 후 수동 캐시 복원 검증 실패의 복구 문구만 추가 보완했다. 이 오류 분기의 문구는 실환경에서 실패를 주입해 재검증하지 않았으며 최종 debug 빌드에 포함했다.
- 검증 저장소는 `%APPDATA%\app.jajan.developer-update-test`다. 재빌드 후 관리 창을 다시 열기 위해 이 시험 앱의 약관 파일만 별도 이름으로 보존하고 첫 실행을 반복했다. 선택 진단 동의는 모든 실행에서 false다. 실제 설치 앱의 동의·로그인 등록·메모는 변경하지 않았다.

## 구현과 설계

- 설정에서 Privacy & Diagnostics 다음의 ‘개발자’ 항목은 기본 숨김이다. 기존에는 고정된 하단 버전 표시가 없어 사이드바 아래에 실제 버전 버튼을 추가했다. 600ms 이내 간격으로 5회 클릭하면 현재 WebView 세션에서 표시한다. 인증·접근 권한으로 설명하지 않는다. 메뉴가 늘거나 높이가 좁아져도 버전 위치를 유지하도록 탐색 목록만 스크롤한다.
- ‘최신 배포판 준비’는 기존 공식 피드·플랫폼 파일명·정상 서명·signed version·trusted filename 검사를 공유한다. 개발자 경로만 안정 버전 `>= current`를 허용한다. 일반 경로의 `> current`는 유지한다. 실제 버전값을 변경하지 않는다. Tauri updater 2.14.1 소스의 `update_from_release` 비교에서 동일 버전이 거절됨을 확인했고, `install` 자체에는 별도의 버전 비교가 없음을 조사했다. 실제 NSIS 재설치 완료는 아래 미검증 범위에 남긴다.
- 캐시의 manifest를 마지막에 원자적으로 교체하며 `_jajan_install_policy=manual-test-v1`을 패키지 메타데이터와 함께 저장한다. 기존 policy 없는 캐시와 `automatic-v1`은 일반 자동 경로다. 알려지지 않은 policy는 거부한다. 수동 policy를 읽은 후 패키지 재검증이 실패해도 수동 대기를 유지해 자동 다운로드·설치로 전환하지 않는다.
- 수동 준비 완료 후 일반 자동 확인·재시도·일반 설치 버튼·준비 알림은 설치를 시작할 수 없다. 재실행 후에도 수동 policy와 공식 패키지를 다시 검증한다. 두 번째 버튼만 기존 공통 설치 절차로 연결한다. 수동 시험을 시작한 준비 패키지는 사용자의 두 번째 행동을 기다리며, 별도 취소 버튼은 이번 범위에 없다.
- 설치 경로는 기존 native activity·편집·저장·내보내기·캡처·터미널 보호, 모든 창 ACK, 최종 재검사, 복원 저장과 설치 직전 서명 확인을 공유한다. 강제 해제·임의 파일 선택·서명 무시를 추가하지 않았다.
- 읽기 전용 상태는 현재/준비 버전, 동일 버전 시험 여부, 다운로드 바이트, 검증·캐시, 안전한 실패 단계와 실제 설치 보류 이유다. 사용자 메모·터미널 명령·출력 수집은 추가하지 않았다.
- [디자인 시스템](../design-system.md), [개발자 SVG](../design/developer-updater.svg), [PNG](../design/developer-updater.png), [설정 탐색 SVG](../design/manager-settings-wireframe.svg), [PNG](../design/manager-settings-wireframe.png)를 갱신했다. PNG는 resvg로 렌더링하고 확인했다.

## 자동 검사

| 검사 | 결과 |
| --- | --- |
| 지원 Node 22.13.1 전체 UI 검사 | 254 통과, 기존 2 건너뜀, 실패 0 |
| Rust 전체 테스트 | 316 통과, 실패 0 |
| 공식 피드 verification feature의 updater 테스트 | 14 통과, 실패 0 |
| 마지막 캐시 복구 문구 보완 후 updater 재검사 | 14 통과, 실패 0 |
| Clippy all-targets `-D warnings` | 통과 |
| 최종 변경 후 `npm run dev:build` | 통과 |
| `npm run dev:info` 및 개발 CLI inspect | root/id/binary와 실제 응답 일치 |
| `git diff --check` | 통과 |

초기 시스템 Node 20 실행에서는 ESM/타이머 관련 기존 테스트가 실패했다. 지원 Node로 전환한 후 모두 통과했다. 새 동작으로 사이드바 기대값을 갱신했고, 느린 클릭 초기화·5회 진입·중복 요청 차단·동일 버전 표시·준비 상태에서 설치 명령 미발행·보류 문구·개발 빌드 비활성·정상 자동 준비와 수동 시험 분리·캐시 policy 직렬화·하위/사전 버전 거부를 검사했다. 기존 정상 서명/변조/플랫폼/버전 바인딩과 작업 보호 테스트도 전체 검사에 포함됐다.

## 실제 Windows UI와 공식 패키지

Computer Use의 반환 창을 경로와 네이티브 제목으로 선택했다. `Jajan [DEV Updater verification · 00102001] — 메모 관리` 창과 위 PID를 대조했다. 설치 앱의 같은 이름을 근거로 삼지 않았다.

- 기본 숨김, 5회 클릭 뒤 개발자 항목 표시, 고정된 하단 버전, 키보드 접근을 확인했다. 초기 UI 실측에서 개발자 패널의 잘못된 HTML 중첩과 좁은 사이드바의 버전 이동을 발견해 수정하고 재빌드한 화면으로 재검증했다.
- 준비 전에는 첫 버튼만 활성, 설치 버튼은 비활성이다. [숨김 화면](developer-updater-20261121/hidden.jpg)과 [초기 패널](developer-updater-20261121/initial-panel.jpg)은 같은 구현의 선행 화면 검증이며 아래 공식 다운로드 증거는 최종 빌드에서 얻었다.
- 격리 verification feature가 처음에는 공식 피드에도 로컬 시험 상대 경로 규칙을 적용해 `asset_url`에서 거부했다. 공식 endpoint일 때는 배포판과 동일한 정확한 GitHub 릴리스 경로 검사를 적용하도록 수정했다. 검증 무시나 URL 범위 확대를 하지 않았다. 별도 로컬 시험 피드 경로는 feature 내부에서 유지했다.
- 2026-11-21 19:54 KST에 **공식 최신 0.3.12** Windows x64 패키지 **6,110,871 bytes**를 실제 다운로드했다. 현재 버전도 0.3.12이며 실제 signed version과 플랫폼 파일명 검증을 통과했다. 캐시 SHA-256: `ECDD3A6808FA142F72825F9523F87C213F443CA917B88FE5F34634EFB63EFF3B`.
- [준비 결과](developer-updater-20261121/prepared.json): `phase=ready`, `manual_test=true`, `signature_verified=true`, `cache_ready=true`, 현재/준비 버전 모두 0.3.12. manifest의 policy는 `manual-test-v1`이다. [실제 준비 완료 화면](developer-updater-20261121/ready.jpg)에 ‘동일 버전 재설치 테스트’와 활성화된 두 번째 버튼을 확인했다. 선택 전송은 false, SDK 비활성 상태다.

- 19:56 KST에 이 시험에서 시작한 release PID 25108을 정확한 실행 경로로 확인해 종료하고 같은 바이너리를 `--serve`로 재실행했다. 새 PID 17304의 [복원 결과](developer-updater-20261121/restored.json)는 현재/준비 버전 0.3.12, 동일한 바이트 수, `manual_test=true`, `signature_verified=true`, `cache_ready=true`, `phase=ready`다. 재다운로드나 버전 위조 없이 저장된 캐시로 복원했다.

- **실제 5분 무입력 초과:** 19:59:43 KST의 Windows `GetLastInputInfo` 읽기 전용 실측은 **345.203초**였다. 300초를 넘긴 뒤 15초 재시도 주기도 지났지만 같은 PID 17304가 유지됐고 `phase=ready`, `manual_test=true`, 설치 handshake 없음이었다. 시험용 터미널을 만들기 전의 관찰이다. [유휴 보류 증거](developer-updater-20261121/idle-held.json). 시간값이나 입력 상태를 조작하지 않았다.
- **숨긴 터미널 보호:** 동일 최종 바이너리를 PID 30176으로 다시 실행하고, 전용 CLI로 명령 없는 Welcome 터미널 `terminal-1795258848832726900`을 생성한 다음 숨겼다. [fixture](developer-updater-20261121/terminal-protection-fixture.json)는 `visible=false`, `running=false`, `pid=null`이다. 실제 터미널 명령을 실행하거나 터미널 UI를 조작하지 않았다.
- Computer Use로 개발자 화면의 ‘준비한 버전 설치·재시작’을 클릭했다. ‘터미널 시작 화면이 남아 있습니다. 관리 창에서 터미널을 삭제한 뒤 다시 설치하세요.’를 그대로 표시하고 `phase=ready`, `manual_test=true`와 캐시를 유지했다. [보류 상태](developer-updater-20261121/terminal-held.json) · [실제 화면](developer-updater-20261121/terminal-held.jpg). Native preflight에서 보류돼 모든 창 ACK 단계에는 진입하지 않았고, installer 실행·프로세스 재시작도 없었다.
- 검증을 마친 뒤 이 작업에서 시작한 release PID 30176만 정확한 실행 경로로 확인해 종료했다. 시험 캐시와 자료는 보존했다. 실제 설치 앱 PID 20272는 계속 실행 중이다. 현재 실행 중인 사용자 앱에 새 개발 코드가 반영됐다는 뜻이 아니다.

## 미검증과 실행 안내

- 이 Windows 호스트의 공식 NSIS 패키지는 사용자 설치 경로를 바꿀 수 있다. 안전하게 분리된 Windows VM/별도 OS 사용자 설치 환경은 이번 작업에 제공되지 않아 **실제 NSIS 설치·앱 교체·재시작 완료는 실행하지 않았다**. 같은 버전 설치 API 경로와 준비 검증이 구현됐다는 사실과 실제 설치 성공을 구분한다.
- macOS 실제 앱, macOS 설치/재시작, 모든 창 ACK의 실환경 전체 조합, IME·저장·캡처 등 각 보호 상황의 실기기 매트릭스는 미검증이다. 기존 보호 테스트 통과가 이 실기기 검증을 대신하지 않는다.
- **Conf 4 / Risk Medium**: Windows 실제 UI·공식 다운로드·재실행·유휴와 native 보호 보류는 확인했다. 실제 설치 완료가 미검증이므로 전체 설치 기능의 실환경 성공(Conf 5)으로 보고하지 않는다. 업데이터 설치 경로에 영향이 있는 복구 가능한 변경이며 자동 병합 대상이 아니다.
- 이 작업 폴더에서 `scripts/jajan-dev.cmd open`으로 준비된 **debug 개발 앱**을 실행한다. 명령은 자동 재빌드하지 않는다. debug 앱은 기존 정책대로 업데이트 기능이 꺼져 있다. 변경 감시는 `npm run dev`다. 별도 release 검증 앱은 실제 사용자 설치 앱과 다르며 검증 결과의 출처를 혼용하지 않는다.
- 후속 아이디어만: 수동 시험 대기 취소 기능, 각 보호 단계의 원문 없는 상태 설명. 이번 범위에 추가 구현하지 않았다.
