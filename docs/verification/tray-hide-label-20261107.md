# 트레이 숨김 명칭·항목 정리 — 2026-11-07

사용자 요청으로 트레이 `전체 창 접어두기 / 복원`을 `숨김`으로 바꾸고 `열린 메모 앞으로` 항목·전용 이벤트 처리를 제거했다. 기존 숨김/복원 토글·등록 키·CLI 원시값은 유지한다. 단축키의 오른쪽 정렬·55% 알파도 유지한다. README·앱에 포함된 사용 스킬에서 제거된 항목의 안내를 정리했다.

- `npm test`: Rust 151·UI 97 통과
- Rustfmt·Clippy -D warnings·개발 빌드·diff 검사 통과
- 변경된 smoke의 Python 구문 검사 통과. 제거된 기능의 검사를 삭제하고 기존 개별 복원 회귀는 유지했다. GUI smoke 전체 재실행은 하지 않았다.
- root: `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/tray-menu-polish`
- instance: `dev-1fd207d59c82431a32969a4a`, 최종 PID **81827**
- binary: 위 root의 `src-tauri/target/debug/agent-gibbon`
- SHA-256: `76b20d80ebc655d5e404a1297eb47247bf4a78aa76691f486bf133d18d8f4312`

자기 개발 앱만 재실행했다. 개발 CLI inspect의 instance/id/고유 본문, 실행 파일 절대 경로·PID, 같은 PID의 네이티브 창 제목을 대조하고 실제 메뉴에서 `숨김`과 제거 결과를 확인했다. 메뉴를 Escape로 닫았고 전체 메모의 전후 숨김 상태가 같았다. [실제 캡처](tray-hide-20261107/preview.png) · [원시 기록](tray-hide-20261107/report.json).

현재 단축키에 붙은 ‘사용 중 · 트레이로 실행’은 기존 입력 소유권 안내이며 이번 변경에서 제거하지 않았다. 토글 동작 자체는 변경하지 않았고 이번 실환경 검증에서 전체 창 토글을 다시 실행하지 않았다. Windows·Intel·전체 플랫폼 CI는 미실행이다.

[디자인 기준](../design-system.md) · [SVG](../design/tray-memo-list.svg) · [PNG](../design/tray-memo-list.png). macOS 표시 Conf 5 / Risk Low. 커밋·push·배포는 하지 않았다.
