# 관리 창 본문 편집과 목록 아이콘 실기기 시험

[최종 관리 창 캡처](manager-edit-20261128.png) · [디자인 기준](../design-system.md) · [시안](../design/manager-notes-wireframe.svg)

- 워크트리 `/Users/egoinglee/dev/project/agent-gibbon`, HEAD `b46e77882527374e324885a8194a8d75efea1e33`와 미커밋 제품 변경, macOS 15.6.1 arm64
- `npm run dev:build` 성공. `npm run dev:info` 인스턴스 `dev-a5210ae16e79437d38490c8f`, 바이너리 `/Users/egoinglee/dev/project/agent-gibbon/src-tauri/target/debug/jajan`
- 1차 job `manager-edit-1128`, SHA-256 `e04a9d4dedcfc47e0186fb232e07372fbb48cee2a388451eedb37b3c9538b78a`, PID 272. 최종 job `manager-edit-1128-final`, SHA-256 `9e437a9a65245286e003329e16af1196b8593f393a119d71b330382c7193ac12`, PID 8710. 최종판에서 겹친 저장 요청은 선행 요청이 모두 끝날 때까지 기다린다.
- 두 시험 직전 기본 체크아웃의 AGENTS.override.md를 읽고 공통 배타 잠금을 획득했다. 1차는 기존 개발 앱 PID 88701의 6f4b 워크트리 실행 경로를 확인하고 정상 종료했다. 각 시험은 Jajan 앱 0개에서 시작했다. 시험 전용 Manager QA.app은 해당 개발 바이너리의 하드링크다. `./scripts/jajan-dev inspect`의 인스턴스·ID·본문과 Computer Use 창 제목·본문을 대조했다. 최종 관리 창의 WindowServer ID 11824와 PID를 대조해 시험 ID만 검색된 창을 캡처했다.

| 기준 | 결과 |
|---|---|
| 숨긴 메모 자동 저장 | 최종 PASS. 관리 창에서 B 본문을 바꾸고 기다리자 inspect의 markdown이 ‘Manager QA B autosaved’와 줄바꿈, revision 3, visible=false였다. 플로팅 창을 열지 않고 디스크에 저장했다. |
| 빠른 선택 전환 | 두 빌드 PASS. 1차에서 A 편집 직후 B를 눌러 A만 변경되고 B의 원래 본문이 유지되는 것을 확인했다. 최종판에서 B 편집 직후 C를 누르자 C의 본문이 표시됐고, B inspect는 ‘Manager QA B switch’와 줄바꿈이었다. |
| 연결 파일 충돌 | 두 빌드 PASS. C의 연결 파일을 외부에서 바꾸고 관리 창에서 편집 후 다른 행을 선택했다. 선택 이동이 중지되고 초안과 원본 파일 ‘Manager QA external’이 보존됐다. |
| 다른 쓰기와 경쟁 | 최종 PASS. C의 초안을 유지한 상태에서 CLI로 C 본문을 ‘Manager QA C remote’로 바꿨다. 이전 초안을 저장하자 revision 충돌 안내가 나타났고 입력이 유지됐다. 변경 취소 후 최신 remote 본문이 표시됐다. |
| 열기구와 접근성 | 최종 PASS. 실제 캡처에 표시 중인 A만 열기구가 보이며, 접근성 행 이름에 유형·제목·‘표시 중’이 포함됐다. 실제 A의 ×를 누른 후 행에서 상태가 사라졌고 inspect는 closed=true, visible=false였다. 1차에서 B를 관리 창으로 표시하고 CLI로 숨기는 양방향 동기화도 확인했다. |
| 비텍스트 상세 | 1차 PASS. 캡처를 클릭해 이미지 상세, 터미널을 클릭해 기존 삭제 버튼, 웹 시작 창을 클릭해 제목·유형·숨기기 동작이 있는 상세를 확인했다. 터미널 삭제 명령은 실행하지 않았다. |

모든 시험 종료 시 앱 정상 종료를 요청했다. 남은 시험 PID와 바이너리를 대조해 SIGTERM으로 종료한 뒤 프로세스 0개를 확인했다. 시험 메모와 생성한 웹 색상 파일을 시험 임시 디렉터리로 옮기고 원래 개발 세션 파일을 복원한 후 공통 잠금을 반환했다. 1차 마지막 자동 저장 조작은 Computer Use 세션 오류로 판정하지 않았고 최종 빌드에서 다시 확인했다.

