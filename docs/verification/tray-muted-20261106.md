# 트레이 단축키 흐림 표시 — 2026-11-06

사용자 요청으로 macOS 트레이 단축키 열에만 기본 labelColor의 알파 0.55를 적용했다. 기능명은 네이티브 기본 글자색을 유지하고 오른쪽 탭 정렬·일반 텍스트 표시를 유지한다. 키캡·배경 박스는 없다. 메뉴 항목은 enabled이며 실제 입력 등록·CLI 원시값·다른 화면 표기는 바꾸지 않았다. 적용 범위는 캡처 3개와 전체 창 접어두기/복원이다.

- Rust 151·UI 97·Rustfmt·Clippy -D warnings·개발 빌드·diff 검사 통과.
- root: `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/tray-menu-polish`
- instance: `dev-1fd207d59c82431a32969a4a`, 최종 PID **81398**
- binary: 위 root의 `src-tauri/target/debug/agent-gibbon`
- SHA-256: `8a05a274a776ce8852e1f774c9f778dc76b0d74111961f2134db2f90b55c5968`
- 자기 개발 앱만 재실행한 뒤 개발 CLI inspect의 id/instance/고유 본문, 실제 PID·실행 경로, 네이티브 창 제목을 대조했다. 재실행 전후 모든 메모의 숨김 상태가 같았다.
- 실제 메뉴에서 기능명과 단축키의 대비·오른쪽 정렬을 육안 확인하고 enabled 상태와 메뉴 선택 응답을 확인했다. [실제 캡처](tray-muted-20261106/preview.png) · [원시 기록](tray-muted-20261106/report.json).
- [디자인 기준](../design-system.md) · [현재 시안](../design/tray-memo-list.png) · [SVG](../design/tray-memo-list.svg).

Windows 제목 색상 처리는 미구현이며 Windows·Intel·다크 모드·선택 강조 상태 전체 검수·전체 플랫폼 CI는 미실행이다. macOS 검증 Conf 5 / Risk Low. 커밋·push·배포는 하지 않았다.
