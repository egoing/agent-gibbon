# #190 모니터 환경 변경 — 구현·검증 기록

## 범위와 기준

- 작업 루트: `/Users/egoinglee/.codex/worktrees/4c75/agent-gibbon`
- 브랜치: `codex/20-monitor-layout-policy`
- 기준 HEAD: `307f7fde87874763e6c5c5009fd903ae2a6949c0`. 컬러 PR #189, 방향 정렬 PR #173, 툴바 확대 PR #180을 포함한다. 아래 결과는 해당 HEAD 위의 **미커밋 작업 변경**에 대한 결과다.
- Tauri lock 2.12.2 / CLI 2.12.1 / tao 0.37.1, Node 24.21.0 / pnpm 11.25.0 / Cargo 1.91.1, macOS ARM64.
- 개발 인스턴스: `dev-54e35761915b899a5349d485`
- 개발 바이너리: `/Users/egoinglee/.codex/worktrees/4c75/agent-gibbon/src-tauri/target/debug/agent-gibbon`

## 구현

`monitor_layout`은 현재 화면 목록·작업 영역·배율을 main thread에서 확인한다. 250ms 주기와 750ms 동일 구성 유지 후 보정은 초기 구현값이다. 화면이 없거나 조회가 실패하면 원래 배치를 덮지 않는다. 캡처·접어두기·등장/크기 측정 전환·마우스 누름·업데이트 동결 중에는 보정을 보류한다.

정상 창과 인접 화면 작업 영역의 합집합에 들어가는 창은 유지한다. 접근 불가 창은 작업 영역 안으로 이동하고 과대 창은 임시 축소한다. 이동한 창만 충돌을 검사하고 정상 창은 장애물로 둔다. 충돌 탐색은 축별 가까운 후보 48개로 제한해 UI thread의 검색량을 제한한다. 공간이 부족한 경우 `diagnostics.monitor_layout.crowded`에 표시하고 기존 창을 임의로 숨기거나 닫지 않는다.

원래 화면/위치/크기와 현재 임시 배치를 별도로 저장한다. 화면 이름·전체 경계·작업 영역·배율이 완전히 일치하는 화면이 단 하나일 때만 복귀한다. 이름만 같은 다른 화면으로 복귀하지 않는다. 동일 화면이어도 식별값·배치·배율이 달라졌다면 자동 복귀하지 못하는 한계가 있다.

직접 drag/resize·워프·방향 정렬은 사용자 기하 소유권으로 처리한다. 단순 drag 클릭만 하고 움직이지 않은 경우는 복귀 정보를 버리지 않는다. 방향 정렬의 5초 취소는 이전 모니터 복귀 관계도 되살린다. 툴바 확대 전 프레임·접힘 크기·배치 제외와 피난 원래 배치를 분리하며, 확대 상태는 환경 변경 후 새 작업 영역에 맞춘다. 피난 후 접었다 펼쳐도 원래 펼친 높이를 잃지 않도록 한다.

시작 journal에 선택 필드로 모니터 복귀 상태·확대 전 복원 정보·이미지 툴바 상하 위치·배치 제외 여부를 추가했다. 기존 snapshot은 기본값으로 읽으며 손상된 복귀 상태는 기존 journal을 보존하고 저장을 중지한다. 접힘·핀·최소화·숨김은 자체 기하 보정에서 변경하지 않고 visibility/focus API를 호출하지 않는다. 이미지 파일을 재작성하거나 텍스트 초안·PTY/WebView를 재생성하지 않는다.

Windows의 기존 OS 이벤트별 terminal geometry 저장/PTY resize 병합 개선은 #22의 별도 범위이며 완료했다고 주장하지 않는다. 웹 child bounds와 PTY fit은 기존 resize 경로를 유지한다.

## 실행한 검사

