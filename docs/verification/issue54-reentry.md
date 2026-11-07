# 이슈 #54 · 이동 후 즉시 재접근 (2026-11-07)

## 변경과 기준

말풍선 표시 중 접근 판정을 조기 반환하던 조건과 이동 시작부터의 2초 쿨다운을 제거했다. 최초 접근은 즉시 반응하고, 이후에는 **도착 후 96 논리 px 초과 이탈 → 56px 미만 재진입**에 한 번씩 반응한다. 이동 중에는 gate를 평가하지 않아 목적지 접근을 예약하지 않는다. 도착 위치에 머무는 포인터는 이탈하기 전까지 추가 이동을 일으키지 않는다.

말풍선은 이동 중 숨기고 정지 후 최대 6초 표시하되 새 접근이 있으면 중단한다. 기존 420ms 이징·네이티브 클릭 통과/비활성화 설정·창 복원 경로는 유지했다. [디자인 기준](../design-system.md#이동-후-즉시-재접근--이슈-54-2026-11-07), [SVG](../design/window-stash.svg)·[PNG](../design/window-stash.png)의 동작 설명을 갱신했다. 제품 외형·배치는 변경하지 않았다. PNG 렌더링을 직접 열어 설명의 겹침·잘림이 없는지 확인했다.

## 출처

- 작업 루트: `/Users/egoinglee/.codex/worktrees/6a93/agent-gibbon`
- 브랜치: `codex/issue-54-indicator-reentry`, 기반 HEAD `9623011` 위의 미커밋 변경.
- `npm run dev:info`: instance `dev-8bc1bbf27052853a25ad4d82`, binary `/Users/egoinglee/.codex/worktrees/6a93/agent-gibbon/src-tauri/target/debug/agent-gibbon`.
- macOS 15.6.1(24G90), ARM64, Rust/Cargo 1.91.1, Node v24.21.0, npm 11.19.0, Swift 6.2.4.
- 수정 전 PID `36413`, 검증 메모 `issue54-0662a793`. 수정 전 report에는 실행 시 바이너리 해시를 수집하지 않았으며 사후 해시를 시작 시 증명으로 사용하지 않는다.
- 수정 후 PID `44722`, 최종 메모 `issue54-0beb5311`. 모든 CLI 응답의 `ok`, `instance`, ID·본문을 대조했다. 실제 프로세스의 절대 실행 경로/PID, CLI의 물리 좌표를 native PID/bounds와 연결하고 Orca 개별 창의 **정확한 네이티브 제목 및 window ID**를 다시 대조했다. 캡처는 그 창 ID에 한정했다.
- 최종 바이너리 SHA-256: `71d618dc0743635291689794168cb2433b15fdb074508764779cc5c39d5eb12c`.
- `stash.rs` SHA-256: `30b57d5be3bfe85569094209d40b1af74476f25e00c60f29c4d814ea6fc4fdf1`.
- 최초 개발 빌드 후 이 작업이 시작한 PID `36413`의 정확한 실행 경로를 확인해 종료했고, 수정본을 다시 빌드·시작한 뒤 PID `44722`와 출처를 다시 확인했다. 설치 앱과 다른 워크트리는 종료하지 않았다.

## 재현과 실행

```sh
npm ci
npm run dev:info
npm run dev:build
# 위 워크트리의 개발 앱만 사용. 기존 이 인스턴스의 stash가 있으면 중단한다.
python3 scripts/stash-reentry-smoke.py before  # 수정 전 바이너리에서만
python3 scripts/stash-reentry-smoke.py after   # 수정 빌드·재시작 후
```

- 수정 전: 첫 회피 후 6초 안내 중 96px 밖으로 이탈하고 다시 접근해도 움직이지 않는 것을 재현했다. [수정 전 원본](issue54-before/report.json), [도착 캡처](issue54-before/first-arrival.png), [캡처 출처](issue54-before/first-arrival-source.json).
- Red: 기존 `step(distance, now, tip_visible)`를 그대로 둔 채 `reentry_during_hint_does_not_wait_for_hint_or_cooldown`을 추가했다. 첫 접근 후 450ms에 이탈, 500ms에 재진입하는 assertion이 실제 실패했다(exit 101). 그 뒤 제품 로직을 수정했다. `cargo test --manifest-path src-tauri/Cargo.toml stash::tests::reentry_during_hint_does_not_wait_for_hint_or_cooldown -- --exact --nocapture`로 수행했다. red 커밋은 만들지 않았고 원본 로그 파일은 별도로 보존하지 않았다.
- 수정 후: 실제 포인터로 최초 접근·두 번째·세 번째 회피, 이동 중 목적지 접근 후 정지 시 추가 추격 없음, 새 이탈·재진입에 다시 반응, 이동 중 안내 숨김·정지 후 안내 표시/만료를 확인했다. 최종 관측의 회피 진행 표본 시각은 8.529 / 9.209 / 10.550초로 간격은 약 0.680 / 1.341초였다. 스크립트는 각 이전 이동 시작 기준 2초 전에 다음 이동이 시작됐는지도 검사했다. 이는 한정된 기능 관측이며 입력 지연 분포나 정확한 네이티브 애니메이션 길이의 벤치마크가 아니다.
- 최종 실행에서 회피 전/후 frontmost PID `74919` 일치, 트레이 복원 후 메모 표시·위치·크기·핀·접힘 상태와 본문 유지, 검증 후 메모 숨김·포인터 원위치 복원을 확인했다. [최종 원본](issue54-after/report.json), [최종 캡처](issue54-after/third-arrival.png), [창 출처](issue54-after/third-arrival-source.json).
- 회피 판정과 반대였던 기존 `stash-smoke.py`·`stash-indicator-smoke.py` 기대를 즉시 재접근으로 바꿨다. `stash-first-approach-smoke.py after`는 최초 접근과 후속 재접근을 함께 검증하는 새 스크립트로 연결하며 before는 역사적 #36 재현용으로만 남긴다. 이 세 기존 스크립트 전체는 이번에 별도로 실행하지 않았다.

## 검증 결과

| 검사 | 결과 |
|---|---|
| Rust 전체 | 183개 통과, 실패·skip 0 ([로그](issue54-checks/rust.log)) |
| Node UI 전체 | 120개 통과, 실패·skip 0 ([로그](issue54-checks/ui.log)) |
| stash 회귀 | 7개 통과(즉시 반복 재진입, 56/96 경계·정지 포인터·단축키·복원 기하 포함) |
| Clippy all-targets `-D warnings` | 통과 ([로그](issue54-checks/clippy.log)) |
| Rustfmt, 변경 Python 4개 문법, `git diff --check` | 통과 |
| 개발 빌드 / version:check | 성공 / 0.2.7 일치 |
| macOS ARM64 실제 CLI·트레이·포인터·정지 후 화면 | 최종 실행 통과, 범위는 위 기록 |
| Windows·Intel 전체 플랫폼 Actions | 미실행 — 커밋 승인 후 push한 SHA로 요청 필요 |

## 실패 관측과 한계

- 처음 셸의 CGWindow 정보에는 창 이름이 없었고 직접 AXWindows와 Orca list-windows도 빈 배열이었다. 이를 창 부재로 판정하지 않았다. #53에 기록된 우회처럼 Orca get-app-state의 PID/window ID·제목·캡처로 출처를 확보했다. 실제 메뉴 조작은 PID를 지정한 System Events와 네이티브 클릭으로 실행했다.
- 반복 실행 한 번은 마지막 frontmost PID 비교가 달라 실패했다. [실패 원본](issue54-focus-mismatch/report.json)을 별도로 보존했다. 이 실행은 실패 시 두 PID를 기록하도록 보강하기 전이며, 해당 변화의 원인이 앱인지 외부 조작인지 확정하지 못했다. 최종 실행의 일치 결과가 이 관측의 원인 규명을 대신하지 않는다. 이후 스크립트는 두 PID와 실패 사유를 기록한다.
- 실제 환경은 동일 배율 2의 모니터 3개 중 한 화면이다. 혼합 DPI·물리 모니터 분리·Windows/Intel GUI는 미검증이다. 혼합 배율에서는 새 smoke의 창 출처 대조가 명시적으로 중단된다.
- 실제 클릭 통과·키보드 비활성화 설정은 변경하지 않았고 코드로 유지 여부를 확인했다. 밑의 다른 앱에 클릭이 전달되는 별도 실기는 이번에 반복하지 않았다. 전역 modifier chord는 다른 인스턴스가 소유하므로 보내지 않았고 PID에 속한 트레이만 사용했다. 최초/복원 애니메이션 전체·IME·캡처·터미널의 종합 회귀를 새로 통과한 것으로 확대하지 않는다.
- 공개 릴리스·설치 앱 교체·커밋·push·원격 CI·이슈 종료는 수행하지 않았다. 같은 gate가 여러 OS에서 쓰이므로 프로젝트 지침에 따라 원격 3플랫폼 검사가 남는다.

**Conf 5(기록된 macOS 기능 검증 범위) / Risk Low**: 변경은 로컬 회피 판정과 안내의 시간 결합 해제다. 창 복원·입력 권한·진단 전송·저장 형식은 변경하지 않는다.

## 후속: 안내 크기 조절

사용자 요청으로 `#tip`의 좌우 고정 폭을 내용 기반 `width: max-content`와 가용 폭 상한으로 변경했다. 오른쪽 아이콘과 간격은 그대로 두고 짧은 내용에서는 왼쪽 빈 공간을 줄인다. SVG/PNG 설명 시안도 갱신했다. UI 검사 120개·CSS 정적 확인·렌더링 확인을 수행하며, 이 CSS 변경 이후 네이티브 실기를 재실행하지 않았다. 위 실기 기록은 재접근 로직에 대한 것이며 후속 CSS 실기의 근거가 아니다. 사용자가 이번 작업의 원격 CI 생략과 커밋·push 진행을 승인했다.
