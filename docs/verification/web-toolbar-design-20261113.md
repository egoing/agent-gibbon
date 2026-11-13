# 브라우저 툴바 공통 디자인 — #152

## 출처와 상태

- root: `/Users/egoinglee/.codex/worktrees/e864/agent-gibbon`
- branch: `codex/web-toolbar-design`, 기준 main `9904f10770d552b8f90ceb8073415d9875c09062`
- HEAD: 계획 커밋 `6a108ab` (제품 변경은 아직 미커밋·미push)
- [이슈 #152](https://github.com/egoing/agent-gibbon/issues/152), [Draft PR #154](https://github.com/egoing/agent-gibbon/pull/154). 최종 커밋·push·병합은 승인 대기.
- 환경: macOS ARM64, Node v24.21.0, Cargo 1.91.1, Tauri 2.12.2 / Wry 0.57.0.
- 개발 ID: `dev-4052ce6137d624782cbabe46`
- binary: `/Users/egoinglee/.codex/worktrees/e864/agent-gibbon/src-tauri/target/debug/agent-gibbon`
- 마지막 검증 PID **71195**, 메모 **web-toolbar-verified-e864**, 네이티브 창 **68466** (아래 source JSON에 기록)(인스턴스 재시작마다 변경).
- 실제 제목: `Agent Gibbon [DEV codex/web-toolbar-design · 4052ce61] — 웹 — web-toolbar-verified-e864`. `scripts/agent-gibbon-dev inspect`의 `ok`, instance, id, URL과 PID 실행 경로를 대조했다. GUI는 팀리더의 단독 슬롯 승인 뒤 PID/정확한 제목으로 특정했다.

## 변경과 판단

웹 전용 CSS의 전역 버튼 규칙과 고정 어두운 팔레트를 제거했다. 폼 스타일은 Welcome에만 적용하고 툴바는 공통 CSS의 버튼 24×22px·아이콘12px·라운드3px·hover16%·키보드 outline을 그대로 사용한다. 행은30px이며 텍스트 읽기 화면의22px 예외를 확대하지 않았다. 터미널 appearance를 공통 `memo-surface-appearance.mjs`로 옮겨 두 표면이 같은 독립 메모 설정과 storage/OS 이벤트를 사용한다. 기존 terminal export는 호환 유지한다. 접힌 URL 변경 시 전체 제목 tooltip을 동기화한다.

핀은 기존 지원 기능이므로 유지하고 해제 불투명도도 공통 규칙을 따른다. 닫기는 현재 페이지를 유지하며 숨긴다. #119의 세션 종료는 구현하지 않았다. 외부 페이지 색상과 IPC 거부·HTTP(S) URL 정책·팝업/다운로드 차단은 변경하지 않았다.

실제 원격 페이지는 첫 검증에서 툴바를 덮었다. 보정 전 PID61983/window68421에서 AX chrome y235, toolbar 높이30, 외부 콘텐츠 y237이었다. 로컬 Tauri/Wry 소스에서 decorated host의 원래 view 좌표와 실제 content 영역 차이를 확인했다. 공식 [Window/add_child](https://docs.rs/tauri/2.12.2/tauri/window/struct.Window.html#method.add_child)와 [WebviewBuilder](https://docs.rs/tauri/2.12.2/tauri/webview/struct.WebviewBuilder.html)를 함께 확인했다.

`web.rs`는 macOS native frame/contentLayoutRect 차이를 매번 계산하여 child 위치와 가용 크기를 보정한다. 최초 부착 뒤 bounds를 재적용하며 resize 이벤트에서는 아직 바뀌지 않은 WebView 크기 대신 현재 native geometry를 읽는다. 보정 실패 시 이번 child를 닫고 새 host만 제거하며 기존 Welcome을 보존한다. 비macOS는 기존 논리 좌표30px 정책을 유지한다.

## 실행 검증

| 검사 | 결과 | 근거 |
|---|---|---|
| `npm run test:ui` | 188 통과 | [UI 로그](assets/web-toolbar-20261113/ui.log) |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 216 통과 | [Rust 로그](assets/web-toolbar-20261113/rust.log) |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 통과 | [Clippy 로그](assets/web-toolbar-20261113/clippy.log) |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` · `git diff --check` | 통과 | 로컬 실행 |
| `npm run dev:build` | 최종 제품 코드 빌드 통과 | [빌드 로그](assets/web-toolbar-20261113/build.log) |
| `scripts/toolbar-layout-smoke.mjs` | 메모·터미널·웹 × 144/300px 6조합 통과 | [로그](assets/web-toolbar-20261113/layout.log) |
| `scripts/web-toolbar-smoke.mjs` | 5테마 × 144/240/600px 15조합 통과 | [로그](assets/web-toolbar-20261113/web-browser.log) |
| SVG → PNG `rsvg-convert` | 렌더·육안 확인 | [시안](../design/web-welcome.png) |

브라우저 검사는 `PLAYWRIGHT_MODULE=/tmp/gibbon-toolbar-browser/node_modules/playwright/index.mjs`로 실행했다. 15조합은 공통 버튼 수치·접기 전후 가로 슬롯·핀 aria-pressed/해제 opacity·키보드 초점·hover·긴 제목 tooltip·Welcome 숨김/remote chrome 높이를 검사하며 터미널 버튼 수치와 직접 비교한다. 기존 smoke의 Tauri pointer/focus 이벤트 모의 객체 누락도 보완했다. 테스트의 외부 페이지는 모의이며 네이티브 child 검증과 구분한다. OS 시스템 모드 변경은 기존 appearance 단위검사로 확인했다.

실제 CLI는 `scripts/agent-gibbon-dev web/inspect/open/close`를 사용했다. 최종 PID71195에서 `https://example.com`을 열고 600×400 →320×240 →600×400 크기를 전환했다. AX 기준 toolbar y235에서 원격 콘텐츠 y265로 정확히30px 분리되고, 작은 창의 child도 폭320·높이182로 맞았다. [작은 창 AX](assets/web-toolbar-20261113/small-ax.txt), [600px 캡처](assets/web-toolbar-20261113/remote-600.png), [320px 캡처](assets/web-toolbar-20261113/remote-320.png), [복원 캡처](assets/web-toolbar-20261113/remote-restored.png)와 각 `*-source.json`에 창 출처를 남겼다.

기존 `scripts/design-system-ax.swift`로 정확한 제목의 버튼을 조작한 뒤 별도 inspect로 결과를 확인했다. 핀 해제(`always_on_top:false`), 접기·펼치기의 physical 높이48→800 복귀, 닫기 후 `visible:false`, CLI open 후 같은 URL·위치·핀 상태의 `visible:true`를 확인했다. [핀](assets/web-toolbar-20261113/pin-inspect.json) · [접기](assets/web-toolbar-20261113/fold-inspect.json) · [펼치기](assets/web-toolbar-20261113/unfold-inspect.json) · [닫기](assets/web-toolbar-20261113/close-inspect.json) · [복원](assets/web-toolbar-20261113/reopen-inspect.json).

## 한계와 정리

- 접힌 native decorated 창은 Orca가 on-screen 창으로 재선택하지 못했다. AX 정확한 제목을 이용한 펼치기와 상태 복귀는 확인했으나 **접힌 실제 화면의 시각적 확인은 미완료**다. 접힌 CSS 레이아웃·말줄임·tooltip은 Chromium에서 확인했다. [회색](assets/web-toolbar-20261113/browser-gray-collapsed.png) · [검정](assets/web-toolbar-20261113/browser-black-collapsed.png).
- 비활성 native hover·첫 물리 클릭·native 드래그·실제 OS tooltip·다중 모니터/scale 이동은 이번 실앱 검증에서 확인하지 않았다. 공통 pointer/fold 회귀검사는 통과했다. 새 공격 스크립트 실환경 검증은 하지 않았고 보안 정책 코드와 관련 기존 테스트를 유지했다.
- Intel·Windows·전체 플랫폼 CI·배포는 실행하지 않았다. 실패 rollback의 강제 오류 주입은 미실행이다.
- 기존 사용자 창/설치 앱/다른 작업 인스턴스는 변경·종료하지 않았다. 검증 창은 숨기고 최종 PID71195의 정확한 경로를 확인해 종료했다. 이전 재빌드 전 PID61983/66817/69211도 이번 작업에서 시작한 프로세스만 종료했다. GUI 슬롯을 해제했다.
- #150/PR #151의 `web.rs`는 create_presented 분리와 show 조건 변경이다. 이 작업의 helper 삽입·child/resize 보정과 의미상 독립적이나 create 선언 인접 문맥 충돌은 수동 병존해야 한다. `docs/design-system.md`의 양쪽 추가 절도 보존해야 한다. **#154를 먼저 병합하고 #151에서 최신 main을 반영**하는 순서를 제안한다. 다른 워크트리는 읽기만 했으며 제품/index/빌드/앱을 공유하지 않았다.

**Conf 4**: 자동 브라우저 검사와 macOS 실제 원격 레이아웃·CLI/AX 동작을 확인했으나 native 접힌 화면 등 위 범위는 미검증이다. **Risk Low**: 변경 영향은 웹/공통 appearance의 로컬 표시와 child 배치이며 쉽게 되돌릴 수 있다. 비macOS 실행 검증 한계는 남는다.

## 후속 사용자 요청: OS 제목줄 제거

위 기록은 제목줄을 유지했던 최초 검증이다. 이후 사용자가 시안의 네이티브 제목줄을 지목해 제거를 요청했으므로 **현재 최종 구현은 제목줄 없는 웹 창**이다. `decorations(false)`를 적용하고, 공통 툴바의 빈 곳/접힌 제목의 왼쪽 pointerdown은 기존 trusted `web_chrome` 명령으로 native 이동을 시작한다. 버튼·SVG 자식은 제외한다. 8방향 공통 resize 핸들을 추가하고 외부 콘텐츠는 좌우·하단3px를 비워 핸들 접근을 유지한다. resize 방향은 serde enum 역직렬화로 허용값만 받으며 외부 콘텐츠 IPC나 ACL 권한을 추가하지 않았다.

최종 출처: 동일 root/branch/instance/binary, **PID13703**, 메모 **web-borderless-e864**, native 창 **68633**, 제목 `Agent Gibbon [DEV codex/web-toolbar-design · 4052ce61] — 웹 — web-borderless-e864`. CLI inspect와 PID 실행 경로를 대조한 뒤 GUI를 선택했다. [600px 실제 화면](assets/web-toolbar-20261113/borderless-600.png) · [320px 실제 화면](assets/web-toolbar-20261113/borderless-320.png) · [600px 복원](assets/web-toolbar-20261113/borderless-restored.png), 각 source JSON에 PID/창 ID·크기를 기록했다. 제목줄/교통등 버튼이 없고 공통 툴바만 표시되며 외부 페이지가30px 아래에 배치되는 것을 육안 확인했다. AX로320×240 크기 변경 후600×400 복원과 접기/펼치기의 physical 높이48→800 복귀를 inspect로 확인했다.

| 최종 검사 | 결과 |
|---|---|
| UI 회귀 | **190 통과**, 빈 공간 드래그/버튼 제외·8방향 primary pointer 동작2개 추가 ([로그](assets/web-toolbar-20261113/borderless-ui.log)) |
| Chromium 웹15조합 | 통과 ([로그](assets/web-toolbar-20261113/borderless-browser.log)) |
| Rust | 첫 전체 검사215통과·PTY 크기 테스트1실패: expected100/actual80 ([원로그](assets/web-toolbar-20261113/borderless-rust.log)); 동일 테스트 단독 재실행 통과 ([재시도](assets/web-toolbar-20261113/borderless-rust-retry.log)), 이후 **전체216 통과** ([최종](assets/web-toolbar-20261113/borderless-rust-final.log)); 무관한 터미널 코드는 변경하지 않음 |
| Clippy / Rustfmt / diff check | 통과 ([Clippy](assets/web-toolbar-20261113/borderless-clippy.log)) |
| 개발 빌드 | 통과 ([로그](assets/web-toolbar-20261113/borderless-build.log)) |
| SVG/PNG | 제목줄을 제거한 시안을 다시 렌더하고 육안 확인 |

Orca background synthetic drag는 입력 전송이 unverified였고 inspect 위치도 바뀌지 않아 **실제 이동 성공으로 판정하지 않았다**. 물리 드래그/resize와 native 접힌 화면 시각 검증은 미완료다. 접힌 창은 CoreGraphics에서 PID13703/창68633/600×24/위치724,207을 확인했으나 Orca 선택과 직접 캡처가 실패했다. 사용자 포커스를 강제로 전환하지 않고 AX 정확한 제목으로 펼쳤다. 자동 이벤트 분리 검사와 실제 상태·화면 검증을 구분한다.

검증 창을 CLI close로 숨긴 후 정확한 실행 경로의 PID13703만 종료하고 GUI 슬롯을 해제했다. 기존 설치 앱/타 작업 창은 건드리지 않았다. 위 Intel·Windows·실제 외부 공격·다중 모니터 등 미검증 범위도 유지한다. **Conf4 / RiskLow**.

병합 기준은 그사이 #150/PR151이 병합되어 main `1f854b3f2ea1036da663c571b0905e259157cda7`로 바뀌었다. 현재 제품 변경을 미커밋으로 보존하고, **최종 커밋 승인 후 최신 main을 반영해 create_presented/show·session_restore와 병존 및 관련 회귀검증을 완료한 뒤 제출해야 한다**. 기본 main 체크아웃·다른 워크트리를 수정하거나 강제 reset/stash하지 않았다. 위 ‘#154 먼저 병합’ 제안은 이 실제 병합 상태로 대체한다. 최종 제품 커밋·push·병합 권한은 여전히 승인 대기다.

## 사용자 요청에 따른 새 실제 캡처

개발 CLI로 `web-live-e864-20261113`에 외부 페이지를 다시 열고 inspect의 instance/id/URL을 대조했다. PID24622의 정확한 e864 debug 경로와 창68659의 메모 ID 포함 제목을 확인한 뒤 Orca로 캡처했다. [새 실제 화면](assets/web-toolbar-20261113/live-request-600.png) · [창 출처](assets/web-toolbar-20261113/live-request-source.json) · [inspect](assets/web-toolbar-20261113/live-request-inspect.json). 검증창을 숨기고 이번 PID만 종료했다. 리로드·뒤로가기·주소 버튼→Welcome 입력 흐름은 사용자 요청대로 별도 [신규 이슈 #158](https://github.com/egoing/agent-gibbon/issues/158)에 등록했으며 현재 툴바 변경에 구현을 섞지 않았다.
