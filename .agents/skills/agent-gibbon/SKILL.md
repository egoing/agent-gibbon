---
name: agent-gibbon
description: macOS 또는 Windows에 설치된 Agent Gibbon 앱으로 플로팅 Markdown 메모를 만들고 조회·갱신·숨긴다. 사용자가 화면 위 메모나 기존 메모 변경을 요청할 때 사용한다.
---

# Agent Gibbon 사용

## 시작하기

[홈페이지](https://agentgibbon.com/)와 [설치·CLI 안내](https://agentgibbon.com/guide/)를 참고한다. 앱은 [GitHub Releases](https://github.com/egoing/agent-gibbon/releases/latest)에서 내려받아 Applications 폴더에 설치한다. Apple Silicon은 `aarch64`, Intel Mac은 `x64` DMG를 사용한다. 배포판은 Apple 배포 서명·공증이 없어 macOS에서 실행이 차단될 수 있다.

설치 앱의 실행 파일은 `/Applications/Agent Gibbon.app/Contents/MacOS/agent-gibbon`이다. 경로를 따옴표로 감싸 호출한다. 파일이 없으면 설치 위치를 확인하고 설치 안내를 제공한다. 메모 사용 요청만으로 소스를 빌드하거나 개발 앱을 실행하지 않는다.

```sh
"/Applications/Agent Gibbon.app/Contents/MacOS/agent-gibbon" list
"/Applications/Agent Gibbon.app/Contents/MacOS/agent-gibbon" open --id my-note --title '메모' --markdown '# 메모'
"/Applications/Agent Gibbon.app/Contents/MacOS/agent-gibbon" update --id my-note --markdown '내용을 갱신했습니다.'
"/Applications/Agent Gibbon.app/Contents/MacOS/agent-gibbon" update --id my-note --color '#dcecff'
"/Applications/Agent Gibbon.app/Contents/MacOS/agent-gibbon" inspect --id my-note
"/Applications/Agent Gibbon.app/Contents/MacOS/agent-gibbon" close --id my-note
```

설치 버전에 따라 제공 기능이 다를 수 있다. 명령 옵션은 해당 실행 파일의 `--help`, 결과 필드는 실제 JSON 응답으로 확인한다. 최신 소스에만 있는 기능을 설치 앱도 지원한다고 가정하지 않는다.

## Windows

Windows 설치 파일과 개발 절차는 저장소의 `docs/windows.md`를 참고합니다. 설치한 `agent-gibbon.exe`의 실제 경로를 확인해 PowerShell에서 `& '실행 파일 절대 경로' list`처럼 호출합니다. CLI 명령과 창 상태 보존 규칙은 macOS와 같습니다. 설치 위치를 추측해 다른 실행 파일을 호출하지 않습니다.

개발 테스트는 해당 워크트리의 `scripts/agent-gibbon-dev.cmd`를 사용합니다. 편집 저장은 Ctrl+S이며 macOS 전용 보조키·포인터 접촉 기능 대신 화면 버튼을 사용합니다.

## 창과 상태 보존

- `--id`는 영문·숫자·하이픈·밑줄 1~64자이며 대소문자를 구분한다. 생략하면 `main`이다. 기존 메모를 먼저 확인하고, 다른 작업의 내용을 덮어쓰지 않도록 ID를 정한다.
- `open`은 앱이 필요하면 실행하고 해당 메모를 만들거나 표시한다. 최초 표시와 사용자의 명시적인 복원 요청에 사용한다.
- 이후 내용 변경은 `update`로 보낸다. 숨긴 메모는 숨긴 채 갱신되며 위치·키보드 포커스·고정 상태를 보존한다. 갱신 실패를 `open`으로 자동 대체하지 않는다.
- `close`는 메모를 숨긴다. 앱 프로세스를 종료하거나 저장 파일을 삭제하지 않는다. 숨긴 메모는 계속 갱신할 수 있다.
- `list`는 숨긴 메모도 포함한다. 앱이 실행되지 않았다면 `list`·`update`·`inspect`·`close`는 실패하며, 자동 실행은 `open`만 수행한다.
- `inspect --id ID`로 본문과 실제 가시성·위치·크기·포커스·고정 상태를 확인한다. 없는 ID의 `update`·`inspect`·`close`는 실패한다.
- 생략한 제목·본문·상태는 보존한다. 이 세 필드의 빈 문자열은 값을 비운다. 화면에 표시할 제목·진행 상태는 Markdown 본문에 넣는다. `--title`·`--status`는 CLI 메타데이터다.
- 색상은 `--color '#RRGGBB'`로 지정한다. 글자색은 자동 선택되며 기존 메모에서 생략하면 색상을 유지한다. 색상 이름·빈 문자열·알파값은 지원하지 않는다.
- 표시 내용과 갱신 시점은 사용자 또는 호출한 워크플로우가 정한다. 이 스킬은 작업계획 작성이나 특정 체크리스트를 강제하지 않는다.

## Markdown과 오류 처리

긴 내용은 파일 또는 표준입력으로 전달한다. 셸이 본문의 `$`나 백틱을 실행하지 않도록 따옴표로 감싼 heredoc 구분자를 사용한다.

```sh
"/Applications/Agent Gibbon.app/Contents/MacOS/agent-gibbon" update --id my-note --file - <<'MD'
# 현재 작업
- [x] 내용 확인
- [ ] 결과 검토
MD
```

- 체크리스트는 `- [ ] 대기 항목`, `- [x] 완료 항목`으로 작성한다. 목록 기호 없는 `[ ] 항목`은 사용하지 않는다.
- 종료 코드와 JSON의 `ok`를 확인한다. `MARKDOWN_TASK_MARKER` 오류는 지적한 행을 고쳐 같은 ID로 재시도한다. 같은 오류가 반복되면 원인을 확인하고 무한 재시도하지 않는다.
- Markdown 표·코드·읽기 전용 체크리스트를 표시할 수 있다. 외부 이미지 로딩이나 링크 이동은 지원하지 않는다. 메모 원문을 외부 서비스로 전송하지 않는다.
- 앱이 본문 변경을 강조하므로 호출자가 변경 표시용 Markdown을 추가할 필요는 없다. 숨긴 메모의 변경은 메모를 다시 열지 않는다.

## 파일 연결과 저장

- `--file 경로`는 원본 Markdown 파일을 연결한다. 설치 앱에는 접근 가능한 파일의 절대 경로를 전달한다. 연필 버튼으로 저장하면 메모와 연결한 파일을 함께 갱신한다.
- `--markdown` 또는 `--file -`로 본문을 교체하면 기존 원본 연결을 해제한다. 제목·상태·색상만 변경하면 연결을 유지한다. `--markdown`과 `--file`은 함께 사용하지 않는다.
- 외부 파일 변경을 자동 감시하지 않는다. `update --id ID --file 경로`로 다시 읽는다. 원본 파일을 다른 도구와 동시에 편집하지 않는다.
- 사용자 편집 중 내용이 갱신되거나 원본 파일이 바뀌면 저장 충돌이 발생할 수 있다. 초안을 보존하고 최신 내용을 확인한 뒤 다시 편집한다.
- 저장 기능을 지원하는 버전은 본문·제목·상태·색상·원본 연결을 로컬에 보관하고 재실행 시 숨긴 상태로 복원한다. 실제 저장 위치는 `inspect.storage_path`, 연결한 원본은 `inspect.note.source_path`로 확인한다. 창 위치·크기·핀·글자 크기·접힘·미저장 초안은 재실행 후 유지되지 않는다.
- 앱 저장 파일을 직접 수정하거나 백업하려면 앱을 종료한 상태에서 작업한다. `close`는 앱 종료가 아니다.

## 사용자가 화면에서 할 수 있는 일

- 메모 본문을 드래그해 이동하고 가장자리로 크기를 조절한다. 수동으로 정한 크기는 내용 갱신 시 유지된다.
- `aA`로 글자 크기를 바꾸고, 연필로 원문을 편집한다. 저장은 ⌘S, 편집 취소는 Escape이며 복사 버튼으로 본문을 복사한다.
- 좌우 화살표 또는 Option + 포인터 접촉으로 화면 반대편에 옮긴다. Control + 포인터 접촉으로 접거나 펼친다. Control + Option + Command + 포인터 접촉은 숨기기다.
- 메뉴 막대의 개별 메모를 선택해 숨긴 메모를 복원할 수 있다. `열린 메모 앞으로`는 이미 열린 메모만 앞으로 가져온다.
- 메뉴 막대에서 숨긴 텍스트 메모를 직접 열면 오른쪽 우선 빈 공간에 바로 표시합니다. 이미 열린 메모와 이미지 메모는 위치를 유지합니다.
- 등장 방향 선택을 지원하는 버전에서는 CLI로 연 메모가 커서 옆에 잠시 대기한 뒤 마우스 방향에 따라 움직인다. 좌우는 해당 가장자리, 위는 접고 오른쪽으로 이동, 아래는 숨기기다. `open` 응답은 이 이동 완료를 기다리지 않는다.

앱 자체에는 AI 대화 모델·계정·클라우드 동기화가 없다. 에이전트가 로컬 CLI를 호출해 메모를 관리한다.

앱 소스 개발·테스트를 요청받은 경우에만 [개발 문서](https://github.com/egoing/agent-gibbon#빌드와-실행)를 읽는다. 개발 테스트에는 해당 워크트리의 `scripts/agent-gibbon-dev`를 사용해 설치 앱의 메모와 분리한다.
