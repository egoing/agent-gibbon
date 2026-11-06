# 수동 투명도 롤오버 복원과 조절 휠 전파 차단 — 2026-11-06

## 원인과 변경

- 이 동작을 복원하는 미병합 작업은 찾지 못했다. 열린 PR은 없었고, 원격 브랜치·전체 이력·#71 기록을 확인했다. #71 구현은 이미 main에 통합되어 있었다. 기존 구현과 문서가 **수동 설정을 hover보다 우선**하여 휠을 한 번 사용하면 포인터 접촉에도 저장한 투명도를 유지했다.
- `sync_opacity`의 수동값 조기 반환을 제거하고, 포인터 접촉 → 저장된 수동값 → 기존 자동 알림/접힘의 순서로 목표값을 결정한다. 메모와 연결된 Feedback은 같은 alpha를 적용한다.
- 접촉 시 즉시 100%, 이탈 후 기존의 2초 대기·600ms ease-in-out으로 저장값에 복귀한다. 재접촉하면 전환을 즉시 중단한다. 휠로 값을 변경할 때는 이전 fade를 버리고 새 값에서 시작하여 즉시 조절하는 동작을 보존한다.
- README·[디자인 시스템](../design-system.md#전체-메모-투명도--71-2026-11-06)과 설명용 [SVG](../design/global-opacity-wheel.svg)·[PNG](../design/global-opacity-wheel.png)를 갱신했다.

## 작업 출처

- 루트: `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/hover-opacity-restore`
- 브랜치: `hover-opacity-restore`, 기준 커밋 `59d817e40e586b2e27dd56723db82194b4061c5c`.
- Orca ID: `da5d2674-a5c4-4f3d-99cb-5f69bdbabe9c::/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/hover-opacity-restore`.
- `npm run dev:info`: instance `dev-edd1ff2d021a706a8df75acd`, binary `<작업 루트>/src-tauri/target/debug/agent-gibbon`.
- macOS ARM64, Node v24.21.0, npm 11.19.0, rustc/cargo 1.91.1, rsvg-convert 2.62.3.
- 마지막 원격 조회 당시 main은 `9f015a6b9198aae3c7765fdf53988d330aafc8c0`이었다. 기준 커밋 이후 main에 추가된 CI 변경은 이 작업의 Rust 파일을 변경하지 않았다. 사용자가 커밋·병합·push·이슈 반영을 승인한 뒤 조절 휠 전파 차단을 추가 요청했다. 사용자 개발 앱은 교체하지 않았다.

## 실행 검증

| 명령 | 결과 |
|---|---|
| `cargo test --locked --manifest-path src-tauri/Cargo.toml` | Rust 153개 통과, 새 회귀 2개 포함. 롤오버 복원 소스에서 확인; 추가 휠 차단 후 155개로 재확인 |
| `npm run test:ui` | Node 95개 통과 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | 통과 |
| `cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 통과 |
| `npm run dev:build` | 이 워크트리의 debug 앱 빌드 통과 |
| `python3 scripts/opacity-hover-smoke.py` | 최종 빌드의 네이티브 검증 통과 |
| `python3 -m py_compile scripts/opacity-hover-smoke.py` | 통과 |
| `rsvg-convert docs/design/global-opacity-wheel.svg -o docs/design/global-opacity-wheel.png` | 렌더 통과, PNG 시각 확인 |
| SVG XML 파싱·`git diff --check` | 통과 |

새 Rust 회귀 검사는 수동 10/35/80/100%, 접힘 여부, hover 진입·지연 복귀, 알림보다 저장값 우선, 다른 메모와의 독립성, 새 휠 값의 즉시 적용을 다룬다. 기존 fade 시간·설정 저장·입력 단위 테스트도 유지한다.

### 실제 창 확인

`opacity-hover-smoke.py`는 이 워크트리의 개발 CLI만 사용하고, 실행 중인 동일 인스턴스가 있으면 중단한다. 새 개발 인스턴스의 투명도 파일만 임시로 35%로 설정한다. 실제 약관·전송 동의 파일은 변경하지 않는다. 원래 포인터와 설정 파일을 복원하고, 자신이 시작한 정확한 서버만 종료한다.

- 최종 실행 PID **96663**. `ps`의 정확한 절대 실행 명령과 `lsof -a -p 96663 -d txt -Fn`의 실행 파일 경로를 대조했다.
- 검증 메모 `hover-opacity-fd3a8f30-a`, `hover-opacity-fd3a8f30-b`. 모든 CLI 응답의 `ok:true`, instance·메모 ID·초기 고유 본문을 확인했다.
- CoreGraphics의 **동일 PID** 창에서 `Agent Gibbon [DEV hover-opacity-restore · edd1ff2d] — <메모 ID>` 제목으로 대상 창을 특정했다. 메모 A 창 **40386**, B 창 **40388**, A의 Feedback 창 **40393**을 기록했다.
- 네이티브 alpha를 **CLI inspect보다 먼저** 읽었다. 두 메모 35% → A 접촉 시 A만 100% → 이탈 직후 100% 유지 → 지연 후 A 35% 복귀를 확인했다.
- A의 본문을 바꿔 Feedback을 표시한 뒤, 본문 창 밖의 Feedback에만 포인터를 올려 A와 Feedback이 함께 100%가 되고 B는 35%를 유지하는 것을 확인했다. 이탈 중 재접촉으로 fade가 중단되고 최종 이탈 후 둘 다 35%가 됐다.
- 포인터 접촉 전후 저장 파일 바이트·전면 앱·창 위치·크기·포커스·고정·표시 상태를 대조했다. 콘텐츠 갱신 자체의 자동 크기 변화와 hover에 따른 변화를 분리해 비교했다. 숨긴 peer의 갱신도 숨김을 유지했다.
- [최종 실행 근거](hover-opacity/evidence.json), [A 접촉 시 창 단독 캡처](hover-opacity/hovered.png). 창 단독 캡처는 데스크톱 합성 투명도 측정용이 아니며 실제 alpha 근거는 CoreGraphics 값이다. 캡처를 시각 확인했다.
- 최종 실행 후 테스트 서버 종료와 설정 파일 복원을 확인했다. 사용자 main 앱·설치 앱·다른 워크트리 프로세스를 종료하지 않았다.

### 최초 시도와 검증 배치 보정

최초 실행은 일반 메모의 35→100→35까지 통과했으나, A 하단의 Feedback이 자동 배치된 B 메모와 겹쳐 Feedback 접촉 시 B도 100%가 됐다는 assertion에서 실패했다. 같은 포인터가 두 창의 사각형 안에 동시에 있었다. [최초 시도 기록](hover-opacity/first-attempt.json)의 실패 상태와 종료·설정 복원 결과를 보존했다.

테스트 전용 B 창만 접근성 API로 떨어뜨려 놓고 같은 35% assertion을 유지하여 재검증했다. 최초 시도의 실패를 통과로 표시하지 않았고 제품 코드를 이 assertion에 맞춰 완화하지 않았다. 최종 실행은 수정된 배치에서 모두 통과했다.

## 조절 휠 전파 차단 — 추가 요청

- 병합 전 사용자 요청으로 투명도 조절에 쓰는 휠이 다른 앱을 스크롤하지 않도록 한다. 일반 휠·가로 휠·다른 키/클릭 혼합은 전달하고, 한 칸 미만의 정밀 입력과 10/100% 한계에서도 조절 입력은 소비한다.
- macOS AppKit global monitor는 이벤트를 취소할 수 없어 scroll-only active session event tap을 별도 thread에 설치한다. 기존 키/클릭/보조키 관찰은 유지하고 휠만 tap에서 한 번 처리한다. 설정 저장과 창 변경은 main thread로 전달한다. Windows는 기존 저수준 mouse hook에서 처리한 휠에 한해 nonzero를 반환한다.
- macOS에서 조절한 트랙패드 제스처의 종료·관성 꼬리는 보조키를 놓은 뒤에도 소비하며 관성으로 설정을 추가 변경하지 않는다. 다음 새 제스처의 일반 스크롤은 전달한다. 입력 담당권·설치 성공이 전제이며 권한 실패나 담당권 미확보 시 입력을 차단하지 않는다.
- 추가 회귀: 입력 소유권·정밀 분수·일반 입력 전달, 조절 제스처의 관성 꼬리와 다음 제스처 복구를 검사했다. Rust **155개**·UI **95개**, Rustfmt·Clippy·개발 빌드가 통과했다.
- `python3 scripts/opacity-scroll-smoke.py` 첫 시도는 기존 main 개발 앱의 입력 점유를 감지하여 입력 합성 전 중단했다. [실패 근거](hover-opacity/scroll-input-unavailable.json)에 `passed:false`, 테스트 서버 종료·설정 복원 성공을 보존했다. 기존 앱은 종료하지 않았다. 이후 사용자가 main 앱을 직접 종료한 뒤 아래 실환경 검증을 완료했다.
- `python3 scripts/opacity-scroll-smoke.py` 재실행은 통과했다. 개발 instance `dev-edd1ff2d021a706a8df75acd`, 메모 `opacity-scroll-f6030b92`, 서버 PID **5771**의 정확한 실행 파일을 lsof로 대조하고 CLI `inspect`의 instance·메모·입력 담당권을 확인했다. 별도 수신 앱 PID **5889**의 `Agent Gibbon separate scroll receiver` 창을 같은 PID의 CoreGraphics 창 정보로 특정했다.
- 별도 앱의 실제 NSScrollView에서 일반 세로 휠은 수신 횟수 **0→1**, 스크롤 위치 **y 2000→1970**으로 전달됐다. 조절 휠은 공통 불투명도 **35→40%**로 변경하면서 수신 횟수·위치를 유지했다. Shift·가로 휠은 전달되어 수신 횟수 **1→3**이 됐다.
- 5픽셀 정밀 입력은 한 칸 미만으로 설정값을 유지하며 차단했고, 이어지는 15픽셀은 누적 한 칸으로 **40→45%**를 적용하며 차단했다. 보조키 해제 후 동일 제스처 종료·관성 시작·관성 종료도 수신 횟수 **3**, 위치 **x 960 / y 1970**, 저장값 **45%**를 유지했다. 이후 상한 **100%**에서도 계속 차단했고, 일반 스크롤 재개 시 수신 횟수 **3→4**, **y 1970→1940**으로 복구됐다. 전체 접어두기는 실행되지 않았다.
- [종단 검증 근거](hover-opacity/scroll-evidence.json)는 `passed:true`, 테스트 서버 종료·원래 설정 복원 성공을 기록한다. 원래 포인터·전면 앱을 보존했고 실제 사용자 메모·약관·진단 동의는 바꾸지 않았다. 이 검증은 OS를 통해 게시한 합성 휠 이벤트와 별도 네이티브 수신 앱을 사용했으며 물리 장치별 검증을 대신하지 않는다.
- 공식 API: [Apple 전역 모니터](https://developer.apple.com/documentation/appkit/nsevent/addglobalmonitorforevents(matching:handler:)), [Apple event tap callback](https://developer.apple.com/documentation/coregraphics/cgeventtapcallback), [Windows LowLevelMouseProc](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelmouseproc).

## 한계와 남은 작업

- 실제 macOS ARM64의 일반 텍스트 메모와 연결된 Feedback을 검증했다. Windows·Intel, 실제 이미지·접힌 메모·터미널·전체 접어두기와의 조합은 이번 GUI 검사에서 실행하지 않았다.
- 롤오버 검증 당시에는 main 앱이 전역 입력을 소유하여 합성 보조키+휠을 보내지 않았다. 이후 main 종료 후 별도 수신 앱으로 합성 휠 차단 종단 검증을 수행했다. 물리 휠의 장치별 양방향·하한 전체 경로와 hover 중 휠 조절 조합은 이번 실제 입력 검사에서 실행하지 않았다.
- 창 겹침 시 가려진 창의 hover 판정은 기존 사각형 기반 감지 규칙을 유지했다. 이번 작업은 포인터 접촉과 수동값의 우선순위를 복원하는 범위다.
- 커밋·병합·push는 사용자 승인 범위다. 최신 main 통합 후 전역 입력·OS별 코드 변경을 이유로 대상 SHA의 세 플랫폼 CI를 요청하며, 실제 결과와 원격 반영은 PR·이슈에 기록한다.
- **Conf 5 (위 macOS 네이티브·합성 입력 종단 검증 범위), Risk Medium.** 미실행 플랫폼과 GUI 조합을 통과로 간주하지 않는다.

## 통합과 원격 검사

- 사용자 승인으로 구현 커밋 `afd9eeb`을 만들고 최신 main `9f015a6`을 통합한 코드 검증 대상은 `5d9f553f649d1948837cbf48a4ecb3d07c99e9f2`다. [PR #75](https://github.com/egoing/agent-gibbon/pull/75), 관련 이슈 [#71](https://github.com/egoing/agent-gibbon/issues/71).
- 전역 입력·OS별 창 동작 변경을 이유로 `git push -u origin hover-opacity-restore` 후 `gh workflow run release.yml --ref hover-opacity-restore`를 실행했다. [실행 42202264919](https://github.com/egoing/agent-gibbon/actions/runs/42202264919)의 `headSha`가 위 대상과 일치함을 확인했다. 코드 검증 대상 SHA에서 macOS ARM64·Intel·Windows x64 모두 Rust/UI 테스트·Rustfmt·Clippy·설치 파일 빌드가 성공했다. `installer-macos-arm64`, `installer-macos-x64`, `installer-windows-x64`의 만료되지 않은 비어 있지 않은 artifact 세 개를 확인했다. `publish`는 수동 실행 정책대로 skipped이며 공개 릴리스는 생성하지 않았다.
- 합성 휠 검증 후 후속 변경은 README 문구 보정·검증 문서·실행 근거만 포함한다. 제품 코드(`src-tauri/`)와 검증 스크립트는 원격 검사를 통과한 SHA와 같음을 diff로 확인했다. 이 문서 후속 커밋은 JSON 근거·문서 일치성·diff 검사로 검증하며 별도의 전체 플랫폼 실행을 요청하지 않는다. 이전 SHA의 CI를 문서 후속 SHA에서 실행한 것으로 기록하지 않는다. 태그·공개 릴리스·설치 앱 교체는 수행하지 않는다.
