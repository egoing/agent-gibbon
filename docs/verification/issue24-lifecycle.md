# #22·#24 터미널 상태 저장과 PTY 종료 대기

- 기준 main 22dc7de. 메모와 기존 앱을 사용하지 않고 독립 macOS 프로세스에서 검증한다.
- 종료 후 출력·폴더·시작 예약 계약과 자식 프로세스, PTY 핸들의 수명을 분리한다. EOF, IPC 단절, 자식 종료를 독립적으로 다룬다.
- 종료된 자식에 대한 추가 신호를 차단하고 전역 잠금 밖에서 정리한다. 전체 대기 상한과 잔여 작업을 기록한다.
- 직접 실행하는 격리 fixture와 상태 전이 검사로 출력 순서, 재사용된 ID, 자원 회수, 다른 프로세스 보존을 검증한다.
- 로컬 결합 검토본에 #22의 창별 기하 저장과 PTY resize를 함께 적용했다. 원격 Draft에는 계획 커밋만 있다. #22는 독립 후보로도 선택할 수 있다.

## 기준과 유효성

2026-11-16 KST. 전용 작업 루트는 `/Users/egoinglee/.codex/worktrees/926a/agent-gibbon`, 개발 인스턴스는 `dev-376944ee536447977a200bac`다. 원격 main과 워크트리의 시작 커밋은 `22dc7de7309f8d47772bb0cf1f2c88768f742ef8`였다. 기존 열린 이슈를 사용했다. 관련된 #22의 Draft와 병합된 터미널 복원 PR을 확인했다. [이슈 #24](https://github.com/egoing/agent-gibbon/issues/24) · [계획 Draft #212](https://github.com/egoing/agent-gibbon/pull/212)

기준 코드에서는 reader가 끝나도 맵의 Running 슬롯이 master와 writer를 소유했다. 앱 종료는 세션 맵을 잠근 채 비웠다. 기존 OwnedChild는 별도 스레드에서 wait했지만 Session 소멸자의 마지막 kill은 이전 try_wait 결과와 무관했다. 캐시에 설치된 portable-pty 0.9.0의 Unix ChildKiller는 std::Child::kill 앞에서 숫자 PID에 SIGHUP을 보낸다. 이 경로는 소스와 kill spy로 확인했다. PID 재사용은 유발하지 않았다.

## 수명과 사용자 동작

| 상황 | 검토본 동작 |
|---|---|
| 정상·비정상 자식 종료 | 감독 스레드가 직접 자식을 회수한다. reader가 마지막 바이트를 배출해야 경량 Exited 상태를 게시한다. inspect는 종료 코드와 가능한 종료 신호를 반환하며 이 상태의 pid·program·pty_size는 null이다. |
| EOF가 먼저 도착 | 자식 종료를 계속 관찰한다. EOF만으로 종료 신호나 종료 문구를 보내지 않는다. |
| 자식이 먼저 종료 | slave가 열려 있으면 출력 배출을 계속한다. 살아 있는 WebView에는 마지막 데이터 뒤 같은 채널로 기존 종료 문구를 보낸다. |
| IPC 단절·창 숨김 | 수신 실패 후에도 출력을 배출한다. 원문은 저장하지 않는다. 같은 WebView의 xterm 스크롤백과 마지막으로 확인한 폴더는 유지한다. 사라진 WebView의 버퍼를 재구축하지 않는다. |
| 종료된 창 유지 | 재시작·Welcome 작업 폴더 변경·목록 삭제는 명시적으로 창을 종료할 때까지 막는다. backend는 write를 거절하고 경량 상태의 resize는 성공으로 처리한다. 출력 화면을 보호하기 위해 Exited도 기존 업데이트 차단 대상이다. 창을 닫아 슬롯이 사라져도 reader·감독 작업이 끝나기 전에는 업데이트를 차단한다. |
| 세션 교체·시작·삭제 | 예약부터 발행까지 같은 토큰을 사용한다. reader는 대상 레코드를 잠근 뒤 토큰을 확인하고 세션 맵을 푼 상태에서 폴더·계획을 저장한다. 새 세션은 이전 reader의 종료 갱신을 받지 않는다. 시작 중인 슬롯과 발행된 슬롯의 닫기 확인은 구분한다. 닫기와 삭제의 OS·파일 호출 동안 같은 ID의 새 시작을 막는다. |
| 앱 종료 | 새 시작을 차단하고 전체 맵을 분리한 뒤 소유 프로세스만 정리한다. 최종 폴더·복원 점검에 공유 75ms를 주며 입력 잠금을 기다리지 않는다. 그 점검이 지연되어도 자식 정리는 시작한다. 전체 PTY 단계의 기다림은 세션 수에 관계없이 하나의 750ms 기한을 사용한다. |

