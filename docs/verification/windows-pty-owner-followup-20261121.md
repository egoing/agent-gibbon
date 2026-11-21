> 역사적 debug 실험 기록입니다. 현재 활성화 수명과 정식 빌드 지원은 [release 검증](windows-pty-owner-release-20261121.md)을 따릅니다.

# Windows PTY 정상 종료·실제 업데이트 후속 검증

2026-11-21 KST. **정상 GUI 종료 후 같은 셸 재접속과, 격리된 실제 NSIS 0.3.14 → 0.3.15 업데이트 후 같은 셸 유지를 확인했다.** 재시작 때 실험용 환경변수가 사라지는 실패도 실제로 재현해 고쳤다. 정식 배포판이나 사용자 설치 앱은 업데이트하지 않았다. 커밋·push·병합·릴리스도 실행하지 않았다.

## 출처와 격리

| 항목 | 값 |
|---|---|
| 작업 root | `C:\Users\egoin\.codex\worktrees\04fb\agent-gibbon` |
| branch / base SHA | `codex/windows-pty-manager` / `974197adbc34f2f72872eb4e6262e270db90ef2d` |
| 기본 개발 instance | `dev-9f4318a9190cf96216037515` |
| 기본 개발 binary | `C:\Users\egoin\.codex\worktrees\04fb\agent-gibbon\src-tauri\target\debug\jajan.exe` |
| 정상 종료 시험 | GUI `13248` → `25100`, owner `7040`, shell `16296`, ID `terminal-1795255148442582200` |
| 업데이트 전용 bundle / exe | `app.jajan.ptyverify04fb` / `jajan-ptyverify-04fb.exe` |
| 업데이트 전용 instance | `dev-001020010200102001020010` (`verification-updater` feature) |
| 검증 설치 폴더 | `C:\Users\egoin\.codex\worktrees\04fb\agent-gibbon\.tmp\pty-update\installed` |
| 최종 업데이트 시험 | GUI `11024` (0.3.14) → `25572` (0.3.15), owner `10144`, shell `32428` |
| 업데이트 시험 ID | `terminal-1795255345836294500` |
| 보존한 사용자 설치 앱 | PID `28264`, `C:\Users\egoin\AppData\Local\Jajan\jajan.exe` |

작업 중 기본 코드의 버전은 0.3.12로 유지했다. `.tmp/pty-update/source`에 소스 사본을 만들어 그 사본에서만 0.3.12~0.3.15 검증 버전을 만들었다. 원격 GitHub 피드·배포 키를 쓰지 않고 `127.0.0.1:18741`의 자체 시험 피드와 일회용 키로 서명한 설치 파일을 사용했다. 소스 사본의 bundle identifier·product name·main binary name은 모두 설치본과 다르게 지정했다. NSIS가 이 이름들을 사용하는 방식은 [Tauri 공식 설치기 템플릿](https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-bundler/src/bundle/windows/nsis/installer.nsi)과 일치한다.

## 확인한 결과

| 시나리오 | 실제 관측 |
|---|---|
| 정상 종료 | debug opt-in CLI `poc-quit`가 트레이의 종료 동작과 같은 `AppHandle::exit(0)`을 호출했다. GUI 13248의 소멸과 owner 7040·shell 16296의 생존 확인. 강제 종료로 대체하지 않았다. [종료 판정](windows-pty-owner-followup-20261121/normal-exit.json) |
| 정상 종료 후 재실행 | GUI 25100의 inspect가 `running:true`, `pid:16296`, 예상 instance를 반환. 화면의 `GUI_PROBE_<PID>_731` 출력이 계속됨을 확인. [이전](windows-pty-owner-followup-20261121/normal-before.json)·[이후](windows-pty-owner-followup-20261121/normal-after.json) |
| 최초 실제 업데이트의 실패 | 0.3.12 → 0.3.13 다운로드·서명 검증·설치·재시작은 됐으나 새 GUI 22524가 실험 모드를 잃었다. `poc-install-update`는 ‘PTY 개발 실험 모드가 아닙니다’로 거부됐고 자동 재접속도 이 회차의 성공으로 계산하지 않았다. owner 10144·shell 32428은 살아 있었다. [당시 진단](windows-pty-owner-followup-20261121/update-after-attempt-diagnostics.json) |
| 수정 후 실제 업데이트 | 0.3.14의 GUI 11024가 0.3.15를 다운로드하고 모든 창 ACK 후 NSIS를 실행. 19:10 KST에 installing 관측. 새 GUI 25572의 실제 실행 경로·`--poc-serve` 인자와 `current_version:0.3.15` 확인. [전환 기록](windows-pty-owner-followup-20261121/update-events.jsonl)·[업데이트 후 진단](windows-pty-owner-followup-20261121/update-fixed-after-diagnostics.json) |
| 같은 셸 유지 | 업데이트 전후 owner `10144`, shell `32428` 유지. 새 GUI의 inspect가 같은 instance·ID·PID와 `running:true`를 반환하고 고정 변수 `731` 출력이 이어짐. 명령을 다시 실행하거나 새 셸을 만든 결과가 아니다. [업데이트 전](windows-pty-owner-followup-20261121/update-fixed-before-terminal.json)·[후](windows-pty-owner-followup-20261121/update-fixed-after-terminal.json) |
| 기존 보호와 메모 | 마지막 설치 준비 기록이 `source_version:0.3.14`, `target_version:0.3.15`, `outcome:prepared`, 모든 창 ACK를 기록. 별도 시험 메모 `PTY-UPDATE-SENTINEL-04FB`도 유지. [메모 확인](windows-pty-owner-followup-20261121/update-fixed-sentinel.json) |

