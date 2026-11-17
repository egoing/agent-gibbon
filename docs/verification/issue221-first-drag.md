# #221 비활성 메모 첫 드래그

텍스트·이미지 메모는 기본 창 설정의 `acceptFirstMouse: true`를 상속했다. 터미널과 웹 호스트는 별도 빌더로 생성되어 Wry의 기본값인 `false`를 사용했다. Wry 0.57.0의 WKWebView는 이 값을 AppKit의 `acceptsFirstMouse:` 결과로 돌려준다. 툴바에 최초 누름이 도달해야 공통 이동 로직이 시작점을 기록할 수 있다. [AppKit 첫 클릭 처리](https://developer.apple.com/documentation/appkit/nsview/acceptsfirstmouse(for:)) · [Tauri 빌더 API](https://docs.rs/tauri/2.12.2/tauri/webview/struct.WebviewWindowBuilder.html#method.accept_first_mouse)

터미널과 로컬 웹 호스트가 텍스트·이미지와 동일한 설정 필드를 참조하도록 했다. 기존 툴바의 4px 이동 임계값, 버튼·입력·resize 제외, 정지 더블클릭과 외부 웹 child의 생성 코드를 유지한다. 새 이벤트 모니터나 IPC를 추가하지 않았다.

### macOS Apple Silicon 전후 비교 · 2026-11-17

| 유형 | 기존 · 외부 앱 | 기존 · 다른 메모 | 수정 · 외부 앱 | 수정 · 다른 메모 |
|---|---:|---:|---:|---:|
| 텍스트 | +96,+44 | +96,+44 | +96,+44 | +96,+44 |
| 이미지 | +74,+44 | +74,+44 | +74,+44 | +74,+44 |
| 터미널 | 0,0 | 0,0 | +51,+44 | +19,+44 |
| 웹 | 0,0 | 0,0 | +19,+44 | +19,+44 |

수치는 첫 누름~해제 한 번 동안 관측한 창 원점의 변화량(dx, dy)이며 단위는 AppKit 논리 포인트다. 위쪽 모니터에서 오른쪽 96pt, 아래쪽 48pt의 OS 합성 드래그를 보냈다. 네 유형 모두 첫 제스처로 이동했고 창 크기는 각 제스처 전후 같았다. 창 변위와 커서 변위의 정확한 일치까지 검증한 결과는 아니다.

- 워크트리 `/Users/egoinglee/.codex/worktrees/be06/agent-gibbon`, 브랜치 `codex/41-inactive-first-drag`, HEAD `16d1b2addff53526651441535b2d2b101a5f4f20`. 기준은 해당 HEAD 위에 37번 통합 패치 `77b921d096747bd2d2473f4ff92c084a25cf43b6ccccdf162bafef517f6c0dac`를 적용하고 manifest 78파일 해시를 대조한 상태다.
- `pnpm dev:info`로 인스턴스 `dev-f5c947a5b4f2a0b122edb232` 및 워크트리 아래 `src-tauri/target/debug/agent-gibbon`을 확인했다. 수정 전후 각각 `pnpm dev:build` 후 이 경로의 프로세스와 `scripts/agent-gibbon-dev inspect --id …` 응답의 성공·인스턴스·고유 메모를 확인했다.
- 기준 PID 22592, 바이너리 SHA-256 `1cc394f8d042331b07899a6b75b0aa7e147244f3e5865d4f53915642ecb0c57e`, 표식 `g41-488e7d`, 터미널 `terminal-1794880544647613000`. 수정 PID 26081, SHA-256 `4ceaeda5df817d4a0168afd06ec423e05f09d0e11c8f7db0e1367bb3e3c9af45`, 표식 `g41-093447`, 터미널 `terminal-1794880611105431000`.
- 타깃은 해당 PID의 네이티브 창 제목에 있는 메모 ID로 특정했다. 각 입력 전 별도 Cocoa 앱의 실제 전경 PID·실행 경로·AX 창 제목 또는 같은 Gibbon PID의 고유 source 메모 포커스를 확인했다. AX 툴바의 빈 지점에서 맨 앞 창의 PID·제목, 버튼 해제 상태, 연속 4회 동일한 창 frame을 확인했다. CGEvent 한 번의 down, 12회의 dragged, 한 번의 up을 사용했다. CLI 물리 좌표는 별도 보존했다.
- 로컬 원자료는 `~/Documents/Codex/2026-11-17/gibbon41/`의 `baseline-controlled-488e7d.json`, `revised-controlled-093447.json`, `regression-clean-e1774f.json`에 있다. 준비 중 다른 메모가 앞을 가리거나 포커스 검증에 실패한 실행은 표에서 제외했다.

### 회귀 검사

수정 빌드에서 네 유형 모두 외부 앱에서 첫 클릭으로 누른 고정 버튼이 한 번 전환됐고, 툴바 빈 곳에서 드래그해 버튼의 원래 화면 좌표에서 놓았을 때는 고정 상태가 유지됐다. 가장자리 드래그로 실제 창 크기가 변했고, 빈 툴바 더블클릭으로 작업 영역까지 확대했다가 기존 frame으로 복원했다.

실제 편집기에서 입력한 문자열의 자동 저장, 터미널의 실제 PTY 명령 입력 및 본문 드래그 중 창 위치 보존, 웹 Welcome 주소 입력과 이동을 확인했다. loopback HTML 버튼의 호출 수로 외부 웹 child는 비활성 첫 클릭에 0회, 뒤이은 활성 클릭에 1회 실행됐음을 확인했다. 저장소는 이 인스턴스 전용 `~/Library/Application Support/org.egoing.floatnote/notes/dev-f5c947a5b4f2a0b122edb232/docs`를 사용했다. 테스트용 저장 자료는 제품 밖에 보존했다. 이슈 검증을 위해 실행한 앱과 별도 Cocoa 포커스 창을 종료하고 원래 전경 PID와 포인터 좌표를 대조했다.

회귀 검사 PID 30207에서 위 19건을 확인했다. PID 34246의 별도 PTY 선택 검사에서는 동일 문자열이 출력된 셀을 수평 드래그해 창을 움직이지 않고 102pt 너비에 선택 강조가 나타나는 것을 해당 창 단독 캡처로 확인했다. 원자료는 같은 폴더의 `native-selection-2130ad.json`, `terminal-selection-before.png`, `terminal-selection-after.png`다.

Rust 308개 통과·1개 제외, UI 223개 통과, Rustfmt, Clippy 및 개발 빌드를 확인했다. Chromium에서 네 유형의 클릭·더블클릭·드래그와 실제 편집기 회귀를 추가 검사했다. 사람이 잡은 물리 마우스·트랙패드, Intel Mac, Windows, 설치본과 서명 빌드는 미검증이다.

최종 정상 개발 경로에서 다시 빌드한 SHA-256 `2a55a68879f78c75f14bd943ba3a84e1daad81bb0f4c329882c5f3feb21d6424`는 별도 프로세스 PID 36507에서 인스턴스·창 출처를 재확인하고 동일한 8개 조합을 모두 재검사했다. 결과는 `final-controlled-f305a3.json`에 있다. 검증용 자료는 별도 폴더에 보존하고 최종 개발 바이너리를 워크트리에 남겼다.
