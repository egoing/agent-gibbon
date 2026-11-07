# #87 자동 업데이트 조사·검증

## 작업 출처

- 작업 루트: `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/87-auto-updater`
- 브랜치: `87-auto-updater`
- 최초 base: `48041c5e3dcdb6db98e136d0098ab30a6343666a` (`refs/is2/base/87` 보존)
- 개발 인스턴스: `dev-b820683081e41a2480b2200c` (`npm run dev:info` 확인)
- 환경: macOS ARM64, Node 24.21.0, npm 11.19.0, Rust/Cargo 1.91.1.
- baseline 원본: `/tmp/agent-gibbon-is2-87-baseline-20261107/`. 회귀(Rust 169개·UI 119개), 타입 검사, Clippy, Rustfmt, macOS ARM64 앱 빌드가 모두 exit 0으로 통과했다. baseline 제외는 0건이다. release 앱 번들 생성까지만 확인했으며 설치·실행 검증과 구분한다.
- 설치 앱을 교체하지 않았다. 화면/GUI·실제 업데이트·다른 OS 실행 검증은 미실시다. 아래 CLI 출처 확인은 baseline debug 앱에 대한 것이다.

## 사용자 결정과 미결정

2026-11-07: 자동 다운로드 후 재시작 또는 사용하지 않는 동안 설치하는 방향으로 결정했다. 사용자 원문: “자동 다운로드 - 재시작이나 사용자가 아무것도 안하고 있는 것 같을 때 업데이트 (재시작시복구가 불가능한 작업이 터미널일 것 같은데 복구가 가능할지 조사해줘)”.

macOS ARM64·Intel 및 Windows x64 구현을 진행하기로 했다. 작업 커밋·push·PR·CI와 무료 updater 서명 키 생성·등록을 별도로 승인받았다. Keychain(service `AGENT_GIBBON_TAURI_UPDATER`) 및 GitHub Actions secret/공개 variable 등록을 확인했다. Enpass 백업 항목을 저장 후 다시 열어 비밀 필드 길이와 공개 키·복구 설명을 확인했다. 마스킹된 비밀 키의 원문 일치·실제 복구는 미검증이다. 실제 설치 앱 교체, 태그·공개 릴리스는 실행하지 않았다. 위험도는 최소 Medium이며 자동 병합 대상이 아니다.

## 현재 터미널 복원 범위 — 코드 조사

| 대상 | 현재 동작 | 근거 |
|---|---|---|
| 제목·마지막 폴더·최근 폴더·영역 | 메타데이터 파일에서 복원. 폴더가 없어지거나 저장 실패한 경우 보장은 없음 | `terminal.rs`: `Terminal`, `save`, `read_records`, `restore` |
| 창 표시 | 사용자가 다시 열 때 창 생성. 앱 시작 시 터미널 자동 실행 없음 | `terminal.rs`: `restore`, `open`; `ui/terminal.js`: Welcome → 명시적 선택 → `terminal_start` |
| 화면·스크롤백 | WebView xterm 메모리에만 존재. 저장·재연결 없음 | `ui/terminal.js`: `new Terminal`, `output.onmessage` |
| 셸 상태·환경·미입력 확정 명령 | 프로세스/화면 상태이며 메타데이터에 저장하지 않음 | `Session`, `Terminal`의 저장 필드 |
| 실행 작업·백그라운드 작업 | 앱 종료 시 세션 제거, 셸/자식 강제 종료. 작업 복구 안 됨 | `main.rs` Exit → `terminal::shutdown`; `Session::drop` |
| 숨긴 터미널 | 숨김은 세션 종료가 아님. 업데이트 안전성 판단에서 제외하면 안 됨 | `terminal.rs`: `handle`의 Close |

