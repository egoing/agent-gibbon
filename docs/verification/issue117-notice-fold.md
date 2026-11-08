# #117 업데이트 안내 Ctrl+포인터 접기 제외 검증

- 대상: egoing/agent-gibbon #117, PR [#132](https://github.com/egoing/agent-gibbon/pull/132), 2026-11-08 macOS ARM64.
- 기준 SHA `aa75157316a380b4f933db2d2fc41dc07a4c5fe3`, ref `refs/is2/base/117`. red SHA `7f6b9dbacee3d4fbd1f3557339ffa987f7b9396e`, ref `refs/is2/red/117`.
- #127 (`770d783a290db99d71743b8fe88a2609b6eba058`)의 안내 제외 판정·두 진입점 차단·상수 공개·회귀 assertion만 재사용한다. 업데이트 주기와 릴리스 문서는 변경하지 않았다. #112는 변경하지 않았다.
- 외형·문구·버튼 배치는 [디자인 시스템](../design-system.md)과 [기존 SVG](../design/updater-ready-notice.svg)·[PNG](../design/updater-ready-notice.png)를 읽고 시각 확인했다. 동작 예외만 추가하므로 시안은 유지한다.

## TDD와 로컬 게이트

원래 모든 label에 허용되던 판정을 먼저 `true` 반환 함수로 분리했다(준비 커밋 `88a34ae`, Rust 208개 통과). 그 다음 테스트만 별도 red 커밋으로 생성했다. [red 로그](issue117/red.log)의 실패는 컴파일/환경 오류가 아니라 안내 label 제외 assertion이다. green에서는 그 테스트를 변경하지 않고 #127의 제외 조건을 넣었다.

| 항목 | 실행 | 결과 |
|---|---|---|
| 기준 | npm ci → npm test → npm run check → npm run dev:build | Rust 208·UI 178, Clippy·개발 빌드 통과 |
| 신규 | cargo test --locked --manifest-path src-tauri/Cargo.toml update_notice_is_excluded_while_memo_windows_keep_contact_folding | red assertion 실패 → green 1개 통과 |
| 전체 타입 | cargo check --locked --manifest-path src-tauri/Cargo.toml --all-targets | 통과 |
| 전체 회귀·UI | npm test | Rust 209·UI 178 통과 |
| 포맷 | cargo fmt --manifest-path src-tauri/Cargo.toml -- --check | 통과 |
| 린트 | npm run check | Clippy all-targets -D warnings 통과 |
| 개발 빌드 | npm run dev:build | 통과; GUI fixture 제거 후 재빌드 별도 확인 |

원본 로그는 [issue117/](issue117/)에 보존했다. 기준 log는 실행 중이던 `/tmp/gibbon117-baseline.log`를 종료 확인 후 복사했다. 기준 npm ci의 출력 자체는 이 log에 없으나 셸의 `&&` 연쇄가 마지막 개발 빌드까지 완료한 것을 확인했다. baseline 제외 0건, 테스트 SKIP 0건이다.

준비 단계 샌드박스 내부 실행은 소켓·PTY 권한 때문에 3개 실패했다([로그](issue117/seam.log)). 동일 소스를 기준 실행과 같은 권한으로 실행해 208개 통과한 [별도 로그](issue117/seam-unsandboxed.log)를 남겼다. 이 실패를 요구사항 red나 코드 수정으로 해결한 회귀로 세지 않았다.

## 실제 개발창 출처와 GUI

- root `/Users/egoinglee/.codex/worktrees/issue117-notice-fold/agent-gibbon`
- instance `dev-36ae4e7398777ce0cdab2feb`, branch `codex/117-notice-fold`
- binary `/Users/egoinglee/.codex/worktrees/issue117-notice-fold/agent-gibbon/src-tauri/target/debug/agent-gibbon`
- `npm run dev:info`/`node scripts/dev.mjs info`의 [원본](issue117/instance.json).
- GUI PID **47454**, 위 binary의 `--serve` 실행 경로를 `ps -p 47454 -o pid=,command=`로 확인했다. 진행 표시를 위해 직접 시작한 이전 PID 43758도 같은 경로를 확인 후 종료했다. 설치 앱 PID 95607은 종료·교체하지 않았다.
- 개발 CLI의 `inspect --id issue117-gui-control`에서 `ok: true`, instance 일치, 고유 본문 `GIBBON117-36ae4e73 GUI 양성대조`를 확인했다([초기 inspect](issue117/inspect-before.json)).
- AX 제목 `Agent Gibbon [DEV codex/117-notice-fold · 36ae4e73] — issue117-gui-control`, window ID 50554와 같은 PID의 `Agent Gibbon · 업데이트`, window ID 50560을 대조했다. [원래 창 목록](issue117/windows-before.json), [최종 스크롤 후 기준 AX](issue117/ax-scrolled-baseline.json), [캡처 메타데이터](issue117/notice-ctrl.png.json).

