# Floatnote

에이전트가 CLI로 갱신하는 작은 플로팅 Markdown 메모. Tauri 2 + Rust + 시스템 WebView로 동작하며 웹 서버나 클라우드를 사용하지 않습니다.

## 지원 기능

- ID로 구분하는 여러 프레임리스 메모 창, 얇은 상단 이동 영역·Option(Alt)+본문 드래그와 가장자리 크기 조절
- 창별 배경색 지정과 읽기 쉬운 흑백 글자색 자동 선택
- 본문 우측 핀·닫기 아이콘, 제목바·하단바 없는 슬림 화면
- 메뉴 막대의 메모 아이콘에서 열린 메모를 앞으로 가져오거나 개별 메모 복원
- 기본 300×240, 최소 160×80 논리 픽셀, 본문 12px·여백 3px
- 체크박스 행은 목록 점·들여쓰기 없이 표시, 일반 목록은 기존 형식 유지
- `open`으로 앱 자동 실행과 표시, `update`로 부분 갱신, `close`로 숨기기
- 갱신 시 포커스·위치·크기·숨김 상태 유지, 명시적인 `open`으로만 복원
- 한글, 표, 코드, 읽기 전용 체크리스트 Markdown 지원
- 원시 HTML은 텍스트로 처리하고 sanitizer로 정리; 이미지·링크 이동·외부 리소스 차단
- CLI는 stdout에 JSON 한 개를 반환; 오류는 `ok: false`와 종료 코드 1

기존 창의 자동 재배치, 사용자 답변, 계정, 클라우드는 지원 범위에 포함하지 않습니다. 앱과 기본 사용 스킬은 작업계획 등 특정 워크플로우를 강제하지 않습니다.

## 빌드와 실행

macOS에서 Rust stable, Node.js/npm, Xcode Command Line Tools 및 macOS SDK가 필요합니다. CPU 아키텍처는 호스트 기본값을 사용합니다.

```sh
npm ci
npm run build
./scripts/floatnote open --title '현재 작업' --status '진행 중' --file examples/progress.md
```

앱 번들: `src-tauri/target/release/bundle/macos/Floatnote.app`

CLI 실행 파일: `src-tauri/target/release/floatnote`

프로젝트의 `scripts/floatnote`는 release 실행 파일을 우선 사용하고 없으면 debug 실행 파일을 사용합니다. 다른 디렉터리에서도 절대 경로로 호출할 수 있습니다. 배포 서명·공증은 설정하지 않았으며 로컬 실행용입니다.

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
- 내용·색상·위치·크기·숨김·고정 상태는 창별로 독립적입니다. 새 창은 현재 모니터부터 작업 영역의 빈 공간을 찾고 다른 모니터도 확인합니다. 보이는 Floatnote 창 및 화면 가장자리와 8px 간격을 확보하며, 빈 공간이 없으면 겹쳐 띄우는 대신 오류를 반환합니다. 숨긴 창은 공간을 점유하지 않습니다. 기존 창의 명시적 복원은 원래 위치를 유지하며 다른 창을 재배치하지 않습니다.
- `list`는 숨긴 창을 포함해 ID순으로 `id`, `title`, `status`, `color`, `visible`, `always_on_top`을 반환합니다. 본문은 개별 `inspect`로 확인합니다.

`--title`, `--status`, `--markdown` 또는 `--file`은 `open`/`update`에서 사용합니다. 생략한 필드는 보존하며 빈 문자열은 값을 비웁니다. `--markdown`과 `--file`은 함께 쓸 수 없습니다. 요청 전체는 JSON 인코딩 후 1MiB 이하, 제목과 상태는 각각 UTF-8 1024바이트 이하입니다.

`inspect`는 실제 창의 가시성·포커스·위치·크기·상단 고정과 현재 메모를 JSON으로 반환합니다. 좌표와 크기는 물리 픽셀입니다. 성공 응답은 네이티브 처리 완료를 뜻하며 화면은 최대 약 350ms 뒤 갱신됩니다. 갱신 시각은 Unix epoch 밀리초로 반환하며 CLI 메타데이터에만 보관합니다. 제목·상태·시각은 본문 공간을 차지하지 않으며 필요한 제목은 Markdown 본문에 작성합니다.

