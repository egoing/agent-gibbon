# Windows 분리형 ConPTY: 정식 프로필·일시 활성화 검증

작업 root는 `C:\Users\egoin\.codex\worktrees\04fb\agent-gibbon`, 브랜치는 `codex/windows-pty-manager`, base는 `974197adbc34f2f72872eb4e6262e270db90ef2d`다. `AGENTS.md`와 복사된 `AGENTS.override.md`의 로컬 개발 완료 정책을 적용했다. 기본 버전 0.3.12는 변경하지 않았다.

## 지원 계약

관리 창에서만 `terminal_persistence_status`와 `terminal_persistence_enable`을 호출한다. Windows는 debug/release 모두 지원하며 기본 OFF다. enable은 검증된 owner 실행 파일을 준비한 후 **현재 GUI 실행의 새 세션 생성만** 활성화한다. 기존 일반 셸을 이동하거나 종료하지 않는다. 일반 재시작과 업데이트 재시작은 신규 생성 OFF로 돌아온다. 과거 영구 ON 파일은 적용하지 않고 같은 설치/instance의 유효한 파일만 OFF로 이행한다. 손상된 파일은 활성화 근거로 쓰지 않는다.

| 반환 필드 | 뜻 |
|---|---|
| `supported` | 현재 플랫폼의 메뉴 API 지원 여부 |
| `enabled` | 이번 GUI 실행에서 이후 새 세션을 분리 owner로 생성하는지 |
| `backend` | Windows는 `windows-conpty-owner` |
| `applies_to` | `new_sessions` |
| `persisted` | 신규 생성 선택의 영속화 여부: 항상 `false` |
| `reconnect_persisted` / `reconnect_sessions` | 별도 checkpoint로 인증된 살아있는 기존 owner 세션의 존재/수 |
| `reason` | 신규 생성의 일시성과 기존 셸 재접속의 독립성을 설명 |

다른 플랫폼의 메뉴 연동은 미지원 사유를 반환한다. 기존 Mac debug Python PoC를 정식 release 지원으로 확대하거나 켜졌다고 표시하지 않는다. 개발자 메뉴 UI·이전/최신 버전 왕복 버튼은 별도 PR 소유다.

## 소유권과 보호

owner가 ConPTY·셸·입출력·KILL_ON_JOB_CLOSE Job Object를 소유한다. GUI는 AF_UNIX IPC 클라이언트다. owner 자체를 Job에 넣은 후 셸을 생성해 자식도 같은 소유 작업에 들어간다. owner별 lock과 클라이언트 HANDLE로 중복 owner/연결 및 강제 종료를 판정한다. owner는 GUI 설치 폴더 밖의 내용 해시별 복사 실행 파일을 사용해 NSIS 교체를 방해하지 않는다.

IPC 폴더는 현재 사용자만 허용하는 보호 DACL을 검사하며 reparse 경로와 긴 AF_UNIX 경로를 거부한다. 개발 instance와 설치 경로 해시 namespace를 구분한다. 재접속 checkpoint는 프로토콜2·무작위 capability·owner PID·셸 PID만 저장한다. 해당 capability와 두 PID가 일치해야 기존 셸에 연결하며, attach-only는 새 owner/셸을 생성하지 않는다. capability 자체는 보고서·로그에 수집하지 않았다.

앱 종료는 detach, 터미널의 명시 종료는 해당 owner와 소유 작업 종료다. 미접속 유예는 기본 300초(개발 변수로 1~900초), 출력은 메모리 최신 64KiB다. 연결이 없어도 출력 drain을 계속하고 입력은 bounded queue를 쓴다. 전체 터미널 화면/스크롤 복원, 재부팅·로그아웃·owner 종료·상위 Job 전체 종료 생존은 제공하지 않는다.

업데이트 예외는 살아있는 호환 owner·분리된 실행 파일·유효 checkpoint·저장 성공·정확히 세션당 reader/supervisor 두 worker·추가 닫기/정리 없음이 확인된 세션만 허용한다. 신규 생성 OFF와 이 판정은 독립적이다. Welcome·일반 PTY·웹 창·저장 실패/진행 중·불완전 checkpoint는 계속 보류한다. 과거 출력의 command hook이 재접속 후 restore_plan을 덮지 않도록 새 명령 제출 전까지 해당 plan 보고를 무시한다.

