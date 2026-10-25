# Floatnote

에이전트가 CLI로 갱신하는 작은 플로팅 Markdown 메모. Tauri 2 + Rust + 시스템 WebView로 동작하며 웹 서버나 클라우드를 사용하지 않습니다.

## 지원 기능

- ID로 구분하는 여러 프레임리스 메모 창, 본문 전체 드래그와 가장자리 크기 조절
- 창별 배경색 지정과 읽기 쉬운 흑백 글자색 자동 선택
- 본문 우측 aA·연필·복사·워프·핀·닫기 버튼, 제목바·하단바 없는 슬림 화면
- 메뉴 막대의 메모 아이콘에서 열린 메모를 앞으로 가져오거나 개별 메모 복원
- 버튼과 크기 조절 가장자리를 제외한 창 전체를 드래그하여 이동, 복사 버튼으로 표시된 본문 전체 복사
- 내용에 맞춰 자동 크기 조절(가로 300 고정, 세로 48~420 논리 픽셀), 본문 12px 기본값과 14·16·18px 전환·여백 3px
- 체크박스 행은 목록 점·들여쓰기 없이 표시, 일반 목록은 기존 형식 유지
- `open`으로 앱 자동 실행과 표시, `update`로 부분 갱신, `close`로 숨기기
- 새 메모와 숨긴 메모를 열면 커서가 있는 모니터에서 커서와 12 논리 px 여백을 둔 가까운 위치에 완전 불투명하게 표시하고, 최소 1초 대기한 뒤 마우스 이동이 감지되면 기존 배치 목적지까지 420ms cubic ease-out으로 이동
- 이미 보이는 메모의 `open`과 내용 갱신에는 등장 이동을 반복하지 않음
- 이동 중 숨기면 목적지를 보존하고, 직접 드래그하면 자동 이동을 중단
- 갱신 시 포커스·위치·숨김 상태와 사용자가 지정한 크기 유지, 명시적인 `open`으로만 복원
- 한글, 표, 코드, 읽기 전용 체크리스트 Markdown 지원
- 원시 HTML은 텍스트로 처리하고 sanitizer로 정리; 이미지·링크 이동·외부 리소스 차단
- CLI는 stdout에 JSON 한 개를 반환; 오류는 `ok: false`와 종료 코드 1

기존 창의 자동 재배치, 사용자 답변, 계정, 클라우드는 지원 범위에 포함하지 않습니다. 앱과 기본 사용 스킬은 작업계획 등 특정 워크플로우를 강제하지 않습니다.

## 글자 크기와 편집

- `aA`를 누르면 **12 → 14 → 16 → 18 → 12px**로 순환합니다. 제목·코드·표도 비례해서 커집니다. 크기는 창별로 유지되며 CLI 갱신이나 숨기기로 초기화되지 않습니다.
- 연필 버튼으로 Markdown 원문을 편집하고 **저장** 또는 **⌘S / Ctrl+S**로 반영합니다. **취소 / Escape**는 편집을 취소합니다. 편집 중에는 텍스트 선택과 입력이 가능하며, 창을 숨겨도 저장하지 않은 초안은 유지됩니다.
- `--file 경로`로 연 메모는 저장할 때 원본 파일도 갱신합니다. `--markdown`이나 `--file -`로 입력한 메모는 앱 안에서만 변경합니다. 저장 대상은 편집 화면에 표시됩니다.
- 제목·상태·색상만 CLI로 갱신하면 파일 연결을 유지합니다. `--markdown`이나 표준입력으로 본문을 교체하면 파일 연결을 해제하고, 다른 `--file`로 교체하면 새 파일에 연결합니다. 원본 파일을 자동 감시하지는 않습니다.
- 편집 시작 후 CLI가 메모를 갱신했거나 원본 파일 내용이 달라졌다면 저장을 거부하고 초안을 유지합니다. 필요한 내용을 복사한 뒤 취소하고 최신 내용으로 다시 편집하세요. 파일이 외부에서 바뀐 경우 CLI의 `update --id ID --file 경로`로 다시 읽습니다.
- 파일 저장은 같은 디렉터리의 임시 파일을 완성한 뒤 교체합니다. 읽기 전용·삭제된 파일이나 검증 오류는 기존 메모를 변경하지 않습니다. 저장 순간 다른 프로그램이 동시에 파일을 쓰는 상황까지 잠그지는 않으므로 동시 편집은 피하세요.
- 앱을 완전히 종료하면 저장하지 않은 초안과 글자 크기는 초기화됩니다. 기존 버전에서 열었던 메모는 새 버전에서 `--file`로 다시 열어야 원본 파일과 연결됩니다.