Windows 실기기, 한국어 IME 조합 중 행 전환, 스크린 리더 음성 출력, 서식 메뉴 전체 조합은 NOT RUN. 접근성 확인은 실제 macOS 접근성 트리 및 입력 포커스에 한정한다.

## 편집 영역과 선택 서식의 최종 화면

[참고와 같은 글자를 선택한 화면](manager-format-selection-20261128.png) · [굵게 적용 후 화면](manager-format-active-20261128.png)

- 시험 ID `manager-format-1128`. 같은 워크트리, HEAD 및 개발 인스턴스에서 미커밋 스타일 변경을 포함해 `npm run dev:build` 성공. 최종 개발 바이너리 SHA-256은 `9eca67ceb55b699cc92c87ee7202d27b20e03b54b5b48d8d20ec0fb87d7f46fe`이다.
- 원본 시험 지침을 다시 읽고 공통 잠금을 획득했다. 기존 개발 앱 PID 12707 종료와 인스턴스 0개를 확인한 뒤 바이너리 하드링크로 만든 시험 번들을 하나만 실행했다. PID 14649, 관리 창 11991, 인스턴스·시험 ID `formatqa1128`·본문을 개발 CLI inspect와 Computer Use로 확인했다.
- PASS. 숨긴 시험 메모에서 사용자 캡처와 같은 문장 중 ‘중입니’를 선택했다. 실제 창 캡처에서 짧은 본문 포커스 표시, 옅은 액션 배경, 한 줄 팝오버, 개별 버튼의 경계가 사라진 것을 확인했다. 목록·설정·탐색 버튼은 기존 스타일을 유지했다.
- PASS. Option+F10으로 굵게 버튼에 포커스가 이동했고 Return으로 굵게 상태가 off에서 on으로 바뀌었다. 저장 중 활성화된 저장 버튼이 완료 후 비활성으로 돌아왔다. inspect 결과는 revision 2, `모아보기 테스트 **중입니**다.`와 줄바꿈, visible=false였다.
- PASS. 시스템 테마에서 검정·흰색으로 바꿔 각각 본문을 선택하고 실제 팝오버 표면과 글자를 확인했다. 종료 전 원래 관리 테마 ‘시스템’을 복원했다. Windows 강제 색상 모드는 NOT RUN.
- 시험 종료 요청 후 실행 중이던 시험 PID와 바이너리를 다시 대조해 SIGTERM으로 정리했다. Jajan 앱 0개, 시험 메모 백업과 기존 세션 복원 후 공통 잠금 반환을 확인했다.

## 본문 기본 입력 커서

최종 job `manager-caret-1128`. 위 워크트리·인스턴스와 HEAD `b46e77882527374e324885a8194a8d75efea1e33`에 미커밋 제품 변경을 포함해 `npm run dev:build` 성공. 바이너리 SHA-256은 `29d19d82470fb24e21ef40ab1801850c1e7b4a5e4ea74302c6ebcac090f2afad`다. 이전 서식 캡처는 왼쪽 표시 제거 전 기록이다.

- PASS. 원본 시험 지침을 다시 읽고 공통 잠금을 획득했다. 기존 개발 앱 PID 17182 종료와 Jajan 앱 0개를 확인했다. 최종 바이너리 하드링크를 실행한 PID 20812의 관리 창 12133을 식별했다. 개발 CLI inspect로 시험 ID `caretqa1128`, 본문, 인스턴스를 대조했다.
- PASS. 문장 ‘오늘 회의에서 초안을 검토합니다.’의 ‘초안’ 앞으로 커서를 이동해 A를 입력했다. [실제 입력 화면](manager-caret-20261128.png)에서 기본 커서는 A와 초안 사이에 있고 첫 줄 왼쪽에는 별도 표시가 없다. inspect로 변경된 본문과 숨김 상태를 확인했다.
- PASS. ‘초안’을 선택하고 Option+F10을 누르자 접근성 포커스가 서식 도구의 Bold 버튼으로 이동했다. 시험 앱 종료와 Jajan 앱 0개, 시험 메모 정리 및 기존 세션 복원 후 잠금을 반환했다. Windows와 물리 한글 IME 조합은 이번 시험 범위 밖이다.
