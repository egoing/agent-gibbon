# #17 잠금 해제 후 최종 앱 실환경 검증 — 2026-11-05

**제품 추가 수정 없이 최종 개발 앱의 실제 READY/ECHO 표시·네이티브 키·resize·동일 PTY 숨김 복원, 느린 탐색 중 기존 입력·크기 변경·조회 응답을 확인했다.** 이전의 화면 표시 미검증 사유는 이번 실행으로 해소했다. Draft PR #52는 유지하며 감독의 병합 판단을 기다린다. 잘못된 interpreter를 portable-pty가 처리하는 [#50](https://github.com/egoing/agent-gibbon/issues/50)은 별도 미해결 범위로 유지한다.

## 제품과 실행 출처

2026-11-05 08:07–08:11 KST, 잠기지 않은 macOS ARM64의 활성 디스플레이 3개에서 실행했다. [환경](environment.json)은 console/login=true이며 잠금 키가 없다. 실제 설치 CLI·유료 모델·인증/신뢰/권한 우회는 사용하지 않았다. 외부 fake CLI와 탐색 shell에만 로그를 넣었고 제품 observer·JS 실행·임시 IPC를 추가하지 않았다.

| 출처 | 근거 |
|---|---|
| 저장소 / 검증 HEAD | `egoing/agent-gibbon`, `1b0a31ae7a72bac5fd9f601efe5edd4443ef0ff9` |
| 작업 루트 | `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/issue-17-terminal-discovery` |
| 개발 정보 | `npm run dev:info`의 root 일치, instance `dev-40ca7f182ba9bf0b29e94b20` |
| 바이너리 | 위 root의 `src-tauri/target/debug/agent-gibbon`, SHA-256 `c8af22a15063f6b9e86ba5348ff54ad53496c13b79cf53a8ce00fa7b136a7ad6` |
| 빌드 / 제품 동일성 | `npm run dev:build` 성공([로그](build.log)); 실행 전 제품 경로 `ui src-tauri scripts`의 `git diff 1b0a31a`가 비어 있음. 후속 변경은 문서와 외부 fixture뿐 |
| 현재 PATH 실행 | 앱 PID `50183`, 첫 터미널 `agent-1793833639537185000`, fake CLI PID `50334`, native window ID `33931` |
| 제한 PATH 실행 | 앱 PID `62412`, 첫 터미널 `agent-1793833831951440000`, fake CLI PID `62893`, native window ID `33969` |
| 캐시 창 | `agent-1793833833622089000`, fake CLI PID `63034`, native window ID `33970` |
| 정확한 창 제목 | `Agent Gibbon [DEV egoing/issue-17-terminal-discovery · 40ca7f18] — <위 agent ID>` |

각 앱 PID의 `ps` 실행 파일 경로·바이너리 해시·개발 정보·진행 메모 `inspect.ok/instance/본문`은 [실행 JSON](run.json)에 있다. 모든 생성·조회·숨김·복원·진행 갱신은 이 워크트리의 `scripts/agent-gibbon-dev`만 사용했다. 재시작 뒤 개발 출처와 새 PID를 다시 확인했다. [native 현재 PATH](current-native.json), [native 제한 PATH](limited-native.json), [캐시 창 native](limited-cache-native.json)의 PID/window ID/정확한 제목과 각 GUI 응답의 snapshot·ScreenCaptureKit 캡처 ID를 대조했다. 전체 화면 캡처는 사용하지 않았다.

## 탐색과 캐시

외부 fixture는 [가짜 Codex](fixtures/fake-codex.py), [탐색 shell](fixtures/discovery-shell.py)이다. `/tmp/gibbon17-40ca7f-unlocked`의 전용 PATH에는 fake CLI와 실행기용 node 링크, `/usr/bin:/bin`만 사용했다. 제한 PATH의 현재 검색 경로에는 Codex/Claude가 없으며 shell만 fake CLI 경로를 반환한다. 일반 로그인 셸이나 실제 사용자 설정을 고치지 않았다.

