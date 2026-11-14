# 피드백 바에서 메모 크기 조절 가능성 조사

조회일: 2026-11-14 KST. 관련: [#160](https://github.com/egoing/agent-gibbon/issues/160), [계획 Draft PR #161](https://github.com/egoing/agent-gibbon/pull/161).

> 후속 결정: 사용자가 Feedback을 메모 내부 하단에 통합하고 Feedback 표시/숨김 자체로 창 크기를 바꾸지 않는 방향을 승인했다. 아래는 그 결정 전 조사 기록이며 권장안 A는 채택되지 않았다. 현재 구현·검증은 [후속 기록](../verification/feedback-inline-20261114.md)을 따른다.

## 결론과 요구 해석

**적용 가능한 설계 경로는 있다. 권장 동작은 피드백 자체를 키우는 대신, 피드백의 노출된 외곽을 끌어 연결된 메모 크기를 조절하는 것이다.** 다만 현재 macOS 의존성에서는 부모의 native resize 시작 API로 단순 전달하는 경로가 지원되지 않는다. 전용 입력 추적과 부모 크기·위치 갱신 또는 창 구조 변경이 필요하며 이번 조사는 실동작을 입증하지 않았다.

사용자 원문은 “피드백 바 부분은 마우스 포인트로 사이즈를 키우는 기능이 적용이 안되고 잇는데 적용이 가능한지 조사해줘.”이다. 제공 이미지에는 툴바 아래 좁은 편집 본문과 그 아래 저장·내보내기·취소 버튼 바가 보인다. 버튼 바의 글자/버튼을 크게 만들기보다 그 아래 외곽에서 **편집 공간을 늘리려는 의도**로 해석한다. 이는 권장 해석이지 사용자 확정 규칙은 아니다. 6번 편집 높이 작업과는 별도이며 작은 편집 높이의 원인 해결을 이번 조사에 섞지 않는다.

| 의미 | 조절 대상 | 효과와 적합성 |
|---|---|---|
| 전체 메모 조절(권장) | 부모 메모의 너비·높이·필요한 위치 | 편집 공간이 늘고 Feedback은 메모 너비와 자동 높이를 따른다. 통합 외곽선과 일관된다. |
| Feedback 자체 높이 조절 | 안내/버튼 영역만 | 긴 오류의 표시 공간은 늘지만 편집 본문은 커지지 않는다. 현재 제보 이미지의 개선 효과가 작다. 별도 요구일 때 검토한다. |

## 조사 기준과 현재 구현

- 작업 루트 `/Users/egoinglee/.codex/worktrees/6eec/agent-gibbon`, 브랜치 `codex/feedback-resize-research`.
- 조사 기준 origin/main `53ab3a710fb54f4702de2499fdc22b9dd8506087`; 계획 커밋 `eb404cfd3acfb8cb59ebd13e18ab8b5b9554b28b`.
- `npm run dev:info`: instance `dev-8d425074f682041f2185812f`, binary `/Users/egoinglee/.codex/worktrees/6eec/agent-gibbon/src-tauri/target/debug/agent-gibbon`.
- 실행 호스트 `arm64`, Node v24.21.0, npm 11.19.0, rustc 1.91.1, gh 2.89.0. Cargo.lock과 로컬 registry source: Tauri 2.12.2, tauri-runtime-wry 2.12.1, Tao 0.37.1, Wry 0.57.0.

위 두 항목의 의미: 개발 정보는 설정상 경로이며 실행 중인 서버를 확인한 증거가 아니다. 앱·서버는 실행하지 않았다.

| 근거 파일 | 확인한 사실 | 영향 |
|---|---|---|
| `src-tauri/src/feedback_window.rs` | 독립 WebviewWindow, `parent(&window)`, `resizable(false)`, 최초 비표시·비포커스 | 부모와 별개 native 창이다. 테두리만 연결되어도 OS resize 영역이 합쳐지지 않는다. |
| `ui/feedback.js`, `ui/feedback.css` | resize grip 없음; 좌클릭 안내/빈 공간은 `drag_feedback`; 버튼·링크·입력 제외, 텍스트 선택 금지 | resize hit-test를 추가하면 기존 이동 경로보다 우선해야 한다. |
| 같은 JS와 `layout_feedback` | refresh 완료 뒤 100ms 타이머로 snapshot/내용 측정/layout; 너비는 부모 outer width, 높이는 DOM 측정값 | Feedback만 resize 가능하게 바꾸면 다음 측정이 사용자 크기를 덮어쓴다. 정확한 주기는 IPC/측정 시간에 따라 100ms보다 길다. |
| `geometry`, `sync` | 아래 우선·불가하면 위; 양쪽 부족 시 큰 여유 구간, 끝내 화면 안 fallback; 부모 move/resize에서도 동기화 | 드래그 중 부착 방향 재선택이나 fallback 전환이 기준점을 바꿀 수 있다. |
| `drag_feedback`, `entrance::drag_note` | 호출 창 label에서 owner를 찾고 부모의 이동 시작 API 호출; 접힘·숨김 검사 | 현재 소스에 `drag_owner` 이름은 없다. 해당 역할은 이 두 함수이며 이동 성공을 resize 성공의 근거로 쓸 수 없다. |
| `ui/app.js`, `sizing.rs` | 본문 grip에서 manual resize와 native resize 시작; 자동 배치 제외·입장 애니메이션 취소. macOS live resize/Windows WM_SIZING으로 수동 의도 감지 | custom resize는 programmatic 갱신이므로 기존 native 감지에만 의존하면 수동 의도를 놓친다. |
| `src-tauri/capabilities/main.json` | resize 시작 권한은 main/note/agent/terminal에 있고 feedback-* 제외 | JS 창 label만 바꿔 범용 API를 호출하는 접근은 현재 권한 경계를 통과하지 않는다. |

[디자인 시스템](../design-system.md), [Feedback SVG](../design/feedback-drag.svg)·[PNG](../design/feedback-drag.png), [활성 테두리 SVG](../design/memo-focus.svg)·[PNG](../design/memo-focus.png), 툴바 SVG를 읽고 관련 PNG를 확인했다. 최신 확정값은 3px 패딩·좌우 3px 구분선 여백, 활성 2px 통합 외곽선이다. Feedback 시안의 하단 설명에는 이전 2px 문구가 남아 있어 최신 문서가 우선한다. 이 기존 불일치를 이번 조사에서 수정하지 않았다.

## 공식 API와 설치 소스가 보여 주는 한계

1. [Tauri JS resize 시작](https://v2.tauri.app/reference/javascript/api/namespacewindow/#startresizedragging)은 방향과 별도 권한을 요구한다. [Tauri Rust Window](https://docs.rs/tauri/2.12.2/tauri/window/struct.Window.html)는 프로그램에 의한 크기·위치 변경과 native drag 시작을 구분한다. 부모 객체의 메서드를 호출할 수 있다는 사실은 다른 창의 클릭 제스처를 넘겨도 OS가 받아 준다는 보장이 아니다.
2. [Tao 0.37.1 문서](https://docs.rs/tao/0.37.1/tao/window/struct.Window.html#method.drag_resize_window)는 직전 왼쪽 버튼 입력 없이 성공을 보장하지 않으며 macOS에서 항상 NotSupported라고 명시한다. 설치 소스 `tao-0.37.1/src/platform_impl/macos/window.rs:961`도 그대로 오류를 반환한다. 메모 본문 가장자리의 기존 macOS resize는 native border가 먼저 처리할 수 있어 이 API와 혼동하면 안 된다.
3. 설치 `tauri-runtime-wry-2.12.1/src/lib.rs:3496`은 Tao resize 호출 결과를 `let _ =`로 버린다. 같은 파일의 dispatcher는 메시지 큐 전송 결과를 반환한다. 따라서 IPC/Promise 성공은 실제 창 크기 변화의 증거가 아니다. [해당 버전 소스](https://docs.rs/crate/tauri-runtime-wry/2.12.1/source/src/lib.rs), [Tao macOS 소스](https://docs.rs/crate/tao/0.37.1/source/src/platform_impl/macos/window.rs).
4. Windows 설치 Tao는 ReleaseCapture 후 대상 HWND에 WM_NCLBUTTONDOWN을 게시한다(`windows/window.rs:490–527`). [Microsoft 문서](https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-nclbuttondown)는 이 메시지가 화면 좌표와 hit-test 방향을 쓰고 기본 처리에서 적절한 시스템 동작을 시작한다고 설명한다. Feedback 위의 포인터가 부모의 실제 외곽 밖에 있으므로 전달 경로의 존재만으로 자연스러운 resize/앵커 유지가 입증되지 않는다. [SetCapture](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setcapture)는 전경 창과 capture 소유권 제약도 명시한다.
5. [Apple performDrag](https://developer.apple.com/documentation/appkit/nswindow/performdrag(with:))는 원래 mouse-down event에 기반한 **이동** API다. [자식 창 부착](https://developer.apple.com/documentation/appkit/nswindow/addchildwindow(_:ordered:))은 부모 이동 시 자식 이동을 설명하며 resize 입력의 전파 계약은 제공하지 않는다. [setFrame](https://developer.apple.com/documentation/appkit/nswindow/setframe(_:display:))으로 위치와 크기를 프로그램에서 설정할 수 있지만 제스처 추적·제약·상태 조율은 앱 책임이다.
6. [Wry 0.57.0 WebViewBuilder](https://docs.rs/wry/0.57.0/wry/struct.WebViewBuilder.html)는 WebView 구성/임베딩 계층이다. 현재 native resize 경로는 Tauri runtime → Tao → OS이며 Wry의 build_as_child를 부모 native 창의 resize 전달 API로 해석하면 안 된다.

위 공식 링크는 모두 2026-11-14 조회. Apple 문서는 제공된 Markdown 링크로 본문을 확인했다. 버전 고정 builder 문서는 웹 도구 접근 실패였으므로 부모 동작의 근거는 실제 생성 코드와 Apple 문서로 제한했다. 위 설명은 각 출처의 제한된 관련 부분만 요약했다.

## 대안 비교

| 대안 | 장점 | 단점·실증 필요 | 판단 |
|---|---|---|---|
| A. Feedback 외곽 입력을 받아 부모 geometry를 갱신하는 전용 resize 세션 | 현재 독립창·자동 높이·통합 외곽선을 유지. 외곽 어느 창에서 시작해도 같은 의미를 정의 가능 | 포인터 capture/창 밖 이동·좌표·이미지 제약·취소를 직접 처리. native resize의 부드러움과 포커스를 별도 실증해야 함 | **단기 권장 설계**. 작은 격리 실험에서 macOS 입력 추적부터 입증하고 제품화 결정 |
| B. Feedback과 본문을 하나의 native 창에 통합 | OS가 한 창의 전체 외곽을 조절. 장기적으로 입력 소유권 단순 | 현재 본문 geometry 불변 계약, 자동 높이, 접힘, 이미지/PTY viewport, 포커스 구조를 광범위하게 바꿈 | 장기 구조 대안. 이번 문제만으로 우선 도입하기에는 범위가 큼 |
| C. Feedback 자체 높이를 수동 조절 | 긴 오류/안내에 더 많은 공간 제공 | 본문 높이는 늘지 않음. 자동 측정과 사용자 높이 우선순위·저장·복구 UI 필요. Windows native 가능성도 별도 검증, macOS는 custom 입력 필요 | 별도 긴 안내 요구가 확인될 때만. 현재 이미지에는 비추천 |

부모 `start_resize_dragging`만 호출하는 간단한 패치는 macOS 공통 해법이 아니므로 독립 대안으로 권장하지 않는다. Windows에서 최적화 경로로 검토할 수 있으나 실험이 선행되어야 한다. 어떤 대안도 단순 `resizable(true)` 변경만으로 완료되지 않는다.

Feedback 자체 native resize를 켜는 것은 C의 입력 수단일 뿐 부모 resize와 다르다. macOS에서 기존 native border의 자동 hit-test가 특정 스타일에서 작동할 가능성은 API의 NotSupported만으로 배제할 수 없다. 그러나 resizable 변경 후 실제 외곽 입력이 발생하는지, 자동 높이와 어떻게 양립하는지는 미검증이다. 별도 창 자체의 크기 변경을 parent 크기로 사후 복사하는 방법도 두 창의 반대쪽 anchor·폴링 소유권 문제를 그대로 가진다.

**유지 가능한 최소 구현 범위:** 현재 독립 창과 자동 높이를 유지하고, Feedback 전용 begin/update/end 명령과 hit 영역만 추가하는 A안을 먼저 실험한다. begin에서 서버가 owner를 도출하여 기존 `sizing::manual_resize(parent, Sizes)`와 같은 경로를 재사용하고, update에서 부모 geometry를 바꾼 뒤 기존 `feedback_window::sync`를 세션 규칙 안에서 재사용하는 방향이다. 이미 있는 macOS `inLiveResize` 및 Windows `WM_SIZING` 관찰 코드는 기존 본문 native resize용으로 유지한다. 이 관찰 코드가 programmatic resize까지 사용자 입력으로 식별해 줄 것으로 기대하면 안 된다. Windows의 실제 client/outer frame 보정도 `windows_window.rs`의 기존 경로와 맞춰야 한다. 제품 구현은 하지 않았다.

## 권장안 A의 상태·좌표 설계 제안 (미확정)

- **한 owner, 한 세션:** backend가 발신 Feedback의 label에서 부모를 결정한다. 클라이언트가 임의 parent label을 전달하게 하지 않는다. 시작 시 노출 edge·방향·부모/자식 frame·DPI·포인터의 전역 physical 좌표·generation을 snapshot한다. `Idle → Resizing → Commit/Cancel`을 owner별로 보유한다. 순번과 session token이 오래된 move/end를 차단해야 한다.
- **동시 갱신 제어:** resize 중 native sync/100ms layout이 부모를 다시 쓰거나 부착 방향을 바꾸지 못하게 한다. 내용 snapshot은 계속 받을 수 있지만 geometry 적용은 세션 소유자만 한다. mutex 보유 중 `set_size`/`set_position`을 호출하면 reentrant resize event로 교착할 수 있으므로 상태 결정·OS 반영을 분리하고 generation으로 재검사한다. 두 창을 OS 수준에서 원자적으로 바꾸는 계약은 없으므로 시각적 중간 상태까지 실증한다.
- **노출된 외곽만:** 아래 부착이면 Feedback 아래/좌우/아래 두 모서리, 위 부착이면 위/좌우/위 두 모서리가 대상이다. 연결 구분선은 기본적으로 resize 영역으로 삼지 않는다. edge=none의 겹침 fallback에서는 잘못된 합성 외곽 계산 대신 resize를 거부하고 본문 외곽 조절을 안내하는 방안을 우선 검토한다. 히트 영역의 폭은 아직 미합의이며 3px 패딩·버튼을 침범하지 않도록 시안 승인 필요다.
- **합성 frame 기준:** 부모 M=(x,y,w,h), 자동 Feedback 높이 f이면 아래 부착의 합성 영역은 (x,y,w,h+f), 위 부착은 (x,y-f,w,h+f)다. pointer delta는 이 합성 외곽에 적용하고 마지막에 M으로 환산한다. 예: 아래쪽을 40px 내리면 f가 같을 때 h만 40px 증가한다. 위 부착의 위쪽을 40px 올리면 합성 top이 40px 감소하고 parent y도 40px 감소·h는 40px 증가해 반대쪽 bottom을 보존한다.
- **너비 변경과 줄바꿈:** 너비를 좁히면 Feedback 자동 높이 f가 증가할 수 있다. 단순히 부모 delta만 더하면 합성 반대쪽 외곽이 움직인다. 합성 target frame을 유지하고 새 f를 측정한 뒤 h=targetHeight-f로 재계산하는 정책이 필요하다. 세션 중 edge를 고정하되 작업 영역을 벗어나면 clamp하고, 종료 후에만 일반 부착 재선택을 수행하는 방안을 검증한다.
- **단위·제약:** 입력과 frame 계산은 같은 모니터의 physical 좌표로 통일하고 CSS 논리 길이는 해당 scale로 변환한다. macOS AppKit의 화면 원점 방향과 Tauri 전역 좌표를 섞지 않는다. 혼합 DPI 이동 시 기준 재취득 또는 세션 종료 정책이 필요하다. min/max는 메모 유형·실제 native 제약에서 가져온다(기본 main 설정 144×32를 모든 유형에 하드코딩하지 않음). f를 제외한 본문 최소 높이·툴바·편집 영역, 작업 영역과 음수 좌표도 동시에 제약한다.
- **이미지와 접힘:** programmatic 크기 변경이 AppKit contentAspectRatio나 Windows WM_SIZING의 이미지 제약을 자동 적용한다고 가정하지 않는다. 원본 비율·툴바를 포함한 기존 의미를 따라 수학적으로 constrain하고 양쪽 모서리 anchor를 검증한다. 접힘·숨김·stash·destroy·revision 교체·모니터 제거 중 하나라도 발생하면 세션을 종료하고 늦은 입력을 무시한다. 접힌 창은 기존대로 Feedback을 숨긴다.
- **수동 의도·초안:** resize 시작에서 부모의 `manual_resize`와 동등한 배치 제외·애니메이션 취소를 적용한다. 본문 재렌더·편집 시작/종료·save/cancel·revision 변경 없이 geometry만 갱신한다. 텍스트/IME 조합·선택·스크롤·초안과 현재 포커스 보존을 검증한다. 비활성 첫 클릭의 포커스 정책은 기존 native 동작과 맞추고 별도 `set_focus`를 추가하지 않는다.
- **이동/버튼 충돌:** 버튼·링크·입력은 기능 유지, resize edge가 아닌 안내/빈 공간은 기존 이동. edge hit이면 이동 mousedown이 실행되지 않게 해야 한다. 현재 안내 텍스트는 user-select:none이므로 새 텍스트 선택을 도입하지 않는다. pointercancel·capture lost·앱 비활성·버튼 놓침에 정리 경로를 두고 창 밖 pointer capture의 WKWebView/WebView2 지원을 실제 확인한다. DOM capture가 부족하면 승인된 별도 native 추적 구현을 검토한다.
- **보안 경계:** 내부 command는 발신 창·owner 관계, 표시·접힘·active 상태, 유한 좌표·허용 방향·상한·세션 일치를 검증한다. 기존 main capability를 feedback-*까지 무조건 넓히지 않는다. 현재 `main.rs:633` 이후 invoke wrapper는 webpage-*의 모든 앱 IPC를 거부하고 updater frozen 동안 일반 명령을 차단한다. 이 경계와 개인정보 게이트를 유지해야 한다. frozen 진입 시 end 명령까지 거부될 수 있으므로 backend 자체 정리/취소와 stale session 해제도 필요하다. 외부 웹 콘텐츠가 새 IPC에 접근하지 못하는지 감사하며 메모 A의 Feedback으로 메모 B를 바꿀 수 없어야 한다.
- **6번 편집 높이와 충돌:** `Sizes.content_height/pending/expanded`, 편집 시 fit/needsFit, 부모 `set_size` 및 Feedback 재측정이 접점이다. 자동 편집 높이는 명시적 사용자 resize를 이후 measurement에서 덮어쓰지 않아야 한다. 자동 측정값과 사용자 지정 크기를 분리해 단일 backend 소유권·우선순위를 합의한 뒤 통합한다. 이번 브랜치에는 해당 구현을 넣지 않았다.

## 추후 구현 완료 기준과 검증 순서

1. 사용자에게 전체 메모 조절 의미와 노출 edge 범위를 확인하고 시안을 제출한다. 승인된 변경만 디자인 시스템·SVG/PNG에 반영하고 렌더링을 확인한다.
2. 먼저 전용 격리 창에서 macOS Feedback 시작 drag → 부모 geometry 변화 → 창 밖 이동 → mouseup/cancel 종료를 입증한다. Windows는 별도 같은 실험으로 native 전달/수동 경로를 비교한다. API 반환값 대신 시작·종료 frame과 포인터/시각 증거로 판정한다.
3. 순수 geometry 검증: 위/아래 부착별 외곽·모서리, 좌우 resize, 양/음 delta, 줄바꿈 높이 변화, min/max clamp, 이미지 비율, 음수 좌표·DPI, edge=none, 순서 역전/오래된 세션의 거부를 검사한다.
4. 출처 확인: 작업 root와 dev:info 기록 후 전용 고유 메모를 개발 CLI로 inspect하여 ok/instance/본문을 대조한다. 정확한 binary PID와 해당 메모 ID가 든 native 창을 특정하고 캡처한다. 재빌드/재시작 때 반복한다. 팀장 GUI 슬롯 승인 후 수행하고 기존 사용자 메모를 변경하지 않는다.
5. 실환경 matrix: macOS ARM64·Intel 및 Windows의 아래/위/화면 경계·배율 변경·복수 모니터, 텍스트 읽기·편집·오류·긴 버튼·이미지, 비활성 첫 클릭·드래그 취소·접힘/숨김·부모 destroy를 확인한다. 좁은 창에서 버튼 누름·이동·스크롤과 resize가 서로 오작동하지 않아야 한다.
6. 합격 기준: 포인터에 대응하는 합성 외곽의 연속 변화, 반대쪽 anchor/너비 정렬 유지, jitter·폴링 원복·이중 테두리·초안 손실·원치 않는 포커스 이동 없음, 종료 후 정상 자동 배치 및 숨김 복원, 다른 owner 변경 거부. 혼합 DPI 허용 오차는 물리 픽셀 반올림 기준으로 사전에 정해 기록한다.
7. 변경 범위에 맞는 UI/Rust 검사와 해당 워크트리 개발 빌드·CLI·GUI 증거를 기록한다. 미실행 플랫폼은 미검증으로 남긴다. 전체 플랫폼 CI·설치 파일 빌드·릴리스는 별도 요청 없이는 실행하지 않는다.

## 이번 검증과 한계

실행: 최신 원격 기준 확인, 열린/최근 종료 이슈와 PR 조사, 코드·Cargo.lock·설치 registry 소스 대조, 공식 웹 문서 확인, 관련 시안 이미지 열람, 도구 버전·dev:info 확인, 문서 링크/차이 검사. 제품 코드와 확정 디자인 규칙·SVG/PNG는 변경하지 않았다.

미실행: 앱 빌드·테스트·실제 CLI 서버 inspect·native drag·실환경 GUI·다른 플랫폼·CI. 개발 앱과 실사용 메모를 생성/조작/종료하지 않았다. 기존 개발 앱의 동작을 이번 조사 결과의 증거로 사용하지 않는다. 새 포인터 추적 방식이 부드럽고 안전하게 작동하는지는 후속 구현의 실증 과제다.

**Conf 1:** 코드·공식 자료 조사이며 실행 검증 없음. **Risk Low:** 현재 변경은 조사 문서뿐이며 쉽게 복구 가능. **제안 구현의 Risk Medium:** 두 native 창의 geometry·입력·상태 소유권을 다루므로 실제 검증 전 완료/안전으로 단정하지 않는다.