실제 설치 피드 없이 debug 안내를 생성하기 위해 [임시 fixture diff](issue117/gui-fixture.patch)로 `initialize`에서 상태만 ready/고유 버전/보류 문구로 넣고 기존 `show_ready_notice`를 실행했다. **접기 처리·창 생성·CSS·UI 코드·updater enabled/설치 경로는 바꾸지 않았다.** diagnostics의 updater.enabled=false, terms_accepted=false, terms_skipped=true, sharing_allowed=false를 확인했다. 다운로드·실제 설치는 수행하지 않았다. fixture 소스는 GUI 종료 후 원문과 `cmp`로 일치 확인하고 재빌드했다. 이 fixture는 최종 제품 코드에 포함하지 않는다.

설치 앱 메모와 겹치지 않게 새 검증 안내만 (600,500), 일반 메모만 (1200,400)으로 이동했다. CLI inspect는 물리 픽셀(배율 2), AX/화면 포인터는 논리 좌표다. 보류 피드백은 PID·제목·버튼으로 제한한 AXPress로 표시했으며 사용자 앱 포커스는 그대로 유지됐다. 콘텐츠 영역을 아래로 스크롤해 **보류 문구가 실제 보이는 상태**로 최종 시험했다.

| 실제 조작·대조 | 결과 |
|---|---|
| Ctrl 유지 후 보류 문구 진입 | 문구·창·버튼 위치/크기와 AX 내용 불변 |
| Ctrl 유지 후 나중에 버튼 진입 | 위와 동일 |
| Ctrl 유지 후 업데이트하고 재시작 버튼 진입 | 위와 동일 |
| 일반 메모 Ctrl 진입 | folded=true, 물리 높이 136→24 |
| 바깥으로 나갔다가 Ctrl 재진입 | folded=false, 높이 136 및 원래 위치·크기 복원 |
| 사용자 세션 보존 | front PID 79839와 포인터 원위치 복원; 설치 앱 모든 on-screen 창 ID·bounds 불변 |

[최종 결과 JSON](issue117/native-results.json), [양성대조 접힘 inspect](issue117/inspect-positive-folded.json), [복원 inspect](issue117/inspect-positive-restored.json), [Ctrl 접촉 캡처](issue117/notice-ctrl.png)를 보존했다. 캡처는 PID·window ID를 지정한 Orca의 실제 창 캡처다. [실행 스크립트](issue117/gui-check.py)와 [native 도우미](issue117/native-helper.swift)는 당시 PID와 임시 경로를 포함하는 증거용이며 범용 실행기로 제공하지 않는다.

초기 `screencapture -l`은 권한으로 실패했고 Orca 캡처로 안내 창 증거를 확보했다. 12pt 접힌 메모는 Orca의 window_not_found로 캡처되지 않으므로 양성대조는 실제 native 조작과 inspect의 folded/geometry 전후값으로 검증했다. 최종 안내 screenshot에는 보류 문구·두 버튼이 보인다. Windows/Intel 실기기 GUI와 실제 설치는 이 시험 범위에 포함하지 않는다.

## 독립 감사·최종 원격 검사

독립 감사, 최종 SHA Release 수동 dispatch의 실행 URL/headSha/3플랫폼 결과는 PR에 갱신한다. 통과 전 자동 병합하지 않는다. 위험도 Low: 안내 label만 접촉 접기에서 제외하고 일반 메모·설치 정책·데이터 모델을 유지한다. is2 커버리지 체크리스트를 대조했으며 새 파이프라인 시나리오를 검증했다는 주장은 추가하지 않았다. Project 보드 등록은 자동 승인 검토가 외부 보드 변경으로 거부하여 생략했다.
