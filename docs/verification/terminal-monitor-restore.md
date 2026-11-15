# macOS 터미널 자동 복원 실기기 검증

워크트리 `/Users/egoinglee/.codex/worktrees/bb80/agent-gibbon`에서 `pnpm dev:info`와 개발 CLI inspect로 인스턴스 `dev-9025141c4fef22daa9f2e25a`를 확인했다. 개발 앱을 정상 종료하고 같은 debug 바이너리를 다시 실행했다. 마지막 재시작의 PID는 32426이었다. 네이티브 창은 이 PID와 창 제목의 터미널 ID로 식별했다.

메인 화면 LG HDR 4K 위쪽에 있는 서브 화면 V28UE-M의 논리 좌표 범위는 x 0–1920, y -1080–0이었다. 테스트 창은 480×360, x 60 또는 560, y -1000 또는 -600에 배치했다. 각 테스트의 최초 실행 계획만 별도 테스트 메타데이터로 준비했다. 실제 창에서 복원을 눌러 프로그램을 시작했고, 자동 복원 여부는 앱이 실행 중에 기록했다.

| 터미널 ID | 실제 실행 | 앱 재시작 후 |
|---|---|---|
| terminal-88114001 | htop | 전경 htop, 새 셸 PID 32466 |
| terminal-88114005 | btop | 전경 btop, 새 셸 PID 32464 |
| terminal-88114003 | htop -d 20 | 편집란에 htop, PID 없음 |
| terminal-88114004 | nano | 편집란에 nano, PID 없음. 취소 후 작업 위치와 실행 버튼 확인 |
| terminal-88114002 | top | 편집란에 top, PID 없음 |

`pnpm test`에서 Rust 224개와 UI 183개가 통과했다. Windows 실기기 검증은 수행하지 않았다.

## 이름 기반 허용 목록과 미지원 OS 차단 검증

2026-11-14 동일 워크트리·인스턴스에서 최신 개발 바이너리를 빌드해 재검증했다. 허용 목록은 `src-tauri/src/terminal/auto-restore-macos.json`으로 옮겼으며 프로그램 이름과 설치 출처를 기록한다. macOS 기본 명령의 상세 조사와 보류 사유는 [후보 조사](../research/macos-auto-restore-commands.md)를 따른다.

`pnpm dev:info`의 root·id·binary를 확인했고 실제 프로세스 실행 파일을 `lsof -a -p PID -d txt -Fn`으로 대조했다. 최초 PID는 72812, 재시작 PID는 76117이다. 개발 CLI inspect 응답의 `ok: true`와 인스턴스 ID를 모든 창에서 대조했다. GUI 조작은 PID와 네이티브 창 제목 `Agent Gibbon [DEV codex/terminal-relaunch-research · 9025141c] — 터미널ID`를 함께 지정한 접근성 API로 수행했다.

CoreGraphics로 메인 LG HDR 4K의 논리 범위 (0, 0, 2048, 1152)와 서브 V28UE-M의 범위 (0, -1080, 1920, 1080)을 확인했다. 이번 여섯 창은 모두 서브 모니터에서 생성·실행·복원했다. 창 크기는 480×360, x는 60·580·1100, y는 -1020·-590이었다.

| 터미널 ID | 종료 전 실행 | 재시작 결과 |
|---|---|---|
| terminal-88114101 | htop | running·auto_restore·restored true, 전경 htop |
| terminal-88114102 | btop | running·auto_restore·restored true, 전경 btop |
| terminal-88114103 | nettop | running·auto_restore·restored true, 전경 nettop |
| terminal-88114104 | htop -d 20 | 셸 PID 없음, 실행 계획 htop과 복원·취소 버튼 |
| terminal-88114105 | top | 셸 PID 없음, 실행 계획 top과 복원·취소 버튼 |
| terminal-88114106 | date | 셸 PID 없음, 실행 계획 date와 복원·취소 버튼 |

