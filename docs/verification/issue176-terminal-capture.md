# #176 터미널·캡처 본문 통일 검증

작업 루트: `/Users/egoinglee/.codex/worktrees/terminal-capture-spacing/agent-gibbon`  
브랜치: `codex/terminal-capture-spacing` · 기준 main `91b17516f02df817abf2a21a04894f3ddcd6aaec`  
계획 커밋 `f578fca` · [이슈 #176](https://github.com/egoing/agent-gibbon/issues/176) · [Draft PR #179](https://github.com/egoing/agent-gibbon/pull/179)

최종 제품 변경은 미커밋이며 별도 승인 대상이다. 아래 검사는 현재 작업 파일에 대한 것이며 원격 계획 커밋의 제품 검사 결과가 아니다.

## 구현과 판단

터미널·캡처 툴바를 22px로 맞추고 본문 상단 추가 여백을 없앴다. 터미널의 기존 3/5/5px 패딩은 상단 0, 좌우/하단 공통 3px로 변경했다. PTY 팔레트는 그대로 두고 그리드 아래의 한 셀 미만 영역도 같은 색으로 채운다. 기존 vendor `.xterm-viewport`의 기본 검정 배경이 남는 문제를 비교 캡처에서 확인하여 전용 CSS로 보정했다.

xterm 6의 FitAddon은 고정 scrollbar 열을 빼므로 overlay와 그대로 병용하면 불필요한 공간이 남는다. 공개 DOM의 실제 셀 폭·높이와 host의 내부 크기로 완전한 셀 수를 계산하고 공개 `resize`로 전달한다. forced-colors에서는 기존 FitAddon과 xterm 기본 scrollbar를 사용한다. xterm 내부 비공개 필드는 사용하지 않는다.

공통 overlay 표시 CSS와 geometry를 재사용하고 xterm 전용 adapter에서 공개 buffer·`scrollToLine`을 사용한다. alternate-screen은 thumb를 숨기며 TUI의 휠·마우스 보고·키 입력은 xterm이 계속 담당한다. [xterm 공개 API](https://xtermjs.org/docs/api/terminal/classes/terminal/#scrolltoline)를 참고했다.

native 터미널 builder에 `transparent(true)`를 추가했다. 이 설정은 창의 투명 WebView 배경을 허용하는 [Tauri 공식 API](https://docs.rs/tauri/latest/tauri/webview/struct.WebviewWindowBuilder.html#method.transparent)를 따른다. CSS 라운드 자체와 활성 2px 선은 기존 공통 구현을 유지한다.

캡처는 툴바만 22px로 바꾸고 `object-fit: contain`·본문 패딩 0·원본 로딩/복사/메모 이동 경로를 유지한다. 비율 차이에 따른 letterbox는 필요한 공간이므로 제거하지 않는다. 기존 제품에 별도 이미지 zoom UI가 있는 것으로 간주하지 않으며 브라우저 배율과 원본 크기 보존을 구분한다.

## 자동 검사

| 검사 | 결과 |
|---|---|
| `pnpm test:ui` | 204 통과 |
| `node scripts/build.mjs cargo test --manifest-path src-tauri/Cargo.toml` | 223 통과; 실제 셸 lifecycle 테스트 포함 |
| `pnpm check` | Clippy 통과 |
| `node scripts/build.mjs cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | 통과 |
| `pnpm dev:build` | 현재 Apple Silicon macOS debug 빌드 통과 |
| `scripts/terminal-surface-smoke.mjs` | 실제 xterm Chromium 18개 조합 통과 |

도구 버전: Node 26.0.0, pnpm 11.25.0, Rust 1.91.1, 아키텍처 arm64. pnpm 중앙 공유 캐시를 사용했으며 의존성 변경은 없다.

브라우저 실행 명령:

```sh
PLAYWRIGHT_MODULE=/Users/egoinglee/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/index.mjs \
SURFACE_OUTPUT=/tmp/gibbon176-evidence node scripts/terminal-surface-smoke.mjs
```

실제 제품 HTML·CSS·xterm을 Chromium에서 로드한다. Tauri IPC만 stub하며 실제 PTY·macOS native 입력 검증과 구분한다. 320×180·640×400·421×263, 회색·흰색·검정, device scale 1·2의 18개 조합이다. device scale은 OS/WebView zoom 조작의 대체 증거가 아니다.

- 180줄 출력과 normal buffer scrollback, Home/End, thumb 드래그·휠 스크롤
- alternate-screen 진입 시 thumb 숨김, 일반 화면 복귀 시 기록 보존
- TUI 방향키의 입력 전달과 mouse reporting 활성 시 휠 보고 전달
- 글꼴 13→14→16→18→13px에서 완전한 그리드 끝과 PTY resize 요청의 열/행 일치
- 좌우/하단 잔여 공간이 한 셀 미만이고 검정 기본 배경이 남지 않음
- PNG 검사: 좌상단 모서리 alpha 0, 그리드 아래 `(32,37,46,255)` 일치(Chromium에 한정)
- 이미지 fixture에 실제 capture CSS를 적용한 aspect/contain·툴바 밀착 검사

캡처 fixture는 터미널 페이지에서 공통 capture DOM/클래스와 원본 비율 이미지를 구성한 스타일 회귀다. 실제 `app.js` 원본 복사·드래그와 native 캡처 흐름을 통과했다고 주장하지 않는다.

[비교 PNG](assets/issue176/browser-comparison.png) · [18조합 수치 JSON](assets/issue176/browser-results.json). 단일 이미지에는 normal scrollback과 TUI, capture 세 테마의 대표 화면을 남겼다. 전체 18조합 원본 PNG는 `/tmp/gibbon176-evidence`에 있다.

## 개발 앱 출처와 native 상태

`pnpm dev:info`:

- root: 위 작업 루트
- instance: `dev-897f90bbf9068cfbb0f7ba67`
- binary: `/Users/egoinglee/.codex/worktrees/terminal-capture-spacing/agent-gibbon/src-tauri/target/debug/agent-gibbon`

개발 CLI로 전용 메모 `verify176-provenance`를 열고 `inspect`의 `ok:true`, 위 instance 일치, 고유 본문 `verify176 / dev-897f90bbf9068cfbb0f7ba67`을 확인했다. Unix socket 소유 PID 37638과 `lsof -a -p 37638 -d txt -Fn`으로 실제 바이너리 절대경로까지 일치했다. 테스트 후 메모를 숨겼다. 이 검사는 terminal/capture 화면 검증이 아니다.

CUA는 debug 실행 파일 경로에 `Invalid app`을 반환했고 초기 Orca는 runtime_unavailable이었다. computer-use 공식 가이드의 `orca open --json` 정상 경로로 runtime 연결을 확인했으며, 이후 사용자가 Orca 반복 실행을 문제 제기하여 팀장이 추가 실행·새 GUI 입력 중단을 지시했다. 후속 native 검증은 그 요청에 따라 보류했다. 설치 앱이나 다른 워크트리로 대체하지 않았다.

## 한계·승인 경계

- 실제 terminal native 창의 활성 라운드 투명도·물리 wheel/키/drag·native PTY resize 및 capture 원본 복사/이동/배율 조작은 사용자에게 방법·범위를 설명하고 결정할 후속 native 확인 대상이다. 이번 변경의 실제 terminal/capture native 창은 열거나 조작하지 않았다.
- Intel Mac·Windows·전체 플랫폼 CI·설치 파일·배포는 미실행이다.
- Conf 4: Chromium 실제 xterm 실행 검증. Risk Medium: 터미널 크기·출력 기록 상태에 영향이 있으나 국소 변경이며 복구 가능하다.
- 최종 제품 커밋·push·병합 승인 전이다. `package-lock.json`은 워크트리 자동 준비 과정에서 생긴 미추적 파일로 변경에 포함하지 않는다.

[디자인 시스템](../design-system.md#터미널캡처-본문-통일--176-2026-11-14) · [설명용 SVG](../design/terminal-capture-surface.svg) · [렌더 PNG](../design/terminal-capture-surface.png)

## 2026-11-15 사용자 검수 후속

사용자가 캡처 좌우의 여백과 원본 대비 크기·위치 변화를 제보했다. 종전 구현은 창 안에서 22px 툴바를 뺀 영역에 contain하여 원본을 축소했다. 최초 이미지 bounds를 그대로 두고 창만 높이 +22px/위치 y−22px(Windows는 위치만 배율 적용)로 늘렸다. 모니터 작업 영역 위쪽 여유가 부족하면 창 원점을 이미지 원점에 두고 toolbar를 하단으로 바꾼다. 크기 변경 뒤 원점을 다시 설정한다. 실제 사용자 캡처를 시험 목적으로 이동·변경하지 않았다.

표면의 레이아웃용 1px 투명 border는 캡처에만 제거한다. 기존 2px 활성 focus overlay는 유지하며 이미지 파일은 변경하지 않는다. native aspect 제약은 toolbar를 포함한 창 크기에 적용되고 이미지는 contain하므로, 이후 사용자의 창 resize에서 발생하는 비율 차이 여백은 남을 수 있다. 전체 화면 크기 캡처에서는 이미지 원점 보존과 toolbar 완전 노출을 동시에 보장하지 않는다.

Welcome에는 텍스트의 `installOverlayScrollbar`를 옵션으로 재사용하여 표시·입력·접근성 동작을 공유했다. PTY thumb와 별도 ID를 사용하며 Welcome 숨김 시 thumb도 숨긴다. 제목 변경 UI와 이벤트를 제거했다. 긴 명령 미리보기는 Welcome과 하나의 스크롤 영역을 사용한다.

- UI 204 통과, Rust 225 통과(캡처 위치·크기 및 상단/음수 모니터·배율 경계 단위 검사 추가).
- 실제 xterm Chromium 18조합 재검증 통과. 작은 Welcome의 native scrollbar 숨김·Home/End·PTY 전환 시 thumb 숨김, 제목 변경 UI 부재도 확인했다.
- 640×360 원본 비율 fixture를 640×382 창으로 표시했을 때 이미지가 x=0, y=22, 640×360이고 하단 toolbar 모드에서는 x=0, y=0, 640×360임을 device scale 1/2에서 확인했다. 원본 1600×900 fixture는 스타일 검증용이다.
- Welcome의 테마/크기 14조합 대비·설정 전환·선택/DOM 보존·키보드 초점·Run 동작 검사 통과.
- macOS arm64 debug 빌드·Clippy·Rustfmt 통과. 실행 명령은 위와 동일하며 출력은 `/tmp/gibbon176-followup-final`, 테마 검사는 `/tmp/gibbon176-welcome-themes`에 있다.
- 실제 새 native 캡처의 원본 픽셀/좌표 비교·메뉴바/혼합 배율에 따른 OS 최종 위치 보정·native Welcome thumb 물리 드래그·Windows 실행은 미검증이다. 단위/브라우저 검증을 이 범위의 실기기 검증으로 대체 보고하지 않는다.
- 팀장이 알린 최신 main `ea0531a`와의 통합 회귀는 아직 하지 않았다. 기존 미커밋 변경을 보존하며 제품 커밋·push 승인 대기를 유지한다.

[캡처 배치 시안](../design/capture-original-bounds.png) · [Welcome 시안](../design/terminal-welcome.png) · [원본 크기 fixture](assets/issue176/capture-exact-above-2x.png) · [하단 툴바 fixture](assets/issue176/capture-exact-below-2x.png) · [Welcome 실제 브라우저](assets/issue176/welcome-gray-2x.png)

## close 승인과 최신 main 통합 — 2026-11-15

사용자가 `close`를 호출하여 이번 범위의 커밋·push·병합과 이슈 기록/종료를 승인했다. 제품 커밋 `34c3b8a`로 변경을 보존하고 최신 main `6d1bf73`(편집기·터미널 복원 통합)을 반영했다. 공통 scrollbar는 편집기 변경 감시와 Welcome 설정을 함께 보존했고, Welcome 복원 패널/일반 패널 전환도 숨김 변경을 관찰하도록 통합했다. 원격 main의 제품 변경은 그대로 보존했다.

- 통합 UI **184개**, Rust **229개** 통과. 수량 차이는 최신 main의 편집기 테스트 구성 변경에 따른 것이며 기존 204/225 결과를 새 통합 결과로 사용하지 않았다.
- 실제 xterm 18조합, Welcome 테마 14조합, Milkdown 브라우저 전체 회귀 통과(`errors: []`, 외부 요청 없음). Milkdown은 편집·저장/자동저장·선택 서식·크기·긴 문서와 공통 스크롤바 회귀를 포함한다.
- 추가로 복원 계획 패널의 thumb 숨김 → 취소 → 일반 Welcome의 thumb 복구를 device scale 1/2에서 확인했다.
- Clippy·Rustfmt·macOS arm64 debug 빌드 통과. 전체 플랫폼 CI와 배포는 실행하지 않았다.
- 검증 실행 파일: `scripts/terminal-surface-smoke.mjs`, `scripts/terminal-theme-smoke.mjs`, `scripts/milkdown-browser.mjs`. 각 로그는 `/tmp/gibbon176-close-*`에 있으며 Milkdown 실행은 공식 설치 Chrome을 선택하는 임시 Playwright wrapper를 사용했다(저장소 제품 변경 없음).
- 이번 close에서 native GUI를 조작하거나 Orca를 실행하지 않았다. 현재 전용 개발 앱이 실행되지 않은 것을 CLI로 확인했다. 실제 사용자 창을 종료·이동하지 않았다.
- 원본 위치의 실기기 픽셀 대조·혼합 배율 OS 보정·물리 입력·Windows/Intel은 위와 같이 미검증이다. 사용자가 이러한 한계가 고지된 상태에서 마무리를 요청했으며 이 기록은 native 통과를 의미하지 않는다.
- `package-lock.json`은 작업/기본 체크아웃 모두에서 기존 미추적 파일로 보존하고 커밋하지 않는다. 이 체크아웃은 Codex 전용 워크트리이며 Orca 워크스페이스 삭제 대상이 아니다.

Conf **4**, Risk **Medium**. 병합 SHA·main 동기화 결과는 이슈 #176의 최종 기록에 남긴다.
