# 로그인 자동 실행 구현·검증 — 2026-11-20

> 아래는 최초 구현 단계의 기록이다. 이후 승인된 설정 UI 간소화·커밋·병합과 배포 담당 분리는 [후속 검증 기록](autostart-settings-20261120.md)을 따른다. 화면은 최신 시안과 브라우저 캡처로 갱신했으며 설정 체크 변경은 즉시 적용된다.

## 작업 공간과 권한

- 작업 루트: `C:\Users\egoin\.codex\worktrees\login-autostart\agent-gibbon`
- 브랜치: `codex/login-autostart`
- 확인한 채팅: local `01a1f7a6-8a33-7ec1-a8f3-41ff496fbb51`, `[검토대기] 로그인 시 자동 실행` (호스트 도구에서 변경 후 재조회 확인)
- 승인 원본: 사이드 채팅 local `01a1f7a1-ad1f-7401-bf99-081da4fe6698`, userMessage `01a1f7a5-6e9d-7de1-a9c0-b5ab61a1513f` (2026-11-20); 팀장 배정으로 구현·검증 착수 승인 확인.
- 시작/base/현재 HEAD: `9753ed98d24ef0d679a2272543f4a93d8a11e870` (0.3.5)
- 개발 인스턴스: `dev-9ef74ed7f8f96bcad543b8f4`
- 실행 파일: 위 루트의 `src-tauri\target\debug\jajan.exe`
- 구현·검증만 승인된 작업이다. 커밋·push·PR·병합·배포·외부 메시지를 실행하지 않았다. 기본 체크아웃·기존 `.tmp`·설치 앱·다른 팀원의 워크트리를 수정하거나 종료하지 않았다. 공통 디자인 문서의 자동 실행 절만 추가했으며 세 줄 높이 작업과 통합 시 해당 문서의 양쪽 변경을 보존해야 한다.
- president `office.py`는 POSIX `fcntl` 기반이므로 Windows에서 실행·수정·우회하지 않았다. 등록부 저장·`check`·팀원 보고 갱신은 미실행이다. 전역 음성 스크립트도 같은 모듈 오류로 실행되지 않아 화면 안내로 진행했다. agent-gibbon 진행 표시 스킬은 현재 경로에 없으며 아래 체크리스트로 실행 근거를 남긴다.

## 계획과 결과

- [x] 전용 워크트리·브랜치·시작 SHA 확인, 디자인 시스템·기존 온보딩/설정 SVG·PNG 확인
- [x] 공식 플랫폼 문서 확인, 앱 선택과 OS 상태의 분리 및 명시 등록 경계 구현
- [x] 온보딩·설정 UI, 개발/검증 제외, 로그인 시작 모드 구현
- [x] Rust·UI 테스트, Windows Clippy·개발 빌드·실행 검증
- [x] 세 SVG 시안 갱신/추가·PNG 렌더링·시각 확인, 실제 UI 자산의 브라우저 검증
- [x] README·디자인 시스템·검증 범위 기록
- [ ] macOS 빌드와 실기기 등록/해제·승인 취소·로그인 시작 검증
- [ ] Windows 설치 앱의 실등록/해제·OS 직접 끄기·실제 로그인 검증
- [ ] 사용자 승인 후 커밋·통합·병합 (이번 승인 범위 밖)

## 구현 범위

첫 실행의 자동 실행 제안만 기본 체크다. 약관과 진단 전송 동의는 기존 기본 해제를 유지한다. Continue에서 약관 저장 성공 뒤 자동 실행 선택을 적용하며, 자동 실행 실패는 약관 수락을 취소하거나 진단 전송 동의를 대신하지 않는다. 실패는 전역 오류로 표시하고 설정에서 재확인할 수 있다. 기존 사용자의 미선택은 그대로 유지한다.

선택 파일은 앱 데이터의 `autostart/<instance 또는 installed>/choice.json`에 원자적으로 저장한다. 선택값(None/On/Off), OS 등록/허용 상태, 사용 가능 여부를 독립적으로 반환한다. 조회·일반 시작·업데이트에 등록을 복구하는 코드는 없다. 명시 적용 시 선택 파일 저장 실패는 OS 변경을 막고, OS 변경 실패는 저장된 의도와 실제 상태의 차이로 드러낸다. 자동 재시도하지 않는다.

macOS는 설치 번들에서 macOS 13 이상의 `SMAppService.mainAppService`를 사용한다. 승인 필요/거부 상태는 활성으로 표시하지 않는다. OS에서 비활성화된 등록을 On 요청으로 다시 등록하지 않고 OS 설정을 안내한다. 기존 앱의 macOS 10.15 최소 지원은 유지되지만 이 기능은 13 미만에서 사용 불가다.

