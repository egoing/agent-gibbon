# #200 메모 드래그 종료와 관계선 재표시 검증 계획

기준 main `22dc7de`. 장시간 눌러 정지해도 양끝 중 하나가 이동 중인 관계는 숨김과 계산 중단을 유지해야 한다. 버튼을 놓은 뒤 최신 native 좌표로 계산한 SVG의 현재 세대 확인만 재표시할 수 있다.

- 텍스트·이미지·터미널·웹의 신뢰된 창 이동 진입점, Tauri/Tao의 OS 요청과 종료 관찰 방식을 대조한다.
- 이동 → 150ms 이상 누른 채 정지 → 재이동 → release를 검증하고 다른 메모 관계의 표시를 확인한다. 이전 요청의 종료·SVG 확인, 시작 실패·취소·삭제·연속 드래그와 기존 포커스·stash·안정화 경계를 살핀다.
- 상태 대역과 실제 OS 입력 증거를 구분한다. 기존 앱·사용자 창·입력 권한을 건드리지 않는다. 종료를 안정적으로 식별할 수 없다면 근거와 실기기 검증 조건을 남긴다.

## 기준 코드와 구현

현재 main은 마지막 이동에서 150ms가 지나면 버튼을 계속 누르고 있어도 양끝 계산에 진입할 수 있었다. 기준 커밋의 실제 `Motion`을 분리 컴파일해 149ms·2초 시점을 대조했다. 물리 드래그 화면 재현은 미실행이다. 열린 #200을 재사용했다. #168은 부모 이슈이며 #199와 Draft #207은 화살촉 렌더링 작업이다.

