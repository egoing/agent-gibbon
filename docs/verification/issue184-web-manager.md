# 웹 세션 관리 목록 (#184)

기준 main 22dc7de에서 관리 목록 조회는 웹 레지스트리를 누락하고 관리 화면은 웹 유형을 제외한다. Welcome 및 주소가 있는 웹 세션을 같은 ID로 조회하고 원래 호스트의 표시 상태를 제어한다.

- 저장 메모·터미널·웹 혼합 목록과 빈 목록에서 조회·선택만으로 창 상태가 바뀌지 않는지 확인한다.
- 숨김·표시, 탐색 실패와 주소 전환, 복원 시 표시되는 행을 확인하고 웹 주소를 텍스트로만 표시한다.
- Rust 상태 검사와 독립 브라우저의 대역 IPC 검증, 실제 네이티브 창·재시작 검증의 범위를 구분한다.

## 구현과 결과

- 관리 창은 웹 레지스트리의 ID마다 한 행을 읽는다. Welcome에는 주소가 없어도 행이 남는다. 주소가 있으면 세션에 지정된 주소를 사용하며 페이지 리디렉션 추적이나 외부 페이지 제목 조회는 하지 않는다. CLI의 상세 inspect 대신 등록된 호스트에서 visible·minimized만 조회해 2초 주기 폴링에서 WebView URL·파일·기하 조회를 피했다.
- 웹 행은 표시 명령에 유형을 포함한다. 수신 시점의 레지스트리에서 호스트를 다시 찾아 같은 세션을 표시하거나 숨긴다. 사라진 웹 세션은 오류를 반환한다. 기존 웹 표시·숨김과 stash 해제 경로를 공유한다. 트레이 최근 목록은 텍스트·캡처·터미널만 대상으로 하는 별도 기준으로 유지하며 더보기는 기존 관리 목록을 연다.
- 관리 화면은 웹 주소와 유형을 textContent로 표시한다. 별도 탐색·편집·터미널 삭제 동작은 제공하지 않는다. main의 webpage- invoke 거절, HTTP(S) URL 검사와 팝업·다운로드 거절 콜백의 코드는 보존했다.

| 검사 | 실제 결과 |
|---|---|
| macOS ARM64 15.6.1, Node 24.21.0, Rust 1.91.1 | npm test Rust 266·UI 199, npm run check, cargo fmt --check, npm run dev:build 통과 |
| Chromium 157 실제 관리 UI + 대역 IPC | 10개 시나리오 통과. 혼합 유형·복수 웹, 숨김/표시, Welcome→주소 동일 ID, 실패 후 기존 행 유지, 제거·같은 ID 재등록, 선택·스크롤, 갱신 경합, 빈 목록, 600×360, HTML처럼 보이는 제목의 텍스트 표시. 페이지 오류·외부 요청 0 |
| Rust 호스트 대역 | 제품의 웹 행 조회·상태 판정·웹 표시 명령 함수를 컴파일해 별도 실행. 호스트 교체 후 새 호스트만 표시·숨김, 최소화 상태 판정, 무관한 세션 보존, stale ID 거절, 약관·업데이트 잠금 거절 확인. 네이티브 API는 대역 |
| 시작 기록과 탐색 | 기존 Rust 테스트에서 숨긴 접힌 웹 상태 왕복, 초기·리디렉션 주소 선택, 중복 기록·잘못된 주소 거절 통과. 기존 웹 Welcome JS 검사에서 유효하지 않은 주소·네이티브 실패 뒤 입력과 재시도 유지 확인. 실제 복원 경로는 저장된 label의 ID로 create_presented를 호출하고 기하·핀을 적용함을 코드로 확인 |
| 시안 | manager-web-wireframe.svg를 rsvg-convert로 PNG 렌더링하고 한글·두 행·표시 버튼·유형 위치를 확인. Chromium의 좁은 화면에서 웹 유형과 잘린 제목이 겹치지 않음을 확인 |

Tauri 2.12.2 / tauri-runtime-wry 2.12.1 / Tao 0.37.1의 설치 소스에서 최소화 해제 경로를 대조했다. Windows의 기본 SW_RESTORE는 대상 HWND를 활성화한다. 웹 관리 버튼은 owner thread에서 SW_SHOWNOACTIVATE를 사용한 뒤 기존 비활성 표시 경로를 따른다. macOS에서는 실제 최소화된 호스트의 deminiaturize 호출 중에만 Tao가 canBecomeKeyWindow 및 canBecomeMainWindow에 사용하는 focusable 값을 보호·복원한다. [Microsoft ShowWindow 동작](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-showwindow) · [Apple NSWindow deminiaturize](https://developer.apple.com/documentation/appkit/nswindow/deminiaturize%28_%3A%29)

작업 루트는 9aa2/agent-gibbon, 개발 인스턴스는 dev-dd228e8c795a7c1ab9c2efc0이다. dev:info와 개발 CLI --instance-info에서 같은 빌드 출처를 확인했다. 앱 프로세스는 시작하지 않았다. 네이티브 창 표시 및 포커스 전환·최소화 애니메이션·핀과 기하 보존·탐색 실패와 재시작 복원·Windows 실행 및 Intel은 실기기 미검증이다.

#119와 web.rs가 겹친다. 결합할 때 종료 처리 중인 레코드를 관리 목록에서 제외하고 해당 ID의 재표시를 거절해야 한다. 종료 처리와 지연된 클릭의 경합, Destroyed 뒤 같은 ID의 새 호스트, 종료 실패 후 기존 행 복구, 명시 hide/stash 후 재표시를 함께 검증한다. 이 검증은 #119 병합 전 독립 패치에서는 수행하지 않았다.

세 제안을 함께 적용한 결과는 [웹 세션 종료·관리 검색 통합 검증](integration-manager-20261116.md)에 기록했다.