| 검사 | 결과 |
|---|---|
| `pnpm install --frozen-lockfile` | 잠금 파일 변경 없이 의존성 설치 |
| `node scripts/build.mjs cargo test --manifest-path src-tauri/Cargo.toml` | Rust **246개 통과**, 새 모니터 정책 12개 포함 |
| `pnpm run test:ui` | 기존 UI **189개 통과** |
| `pnpm run check` | Clippy all-targets `-D warnings` 통과 |
| `pnpm run dev:build` | 4c75 전용 debug 개발 빌드 통과 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` / `git diff --check` | 통과 |
| `rsvg-convert docs/design/monitor-layout.svg -o docs/design/monitor-layout.png` | 렌더 후 이미지 육안 확인 |

초기 Clippy 검사에서 불필요한 unit 표현, 추가 테스트의 불필요한 clone을 찾아 고친 뒤 재검사했다. 실패를 통과로 집계하지 않는다. 정적·기하 테스트는 실제 물리 모니터 연결 시험을 대신하지 않는다.

새 단위 검사는 정상/인접 화면 걸침 보존, OS가 먼저 옮긴 창의 원래 배치 유지, 분리 중 직접 이동·resize 우선, 임시 배치 직렬화·재시작, 작업 영역 축소 후 원래 크기 복귀, 이미지 비율, DPI별 크기 변환과 원점 유지, 충돌/용량 부족, 모호한 식별·유효하지 않은 상태 거부, undo/확대의 모니터 복귀 관계, 연속 이벤트 안정화, 피난 후 접힘/펼침을 다룬다. 기존 journal의 새로운 필드 기본값과 URL·접힘 데이터 유지도 검사한다.

## macOS 개발앱 CLI와 저장 fixture 확인

[출처·전후 상태 JSON](monitor-layout/native-results.json)을 확인한다. 실제 `dev:info` root/id/binary와 프로세스 실행 파일/PID, `inspect.ok/instance/id/본문`을 대조했다. 검증 메모는 `monitor20-20261115-check`, 본문은 `MONITOR20-NATIVE-20261115 검증 전용 메모`다. 실제 GUI 캡처·포인터·전역키 조작은 하지 않았다.

- 검증 전에 해당 인스턴스 프로세스와 데이터가 없음을 확인했다.
- 새 메모 표시와 컬러 소문자 정규화, 숨김·비포커스·위치/크기 checkpoint를 확인했다.
- **이 테스트 메모의 저장 fixture에만** 부재한 원래 화면/화면 밖 위치를 기록하고 같은 개발 바이너리로 시작했다. 실제 native 창이 작업 영역 안으로 보정되고 원래/임시 배치가 나뉘어 저장되며 숨김과 비포커스가 유지됨을 확인했다.
- 원래 화면 부재 상태로 다시 시작해도 원래 위치가 유지되는 것을 확인했다. 실제 현재 화면과 일치하는 home 정보를 fixture에 심어 시작했을 때 원위치 복귀와 임시 home 해제를 확인했다.
- 직접 시작한 동일 바이너리 PID `62723/63067/63280/63456`만 종료했고 해당 검증 종료 시 잔여 프로세스는 없었다. 설치앱·다른 워크트리·사용자 메모는 변경/종료하지 않았다.
- 최초 스크립트는 컬러 `#A1B2C3` 입력을 앱이 `#a1b2c3`로 정규화하는 기존 계약을 기대하지 못해 중단됐다. 제품 변경 없이 기대값을 바로잡아 재실행했다. [최초 중단 기록](monitor-layout/native-first-attempt.json)을 보존한다.
- 재사용 가능한 [`scripts/monitor-layout-native-smoke.py`](../../scripts/monitor-layout-native-smoke.py)는 현재 워크트리의 새 인스턴스 데이터만 허용하고 기존 프로세스/데이터가 있으면 거부한다. GUI 차례 조율 후 실행한다. 저장 fixture 시험이지 물리 화면 전환 시험이 아니다.

위 CLI 실행 후 최종 검토에서 배치 제외 필드 보존과 피난 후 접힘 크기 보존을 보강했다. 그 보강은 최종 Rust246/Clippy/build에 포함되며, 아래 사용자 준비 세트는 최종 빌드를 사용한다. 앞선 저장 fixture 결과를 모든 최종 변경의 실환경 통과로 확대하지 않는다.

## 준비 중 발견한 시작 포커스 문제

사용자 준비 세트의 visible 복원 시 C의 `focused=true`를 관측했고 `NSWorkspace.frontmostApplication`도 실제 해당 Agent Gibbon PID로 바뀌어 준비 완료로 처리하지 않고 중단했다. 처음에는 정상 창의 불필요한 `unminimize()` 호출과 시작 시점의 영향을 점검했다. 최소화 상태가 실제로 다를 때만 전환하고 `RunEvent::Ready`에서 포커스를 마무리하도록 보강했지만, 이것만으로는 해결되지 않아 추가 조사했다.

