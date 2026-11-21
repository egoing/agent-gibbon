> 보존된 과거 실험 기록입니다. 분리 기능은 [장기 과제 #279](https://github.com/egoing/agent-gibbon/issues/279)로 유예하고 이 작업 브랜치에서 실행 경로를 제거했습니다. 본문의 지원·검증 설명은 실험 당시 범위이며 현재 기능 활성화를 뜻하지 않습니다. [선별 제거 보고서](pty-separation-deferred-20261122.md)를 참고하세요.

> 역사적 debug 실험 기록입니다. 현재 활성화 수명과 정식 빌드 지원은 [release 검증](windows-pty-owner-release-20261121.md)을 따릅니다.

# Windows ConPTY 분리 소유자 개발 실험

> 아래는 최초 검증 기록이다. 이후 정상 GUI 프로세스 종료와 실제 NSIS 업데이트 재접속을 추가 확인하고 실행 파일 분리·실험 모드 전달을 보완했다. 현재 결과와 실행 방법은 [후속 검증 기록](windows-pty-owner-followup-20261121.md)을 따른다.

2026-11-21 KST. 기본 배포 동작을 바꾸지 않는 debug opt-in 구현이다. 앱 프로세스 강제 종료 후 기존 PowerShell PID와 변수 값을 유지한 자동 재접속을 실제 Windows GUI에서 확인했다. 정상 종료의 백엔드 분기와 IPC 연결 해제는 통과했으나, GUI 트레이의 정상 종료 메뉴를 직접 누르는 종단 검증 및 실제 업데이트 설치·재시작은 미검증이다.

## 작업 출처

| 항목 | 값 |
|---|---|
| root | `C:\Users\egoin\.codex\worktrees\04fb\agent-gibbon` |
| branch | `codex/windows-pty-manager` |
| base / 조사 시 origin/main | `974197adbc34f2f72872eb4e6262e270db90ef2d` |
| instance | `dev-9f4318a9190cf96216037515` |
| binary | `C:\Users\egoin\.codex\worktrees\04fb\agent-gibbon\src-tauri\target\debug\jajan.exe` |
| GUI 시험 세션 | `terminal-90320261121` |
| 관리 프로세스 / 셸 PID | `32408` / `32148` |
| GUI 종료 전 / 재실행 PID | `3620` / `11624` |
| 정상 유지한 설치 앱 | `C:\Users\egoin\AppData\Local\Jajan\jajan.exe`, PID `28264` |
| 도구 | Cargo 1.99.1, Node 24.22.3, pnpm 11.25.0 |

`AGENTS.md`와 로컬 `AGENTS.override.md`의 개발 빌드 정책이 이 워크트리에 존재함을 확인했다. 다른 체크아웃의 미커밋 변경은 조작하지 않았다. 커밋·push·병합·릴리스는 실행하지 않았다.

## 설계와 경계

Mac 기준 커밋은 Python 소유자가 Unix PTY를 열고 GUI 안의 임시 전경 클라이언트가 바이트를 중계하는 PoC다. 64KiB 출력 꼬리와 짧은 detach lease가 있으며, Windows ConPTY·resize·Job Object 경로는 없다. Windows에서 Python PTY를 흉내 내지 않고 같은 Rust 실행 파일의 debug 전용 `--pty-owner` 모드가 실제 ConPTY·셸 핸들을 소유한다. GUI는 `portable-pty` 인터페이스의 IPC 어댑터를 사용한다.

- GUI 앱 정상 종료는 기존 `Event::Quit`에서 detach한다. 터미널 명시 종료 및 시작 실패의 정리는 stop을 보낸다. GUI가 죽으면 관리 프로세스가 실제 클라이언트 process HANDLE의 종료를 감지한다. HANDLE을 보유하므로 PID 재사용으로 연결 생존 여부를 오판하지 않는다.
- 소유자가 셸 생성 **전** 자신을 비상속 Job Object에 넣고 `KILL_ON_JOB_CLOSE`를 설정한다. 셸과 처음부터 생성한 자식은 같은 Job 계층에 속한다. 명시 종료·lease 만료·소유자 사망 시 그 작업만 정리한다. GUI의 Job에는 새로 넣지 않는다.
- 소유자 시작은 `DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_BREAKAWAY_FROM_JOB`을 우선 시도한다. 현재 실행기의 Job이 breakaway를 거부하면 breakaway 없이 재시도한다. 상위 Job 전체 강제 종료나 `taskkill /T`로 소유자까지 지정하는 경우 생존을 보장하지 않는다. Job 설정 실패는 셸 생성 전에 실패한다.
- 워크트리별 AF_UNIX IPC를 사용한다. 별도 TCP 서버·네트워크 포트는 없다. IPC 하위 디렉터리를 현재 사용자 SID만 허용하는 protected DACL로 원자 생성하고, 기존 디렉터리도 DACL 일치를 검사한다. reparse 경로를 거부한다. 현재 사용자와 동등한 권한의 악성 프로세스까지 격리하는 보안 경계는 아니다. 다른 사용자 토큰의 실제 접근 거부 실험은 하지 않았다.
- ID 해시별 파일 잠금이 중복 소유자를 거부한다. 한 번에 하나의 클라이언트만 붙으며 메모리의 임의 capability가 입력·출력·resize·stop 요청을 제한한다. capability·명령·출력은 로그에 기록하지 않는다.
- 출력 전용 스레드는 GUI 접속 여부와 무관하게 ConPTY를 비운다. 최신 64KiB만 메모리에 보관하므로 오래된 출력은 유실될 수 있고 UTF-8/ANSI 경계 및 전체 화면 상태는 복원하지 않는다. 입력은 별도 스레드의 8×8KiB 제한 큐로 보내며 포화 시 오류를 반환한다. IPC 요청·응답에도 크기 제한과 2초 read/write timeout이 있다.
- `portable-pty`의 ConPTY는 초기 커서 위치 질의를 사용한다. GUI 없이도 초기 협상이 끝나도록 소유자가 최초 질의 한 번을 소비하고 `1;1` 응답을 보낸다. 이후 터미널 질의는 기존 GUI가 처리한다. 초기 검사 실패에서 이 누락을 발견해 고쳤다.
- GUI는 살아 있는 소유자가 확인된 경우 `reattach` 전용 경로를 호출한다. 조회 이후 소유자가 사라져도 새 소유자나 셸을 만들지 않는다. 기존 소유자에 초기 명령이 포함된 재접속 요청은 거부한다. 셸을 재실행해 생존으로 보고하지 않는다. 재접속 시 새 화면의 cell 크기를 소유자에 전달하며 기존 prompt token을 계속 사용한다.
- 미접속 유예 시간은 기본 300초이며 `JAJAN_POC_DETACH_SECONDS`로 1~900초를 선택한다. 최초 접속이 없으면 5초 후 종료한다. 재부팅·로그아웃·유예 만료 이후의 프로세스 복원은 제공하지 않는다.

ConPTY 채널을 별도 스레드에서 비워야 하는 이유와 종료 교착 조건은 [Microsoft ConPTY 세션 문서](https://learn.microsoft.com/en-us/windows/console/creating-a-pseudoconsole-session), Job의 자식 상속·종료 범위는 [Microsoft Job Objects 문서](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects)를 참고했다. 기존 기본 동작과 외형은 유지하며 [디자인 시스템](../design-system.md)의 실험 범위를 따른다.

## 검증

| 검사 | 판정 및 근거 |
|---|---|
| 정상 클라이언트 연결 해제 | 실제 소유자에 detach 후 같은 셸 PID·변수 `731` 유지, 재접속 입력·출력 확인 |
| 앱 정상 종료 백엔드 | 단위 검사에서 실제 lifecycle supervisor에 `Event::Quit`를 전달, 요청이 `status`, `detach`뿐이며 `stop`이 없음을 확인 |
| 강제 클라이언트 종료 | 별도 무해한 시험 클라이언트를 죽인 뒤 기존 소유자에 재접속, 같은 셸 PID·변수 유지 |
| GUI 강제 종료·재실행 | 자체 GUI PID 3620만 강제 종료, 소유자 32408·셸 32148 생존 확인, 재실행 GUI 11624의 inspect에서 `running:true`, `pid:32148`, 동일 instance 확인. [종료 전 inspect](windows-pty-owner-20261121/gui-before.json), [종료 후 inspect](windows-pty-owner-20261121/gui-after-crash.json) |
| GUI 화면 | computer-use 스킬의 `@oai/sky` 사용. Win32 프로세스 경로와 PID를 먼저 확인하고, 반환된 창 중 `04fb` 경로와 세션 ID가 유일하게 일치하는 창 선택. [이전 화면](windows-pty-owner-20261121/gui-before.png)·[재접속 화면](windows-pty-owner-20261121/gui-after-crash.png)에서 고정 시험 표식 확인. 해당 tree와 native 창 제목도 같은 폴더에 보관 |
| 미접속 대량 출력 | 연결을 끊은 상태에서 30,000줄, 약 1.9MiB 출력 후 완료 표식 수신. 셸은 새 PID로 바꾸지 않았고 이후 변수 조회 성공 |
| resize | IPC 크기 101×33 확인 후 실제 PowerShell `[Console]::WindowWidth/Height` 출력도 101×33 확인 |
| 중복 소유자·중복 접속 | 추가 소유자 실행이 기존 세션을 바꾸지 않으며 두 번째 attach 거부 |
| capability | 잘못된 capability의 stop 거부, 원래 셸 유지 |
| 명시 종료 | stop 후 셸 및 시험용 PowerShell 자식 모두 종료 확인. 사용자 프로세스는 대상에 넣지 않음 |
| 소유자 없는 재접속·유예 만료 | attach-only 요청이 셸을 만들지 않고 거부됨을 확인. 1초 lease의 detach 뒤 소유자·셸 모두 종료 확인 |
| 버퍼 | 64KiB 상한과 최신 꼬리 보관 단위 검사 통과 |
| 자동 재접속 UI | 살아 있는 소유자에서는 plan·execRequest 없이 `reattach`만 요청하는 회귀 검사 통과 |
| Rust | `npm test`: 단위 316 통과, 실기기 통합 1 통과, GUI fixture 1 의도적 건너뜀. 마지막 수명 경계 회귀 추가·확장 뒤 관련 3개 별도 통과 (전체 317개 중 314개 필터 제외) |
| UI | `npm test`: 253개 중 251 통과, 기존 환경 조건부 2 건너뜀 |
| 정적 검사 | `npm run check` (Clippy all-targets, warnings 금지) 통과 |
| 최종 개발 바이너리 | 마지막 코드 변경과 관련 검증 뒤 `npm run dev:build`, `npm run dev:info` 통과. [빌드 로그](windows-pty-owner-20261121/dev-build.log)·[root/ID/binary](windows-pty-owner-20261121/dev-info.txt) |

[전체 검사 로그](windows-pty-owner-20261121/tests.log), [실기기 세부 로그](windows-pty-owner-20261121/integration.log), [마지막 수명 경계 3개 검사](windows-pty-owner-20261121/lifecycle.log), [Clippy](windows-pty-owner-20261121/clippy.log)를 보관했다. GUI 검증 이후 셸 핸들이 먼저 회수되어도 소유자 제어 연결을 유지하도록 보완했고, 이 마지막 변경은 관련 3개 검사와 전체 검사 당시의 실기기 통합으로 확인했다. GUI 화면 검증 자체를 다시 실행했다고 계산하지 않는다.

기본 PATH의 Node 20.11에서는 프로젝트 지원 버전보다 낮은 MockTimers 오류가 발생했다. Codex 제공 Node 24.22.3을 PATH 앞에 두고 재실행한 결과만 통과로 기록했다. 의존성 설치는 프로젝트의 pnpm lockfile을 그대로 사용한 `pnpm install --frozen-lockfile`이며 lockfile을 갱신하지 않았다. Windows Python은 AF_UNIX가 없어 IPC 검증은 Rust의 `uds_windows`로 수행했다.

처음 GUI fixture는 테스트 실행기 종료와 함께 정리되었고, breakaway 플래그 단독 실행도 현재 실행기에서 접근 거부됐다. 이를 GUI 생존 성공으로 계산하지 않았다. 최종 GUI 검증은 fixture 실행기를 유지한 채 소유자를 준비하고, 별도 GUI 프로세스만 종료·재실행했다. 따라서 GUI의 소유자 **최초 생성**부터 실행기 종료까지 이어지는 종단 검증과 상위 실행기 Job 종료는 미검증이다. GUI가 기존 소유자에 자동 재접속하는 실제 경로와 소유자 내부 ConPTY 동작은 위에서 별도로 확인했다.

GUI 트레이 정상 종료 메뉴는 지원 UI 도구가 알림 영역 창을 반환하지 않아 직접 실행하지 않았다. 정상 종료를 강제 종료로 대체해 통과 처리하지 않았다. 실제 업데이트 설치도 수행하지 않았다. 특히 현재 소유자가 GUI와 같은 exe를 사용하므로 살아 있는 소유자가 Windows 파일 잠금을 유지한다. 버전별 소유자 실행 파일 분리·교체 및 업데이트 보호 연동을 해결하기 전까지 정식 업데이트 생존 기능으로 사용할 수 없다.

## 실행

지원되는 Node 22.13 이상을 사용한다. 현재 워크트리에서 다음 명령을 실행한다.

```powershell
Set-Location 'C:\Users\egoin\.codex\worktrees\04fb\agent-gibbon'
$env:JAJAN_POC_WINDOWS_PTY='1'
scripts/jajan-dev.cmd open
```

개발 빌드는 최종 검증 뒤 `npm run dev:build`로 준비하며 `open` 자체는 재빌드하지 않는다. 변경 감시 실행은 같은 환경에서 `npm run dev`다. 실험 변수는 앱 프로세스가 시작할 때 적용되므로 이미 켜진 앱에는 소급 적용되지 않는다. 새 터미널의 명시적 셸 시작은 기존 UI를 사용한다.

재현 가능한 실기기 검사는 `node scripts/build.mjs cargo test --manifest-path src-tauri/Cargo.toml --test windows_pty_owner -- --nocapture`다. GUI fixture는 같은 명령에서 `seed_isolated_gui_session -- --ignored --nocapture`로 명시 실행하며, 현재 워크트리에서 만든 고유 ID와 stop 표식 경로를 출력한다. 이 프로세스를 유지하며 해당 ID를 개발 CLI로 열고, 끝날 때 출력된 stop 표식 파일을 만들면 fixture가 소유자와 시험 셸을 정리한다. 900초 제한도 있다. GUI fixture가 동작하는 동안 같은 exe를 재빌드하지 않는다.

Conf: 5 (실제 Windows ConPTY 및 GUI 강제 종료 재접속의 한정된 범위). Risk: Medium (opt-in 로컬 세션 수명·IPC·프로세스 정리). 미검증 범위는 위 표와 본문대로 남긴다.

검증에서 만든 GUI 11624와 소유자 32408·셸 32148을 정리했고 시험 터미널 메타데이터 두 개만 제거했다. [정리 당시 프로세스 목록](windows-pty-owner-20261121/cleanup-processes.json)은 설치 앱 28264만 남았음을 보여 준다. 진행 메모 `windows-pty-progress-04fb`는 보존하며 최종 개발 빌드로 다시 표시한다. 이 앱을 띄운 채 재빌드하면 파일 잠금이 생길 수 있으므로 이후 재빌드는 해당 개발 인스턴스의 정상 종료 뒤 진행한다.

최종 빌드로 다시 시작한 개발 GUI는 PID `3840`이다. Win32 실행 경로가 위 binary와 일치하고 진행 메모 inspect가 `ok:true`, 예상 instance, 최종 체크리스트 본문을 반환함을 확인했다. 설치 앱 28264는 그대로 유지됐다. 현재 시험용 관리 프로세스나 셸은 남겨 두지 않았다.
