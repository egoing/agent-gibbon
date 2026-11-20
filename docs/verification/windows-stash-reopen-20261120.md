# Windows 숨김 직후 재표시 — 2026-11-20

## 대상과 출처

- 작업 루트: `C:\Users\egoin\.codex\worktrees\windows-stash-stall\agent-gibbon`
- 브랜치: `codex/windows-stash-stall`
- 구현 시작 start/base HEAD: `9753ed98d24ef0d679a2272543f4a93d8a11e870` (0.3.5, 미커밋 변경)
- `npm run dev:info`: instance `dev-0ea2e3d33bb9413ffea7e257`, binary `위 루트\src-tauri\target\debug\jajan.exe`.
- Windows x64, Rust 1.99.1, Cargo 1.99.1, Node 24.22.3, pnpm 11.25.0. 기본 PATH의 Node20 대신 기존 Codex runtime Node24를 사용했다. `pnpm install --frozen-lockfile` 성공, lockfile 변경 없음.
- 개발 CLI: 이 루트의 `scripts/jajan-dev.cmd`만 사용했다. `inspect.ok=true`, instance 일치 및 본문 고유표식 `STASH-WINDOWS-20261120-A`를 확인했다.
- 실행 파일 절대 경로를 Win32_Process로 대조했다. 최초 원본 빌드 PID30248, 임시 훅 원본 빌드 PID27044, 수정+훅 빌드 PID10572. 직접 시작한 해당 PID만 종료했다.
- President 등록 CLI는 Windows 미지원이므로 실행하지 않았다. 기본 체크아웃·다른 팀원의 소스는 변경하지 않았다. Sentry 멈춤 조사는 사용자 요청으로 별도 팀원에게 넘겼으며 이 변경의 해결 범위가 아니다.

## 원인과 최소 수정

`place_indicator`가 일반 메모의 `show_without_focus`를 호출해 `reset_for_reveal` → `sync_opacity` → `windows_window::opacity`로 진입했다. 안내창의 `set_ignore_cursor_events(true)`는 Tao에서 WS_EX_LAYERED/WS_EX_TRANSPARENT를 설정하지만 메모용 네이티브 alpha를 설정하지 않는다. 이 HWND에서 `GetLayeredWindowAttributes`가 실패하고, stash tick의 오류 복구가 `toggle`을 호출해 이미 숨긴 메모를 다시 펼쳤다.

안내창만 기존 `show_frame`으로 표시한다. Windows의 포커스를 뺏지 않는 표시 구현은 그대로 사용하며, 일반 메모의 alpha 조회 오류 처리와 stash 실패 복구를 완화하거나 제거하지 않는다. CSS 투명도(아이콘 .85), 위치·크기, 클릭 통과·비활성화와 애니메이션은 변경하지 않았다. 이전 로컬 커밋은 참고만 했고 cherry-pick하지 않았다.

[Microsoft 알파 조회 계약](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getlayeredwindowattributes)과 설치된 Tao/Tauri 소스를 확인했다. 네이티브 회귀 테스트는 WS_EX_LAYERED/WS_EX_TRANSPARENT만 설정한 실제 Win32 HWND에서 조회가 실패하고 명시 alpha를 쓴 뒤에는 조회되는 것을 확인한다. 이 테스트는 일반 메모의 실패를 숨기는 대신 안내창이 이 경로에 들어가면 안 되는 이유를 보존한다.

## 실환경 전후 비교

단축키 lease가 처음에 다른 담당에 점유된 것으로 보고돼 다른 인스턴스에 영향을 줄 수 있는 전역키 자동화를 사용하지 않았다. 임시 로컬 검증 훅을 `stash::install`에 넣어 이 인스턴스의 전용 signal 파일을 소비할 때 main thread에서 실제 `stash::toggle`을 실행했다. 신규 IPC/CLI 명령을 추가하지 않았다. 수정 전후 모두 같은 훅을 사용했고 시험 후 전체 훅을 제거해 최종 빌드를 다시 수행했다. 최종 변경에 환경변수·signal 처리·임시 훅은 없다. 따라서 아래 시험은 실제 native 창과 stash tick의 검증이며 트레이 클릭/실제 키 입력 전달 검증과 구분한다.

