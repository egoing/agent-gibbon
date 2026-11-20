# 로그인 자동 실행 설정 간소화와 통합 — 2026-11-20

## 승인과 대상

사용자 원문은 사이드 채팅 local `01a1f7f0-822b-73f2-8c50-1c498a1c4b1e`의 메시지 `01a1f7ff-e571-7262-a07b-f356d866c583`이다. 권고 UI 수정·검증·커밋·원격 반영·문제 없으면 main 병합을 승인했다. 이어 사용자가 배포는 팀장이 새로 배정한 전담 팀원에게 맡기도록 정정했다. 구현 담당은 버전·태그·전체 플랫폼 릴리스 실행·공개 배포를 수행하지 않고 병합 SHA를 팀장에게 인계한다.

- 작업 루트: `C:\Users\egoin\.codex\worktrees\login-autostart\agent-gibbon`
- 브랜치: `codex/login-autostart`
- 최초 base: `9753ed98d24ef0d679a2272543f4a93d8a11e870`
- 개발 인스턴스: `dev-9ef74ed7f8f96bcad543b8f4`
- 같은 워크트리의 사이드 채팅이 작성한 프라이버시 메뉴 순서·관련 시안 변경은 같은 사용자 요청으로 포함한다. 다른 워크트리의 세 줄 높이 작업 등 미커밋 변경은 포함하지 않는다.
- 기존 개발 앱 PID 14092가 같은 워크트리 바이너리로 실행 중임을 확인했다. 이 작업에서 시작한 프로세스가 아니므로 종료·재시작하지 않는다. 기존 `.tmp`와 설치 앱을 보존한다.
- president의 POSIX 전용 등록 CLI는 실행/우회하지 않는다. voice-guide도 이 Windows에서 기존에 확인한 `fcntl` 제한에 따라 화면 안내로 대체한다.

## 최종 동작과 디자인

- 설정 사이드바는 스킬 → 테마 → 자동 업데이트 → 로그인 자동 실행 → Privacy & Diagnostics 순서다. 초기 프라이버시 선택과 관리 창을 열어 둔 동안 마지막 설정 선택 보존은 유지한다.
- 설정 체크 변경 즉시 선택 저장과 OS 적용 요청을 한 번 실행한다. 저장 중 체크를 비활성화하고 업데이트 설치를 보류한다. 상시 ‘선택 적용’과 ‘상태 새로고침’은 제거했다.
- 설정 진입·앱 포커스 복귀에서 상태를 다시 읽는다. 조회 실패 때만 ‘다시 시도’를 표시한다. 이 버튼은 상태 조회만 하며 등록 재시도나 자동 On을 수행하지 않는다.
- ‘OS 시작 설정 열기’는 승인 필요 또는 Windows 허용 미확인 상태에서만 표시한다. 개발·검증 안내는 해당 실행 환경의 사용 불가 상태에만 표시한다. 일반 설치 앱에 개발 안내를 상시 표시하지 않는다.
- 첫 실행 기본 체크 제안은 Continue 후에만 등록한다. 약관·진단 전송 동의의 분리와 OS에서 끈 등록을 시작·업데이트 때 임의로 되살리지 않는 경계를 유지한다.
- 공통 기준은 [디자인 시스템](../design-system.md#로그인-자동-실행--2026-11-20), 시안은 [자동 실행 SVG](../design/login-autostart.svg) · [PNG](../design/login-autostart.png), [설정 탐색 SVG](../design/manager-settings-wireframe.svg) · [PNG](../design/manager-settings-wireframe.png), [업데이트 SVG](../design/updater-settings.svg) · [PNG](../design/updater-settings.png), [온보딩 SVG](../design/privacy-onboarding.svg) · [PNG](../design/privacy-onboarding.png)를 따른다.

## 검증 결과

- Windows x64, Node 24.22.3, Cargo/rustc 1.99.1에서 검사한다. Node PATH는 Codex 번들 버전을 사용했다.
- UI 테스트: 241개 중 240개 통과, 기존 skip 1개, 실패 0.
- Rust 테스트: 310개 통과 (최신 main 통합 전).
- Rustfmt·Clippy(`--all-targets -- -D warnings`)와 Windows debug 빌드 통과. 빌드는 `node scripts/build.mjs cargo build --manifest-path src-tauri/Cargo.toml --target-dir src-tauri/target/autostart-build-check`로 수행해 실행 중인 개발 앱의 표준 바이너리를 덮어쓰지 않았다. 이 별도 출력 바이너리는 실행·CLI 검증에 사용하지 않았다.
- 관련 집중 시험 23개 통과: 즉시 저장·저장 실패·미선택·On/Off·동의 분리·OS 상태별 조건부 버튼·조회 실패 재시도·메뉴 순서·저장 중 updater 보류.
- 실제 HTML/CSS/JS 자산의 Chrome 검사 통과. IPC는 mock이며 OS 설정을 변경하지 않는다. 최소 600×360에서 키보드로 Off를 선택한 직후 저장됨을 확인했고 pageerror는 0이다.
- 네 SVG를 PNG로 렌더했다. [Windows 허용 미확인 화면](login-autostart-20261120/settings-windows.png), [최소 크기·어두운 테마·Off 화면](login-autostart-20261120/settings-minimum-dark.png)을 최신 동작으로 갱신했다.
- 최초 Windows 실프로세스 로그인 시작 경로의 포커스·창 상태 검사와 OS 실등록 미검증의 구분은 [초기 기록](login-autostart-20261120.md)을 따른다. 기존 앱을 보존하기 위해 이번 UI 수정 후 해당 앱을 재시작해 시험한 것으로 보고하지 않는다.

## 통합·원격 반영

- [x] 즉시 적용 및 조건부 복구 행동 구현
- [x] 사이드 채팅의 프라이버시 순서 변경 포함, 문서·시안·브라우저 캡처 갱신
- [x] 집중 시험·전체 UI·Rust 검사
- [ ] 최신 main 통합 후 관련 검사·Windows 빌드 확인
- [ ] 이번 작업 전체 파일 커밋·push·PR 병합, 대상 파일별 원격 반영 확인
- [ ] 기본 체크아웃 main과 원격 main SHA 일치 확인
- [ ] 팀장에게 병합 SHA·검증 범위·배포 미실행 사실 인계

위 항목은 실제 수행 뒤 갱신한다. 전체 플랫폼 검사·macOS 실기기·양쪽 OS 실등록·실제 로그인·설치 시험은 구현 담당이 실행한 검증으로 표시하지 않는다. **Conf 4 / Risk Medium.**
