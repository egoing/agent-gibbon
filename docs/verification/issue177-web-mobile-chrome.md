# 웹 메모 툴바와 모바일 표시 검증 — #177

## 출처와 범위

- 작업 루트 `/Users/egoinglee/.codex/worktrees/web-mobile-chrome/agent-gibbon`, 브랜치 `codex/web-mobile-chrome`, 기준 `91b1751`, 계획 커밋 `2e49442`. 제품 변경은 미커밋이다.
- `npm run dev:info`: instance `dev-bc6e1725e127f16cdc1f2999`, binary `/Users/egoinglee/.codex/worktrees/web-mobile-chrome/agent-gibbon/src-tauri/target/debug/agent-gibbon`.
- macOS 15.6.1 ARM64, Node26.0.0, pnpm11.25.0, Rust1.91.1, Tauri2.12.2/Wry0.57.0 설치 소스 확인. Intel/Windows 실행 및 전체 플랫폼 CI/배포는 하지 않는다.

## 실제 baseline

팀장 GUI 슬롯에서 PID37730의 실행 파일 경로와 CLI inspect의 instance·ID `web-issue177-native`·native 제목 `Agent Gibbon [DEV codex/web-mobile-chrome · bc6e1725] — 웹 — web-issue177-native`를 대조했다. Orca get-app-state 창ID73476과 일치한다. CUA는 번들 없는 debug 앱 선택이 불가능했고 computer-use 스킬의 PID 선택 경로를 사용했다.

Welcome URL 입력 focus→실행→loopback18777 `/redirect`→`/page`의 실제 HTTP와 화면을 확인했다. 첫 요청과 redirect 후 요청은 desktop Mac UA였고 innerWidth594/innerHeight367/visualViewport579이었다. 외부 child가 y30부터 나타나면서 앱 툴바는 AX에 남지만 픽셀에는 없었다. OS 제목줄은 baseline에서도 없었다. 수정 결과는 아래에서 별도 판정한다.

baseline capture: `/var/folders/ds/zm2jvd217ss_q5jy2bjcbb9c0000gn/T/orca-computer-use/00a3f508-148d-4684-9d9c-c896c68af3f8-screenshot.png`, snapshot `/tmp/gibbon-issue177/baseline-state.json`, HTTP `/tmp/gibbon-issue177/requests.jsonl`.

## 자동 검사

- `npm run test`: Rust223/UI204 통과.
- `npm run check`: Clippy all-targets -D warnings 통과.
- `npm run dev:build`: 통과. 재시작 후 최신 실제 동작 확인은 별도다.
- `scripts/web-toolbar-smoke.mjs`: 5테마×3너비(144/240/600), 주소 focus→실제 form submit→Welcome hidden 후 상단 위치·버튼·접힘·핀·hover·키보드 검사15조합 통과. mock IPC의 Chromium 검사이며 native 대체 증거가 아니다.
- `scripts/web-presentation-smoke.mjs`: authored viewport 보존, 누락 viewport 추가,3px 스타일, forced-colors native복귀, bridge 미생성 및 fresh navigation 검사 통과(Chromium).
- `rsvg-convert docs/design/web-welcome.svg -o docs/design/web-welcome.png` 실행, PNG 실제 열람 완료.

## 공식 API와 한계