컴퓨터 화면은 computer-use 스킬의 `@oai/sky`로 확인했다. Win32 process path·PID를 먼저 읽은 뒤 반환된 native 창 중 해당 exe 경로와 고유 세션 ID가 유일하게 일치하는 창을 선택했다. [정상 종료 전](windows-pty-owner-followup-20261121/normal-before.png)·[재실행 후](windows-pty-owner-followup-20261121/normal-after.png), [업데이트 전](windows-pty-owner-followup-20261121/update-before.png)·[업데이트 후](windows-pty-owner-followup-20261121/update-after.png)의 창 단위 캡처와 accessibility tree를 같은 폴더에 보관했다. 시험 셸에는 고정 표식만 출력했다. 사용자 터미널 명령이나 기록을 읽지 않았다.

정상 종료 **메뉴 클릭 자체**는 UI 도구가 알림 영역 창을 제공하지 않아 실행하지 않았다. 대신 실제 GUI 프로세스에 동일한 정상 종료 API를 호출하고 재실행한 종단 검증이다. 업데이트는 mock 재시작이 아니라 Tauri updater의 다운로드·서명 검증·보호 확인·실제 NSIS 설치·새 exe 실행까지 수행했다. 설치 중 exe 교체 순간의 CLI 조회 한 번이 Windows 실행 오류를 반환했으며, 이 조회를 통과로 계산하지 않고 설치 후 별도로 경로·버전·PID를 재확인했다.

## 보완 내용

- **실행 파일 수명 분리:** 소유자는 현재 사용자 전용 IPC 폴더의 `owner-<내용 해시>.exe` 사본을 사용한다. 임시 파일을 원자적으로 게시하고 기존 파일도 해시와 reparse 속성을 검사한다. GUI/설치 폴더의 exe가 교체되어도 기존 소유자가 원래 프로토콜로 계속 실행된다. 캐시 실행 파일은 자동 삭제하지 않는다.
- **명시적 실험 선택 유지:** Windows 설치기의 새 프로세스가 호출 에이전트의 환경변수를 상속한다고 가정하지 않는다. 최초 CLI가 명시적 opt-in으로 GUI를 시작할 때 내부 `--poc-serve` 인자를 사용하고, 이 인자가 설치 후 실행에도 전달되어 실험 선택을 유지한다. debug 개발 인스턴스에만 적용한다.
- **호환 세션만 예외 허용:** 기존 업데이트 보호는 정식 배포·일반 PTY·시작 화면·종료 세션에서 그대로 작동한다. opt-in에서 살아 있는 protocol 1 소유자, GUI 밖의 소유자 실행 파일, 현재 세션의 reader·supervisor만 남은 상태가 확인될 때만 예외를 허용한다. 닫기·저장·추가 정리 작업이 있으면 보류한다. 편집/저장/내보내기/캡처/웹 창 보호와 모든 WebView ACK도 유지한다.
- **창 복원:** 검증된 debug 경로의 업데이트 snapshot에 있는 터미널 창을 metadata에서 숨긴 상태로 먼저 준비한 뒤 저장된 위치·표시 상태를 적용한다. 살아 있는 소유자가 있을 때만 UI가 `reattach`를 호출한다.
- **시험 제어:** `poc-quit`·`poc-restart`는 opt-in debug 인스턴스에서만 사용할 수 있다. `poc-check-update`·`poc-install-update`는 추가로 격리된 `verification-updater` 빌드여야 하며 기존 보호를 그대로 통과해야 한다. 응답의 `scheduled:true`는 작업 예약만 뜻하므로 설치 완료는 진단과 새 프로세스로 별도 확인했다.

