# Floatnote 검증 기록

검증일: 2026-10-10 (Asia/Seoul)

## 창 전체 드래그·본문 복사 — 후속 변경

본문과 빈 공간에서 마우스를 누르면 창을 드래그합니다. 버튼과 크기 조절 영역은 제외하며 텍스트 선택 대신 복사 아이콘으로 렌더링된 본문 전체를 복사합니다. Tauri 공식 clipboard-manager 플러그인의 쓰기 권한만 허용합니다.

- `npm test`: Rust 16개 + Node 2개 통과. `npm run check`와 release 빌드 통과.
- 실제 본문/빈 공간 드래그, 버튼 클릭 시 위치 보존, 복사 내용의 창별 격리 검증을 `scripts/multiwindow-ui-smoke.py`에 추가했습니다. 테스트는 클립보드의 모든 표현을 임시 보관하고 사용자가 중간에 다른 내용을 복사하지 않았을 때 복원합니다.
- 최신 UI 실환경 검증은 보류입니다. 화면 잠금(`CGSSessionScreenIsLocked = 1`) 때문에 접근성 창 조회가 실패했습니다. 실행 중인 이전 빌드와 기존 창 상태를 보존했으며 새 빌드는 아직 실행하지 않았습니다.
- 잠금 해제 후 기존 창 상태를 보존해 새 빌드를 실행하고 `python3 scripts/multiwindow-ui-smoke.py`로 확인합니다. 아래 화면 캡처 및 실환경 통과 기록은 복사 버튼 추가 전 버전입니다.
- 최신 변경 Conf 3 / Risk Low. 공식 API: [Clipboard](https://v2.tauri.app/plugin/clipboard/).

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