## 격리 환경과 실제 수행

정식 `release` 최적화 프로필에 `verification-updater`를 사용했다. 전용 bundle `app.jajan.ptyrelease04fb`, exe `jajan-ptyrelease-04fb.exe`, instance `dev-041704170417041704170417`, 설치 폴더 `.tmp/pty-release/installed`다. 자체 일회용 서명 키와 `127.0.0.1:18742` 피드만 사용했으며 공개 배포 키·GitHub 피드는 사용하지 않았다.

`.tmp/pty-release/source`는 이번 소스의 시험용 사본이다. 사본에만 관리 창의 실제 production status/enable/updater API를 호출하는 시험 버튼과 고정된 무해한 셸 생성/상태 확인 hook을 추가했다. 실제 5회 클릭 개발자 메뉴의 통합 검증을 대신하지 않는다. 전용 가상 동의 기록은 `terms:true, sharing:false`로 준비했으며 실제 사용자 동의·설치본 설정을 바꾸지 않았다. 시험 사본의 셸은 `-NoProfile`을 사용했다.

| 시나리오 | 실제 결과와 근거 |
|---|---|
| 기본 OFF | 초기 status의 `enabled:false,persisted:false,reconnect_sessions:0`. [화면](windows-pty-owner-release-20261121/01-default-off.jpg) |
| 실제 API 활성화 | 관리 창에서 enable을 클릭해 `enabled:true,persisted:false`. 이미 생성된 일반 셸 PID32128을 유지했다. [화면](windows-pty-owner-release-20261121/02-enabled.jpg) |
| GUI가 owner를 생성 | GUI33052의 실제 자식 owner32288, 그 owner의 셸32700. 외부에서 seed한 셸을 GUI가 만든 것으로 계산하지 않았다. |
| 정상 종료 | GUI33052·일반 셸32128 종료, owner32288·분리 셸32700 생존. [판정](windows-pty-owner-release-20261121/normal-exit.json) |
| 일반 재시작 | 신규 생성 OFF와 reconnect 1을 별도 표시. [상태](windows-pty-owner-release-20261121/03-restart-creation-off.jpg), [동일 PID inspect](windows-pty-owner-release-20261121/after-restart.json) |
| 재시작 후 새 일반 셸 | 신규 enable 없이 생성한 셸8320은 새 GUI25568의 직접 자식이었다. 이후 정상 GUI 종료 때 종료됐다. |
| 일반 셸 업데이트 보류 | 다운로드·검증 완료 후 `ready`이지만 일반 셸 때문에 설치 보류. [보호 판정](windows-pty-owner-release-20261121/native-update-blocked.json) |
| 실제 NSIS 0.3.18→0.3.19 | 다운로드·서명·창 ACK·설치·재시작 후 current_version 0.3.19, 셸32700 유지. [업데이터](windows-pty-owner-release-20261121/after-update-updater.json), [셸](windows-pty-owner-release-20261121/after-update.json) |
| 상태 보존 | 기존 셸에 고정 변수731을 한 번 설정한 후 업데이트 뒤 초기화 없이 읽어 같은 PID·731 확인. [전](windows-pty-owner-release-20261121/state-before-update.json), [후](windows-pty-owner-release-20261121/state-after-update.json) |
| 강제 GUI 종료 | 최종 메타데이터 수정이 포함된 0.3.20 GUI6324만 출처 검사 후 강제 종료. 재실행 후 셸32700과 restore_plan `codex` 유지. [결과](windows-pty-owner-release-20261121/after-forced-exit.json) |
| 최종 NSIS 0.3.23→0.3.24 | 실제 설치 버튼→창 ACK→NSIS→GUI1004 재시작. owner32288/셸32700과 restore_plan `codex` 유지. [전](windows-pty-owner-release-20261121/final-before-update.json)·[후](windows-pty-owner-release-20261121/final-after-update.json)·[준비 ACK](windows-pty-owner-release-20261121/final-update-handshake.json)·[프로세스 계보](windows-pty-owner-release-20261121/final-processes.json)·[신규 OFF 화면](windows-pty-owner-release-20261121/07-final-update-off.jpg) |
| 최종 상태 보존 | 업데이트 뒤 변수를 다시 설정하지 않고 기존 셸32700의 731 조회 통과. [판정](windows-pty-owner-release-20261121/final-state-after-update.json) |
| 명시적 종료 | computer-use로 터미널 종료 버튼을 눌러 owner32288과 셸32700 소멸 확인. [결과](windows-pty-owner-release-20261121/explicit-stop.json) |
| 이전 영구 ON 이행 | 이 시험 전용 notes/instance에 version1 enabled=true 파일을 놓고 재시작했다. 파일은 enabled=false로 이행하고 status 신규 생성 OFF 유지. 사용자 설치본에는 적용하지 않았다. |
| 설치 경로 namespace | 최종 debug 바이너리를 두 시험 설치 경로에 복사하고 instance 환경변수 없이 실제 설치 namespace 코드를 실행했다. 같은 세션 ID가 다른 owner·셸을 만들며 교차 capability를 거부하고 명시 stop 시 각각 정리했다. [결과](windows-pty-owner-release-20261121/installed-namespace.json). 이 항목은 release NSIS의 무인스턴스 설치 시험과 구분한다. |

