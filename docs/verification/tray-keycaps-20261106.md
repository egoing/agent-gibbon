> 키캡 실험은 사용자 요청으로 철회했다. 현재는 [일반 텍스트 표시 기록](tray-plain-20261106.md)을 따른다.

# 트레이 전용 키캡 — 2026-11-06

사용자 요청으로 **트레이 메뉴만** 변경했다. 메모 툴팁·Toast·접어두기 말풍선, 단축키 등록과 CLI 원시값은 그대로다.

- 캡처 세 항목: `[Ctrl] [⇧] [1/2/3]`
- 전체 창 접어두기/복원: macOS `[Ctrl] [Opt] [⌘]`, Windows `[Ctrl] [Alt] [Win]`
- macOS에서는 네이티브 NSMenuItem의 attributed title에 키별 시스템 배경색·글자색과 좌우 여백을 적용한다. 실제 모양은 **각진 옅은 배경 박스**이며 둥근 테두리나 사용자 정의 메뉴 뷰를 추가하지 않았다. 선택·접근성·클릭은 네이티브 항목으로 유지한다. 보조 설명은 박스 밖에 남는다. 메모 제목과 다른 메뉴 항목은 이 서식 대상에서 제외한다.
- Windows는 괄호형 텍스트를 사용한다. 그래픽 키캡·Windows 실제 실행·다크 모드 및 전체 플랫폼 CI는 미검증이다.

[디자인 기준](../design-system.md#트레이-전용-키캡--2026-11-06) · [SVG](../design/tray-memo-list.svg) · [PNG](../design/tray-memo-list.png) · [실제 메뉴](tray-keycaps-20261106/menu.png).

## 검사

`npm test`: Rust **151**, UI **97** 통과. Rustfmt·Clippy `-D warnings`·개발 빌드·`git diff --check` 통과. 수정한 네이티브 smoke의 Python 구문 검사와 AX helper Swift 컴파일도 통과했다. smoke 전체를 재실행한 것은 아니다. 키캡 변환·가용성 설명 보존과 다른 UI 변환이 유지되는 단위 검사를 추가했다.

## 실제 출처와 결과

- root: `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/tray-menu-polish`
- instance: `dev-1fd207d59c82431a32969a4a`
- PID **3044**, binary: 위 루트의 `src-tauri/target/debug/agent-gibbon`
- SHA-256: `fc9029eda405a0f810dc29e7291cf22850791e6815f3bfa057160c75fca96c30`
- 검증 메모: `tray76-keycap-proof`, 본문의 같은 고유 표식과 inspect instance/id, 해당 PID의 고유 네이티브 창 제목을 대조했다.

자신이 시작한 이전 개발 PID 63665만 종료·재실행했다. 메뉴는 해당 PID의 네이티브 창 범위만 캡처해 키별 박스와 Ctrl/Opt/⌘/⇧를 육안 확인했다. 실제 메뉴의 `메모 관리`를 선택해 같은 PID의 관리 창이 열리는 것도 확인했다. [원시 기록](tray-keycaps-20261106/report.json).

macOS AX는 꾸민 제목을 공백 포함 문자열로 반환한다. 테스트용 AX helper는 공백/텍스트 괄호를 정규화해 같은 키 조합의 항목을 선택하며, 트레이 smoke의 기대값도 갱신했다. 실제 입력 등록 문자열이나 이벤트 ID를 바꾸지는 않았다.

macOS 표시 및 메뉴 선택 **Conf 5**, 로컬 트레이 표시 변경 **Risk Low**. 커밋·push·배포는 하지 않았다.