앱이 실행되지 않았다면 `update`, `close`, `inspect`, `list`는 실패합니다. 자동 실행은 `open`만 수행합니다. 닫기 버튼·Escape·네이티브 닫기 요청은 프로세스를 종료하지 않고 창만 숨깁니다. 숨긴 뒤에도 메모 내용을 갱신할 수 있습니다. 직접 앱 번들을 실행하는 것도 명시적인 열기로 취급합니다. 메뉴 막대의 Floatnote 아이콘에서 **열린 메모 앞으로**를 선택하면 보이는 창만 앞으로 가져오며, 아래 개별 메모를 선택하면 숨긴 창도 복원합니다. 이 조작은 고정 상태와 위치를 바꾸지 않습니다.

메모·색상·위치·크기를 메모리에만 보관합니다. `close`는 삭제가 아니므로 숨긴 창도 같은 ID로 다시 열 수 있습니다. 프로세스를 완전히 종료하면 모든 창 상태가 초기화되며, 재부팅 후 자동 실행하지 않습니다. 종료가 필요하면 활성 상태 보기에서 Floatnote 프로세스를 종료할 수 있습니다. 구버전 앱이 실행 중이면 새 CLI의 ID·색상 요청을 처리할 수 없으므로, 필요한 메모를 보존한 후 프로세스를 종료하고 새 실행 파일로 다시 여세요. 시작 오류 진단은 `src-tauri/target/release/floatnote --serve`를 사용합니다.

## 로컬 통신과 안전한 렌더링

동일 사용자 전용 `/tmp/floatnote-<uid>/` 디렉터리(0700)의 Unix socket으로 요청을 전달합니다. 파일 잠금으로 중복 앱을 막고, 잠금을 획득한 프로세스만 이전 socket을 정리합니다. 별도 HTTP 서버나 포트는 없습니다. 같은 사용자 계정의 다른 프로세스는 CLI와 동일한 권한으로 메모에 접근할 수 있습니다. 메모 내용은 디스크에 저장하지 않습니다.

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
# macOS 자동화/손쉬운 사용 권한이 있는 환경에서 UI 버튼 검증
python3 scripts/macos-ui-smoke.py
python3 scripts/multiwindow-ui-smoke.py
python3 scripts/macos-tray-smoke.py
```

`multiwindow-smoke.py`는 별도 ID의 창 두 개로 독립 상태·색상·목록·잘못된 IPC·기본 창 호환성을 검사하고 테스트 창만 숨깁니다.

`smoke.py`는 실제 앱을 실행하거나 기존 앱의 메모를 교체하고 마지막에 숨깁니다. 포커스·위치·크기 보존, 반복 갱신, 닫은 뒤 갱신, 다시 열기, JSON 오류, 한글 체크리스트와 위험한 HTML 차단을 검사합니다. 실행 중 메모와 사용자 입력을 동시에 조작하지 마세요. `macos-ui-smoke.py`는 실제 WebView의 한글·체크리스트와 핀/숨기기 버튼을 macOS 접근성 API로 검증합니다. `multiwindow-ui-smoke.py`는 두 창의 본문과 고정/숨기기 버튼이 해당 창에만 적용되는지 확인합니다. `macos-tray-smoke.py`는 실제 메뉴 막대 아이콘으로 개별 숨긴 메모 복원과 열린 메모 앞으로 기능을 확인합니다. UI 스크립트는 macOS CLT의 Swift와 접근성 권한이 필요합니다. 화면 잠금을 해제하고 다른 앱에 포커스를 둔 상태에서 실행하세요. 얇은 상단 영역 또는 Option(Alt)+본문 드래그와 가장자리 크기 조절도 확인합니다.

환경과 수행 결과는 [검증 기록](docs/verification.md)에 기록합니다.

## 공식 참고 자료

- [Tauri macOS 사전 요구 사항](https://v2.tauri.app/start/prerequisites/)
- [Tauri Window API](https://v2.tauri.app/reference/javascript/api/namespacewindow/)
- [Rust WebviewWindow API](https://docs.rs/tauri/latest/tauri/webview/struct.WebviewWindow.html)
- [Tauri capabilities](https://v2.tauri.app/security/capabilities/)