Unix에서는 자손 PID와 foreground/process group에 SIGKILL을 보내며 Windows에서는 `taskkill /T /F` 후 child kill을 수행한다. 일반 PTY를 다시 만드는 것은 기존 작업 복원이 아니다. 셸 history 파일이 있더라도 명령 재실행은 부작용과 중복 실행을 만들 수 있어 복구로 취급하거나 자동 수행하지 않는다.

기존 `welcome::busy`는 프롬프트 토큰·전경 그룹·자식 유무를 사용해 개별 창 닫기 확인을 돕는다. 프롬프트 대기는 셸 변수·화면·입력 상태가 사라져도 된다는 승인과 같지 않다. 입력 부재·출력 부재만으로 실행 작업 종료를 판별하지 않는다.

## 설치 보호 조건 — 조사에서 구현으로 반영

- 자동 다운로드는 설치·재시작과 분리한다. 다운로드 후에도 서명 검증 완료 전 설치 가능 상태로 표시하지 않는다.
- 살아 있거나 시작 중인 PTY가 하나라도 있으면 자동 설치를 보류한다. 잠금 실패·자식 상태 조회 오류도 보류한다. 숨김 PTY도 포함한다.
- 편집 중 메모·저장/내보내기·캡처/영역 선택·창 전환 등 상태가 불명확하면 보류한다. 응답하지 않는 WebView를 안전 상태로 간주하지 않는다.
- idle은 보조 조건이다. 설치 직전 상태를 다시 확인하고 새 터미널/편집 시작과의 경쟁을 막는다. macOS CoreGraphics / Windows GetLastInputInfo로 5분 입력 부재를 확인한다. 재실행 시 이전 다운로드가 검증되면 안전 조건을 확인해 한 번 설치를 시도하고, 보류 후에는 idle 조건을 적용한다.
- 사용자가 명시한 재시작/업데이트에도 작업 손실 보호를 적용한다. 보류 이유와 사용자가 종료/저장 후 다시 시도할 경로를 제공한다.
- debug·개발 인스턴스의 자동 네트워크·설치를 차단한다. 약관 생략을 실제 수락·진단 동의로 취급하지 않는다.
- Windows updater 설치는 앱을 종료할 수 있으므로 설치 호출 전에 보호해야 한다. `on_before_exit`만으로 거절 가능한 사전 게이트를 대신하지 않는다.

## 터미널 조사에 대한 실행 근거

이번 baseline에서 `terminal::welcome::lifecycle_tests::real_shell_prompt_builtin_busy_and_child_cleanup`가 통과했다. 테스트 전용 PTY에서 셸 builtin 작업 실행 중 busy 판정, 세션 drop 후 셸 회수, background 자식 종료를 실제 확인하는 검사다. `welcome_metadata_preserves_only_last_path_title_and_geometry`, `approved_recent_folders_survive_metadata_roundtrip`, `legacy_logical_and_windows_physical_origins_survive_restore`도 통과했다. 기존 사용자 터미널은 종료하거나 변경하지 않았다. 이 결과는 현재 종료·메타데이터 동작의 근거이며 updater 재시작이나 tmux 재접속 검증은 아니다.

## tmux·외부 세션 생존 방식

