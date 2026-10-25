# Floatnote 검증 기록

검증일: 2026-10-10 (Asia/Seoul)

## 편집 기능 재검증 (2026-10-14)

- `npm test` 26개(Rust 21 + Node 5), Clippy, Rustfmt, JS/Python 문법과 diff 검사를 다시 통과했습니다.
- 재사용 가능한 `scripts/editor-ui-smoke.py`를 추가했습니다. 최신 UI·Rust 구현과 동일함을 파일 비교로 확인한 독립 macOS 앱에서 실행했습니다. 테스트는 고유 ID의 메모 두 개와 임시 Markdown 파일만 사용하고 테스트 창을 숨긴 뒤 원래 앱 포커스를 복원합니다.
- 실제 UI 통과: 4단계 글자 크기, 주변 버튼과 동일한 크기, 창별 독립 설정, 갱신·숨김 후 크기 유지, 한글 원본 저장·체크박스 렌더링, 취소, 숨긴 초안 유지, CLI/원본 파일 충돌 방지, Markdown 오류 교정 후 재저장, 파일 연결 해제 후 빈 메모 저장, 위치·크기·고정 상태 보존.
- 첫 실행에서 테스트 자동화의 입력 포커스 누락으로 교정 문자가 반영되지 않았습니다. 입력란에 포커스를 주고 실제 입력값을 확인하도록 테스트를 수정한 뒤 전체가 통과했습니다. 앱 코드 변경은 필요하지 않았습니다.
- 기존 운영 프로세스와 메모는 유지했습니다. 테스트 인스턴스만 종료했습니다. 단축키의 실제 키보드 입력과 전체 트레이 회귀는 이번 재검증 범위에 포함하지 않았습니다.
- Conf 5 / Risk Medium.

## 글자 크기 4단계·연필 편집 (2026-10-14)

- Intel x86_64, Node v24.21.0, Cargo 1.97.1에서 Rust 21개 + Node 5개, Clippy, Rustfmt, JS 문법 및 diff 공백 검사를 통과했습니다. release 앱 번들도 빌드했습니다.
- `aA`는 12·14·16·18px를 순환합니다. 주변 버튼과 동일한 19px 버튼, 14px SVG, 1.7 선 두께를 사용합니다. 본문 크기를 바꿔도 아이콘 크기는 유지됩니다.
- 기존 프로세스의 메모를 보존하기 위해 임시 소스 복사본에서 IPC 디렉터리와 앱 식별자만 분리한 debug 인스턴스를 만들고 실제 macOS 접근성 API로 버튼을 조작했습니다. 실제 4단계 순환, 연필에서 한글 Markdown 편집·원본 파일 저장·렌더링 반영, 취소, CLI 갱신 충돌, 원본 파일 변경 충돌을 확인했습니다.
- 단위 테스트는 파일 저장 성공·삭제된 파일·잘못된 체크리스트·오래된 편집 거부, 파일 연결 유지/해제, 메타데이터 유지와 빈 본문 저장을 검사합니다. UI 로직 테스트는 폴링 중 초안 유지, 오류 메시지 유지, 저장·취소·Escape, 편집 영역 드래그 방지를 검사합니다.
- 파일은 동일 디렉터리의 임시 파일을 완성하고 교체합니다. 파일 내용 비교 후 교체 사이의 외부 쓰기까지 직렬화하지는 않습니다. 기존 inode의 확장 속성·ACL·하드링크 관계는 보존하지 않으며 일반 텍스트 파일 편집을 대상으로 합니다.
- 기존 실행 앱은 이전 버전으로 유지했습니다. 새 빌드는 재실행 후 적용되며 기존 버전의 메모는 원문/파일 경로가 없어 자동 이관하지 않았습니다. 기존 전체 드래그·트레이 회귀 스크립트는 이번에 재실행하지 않았습니다.
- Conf 5 / Risk Medium: 실제 파일을 변경하는 기능이며 일반 파일 저장과 충돌 처리를 실환경에서 확인했습니다.

## Markdown 오류 안내·포인터 반투명