실기기 업데이트를 확인한 뒤, 제거된 세션의 정리 worker나 진행 중인 닫기·저장이 남은 경우도 보류하도록 마지막 보호 조건을 추가했다. 이 마지막 보수적 조건은 전용 회귀 검사와 전체 로컬 검사로 확인했으며, 동일 NSIS 회차를 다시 실행한 결과로 표기하지 않는다.

## 로컬 검사와 한계

후속 전체 검사·Clippy·최종 개발 빌드 로그를 이 문서와 같은 이름의 폴더에 보관한다. Rust 회귀는 동일 셸·변수·입출력·resize, 미접속 약 1.9MiB 출력, 중복 소유자/접속 거부, 잘못된 capability 거부, 명시 종료와 자식 정리, lease 만료, 소유자 없는 attach-only의 새 셸 생성 금지, 소유자 사본의 원본 해시 일치, 호환/생존/파일 분리 조건을 검사한다. 마지막 검사는 다른 종료 worker·닫기·저장이 있는 경우와 현재 두 세션 작업만 남은 경우를 구분한다.

시험용 소유자의 최초 생성은 backend fixture에서 수행했다. GUI에서 새 터미널을 선택해 최초 소유자를 생성하는 클릭부터 전 과정을 이어 붙인 E2E는 아니다. 실제 앱 종료/설치/재접속과 실제 GUI 출력은 위에서 확인했다. 정식 release 빌드·배포 서명 키·다른 Windows 버전·상위 Job 전체 강제 종료·재부팅·로그아웃은 검증하지 않았다. 출력 보관 64KiB와 기본 미접속 300초(1~900초)의 실험 제한은 유지한다. GUI의 완전한 화면/스크롤 상태를 저장하는 기능은 아니다.

## 실행과 정리

```powershell
Set-Location 'C:\Users\egoin\.codex\worktrees\04fb\agent-gibbon'
$env:JAJAN_POC_WINDOWS_PTY='1'
scripts/jajan-dev.cmd open
```

`open`은 재빌드하지 않으며 최종 변경 뒤 `npm run dev:build`로 준비한 이 워크트리의 debug 바이너리를 사용한다. 변경 감시는 `npm run dev`다. 실험 앱의 정상 종료를 CLI로 시험하려면 같은 워크트리에서 `scripts/jajan-dev.cmd poc-quit`를 쓴다. 이후 환경변수를 제거하고 새로 시작하면 실험을 사용하지 않는다. 이미 분리된 셸은 명시 종료하거나 lease 만료까지 유지될 수 있다.

GUI fixture를 재현할 때는 현재 유틸의 고정 정지 표식 `.tmp/windows-pty-gui-stop`을 먼저 제거하고 `node scripts/build.mjs cargo test --manifest-path src-tauri/Cargo.toml --test windows_pty_owner seed_isolated_gui_session -- --ignored --nocapture`를 실행한다. 출력된 고유 terminal ID를 개발 CLI로 열어 확인한 뒤 해당 정지 표식 파일을 만들면 fixture가 소유 작업만 정리한다. 일반 실기기 회귀는 같은 명령에서 fixture 이름과 `--ignored`를 빼고 실행한다.

시험 앱은 정상 종료한 뒤 전용 uninstaller로 제거했다. 시험 셸·소유자와 loopback feed도 종료했고 일회용 개인 키와 임시 공개 키 파일은 폐기했다. 공개 키가 포함된 시험 설정과 서명된 로컬 설치 자산·기록은 `.tmp/pty-update`에 남겼다. [제거 결과](windows-pty-owner-followup-20261121/cleanup-installer.json)는 전용 exe 제거와 설치 앱 PID 28264의 생존을 확인한다.

Conf: **5** (이 Windows 기기의 정상 종료 및 격리된 실제 NSIS 업데이트·동일 셸 재접속 범위). Risk: **Medium** (opt-in 프로세스 수명·로컬 IPC·업데이트 보호 예외). 정식 기본 동작을 바꾸거나 배포한 결과는 아니다.
