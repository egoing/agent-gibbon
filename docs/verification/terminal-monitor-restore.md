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
