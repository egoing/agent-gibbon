# #36 최초 접근 즉시 회피 검증

2026-11-05 macOS ARM64, Rust 1.91.1, Node v24.21.0에서 수행했다. 기준 HEAD와 원격 main은 모두 `661d834a9f09d8d3f5800474d12ab4f48ebeee0e`였다.

## 결과와 원인

초기 안내 6초 동안 `EscapeGate` 호출 자체가 생략되어 첫 접근이 지연됐다. 최초 회피 이력이 없으면 초기 안내를 중단해 즉시 회피하도록 바꿨다. 회피 이력이 생긴 뒤에는 기존 안내 보호·96px 이탈 후 재진입·2초 쿨다운을 그대로 유지한다. 보호 시간의 접근·이탈은 예약하지 않는다.

420ms smoothstep 이동, 이동 중 말풍선 숨김, 마지막 네이티브 이동 다음 틱부터 6초 표시, 클릭 통과·비활성화 경로는 유지했다. 디자인 시스템과 설명 시안의 최초 접근 기준을 함께 갱신했다.

## 출처

- 작업 루트: `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/issue-36-first-approach`
- `npm run dev:info`: `dev-8fbfdca2d77ba80c9f021e8f`
- 실행 파일: 위 루트의 `src-tauri/target/debug/agent-gibbon --serve`
- 수정 전 PID `31113`, 메모 `issue36-first-approach`
- 수정 후 PID `40620`, 메모 `issue36-first-approach-0169d7c1`
- 두 실행 모두 해당 루트 개발 CLI의 `inspect`에서 `ok:true`, instance 일치, 고유 ID·본문을 확인했다. 각 캡처는 동일 PID의 `Agent Gibbon [DEV egoing/issue-36-first-approach · 8fbfdca2] — stash-indicator` 네이티브 제목과 window number에 연결했다. 네이티브 창 정보는 캡처 옆 `*-source.json`에 있다.
- Orca computer capabilities/drag 도움말과 창 목록을 조사했다. 순수 이동 API가 없어 감독자가 승인한 기존 `entrance-pointer.swift`를 사용했다. CGWarp와 mouseMoved만 보내며 클릭은 보내지 않았다. 기존 `stash-key.swift`로 세 보조키를 누르고 놓았다. 원래 실제 포인터 좌표 `(655.5,1012.48046875)`를 양쪽 실행 모두 복원했다.

## 실제 포인터 전후 비교

시간은 각 report의 monotonic 기준이며 CLI·포인터 도구 실행 시간을 포함한다. 밀리초 단위 지연의 정밀 계측을 주장하지 않는다.

| 상황 | 수정 전 | 수정 후 |
|---|---|---|
| 최초 숨김 뒤 초기 안내 중 접근 | t2.354 `(2524,1404)` 접근, t2.639에도 아래쪽 y1380 정지·안내 표시 | t5.537 동일 좌표 접근, t5.812에는 y585 이동 중·안내 숨김 |
| 첫 이동 완료 | 초기 안내가 끝나야 같은 포인터에서 지연 이동 | t6.150 y37 도착·안내 표시 |
| 도착 직후 아이콘 접촉 | 안내 중 정지 유지 | t6.309 `(2524,61)` 접근, t6.727 정지·안내 표시 |
| 안내 중 96px 이탈·재진입 | 추가 이동 없음 | t6.727 `(600,500)` 이탈, t6.770 재진입, t7.189 추가 이동 없음 |
| 안내 종료 뒤 계속 접촉 | 자동 추격 없음 | t12.263에도 y37 유지·안내 종료 |
| 안내 종료 뒤 96px 이탈·재진입 | 한 번 이동 | t12.428 재진입, t12.543부터 이동, t12.923 y1380 도착·안내 표시 |
| 이동과 안내 | 중간 위치 기록·이동 중 안내 숨김·도착 후 표시 | 두 번째 이동 y102→408→826→1270→1380; 중간 프레임 안내 없음, 최종 도착 안내 있음 |
| 포커스·복원 | 전면 PID 16242 유지·동일 단축키 복원 | 전면 PID 16242 유지·동일 단축키 복원 |

- [수정 전 시간·좌표·상태 전체](issue36-before/report.json), [최초 접근 화면](issue36-before/first-response.png), [도착 화면](issue36-before/arrived.png)
- [수정 후 시간·좌표·상태 전체](issue36-after/report.json), [첫 반응 뒤 캡처](issue36-after/first-response.png), [도착 화면](issue36-after/arrived.png)

캡처 명령 실행에는 시간이 걸리므로 수정 후 `first-response.png` 자체를 이동 중 프레임이라고 해석하지 않는다. 이동 중 상태·중간 좌표·안내 숨김은 report의 inspect 샘플로 판정했다. 실제 420ms 설정은 변경하지 않았으며 두 번째 이동은 재진입→도착 관측이 495ms이고 그 안에 CLI/포인터 호출 비용과 폴링 간격이 포함된다. 별도 프레임 단위 정밀 타이밍 계측은 하지 않았다.

최초 재현 하네스 시도에서는 숨김 완료 직후 inspect의 표시 상태와 네이티브 배치 사이 한 틱 차이를 고려하지 못해 잘못된 이전 좌표를 사용했고 비교가 실패했다. 앱 수정 전에 네이티브 배치가 안정된 뒤 좌표를 읽도록 하네스를 고쳐 수정 전 재현을 완료했다. 위 수정 전 증거는 완료된 재현 실행이다.