[Tauri WebviewBuilder](https://docs.rs/tauri/2.12.2/tauri/webview/struct.WebviewBuilder.html)의 user_agent와 고정 initialization_script를 사용한다. 외부 invoke 거부, URL validate, 팝업/다운로드 차단은 기존 그대로다. 고정 script는 스타일과 누락된 viewport 힌트만 추가하며 외부 입력을 실행하거나 IPC 권한을 부여하지 않는다. 사이트 authored viewport는 유지한다.

[WebKit 공식 WKWebpagePreferences 헤더](https://github.com/WebKit/WebKit/blob/main/Source/WebKit/UIProcess/API/Cocoa/WKWebpagePreferences.h)는 preferredContentMode를 iOS 전용으로 명시한다. macOS에서 비공개 모바일 기기 설정을 호출하지 않는다. [Tauri ScrollBarStyle](https://docs.rs/tauri/2.12.2/tauri/webview/enum.ScrollBarStyle.html)의 Fluent overlay는 Windows 전용이며 데이터 디렉터리 전체에 동일 설정을 요구하므로 외부 한 창만을 위해 전역 설정을 바꾸지 않는다.

3px CSS는 native scrollbar의 표시 스타일이며 텍스트 메모의 별도 DOM overlay와 완전 동일한 hit/숨김/공간 사용은 보장하지 않는다. 서버별 UA/화면폭/쿠키/서비스 정책과 client platform/touch 신호 판정은 사이트에 따른다.

## 수정 native 검증

최신 빌드 재시작 후 PID65368 / native 창ID73583 / 동일 binary와 instance / ID `web-issue177-fixed` / 제목의 고유 ID를 대조했다. Welcome URL field를 focus→정확한 loopback 주소 입력(AXReplaceSelection verified)→실행→redirect→page 후 캡처에서 앱 툴바 버튼이 유지됨을 확인했다. OS 제목줄도 없다. child 콘텐츠 시작 위치는 정상인30px였고, 입력 focus 후 Welcome hidden 전환의 trusted chrome 스크롤/레이아웃 영향이 원인으로 추정된다. viewport 고정 후 동일 경로에서 버튼 표시를 확인했으며 native DOM scroll offset을 직접 계측한 것은 아니다.

실제 최초 `/redirect` 및 `/page` HTTP User-Agent와 페이지 navigator.userAgent는 모두 `Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1`로 일치했다. 페이지는 innerWidth594/innerHeight367/visualViewport579/dpr2를 보고했다. 이 값은 페이지 load 시 첫 측정이며 후속 스타일 적용 후 viewport와 혼동하지 않는다. `second` 링크로 `/second` 이동 후 해당 페이지의 back 버튼으로 `/page` 복귀를 실제 AX/화면에서 확인했다. reload는 아직 누르지 않았다.

[수정 native 캡처](issue177-native-toolbar.png)는 고정 toolbar와 실제 모바일UA를 함께 보여준다. 가로 thumb는3px 스타일을 따르지만 세로 native scrollbar는 두꺼운 상태였다. 이후 **공개 NSView 하위의 NSScrollView에만 Overlay를 지정하는 native 후보를 추가해 빌드·검사를 통과했으나 이 후보의 실제 수용 여부는 미검증**이다. OS 전역 설정이나 비공개 WK API는 사용하지 않는다. 관련 [Apple 공식 scrollerStyle](https://developer.apple.com/documentation/appkit/nsscrollview/scrollerstyle)의 공개 setter를 사용하며 모든 하위 view가 NSScrollView라고 가정하지 않고 superclass 확인 후 적용한다. 현재는 attachment 직후 한 번만 적용한다. 시작 패닉 후 early page-load callback을 제거했으므로 재탐색 뒤 native 설정 유지 여부는 별도 미검증이다. WebKit이 이 public descendant를 노출하지 않거나 설정을 거부하면 native fallback이 남는다.

## GUI 중단과 남은 검증

사용자가 Orca가 열리는 문제를 제기했고 팀장이 추가 실행·재기동과 native GUI 입력 중단을 지시했다. 현재 이를 준수하며 기존 Orca/사용자 앱을 종료하지 않는다. 시험창은 `visible:false`, `focused:false`로 확인했다. desktop PID 선택을 위해 스킬 가이드에 따라 Orca를 한 번 기동했으며 자동 반복 기동하지 않았다. 변경 코드 자체에는 Orca 실행 경로가 없다.

남은 실제 native 검사: 최신 Overlay 후보의 수용·viewport 재측정, 외부 invoke의 exact guard 오류 확인(읽기·쓰기·terminal_start·plugin/window), reload·resize·toolbar drag·fold/unfold·hide/reopen·외부 권한과 비HTTP(S) 탐색 거부. 고정 fixture는 `tests/fixtures/web-presentation-probe.html`에 준비했다. 이 fixture는 raw 사용자 메모 내용을 전송하지 않으며 probe 결과/일반 viewport 측정만 loopback에 보낸다. 미실행을 통과로 기록하지 않는다.

최신 후보는 Conf3(관련 테스트 통과), RiskMedium(외부 WebView/native 상태·사이트 표시 영향)이다. 툴바/UA 부분은 실제 환경 확인(Conf5)을 수행했지만 전체 변경의 Conf를 그 수준으로 올리지 않는다. Windows/Intel 실환경, 전체 플랫폼 CI/배포는 미실행이다. 최종 제품 커밋·push·병합은 승인받지 않았고 수행하지 않았다.


## 2026-11-14 개발 시작 패닉 수정

사용자가 `node scripts/dev.mjs dev` 시작 중 Wry0.57.0 wkwebview/mod.rs:1370 URL unwrap 패닉을 보고했다. 최신 실기기 crash report `agent-gibbon-2026-11-14-215319.ips`의 triggered thread에서 `web_chrome → web::inspect → Webview::url → Wry InnerWebView::url → url_from_webview → Option::unwrap_failed`를 확인했다. native scroller 호출이 해당 crash stack에 나타난 것은 아니다.

Wry의 URL getter는 초기 WKWebView.URL이 nil일 때 panic하므로 macOS inspect는 공개 nullable `URL`/`absoluteString` getter를 사용하고 실제 관측값을 캐시에 보관한다. dispatcher가 callback을 지연할 수 있어 UI thread에서 기다리지 않으며 `current_url`은 최초 미관측/미로드 때 null이다. `navigation_url`은 검증을 통과한 탐색 target이고 성공/commit을 뜻하지 않는다. 원래 `url`은 요청 주소다. 새로운 허용 탐색 시 이전 관측 캐시를 비운다. 거부 scheme은 탐색 캐시를 바꾸지 않는다. 빠른 page-load callback 자체도 Wry의 nullable URL 경로를 사용하므로 추가했던 해당 callback을 제거했다.

세션 저장은 유효한 실제 current_url을 우선하고 아직 없거나 about:blank이면 요청한 HTTP(S) 주소를 복원용으로 보관한다. 이를 화면의 관측 current_url로 대신 보고하지 않는다. null/about:blank/redirect/원래 요청/거부 scheme을 단위 회귀에 추가했다.

`RUST_BACKTRACE=1 node scripts/dev.mjs dev`로 직접 시작하고 기존 시험 웹2개·main·이미지의 hidden 상태를 보존했다. 서버 off에서 current_url:null인 inspect/list가 정상 응답하고 앱이 살아 있으며, server on으로 본인 dev 재시작 후 current_url:/page가 실제 관측됐다. 이어 loopback /page가302→/loaded를 응답하도록 해서 재시작했고 `url:/page`, `navigation_url:/loaded`, `current_url:/loaded`를 CLI로 구분 확인했다. `dev:info`와 instance는 위와 같고 첫 성공 실행 PID49680은 lsof txt로 정확한 작업 binary를 대조했다. 추가 GUI/Orca/컴퓨터유즈 입력은 하지 않았다.

복원된 hidden 외부 페이지의 고정 fixture 자동 보고에서 HTTP/navigator 모바일UA 일치, innerWidth594/clientWidth591/visualViewport591, CSS scrollbar3px를 확인했다. 3px 공간이 남으므로 텍스트의 완전한 overlay와 동일하다고 보고하지 않는다. native Overlay 설정의 시각적 수용 여부는 여전히 미검증이다.

외부 자동 probe의 manager_notes/save_note/terminal_start/web_chrome는 `not allowed. Plugin not found`, plugin:window|set_title은 permission 거부였다. 원문 메모 데이터나 terminal 실행 결과를 받지 않았고 title도 바뀌지 않았다. 이는 permission 계층에서 차단된 실제 증거이며 커스텀 invoke guard의 exact문구까지 도달해 시험한 것으로 보고하지 않는다. HTTP/fixture 로그는 `/tmp/gibbon-issue177/requests.jsonl`이다.

최종 관련 검사: Rust224/UI204 통과, Clippy -D warnings 통과, rustfmt/diff-check 통과, 개발 빌드 및 정상 dev 시작 확인. 시작 패닉 수정은 실제환경 확인(Conf5), 전체 작업은 앞서 남긴 native 상호작용 미검증 때문에 전체Conf3/RiskMedium을 유지한다. 모든 제품 변경은 미커밋이다.

## close 승인과 최신 main 통합

사용자가 2026-11-14 `$close`를 직접 호출해 현재 작업의 커밋·push·일반 병합을 승인했다. 제품 커밋 `e66d3d9`를 만들고 최신 origin/main `470bb52`를 merge한 `8b21ac3`에서 관련 검사를 다시 수행했다. 디자인 시스템 충돌은 웹 규칙과 Milkdown/자동저장 규칙 양쪽을 보존했다. 다른 워크트리의 미커밋 작업은 가져오지 않았다.

통합 코드에서 Rust224·UI180(최신 main의 테스트 구성)·Clippy -D warnings·개발 빌드·rustfmt·diff-check, Chromium toolbar15조합·presentation 검사 통과. `RUST_BACKTRACE=1 node scripts/dev.mjs dev` 정상 실행과 `web-issue177-fixed`의 instance 일치·hidden/focus/기하 보존·offline current_url:null inspect 성공을 확인했다. GUI/Orca를 사용하지 않았다. 전체 플랫폼 CI·Windows/Intel 실환경·공개 배포는 미실행이다.

#184는 사용자가 별도로 요청한 웹 목록 누락 후속 이슈다. 코드에서 manager_notes가 웹 entry를 합치지 않는 것을 확인했으며 이번 PR에 목록 기능을 혼입하지 않는다. #177은 native resize/drag/fold/reload/권한 UI 및 세로 overlay의 실제 외형 검증이 남아 있으므로 이번 병합으로 자동 종료하지 않는다. 남은 원래 요구를 완료한 것처럼 옮기거나 숨기지 않는다. 전체 Conf3/RiskMedium을 유지한다.