- 메모 ID `stash-regression-20261120`, 기존 숨김 대조 메모 ID `main`(이 인스턴스에서 생성한 시작 메모).
- 수정 전: signal 소비 후 1.8초 시점 `visible=true`, `stash.active=false`로 되돌아옴. 창 크기450×72 유지. 사용자 증상과 동일한 재표시 경로를 재현했다.
- 수정 후: 1.2초 시점 `visible=false`, `stash.active=true`, `animating=false`, `restoring=false`, `tip_visible=true`, 대상창 목록이 해당 메모 하나임을 확인했다.
- 3회 연속 복원→숨김: 복원마다 visible=true/active=false, 숨김마다 visible=false/active=true. 복원 위치(3390,2016), 크기450×72, pin=true, focused=false 유지. 각 복원 뒤 일반 자동 반투명도 .5를 확인했다. 첫 표본의 알파는 기존 fade 진행 중 .776이므로 저장 알파의 정확한 동등성 시험이라고 해석하지 않는다.
- 숨긴 상태 `update --change-mode replace`로 본문을 바꾸어도 숨김 유지. 다음 toggle에서 바뀐 본문으로 정상 복원.
- 원래 숨긴 `main`은 반복 시험 내내 visible=false 유지.
- GUI: PID10572와 같은 실행 경로의 `stash-indicator` 창 HWND4065954를 도구 반환값으로 특정했고 실제 마스코트 표시를 확인했다. 캡처는 `.tmp/indicator-fixed.jpg`(로컬 증거, 배포/커밋 대상 아님). 이전 메모 HWND4063476에서는 제목 ID·고유표식도 화면/inspect와 대조했다. 안내창 관찰은 활성화 없이 수행했다.
- 로컬 전후 JSON: `.tmp/baseline-before.json`, `baseline-after.json`, `fixed-hidden.json`, `cycles.json`, `hidden-update.json`, `final-restore.json`. 개인정보가 없는 전용 테스트 메모만 사용했다.

## 최종 검사

- `npm run dev:build`: 훅 제거 최종 코드 Windows 개발 빌드 성공.
- `node scripts/build.mjs cargo test --manifest-path src-tauri/Cargo.toml --quiet`: 305개 통과(Windows 실제 HWND 회귀 포함), 실패/건너뜀 없음.
- `npm run test:ui`: 229개 통과, 1개 Windows에서 POSIX executable fixture라 건너뜀, 실패 없음.
- `npm run check`: Clippy all-targets / -D warnings 통과.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`, `git diff --check`: 통과.
- 훅 제거 최종 바이너리 PID9228의 절대 실행 경로를 재확인했다. `stash-final-20261120`/`STASH-FINAL-20261120` 고유표식으로 open/inspect/close/update/inspect 확인: ok=true, 같은 instance, 숨긴 채 본문 갱신 유지. 해당 PID만 종료했다. 실제 stash 반복 검증은 앞의 동일 수정+임시 훅 빌드에서 수행한 것으로 구분한다.
- 최종 변경은 stash 표시 경로, Windows native 테스트, 디자인 시스템의 적용 상태와 이 검증 기록 네 파일이다. main.rs·의존성·빌드 설정·버전·개인정보 경계는 변경하지 않았다. 다른 팀원의 main.rs/디자인 시스템 변경과 통합 시 이 절의 문서 링크만 보존하면 된다.
- 위 검증 당시에는 미커밋 상태였으며 이후 사용자 승인에 따라 커밋·PR·병합을 진행한다. 공개 배포는 별도다. Conf4 / Risk Low(안내창의 기존 표시 함수 사용으로 국소 복구 가능).
- 2026-11-20 사용자가 “오케이, 해결됐어”라고 동작 해결을 보고했다. 해당 사용자 시험의 바이너리 출처·입력 경로는 별도로 재확인하지 않았으므로 에이전트의 위 실행 근거와 구분한다.

## 범위와 직접 확인

확정된 [디자인 기준](../design-system.md#전체-창-접어두기--복원--2026-11-04)을 그대로 구현했다. 외형·배치 변경이 없어 기존 window-stash SVG/PNG는 변경하지 않았다.

수동 확인은 이 워크트리에서 Node22.13+ 환경으로 `npm run dev:build` 후 `scripts/jajan-dev.cmd open --id stash-manual --markdown "숨김 수동 확인"`을 실행한다. 해당 개발 트레이의 숨김 메뉴로 숨기면 메모가 다시 커지지 않고 안내 마스코트가 남아야 한다. 다시 숨김을 선택하면 원래 위치·크기로 복원돼야 한다. 설치 앱 트레이와 구분한다.

미검증: 최종 훅 제거 바이너리의 트레이 클릭 및 실제 보조키 입력, 안내창 포인터 회피/클릭 통과의 실입력, 빠른 재토글, 혼합 DPI/다중 모니터, 이미지·터미널·외부 웹·미저장 초안, 핀 해제 창, macOS/Linux. 전체 플랫폼 CI·설치·배포는 수행하지 않았다. Windows 멈춤/Sentry panic 해결을 주장하지 않는다.
