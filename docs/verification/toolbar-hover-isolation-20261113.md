# 다중 메모 툴바 hover 격리

## 원인과 변경

Tauri 2.12.2의 전역 `event.listen`은 기본 target이 `Any`이므로 `emit_to(label)` 이벤트도 모든 전역 구독자가 받는다. 따라서 한 창의 local 좌표와 null 이탈이 다른 창의 툴바 hit-test에 사용됐다. 설치된 Tauri `scripts/bundle.global.js`의 listen 기본값과 `src/event/listener.rs`의 Any 허용 경로를 확인했다.

`ui/toolbar.js`는 현재 WebviewWindow의 listen을 사용한다. 별도의 앱 전역 상태·포커스 변경·입력 합성을 추가하지 않는다. 두 독립 UI 컨텍스트에서 A 진입, B 이탈, A 이탈, B 진입을 전달하는 회귀 검사를 추가했다. 다른 창의 이벤트가 강조를 만들거나 지우지 않아야 한다.

추가로 기존 native 판정은 사각형 포함만 검사해 겹친 두 창 모두에 좌표를 만들었다. macOS `NSWindow.windowNumberAtPoint:belowWindowWithWindowNumber:`의 실제 최상단 마우스 대상과 현재 NSWindow 번호를 비교해, 가려진 창에는 null을 보낸다. 다른 앱의 가림도 동일하게 처리한다. 기존 논리 좌표 변환·변경 시에만 발행하는 정책을 유지한다.

[디자인 시스템](../design-system.md)의 기존 외형과 비활성 hover 정책을 구현하는 수정이며 SVG/PNG 시안은 변경하지 않았다.

## 실행 출처와 검증

- 작업 루트 `/Users/egoinglee/.codex/worktrees/02ec/agent-gibbon`.
- `npm run dev:info`: root는 위 경로, instance `dev-f3ab8dc6a8d44e989b85031f`, binary `/Users/egoinglee/.codex/worktrees/02ec/agent-gibbon/src-tauri/target/debug/agent-gibbon`.
- macOS ARM64, Node v26.0.0, rustc 1.91.1. 개발 빌드 성공 뒤 협의된 기존 PID96953을 재시작했고 PID6291의 실제 실행 파일 절대 경로를 확인했다.
- 검증 메모 `hover-02ec-a`, `hover-02ec-b`: 해당 워크트리 개발 CLI inspect의 `ok:true`, instance, ID, `HOVER-02EC-20261113` 본문 표식을 확인했다. AX snapshot의 해당 ID가 포함된 네이티브 제목으로 창을 구별했다.
- 사용자 창을 재시작하기 직전 snapshot·메모별 inspect로 기존 상태를 저장했다. 편집 초안 2개·관리창·접힘을 복원하고 기존 창의 위치·크기·초안 전체와 메모별 표시·접힘·핀 상태가 일치함을 assert했다. 포인터와 전면 앱도 원래 값으로 복원했다. 작업용 검증 메모 두 개는 숨기고 진행 메모는 남겼다.

| 검사 | 결과 |
| --- | --- |
| `npm run test:ui` | 186개 통과, 두 WebView 이벤트 격리 회귀 포함 |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 216개 통과 |
| `npm run check` | Clippy 통과 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | 통과 |
| `npm run dev:build` | 통과 |
| `git diff --check` | 통과 |

## 실환경의 한계

실제 개발 메모 2개를 열어 나란한 배치, 각 닫기 버튼 좌표 진입, 겹치기, 포인터를 둔 채 창 이동, 이탈을 실행했다. 그러나 배경 강조의 실제 결과는 시각 검증하지 못했다. Orca `list-windows --app pid:6291`은 빈 목록이었고, PID의 CGWindowBounds와 AX 제목/기하를 대조해 얻은 창 번호에 대한 `screencapture -x -l`은 `could not create image from window`로 실패했다. CUA 실행 파일 경로 선택도 `Invalid app`이었다. 설치 앱이나 다른 워크트리로 대체하지 않았다.

[실행 메타데이터](assets/toolbar-hover-20261113/cases.json)는 수행한 동작·AX 기하만 기록하며 모든 경우 `visual_verified:false`다. 이미지/터미널/웹·실제 장치 입력·다중 모니터·Intel·Windows와 전체 플랫폼 설치 빌드는 미검증이다. 기존 #143의 브라우저/앱 기록을 이번 실행 증거로 재사용하지 않는다.

Conf 3(회귀 테스트 통과), Risk Low(툴바 시각 상태의 국소 변경). 커밋·푸시·병합·배포 없음.