## 워프

- 좌우 화살표 아이콘을 누르거나 macOS에서 Alt(⌥ Option)를 누른 채 포인터를 노트에 올리면 현재 모니터의 반대편으로 이동한다. 노트 중심으로 좌우를 판정하고, 화면 가장자리에서 8px 간격과 처음 열린 세로 위치를 사용한다. 화면·크기가 바뀌면 작업 영역 안으로 위치를 제한한다.
- 크기·본문·편집 초안·고정·포커스를 유지한다. 다른 노트의 위치는 바꾸지 않으며 이동 목적지의 노트와 겹칠 수 있다. 노트가 화면보다 크면 이동하지 않고 오류를 표시한다.
- Alt 접촉은 다른 앱에 포커스가 있어도 동작한다. 이동 자체로 반복 실행되지 않으며, 포인터를 노트 밖으로 움직였다가 다시 들어오거나 Alt를 놓았다가 다시 누르면 재실행된다.
- 실행 후 노트 하단에 12초 동안 단축키 안내가 표시된다. ‘닫기’는 이번 안내만 닫고, ‘다시 보지 않기’는 모든 노트에 적용되어 앱 재실행 후에도 유지된다. 숨긴 노트에는 워프를 실행하지 않는다.


## 빌드와 실행

macOS에서 Rust stable, Node.js/npm, Xcode Command Line Tools 및 macOS SDK가 필요합니다. CPU 아키텍처는 호스트 기본값을 사용합니다.

```sh
npm ci
npm run build
./scripts/floatnote open --title '현재 작업' --status '진행 중' --file examples/progress.md
```

앱 번들: `src-tauri/target/release/bundle/macos/Floatnote.app`

CLI 실행 파일: `src-tauri/target/release/floatnote`

프로젝트의 `scripts/floatnote`는 release 실행 파일을 우선 사용하고 없으면 debug 실행 파일을 사용합니다. 다른 디렉터리에서도 절대 경로로 호출할 수 있습니다. Apple 배포 서명·공증은 설정하지 않았습니다. 다운로드한 배포판은 macOS 보안 확인에서 차단될 수 있습니다.

```sh
# 개발 모드: 별도 프론트엔드 서버가 필요하지 않습니다.
npm run dev
# 개발 실행 파일만 만들기
cargo build --manifest-path src-tauri/Cargo.toml
```

앱 아이콘은 `assets/icon.svg`를 바탕으로 Tauri CLI로 생성했습니다.

## CLI

```sh
./scripts/floatnote open --title '구현 작업' --status '진행 중' --file examples/progress.md
./scripts/floatnote update --status '검증 중'
printf '# 결과\n- [x] 구현 완료\n- [ ] 리뷰\n' | ./scripts/floatnote update --file -
./scripts/floatnote close
# 숨긴 상태에서 내용만 변경합니다.
./scripts/floatnote update --markdown '검증이 끝났습니다.' --status '완료'
./scripts/floatnote open
./scripts/floatnote inspect
./scripts/floatnote --help
```

여러 창은 ID로 구분합니다. ID를 생략하면 기존과 같은 기본 창 `main`을 사용합니다.

```sh
./scripts/floatnote open --id research --title '자료' --color '#fff1b8' --markdown '# 조사 메모'
./scripts/floatnote open --id build --title '구현' --color '#18304a' --markdown '# 구현 메모'
./scripts/floatnote update --id research --color '#dcecff'
./scripts/floatnote inspect --id build
./scripts/floatnote close --id research
./scripts/floatnote list
```