| 최종 앱 조건 | 첫 창 / 다음 캐시 창까지 running | 탐색 셸 실행 |
|---|---|---|
| 현재 PATH의 fake Codex, PID 50183 | 0.442 / 0.437초 | 0회 |
| 현재 PATH에 CLI 없음, 1.2초 지연 로그인 경로, PID 62412 | 1.587 / 0.291초 | 두 창 합계 `-lc` 1회, `-ilc` 0회 |

현재 PATH 실행에서는 shell 로그가 생성되지 않았다. 제한 PATH의 [shell 로그](limited-cache-shell-events.jsonl)는 begin/end 한 쌍뿐이며 두 창의 PTY start와 PID는 [외부 CLI 로그](fake-events.jsonl)에 있다. 캐시 창은 숨겼다가 복원한 뒤에도 PID `63034`가 같고 [실제 READY 화면](limited-cache-visible.png)을 확인했다.

위 시간은 개발 CLI 프로세스·창/WebView·탐색·PTY·fake CLI 시작·polling을 포함하는 단일 실행값이다. 탐색만의 시간이나 실제 모델 CLI의 준비 시간이 아니며 p50/p95·Finder 직접 실행 성능 보장은 아니다. 탐색 자체의 cold/cache 분리 측정(389.054ms/620.666µs), Codex 우선·Claude fallback·미설치·실제 zsh의 `.zshrc` 전용 경로·동시 요청·중복 시작·timeout 정리는 [기존 94개 Rust 검사와 최초 기록](../terminal-discovery-20261105.md)에 별도로 남아 있다. 이번 실행은 최종 앱의 제한 PATH와 login 첫 탐색·캐시를 재확인했으며 `.zshrc` 경로를 다시 GUI 실행했다고 주장하지 않는다.

## 실제 표시·네이티브 입력·resize·숨김 복원

| 검증 | 실행과 직접 확인한 결과 |
|---|---|
| 실제 본문 | `orca computer get-app-state --app pid:50183 --window-id 33931 --json`; 창의 agent ID와 초록 `GIBBON17_FINAL_READY`, `SIZE 89x28` 확인([화면](current-ready.png), [원본](current-ready.json)) |
| 텍스트/네이티브 키 | fresh AX `Terminal input` 클릭 후 `type-text NATIVE17_A`, `press-key Return`, 네이티브 키 `1`, `7`, `Return`. `31 37 0d` 바이트와 `GIBBON17_ECHO:17`이 외부 로그·[실제 화면](current-native-echo.png)에 일치 |
| resize | `drag --from-x 717 --from-y 457 --to-x 857 --to-y 547`; native 논리 크기 720×460→860×550, inspect 물리 1440×920→1720×1100, PTY 89×28→107×34, fake CLI SIGWINCH와 [화면](current-resize.png) 일치 |
| 네이티브 숨기기 | fresh AX 숨기기 버튼 클릭; [inspect](current-hidden-inspect.json)의 visible=false, PID `50334`·running=true·PTY 크기 유지 |
| 동일 세션 복원 | `scripts/agent-gibbon-dev open --id agent-1793833639537185000`; [inspect](current-restored-inspect.json) visible=true, 동일 PID·window ID·위치·크기. [READY/ECHO 스크롤백 화면](current-restored.png), start 이벤트 1회 |

Orca의 synthetic/unverified 표기만으로 성공을 판단하지 않았다. 실제 ECHO 화면, fake CLI 입력 바이트·SIGWINCH, 제품 inspect 응답을 교차 확인했다. 초기 동작은 백그라운드로 전달됐으나 이후 포커스가 바뀌는 관측도 있어 백그라운드 메타데이터만으로 포커스 보존을 단정하지 않는다. 전면 조작이 필요할 때 안내하고 직전 앱/포인터를 보관·복원했다.

## 느린 탐색과 기존 입력·resize·조회가 겹친 근거