- 텍스트·캡처/이미지의 index.html, 터미널과 웹 호스트가 모두 toolbar.js를 사용한다. 왼쪽 포인터가 4 CSS px 이동하면 chrome_drag → chrome_window → drag_note로 들어온다. 웹 호스트의 별도 web_chrome drag 진입점도 같은 Rust 이동 함수를 사용한다. 현재 web-chrome.js는 resize·pin·close를 호출한다. resize와 CLI·키보드·정렬·모니터 이동 이벤트에는 드래그를 등록하지 않는다.
- Tauri 2.12.2, tauri-runtime-wry 2.12.1은 DragWindow를 디스패치하고 내부 Tao 호출 결과를 버린다. Tao 0.37.1의 macOS 구현은 AppKit에 넘기며 macOS SDK NSWindow.h는 즉시 반환 및 mouseUp 미전달 가능성을 명시한다. Windows는 WM_NCLBUTTONDOWN/HTCAPTION을 PostMessageW로 보낸다. [Apple NSWindow 문서](https://developer.apple.com/documentation/appkit/nswindow/performdrag%28with%3A%29).
- 시작·관련 선 숨김·native 요청을 UI 스레드에서 순서대로 처리하고 native 호출 전에 상태 잠금을 해제한다. 드래그 중에는 그 메모의 Moved로 안정화 시계를 재시작하지 않는다. 현재 드래그에 속하지 않는 이동, 크기 변경과 전체 복원 대기는 기존 150ms를 사용한다. native 요청 전달 오류는 해당 요청 번호가 여전히 현재일 때만 해제한다. 내부 OS 시작 오류가 전달되지 않더라도 버튼을 놓으면 추적이 끝난다.
- 드래그를 시작한 경우에만 기존 관계 갱신 루프에서 버튼과 창 수명을 확인한다. 활성 드래그의 다음 확인 간격은 최대 32ms를 목표로 하며 실제 메인 루프 스케줄링 지연이 더해질 수 있다. macOS에서는 pressedMouseButtons와 CoreGraphics 세션의 왼쪽 버튼 놓기 누적 횟수를 이용한다. Windows에서는 기본 버튼 매핑을 고려해 기존 버튼 조회를 재사용하고 해당 HWND의 기존 subclass에서 이동 루프 시작/종료·파괴를 기록한다. 새 요청 시작 전 이미 시작한 루프의 종료는 새 요청을 지울 수 없다. subclass 설치 실패 시 버튼과 창 존재 확인이 남으며 두 샘플 사이 release·재누름을 놓치면 다음 release까지 숨김이 길어질 수 있다. 전역 훅과 권한을 추가하지 않았다. [Windows 이동/크기 루프 종료](https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-exitsizemove) · [기본 버튼과 물리 버튼 조회](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getasynckeystate).
- 창 숨김과 최소화는 되돌릴 수 있으므로 그 자체를 버튼 놓기로 해석하지 않는다. release는 재계산을 허용하고, 실제 표시에는 포커스·stash·가시성·겹침 검사와 최신 좌표의 SVG 확인이 필요하다. 창을 다시 만들어도 이전 세대 번호를 재사용하지 않는다. SVG 확인 직전 양끝이 사라지거나 달라지면 해당 결과를 버리고 다시 계산한다. 한 개의 UI tick 응답을 기다리는 동안 native 이동 루프가 5초 이상 걸려도 관계 갱신 스레드가 종료되지 않도록 했다.

## 확인 결과

macOS ARM64, Node 24.21.0, pnpm 11.25.0, Rust/Cargo 1.91.1. 전용 워크트리 8a9a, 인스턴스 dev-b6ea89691b5aa43ee6c28e1c. npm test에서 Rust 270개·UI 198개, npm run check, Rustfmt, git diff --check, npm run dev:build를 통과했다. 전용 개발 CLI --instance-info로 실행 파일의 manifest 경로와 인스턴스 ID를 확인했다. 기존 GUI·서버를 시작하거나 inspect하지 않았다.

- 기준 Motion의 150ms 조기 복귀를 확인했다. 수정본의 실제 tick 중 관계 순회·기하·draw 호출과 relation_ready를 메모리 창 대역에 연결해 재생했다. 시간·버튼·포커스·창 IO를 대체했으며 설정 읽기/등록·OS 표시 호출은 제외했다. 이동→누른 채 2초 정지→7.6초에 재이동 동안 관련 선은 숨겨졌고 양끝의 좌표·크기 읽기는 0회였다. 다른 관계선은 계속 표시됐고 그 양끝도 재계산하지 않았다.
- 7.601초에 release를 입력한 뒤 새 draw 요청까지 발생했지만 선은 여전히 숨겨졌다. 이전 ack와 draw 뒤에 좌표가 바뀐 경우를 거절했다. 새 payload의 실제 포트 좌표가 (−8200, 50)에 맞고 대응하는 ack에서 표시됨을 확인했다. SVG DOM 자체의 갱신·렌더 세대 합치기는 UI의 relation-render 테스트로 확인했다.
- 같은 대역에서 무포커스·stash와 복원 안정화·ack 전에 최소화·겹침·표면 재생성·대상 삭제를 검증했다. 제품 Rust 테스트로 새로운 드래그와 오래된 실패 통지, 현재 요청 실패, 버튼 재누름과 native 종료, 닫힌 호스트를 검증했다. Windows MoveLoops 구현을 별도로 추출해 늦은 이전 종료·새 시작·종료·파괴에 대한 결과를 확인했다. 실제 HWND 메시지 전달 시험은 미실행이다.
- SVG에서 1040×620 PNG를 렌더하고 변경 문구를 육안 확인했다. 선의 정지 외형·렌더러 payload·관계 저장 형식·PTY는 그대로다. [디자인 시스템](../design-system.md) · [상호작용 SVG](../design/relation-prototype.svg) · [PNG](../design/relation-prototype.png).

물리 마우스·트랙패드의 화면 밖 release, AppKit/Windows 실제 취소·빠른 재드래그, 혼합 배율/Space, Intel·Windows 빌드와 실행은 미검증이다. macOS에서 별도의 native 취소 신호 없이 기본 버튼이 계속 눌려 있으면 놓을 때까지 숨김을 유지할 수 있다. 이슈의 물리 시험 완료 기준은 열어 둔다. Conf 3, Risk Medium(창 표시와 이벤트 수명 상태). 배포·기존 앱 재시작·전역 입력 조작 없이 검토를 마쳤다.

## #199 화살촉 접합과의 결합 검증 · 2026-11-16

앞 절은 #200 단독 후보의 결과다. 결합 후보에서는 #199의 shaft/head 계산과 연결 미리보기 공통 helper를 적용했다. 두 시안의 SVG를 합성·렌더하고 PNG를 육안으로 확인했다. 제품 Rust, 렌더러 payload와 확인 명령은 두 후보의 계약을 유지한다. [#199의 기하·시안 기준](issue199-arrow-junction.md).

실제 Rust tick이 보낸 drawRelation 호출을 Chromium 139에 넘겼다. 제품 JS가 DOM을 갱신한 뒤 보낸 relation_ready 세대를 지연시켜 같은 Rust 확인 함수에 전달했다. 창·시간·버튼·포커스·IPC 전송은 대역이다. 7.6초의 드래그와 중간 2초 정지 동안 관련 선의 위치 읽기는 0회였고 무관한 선은 계속 표시됐다. release 직후 재이동·포커스 상실·전체 숨김/복원·최소화·겹침·삭제·표면 재생성에서 표시 경계를 확인했다.

추가로 대각선 양방향 연결을 45px 간격에서 release했다. 원래 양끝에 맞춰 14px 머리를 그린 세대 8의 확인이 도착하기 전에 대상 창을 1px 간격으로 옮겼다. Rust가 그 확인을 거절했고 세대 9에서 머리 둘이 각각 0.25px로 줄어 양끝에 맞은 뒤 표시됐다. 통상 길이·좁은 간격·화살표 없음 등을 포함한 Chromium 렌더 252쌍, 뷰포트 가장자리 12건도 통과했다.

결합 소스에서 Rust 266개, UI 201개, Rustfmt·Clippy·macOS ARM64 개발 빌드를 통과했다. 실제 로그인 셸 프로필을 실행하는 Rust 4개는 제외했다. 기존 앱·물리 release·혼합 모니터·Windows 네이티브 실행과 Windows 타깃 컴파일은 미검증이다. Chromium 렌더 Conf 4, 네이티브 수명 Conf 3, 결합 Risk Medium.