Windows는 NSIS current-user 배포에 맞춰 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`의 `Jajan` 값 하나를 관리한다. 실행 파일 경로를 인용하고 `--login-startup`만 전달한다. 다른 실행 파일을 가리키는 항목은 덮어쓰거나 삭제하지 않는다. 이미 같은 항목이 있으면 다시 쓰지 않는다. `StartupApproved`는 문서화된 조회 API/바이너리 규격이 확인되지 않아 읽어서 추정하거나 수정하지 않는다. **따라서 Windows OS의 실제 허용 여부는 자동 판정하지 않으며 ‘등록됨 · Windows 허용 여부 미확인’이라고 표시하고 OS 시작 앱 설정 확인을 제공한다.** 이 제한을 OS 활성 확인 완료로 해석하면 안 된다.

debug·개발 인스턴스·`verification-updater` 빌드에서 OS 상태 조회·등록/해제를 하지 않는다. 로그인 시작은 초기 `open`·온보딩 관리 창 열기를 생략하고 기존 창 복원을 사용한다. macOS는 로그인 Apple Event를 확인하고 해당 활성화 알림에 관리 창을 열지 않는다. Windows는 `--login-startup`에서 콘솔을 분리한 뒤 UI 서버만 시작한다. 기존 터미널 복원 모듈을 바꾸거나 명령 재실행 기능을 추가하지 않았다.

## 수행한 검증

환경: Windows x64, Node 24.22.3, Cargo 1.99.1, rustc 1.99.1, Tauri CLI 2.12.1. 시스템 기본 Node는 20.11.0으로 엔진 요구보다 낮아 검사·빌드는 Codex 번들 Node 24의 PATH를 사용했다. 의존성은 전용 워크트리에서 frozen lockfile로 설치했다.

| 검사 | 실제 결과 |
|---|---|
| `node scripts/build.mjs cargo test --manifest-path src-tauri/Cargo.toml` | Rust 310개 통과 |
| `node scripts/test-ui.mjs` | 237개 중 236개 통과, 기존 skip 1개, 실패 0 |
| `node scripts/build.mjs cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 통과 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 통과 |
| `node scripts/dev.mjs build` (`npm run dev:build`와 동일 실행기) | Windows debug 빌드 통과 |
| `git diff --check` | 통과 |
| `node scripts/autostart-browser-smoke.mjs` | 실제 관리 UI 자산·mock IPC로 기본 체크/등록 없음/Continue/OS 승인 필요/Windows 미확인/Off/개발 제외/600×360/키보드 적용 통과; pageerror 0 |
| 세 SVG의 Chrome PNG 렌더 | 온보딩·설정 탐색·자동 실행 상태 시안 렌더 및 시각 확인 |
| `node scripts/autostart-runtime-smoke.mjs` | 아래 전용 Windows 개발 프로세스 실행 근거 참조 |

Rust 신규 검증은 개발/검증 제외 조합, 미선택과 Off의 재읽기, OS 거절 후 선택 보존·조회 시 재시도 없음, 파일 저장 실패 시 OS 미호출, 공백·한글 경로 인용/길이 제한, 등록/해제 계획, 기존 등록 보존·다른 경로 거부를 다룬다. 실제 Run 키를 쓰지 않는 순수 전이/임시 파일 시험이다. UI 신규 7개 테스트는 동의 분리·On/Off·OS 승인 필요·개발 제외·오류·초안 보존을 확인한다.

### Windows 실제 개발 프로세스

[원시 실행 근거](login-autostart-20261120/runtime.json)는 `dev:info` 경로/ID와 각 `inspect`의 `ok: true`·`instance`·검증 본문을 대조했다. 네이티브 창은 해당 실행 파일 경로로 조회한 PID에 속하고 제목에 검증 메모 ID가 있는 창으로 연결했다. 최종 재실행의 PID는 532→28880이다. 처음 수행한 신선한 인스턴스 시험(PID 17620→19104)에서는 초기 표시 창이 없었고 관리 창도 열리지 않았다. 최종 기록은 재실행 시험으로 기존 표시 메모 복원도 포함한다.

- `--login-startup` 직접 시작 전후 foreground PID 유지 확인. 재시작 복원 때도 다른 앱의 foreground PID 유지 확인.
- 새 검증 메모 두 개를 개발 CLI로 생성한 후 하나만 숨기고 세션 저널의 표시/숨김 반영을 기다렸다. 다시 `--login-startup`으로 시작한 뒤 하나는 표시, 하나는 숨김, 본문·위치·크기·핀 상태 보존을 확인했다.
- 네이티브 PID와 메모 ID가 포함된 창 제목·가시성을 확인했고, 보이는 관리 창이 없었다. 개발 진단 응답의 `autostart.available=false`, `os_status=unavailable`, `selected=null`을 확인했다.
- HKCU Run의 Jajan 항목은 전후 모두 없음(null)으로 유지됐다. 테스트가 시작한 정확한 PID만 종료했으며 다른 앱/워크트리 프로세스를 종료하지 않았다. 최종 검증이 만든 메모는 숨긴 뒤 종료했다. 테스트 데이터는 해당 개발 인스턴스에만 남는다.
- 실제 OS 로그인 호출, 설치 앱 설정 변경, 업데이트 설치, macOS 포커스는 수행하지 않았다. 모의 로그인 시작 경로의 실프로세스 확인과 OS 로그인 자체를 구분한다.

### 화면과 재현