date 실행 계획에 ` ; touch /tmp/bb80-current-edited-proof`를 실제 키 입력으로 추가했다. 접근성 값에서 편집 내용을 확인하고 복원을 눌렀으며 해당 파일 생성을 확인했다. 인자가 있던 htop 창에서 취소를 누른 뒤 작업 위치·실행 버튼과 셸 미실행을 확인했다. 결과 요약은 `/tmp/bb80-current-results.json`, 실행 로그는 `/tmp/bb80-current-live.log`에 남겼다. 테스트용 터미널 기록과 생성 파일은 검증 후 제거했다.

`pnpm test`에서 Rust 225개와 UI 184개가 통과했다. Windows·Linux에 대해 백엔드 복원 실행 정책을 검사했고 UI에서는 미지원 안내·자동 실행 차단·편집 후 복원 차단·취소 후 Welcome을 검증했다. `pnpm check`와 `pnpm dev:build`도 통과했다. Windows·Linux의 네이티브 빌드와 실기기 실행은 수행하지 않았다. Windows 후속 작업은 [#185](https://github.com/egoing/agent-gibbon/issues/185)에 기록했다.

Conf 5는 위 macOS 실제 재시작 흐름에 해당한다. Windows·Linux 차단의 확신도는 정책·UI 테스트 기준 Conf 3이다. Risk Medium으로, 창별 저장 상태와 프로세스 시작 동작에 영향을 준다.

## 2026-11-15 close 최종 통합

origin/main `ea0531a`의 웹 메모·시작 패닉 방지 변경을 통합했다. 독립 코드 감사에서 셸 종료 뒤 자동 복원 플래그가 남는 경우와 bash의 PROMPT_COMMAND가 마지막 사용자 명령을 덮어쓰는 경우를 발견해 보강했다. 셸이 종료되면 자동 복원 상태를 해제하며, 아직 세션이 없는 복원 대기 창의 기록은 유지한다.

bash에는 프롬프트 전후 상태를 구분하는 훅을 추가했다. macOS 기본 bash 3.2의 문자열 PROMPT_COMMAND와 최신 bash의 배열 형식을 구분한다. 초기 보강 과정에서 긴 초기화 입력이 PTY 버퍼를 막는 실패가 있었으며, 초기화 정의를 자식 셸 환경으로 전달한 뒤 짧은 eval과 환경 변수 제거를 수행하도록 고쳤다. 실제 bash에서 date·빈 입력·pwd를 실행하는 회귀 테스트가 통과했다.

최종 `pnpm test`는 Rust 227개·UI 184개 통과, `pnpm check`·`pnpm dev:build`·Rustfmt·origin/main 대비 diff 검사도 통과했다. 로그는 `/tmp/bb80-close-tests.log`, `/tmp/bb80-close-clippy.log`, `/tmp/bb80-close-build.log`에 있다.

동일 작업 루트·인스턴스의 실제 바이너리와 PID를 대조하고 V28UE-M의 여섯 테스트 창을 재검증했다. 최초 PID 17481, 첫 재시작 PID 20603에서 htop·btop·nettop 자동 실행과 인자 있는 htop·top·date의 추천 대기, 편집 입력의 임시 파일 생성, 취소 후 Welcome을 확인했다. 이어 테스트 htop 셸만 종료하고 inspect의 running·auto_restore가 모두 false임을 확인했으며, 다시 시작한 PID 22145에서도 해당 창이 자동 실행되지 않았다. 검증용 자식 프로세스만 종료했고 기존 설치 앱·다른 워크트리는 조작하지 않았다. 실제 실행 로그는 `/tmp/bb80-close-live.log`에 있다.

Windows·Linux 네이티브 실행, Intel Mac, PowerShell 7, 에이전트 대화 재개는 이번 최종 검증 범위에 포함하지 않았다. OS별 복원 실행 차단의 정책·UI 테스트는 통과했다.
