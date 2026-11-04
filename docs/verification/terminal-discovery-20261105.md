# 터미널 CLI 탐색 개선 #17 — 2026-11-05

탐색 지연과 세션 목록 잠금은 변경 전 실제 개발 앱에서 재현했고, 성공 캐시·단계별 fallback·창별 시작 예약을 구현했다. **최종 앱의 본문 표시와 네이티브 입력은 확인하지 못해 병합 보류**다. 관련 후속은 [#50](https://github.com/egoing/agent-gibbon/issues/50)(잘못된 interpreter의 PTY spawn)과 [#51](https://github.com/egoing/agent-gibbon/issues/51)(빈 본문/접근성 창 목록)이다.

## 출처와 재현 범위

- 기준 main/HEAD: `607431c555af96216ff026685ee08026473b9665`, `git fetch origin main` 뒤 일치 확인. 시작 시 clean이며 root `package-lock.json`은 변경하지 않았다.
- root: `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/issue-17-terminal-discovery`.
- `npm run dev:info`: instance `dev-40ca7f182ba9bf0b29e94b20`, binary는 위 root의 `src-tauri/target/debug/agent-gibbon`. Node 24.21.0, rustc/cargo 1.91.1, macOS arm64, portable-pty 0.9.0, xterm 6.0.0.
- 모든 앱 통신은 이 root의 개발 CLI를 사용했다. 최초 시작 1회에는 같은 실행기의 `node scripts/dev.mjs cli` 진입점을 사용했고, 이후 `scripts/agent-gibbon-dev`만 사용했다. 설치 앱·다른 워크트리로 대체하지 않았다.
- 매 재시작 뒤 `inspect.ok/instance`, ID/본문 표식, `ps`의 정확한 binary/PID를 대조했다. GUI는 해당 PID의 CGWindow ID와 agent-ID를 포함한 정확한 native 제목으로 연결했다. 전체 화면 캡처를 쓰지 않았다.
- 처음 active display=0인 절전을 확인했다. 자체 `caffeinate -d -u -t 3600` PID 86875로 일시적으로 깨운 후 display=2와 앱 재시작을 확인했다. 영구 전원 설정은 변경하지 않았다.

[정량 원본·인스턴스·PID·창 제목·검사 응답](terminal-discovery-20261105/report.json)에 각 실행과 SHA-256을 보관했다. `/tmp/gibbon-issue17/`는 원본/중간 실패 로그와 임시 빌드의 보관 위치다.

## 변경 전후 측정

모델을 호출하지 않는 [fake Codex](terminal-discovery-20261105/fake-codex.sh)와 [1.2초 지연 shell](terminal-discovery-20261105/fake-shell.sh)을 사용했다. 셸 탐색만의 단축과 PTY/CLI/창 생성 비용을 구분한다. 아래 값은 단일 실행의 관측값이며 p50/p95 또는 범용 성능 보장은 아니다.

| 조건 | 첫 창 / 다음 창까지 running | 셸 실행 | 기존 터미널 조회 |
|---|---|---|---|
| 변경 전 원본 앱 PID 76424 | 별도 수치 미측정 | 두 번째도 `-ilc` | 느린 탐색 중 CLI inspect **1.116초** |
| 변경 전 + 짧은 observer, PID 835 | **1.460 / 1.450초** | `-ilc` 2회 | IPC info 30표본 중 최대 **1210ms** |
| 수정본 + 같은 observer, PID 96323, 현재 PATH의 Codex | **0.292 / 0.317초** | **0회** | IPC info 35표본 중 최대 **3ms** |
| 수정본 + observer, PID 97501, 제한 PATH | **1.492 / 0.284초** | `-lc` **1회** | IPC info 32표본 중 최대 **8ms** |
| **최종 계측 없는 앱 PID 2915**, 제한 PATH | **1.554 / 0.291초** | `-lc` **1회** | 실행 파일 삭제 후 재탐색 중 CLI inspect **0.067초**, 기존 PID 유지 |

- 탐색만의 분리 측정: `cargo test --manifest-path src-tauri/Cargo.toml terminal::discovery::tests::discovery_and_cache_times_are_separate_from_cli_startup -- --nocapture`, **cold 389.054ms / cache 620.666µs**. 150ms 지연 fixture와 프로세스 시작/폴링 비용이 포함되며 CLI는 실행하지 않는다. [원본 로그](terminal-discovery-20261105/discovery-timing.log).
- 창 running 수치는 개발 CLI 프로세스 실행·WebView·PTY·로컬 CLI 시작과 polling을 포함한다. 실제 Codex/Claude 자체의 대화 준비 시간, Finder 실행 환경, 유료 모델 응답 시간을 측정한 값이 아니다.
- 최종 pristine 첫 ID `agent-1793820369572072000`, 다음 ID `agent-1793820371426656000`, 삭제 후 재탐색 ID `agent-1793820392725209000`. 각 inspect instance는 위 개발 ID와 일치한다. 상세 PID/PTY/창 ID는 JSON을 기준으로 한다.

## 구현과 검증

- 현재 PATH에서 Codex를 발견하면 셸을 실행하지 않는다. 없으면 `-lc` → `-ilc`를 진행하며 현재/로그인 PATH의 Claude만 발견돼도 Codex 탐색을 마친 뒤 fallback한다. PATH 누락과 `.zshrc` 전용 설치 경로를 fixture로 확인했다. zsh 전용 검사는 macOS에 한정한다. Windows 터미널의 기존 미지원 경계를 유지한다.
- 성공 경로/PATH와 파일 metadata를 앱 메모리에 공유한다. 파일 삭제·교체·실행 비트 변경은 캐시를 무효화하고, 동시 창 요청은 같은 탐색을 기다린다. 기다리는 시간도 각 요청의 전체 예산에 포함된다.
- 셸 단계 최대 3초, 전체 탐색 최대 4초를 적용한다. spawn 오류는 같은 예산으로 한 번만 재탐색한다. 확인된 비정상 CLI 종료는 캐시를 무효화하며 인증/신뢰 실패일 수 있어 자동 재시작하지 않는다. spawn Err 재시도는 결정적인 주입 검사로 2회 상한/기한 공유를 확인했고, 실제 `exit 23` PTY는 종료 감지·다음 명시 요청의 재탐색을 확인했다.
- timeout/error 시 전용 셸 프로세스 그룹과 자식을 kill/wait한다. 성공 셸이 시작한 백그라운드 서비스는 기존 정책대로 보존한다. 실패/정리 회귀는 살아 있는 sleep 자식이 남지 않음을 검사했다.
- 세션 목록에는 창별 Starting 예약만 짧게 기록하고 탐색·PTY 시작 동안 잠금을 놓는다. 같은 창 중복을 거부하고 실패 시 예약을 해제한다. 다른 창의 탐색 중 기존 PTY 입력 ECHO·97×31 resize·프로세스 상태 확인이 진행됨을 실제 portable-pty fixture로 검사했다.

| 최종 검사 | 결과 |
|---|---|
| `npm test` | Rust **94개**, UI **53개** 통과 ([로그](terminal-discovery-20261105/test-pristine.log)) |
| `npm run check` | Clippy 경고 오류 처리 통과 ([로그](terminal-discovery-20261105/clippy-pristine.log)) |
| `npm run dev:build` | 성공 ([로그](terminal-discovery-20261105/build-final.log)), 위 PID 2915로 새 실행 |
| Rustfmt / `npm run version:check` / `git diff --check` | 통과 / **0.2.7** / 통과 |
| 실제 개발 앱 observer IPC | fake CLI 수신·resize 응답·중복 요청 거부·info·숨김/복원 PID 보존 확인 |
| 최종 계측 없는 앱 | 탐색·캐시·삭제 후 재탐색·기존 inspect 응답·숨김/복원 PID 보존 확인 |
| 최종 본문 표시·네이티브 키 입력 | **미검증**, 통과로 집계하지 않음 |

## 계측과 실패 경계

일시적 [observer 패치](terminal-discovery-20261105/observer-only.patch)는 기존 `terminal_write/resize/info`를 호출하고 fake CLI가 받은 로컬 입력에 결과를 기록한다. 인증·신뢰·권한 경로는 변경하지 않았다. [observer 원본](terminal-discovery-20261105/observer.js), [실행 harness](terminal-discovery-20261105/live.py)는 환경별 재현 자료이며 실행 전에 그 안의 root/instance/fixture 경로를 확인해야 한다. 최종 `ui/terminal.js` diff는 비어 있고 제품 빌드에서 observer를 제거했다.

- 최초 observer의 한 줄 JSON은 PTY canonical 입력 상한을 넘겨 후속 기록을 얻지 못했다. 이를 입력 성공으로 보고하지 않고 짧은 PROBE/요약으로 바꿔 동일한 baseline/수정본 observer를 비교했다.
- 140ms timeout fixture는 셸 본문 시작 전 만료되어 PID 기록이 없었다. 단계 900ms/전체 1400ms로 본문 시작과 descendant PID를 관측한 뒤 단계/전체 상한 및 자식 소멸을 검증했다. 제품 시간 제한은 바꾸지 않았다.
- 잘못된 shebang은 portable-pty에서 예상한 spawn Err 대신 자식의 Rust `output.write` assertion abort와 Ok PID를 보였다. 실제 GUI 앱에서 이 오류를 실행하지 않았으며 부모 GUI의 이 경로 생존/응답은 미검증이다. [#50](https://github.com/egoing/agent-gibbon/issues/50)을 따로 발행했고 원인 확정·라이브러리 교체는 이번 범위에 포함하지 않았다.
- [변경 전 캡처](terminal-discovery-20261105/before-observer-short.png), [수정본 observer 캡처](terminal-discovery-20261105/after-observer.png), [최종 pristine 캡처](terminal-discovery-20261105/final-pristine.png)는 제목 아래가 비어 보인다. observer의 xterm buffer에는 READY/PROBE/ECHO가 있고 요소 높이는 428px였다. 네이티브 전면 입력도 fake CLI 로그로 성공을 확인하지 못했으며 전면 앱 PID **427**과 포인터는 복원했다. granted 권한 응답에도 AX 창 목록이 비어 있으며, 환경/표시/캡처 중 원인은 미확정이다. [#51](https://github.com/egoing/agent-gibbon/issues/51)에 분리했다.

**Conf 3 / Risk Medium**. 탐색 개선의 실제 일부 경로와 단위·PTY 통합 검사는 확인했지만 최종 사용자 화면·네이티브 입력 E2E를 완료하지 못했다. Intel Mac·Windows·Finder의 실제 축소 환경·실제 설치 CLI 인증/신뢰·한글 IME·교차 워크트리 격리 재검증·공개 릴리스는 미실행이다. 형상·색·아이콘을 바꾸지 않았으며 [디자인 시스템](../design-system.md#cli-탐색과-시작-실패--17-2026-11-05)에 안내 기준과 적용 상태를 갱신했다.

## 감독 판정과 종료 정리

감독자는 **Draft PR 보존·병합 금지**, 이후 **#51 → #50** 순차 처리를 지시했다. 구현은 한국어 커밋 `ea5782f`로 보존했고 [Draft PR #52](https://github.com/egoing/agent-gibbon/pull/52)를 생성했다. CI 완료를 기다려 병합하거나 #17을 닫지 않았다.

자체 최종 앱 PID **2915**의 정확한 binary를 다시 확인한 뒤 종료했다. 이 워크트리 앱 프로세스는 **0개**, 기록한 fake CLI 자식은 **0개**, 자체 caffeinate PID **86875**도 종료됨을 확인했다. 추가 자식 그룹 강제 종료는 필요하지 않았다. 생성한 검증 프로젝트 13개와 대응 metadata 13개만 `/tmp/gibbon-issue17/test-storage/`로 보관했고 진행 메모는 유지했다. 설치 앱·다른 워크트리·다른 caffeinate는 종료하지 않았다. 네이티브 입력 helper는 포인터를 복원했고 전면 앱 PID **427**이 초기와 같음을 확인했다. 이동한 ID/경로·검사한 PID와 최종 상태는 [JSON의 cleanup](terminal-discovery-20261105/report.json)에 있다.
