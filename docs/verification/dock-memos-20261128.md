# macOS Dock의 열린 메모 목록

시험 ID `dock-1128-1138`. 기대 결과는 현재 표시 중인 메모마다 Dock에 한 항목이 생기고, 제목과 가시성 변경이 반영되며 선택한 창이 앞으로 오는 것이다. 시험 호스트는 macOS 15.6.1 arm64, Node 24.21.0, Cargo 1.91.1이다.

- 워크트리 `/Users/egoinglee/dev/project/agent-gibbon`
- HEAD `b54cc66a9ec9cc22db90be9750d6ceb41eb213cf`와 미커밋 제품 변경
- 인스턴스 `dev-a5210ae16e79437d38490c8f`
- 개발 바이너리 `/Users/egoinglee/dev/project/agent-gibbon/src-tauri/target/debug/jajan`
- 시험 바이너리 SHA-256 `dacef1af309c457f0eb3ff71aca6e4c329a0f2110576ffbc7b204f14b922e326`. Computer Use용 임시 번들 `/private/tmp/jajan-dock-1128-test/Dock Check.app/Contents/MacOS/jajan`은 같은 파일의 하드링크다.

기본 체크아웃의 시험 지침을 읽고 공통 배타 잠금을 획득했다. 기존 개발 앱 PID 31313의 실행 경로가 6f4b 워크트리인 것을 확인해 정상 종료했고 Jajan 프로세스 0개를 확인했다. 시험 앱 PID 43501의 실행 경로, `./scripts/jajan-dev inspect --id dock1128a` 응답의 인스턴스·본문, Computer Use의 창 제목·본문을 대조했다.

| 기준 | 결과 | 실제 관찰 |
|---|---|---|
| 메모 표시 제목 변경 | PASS | 시험 메모 두 개를 열고 A의 제목을 Dock Alpha에서 Dock Gamma로 바꾸자 실제 창의 접근성 제목도 바뀌었다. |
| 닫기와 다시 열기 | PASS | Computer Use로 A의 닫기 버튼을 누른 뒤 B가 포커스를 받았고, A의 inspect는 closed=true, visible=false였다. 같은 ID를 다시 열자 Dock Gamma와 visible=true가 돌아왔다. |
| 앱 재열기에서 관리 창 열기 | PASS | Finder에서 실행 중인 시험 번들을 다시 열자 동일 인스턴스의 메모 관리 창이 열렸다. |
| 실제 Dock 메뉴와 선택 | BLOCKED | Computer Use가 Dock의 번들 ID, 앱 이름, 전체 앱 경로에 연결할 때 각각 -10005 timeoutReached를 반환했다. 키보드로 Dock 포커스 이동도 시도했다. 실제 메뉴 목록·개별 항목 선택·기본 Dock 명령은 관측하지 못했다. |
| 제목 변경, 닫기·숨기기·재열기의 Dock 반영과 같은 제목의 두 창 선택 | NOT RUN | 실제 Dock 메뉴를 조작할 수 없어 창 상태만 확인했다. |
| 터미널·웹·캡처의 Dock 항목 | NOT RUN | 텍스트 메모 두 개로 시험했다. |

Finder 시험 창과 접근성 검사기를 닫고 검사기 프로세스 종료를 확인했다. A와 B를 숨긴 후 시험 앱에 정상 종료를 요청했으나 실행이 계속돼, 경로를 다시 확인한 시험 PID에 SIGTERM을 보냈다. Jajan 프로세스 0개를 확인한 뒤 두 시험 메모 디렉터리만 임시 시험 폴더로 옮기고 공통 잠금을 반환했다. Dock 메뉴 캡처는 확보하지 못했다.

## 최종 바이너리로 실제 Dock 재시험

시험 ID `dock-1128-final`. 같은 HEAD와 워크트리에서 메뉴가 열려 있는 동안 대상 창의 참조를 유지하도록 보완한 뒤 `npm run dev:build`가 성공했다. 최종 SHA-256은 `8c2b0014177b2a6ead19bacc4da590b8b9b8513ef7f6e09d8fd51a17df3e7d97`이다. 원본 지침을 다시 읽고 352.7초 대기 후 공통 잠금을 획득했다. Jajan 프로세스 0개를 확인하고 시험 번들의 하드링크를 최종 바이너리로 교체했다. 단일 시험 앱 PID는 54607이었다. 위와 같은 개발 CLI로 인스턴스·ID·본문을 대조했다.

Computer Use로 시험 메모 창을 조작하고, Dock은 요청받은 macOS 접근성 경로로 직접 조작했다. 실제 Dock의 `Dock Check` 아이콘이 제공하는 `AXShowMenu`로 메뉴를 열고 각 메뉴 항목의 `AXPress`를 실행했다. 메뉴의 접근성 좌표와 크기로 잘라낸 [실제 메뉴 화면](dock-memos-20261128.png)에서 제목 두 개와 기본 명령을 확인할 수 있다. 물리 마우스 우클릭 자체는 시험하지 않았다.

| 최종 바이너리 기준 | 판정과 관찰 |
|---|---|
| 열린 메모와 다른 앱에서의 전환 | PASS. Dock Alpha와 Dock Beta가 각각 한 번 표시됐다. ChatGPT가 전경일 때 Beta를 선택하자 PID 54607이 전경이 됐고 B inspect의 focused가 true였다. |
| 제목 갱신 | PASS. A를 Dock Gamma로 바꾸고 메뉴를 다시 열자 Gamma와 Beta가 나타났다. 이전 Alpha는 사라졌다. |
| 숨기기·재열기 | PASS. 개발 CLI로 B를 숨긴 뒤 Gamma 하나가 남았고, B를 Gamma로 다시 열자 같은 제목 두 개가 나타났다. |
| 같은 제목에서 정확한 대상 선택 | PASS. 첫째 항목을 선택한 뒤 A/B의 focused는 true/false, 둘째 선택 뒤 false/true였다. Computer Use로 둘째 창의 ID와 본문 B를 대조했다. |
| 메모 창 닫기·재열기 | PASS. B의 닫기 버튼을 누르자 메뉴에 Gamma 하나만 남고 B는 closed=true였다. 같은 ID를 Beta로 다시 열자 Gamma와 Beta가 각각 한 번 나타났다. |
| 기본 Dock 동작과 관리 창 | PASS. 옵션·모든 윈도우 보기·가리기·종료가 표시됐다. 아이콘의 기본 누르기로 기존 관리 창이 열렸으며 다음 Dock 메뉴에 시스템 관리 창 항목과 별도로 메모 목록이 표시됐다. 실제 종료 명령으로 시험 앱이 종료됐다. 옵션 하위 설정 변경과 모든 윈도우 보기·가리기 명령 실행은 NOT RUN. |
| 웹 시작 창 | PASS. 웹 시작 창이 열린 동안 ‘웹 시작’ 한 항목이 추가됐다. 모든 시험 메모를 숨기자 관리 창과 기본 명령만 남았다. 캡처·터미널 유형 별도 실행은 NOT RUN. |

시험 앱 종료 후 Jajan 프로세스 0개를 확인했다. 생성한 메모 두 개, 웹 창 색상 파일, 해당 세 창의 세션 항목을 시험 임시 폴더로 백업·정리하고 공통 잠금을 반환했다.