PID `62412`의 기존 터미널 `agent-1793833831951440000`을 사용했다. 캐시된 **자체 fixture 실행 파일**을 삭제해 무효화한 뒤 새 창 `agent-1793833882704752000`을 생성했다. 탐색용 새 창은 CLI로 즉시 숨겨 기존 창에 대한 입력 대상을 안정적으로 유지했으며, 숨긴 새 창의 탐색은 계속됐다. 외부 shell은 2.2초 지연 후 다른 fixture 경로를 반환했다. 새 CLI PID `65253`의 running도 최종 확인했다. 숨기기 전후를 포함한 원본 명령·시각·응답은 [responsive.json](responsive.json), shell 원본은 [slow-shell-events.jsonl](slow-shell-events.jsonl)에 있다.

| 사건 | Unix 시각(초) | shell begin 이후 | 결과 |
|---|---|---|---|
| 외부 shell begin | 1793833882.958628 | 0 | PID 64280, `-lc` |
| 기존 CLI의 네이티브 키 `1` 수신 | 1793833883.201508 | +0.243초 | PID 62893, hex `31` |
| 기존 CLI의 Return / ECHO | 1793833883.404965 / 1793833883.405416 | +0.446 / +0.447초 | hex `0d`, `SLOW17_1` |
| 기존 CLI의 SIGWINCH | 1793833883.830629 | +0.872초 | 107×34 |
| 외부 shell end | 1793833885.160514 | +2.202초 | 같은 PID 64280 |

조회 20회는 모두 예상 instance·동일 기존 PID·running=true였으며 최대 **89.787ms**였다. 그중 **18회는 호출 시작과 종료 모두 shell begin/end 안**에 들어간다. 네이티브 키/ECHO와 resize도 shell end 전에 외부 CLI가 수신했다. [실제 ECHO·크기 화면](slow-result.png)과 제품 inspect가 같은 기존 세션임을 확인했다. 이는 이번 fixture·환경에서 탐색 잠금 때문에 기존 사용자 경로가 멈추지 않음을 보이는 실행 근거이며 모든 부하에서의 지연 상한 보장은 아니다. 기존 코드의 **1.116초 조회 대기**는 [최초 수정 전 기록](../terminal-discovery-20261105.md)의 별도 실행이며 이번 최종 앱 수치와 혼동하지 않는다.

## 실패 시도·도구 관측·한계