- ID는 영문·숫자·하이픈·밑줄 1~64자이며 대소문자를 구분합니다. 같은 ID의 `open`은 기존 창을 복원하고 다른 ID는 새 창을 만듭니다.
- `update`·`close`·`inspect`는 기존 ID만 받습니다. 없는 창은 오류이며 다른 창에 대신 적용하지 않습니다.
- 색상은 `open`·`update`의 `--color '#RRGGBB'`로 지정합니다. 기본값은 `#20252e`, 생략하면 기존 색상을 유지하며 응답은 소문자로 정규화됩니다. 빈 값·이름·알파 채널은 받지 않습니다. 기본색 복원은 `--color '#20252e'`입니다.
- 색상은 창 전체 배경에 적용되고 글자색은 대비가 높은 흑백으로 선택됩니다. 코드·버튼 배경도 함께 바뀝니다.
- 내용·색상·위치·크기·숨김·고정 상태는 창별로 독립적입니다. 새 창은 화면 오른쪽부터 배치하고 겹치지 않는 세로 위치를 무작위로 선택합니다. 오른쪽 열이 차면 왼쪽의 빈 열을 찾고, 현재 모니터에 공간이 없으면 다른 모니터도 확인합니다. 보이는 Floatnote 창 및 화면 가장자리와 8px 간격을 확보하며, 모든 모니터에 빈 공간이 없으면 겹치는 면적이 가장 작은 화면 내 위치에 배치합니다. 숨긴 창은 공간을 점유하지 않습니다. 기존 창을 다시 열 때도 목적지를 새로 계산하고, 이미 보이는 다른 창은 이동시키지 않습니다.
- `list`는 숨긴 창을 포함해 ID순으로 `id`, `title`, `status`, `color`, `visible`, `always_on_top`을 반환합니다. 본문은 개별 `inspect`로 확인합니다.

`--title`, `--status`, `--markdown` 또는 `--file`은 `open`/`update`에서 사용합니다. 생략한 필드는 보존하며 빈 문자열은 값을 비웁니다. `--markdown`과 `--file`은 함께 쓸 수 없습니다. 요청 전체는 JSON 인코딩 후 1MiB 이하, 제목과 상태는 각각 UTF-8 1024바이트 이하입니다.

체크리스트는 각 행을 `- [ ] 항목` 또는 `- [x] 완료 항목`으로 작성합니다. 목록 기호가 빠진 `[ ] 항목`은 `MARKDOWN_TASK_MARKER` 오류로 거부합니다. JSON의 `ok: false`, `error`에 행 번호·수정 예시를 반환하고 종료 코드는 1입니다. 기존 메모의 내용·제목·색상·상태는 변경하지 않으며, 호출자는 해당 행을 수정한 뒤 같은 요청을 재시도할 수 있습니다. 코드 블록·인라인 코드·정상 링크는 허용합니다. 일반 문장 전체를 검사하는 Markdown 린터는 아닙니다.

macOS에서는 마우스 포인터가 노트 밖에 있으면 창 전체가 50% 반투명이고, 노트 위에 있으면 완전 불투명해집니다. 창 포커스·위치·고정 상태는 바꾸지 않습니다. 포인터 진입/이탈과 주기적 동기화로 드래그·재열기 이후에도 복구되며 `inspect.opacity`에서 0.5 또는 1.0을 확인할 수 있습니다.

`inspect`는 실제 창의 가시성·포커스·위치·크기·상단 고정과 현재 메모를 JSON으로 반환합니다. 좌표와 크기는 물리 픽셀입니다. 성공 응답은 네이티브 처리 완료를 뜻하며 화면은 최대 약 350ms 뒤 갱신됩니다. 갱신 시각은 Unix epoch 밀리초로 반환하며 CLI 메타데이터에만 보관합니다. 제목·상태·시각은 본문 공간을 차지하지 않으며 필요한 제목은 Markdown 본문에 작성합니다.

앱이 실행되지 않았다면 `update`, `close`, `inspect`, `list`는 실패합니다. 자동 실행은 `open`만 수행합니다. 닫기 버튼·Escape·네이티브 닫기 요청은 프로세스를 종료하지 않고 창만 숨깁니다. 숨긴 뒤에도 메모 내용을 갱신할 수 있습니다. 직접 앱 번들을 실행하는 것도 명시적인 열기로 취급합니다. 메뉴 막대의 Floatnote 아이콘에서 **열린 메모 앞으로**를 선택하면 보이는 창만 앞으로 가져오며, 아래 개별 메모를 선택하면 숨긴 창도 복원합니다. 이 조작은 고정 상태와 위치를 바꾸지 않습니다.