첫 시험 사본의 설치 자산 폴더에 이전 버전이 함께 남아 이전 패키지를 선택한 회차는 최종 성공에서 제외했다. 버전을 정확히 지정한 0.3.18부터 계산했다. 초기 시험 hook의 중복 cursor 응답이 고정 명령을 흐트러뜨린 것도 상태 검증 성공으로 쓰지 않았다. 별도 인증된 시험 클라이언트로 고정 변수 설정/조회 결과만 판정하고 사용자 원문은 수집하지 않고 고정된 무해한 시험 출력만 확인했다.

최종 시험 중 시작 직후 worker3(시작 예약+reader+supervisor)에서는 업데이트가 안전하게 보류됐다. 상세 시험 진단으로 시작 완료 후 세션1/worker2·metadata_ready/remote 검증 정상 상태를 확인하고 .23→.24 설치를 통과했다. 상세 진단 필드는 시험 사본에만 추가했으며 제품 status API에는 포함하지 않았다. .20→.21의 보류 회차는 성공으로 계산하지 않았다.

시험 앱은 전용 uninstaller로 제거하고 일회용 키와 loopback feed도 정리했다. [정리 결과](windows-pty-owner-release-20261121/cleanup.json). `.tmp`의 시험 소스·서명된 자산과 전용 synthetic 앱 데이터는 재현 근거로 남기고 사용자 설치 앱은 건드리지 않았다.

## 자동 검증과 실행

최종 코드에서 Rust 단위327개, 실제 Windows ConPTY 통합1개, UI251개가 통과했다(UI2개 skip, 수동 fixture1개 ignored). 통합 시험은 1.9MiB 미접속 출력·동일 PID/변수·재입력·resize·중복 owner/client 거부·잘못된 token 거부·강제 client 종료·소유 descendant 정리·lease 만료를 확인한다. Clippy `-D warnings`, `git diff --check`, `npm run dev:build`, `npm run dev:info`를 실행했다.

```powershell
Set-Location 'C:\Users\egoin\.codex\worktrees\04fb\agent-gibbon'
npm test
npm run check
npm run dev:build
npm run dev:info
scripts/jajan-dev.cmd open
```

`open`은 자동 재빌드하지 않는다. 기본 개발 instance는 `dev-9f4318a9190cf96216037515`, 실제 개발 binary는 `C:\Users\egoin\.codex\worktrees\04fb\agent-gibbon\src-tauri\target\debug\jajan.exe`다. 이번 최종 개발 바이너리를 사용자 설치본이나 다른 작업 앱으로 대체하지 않았다. 변경 감시는 `npm run dev`다.

Conf **5**: 이 Windows 기기의 실제 ConPTY와 격리된 정식 프로필 NSIS 교체 범위. Risk **Medium**: opt-in 프로세스 수명·IPC·메타데이터·업데이트 보호 예외에 영향. 공개 설치본 교체, 다른 Windows 버전, 실제 5회 클릭 메뉴 연동과 공개 릴리스는 이 worker의 완료 범위와 구분한다. main 병합·버전업·배포는 배정된 통합 담당이 소유한다.