[tmux 공식 매뉴얼](https://man.openbsd.org/tmux)은 세션을 소유한 서버와 접속 클라이언트가 별도 프로세스이며 로컬 소켓으로 통신한다고 설명한다. 클라이언트를 분리해도 서버가 살아 있으면 작업과 화면에 다시 붙을 수 있다. 서버 종료·호스트 재부팅까지 복원하는 기능은 아니다.

이는 현재 앱이 소유한 PTY/셸을 종료하는 구조와 다르다. 앱이 tmux를 내장·자동 시작하려면 별도 장기 실행 서버/의존성/세션 소유권·정리·재접속 정책이 필요하며 프로젝트의 “서버 추가 금지”와 충돌할 수 있다. Windows 네이티브 PTY와 동일하게 사용할 수도 있다고 가정하지 않는다. 사용자가 이미 관리하는 외부 tmux도 현재 자식 종료 정책에서 안전하다고 무조건 보장할 수 없다. 이번 업데이트는 tmux 자동 도입에 의존하지 않고 살아 있는 PTY를 설치 차단 조건으로 삼는 방향을 권장한다.

## 공식 updater 확인

[Tauri updater](https://v2.tauri.app/plugin/updater/) 확인일: 2026-11-07. 문서 최소 Rust는 1.90으로 로컬 1.91.1이 충족한다. 필수 업데이트 서명 검증, static JSON manifest, 다운로드·설치 분리 API를 사용 가능하다. Windows 설치는 앱 종료를 수반할 수 있다. 현재 0.2.7에는 updater가 없어 기능 포함 버전까지 한 번 수동 설치가 필요하다. 업데이트 서명은 Developer ID/공증·Windows 코드 서명과 다른 신뢰 경계다.

## TDD 첫 red — 보존 기록

`tests/updater-release.test.mjs`에서 updater 배포 준비가 서명된 업데이트 자산 누락을 거부하는지 검사한다. 플랫폼 선택에 의존하지 않는 부정 케이스다. `node --test tests/updater-release.test.mjs` 결과 2개 중 1개 PASS, 1개 FAIL(exit 1). 실패 식별자는 `updater release rejects installer-only assets without signed update bundles`이며 원인은 기존 `prepareAssets`가 updater 옵션을 무시하여 예외 없이 설치 파일만 허용하는 것이다. 기존 manifest 보존 케이스는 이미 통과한다.

원본 로그: `/tmp/agent-gibbon-is2-87-updater-release-red.log`, 명령·종료 코드: 같은 이름의 `.json`. 실패를 확인한 원래 테스트만 `35db450d9dc4231485254abd4185e175f122718e`에 커밋하고 `refs/is2/red/87`로 보존했다. 사용자 승인을 기다리는 동안 로컬 구현을 먼저 진행했으므로 스킬의 구현 전 red 커밋 순서와 차이가 있다. red 커밋에는 구현을 포함하지 않았으며 최초 실패 로그는 그대로 보존했다. Draft PR은 #90이다.

## 개발 CLI 환경 준비

`npm run dev:build` 통과. 해당 루트의 `scripts/agent-gibbon-dev list`는 미실행 앱 오류를 반환했고, 이어 전용 메모 `auto-update-worker-87-20261107`을 open하여 새 개발 앱을 시작했다. 기존 설치 앱·다른 워크트리는 종료하지 않았다. `inspect` 결과 `ok:true`, instance `dev-b820683081e41a2480b2200c`, 메모 ID와 본문의 `자동 업데이트 #87` 표식을 대조했다. 네이티브 프로세스 PID 93736의 실행 경로는 이 WORK_DIR의 `src-tauri/target/debug/agent-gibbon --serve`이다. 응답 `focused:false`. 원본 응답은 `/tmp/agent-gibbon-is2-87-progress-inspect.json`에 보존했다. GUI 조작/캡처는 아직 하지 않았다. 진행 메모는 유지하며 사용자가 숨기면 다시 open하지 않는다.

## 구현 검증 — 2026-11-07

- updater 2.14.1, Tauri CLI 2.12.1. Rust 버전 조건은 충족했다. 공식 정식 GitHub manifest만 사용하며 `requireSignedVersion`을 적용한다. 서명이 인증한 플랫폼 파일명까지 검사하여 같은 버전의 다른 CPU 패키지 교체를 거부한다.
- 배포 검증은 실제 Ed25519/Blake2b payload 서명과 trusted comment를 확인한다. 태그 배포는 `.pub` 파일과 저장소 공개 키 variable을 대조한다. dispatch는 runner별 일회용 키를 사용한다. 테스트용 공개 키·서명 fixture의 개인 키는 폐기했으며 운영 키와 무관하다.
- 신규 테스트: 서명 자산 누락/기존 manifest 보존, 안정 버전 비교, 변조/잘못된 키/교차 플랫폼 교체, 실제 CLI 서명 호환, 설치 예약/새 작업 경쟁, 모든 창의 현재 epoch ack, 입력 차단/해제, 설정 UI 상태/중복 요청을 검사했다. `node --test tests/updater-release.test.mjs` 7개 통과. Rust updater 검사 5개 통과.
- 전체 로컬 결과: Rust 179개·Node UI 140개, `cargo check --all-targets`, Clippy `-D warnings`, Rustfmt 통과. baseline 제외·skip는 0. 원본 `/tmp/issue87-rust-tests.log`, `/tmp/issue87-ui-tests.log`, `/tmp/issue87-check.log`, `/tmp/issue87-clippy.log`, `/tmp/issue87-fmt.log`.
- macOS ARM64 앱 번들 빌드 통과(`/tmp/issue87-build.log`). 설치·공개 배포와 구분한다. 최종 소스 변경 후 같은 빌드를 재실행해 통과했다.
- 독립 검토: 구현 이력을 상속하지 않은 검토자가 전체 diff·red·로그를 읽고 테스트 완화/실제 테스트 퇴행/스코프 이탈 미발동을 확인했다. 예비 검토의 startup 재표시·stash 예약·배포 서명 검증·플랫폼 바인딩 지적을 수정하고 재검토했다.

## 실제 개발 앱 재시작 복원 검사

설치 업데이트를 실행하지 않고 테스트 개발 인스턴스의 복원 파일로 창 복원을 검사했다. 작업 루트와 인스턴스는 상단 기록과 동일하다. 검증 메모는 `updater-verify-87`, 고유 본문 표식은 `issue87-native-20261107`이다. `inspect.ok=true`와 응답 instance, 실제 프로세스 실행 파일을 대조했다.

1. PID 76143의 실행 파일이 이 워크트리 debug 바이너리인지 확인하고 검증 메모를 숨겼다. main·검증 메모의 position/size/pin/visible을 저장한 뒤 이 프로세스만 종료했다. 기존 앱·다른 워크트리는 유지했다.
2. 앱이 멈춘 동안 해당 개발 저장소의 `.update-windows.json`에 테스트 snapshot을 만들고 `npm run dev -- --no-watch`로 인자 없는 GUI startup을 실행했다. 최초 검사에서 macOS가 resize 때 Y 좌표를 이동시키는 실패를 확인했다.
3. size → position 순서로 수정하고 동일한 원래 좌표로 재실행했다. 최종 PID 91272, `lsof -p 91272 -d txt`로 실제 바이너리 경로를 확인했다. main과 검증 메모 모두 위치·크기·고정·숨김이 정확히 일치하고 snapshot이 소비된 것을 확인했다. 일반 시작 경로가 숨긴 main을 다시 열지 않았다.
4. 개발 CLI diagnostics의 `updater.enabled=false`, `status.phase=disabled`를 확인했다. 개발 환경 약관 생략은 진단 전송 동의로 바꾸지 않았다.

원본: `/tmp/issue87-restore-expected.json`, `/tmp/issue87-restore-result.json`, `/tmp/issue87-native-launch.log`, `/tmp/issue87-native-inspect.json`. 최초 실패를 성공으로 기록하지 않으며 수정 뒤 동일 조건 재검증만 PASS다. 네이티브 창 출처를 Orca 접근성 트리의 PID·메모 제목·고유 본문과 대조했다. 새 설정 패널은 VM 테스트와 SVG→PNG 렌더 검증을 수행했으며 실제 네이티브 설정 패널 조작은 미검증이다.

## 제한과 원격 검증

- 살아 있는 PTY뿐 아니라 아직 열린 터미널 Welcome 창, 접힌 메모, 진행 중인 layout도 보수적으로 설치를 보류한다. 터미널을 명시적으로 종료하고 메모를 펼친 뒤 다시 시도한다. 과거 터미널 메타데이터만 남은 상태는 차단하지 않는다.
- 실제 설치 앱 교체·새 버전 공개, Windows/Intel 실제 GUI 실행은 하지 않았다. 원격 3개 플랫폼 CI는 검토 커밋 push 후 실행하며 결과·정확한 SHA·실행 URL은 [PR #90](https://github.com/egoing/agent-gibbon/pull/90)의 검증 표에 남긴다. 이 문서 작성 시점의 원격 결과를 통과로 선기록하지 않는다.
- 위험도 Medium 이상, 자동 병합하지 않는다. 디자인 기준과 시안은 [디자인 시스템](../design-system.md#자동-업데이트-설정--87-2026-11-07)을 따른다.

## is2 커버리지 대조

2026-11-07 실사용 #87 / PR #90, 구현·검토 Codex GPT-6 Astra, 반복 UI·배포 작업 GPT-6 Luna. 이번 실행은 Medium 프런트엔드·백엔드, 구현 이력 없는 독립 감사와 수정 재검토, 실제 macOS 복원 실패/수정/재검증의 근거다. 자동 병합·충돌 rebase·서버/DB 마이그레이션은 실행하지 않았다. 공유 스킬 원본 유지보수는 요청 범위가 아니므로 커버리지 대조 결과를 이 프로젝트 기록에 남긴다.

### 오프라인 재확인 회귀 수정

독립 감사에서 다운로드 완료 상태의 수동 재확인이 오프라인으로 실패하면 설치 가능한 상태가 사라지는 문제를 발견했다. 기존 ready 상태를 복구하도록 수정했다. `/tmp/issue87-offline-red.log`에서 오류 상태와 ready 기대값의 실패를 확인한 뒤 전체 Rust 179개 및 정적 검사·앱 번들을 다시 통과했다. 이전 SHA `db5c337ed6f00133526d00195a5b2771e72f4ac1`의 CI [42324640587](https://github.com/egoing/agent-gibbon/actions/runs/42324640587)는 이 수정 반영을 위해 취소했으며 통과로 계산하지 않는다.

### 원격 서명 빌드 실패와 수정

첫 최종 SHA `f8d63d5b8d3372e50b5cfebf87c72aa1e6dc7f5c`의 [CI 42325527425](https://github.com/egoing/agent-gibbon/actions/runs/42325527425)는 macOS ARM64의 테스트·Clippy·앱/DMG 생성 후 updater 서명에서 `failed to decode pubkey: Missing comment in public key`로 실패했다. `TAURI_UPDATER_PUBLIC_KEY`를 Rust 빌드에는 전달했지만 Tauri CLI 설정의 빈 공개 키를 덮어쓰지 않은 원인이다. 이 실패를 통과로 계산하지 않는다.

`updater-build-config.mjs`가 환경의 공개 키를 JSON overlay의 `plugins.updater.pubkey`에 정확히 전달하도록 고쳤다. 값 보존·키 누락 거부 테스트 2개 및 전체 Node 140개, workflow YAML·11개 bash 구문 검사를 통과했다. 독립 감사자는 추가 blocker와 테스트 완화·baseline 퇴행·스코프 이탈이 없음을 확인했다. 운영 키를 교체하지 않았으며 재시도 결과와 최종 SHA는 PR에 기록한다.

일회용 키를 생성해 실제 로컬 `tauri build --bundles app --config <overlay> -- --locked`를 실행했다. 앱·updater tar.gz·서명 파일 생성과 종료 코드 0을 확인했다(`/tmp/issue87-local-sign-build.log`). 테스트 개인 키는 임시 디렉터리 정리와 함께 폐기했고 설치된 앱은 교체하지 않았다. Windows 첫 CI도 동일한 공개 키 누락 오류임을 로그로 확인했다.