## 실행 검사

- `npm run dev:build` 수정 전 기준 빌드, 수정 후 재빌드 모두 통과; 수정 전 앱 종료 뒤 새 PID에서 수정 반응을 확인했다.
- `npm test`: Rust **77**, UI **53** 통과.
- `npm run check`: Clippy all-targets, 경고 오류 처리 통과.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`: 통과.
- `rsvg-convert docs/design/window-stash.svg -o docs/design/window-stash.png`: 렌더링하고 직접 열어 글자·배치 확인.
- 수정 전: `python3 /tmp/gibbon-issue36/repro.py before` (기존 Swift 하네스를 사용한 일회 재현).
- 수정 후: `python3 scripts/stash-first-approach-smoke.py after`: 통과. 같은 핵심 시나리오를 재실행할 수 있도록 전후 모드를 가진 스크립트를 남겼다. 개발 앱 실행/종료는 호출자가 소유하며 독립된 테스트 인스턴스에서 사용한다.
- 회귀 단위 검사는 최초 안내 중 즉시 회피, 회피 후 안내 보호, 보호 중 이탈 비예약, 56px/96px 경계, 2초 쿨다운을 검사한다.

## 범위와 한계

Conf **5**, Risk **Low**: 실제 macOS ARM64 개발 앱에서 해당 증상을 전후 비교했고 국소 접근 게이트만 바꿨다. Windows·Intel·모니터 해제·다른 DPI 조합은 이번에 실행하지 않았다. 전체 창 접어두기 기능의 다른 독립 시나리오는 범위를 확대해 반복하지 않았다. 새 독립 문제는 발견하지 않았다. 설치 앱·다른 워크트리 앱은 시작·종료하지 않았다. 검증을 위해 직접 시작한 PID 31113과 40620만 종료했으며 [최종 종료 확인](issue36-after/cleanup.json)을 남겼다. 감독자가 검증 후 이번 변경의 커밋·push·PR 생성을 승인했으며 병합 판단은 감독자에게 남긴다.

[디자인 시스템](../design-system.md#최초-접근-즉시-회피--이슈-36-2026-11-05) · [시안 SVG](../design/window-stash.svg) · [시안 PNG](../design/window-stash.png)

## main b931ac2 통합 확인 (2026-11-05)

clean 상태의 `ac32ad6`에 origin/main `b931ac2`를 병합했다. 검증 문서 충돌은 #36·#34·#43 양쪽 기록을 보존했다. 디자인 시스템의 열기 순서·전환 보정과 최초 접근 기준을 함께 유지했다.

- 통합 개발 빌드, Rust **84**·UI **53**, Clippy all-targets 경고 오류 처리, Rustfmt, 버전 0.2.7 검사, diff 검사 통과. 원본 [테스트 로그](issue36-integrated/tests.log)·[Clippy 로그](issue36-integrated/clippy.log).
- `python3 /tmp/gibbon-issue36-integrated-smoke.py after` 1회 실행. 저장 경로만 `issue36-integrated`로 격리한 기존 하네스이며 최초 접근까지 **도달하지 못해 실패**했다. 세 보조키 입력 뒤 `stash.active=false`가 10초 유지됐다. [시간·상태 보고](issue36-integrated/report.json).
- root는 위 출처 절과 같고 instance는 `dev-8fbfdca2d77ba80c9f021e8f`, 최초 PID **4623**, 고유 메모 `issue36-first-approach-571216f1`이다. [inspect](issue36-integrated/inspect.json)의 ok/instance/본문과 [절대 실행 경로](issue36-integrated/process.json)를 확인했다. [PID별 창 목록](issue36-integrated/windows.json)은 비어 있어 네이티브 창·표식/캡처 대조는 완료하지 못했다.
- 후속 원인 확인에서 CLI `open`은 ok:true이나 5초 뒤 visible:false/layout_pending:false였다. `npm ci` 후 `npm run dev -- --no-watch`로 자체 앱만 재시작한 PID **10994**에서도 새 메모 `issue36-runtime-check`·본문 `ISSUE36-RUNTIME-CHECK`가 같은 상태였다. [재시작 inspect](issue36-integrated/restarted-inspect.json)·[창 목록](issue36-integrated/restarted-windows.json)·[런타임 로그](issue36-integrated/runtime.log). lsof txt로 같은 절대 실행 파일을 확인했다. 단축키 표시는 권한/잠금 경고 없는 Ctrl+Alt+Cmd이며 권한 설정은 바꾸지 않았다. 비가시의 제품/환경 원인은 미확정이고 새 이슈를 생성하지 않은 채 감독자에게 근거를 보고했다.
- 감독자는 실패 원인 확인에 필요한 추가 재검증을 허용했으나 제품 최초 접근에 진입할 전제가 충족되지 않아 두 번째 스모크는 실행하지 않았다. 이전 전후 성공 기록을 통합 GUI 통과로 재사용하지 않는다. 통합의 정적/테스트 검증은 Conf 3, 해당 GUI는 미검증이며 Risk Medium으로 유지한다.
- [정리](issue36-integrated/cleanup.json): 자체 시작 PID 4623·10994만 종료했다. 하네스가 원래 포인터를 복원했고 검증 메모를 숨겼다. 다른 앱·워크트리 및 사용자 권한은 변경하지 않았다. 통합 커밋·push 후 최종 head CI는 별도 확인하며 GUI 차단이 남아 있는 동안 병합과 #36 닫기는 보류한다. #35·#44는 유지한다.
