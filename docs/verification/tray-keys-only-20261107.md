# 숨김 단축키만 표시 — 2026-11-07

트레이의 단축키 변환에서 상태 설명을 생략한다. `숨김` 오른쪽에는 macOS `Ctrl Opt ⌘`, Windows `Ctrl Alt Win`만 표시한다. `사용 중`·`입력 감지 권한 필요`·`트레이로 실행` 같은 부가 정보는 트레이에서만 생략하며 실제 입력 소유권·등록·동작·CLI 원시값·다른 화면의 안내는 유지한다. 기존 오른쪽 정렬과 55% 알파를 유지한다.

- Rust 151·UI 97·Rustfmt·Clippy -D warnings·개발 빌드·diff 검사 통과.
- 상태 설명 생략과 다른 화면의 설명 보존을 같은 회귀 검사에서 확인했다. macOS 사용 중 안내와 Windows 권한 안내의 두 경우를 검사했다.
- root: `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/tray-menu-polish`
- instance: `dev-1fd207d59c82431a32969a4a`, 최종 PID **10293**
- binary: 위 root의 `src-tauri/target/debug/agent-gibbon`
- SHA-256: `23a51946d57c73782ff05781071601096b2c626c3b5daa288cc0960101d7b1d5`

자기 개발 앱만 재실행했다. 개발 CLI inspect의 instance/id/고유 본문, 실제 PID·실행 경로 및 같은 PID의 고유 창 제목을 대조했다. 실제 메뉴의 `숨김` 항목이 `Ctrl Opt ⌘`만 표시하고 enabled임을 확인했다. 전후 전체 메모의 숨김 상태도 같았다. [실제 캡처](tray-keys-only-20261107/preview.png) · [원시 기록](tray-keys-only-20261107/report.json).

기존 [시안](../design/tray-memo-list.png)·[SVG](../design/tray-memo-list.svg)의 키만 표시하는 기준과 일치하므로 도안 자체는 변경하지 않고 [디자인 기준](../design-system.md)의 적용 규칙을 갱신했다. Windows 실제 실행·전체 플랫폼 CI는 미실행이다. macOS 표시 Conf 5 / Risk Low. 커밋·push·배포는 하지 않았다.