`--file` 원본에 저장한 편집 내용 외의 메모·색상·글자 크기·위치·크기는 메모리에만 보관합니다. `close`는 삭제가 아니므로 숨긴 창도 같은 ID로 다시 열 수 있습니다. 프로세스를 완전히 종료하면 모든 창 상태가 초기화되며, 재부팅 후 자동 실행하지 않습니다. 종료가 필요하면 활성 상태 보기에서 Floatnote 프로세스를 종료할 수 있습니다. 구버전 앱이 실행 중이면 새 CLI의 ID·색상 요청을 처리할 수 없으므로, 필요한 메모를 보존한 후 프로세스를 종료하고 새 실행 파일로 다시 여세요. 시작 오류 진단은 `src-tauri/target/release/floatnote --serve`를 사용합니다.

## 로컬 통신과 안전한 렌더링

동일 사용자 전용 `/tmp/floatnote-<uid>/` 디렉터리(0700)의 Unix socket으로 요청을 전달합니다. 파일 잠금으로 중복 앱을 막고, 잠금을 획득한 프로세스만 이전 socket을 정리합니다. 별도 HTTP 서버나 포트는 없습니다. 같은 사용자 계정의 다른 프로세스는 CLI와 동일한 권한으로 메모에 접근할 수 있습니다. 연필 편집에서 저장을 누르면 연결된 `--file` 원본만 디스크에 갱신합니다. 별도의 메모 저장소는 만들지 않습니다.

Rust에서 Markdown을 HTML로 변환하고 정리한 뒤 WebView에 전달합니다. HTML 입력을 실행하지 않으며 체크리스트는 ☑/☐로 표시합니다. 이미지와 링크 이동은 지원하지 않습니다. CSP와 최소 창 조작 권한을 적용했습니다.

## 프로젝트 로컬 스킬

[Floatnote 사용 스킬](.agents/skills/floatnote/SKILL.md)은 에이전트가 실제 CLI의 창 생성·조회·갱신·색상 지정과 숨김 보존 규칙을 담고 있습니다. 작업계획 표시 같은 운영 정책은 사용자 `AGENTS.md`에서 정합니다. 예제는 [examples/progress.md](examples/progress.md)입니다.

## 검증

```sh
npm test
npm run check
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
node --check ui/app.js
python3 scripts/smoke.py
python3 scripts/multiwindow-smoke.py
python3 scripts/markdown-validation-smoke.py
# macOS 자동화/손쉬운 사용 권한이 있는 환경에서 UI 버튼 검증
python3 scripts/editor-ui-smoke.py
python3 scripts/macos-ui-smoke.py
python3 scripts/multiwindow-ui-smoke.py
python3 scripts/macos-tray-smoke.py
```

`multiwindow-smoke.py`는 별도 ID의 창 두 개로 독립 상태·색상·목록·잘못된 IPC·기본 창 호환성을 검사하고 테스트 창만 숨깁니다.

`smoke.py`는 실제 앱을 실행하거나 기존 앱의 메모를 교체하고 마지막에 숨깁니다. 포커스·위치·크기 보존, 반복 갱신, 닫은 뒤 갱신, 다시 열기, JSON 오류, 한글 체크리스트와 위험한 HTML 차단을 검사합니다. 실행 중 메모와 사용자 입력을 동시에 조작하지 마세요. `macos-ui-smoke.py`는 실제 WebView의 한글·체크리스트와 핀/숨기기 버튼을 macOS 접근성 API로 검증합니다. `multiwindow-ui-smoke.py`는 두 창의 본문과 고정/숨기기 버튼이 해당 창에만 적용되는지 확인합니다. `macos-tray-smoke.py`는 실제 메뉴 막대 아이콘으로 개별 숨긴 메모 복원과 열린 메모 앞으로 기능을 확인합니다. UI 스크립트는 macOS CLT의 Swift와 접근성 권한이 필요합니다. 화면 잠금을 해제하고 다른 앱에 포커스를 둔 상태에서 실행하세요. 본문 전체 드래그와 가장자리 크기 조절도 확인합니다.