직접 원인은 CLI `--serve` 경로가 `initial_open=false`여서 `Targets::initial()` 대신 비어 있는 기본값을 사용한 것이었다. 시작 복원 때 돌려줄 foreground 대상이 애초에 없었다. 모든 서버 시작에서 Builder/런타임 이전에 초기 대상을 캡처하고, Tao의 launch 활성화 및 Tauri setup이 끝난 Ready에서 한 번 소비하도록 변경했다. `focus::restore`의 ‘이미 사용자 제3앱으로 전환했다면 건드리지 않음’ guard는 그대로다. 실패한 시도를 통과로 집계하지 않는다.

설치된 tao 0.37.1 소스의 `is_focused()`는 AppKit `isKeyWindow()`를 사용한다. [Apple 문서](https://developer.apple.com/documentation/appkit/nswindow/iskeywindow)는 이를 해당 앱의 key window로 정의하므로, 이 값만으로 실제 다른 앱 포커스 보존을 판정하지 않고 활성 앱 PID도 기록한다. 재준비의 시작 전·직후·안정 후 PID는 아래 JSON을 따른다.

포커스 회귀 중 CLI 시작 전 다른 앱 보존은 실제 read-only 관측으로 확인한다. 시작 도중 제3앱 선택 guard와 정상 사용자 메모 클릭은 코드를 검토하되 사용자의 앱을 자동 전환해 시험하지 않는다. 실제 제3앱 선택·메모 클릭·최소화 GUI는 미검증이다. snapshot의 최소화 상태가 현재 native 상태와 다를 때만 minimize/unminimize하도록 조건을 검토했다.

## 사용자 실검증 준비

[단계별 시나리오](monitor-layout-manual-scenarios.md)와 [준비 세트·출처](monitor-layout/manual-prepared.json)를 따른다. 최종 빌드의 새 검증 메모 A/B/C와 숨김 H만 배치하고 앱을 유지한다. 유지 중인 PID는 `86848`이며, 실제 바이너리 경로와 CLI instance를 다시 대조했다. A는 LG HDR 4K, B/C는 V28UE-M의 읽기 조회된 경계와 Tauri 경계를 대조해 배치했다. H는 숨김이다. 시작 전 foreground는 `20087`(ChatGPT), 프로세스 직후는 `86848`(agent-gibbon), 안정 후는 다시 `20087`(ChatGPT)였다. 즉 초기의 일시 활성화는 남아 있으며 안정 후 원래 앱으로 복귀하는 것을 확인한 범위다. 시작 중 전혀 활성화되지 않는 것으로 보고하지 않는다. 사용자가 직접 움직인 이후의 최신 상태를 기준으로 삼고 초기 배치로 덮지 않는다.

실제 모니터 설정·연결은 에이전트가 바꾸지 않았다. 사용자의 물리 분리/재연결·해상도/배율 변경 결과는 아직 미확인이다. 사용자 확인을 받아 단계별 기록을 추가한다.

## 미검증과 판정

- 실제 연결·분리 중 OS 이벤트 순서, 혼합 DPI, 화면 회전·주 모니터 변경·도킹·덮개·잠자기 복귀.
- 실제 IME/초안·선택·핀·최소화, 확대 원복·방향 정렬 취소의 물리 GUI 통합.
- 이미지·살아 있는 PTY·외부 웹·관계선의 환경 변경 실환경 통합.
- Windows/Intel·다른 Tauri 플랫폼 실행. 전체 플랫폼 CI·설치·배포 미실행.
- 모든 창의 조작 영역 동시 노출이 물리적으로 불가능한 작은 화면/과밀 상태에서는 현재 안전 배치와 기존 개별 접근을 유지한다. 메모 관리는 현재 웹 창을 포함하지 않으므로 웹은 트레이/CLI 개별 열기를 사용한다. 별도 복구 UI를 검증했다고 주장하지 않는다.

전체 **Conf 3 / Risk Medium**. macOS 저장 fixture와 CLI 확인 범위만 실제 실행 근거가 있다. 사용자 물리 시험 전 이슈 완료·최종 제품 커밋·병합·배포를 수행하지 않는다.

## 사용자 실제 시험 — 21:00~21:01 KST

사용자가 V28UE-M 분리를 수행한 뒤 동일 PID53980/binary/instance와 각 고유본문을 대조했다. 화면3→2, pending=false/error=null/crowded=false였고 A 위치·크기는 불변, B/C는 남은 작업 영역으로 이동하며 크기를 유지했고 H는 숨김을 유지했다. 남은 화면의 논리 해상도도 함께 바뀐 상태였으므로 연결 여부만 단독 변수인 시험으로 해석하지 않는다. [분리 직후 기록](monitor-layout/manual-after-disconnect.json).

다음 사용자 done 후 조회에서는 이미 화면3대로 재연결되어 있었다. B는 원래 물리위치(200,-1960)·680×420 크기로 복귀했고 A는 유지, H는 숨김이었다. C는 분리 중 사용자가 정한 위치(3118,200)에 유지됐고, 저장된 복귀 관계도 원래 외부 화면이 아닌 사용자 최신 배치 쪽으로 바뀌었다. C 크기는720×460으로 이전과 같아 **직접 이동 우선은 확인했지만 직접 resize 우선 시험은 아직 미완료**다. [사용자 조작·재연결 후 기록](monitor-layout/manual-after-user-change.json).

그 전에 프로세스가 종료된 시도는 사용자가 자신의 실수로 정정했으며 모니터 보정 충돌로 판정하지 않는다. 재시작 후 화면3개와 기존 배치를 확인한 [재시도 기준](monitor-layout/manual-retry.json)에서 위 실제 시험을 진행했다.

## 검토 제출 상태

계획 문서만 선별한 커밋 `60e66ccf04e98ef90c8d2f118e0c7b556037d4e4`를 push하고 [Draft PR #193](https://github.com/egoing/agent-gibbon/pull/193)을 준비했다. PR의 실제 파일은 `docs/plans/monitor-environment-layout.md` 하나이며 제품 파일은 포함하지 않는다. 제품 검사/바이너리는 계획 커밋 전 기준이나 현재 Rust 소스·바이너리 해시가 실제 준비 기록과 일치함을 재확인했다. [검토 제출·완료 기준별 남은 일](monitor-layout-submission.md)을 따른다.

마지막 프로세스 조회에서는 해당 개발앱이 실행 중이지 않았다. 에이전트가 종료하거나 사용자 최신 기하를 초기화하지 않았으며 자동 재시작도 하지 않았다. 사용자 최종 성공 보고와 inspect 기록에서 C의 마지막 위치 변경은 확인했지만 크기는 여전히720×460이었다.

## 사용자 병합 승인과 최종 반영

팀장 채팅에서 사용자가 `20 병합승인`을 명시했다. 이에 따라 PR193을 계획 전용 Draft에서 최종 제품 범위로 갱신하고 제품 커밋·push·병합 및 기본 체크아웃 main fast-forward 동기화를 진행한다. 공개 배포와 추가 실환경 시험은 승인 범위에 포함하지 않는다. 앞선 승인 대기/미커밋 표기는 그 단계의 기록이다.

최신 origin/main은 실행 검증 기준 `307f7fde87874763e6c5c5009fd903ae2a6949c0`과 같았다. Rust 소스 SHA256 `4e0e569a791a97172c74adb77b359e5a8da0a6ef4ba7b50efb442352ffa46653`, 검증 debug 바이너리 SHA256 `faf4518a7188891eebe5993141ad986af98ccad70226cbfbd5df798a6e08687e`가 준비 기록과 현재 파일에서 일치하므로 동일 검사를 반복하지 않았다. 최종 문서의 상태 표기만 승인에 맞춰 갱신했다.

실제 미완료 검증을 완료로 바꾸지 않는다. C 실제 resize 우선·큰 창 축소·혼합 DPI/Windows/Intel·유형/상태별 전환·실제 임시 피난 중 재시작 및 초기 일시 활성화의 한계가 남아 있어 이슈190은 열린 상태로 유지한다. 다른 이슈로 미완료 기준을 옮겨서 닫지 않는다. 현재 사용자 테스트 메모·프로세스와 다른 워크트리 상태를 조작하지 않는다.
