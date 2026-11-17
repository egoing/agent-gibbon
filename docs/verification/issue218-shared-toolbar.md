# #218 공통 툴바 행과 웹 본문 시작 위치

기준 커밋 `16d1b2addff53526651441535b2d2b101a5f4f20`, 작업 루트 `/Users/egoinglee/.codex/worktrees/eb0e/agent-gibbon`, 브랜치 `codex/38-shared-memo-toolbar`. 사용자 첨부에서 웹 도구 버튼 바로 아래에 표시된 띠를 확인했다. 기존 웹의 시작 화면과 외부 콘텐츠는 모두 30px를 확보했고, 버튼과 다른 메모의 행은 22px였다.

기존 toolbar.js와 유형별 도구 구성을 유지했다. 네 유형의 행과 웹 시작 화면이 CSS 토큰을 사용하고 Rust 빌드는 같은 값으로 캡처 창과 외부 WebView 좌표를 계산한다. child 생성·재부착·리사이즈·배율 변경에 기존 네이티브 inset과 좌우·하단 3px 정책을 적용한다.

## Chromium 158 측정

144×80 및 300×100 논리 px에서 `node scripts/toolbar-layout-smoke.mjs`로 DOM 경계를 측정했다. 터미널 실행부와 네이티브 호출은 대역을 썼다. 창 테두리에 따라 행의 시작점이 1px 달라도 행 끝에서 본문까지의 간격은 같다.

| 유형 | 행 시작 y | 행 높이 | 본문 시작 y | 행 아래 간격 |
|---|---:|---:|---:|---:|
| 텍스트 | 1 | 22 | 23 | 0 |
| 이미지 | 0 | 22 | 22 | 0 |
| 터미널 | 1 | 22 | 23 | 0 |
| 웹 시작 화면 | 0 | 22 | 22 | 0 |

`web-toolbar-smoke.mjs`에서 5개 테마 × 3개 너비로 주소 입력 후 시작 화면이 사라져도 행 y=0, 높이 22를 유지하고, 핀·접기·펼치기·hover 상태가 동작함을 확인했다. `toolbar-focus-smoke.mjs`로 네 유형의 Tab/Shift+Tab과 강제 색상 표시, `toolbar-work-area-smoke.mjs`로 드래그·더블클릭·버튼 분리를 검증했다.

`pnpm test` Rust 303 통과·1 ignored, UI 215 통과. `pnpm check`, `cargo fmt --manifest-path src-tauri/Cargo.toml --check`, `pnpm dev:build` 통과. 공통 툴바와 웹 SVG를 PNG로 렌더링해 확인했다. 개발 인스턴스는 `dev-c11bf1af1b6708ea9f381673`, 실행 파일은 작업 루트의 `src-tauri/target/debug/agent-gibbon`이다.

## 실제 macOS ARM64 창 측정

2× 화면에서 전용 개발 CLI의 `open/web/inspect`로 각 메모 ID·인스턴스·가시성 및 텍스트 본문/이미지 메타데이터/터미널 제목/웹 URL을 확인했다. PID 94319의 실제 실행 경로를 `ps`로 확인하고, PID와 메모 ID가 일치하는 네이티브 창 제목으로 AX 요소와 CoreGraphics 창 번호를 대조했다. 창별 `screencapture -x -o -l <창번호>`를 사용했다. 다음 값은 창 상단 기준의 논리 px다.

| 실제 창 | 창 번호 | 행 시작 | 행 높이 | 본문 시작 | 아래 간격 |
|---|---:|---:|---:|---:|---:|
| [텍스트](assets/issue218/text.png) | 85395 | 1 | 22 | 23 | 0 |
| [이미지](assets/issue218/image.png) | 85392 | 0 | 22 | 22 | 0 |
| [터미널 시작 화면](assets/issue218/terminal.png) | 85506 | 1 | 22 | 23 | 0 |
| [웹 시작 화면](assets/issue218/welcome.png) | 85507 | 0 | 22 | 22 | 0 |
| [웹 첫 페이지](assets/issue218/first.png) | 85551 | 0 | 22 | 22 | 0 |
| [링크 탐색 후](assets/issue218/next.png) | 85551 | 0 | 22 | 22 | 0 |
| [240×160 최소 웹 창](assets/issue218/small.png) | 85551 | 0 | 22 | 22 | 0 |

웹은 `127.0.0.1`의 여백 없는 HTTP 페이지를 개발 CLI로 열고, 실제 WebView의 링크를 눌러 다음 페이지로 이동했다. 각각 현재 URL을 확인한 후 측정했다. 600×400에서 외부 WebView는 594×375, 240×160에서 234×135였다. 좌우·하단 3px 크기 조절 영역을 유지한다. [측정 원본과 창 출처](assets/issue218/measurements.json).

이미지와 터미널은 이 인스턴스에 만든 합성 저장 레코드를 앱에서 복원해 열었다. 터미널 본문은 작업 시작 화면으로 측정했다. 실제 캡처 제스처와 PTY 실행, 물리 키 입력·드래그·혼합 DPI·Intel·Windows는 이번 네이티브 검사에서 수행하지 않았다. 웹 주소 입력란의 AX 값 설정 시도는 입력값으로 반영되지 않아 폼 제출은 Chromium에서만 확인했다. 네이티브 웹 탐색은 CLI 최초 열기와 실제 페이지 링크 이동으로 확인했다. 고유 검증창을 숨긴 뒤 해당 바이너리 PID 94319와 이번 HTTP 서버 PID 96527을 종료했다.

Conf 5 (macOS ARM64에서 네 유형의 실제 간격과 웹 탐색 및 최소 크기), Risk Low (로컬 표시와 WebView 좌표 변경).