- 숨김 복원 뒤 다중 창에서 Orca가 `background_unavailable` 또는 `window_not_focused`를 반환한 시도는 입력 성공으로 집계하지 않았다([예](slow-coordinate-focus.json), [예](slow-background-focus.json)). 처음 2.8초 shell 시도도 단계 제한으로 완료하지 못해 최종 응답성 수치에서 제외했다. 오래된 실패 후속 polling 프로세스는 정확한 자체 PID로 종료했다. 제품 코드를 바꾸지 않고 제한 PATH로 앱을 재시작해 새 PID·inspect·창 출처를 다시 확인했다.
- 제한 PATH 새 앱의 AX 클릭·백그라운드 좌표 클릭만으로는 focus가 확인되지 않았다. 한 번의 전면 좌표 클릭으로 `focusedElementId=12`를 확인한 뒤 위 응답성 검증을 실행했다([원본](limited-foreground.json)). 입력을 보내도 결과가 불명인 시도는 최종 근거로 사용하지 않았다.
- `list-windows`의 빈 목록은 이번에도 [관측](current-list-windows.json)했지만 같은 PID의 `get-app-state`가 정상 title·AX·개별 창 캡처를 제공했다. 이는 [#51 원본 앱 검증](https://github.com/egoing/agent-gibbon/issues/51)과 같은 도구 열거 경로의 한계이며 제품의 표시 실패로 판정하지 않는다. 후속 도구 초안은 기존 #51 보고서와 중복이므로 새 제품 이슈를 만들지 않았다. native helper의 오래된 AX 배열 조회도 자체 helper만 실패했으며 제품 검증 근거로 사용하지 않았다.
- Windows·Intel 실환경, Finder 직접 실행, 실제 설치 모델 CLI 인증/신뢰, 전체 한글 IME, 교차 워크트리 격리 재검증, release/설치/공개 배포는 미실행이다. [#50](https://github.com/egoing/agent-gibbon/issues/50)의 잘못된 interpreter PTY 오류는 실행하지 않았고 해결을 주장하지 않는다.

## 검사와 판정

- `npm run dev:build` 통과, `npm test` Rust **94개**·UI **53개** 통과([로그](test.log)), `npm run check` 통과([로그](clippy.log)), Rustfmt·버전 검사 **0.2.7**·`git diff --check` 통과.
- `python3 docs/verification/terminal-discovery-unlocked-20261105/verify.py`: instance, 네이티브 출처, ECHO, resize, 같은 PID의 숨김 복원, cache shell 1회, 탐색과 입력/resize/inspect의 시간 겹침을 기록 데이터에서 검증했다. 이 검사는 저장한 실행 근거의 일관성 검사이며 새 GUI 실행은 아니다.
- 시작 HEAD `1b0a31a`의 GitHub Release CI에서 validate·macOS ARM64·Intel·Windows x64가 모두 성공했고 publish는 skipped였다([실행](https://github.com/egoing/agent-gibbon/actions/runs/41897698016)). 최종 문서 커밋의 CI 상태는 감독 인계 시 별도 보고한다.

**Conf 5 / Risk Medium**: macOS ARM64의 계측 없는 최종 앱에서 요청된 fake CLI 경로를 실제 확인했다. 세션 잠금·캐시의 제품 변경 영향은 Medium이며, 이번 추가 변경은 문서·외부 증거뿐이다. 형상·색·아이콘·기준 변경 없이 [디자인 시스템](../../design-system.md#cli-탐색과-시작-실패--17-2026-11-05)의 적용 상태와 검증 범위만 갱신했다. GitHub 게시 금지 지시에 따라 [최종 PR 설명 파일](pr-body.md)을 감독에게 제공하며 직접 PR 편집·병합은 하지 않는다.

## 자체 자원 정리

[cleanup.json](cleanup.json)에 정확한 실행 파일의 자체 앱 PID `50183`·`62412`, fake CLI PID 및 마지막 탐색 shell PID가 모두 소멸했음을 기록했다. 이번 작업은 caffeinate를 만들지 않았고 기존 PID `16291`은 유지했다. 이번에 생성한 프로젝트 7개와 대응 메타데이터 7개만 `/tmp/gibbon17-40ca7f-unlocked/test-storage`에 보관했으며 이전 메모·터미널 경로는 건드리지 않았다. [진행 메모의 최종 내용](progress-final.json)은 원래 저장 위치에 남겼다. 앱 종료는 메모 숨기기와 별도로 수행했다.

전면 조작 직전에 기록한 앱 PID `49894`와 포인터로 복원 명령을 실행했고, 최종 정리 시 같은 앱이 전면임을 재확인했다. 사용자가 작업 중 포인터를 움직일 수 있어 정리 시 좌표가 초기 좌표와 항상 같다고 주장하지 않는다. 다른 앱의 위치·크기·숨김 상태는 조작하지 않았다. 사용자 `package-lock.json`의 시작·종료 SHA-256은 `952dd9010b1fbd5122c2effd7fe485583ce3e31b4efb9ee47cc579b133333fde`로 같다.

`fixtures/live.py`·`responsive.py`·`computer.py`는 이번 실행의 명령과 교차검증 방식을 보존한 외부 harness다. 재실행하려면 root·fixture 경로·현재 native window ID·PID·캐시 상태를 먼저 확인해 바꿔야 하며, 저장된 PID/창 ID를 다른 세션에 그대로 적용하지 않는다. `verify.py`만으로는 저장된 근거를 읽기 전용 검사할 수 있다.