환경과 수행 결과는 [검증 기록](docs/verification.md)에 기록합니다.

## 공식 참고 자료

- [Tauri macOS 사전 요구 사항](https://v2.tauri.app/start/prerequisites/)
- [Tauri Window API](https://v2.tauri.app/reference/javascript/api/namespacewindow/)
- [Rust WebviewWindow API](https://docs.rs/tauri/latest/tauri/webview/struct.WebviewWindow.html)
- [Tauri capabilities](https://v2.tauri.app/security/capabilities/)

## 배포판 제작과 설치

설치 파일은 [GitHub Releases](https://github.com/egoing/floatnote/releases)에서 받습니다. 비공개 저장소이므로 저장소 접근 권한이 필요합니다.

```sh
npm run build:release
```

호스트 CPU용 앱과 DMG를 `src-tauri/target/release/bundle/`에 만듭니다. v0.2.2은 Apple Silicon용 `dmg/Floatnote_0.2.2_aarch64.dmg`를 제공합니다. 기존 Intel Mac용 v0.1.0은 이전 릴리스에서 받을 수 있습니다. v0.2.2의 Intel 빌드는 제공하지 않습니다.

DMG를 열어 **Floatnote.app을 Applications로 복사**한 후 실행합니다. 이 배포판은 Apple Developer ID 서명·공증을 하지 않았습니다. 신뢰하는 출처에서 받은 파일인지 확인한 후 macOS의 **시스템 설정 → 개인정보 보호 및 보안 → 그래도 열기** 절차가 필요할 수 있습니다.

설치된 앱에 포함된 CLI도 사용할 수 있습니다.

```sh
/Applications/Floatnote.app/Contents/MacOS/floatnote open --markdown '# 안녕하세요'
```

새 버전으로 교체하기 전 필요한 메모를 저장하고 실행 중인 앱을 종료하세요. 메모·창 상태는 앱 종료 시 초기화됩니다.

커서 옆 대기와 등장 이동 중에는 창을 완전 불투명하게 표시하고, 도착 후 기존 포인터 기반 반투명 동작을 적용합니다. 표시 후 첫 1초 동안은 마우스가 움직여도 자동 이동하지 않습니다. 그동안 감지한 이동은 기억했다가 1초가 지나면 이동을 시작합니다. 마우스가 그대로 있으면 시간 제한 없이 대기합니다. CLI `open`은 표시 준비만 기다리고 반환하며 마우스 이동을 기다리지 않습니다. 응답의 `animating: true`는 대기 또는 이동 중이라는 뜻입니다.

커서가 화면 오른쪽 아래에 있으면 노트는 왼쪽 위를 우선합니다. 화면 작업 영역 안으로 보정한 뒤에도 커서를 가리지 않는지 검사하고, 음수 좌표와 서로 다른 화면 배율을 고려합니다. 창이 너무 크거나 커서 주변에 안전한 공간이 없으면 크기를 보존하고 기존 배치 목적지에 바로 표시합니다. 직접 드래그하면 자동 이동을 취소하며, 대기 중 숨기면 목적지를 보존합니다. 대기 중 내용 변경에 따른 자동 크기는 이동이 끝난 뒤 반영됩니다.

## 내용에 맞춘 자동 크기

새 패널은 내용을 측정한 크기로 커서 옆에서 등장합니다. 내용·글자 크기가 바뀌면 가로 300 고정, 세로 48~420 논리 픽셀 범위에서 다시 맞추며, 최대 높이를 넘는 본문은 스크롤합니다. 화면이 작으면 작업 영역 안으로 자동 크기를 제한합니다.

사용자가 가장자리를 드래그하면 해당 창은 앱을 종료할 때까지 수동 크기를 유지합니다. 이후 내용 갱신이나 숨김·복원으로 자동 크기를 덮어쓰지 않습니다. 수동 조절은 자동 범위 밖으로도 가능하며, 조작 버튼을 위한 하한 144×32 논리 픽셀만 적용하고 최대 크기는 제한하지 않습니다. 사용자가 크게 만든 공간은 그대로 유지합니다.
