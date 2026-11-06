# #65 읽은 뒤 포커스 이탈 시 정리 (2026-11-07)

사용자 승인으로 롤오버/포커스 진입 즉시 확인을 폐기했다. 메모 또는 연결된 Feedback에 포커스가 있는 동안 마지막으로 렌더링한 revision을 기억하고, 두 영역 모두에서 포커스가 떠날 때 diff와 완료 강조를 함께 정리한다. 읽기 중 새 갱신은 유지하고 이탈 순간 도착한 미열람 revision은 보존한다. 편집 중 이탈은 초안을 보존하며 확인하지 않는다.

백엔드는 현재 Note revision 일치와 저장 성공을 확인한 뒤 비교 기준·일시 완료 강조·알림을 정리한다. 완료 강조만 있는 replace도 같은 경로를 사용한다. 롤오버는 Toolbar·투명도에만 영향을 준다. 일반 본문/완료 강조의 표시 방식과 기존 혼합 갱신 제한은 유지한다.

- 작업 루트 `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/diff`, instance `dev-b09222c3d68057de28b6eb9c`.
- Rust 161개, UI 98개, Rustfmt·Clippy·개발 빌드 통과.
- UI 회귀는 포커스 진입 유지, 완료 강조 이탈 정리, 연결 Feedback 이동, 이탈 순간 새 revision 보존, 초안·저장 실패를 검증한다.
- 시안 SVG의 동작 설명과 PNG 렌더·시각 확인 완료. README·기본 스킬·디자인 시스템·이슈 #65 정책을 갱신했다.
- 초기 native fixture의 제목 클릭 경로에서 새 갱신 검사에서 표시 소실/IPC 일시 오류가 발생했다. 이를 통과로 보고하지 않는다. 제목 클릭의 드래그 경로가 영향을 줄 가능성이 있어 검사 방식을 바꿨으나 원인은 확정하지 않았다. 실제 사용자 드래그 동작과 포커스 전환 검사를 분리하기 위해 PID·정확한 창 제목에 대한 접근성 Raise/Main으로 포커스를 지정한다.
- Windows·Intel 실환경 및 모든 편집/원본 충돌 GUI 조합은 미검증. 커밋·push·전체 플랫폼 CI는 승인 대기.

[디자인 시스템](../design-system.md) · [시안](../design/feedback-preview.png).

최종 접근성 포커스 지정 시도도 기대한 읽음 해제까지 도달하지 못해 native 전체 검사는 미통과다. 자동 검사 통과와 구분하며 실제 포커스 획득/이탈 및 WebView 관찰 타이밍의 추가 확인이 남아 있다. 테스트 전용 서버는 종료했다. Conf 3, Risk Medium.

## close 실증 중 최종 재검증

- 위 native 미통과 상태를 보완했다. 제목 클릭과 AX Raise 대신 테스트 메모의 관찰된 Toolbar 고정 버튼 위치를 실제 클릭해 포커스를 얻고 핀을 두 번 전환해 원상복구했다. 포커스 지정 방법만 고쳤으며 제품 코드를 검사에 맞춰 바꾸지 않았다.
- 최종 서버 PID **36728**, instance `dev-b09222c3d68057de28b6eb9c`. [실행 근거](issue65-focus/evidence.json)의 `passed:true`, `test_server_stopped:true`를 확인했다. lsof 실행 경로·동일 PID의 정확한 창 제목과 CLI instance/고유 본문을 대조했다.
- 롤오버로 diff가 유지되고, 포커스 진입에도 유지되며, 다른 테스트 메모로 포커스를 옮기면 정리됐다. 포커스 중 새 diff는 유지되고 다음 이탈에 정리됐다. 완료 강조의 hover 유지, 숨긴 peer 갱신, geometry·핀 보존을 확인했다. 변경 전용 Feedback도 보이지 않았다.
- 이번 macOS native 범위 Conf 5, Risk Medium. Windows·Intel 실제 GUI 및 모든 편집 조합은 여전히 미검증이다.
