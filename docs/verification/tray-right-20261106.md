# 트레이 단축키 오른쪽 정렬 — 2026-11-06

트레이의 캡처 3개와 전체 접어두기/복원만 변경했다. 기능명은 왼쪽, 단축키는 오른쪽 공통 열에 정렬하고 `·`를 제거했다. 키캡·배경 박스는 없다. macOS는 `Ctrl ⇧ 1/2/3` 및 `Ctrl Opt ⌘`, Windows는 `Ctrl ⇧ 1/2/3` 및 `Ctrl Alt Win`이다. 실제 키 등록·CLI 원시값·메모 Tooltip·말풍선은 그대로다.

macOS는 네이티브 NSMenuItem을 유지하며 attributed title의 오른쪽 탭 정렬을 사용한다. 네이티브 content 기준 탭 끝은 310 논리 px다. 현재 SDK의 Swift `NSTextAlignment.right.rawValue`가 2임을 직접 확인했다. 메모 제목은 이 서식 대상에서 제외하고 기존 가시성 서식을 유지한다. Windows는 네이티브 탭 열을 사용하며 실환경 검증은 미실행이다.

- `npm test`: Rust 151·UI 97 통과
- Rustfmt·Clippy -D warnings·개발 빌드·diff 검사 통과
- source root: `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/tray-menu-polish`
- instance: `dev-1fd207d59c82431a32969a4a`, 최종 PID **65431**
- binary: 위 루트의 `src-tauri/target/debug/agent-gibbon`
- SHA-256: `ca94585faac13cb8281d4543a0f4c2c8937b8356baa2c8ee71fa82e620903621`
- 해당 개발 CLI inspect의 id/instance/고유 본문, 실제 PID·실행 파일 및 같은 PID 네이티브 창 제목을 대조했다. 이전 자기 개발 앱만 종료하고 `scripts/agent-gibbon-dev --serve`로 재실행하여 기존 메모를 열지 않았다. 전후 전체 창의 숨김 상태가 같음을 확인했다.
- 실제 메뉴 캡처에서 숫자와 ⌘의 오른쪽 끝이 같은 열에 맞는 것을 확인했다. 첫 시도는 가운데 정렬로 보여 현재 SDK 상수를 확인한 뒤 올바른 값으로 고쳤다. 이후 최신 빌드를 재실행·재검증했다. 메뉴의 열린 메모 앞으로 선택도 정상 응답했다.

[실제 캡처](tray-right-20261106/preview.png) · [원시 기록](tray-right-20261106/report.json) · [디자인 기준](../design-system.md) · [시안 SVG](../design/tray-memo-list.svg) · [PNG](../design/tray-memo-list.png).

테스트용 네이티브 메뉴 기대 문자열도 탭 구분으로 갱신하고 Python 구문 검사를 수행했다. 모든 GUI smoke를 재실행한 것은 아니다. Windows·Intel·다크 모드 전환·전체 플랫폼 CI는 미실행이다. macOS 표시 Conf 5 / Risk Low. 커밋·push·배포는 하지 않았다.