- 행 첫머리의 `[ ]`, `[x]`, `[X]` 뒤에 공백 또는 줄 끝이 오면 목록 기호 누락으로 감지합니다. CLI와 서버 모두 수정 전에 검증하며 `MARKDOWN_TASK_MARKER`·행 번호·수정 예시를 JSON error 문자열로 반환합니다. 코드 블록·인라인 코드·정상 링크·이스케이프는 제외합니다.
- Rust 19개 + Node 2개, Clippy, release 빌드, JS/Python 문법, 스킬 검증 통과. 스킬 검증기의 PyYAML 누락은 격리된 임시 가상 환경에 설치해 해결했습니다.
- `scripts/markdown-validation-smoke.py`: 실제 CLI 종료 코드 1, 행 번호·교정 예시, raw Unix socket 요청 거부, 모든 메모 필드 보존, 수정 후 성공, 코드 블록 허용 통과.
- macOS 창 전체의 네이티브 alpha를 사용합니다. `scripts/macos-opacity-smoke.swift 'Floatnote — markdown-opacity-plan'`에서 실제 포인터를 창 밖→안→밖으로 이동하고 CoreGraphics 창 정보로 alpha `0.5 → 1.0 → 0.5`를 확인했습니다. 테스트 후 포인터를 원래 위치로 복원합니다.
- 새 release 실행 시 기존 메모 30개의 위치·크기·핀·숨김을 복원했습니다. 다른 호출자가 다시 입력했던 목록 기호 없는 체크 항목도 발견해 복원 과정에서 표준 체크리스트 문법으로 정정했습니다.
- `scripts/multiwindow-ui-smoke.py`도 최신 빌드에서 통과했습니다. 실제 전체 창 드래그·복사·클립보드 복원·핀·숨김·크기 조절·포커스 보존에 회귀가 없었습니다.
- 기본 사용 스킬에 올바른 문법, 오류 후 수정·재시도, 반복 실패 시 중단 절차를 추가했습니다. 이 검증은 Markdown 전체의 문법 검사가 아니라 실제 발생한 체크리스트 실수에 한정합니다.
- Conf 5 / Risk Low. 반투명 동작은 macOS에서 검증했으며 다른 OS는 현재 이 효과를 적용하지 않습니다. 공식 근거: [NSWindow alphaValue](https://developer.apple.com/documentation/appkit/nswindow/alphavalue).

## 오른쪽 우선·세로 무작위 배치

- 새 창은 현재 화면의 오른쪽 여백 8px부터 배치하며, 기존 창과 겹치지 않는 모든 세로 좌표 중 무작위 선택합니다. 오른쪽 열이 차면 왼쪽 빈 열을 사용합니다. 기존 창 복원은 다시 배치하지 않습니다.
- Rust 17개 + Node 2개, Clippy, release 앱 빌드 통과. 오른쪽 정렬·허용 세로 구간·배치 변화·충돌 회피를 단위 테스트로 확인했습니다.
- 최신 release를 실행하고 실제 창 8개를 순차 생성해 물리 x=2584의 오른쪽 정렬과 서로 다른 y 좌표(206, 972, 280, 1146, 762, 1226, 908, 1298)를 확인했습니다. 네이티브 이동 반영 후 inspect로 측정했습니다. open 직후 응답 좌표는 이벤트 처리 타이밍에 따라 일시적으로 이전 좌표일 수 있습니다.
- 다중 창 CLI 회귀 테스트 통과. 노란색·파란색 테스트 창을 실제 실행했습니다. 기존 숨김/표시·고정·크기를 복원했으나 화면 밖에 있던 기본 창의 y=-154는 macOS에 의해 y=50으로 보정됐습니다.
- Conf 5 / Risk Low. 실제 다중 모니터 검증은 수행하지 않았습니다.

## 창 전체 드래그·본문 복사 — 후속 변경

본문과 빈 공간에서 마우스를 누르면 창을 드래그합니다. 버튼과 크기 조절 영역은 제외하며 텍스트 선택 대신 복사 아이콘으로 렌더링된 본문 전체를 복사합니다. Tauri 공식 clipboard-manager 플러그인의 쓰기 권한만 허용합니다.

- `npm test`: Rust 16개 + Node 2개 통과. `npm run check`와 release 빌드 통과.
- 실제 본문/빈 공간 드래그, 버튼 클릭 시 위치 보존, 복사 내용의 창별 격리 검증을 `scripts/multiwindow-ui-smoke.py`에 추가했습니다. 테스트는 클립보드의 모든 표현을 임시 보관하고 사용자가 중간에 다른 내용을 복사하지 않았을 때 복원합니다.
- 화면 잠금 해제 후 최신 release를 적용했습니다. 보이는 기존 창 6개의 위치·크기·고정 상태를 비교해 보존을 확인했습니다. `scripts/multiwindow-ui-smoke.py`에서 실제 본문/빈 공간 드래그, 본문 복사와 클립보드 복원, 버튼 클릭 시 위치 유지, 창별 핀·숨김 격리, 모서리 크기 조절, 배경 포커스 보존을 모두 통과했습니다.
- `scripts/macos-tray-smoke.py`도 최신 빌드에서 통과했습니다. 아래 화면 캡처를 최신 복사·핀·닫기 아이콘이 있는 실제 창으로 교체했고 본문이 버튼을 피해 줄바꿈되는 것을 확인했습니다.
- 최신 변경 Conf 5 / Risk Low. 공식 API: [Clipboard](https://v2.tauri.app/plugin/clipboard/).

## 슬림 UI·신규 창 배치·메뉴 막대 복원 — 이전 빌드 검증

사용자 후속 요청을 같은 브랜치와 PR #4에 반영했습니다. 아래 최초 제출 시점의 화면 잠금 보류는 이번 실환경 검증으로 해소했습니다.

| 검증 | 결과 |
|---|---|
| `npm test` | Rust 16개 + Node 2개 통과 |
| 배치 단위 테스트 | 빈 위치 유지, 기존 창 간격, 창 사이 빈 공간, 음수 모니터 좌표·배율, 공간 부족, 반복 생성 충돌 검사 통과 |
| 체크박스 렌더링 | 점·들여쓰기 없는 체크 행, 일반/혼합/중첩/느슨한 목록 구분과 HTML 안전성 통과 |
| Clippy / Rustfmt / JS·Python 문법 / diff 공백 | 통과 |
| `npm run build` | Intel x86_64 release 앱·번들 생성 성공 |
| `scripts/smoke.py` | 기본 창 CLI 회귀 통과 |
| `scripts/multiwindow-smoke.py` | 실제 신규 창끼리 및 기존 보이는 창과 겹치지 않음, 기본 창 호환·색상·목록·숨김·포커스·재열기 통과 |
| `scripts/macos-ui-smoke.py` | 기본 창의 실제 본문·핀·숨기기 버튼·배경 갱신 통과 |
| `scripts/multiwindow-ui-smoke.py` | 두 WebView 본문 격리, 핀·숨기기 독립 동작, 얇은 상단 영역 드래그와 모서리 크기 조절, 배경 포커스 보존 통과 |
| `scripts/macos-tray-smoke.py` | 실제 메뉴 막대 아이콘 클릭, 백그라운드 조회·상태 갱신 중 메뉴 유지, 열린 메모 앞으로가 숨긴 창을 열지 않음, 선택한 숨긴 메모만 복원하며 위치·크기·고정 유지 확인 |
| 실제 화면 캡처 | 체크 행의 점 제거, 제목·하단바 제거, 핀·닫기 옆 줄바꿈과 아래 전체 너비 사용 확인 |

![실제 슬림 메모와 버튼을 피한 줄바꿈](slim-ui.png)

기본 창은 300×240 논리 픽셀, 최소 160×80, 본문 12px/1.4, 자간 -0.025em, 패딩 3px입니다. 사용자 조작으로 더 작게 줄인 실제 창도 확인했습니다. 기존 창의 이동·크기·고정 상태는 이후 갱신에서 유지합니다.

처음 실환경 검사에서 신규 창의 `open` 응답이 네이티브 이동 이벤트 처리 전의 좌표를 반환하는 문제를 발견했습니다. 실제 배치는 올바르지만 JSON 좌표가 이전 값이었으며, `open` 성공 후 별도 IPC 차례에서 실제 프레임을 다시 조회하도록 고쳐 회귀 테스트를 통과했습니다. UI 테스트는 변경된 접근성 계층에 맞게 본문 그룹에서 버튼을 찾고, 메뉴 막대는 실제 마우스 입력으로 검증합니다. 입력 후 포인터와 원래 앱 포커스를 복원합니다. 메뉴는 창 생성 또는 제목 변경 때만 갱신해 백그라운드 조회·상태 갱신이 사용자가 연 메뉴를 닫지 않도록 했습니다.

새 창은 보이는 Floatnote 창만 피합니다. 현재 모니터부터 각 모니터 작업 영역을 검사하고 8 논리 픽셀 간격을 확보하며, 공간이 부족하면 오류를 반환합니다. 기존 창 복원은 위치를 보존합니다. 다른 앱의 창은 배치 장애물에 포함하지 않습니다. 다중 모니터·서로 다른 DPI는 좌표 단위 테스트 및 구현 검토를 했으나 실제 다중 모니터 환경에서는 검증하지 않았습니다.

- Conf 5: 이 Intel Mac에서 실제 앱·CLI·WebView·메뉴·포인터 입력으로 확인했습니다.
- Risk Medium: 다중 창 배치·CLI 응답·앱 상태에 영향을 주며 로컬에서 되돌릴 수 있습니다.
- 공식 API 확인: [Monitor 작업 영역과 배율](https://docs.rs/tauri/latest/tauri/window/struct.Monitor.html), [System Tray](https://v2.tauri.app/learn/system-tray/).

## 다중 창·색상 최초 제출 시점 — 이슈 #3

현재 Floatnote 폴더의 `3-multiwindow-color` 브랜치에서 구현했습니다. 아래 초기 프로토타입 기록과 구분합니다.

| 검증 | 결과 |
|---|---|
| 기준 테스트 | 변경 전 Rust 4개 통과 |
| `npm test` | Rust 9개 + Node 2개 통과. ID/색상 검증, 거부 시 원자성, 기본 창 호환, 중복 옵션, 라벨 충돌 및 4,096개 RGB 샘플의 본문·코드/버튼 배경 대비 검사 |
| `npm run check` | Clippy 모든 타깃, 경고를 오류로 처리하여 통과 |
| Rustfmt / JS / Python 문법 / diff 공백 | 통과 |
| `npm run build` | Intel x86_64 release 실행 파일과 Floatnote.app 생성 성공 |
| `python3 scripts/smoke.py` | 기존 기본 창 CLI 회귀 통과 |
| `python3 scripts/multiwindow-smoke.py` | 실제 창 2개의 독립 내용·색상·목록·반복 갱신·숨김·재열기·위치/크기/포커스 상태 보존, 잘못된 raw IPC 거부, 기본 창 호환 통과 |
| `python3 scripts/multiwindow-ui-smoke.py` | 실행했으나 미완료: macOS 화면 잠금으로 System Events가 창을 조회하지 못함(exit 1, -1728). 버튼 격리와 실제 렌더링은 미검증 |
| 실화면 색상·드래그·리사이즈 | 이번 확장에서는 미검증. 초기 프로토타입 결과를 새 다중 창 검증으로 간주하지 않음 |

첫 빌드는 로컬 npm 의존성 미설치로 `tauri: command not found`(127)였으며 `npm ci` 후 성공했습니다. UI 테스트 작성 중 AppleScript 문자열의 Unicode 이스케이프 오류를 고쳤으나, 수정 후 실행은 화면 잠금 때문에 중단됐습니다. CoreGraphics 세션 정보로 잠금 상태를 확인했고, 잠금 해제는 자동 수행하지 않았습니다.

실제 CLI의 네이티브 상태는 검증했지만, 잠긴 세션에서는 사용자가 보는 포커스·색상과 버튼 동작을 최종 확인할 수 없습니다. 구버전 프로세스에서 신버전으로 전환할 때 창 위치가 기본값으로 초기화됐으며, 잠금 때문에 접근성 API로 이전 위치를 복원하지 못했습니다. 이는 프로세스 재시작 시 상태를 초기화하는 기존 정책입니다.

수동 확인: 화면 잠금을 해제한 뒤 다른 앱에 포커스를 두고 `python3 scripts/multiwindow-ui-smoke.py`를 실행합니다. 이어 서로 다른 ID로 밝은 색과 어두운 색 창을 열어 배경과 글자 가독성을 확인하고, 각각 드래그·리사이즈한 뒤 다른 창의 `update`가 위치나 크기를 바꾸지 않는지 확인합니다. 테스트용 창은 종료 시 숨겨집니다.

- Conf 5(실제 네이티브 CLI 검증), Risk Medium(다중 창 상태와 CLI 계약 변경). GUI 미검증 범위는 위와 같습니다.
- 메모·색상·위치의 영구 저장 및 프로세스 간 자동 복원은 범위 밖입니다.
- 창 생성과 권한 범위는 [Tauri WebviewWindowBuilder](https://docs.rs/tauri/2.12.2/tauri/webview/struct.WebviewWindowBuilder.html)와 [Capabilities](https://v2.tauri.app/security/capabilities/)를 확인했습니다.

## 초기 프로토타입 기록

다음은 확장 전 단일 창 버전의 검증 기록입니다.

## 환경

| 항목 | 확인 결과 |
|---|---|
| macOS | 15.8 (24H23) |
| CPU / 실행 파일 | Intel x86_64 / Mach-O 64-bit x86_64 |
| Rust / Cargo | 1.97.1 |
| Node.js / npm | 24.21.0 / 11.19.0 |
| Tauri Rust / CLI | 2.12.2 / 2.12.1 |
| 개발 도구 | Command Line Tools와 macOS SDK 설치됨 |
| 전체 Xcode | 미설치 (`xcodebuild -version` 실패) |
| origin | egoing/floatnote, private 확인 |

전체 Xcode 없이 설치된 CLT/SDK로 데스크톱 release 빌드와 `.app` 번들 생성에 성공했습니다. CLT 경로는 `/Library/Developer/CommandLineTools`입니다. Clippy는 누락되어 rustup으로 설치한 뒤 사용했습니다.

## 수행 결과

| 검증 | 결과 |
|---|---|
| Rust 단위 테스트 | 4개 통과: 안전한 Markdown, 부분 갱신, 거부된 요청, 잘못된 CLI 옵션 |
| Clippy | 모든 타깃, 경고를 오류로 처리하여 통과 |
| Rustfmt / JS 문법 | 통과 |
| release 빌드 / 앱 번들 | x86_64 성공, 앱 번들 약 11.83MiB |
| 실제 CLI | 자동 실행, JSON 응답/오류 종료 코드, 연속 갱신, 숨김, 재열기 통과 |
| 창 상태 보존 | 갱신 전후 실제 네이티브 위치·크기·포커스·가시성·상단 고정 비교 통과 |
| 실제 WebView | 접근성 트리에서 한글 제목·본문·☑/☐·갱신 시각 확인 |
| 앱 수명 / 시작 포커스 | 새 자동 실행과 별도 명령 세션에서 동일 프로세스 유지, Orca 포커스 보존 확인 |
| 실제 버튼 | 상단 고정 켜기/끄기, 닫기 버튼, 닫은 뒤 update로 숨김 유지 확인 |
| 마우스 드래그 | 제목 영역 드래그로 물리 좌표 (1540,348) → (1620,408) 변화 확인 |
| 마우스 크기 조절 | 우하단 드래그로 물리 크기 760×920 → 860×1000 변화 확인 |
| 화면 검토 | 아래 앱 창 캡처로 텍스트·체크리스트·상태·버튼 표시 확인 |
| 로컬 스킬 | 스킬 검증기 통과, 사용 예제 CLI 실행 확인 |

![실제 macOS WebView 화면](floatnote.png)

## 재현

```sh
npm ci
npm run build
npm test
npm run check
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
node --check ui/app.js
python3 scripts/smoke.py
python3 scripts/macos-ui-smoke.py
```

UI 검증은 macOS 자동화/손쉬운 사용 권한이 필요하며 실행 중 다른 앱에 포커스를 둡니다. 두 스크립트 모두 검증용 메모로 교체하고 마지막에 숨깁니다. 드래그·크기 조절은 실제 CGEvent 마우스 입력으로 별도 확인했습니다. 마우스 위치는 입력 후 복원하고 검증 뒤 Orca로 포커스를 돌렸습니다.

## 범위와 한계

- 확인한 실제 환경은 이 Intel Mac 한 대입니다. Apple Silicon, Linux, Windows 검증은 수행하지 않았습니다. 첫 버전의 IPC는 Unix 전용입니다.
- 배포 서명·공증·DMG·다른 Mac 설치 검증은 수행하지 않았습니다. 로컬 앱 번들만 생성했습니다.
- 메모 내용·창 위치는 프로세스 수명 동안만 유지됩니다. 영구 저장·재부팅 후 자동 실행은 없습니다.
- 체크리스트는 읽기 전용이며 이미지·링크 이동·원시 HTML 실행은 지원하지 않습니다.
- Conf 5: 이 환경에서 실제 앱과 CLI를 실행해 확인했습니다. Risk Low: 로컬 단일 메모와 개발 파일 범위이며 쉽게 되돌릴 수 있습니다.


## 2026-10-14 워프 기능

- 좌우 위치 계산: 현재 모니터의 작업 영역·8 논리 px 여백·최초 세로 위치, 음수 모니터 원점, 화면 밖 높이 보정, 과대 창 거부 단위 테스트 통과.
- 접촉 상태: Alt 없는 접촉 무시, 이동 직후 반복 방지, 실제 포인터 이탈 후 재진입·Alt 재입력으로 재실행 테스트 통과.
- UI: 아이콘·접촉 후 안내 표시, 닫기·다시 보지 않기, 다른 노트/재로딩에서 설정 유지, 실패 시 초안 유지 테스트 통과. Rust 23개 + Node 7개, Clippy 경고 없음, 릴리스 app 빌드 성공.
- 실제 Intel macOS에서 별도 식별자·소켓을 가진 QA 앱으로 아이콘 좌우 왕복, 수동 이동 후 최초 높이 복원, 다른 앱 포커스 중 Alt 접촉, 반복 접촉, 연속 튕김 방지, 초안·크기·고정·본문 유지, 숨긴 노트 미이동을 확인했다. 실제 QA 앱 종료·재실행 후에도 ‘다시 보지 않기’가 유지됨을 확인했다.
- `scripts/warp-ui-smoke.py`: 새 테스트 프로필의 격리 앱에 `FLOATNOTE_TEST_BIN`과 `FLOATNOTE_TEST_PROCESS`를 지정한다. macOS 접근성 권한과 Swift 컴파일러가 필요하다. 고유 테스트 노트만 만들고 종료 시 숨기며 Option 키를 해제하고 이전 앱 포커스를 복원한다. 안내 설정을 변경하므로 사용자 앱에서 실행하지 않는다.
- 기존 편집 UI 실환경 회귀 검증도 전체 통과했다(글자 크기·파일 저장·초안·충돌·검증 오류 교정·파일 연결 해제).
- 워프는 다른 노트와의 겹침을 허용한다. 여러 DPI/모니터 간 실제 이동과 화면보다 큰 창의 GUI 오류는 실기기에서 재현하지 않았으며 계산 테스트로 검증했다. 일반 실행 중인 사용자 앱과 메모는 재시작하지 않았다.
- API 근거: [Tauri Monitor 작업 영역](https://docs.rs/tauri/latest/tauri/window/struct.Monitor.html), [NSEvent modifierFlags 바인딩](https://docs.rs/objc2-app-kit/latest/src/objc2_app_kit/generated/NSEvent.rs.html).


## 2026-10-14 v0.1.0 Intel Mac 배포판

- `npm test`(Rust 23 + Node 7), `npm run check`, Rust 포맷·JS 문법·diff 검사 통과.
- `npm run build:release`로 app과 `Floatnote_0.1.0_x64.dmg` 생성. DMG 약 3.60 MiB, Apple Developer ID 서명·공증 미설정.
- `hdiutil verify` 통과 후 읽기 전용 마운트하여 버전 0.1.0, x86_64 실행 파일, Applications 링크 확인. 마운트된 앱 내부 CLI `--help` 정상 실행 후 볼륨 해제.
- 다운로드·격리 속성이 붙은 다른 Mac에서의 최초 실행 및 Apple Silicon 실행은 검증하지 않았다. 체크섬은 배포 자산 `SHA256SUMS.txt`에 제공한다.

## 2026-10-23 패널 등장 이동

- arm64, rustc 1.91.1 환경에서 개발 빌드와 `cargo clippy --all-targets -- -D warnings` 통과.
- `git diff --check` 통과. 새 debug CLI로 기존 앱의 메모 조회 성공.
- 새 창·숨긴 창은 해당 모니터 작업 영역 중앙에서 목적지까지 420ms cubic ease-out으로 이동하도록 구현.
- 실행 중인 앱의 다른 작업 메모를 보존하기 위해 재시작하지 않음. 새 앱의 실제 애니메이션, 드래그 중단, 숨김 복원, 다중 모니터 동작은 미검증.

### 실제 앱 재검증

- 별도 임시 소스 복사본의 IPC 경로만 분리하여 기존 앱·메모를 유지한 채 새 앱 실행.
- 최초 실행에서 macOS 물리 좌표 1px 반올림을 사용자 이동으로 오인해 중앙에서 중단되는 결함 재현. 2px 오차 허용 및 배치 계산 목적지 직접 전달로 해결.
- 실제 IPC 조회: 0.002초 (1748, 938) → 0.203초 (3206, 418) → 0.444초 (3480, 320), cubic ease-out 이동 확인.
- 이동 중 opacity 1.0, 도착 후 포인터가 밖일 때 0.5 확인.
- 이동 도중 숨기기·다시 열기 후 목적지 (3480, 320) 보존 확인.
- 테스트 앱 창은 사용자 확인을 위해 유지. 기존 앱은 재시작하지 않음. 직접 드래그와 다중 모니터는 미검증.

## 2026-10-23 내용 크기와 혼잡 화면 배치

- 자동 크기: 가로 240~480, 세로 48~420 논리 픽셀. 새 창은 WebView에서 실제 내용을 측정한 뒤 배치·등장한다.
- 실제 테스트 앱의 짧은 체크리스트는 240×100 논리 픽셀(480×200 물리 픽셀)로 표시됐고, 캡처에서 하단의 큰 여백이 사라진 것을 확인했다.
- 긴 본문은 480×420 논리 픽셀(960×840 물리 픽셀) 상한을 지켰다.
- 실제 연결 모니터에서 16개 창까지 겹침 없이 배치했고 17번째 창은 모든 화면에서 빈 공간을 찾지 못해 최소 겹침 대안으로 표시됐다. 임시 테스트 창은 숨기고 예시 패널만 유지했다.
- Rust 기존 테스트 23개 통과. 개발 빌드·Clippy·JavaScript 문법·diff 공백 검사 통과.
- 수동 크기는 자동 범위 밖으로 허용하며 하한은 144×32, 상한은 없다. 드래그 시작 시 자동 조절 중단을 구현했다. GUI 자동화의 합성 드래그로는 실제 크기 변화가 확인되지 않아 수동 드래그와 갱신 후 유지의 실환경 검증은 미완료다.
- 기존 앱과 메모를 보존하기 위해 별도 IPC 경로의 임시 테스트 빌드를 사용했다. 실제 설치 앱 교체와 커밋은 하지 않았다.

## v0.2.0 배포 범위

- 사용자 승인에 따라 변경 사항 커밋 후 Apple Silicon(arm64)용 앱·DMG를 빌드하고 비공개 GitHub 저장소에 릴리스한다.
- 기존 v0.1.0 Intel Mac 릴리스는 보존한다. 이번 버전의 Intel 실행 검증과 빌드는 포함하지 않는다.
- Apple Developer ID 서명·공증은 기존과 동일하게 제공하지 않는다. 실행 중인 기존 앱과 메모는 자동 교체·재시작하지 않는다.

### Apple Silicon 배포 서명

최초 앱 번들 검사에서 링커 서명만 남아 리소스 봉인이 없는 상태를 발견했다. `bundle.macOS.signingIdentity`를 `-`로 명시해 Tauri가 번들 전체에 애드혹 서명을 하도록 변경하고 앱·DMG를 다시 생성한다. Developer ID 인증·공증을 의미하지 않는다.

### 배포 빌드 확인 결과

- v0.2.0 arm64 앱·DMG 생성 완료. 앱 번들의 `codesign --verify --deep --strict` 통과.
- DMG 이미지 무결성 검사 통과. 읽기 전용으로 마운트한 DMG 안 앱도 서명 검증을 통과했고 버전 0.2.0, 빌드된 앱과의 바이너리 SHA-256 일치, CLI `--help` 실행을 확인했다.
- 다운로드 검증용 SHA256SUMS.txt를 배포 파일과 함께 제공한다.

## 2026-10-25 폭 통일·복원 배치·중앙 대기

- 자동 폭을 300 논리 픽셀로 통일하고 세로 크기만 내용에 맞춘다. 사용자가 조절한 크기는 유지한다.
- 숨긴 창을 다시 열 때 목적지를 재계산하며, 이동 중인 다른 패널의 최종 위치도 점유된 자리로 계산한다.
- 중앙에서 3초 대기 후 420ms cubic ease-out으로 이동한다. 대기 중에도 불투명도를 1.0으로 유지한다. CLI open은 이동 완료 상태를 조회할 때까지 기다린다.
- 별도 IPC의 실제 테스트 앱에서 짧고 긴 본문 모두 폭 600 물리 픽셀(배율 2에서 300 논리 픽셀), 중앙 대기 중 위치 불변과 opacity 1.0, 이동 후 연속 재열기 비중첩을 확인했다.
- 개발 빌드·Clippy·JavaScript 문법·diff 검사 통과. 검증 후 로컬 설치 앱을 교체했다. 커밋·GitHub 배포는 하지 않았다.

- 로컬 앱 빌드·애드혹 서명 후 /Applications/Floatnote.app을 교체하고 재실행했다. 직전 메모 72개의 내용·원본 파일 연결·숨김 상태를 복원·대조했으며 설치 앱과 빌드 바이너리 SHA-256 일치를 확인했다. 이전 앱과 메모 백업은 ~/Library/Application Support/Floatnote/backups/restart-20261025-012550/에 보존했다.

### v0.2.1 배포 준비

자동 폭 통일·중앙 3초 대기·재열기 목적지 재계산을 v0.2.1로 배포한다. Apple Silicon용 앱·DMG에 애드혹 서명을 적용하고, 설치 후 메모 내용·원본 연결·숨김 상태를 복원한다.

- v0.2.1 Rust 23개·Node 7개·Clippy·포맷·문법 검사 통과. Apple Silicon 앱·DMG 생성 후 앱 서명, DMG 무결성, 내부 앱 버전 0.2.1과 빌드 바이너리 SHA-256 일치를 확인했다. CLI --help 실행도 통과했다.

## 2026-10-25 본문 변경 알림 — 이슈 #5 / PR #6

- 기준 `ff0712c4421364ff9a60ce132123ddafb01be8d8`, red `cd2b6a2a930b1eeda2519ae2e15b18629ead9d82`. 최초 기준 트리에서 회귀·타입·린트·빌드를 실행했고 모두 통과했습니다. 기존 실패 제외는 0건입니다.
- 환경: Apple Silicon arm64, Rust/Cargo 1.91.1, Node 24.21.0, npm 11.19.0, Tauri CLI 2.12.1. 기존 사용자 앱은 종료·교체하지 않았습니다.
- 요구사항 테스트 4개가 `change_html`/삭제 표시 부재로 실제 실패한 뒤 테스트 전용 red 커밋을 push했습니다. 원본 `/tmp/floatnote-is2-5/red.log`. 기존 red 테스트의 assertion은 유지했습니다.
- 신규 게이트: `cargo test --manifest-path src-tauri/Cargo.toml feedback` 및 `node --test --test-name-pattern=feedback tests/editor.test.mjs`. 회귀: `npm test`. 타입: `cargo check --manifest-path src-tauri/Cargo.toml --all-targets`. 린트: `npm run check`, Rustfmt, JS/Python 문법, diff 공백. 빌드: `npm run build`.
- 단위 검증: 변경 행·분리된 변경 사이 유지 행, 체크리스트·표·여러 줄 코드, 삭제-only·빈 본문·초기 표시, 메타데이터·동일 본문, HTML 안전성, 편집 저장, 큰 본문의 제한된 비교, 창별 기한·연속 갱신·만료, revision 변경 없는 강조 해제와 초안 유지.
- 실환경 격리: 작업 경로의 `src-tauri/target/is2-instance`에 현재 소스를 복사하고 **IPC 디렉터리와 앱 식별자만** 변경해 별도 debug 실행 파일을 빌드했습니다. 기존 앱의 IPC `/tmp/floatnote-<uid>`와 구분되는 `/tmp/floatnote-is2-5-<uid>`를 사용했습니다. UI와 기능 소스는 같으며 이 격리 변경은 배포 소스에 포함하지 않습니다.
- 실제 macOS/CLI 통과: CoreGraphics PID·좌표로 테스트 창을 식별하여 전면 순서와 alpha 확인, 다른 앱 포커스·좌표·크기·핀 유지, 다른 창 alpha 불변, 마지막 실제 변경부터 3초 복귀, 오래된 기한이 새 알림을 끊지 않음, 메타데이터/동일 본문 무연장, 숨김 중 갱신·재열기에서 이전 효과 없음, 빈 본문 삭제 효과 만료. 원본 `/tmp/floatnote-is2-5/native-feedback-cli.log`.
- UI 한계: osascript 접근성 권한 오류(-1728), CoreGraphics 창 제목 비공개, Orca 연결에서도 해당 프로세스 창 목록이 비어 있어 **강조 색상/삭제 안내의 실제 화면·복사 버튼·편집/워프/수동 크기 UI·핀 해제 상태의 전면 순서**는 이번 환경에서 확인하지 못했습니다. 초기 UI 실행 실패 로그는 `/tmp/floatnote-is2-5/native-feedback.log`. 제목 대신 PID·좌표로 native 순서를 검증했으며 UI assertion은 `FLOATNOTE_TEST_UI=1`로 별도 보존했습니다. UI 미검증을 통과로 계산하지 않습니다.
- 재현: 격리 인스턴스에 `FLOATNOTE_TEST_BIN`, `FLOATNOTE_TEST_PROCESS`, 필요시 `FLOATNOTE_TEST_PID`를 지정하고 `python3 scripts/update-feedback-smoke.py`를 실행합니다. 접근성 권한이 있는 환경은 `FLOATNOTE_TEST_UI=1`, 스크린샷 권한도 있으면 `FLOATNOTE_TEST_SCREENSHOT=/tmp/feedback.png`를 지정합니다. 기존 `editor-ui-smoke.py`, `warp-ui-smoke.py`도 같은 격리 바이너리 환경변수로 실행할 수 있습니다.
- 원본 baseline/final 명령·종료 코드는 `/tmp/floatnote-is2-5/baseline.json`, `final.json`, 각 `baseline-*.log`, `final-*.log`에 있습니다. 최초 baseline은 덮어쓰지 않았습니다.
- 사전/최종 Risk Medium: 창 순서와 native/UI 상태 상호작용. 자동 병합하지 않고 사전 리뷰로 제출합니다. Conf 5는 위에 명시한 실제 네이티브 동작 범위에 한정하며 UI 미검증 범위는 별도입니다.
- 공식 API 근거: [Tauri WebviewWindow](https://docs.rs/tauri/latest/tauri/webview/struct.WebviewWindow.html), [NSWindow orderFrontRegardless](https://developer.apple.com/documentation/appkit/nswindow/orderfrontregardless()). 기존 포커스 비탈취 전면 배치 경로를 재사용했습니다.

### is2 커버리지 대조

2026-10-25 실사용, egoing/floatnote #5 / PR #6, Codex CLI 0.164.0(현재 담당 세션), Orca 1.4.239. 스킬 커버리지 표와 대조한 이번 실행의 증거는 **Medium 프런트엔드·백엔드 작업 → 자동 병합 보류 → Ready PR** 경로입니다. red/green·5종 게이트·실제 native 검증·독립 감사의 결과를 PR에 기록합니다. 새 터미널 준비는 반복하지 않았고 기존 base ref를 유지했습니다. 권한 제한 UI는 미검증입니다. 독립 감사가 동일 본문 저장 시 만료된 강조가 재등장하는 결함을 발견했고, 재현 테스트를 추가해 실패를 기록한 뒤 1회 수정 재시도를 수행했습니다. mock은 실제 모델의 동일 본문 저장 동작에 맞게 보완했고 기존 assertion은 유지했습니다. 의심 발동·rebase 충돌·자동 병합·롤백 경로는 이번에 검증하지 않았습니다. 공유 스킬 원본의 유지보수는 작업 범위가 아니므로 원본 표를 수정하지 않고 이 프로젝트 기록을 남깁니다.

- 감사 후 보완: 저장 직후에는 원본 렌더링을 표시하고, 다음 snapshot의 실제 효과 상태로 강조 여부를 결정합니다. 추가 테스트가 실제 실패한 원본은 `/tmp/floatnote-is2-5/audit-red.log`, 보완 후 5종 게이트 원본은 `retry1.json`, `retry1-*.log`입니다. 앞선 final 로그는 이전 시도 증거로 그대로 보존했습니다.

## 2026-10-25 후속 요청 — 커서 확인까지 변경 강조 유지

사용자 요청: diff는 도입하지 않고 행 강조를 유지하며, 커서가 노트에 닿거나 다음 변경이 올 때 이전 강조를 지웁니다. 3초 불투명 효과는 유지합니다.

- 불투명 기한과 미확인 강조 상태를 분리했습니다. 3초가 지나도 강조·삭제 안내는 유지되고, 커서 진입/노트 내부 이동으로 현재 표시된 변경을 확인합니다. 이미 커서가 노트 위에 있을 때 갱신되면 이후 움직임으로 확인합니다.
- 다음 실제 본문 변경은 직전 본문과의 비교 결과로 강조를 교체합니다. 메타데이터/동일 본문은 유지합니다. 숨김은 불투명 기한만 종료하고 미확인 강조는 유지하며, 숨긴 상태의 변경도 새 강조로 교체하되 창은 열지 않습니다.
- 확인 명령은 창·변경 revision에 묶여 늦게 도착한 이전 확인이 새 변경을 지우지 않습니다. UI는 이미 확인한 revision의 늦은 snapshot도 무시합니다. 확인은 불투명 기한을 줄이지 않습니다.
- 후속 기준은 검증된 `141c6acbff193a150c21ed51fb92e1d5e68c4b89`이며 최초 is2 base/red/attempts ref와 baseline을 보존합니다. `3ef1700`에서 변경된 요구사항의 실제 red를 기록했습니다. 기존 3초 후 강조 해제 assertion을 사용자 요청에 맞게 커서 확인 방식으로 교체했고 초안·메타데이터 보존 assertion은 유지했습니다.
- 5종 게이트 PASS: 원본 `/tmp/floatnote-is2-5/persistent.json`, `persistent-{new,regression,typecheck,lint,build}.log`. Rust 30개, Node 10개. 추가 stale snapshot assertion 및 스모크 문법도 재확인합니다.
- 새 IPC `/tmp/fn-persist-5-<uid>`와 앱 식별자로 검증 인스턴스를 분리했습니다. 기존 사용자 앱 및 앞선 시연 노트는 종료하지 않았습니다.
- 실환경 커서 입력 시도는 실패했습니다. `CGEvent.post`로 좌표 (5,40)을 요청했으나 실제 (1123,669)가 유지되어 입력 미전달을 확인했습니다. `persistent-native.log`의 커서 확인 실패는 이 환경 제한이며 실제 커서 확인 통과로 보고하지 않습니다. 스모크는 이후 요청/실제 좌표를 비교하여 미전달만 SKIP으로 분리하고, 입력이 전달되면 강조 해제 assertion을 반드시 실행합니다.
- 실제 창 순서·3초 alpha 및 강조 상태 유지·숨김/재열기/빈 본문은 `persistent-native-partial.log`에 기록합니다. 강조 색상/커서 접촉/편집·복사·워프 UI 실환경 제한은 남아 있습니다. 후속 변경의 Conf 3(기능 경계조건 단위 검증), Risk Medium이며 자동 병합하지 않습니다.
- 커버리지 대조: 사용자 요구사항 변경에 따른 정당한 테스트 변경, 불투명·확인 상태 분리, stale 확인/snapshot 처리의 실사용 검증을 추가했습니다. 공유 스킬 원본은 수정하지 않습니다.
## 2026-10-25 커서 근처 등장 — 이슈 #7

- 기준 SHA `ff0712c4421364ff9a60ce132123ddafb01be8d8`에서 baseline을 캡처했다. `npm test`(Rust 23 + Node 7), `cargo check --all-targets`, Clippy·Rustfmt·JS 문법·diff 검사, `npm run build` 모두 통과했으며 제외한 기존 실패는 없다.
- 환경: Apple Silicon arm64, Rust/Cargo 1.91.1, Node 24.21.0, npm 11.19.0, Tauri Rust 2.12.2. 원본 로그는 `/Users/egoinglee/Library/Logs/floatnote-is2-7`에 보존한다.
- TDD red: `scripts/entrance-smoke.py`를 실제 기준 release 앱의 격리 사본에서 실행했다. `open`이 3.589초 동안 대기하여 2.5초 반환 상한 assertion이 실패했다. red ref는 `refs/is2/red/7`에 보존한다.
- 실제 앱 검증은 release 실행 파일 사본의 IPC 디렉터리 문자열만 같은 길이의 `/tmp/floatnot7-`로 교체하고 애드혹 서명한 `floatnote-qa7` 프로세스에서 수행했다. 테스트가 만든 프로세스만 종료하며 사용자 앱을 재시작·교체하지 않았다. 테스트는 고유 ID 메모만 만들고 커서·전면 앱을 복원한다.
- 최초 green: Rust 27 + Node 7, 타입·린트·빌드 통과. 실제 커서 오른쪽 하단에서 왼쪽 위 여백, 3.6초 이상 위치 불변과 opacity 1.0, 즉시 CLI 반환, 이미 보이는 창의 open/갱신 유지, 마우스 이동 후 목적지 이동·opacity 0.5, 지연된 자동 크기 적용, 숨김/복원/취소/창 독립성 통과.
- 추가 드래그 검증은 최초 AppleScript Unicode escape 오류, 1차 교정 후 접근성 권한 거부(-1728), CLI 프레임 조회로 바꾼 2차 실행에서는 드래그 최종 위치 assertion 실패를 기록했다. assertion을 약화하지 않았으며 별도 작업자에게 에스컬레이션했다.
- 독립 감사에서 테스트 완화·실행 결과 퇴행·스코프 이탈은 미발동이었다. 다른 배율의 모니터에서 대기하는 창의 목적지 예약 크기 결함(P2)을 발견해 보완 대상으로 기록했다.
- 실제 다중 모니터·혼합 배율 장비 검증은 수행하지 않았다. 단위 테스트와 API/소스 검토 범위를 실제 장비 검증으로 간주하지 않는다.
- 좌표 API 근거: [Tauri WebviewWindow](https://docs.rs/tauri/latest/tauri/webview/struct.WebviewWindow.html). 설치된 Tao 0.37.1의 macOS cursor_position, monitor position/size, window outer_size 구현을 함께 확인했다.

### 에스컬레이션 보완과 남은 실환경 검증

- 목적지의 물리 크기를 함께 저장하여 대기 모니터의 배율과 무관하게 예약 영역을 유지한다. 목적지 2x/커서 1x 및 반대 방향을 단위 테스트로 검증한다.
- 본문 드래그는 백엔드에서 등장 상태를 취소한 뒤 시작한다. 수동 크기 조절도 현재 위치를 유지하며 취소한다. 숨김·워프에 사용하는 목적지 완료 처리와 구분한다. UI 이벤트 단위 테스트에서 본문 드래그 요청과 편집/버튼/보조 버튼 제외를 확인한다.
- `CGPreflightPostEventAccess() == false`로 합성 버튼 이벤트 권한 부재를 확인했다. `scripts/macos-input.swift`는 입력 전 이를 검사하고 `INPUT_UNAVAILABLE`과 종료 코드 77을 반환한다. 커서 위치 이동은 가능했지만 클릭·드래그가 전송되지 않아 기존 실패를 실제 드래그 결함 재현으로 해석하지 않는다.
- 보완 후 Rust 28 + Node 9, 타입·린트·release 앱 빌드와 기존 red 실앱 스크립트 전체가 통과했다. 기존 red 테스트의 assertion은 바꾸지 않았다.
- 실제 버튼 드래그·수동 크기 조절은 **SKIP(입력 권한 없음)**, 실제 혼합 배율 다중 모니터는 **미검증**이다. 자동 병합을 보류하며 권한이 허용된 환경에서 `entrance-interaction-smoke.py`를 격리 앱으로 실행한 후 리뷰해야 한다. 기존 앱 설치·교체·재시작은 수행하지 않았다.

### 후속 요구: 최소 1초 대기

- 표시 직후 1초간 마우스가 움직여도 자동 이동하지 않는다. 이때 감지한 움직임은 기억했다가 1초 후 출발하며, 움직임이 없으면 계속 대기한다. 타이머는 네이티브 표시 완료 뒤 시작한다.
- 새 단위 테스트는 기존 코드에서 100ms 시점의 조기 출발로 실패했다(`hold-red.log`). 수정 후 999ms 대기·1000ms 출발·420ms 이동, 조기 이동 후 원래 포인터 위치로 돌아오는 경우를 포함해 Rust 29 + Node 9 테스트가 통과했다. 타입·Clippy·포맷·release 빌드도 통과했다.
- 격리 실앱에서 조기 포인터 이동 후 14회 연속 불투명·위치 불변 및 최소 대기 후 도착을 확인했다(`hold-live-pass.log`). 최초 테스트는 무작위 목적지가 커서 아래에 있어 opacity 1.0을 반환했다. 외부 포인터 반투명 검사 전에 커서를 노트 밖 화면 모서리로 옮기도록 테스트 조건만 바로잡고 assertion은 유지했다.
- 사용자 확인용 격리 테스트 앱을 별도 실행한다. 기존 설치 앱·노트는 그대로 유지한다. 실제 버튼 이벤트 권한 제약과 다중 모니터 미검증 범위는 이전과 같다.

## v0.2.2 Apple Silicon 릴리스

- 커서 근처 등장·최소 1초 불투명 대기·마우스 이동 후 배치·빠른 CLI 응답을 v0.2.2로 배포한다. 미병합 PR #6 변경은 포함하지 않는다.
- package.json·npm lock·Cargo package/lock·Tauri 버전을 0.2.2로 맞췄다.
- Rust 29 + Node 9, Clippy·Rustfmt·JS·diff 검사와 `npm run build:release`(arm64 app·DMG) 통과.
- DMG 무결성 검사, 읽기 전용 마운트 앱의 `codesign --verify --deep --strict`, 버전 0.2.2, arm64, 빌드 앱과 바이너리 바이트 일치, CLI `--help`를 확인했다. 검증 후 마운트를 해제했다.
- DMG와 SHA256SUMS.txt, 검증 메타데이터를 로컬 `~/Library/Application Support/Floatnote/releases/v0.2.2/`에 보관한다. 코드·게이트 원본 로그는 `~/Library/Logs/floatnote-is2-7/`에 보관한다.
- 애드혹 서명이며 Developer ID·공증 및 다른 Mac의 최초 설치는 미검증이다. 자동화 드래그·실제 다중 모니터 미검증 한계는 이전 기록과 같다.

## v0.2.3 통합·릴리스

- 사용자 명시 승인으로 PR #6 병합·push·릴리스·로컬 설치 및 해당 워크스페이스 삭제를 진행한다. Medium의 자동 병합이 아닌 사용자 승인 병합이다.
- 최신 main `dc8990e`의 커서 근처 등장·최소 1초 대기를 유지하며 통합했다. 충돌은 검증 문서의 두 기록을 모두 보존하여 해결했다. 모든 패키지 버전을 0.2.3으로 맞췄다.
- Rust 36 + Node 12, cargo check, Clippy·Rustfmt·JS 문법 통과. `npm run build:release`는 샌드박스에서 DMG 생성이 실패했고 권한을 허용한 동일 명령으로 app·DMG 생성이 완료됐다. 원시 실패 로그와 성공 로그를 모두 보존한다.
- 앱 및 DMG 내부 앱의 애드혹 서명, DMG 무결성, 내부 버전 0.2.3, 바이너리 바이트 일치와 CLI --help를 확인했다. Developer ID·공증은 제공하지 않는다.
- 최종 release 실행 파일에서 IPC 문자열만 같은 길이의 별도 namespace로 교체·애드혹 서명하여 native 격리 검증했다. 최소 1초 대기와 등장 회귀 모두 PASS. 등장 중 본문 변경은 독립적인 3초 불투명을 유지해야 하므로 기존 스모크의 즉시 0.5 기대를 1.0 및 기한 후 0.5 검사로 교체했다.
- `CGWarpMouseCursorPosition`은 실제 커서 위치를 이동시킬 수 있지만, `CGPreflightPostEventAccess`가 false이면 WebView에 합성 pointer 이벤트를 전달하지 못한다. 좌표 일치만으로 이벤트 전달을 단정했던 검사 실패를 보존했고, 권한 확인을 추가하여 이 경우만 SKIP한다. 실제 커서 접촉·강조 색상·편집/복사/워프/수동 크기 UI 한계는 남아 있다.
- 검증 로그는 `/tmp/floatnote-is2-5/release-*`, `entrance-hold-smoke-release.log`, `entrance-smoke-release.log`, `update-feedback-smoke-release.log`에 있으며 삭제 전 `~/Library/Logs/floatnote-is2-5/`로 보존한다. 원본 감사 ref와 시도 이력도 번들로 보존한다.
- 로컬 배포는 기존 메모·초안·위치·숨김 보존을 위해 앱 설치 파일을 교체하되 실행 중인 기존 프로세스는 재시작하지 않는다. 설치 버전은 다음 앱 재시작부터 적용된다. 이전 앱과 메모 snapshot을 로컬 private 백업에 남긴다.

## Sentry 오류 수집 (2026-10-25)

- macOS arm64, Node v24.21.0, Rust 1.91.1에서 확인했습니다.
- `npm test`: Rust 37개, Node 13개 통과. 오류 메시지의 민감정보 제거와 WebView의 고정 오류 종류 전달·IPC 실패 처리를 검증했습니다.
- `npm run check`, Rustfmt, JavaScript 문법 검사, `git diff --check` 통과.
- `npm run build`: macOS 앱 번들 생성 및 새 release CLI `--help` JSON 응답 확인.
- `egoing-personal` 개인 조직의 `investing-bot`과 같은 팀에 `floatnote` 프로젝트를 생성하고 공개 수신 DSN을 기본값으로 연결했습니다.
- 명시적 실제 전송 테스트 `telemetry::tests::live_sentry_delivery` 1개 통과. Sentry에서 `FLOATNOTE-1` (`Floatnote Sentry connection test`, 릴리스 `floatnote@0.2.3`, environment `development`) 이벤트 수신을 직접 확인했습니다: https://egoing-personal.sentry.io/issues/7811173579/
- DSN 반영 후 일반 테스트 50개 통과(실제 전송 테스트 1개는 기본 생략), Clippy·Rustfmt·앱 빌드를 다시 통과했습니다. 기존 앱 프로세스와 사용자 메모는 유지했으며, 새 번들은 빌드 산출물 경로에 생성했습니다.
- Conf 5(실제 SDK 전송·수신) / Risk Medium: 외부 서비스로 진단 이벤트를 전송합니다. 실제 WebView에서 오류를 유발하는 E2E는 수행하지 않았습니다.

## v0.2.4 변경 표시·마우스 확인 및 릴리스

- 추가는 녹색, 수정은 호박색, 삭제된 이전 행은 붉은색 배경과 취소선으로 표시한다. 별도 삭제 안내를 제거했다. 삭제 전후 텍스트는 안전한 Markdown 렌더링을 거치며 원문과 최종 HTML은 최신 내용만 유지한다.
- 비활성 WebView에서도 네이티브 포인터 좌표로 접촉을 확인한다. 확인은 포커스나 3초 불투명 기한을 바꾸지 않는다.
- 환경: Apple Silicon arm64, Rust 1.91.1, Node 24.21.0, Tauri CLI 2.12.1. Rust 37 + Node 12 테스트, Clippy, Rustfmt, diff 검사와 app·DMG 빌드 통과.
- 최종 v0.2.4 release 앱 사본의 IPC namespace만 격리하여 네이티브 검증했다. 전면 순서·불투명 기한·동일 본문/메타데이터 무효과·포커스/핀/위치 보존·숨김 갱신·빈 본문 삭제 표시 유지가 통과했다. 합성 입력 권한 없이 실제 커서 좌표 접촉으로 강조가 해제되고 기존 전면 앱과 포커스가 유지되는 것을 확인했다.
- 접근성 UI를 통한 핀 버튼·편집·복사 및 색상/취소선 픽셀 검증은 미실행이다. 다른 Mac·Intel·다중 모니터 검증은 수행하지 않았다.
- DMG 무결성, 내부 앱 애드혹 서명, 버전 0.2.4, 빌드 앱과 내부 바이너리 바이트 일치, CLI --help 통과. Developer ID 서명·공증은 제공하지 않는다.
- 사용자 승인으로 PR 병합·비공개 GitHub 릴리즈·로컬 설치를 진행한다. 실행 중인 기존 앱은 메모 보존을 위해 재시작하지 않으며 새 버전은 다음 시작부터 적용된다. 설치 전 메모 snapshot·이전 앱과 릴리즈 파일은 로컬 private 백업에 보존한다.
- 검증 로그: ~/Library/Logs/floatnote-v0.2.4/. 릴리즈·백업: ~/Library/Application Support/Floatnote/releases/v0.2.4/.

### Sentry 병합 검증

- 최신 `origin/main`의 v0.2.4를 통합했고 검증 기록의 양쪽 내용을 보존했습니다.
- 통합 상태에서 Rust 38개(실제 전송 테스트 1개 기본 생략)와 Node 13개, Clippy, Rustfmt, diff 검사와 v0.2.4 macOS 앱 빌드를 통과했습니다.


## Control 접촉 접기 (2026-10-25)

- 현재 Mac의 Apple 스티커 접힌 창을 화면과 Window Server 경계로 확인했다. 논리 크기 300×12, Retina 캡처 600×24였으며 메모의 접힌 높이도 12 논리 px로 맞췄다. 제목은 10px 한 줄과 말줄임, 오른쪽에는 펼치기 버튼만 표시한다.
- Control + 포인터 접촉으로 창별 접기·펼치기를 전환한다. 원래 논리 크기를 저장·복원하고 접힌 동안 자동 맞춤과 가장자리 크기 조절을 막는다. 본문·편집 초안은 유지한다.
- 포인터가 밖에 있을 때 접힘 불투명도 0.8, 펼침 0.5이며 기존 포인터 접촉·등장·변경 알림의 1.0 우선 규칙을 유지한다.
- Rust 36 + Node 13 테스트, Clippy, Rustfmt, JS 문법 및 app 빌드를 통과했다. Node 회귀 테스트는 접힘 중 CLI 갱신과 편집 초안·제목 표시·펼치기 버튼 복원을 확인한다.
- 소스 사본의 IPC 디렉터리만 별도로 지정한 임시 debug 앱에서 실제 macOS 입력으로 접기·펼치기, 중복 토글 방지, 12px 높이, 0.8/0.5 불투명도, 이전 크기 복원, 접힘 중 갱신, 펼치기 버튼 클릭과 다른 메모의 독립 상태를 확인했다. 임시 앱은 검증 후 종료했다. 검증 도구와 로그는 `/tmp/floatnote-fold-validation/`에 있다.
- 검증 입력 도구의 초기 시도는 등장 이동 중 좌표와 합성 Control 해제 플래그 때문에 실패했다. 등장 완료 후 좌표를 다시 읽고 해제 이벤트의 플래그를 비운 뒤 검증했다. 제품 코드의 성공으로 오인하지 않았다.
- 기존 설치 앱과 실행 중 메모는 보존했다. 실제 다중 모니터 배율 변경 중 접기·펼치기는 미검증이다. Conf 4 / Risk Low.


### 명시적 접기 버튼과 롤오버 안내

- 펼친 상태의 도구 모음에 위쪽 화살표 접기 버튼을 추가했다. 접히면 같은 버튼이 오른쪽 아래쪽 화살표와 ‘메모 펼치기’로 전환된다. 롤오버에는 Control을 누른 채 메모에 포인터를 올리는 단축키를 기본 툴팁으로 표시한다.
- Rust 36 + Node 14 테스트, Clippy·Rustfmt·JS 문법·diff 및 app 빌드 통과. Node 테스트는 버튼의 양방향 전환·접근성 이름·펼침 상태·단축키 툴팁 문구를 검증했다.
- `/tmp/floatnote-fold-buttons/`의 격리 앱에서 펼친 상태의 접기 버튼과 접힌 상태의 펼치기 버튼을 실제 클릭해 전환을 확인했다. 이전 크기 복원·12px 높이·불투명도·Control 전환 검증도 통과했다. 툴팁의 실제 화면 출현은 별도로 자동 검증하지 않았다.
- 새 미리보기 창을 실행하고 이전 미리보기만 숨겼다. 기존 설치본과 다른 사용자 메모는 유지했다.


## v0.2.5 접기 기능 통합·릴리스

- 사용자 명시 승인으로 접기 기능 커밋·최신 main 통합·PR 병합·릴리스·설치 및 folding 워크스페이스 삭제를 진행한다. v0.2.4의 변경 표시와 Sentry 통합을 보존했다. 문서와 CSS의 충돌은 양쪽 기능을 유지하도록 해결했다.
- 패키지·npm lock·Cargo package/lock·Tauri 버전을 0.2.5로 맞췄다. Apple Silicon arm64, Rust 1.91.1, Node 24.21.0 환경이다.
- 통합 후 Rust 38 + Node 15 테스트 통과, 실제 Sentry 전송 테스트 1개는 기본 생략했다. Clippy·Rustfmt·JS 문법·diff 검사 및 app·DMG 빌드를 통과했다.
- 최종 release 바이너리 사본에서 IPC 경로만 같은 길이의 별도 namespace로 바꾸고 애드혹 서명하여 격리 검증했다. 실제 접기/펼치기 버튼 클릭과 Control 접촉 전환·중복 토글 방지·12px 높이·0.8/0.5 불투명도·이전 크기 복원·접힘 중 갱신·창별 독립 상태를 확인했다.
- DMG 무결성, 내부 앱 서명·버전 0.2.5·빌드 바이너리 일치 및 CLI --help를 확인했다. 검증용 마운트는 해제했다. Developer ID 서명·공증·Intel·다중 모니터 배율 변경 및 툴팁 픽셀 자동 검증은 미수행이다.
- 설치는 기존 앱과 메모 snapshot을 로컬 private 경로에 백업하고 앱 파일을 교체한다. 기존 프로세스는 초안·위치·숨김 보존을 위해 유지하며 새 버전은 다음 재시작부터 적용한다.
- 릴리스 파일·이전 앱·메모 snapshot은 `~/Library/Application Support/Floatnote/releases/v0.2.5/`, 테스트 로그·격리 검증 도구는 `~/Library/Logs/floatnote-v0.2.5/`에 보존한다. 설치와 게시 확인 후 Orca CLI로 이번 folding 워크스페이스만 삭제한다.
- Conf 4 / Risk Medium: main에서 통합한 기존 Sentry 외부 오류 수집을 포함한다. 접기 기능 자체는 국소적이며 복구 가능하다.


## Agent Gibbon 이름 전환 (2026-10-25)

- 앱 표시명은 Agent Gibbon, 저장소·CLI·스킬 이름은 agent-gibbon으로 전환했다. GitHub 저장소의 private 상태를 확인했다. 이전 scripts/floatnote는 새 CLI로 연결하는 호환 진입점이다.
- 기존 메모와 설정을 위해 앱 식별자, Unix socket 디렉터리와 UI 설정 키를 유지했다. 새 Sentry 환경변수를 우선하고 이전 환경변수도 지원한다.
- Apple Silicon MacBook(Cargo 1.91.1)과 Intel iMac Pro(Cargo 1.97.1)에서 각각 Rust 38개 및 Node 15개 테스트, Clippy와 앱 빌드를 통과했다. 실제 Sentry 전송 테스트 1개는 기본 생략했다. 최신 Clippy가 발견한 기존 오류 전파 구문은 동작이 같은 `?` 구문으로 정리했다.
- 두 컴퓨터에서 release 바이너리 사본의 IPC 경로만 같은 길이의 별도 경로로 바꾸고 애드혹 서명하여 격리 검증했다. 새 프로세스 실행, 두 메모의 독립 내용, 숨김 갱신과 위치 보존, CLI 도움말을 확인했다. 검증 프로세스만 종료했다.
- 각 컴퓨터의 /Applications/Agent Gibbon.app을 설치하고 서명 검사 및 빌드 번들 내부 바이너리와 바이트 일치를 확인했다. 새 CLI와 호환 CLI의 도움말, 기존 프로세스 메모 조회도 통과했다. 기존 앱은 실행 상태로 보존했고 새 UI 이름은 다음 앱 시작부터 적용된다.
- 두 컴퓨터의 소스 파일 해시가 일치함을 확인했다. 저장소 실경로와 새 Orca 등록 경로는 ~/dev/project/agent-gibbon, 새 워크트리 기본 경로는 ~/orca/workspaces/agent-gibbon이다. Agents·Codex·Claude의 설치 스킬은 새 저장소의 스킬을 가리킨다.
- Orca 기존 등록은 'Agent Gibbon · 이전 대화'로 보존했다. 이전 경로는 호환 심볼릭 링크로 유지한다. 이전 대화 로그의 경로를 다시 쓰거나 기존 세션 등록을 삭제하지 않았다.
- 이번 검증에서 전체 버튼·트레이·접기·워프 UI 회귀 및 Developer ID 서명·공증, 새 DMG 배포는 수행하지 않았다. 로그는 각 컴퓨터의 ~/Library/Logs/agent-gibbon-rename-20261025/에 있다.
- Conf 4 / Risk Medium: 두 CPU 환경에서 실제 앱 검증을 수행했으며 경로·설치·Orca 등록 변경은 호환 경로와 기존 등록을 통해 복구 가능하다.


## v0.2.6 접촉 닫기 (2026-10-25)

- Control + Option + Command 접촉 시 닫기를 워프·접기보다 우선 처리한다.
- macOS Apple Silicon, Rust 1.91.1, Node 24.21.0에서 테스트 53개 통과, 기존 라이브 Sentry 테스트 1개 제외. Clippy와 앱·DMG 빌드 통과.
- 릴리즈 실행 파일의 IPC 경로만 같은 길이의 테스트 경로로 바꾼 임시 실행본을 재서명하여 기존 앱과 격리했다. 실제 CoreGraphics 키·포인터 입력으로 숨김, 접기 상태·위치·본문 보존을 확인했다. 숨긴 뒤 CLI 갱신 시 숨김 유지도 확인했다.
- 접힌 상태·편집 초안의 실제 접촉 검증은 별도로 실행하지 않았다. 설치는 기존 프로세스를 종료하지 않고 앱 번들만 교체하여 기존 메모를 보존한다.


## v0.2.7 두 CPU 배포·설치 (2026-10-25)

- 이름 전환을 main에 통합한 뒤 다른 작업에서 게시한 v0.2.6 접촉 닫기 변경도 보존했다. 버전 충돌을 피하기 위해 이번 Apple Silicon·Intel 통합 배포는 v0.2.7로 올렸다.
- 두 CPU에서 Rust 38개·Node 15개 테스트, Clippy, 앱 빌드와 격리 CLI 검증을 통과했다. 실제 Sentry 전송 테스트 1개는 생략했다. 격리 검증은 앱 시작·도움말·두 메모의 독립 내용·숨김 갱신·위치 보존을 확인했다.
- 두 DMG의 hdiutil 무결성, 내부 앱 버전 0.2.7·CPU 종류·애드혹 서명·빌드 번들과 바이너리 일치·CLI 도움말을 확인했다. 검증 마운트는 해제했다. Intel은 Finder 자동화가 대기하여 bundle_dmg.sh의 --skip-jenkins 옵션으로 아이콘 자동 배치를 생략했고 앱과 Applications 바로가기를 포함했다.
- 두 Mac에 /Applications/Agent Gibbon.app을 설치하고 설치본 서명·버전·바이너리 일치·기존 메모 CLI 조회를 확인했다. 기존 프로세스를 종료하지 않아 메모·초안·숨김을 보존했으며 새 버전은 다음 앱 실행부터 적용된다.
- 기존 앱과 메모 snapshot·설치 자산은 ~/Library/Application Support/Agent Gibbon/releases/v0.2.7/, 검증 로그는 ~/Library/Logs/agent-gibbon-v0.2.7/에 보존한다.
- Developer ID 서명·공증 및 전체 UI 회귀는 이번 배포에서 수행하지 않았다. Conf 4 / Risk Medium.
