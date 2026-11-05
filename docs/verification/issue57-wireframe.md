# #57 확정 와이어프레임 구현·검증

2026-11-05 사용자 확정안이 이전 전문가 합의의 원클릭 실행·단일 경로 정책을 대체했다. [디자인 시스템](../design-system.md#사용자-확정-와이어프레임--2026-11-05)이 현재 구성의 정본이다.

## 동작

- 작업 위치의 폴더 열기만 아이콘을 사용한다. 최근 폴더는 이름과 부모 경로, 추천 작업은 이름만 표시한다. 선택 배경과 `aria-pressed`를 함께 제공한다.
- 최근 폴더는 기본 3개, 더 보기로 최대 20개 표시한다. 창별 저장은 최대 20개이며 기존 마지막 경로는 자동으로 이어받는다. 여러 창의 기록을 중복 제거하고 현재 창의 선택을 우선한다. 없는 최근 폴더는 제외하되 현재 창의 삭제된 마지막 경로는 실행 오류와 함께 다른 선택을 요구한다.
- 선택만으로 실행하거나 폴더를 서버에 저장하지 않는다. 실행 버튼을 눌렀을 때 검증된 폴더를 저장하고 선택한 앱/셸을 시작한다. 폴더 선택기 취소는 현재 선택을 보존하며 선택기 대기 중 실행 버튼을 비활성화한다.
- 명령 미리보기는 읽기 전용이며 폴더와 작업 선택에 따라 갱신한다. 백엔드가 인용한 경로·실행 파일을 표시한다. 실제 PTY는 cwd를 별도로 설정하고 앱 명령을 보내므로 미리보기는 동등한 시작 동작의 표현이며 내부 프롬프트 제어 코드는 표시하지 않는다. Windows는 PowerShell 5 호환을 위해 `&&` 대신 `Set-Location -ErrorAction Stop`과 다음 줄 명령을 사용한다.
- 남는 공간에는 수직 가운데 정렬하고 작은 창은 위부터 스크롤한다. 제목 변경은 하단 접힌 영역에 유지한다. 기존 에이전트, PTY 수명, 숨김과 종료 정책은 변경하지 않는다.

## TDD와 검증

작업 루트 `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/57-terminal-welcome`, 개발 인스턴스 `dev-868cf91c545212324aa21ebf`.
기준 `e55d3644858368ebb46399140646bcc59d7d83cf`. 추가 red 커밋은 `refs/is2/red/57-wireframe` (`d4608b8`)에 보존했으며 최초 is2 base/red ref를 덮어쓰지 않았다. red는 최근 폴더 직렬화 누락과 작업 선택 시 즉시 실행됨을 각각 실제 assertion 실패로 확인했다.

| 검사 | 결과 |
|---|---|
| 추가 red → green | metadata roundtrip 및 선택 후 Run 실패 확인 후 통과 |
| 터미널 범위 | `cargo test --manifest-path src-tauri/Cargo.toml terminal::` 25/25 PASS |
| 전체 회귀 | Rust 107/107, 최종 UI 67/67 PASS |
| 타입 | `cargo check --manifest-path src-tauri/Cargo.toml` PASS |
| 린트 | Rustfmt check, Clippy all-targets `-D warnings`, JS 구문, diff check PASS |
| 빌드 | `npm run dev:build` PASS |
| 브라우저 | 실제 HTML/CSS/xterm + 모의 Tauri, 720×460·320×240·720×900 PASS |

브라우저에서는 최근 폴더 3개/더 보기, 선택 배경·미리보기, 선택 시 미실행, Run 직전 경로 저장 순서, 홈·셸 전용, 폴더 선택기 취소·대기, 키보드·작은 창 스크롤·긴 경로, 지연 실행 실패의 포커스 보존을 확인했다. 720×900에서는 콘텐츠 상하 여백 차이 2px 이내로 수직 중앙 정렬을 검증했다. [전체 화면](issue57-wireframe-browser.png)과 [작은 화면](issue57-wireframe-small.png), 갱신된 SVG/PNG를 직접 시각 확인했다.

기존 원클릭/명령 details 테스트 변경은 사용자 확정안에 따른 기대 동작 변경이다. 기존 에이전트 숨김·실패 복구·제목·종료 검증을 유지했다. 원본 로그는 `/tmp/gibbon-wireframe/`에 있다 (`red-rust.log`, `red-ui.log`, `new-final.log`, `regression.log`, `ui-final.log`, `typecheck-final.log`, `clippy-final.log`, `build-final.log`).

## 한계

이전 부분 범위 discovery 테스트 실패는 이번 동일 범위에서 통과했다. 해당 탐색 코드나 assertion을 바꾸지 않았으므로 근본 원인을 해결했다고 주장하지 않으며 이전 불안정 이력을 보존한다. 최신 빌드로 실행 앱을 재시작하거나 native GUI/VoiceOver/Windows 실앱을 재검증하지 않았다. 브라우저 검증은 실제 PTY·OS 폴더 선택기 검증을 대체하지 않는다. 기존 PS1·여러 줄 입력의 보수적 종료 확인 한계는 유지한다. Risk Medium, 자동 병합 없이 리뷰 대기.

독립 최종 감사: 테스트 완화·추가 퇴행·스코프 이탈 없음. 발견한 취소 선택 초기화와 목록 상한 경계를 수정한 뒤 재검토했으며, 감사자가 UI 단위 13개를 직접 실행하고 나머지 원본 로그를 확인했다.