- [온보딩 전체 내용](login-autostart-20261120/onboarding.png): 전체 내용을 한 장으로 확인하기 위해 캡처 동안 해당 패널의 스크롤 제한만 해제했다. 944×720 고정 창에서 전체가 한 번에 보인다는 뜻은 아니다. 동작 시험에서는 원래 스크롤을 복원했다.
- [Windows 허용 미확인 상태](login-autostart-20261120/settings-windows.png), [600×360 어두운 테마·Off](login-autostart-20261120/settings-minimum-dark.png): 실제 HTML/CSS/JS 자산, mock IPC다. OS 등록 증거가 아니다.
- [디자인 시스템](../design-system.md#로그인-자동-실행--2026-11-20), [자동 실행 SVG](../design/login-autostart.svg) · [PNG](../design/login-autostart.png), [온보딩 SVG](../design/privacy-onboarding.svg) · [PNG](../design/privacy-onboarding.png), [설정 탐색 SVG](../design/manager-settings-wireframe.svg) · [PNG](../design/manager-settings-wireframe.png).

브라우저 재현은 `PUPPETEER_MODULE`에 설치된 puppeteer-core ESM 파일 URL, `CHROME_BIN`에 Chrome 실행 파일, `AUTOSTART_OUTPUT`에 아티팩트 경로를 지정한 뒤 브라우저 스크립트를 실행한다. 검증 도구는 임시 폴더에 설치해 제품 의존성에 추가하지 않았다. 런타임 스크립트는 Windows와 빌드된 이 워크트리의 debug 바이너리가 필요하며 해당 개발 앱이 실행 중이면 시작을 거부한다. OS 등록은 수행하지 않는다.

직접 UI를 확인하려면 이 루트에서 `npm run dev` 후 트레이의 메모 관리 → 설정 → 로그인 자동 실행을 연다. 개발 앱에서는 등록 불가 안내와 비활성 체크박스가 정상이다. 첫 실행 UI 흐름만 확인할 때는 앱을 종료한 상태에서 `JAJAN_REQUIRE_TERMS=1`을 지정해 개발 앱을 시작한다. OS 실등록을 확인하려고 개발 보호를 해제하지 않는다.

## 한계·남은 판단

Windows 등록/해제 API 코드는 컴파일됐고 순수 전이 시험을 통과했지만 실제 사용자 Run 키의 쓰기는 실행하지 않았다. Windows OS 직접 비활성화·업데이트 이후 유지·실제 로그인은 보호된 별도 실기기 시험이 남았다. macOS Objective-C 브리지는 공식 API를 확인해 작성했으나 이 호스트에서 빌드·등록·승인 취소·Apple Event 타이밍을 검증하지 않았다. 특히 로그인 이벤트와 초기 setup/활성화 알림 순서는 Mac에서 확인해야 한다. 전체 플랫폼 CI·설치 파일 빌드·배포는 요청 범위에 따라 실행하지 않았다.

Windows 활성 상태 자동 판정이 별도 제품 요구라면 공식 지원 조회 수단 또는 사용자가 허용한 추가 접근을 정한 뒤 확장해야 한다. 현재 구현은 확인하지 못한 상태를 활성이라고 표시하지 않는다. macOS 10.15–12에도 이 기능을 제공해야 한다면 별도 호환 구현의 범위를 결정해야 한다.

**Conf 4:** Windows 모의 로그인 시작의 실프로세스 확인과 브라우저 흐름까지 통과했다. **Risk Medium:** 로그인 지속 등록과 시작 시 창 상태에 영향을 주며, OS 실등록·macOS 검증은 남았다. 구현 완료와 플랫폼 실등록 검증 완료를 구분한다.

## 공식 근거

- [Apple SMAppService](https://developer.apple.com/documentation/servicemanagement/smappservice): macOS 13+ 등록·해제·상태·OS 설정 진입.
- [Apple requiresApproval](https://developer.apple.com/documentation/servicemanagement/smappservice/status-swift.enum/requiresapproval): 등록됐지만 승인 필요, 사용자의 OS 동의 철회도 이 상태로 보고됨.
- [Apple 로그인 시작 이벤트](https://developer.apple.com/documentation/coreservices/1556410-launch_apple_event_constants/keyaelaunchedasloginitem): 로그인에 맞는 동작만 수행하고 새 문서를 여는 동작을 피할 근거.
- [Microsoft Run/RunOnce](https://learn.microsoft.com/en-us/windows/win32/setupapi/run-and-runonce-registry-keys): 데스크톱 앱의 사용자별 Run 등록과 260자 제한.
- [Microsoft 시작 앱 설정](https://support.microsoft.com/en-us/windows/experience/startup-boot/configure-startup-applications-in-windows): 사용자의 시작 앱 활성/비활성 설정.
- [Tauri Autostart](https://v2.tauri.app/plugin/autostart/): 공식 플러그인의 LaunchAgent/AppleScript 방식 대신 승인 상태를 직접 다루는 macOS SMAppService 브리지를 선택했다. Windows도 앱 선택·OS 등록을 분리하기 위해 작은 로컬 모듈로 구현했다.
