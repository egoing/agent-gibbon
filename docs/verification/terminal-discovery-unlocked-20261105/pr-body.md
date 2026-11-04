## 변경 결과

터미널을 열 때마다 대화형 로그인 셸을 실행해 생기던 탐색 지연과, 그동안 기존 터미널의 입력·resize·조회가 같은 세션 목록 잠금에 대기하던 문제를 줄였습니다. 현재 PATH의 Codex를 먼저 찾고 필요할 때만 `-lc`→`-ilc`를 실행하며 Codex 우선·Claude fallback을 유지합니다. 성공 경로/PATH를 앱 수명 동안 공유하고 창별 Starting 예약으로 중복 시작을 거부하며, 탐색·PTY 시작 중 전체 세션 목록 잠금을 유지하지 않습니다.

실행 파일 삭제·교체·실행 권한 변경·확인된 비정상 종료 시 캐시를 무효화하고, spawn 실패는 같은 전체 예산으로 한 번만 재탐색합니다. 셸 단계 3초·전체 4초 제한과 timeout/error의 전용 프로세스 그룹 정리를 유지하며 인증·신뢰 실패일 수 있는 실행된 CLI를 자동 재시작하지 않습니다. UI 기준·시안·인증/권한 경로는 변경하지 않았습니다.

## 검증

- Rust 94개·UI 53개, Clippy·Rustfmt·개발 빌드·버전 0.2.7 검사 통과.
- 잠금 해제된 macOS ARM64의 **계측 없는 최종 앱**에서 READY/ECHO 실제 표시, 네이티브 키 `1·7·Return`, 89×28→107×34 resize, 동일 PTY PID·위치·크기·스크롤백의 숨김 복원 통과.
- 최종 앱의 제한 PATH 첫 창 1.587초 / 캐시 창 0.291초, 두 창 합계 로그인 셸 1회. 현재 PATH의 Codex는 두 창 모두 셸 호출 0회. 창까지의 시간은 탐색 외 WebView·PTY·fake CLI 시작·polling을 포함합니다.
- 외부 탐색 셸이 2.202초 실행되는 동안 기존 PTY가 +0.243초에 네이티브 키, +0.447초에 ECHO, +0.872초에 resize를 수신했습니다. 기존 inspect 20회 최대 89.787ms이며 18회는 시작·종료 모두 탐색 중입니다. instance·정확한 바이너리 해시/PID·네이티브 창 제목/ID·화면·외부 fixture 로그를 교차검증했습니다.
- 탐색 자체의 cold/cache 분리 측정, Codex 우선·Claude fallback·미설치·실제 zsh `.zshrc` 전용 경로·동시/중복 요청·파일 변경/실패·timeout 정리는 단위·PTY 통합 검사와 최초 기록에 있습니다. 수정 전 1.116초 조회 대기는 별도 baseline 실행의 근거입니다.

[최초 구현·전후 측정](https://github.com/egoing/agent-gibbon/blob/egoing/issue-17-terminal-discovery/docs/verification/terminal-discovery-20261105.md) · [최종 앱 실행 근거·화면·겹친 시간 구간](https://github.com/egoing/agent-gibbon/blob/egoing/issue-17-terminal-discovery/docs/verification/terminal-discovery-unlocked-20261105/README.md) · [디자인 시스템](https://github.com/egoing/agent-gibbon/blob/egoing/issue-17-terminal-discovery/docs/design-system.md#cli-탐색과-시작-실패--17-2026-11-05)

## 범위와 남은 검증

#51은 잠금 해제 후 원본 앱과 이번 수정본 모두에서 표시·입력 경로를 확인해 이전 화면 검증 보류 사유가 해소됐습니다. Orca `list-windows`의 빈 목록은 정상 `get-app-state`·입력·캡처와 불일치하는 도구 관측 한계로 남깁니다. 잘못된 interpreter에 대한 portable-pty 오류 #50은 별도 미해결이며 이번 개선으로 해결됐다고 주장하지 않습니다.

실제 모델·인증 우회 없이 fake CLI만 사용했습니다. Windows·Intel 실환경, Finder 직접 실행, 실제 모델 CLI 인증/신뢰, 전체 한글 IME, 교차 워크트리 격리 재검증과 release/설치 검증은 미실행입니다. Conf 5 / Risk Medium은 macOS ARM64의 요청된 fake CLI 사용자 경로에 한정합니다. Draft 상태를 유지하고 감독의 병합 판단을 기다립니다.

Closes #17