750ms는 PTY 정리 단계에서 공유하는 대기 예산이다. 운영체제 스케줄링 오차가 있고, 직접 자식 상태 확인 실패나 OS 호출 중단이 있으면 정리가 남을 수 있다. 같은 앱에서 최종 폴더와 복원 적격성을 기록하는 작업은 이 기한 후에도 계속한다. 마지막 기록 생산자가 끝나면 입력을 닫고 기하와 실패한 저장을 flush한 뒤 저장 작업자의 종료를 기다린다. 파일 I/O와 join, 창 상태 체크포인트 및 telemetry 종료에는 이 시간 상한이 적용되지 않는다. 세션마다 감독 스레드 하나가 100ms 간격으로 상태를 확인한다. 입력 역압력은 별도 이슈 #21의 범위다.

## 프로세스 소유권과 호환성

- macOS에서 portable-pty는 spawn 전에 setsid를 실행한다. waitid의 WNOWAIT로 앱이 직접 소유하고 아직 reap하지 않은 PID만 기준으로 삼는다. 같은 유효 UID의 숫자 PID 중 전용 SID 구성원만 선택한다. task name port로 커널 audit token을 얻고 SID를 다시 확인한다. 직접 자식의 종료 뒤에는 당시 확보한 토큰만 보존한다. 자연 종료에는 하위 작업에 신호를 보내지 않는다. 명시적 창 종료나 앱 종료에서만 세대를 확인하는 신호 API를 호출한다. 숫자 PID를 이용해 이전 SID·PGID·descendant를 재탐색하거나 그룹 전체를 종료하지 않는다.
- macOS 15.6.1 ARM64에서 일반 fixture와 ad-hoc hardened runtime fixture가 모두 통과했다. Xcode SDK 26.2의 공개 헤더 `usr/include/libproc.h`에 `proc_signal_with_audittoken` 선언이 있으나 이 선언에는 최소 OS 버전 표시가 없다. XNU 6153.141.1의 공개 libproc wrapper에서는 같은 심벌을 찾지 못했다. 앱의 최소 지원 macOS 10.15에서 하위 작업 정리까지 보장할 근거가 없다. 심벌 없음, 토큰 취득 실패, 신호 실패 시 숫자 PID fallback을 사용하지 않고 미확인 경고를 남긴다. 공식 Developer ID 서명·공증 앱과 Catalina 실기기는 미검증이다. [Apple의 현재 libproc 구현](https://github.com/apple-oss-distributions/xnu/blob/main/libsyscall/wrappers/libproc/libproc.c) · [XNU 6153.141.1의 libproc](https://github.com/apple-oss-distributions/xnu/blob/xnu-6153.141.1/libsyscall/wrappers/libproc/libproc.c)
- 스냅샷 이후 생성된 손자 프로세스, 별도 세션으로 분리된 작업, 다른 UID, exec으로 세대가 바뀐 프로세스는 종료 범위에서 빠질 수 있다. 전체 프로세스 트리의 종료 보장은 보류한다. 일반 종료 화면이 이미 열린 경우에도 보존한 작업이 있을 수 있으면 기존 확인 대화를 사용한다.
- Windows는 portable-pty의 WinChild HANDLE을 유지하고 try_wait가 살아 있음을 확인한 경우에만 시스템 디렉터리의 taskkill.exe에 기존 /PID, /T, /F를 전달한다. 외부 유틸리티 대기는 정리 스레드에서 수행한다. 이미 끝난 루트를 PID로 다시 찾지 않는다. Windows 표준 라이브러리와 portable-pty 0.9.0을 사용해 변경 helper 3개의 Windows cfg를 타입 검사했다. Windows 제품 링크·실행·ConPTY EOF·실제 트리 종료는 미검증이다. Job Object 전환은 제품 검토본에 없다. Linux 실기기 및 기존 그룹별 종료 호환성도 미검증이다.

## #24 단독 후보의 시험과 재현

macOS 15.6.1 ARM64, rustc·Cargo 1.91.1, Node 24.21.0, pnpm 11.25.0, Clang 17.0.0. 테스트 바이너리를 절대 경로로 직접 exec하고 빈 환경에 fixture 모드 하나만 전달했다. 작업 폴더는 각각 임시 디렉터리다. PTY 원문, 환경, 사용자 프로필을 시험 로그에 남기지 않았다. 실제 메모·GUI·설치 앱·실행 중 터미널을 열거나 조작하지 않았다.

- 관련 Rust 24개 통과. 별도 독립 측정 1개 통과. 셸 프로필 실행을 피하려고 기존 대화형 셸 테스트 4개를 제외했으며 다른 제품 전체 Rust 테스트는 실행하지 않았다.
- 자식의 정상 종료, exit 23, SIGTERM, 선행 EOF, 늦은 slave EOF, 256KiB 출력과 지연된 소비, 단절된 채널, 세대 교체 및 미회수 작업의 업데이트 차단을 검증했다. FFI 호출을 실행하지 않는 kill spy로 이미 회수된 자식과 상태 확인 실패 시 신호 0회, 확인된 생존 시 신호 1회를 확인했다.
- 8개 실제 자식, 잠긴 세션, 미완료 Starting 예약을 함께 유지한 시험에서 기한 후 반환했고 8개 직접 자식은 모두 reap되어 있었다. 별도로 실제 PTY를 읽지 않는 자식에 1MiB를 써서 입력 쓰기가 멈춘 상태를 확인한 뒤 종료했다. 최종 메타데이터 콜백을 기한 너머까지 막은 시험에서도 직접 자식은 이미 회수됐다.
- 직접 자식과 별도 foreground/background 그룹이 HUP을 무시하는 경우, 부모가 먼저 reap된 경우, 종료 상태에 도달해도 하위 작업이 살아 있는 경우를 시험했다. 각 시험의 자체 sentinel은 대상 정리 직후 살아 있었고 시험 종료 때 별도로 회수했다. 기본 controlling tty는 Darwin에서 leader 종료 시 조기 hangup될 수 있어 slave 유지 검사는 별도의 noncontrolling PTY fixture에서 수행했다.
- UI 199개, Rustfmt, Clippy all-targets, macOS 개발 빌드가 통과했다. 재생성 WebView에서 종료 코드 0과 기존 자동 복원 플래그가 함께 와도 새 명령을 시작하지 않는지 JavaScript fixture로 검사했다. 실제 Tauri IPC 렌더·IME·네이티브 화면은 미검증이다. 개발 바이너리의 instance-info와 manifest 경로만 확인했으며 실행 중인 서버의 inspect·GUI 출처 검증으로 간주하지 않는다.
- 48개의 실제 세션을 같은 프로세스에 만들고 종료 상태 48개를 유지했다. 12개마다 안정화 뒤 측정한 네 시점 모두 FD 4개, 스레드 2개였다. RSS와 실제 PID·바이너리 해시·명령·실패 판정은 영속 검토 패키지에 보존했다. 장시간 메모리 누수 부재를 보장하는 시험은 아니다.

```sh
cargo fmt --manifest-path src-tauri/Cargo.toml --check
npm run check
npm run test:ui
npm run dev:build
npm run dev:info
./scripts/agent-gibbon-dev --instance-info
node scripts/build.mjs cargo test --manifest-path src-tauri/Cargo.toml terminal:: -- --nocapture --test-threads=1 \
  --skip bash_prompt_commands_and_blank_lines_do_not_replace_the_user_command \
  --skip real_shell_prompt_builtin_busy_and_child_cleanup \
  --skip restore_runs_edited_command_in_real_shell_and_reports_program \
  --skip shell_command_exit_returns_to_same_pty_and_cwd_is_observed
node scripts/build.mjs cargo test --manifest-path src-tauri/Cargo.toml repeated_owned_sessions_release_handles_and_workers -- --ignored --nocapture --test-threads=1
```

## #22와 결합한 소스의 검증

창별 120ms 저장 기한과 원자 교체, 명시 요청의 같은 값 재시도를 유지한다. 모든 메타데이터 편집은 직렬화된 쓰기 순서 안에서 최신 레코드를 사용한다. 시작·입력은 해당 세대를 확인한다. 늦은 reader와 자연 종료 알림은 동일 ID의 새 세션에 영향을 주지 않는다. 자식 상태·작업 폴더 관측은 레코드 잠금 밖에서 실행한다. 명시적 닫기는 마지막 폴더·복원 상태를 관측하고 네이티브 창 파괴가 성공하면 같은 레지스트리 잠금 아래 슬롯과 최종 저장 책임을 인수한다. 파일 잠금을 얻기 전에 Control을 해제해 감독 작업에 종료를 통지한다. 닫기 가드는 저장이 끝날 때까지 동일 ID의 시작을 막는다. 앱 종료가 슬롯 인수보다 빠르면 앱 종료 작업이 최종 상태를 기록한다. 명시적 닫기가 먼저면 앱 종료 작업은 해당 저장이 끝난 뒤에 기록 입력을 마감한다.

| 결합 소스의 시험 | 실제 결과 |
|---|---|
| 느린 실제 파일 쓰기 + 진행 중인 PTY, 잠긴 세션, Starting 및 Exited | 쓰기를 차단한 채 직접 자식 두 개의 회수를 757ms에 확인했다. 120ms 더 기다려도 flush는 완료되지 않았다. 쓰기를 재개한 뒤 885ms에 마지막 폴더와 x=89, 물리 위치 플래그 및 네 세션의 복원 적격성 해제를 디스크에서 확인했다. 저장 마감 이후 명시 편집과 종료 중 새 시작도 거부됐다. |
| 명시적 닫기와 후속 앱 종료 | 수정 전, 파일 잠금 대기에서는 창 파괴 뒤에도 자식이 남았지만 앱 종료가 슬롯을 찾아 회수했다. 슬롯 제거 뒤 persist 정체에서는 앱 종료의 PTY 대기 반환 시점에도 자식이 남았다. 수정 후에는 두 상황 모두 저장이 정체된 채 앱 종료 **전**에 직접 자식의 회수를 확인했다. 입력 잠금이 없는 경우에는 마지막 cwd도 파일에 남았다. 세 경우를 각각 5회 반복했다. 실제 파일·감독 스레드·자식을 사용했고 네이티브 창 파괴만 대역을 썼다. |
| 동일 ID 재사용과 쓰기 순서 | 자연 종료한 실제 자식의 세대를 보존한 채 파일 쓰기를 막고 새 세대를 예약했다. 예전 reader와 종료 알림이 새 계획·복원 적격성·기하를 덮지 않았다. 같은 값 실패 재시도와 관측 억제, 오래된 쓰기 선정 후 삭제·재생성, 고빈도 창의 고정 기한 및 먼저 만료된 창 우선을 검증했다. |
| 자원 및 저장 횟수 | 8개 실제 자식과 미완료 시작 예약의 공유 대기는 760ms. 48세션을 12개씩 종료한 뒤 네 지점 모두 FD 4개·스레드 2개. 단일 창의 기하 이벤트 240건은 파일 쓰기 1회, 네 창의 960건은 4회였고 마지막 좌표가 모두 일치했다. |
| 검사 범위 | Rust 283건, 별도 측정 2건, UI 205건, 실제 xterm·Chromium의 테마·크기·배율 18개 조합, Rustfmt, Clippy all-targets, macOS 개발 빌드 통과. Chromium은 native IPC를 모의했다. 대화형 셸 프로필 시험 4건을 제외했고 네이티브 앱과 설치본은 실행하지 않았다. |
| 프로세스 식별 | macOS 15.6.1 arm64 일반 및 ad-hoc hardened fixture에서 foreground/background·고아·종료 이후 하위 작업을 확인했다. 별도 sentinel은 세 경우 모두 생존했다. Windows helper 3개는 실제 대상 표준 라이브러리로 타입 검사했다. |

Catalina의 audit-token API 가용성, 공식 서명·공증 앱과 Windows ConPTY의 실제 종료는 확인이 필요하다. 이 판단 전까지 #24의 제품 병합을 보류한다. #22 단독 후보는 기존 PR #209에서 별도로 검토할 수 있다. [#22 단독 검증](issue22-terminal-geometry.md) · [디자인 시스템](../design-system.md)
