# 텍스트 메모 입력 불가 조사 — #202 / 작업 24

## 빈 메모 후속 재현과 수정

사용자가 기존 글자는 편집할 수 있고 빈 메모에는 입력할 수 없다고 확인했다. 읽기 전용 CLI 조회에서 전용 개발 인스턴스의 영역 텍스트 메모가 표시·포커스 상태이고 원문 길이 0, 접힘 해제, 오류 안내 없음임을 확인했다. 실행 PID 27464는 이 워크트리의 debug 바이너리였다.

300×240 독립 Chromium 창에서 본문 내부 높이는 213px, 실제 편집 영역은 첫 줄 16.8px였다. 본문 아래쪽 75% 지점을 클릭한 뒤 실제 키 이벤트로 `first`를 입력하면 수정 전에는 본문이 빈 채였고 포커스는 스크롤 컨테이너에 있었다. 기존 글자를 직접 클릭하면 편집됐다. CSS의 백분율 최소 높이가 중간 래퍼의 자동 높이를 거치며 본문 끝까지 전달되지 않았다.

본문과 Milkdown 래퍼를 세로 flex로 배치해 남는 공간도 편집기가 차지하도록 했다. 수정 후 같은 지점에서 첫 글자 입력·명시 저장·저장된 Markdown 재로딩이 통과했다. 비활성 macOS WKWebView에서도 빈 공간의 hit-test 대상이 편집기이고 높이가 213px였으며 DOM 입력과 모의 저장이 성공했다. 브라우저 전체 편집 회귀 및 UI 198개 통과. 실제 사용자 앱의 OS 키입력·물리 한글 IME·Intel/Windows는 미검증이다. 사용자가 열어 둔 메모에는 입력하거나 재시작하지 않았다.

[Chromium 결과](assets/issue202/empty-click-chromium.json) · [WKWebView 결과](assets/issue202/empty-hit-webkit.json)

전용 캐시에서 `pnpm dev:build` 성공. `pnpm dev:info`와 바이너리 `--instance-info`의 root·instance·manifest_dir 일치, SHA-256 `8244fbe3be1943422af52f9f3e1f217fcb127e9e8b267b97f2db55c64059da86`. 빌드 로그: `/Users/egoinglee/Documents/Codex/2026-11-16/gibbon-prebuild/24/empty-fix-X5Ldvz/build.log`.

## 1차 조사 당시 상태

사용자 제보는 입력 불가이며, 원문 보호 안내나 원문 복사 표시는 보이지 않는다고 직접 확인했다. 원인은 아직 확정하지 않았고 제품 동작은 변경하지 않았다. 배포 작업 23은 중단 상태이며 이 기록은 배포 승인이나 장애 해결을 뜻하지 않는다.

## 조사 출처

- 루트: `/Users/egoinglee/.codex/worktrees/ba77/agent-gibbon`
- 브랜치: `codex/text-note-input-24`
- 기준 main/origin/main: `22dc7de7309f8d47772bb0cf1f2c88768f742ef8`
- 계획 커밋: `5954a63`; 이슈 #202, Draft PR #203
- Node 24.21.0, npm 11.19.0, pnpm 11.25.0, Rust 1.91.1, macOS ARM64
- 개발 인스턴스: `dev-e51a21fd91ea5adfa8ad977d`
- 전용 바이너리: `/Users/egoinglee/.codex/worktrees/ba77/agent-gibbon/src-tauri/target/debug/agent-gibbon`
- 독립 메모: `task24-input-fixture`, 고유 본문 `task24-ba77-input-fixture`

## 실행과 결과

| 검사 | 실제 결과와 한계 |
|---|---|
| `pnpm install --frozen-lockfile` | lockfile 변경 없이 의존성 설치 |
| `npm run test:ui` | 기존 UI 198개 통과 |
| `npm run dev:build` | 전용 macOS ARM64 debug 빌드 통과 |
| `PLAYWRIGHT_MODULE=/tmp/gibbon-playwright/node_modules/playwright/index.mjs GIBBON_SMOKE_OUTPUT=/tmp/gibbon24-browser node scripts/milkdown-browser.mjs` | Chromium 158.0.8127.18, 실제 전체 app/번들, native IPC만 모의. 키다운 경유 글자 입력·Enter/Backspace·DOM 포커스·명시저장·undo/redo·체크박스·충돌·숨김/접힘·선택서식·자동저장·저장 중 추가입력·한글 조합 이벤트 대기·미지원 원문 보호·66k 문서 편집 통과. 오류0·외부요청0. 물리 IME 미검증. [JSON](assets/issue202/chromium-results.json) |
| 독립 비활성 WKWebView fixture | 전용 번들 및 사용자 main에서 생성된 번들 각각 전체 app.js 초기화와 DOM `execCommand` 입력·자동저장 확인. 다른 앱 활성화/키 입력 주입 없이 수행. Tauri IPC 모의이며 실제 클릭/물리 IME 증거가 아니다. 임시 fixture `/tmp/gibbon24-webkit.swift`, `/tmp/gibbon24-fixture.html` |
| 전용 개발 CLI `open` / `inspect` | `ok:true`, instance/ID/본문 일치. 초기 PID11004, 진단 재시작 PID12001·12579·14163·15059의 exact 바이너리 경로 확인. 기존 사용자 앱은 종료/변경하지 않음. 전역 단축키는 다른 인스턴스 담당으로 유지 |
| 전용 실제 Tauri 초기화 진단 | 내용 비포함 임시 모듈 import probe에서 app.js/Milkdown import 성공, DOM ready complete, 전용 메모 에디터 생성·`contenteditable=true` 확인. [JSON](assets/issue202/native-initialization.json). 화면 목록 `[]`, Feedback ‘배치할 화면이 너무 작습니다’, 메모 `visible:false`로 실제 GUI/OS입력 검증은 불가. 입력 장애의 재현으로 판정하지 않음. probe 제품 코드는 제거함 |

기존 브라우저 스모크는 이미 제거된 `mousedown → drag_note` 계약을 검사해 중간 실패했다. 현재 툴바의 `pointerdown + 4px 이동 → chrome_drag` 계약으로 검사를 갱신해 전체 회귀를 실행했다. 이 검사 노후화는 사용자 입력 장애의 원인으로 판단하지 않는다.

## 1차 조사 당시 남은 검증

- 본문 클릭 시 입력 커서가 생기는지 사용자 실행 환경 확인
- 출처를 확인한 전용 메모에서 실제 OS 클릭·키입력·물리 한글 IME
- 직접 원인 재현과 최소 수정, 연결 원본 저장·재열기 회귀
- Intel/Windows 실제 실행 및 전체 플랫폼 release CI는 미실행

확신도는 독립 브라우저 회귀에 한해 Conf4이며, 사용자 장애 원인 판단은 Conf1이다. 편집·저장 상태에 영향할 수 있어 Risk Medium으로 유지한다.

## 실제 GUI 차례 확인

팀장에게 24번 전용 fixture의 GUI 차례를 배정받았지만 CUA `getApp`에 exact debug 바이너리 경로를 전달하면 `Invalid app`을 반환했다. 도구가 개발 실행 파일을 앱으로 선택하지 못하는 한계이며 자동승인 거부가 아니다. 직전 inspect에서 instance/표식 일치·visible false를 확인했고 클릭·OS 키입력은 보내지 않았다. 검증 전용 메모만 close하고 exact 경로를 재확인한 PID15059만 종료했다. 사용자 앱/기존 메모·전역 입력 담당은 보존했다.

## 사용자 확인이 필요한 최소 재현 절차

1. 입력 불가를 관찰한 앱/메모 종류가 플로팅 텍스트 본문인지 확인한다(관리창 상세는 현재 읽기 전용이다).
2. 기존 내용 수정 대신 같은 실행 인스턴스에서 새 독립 일반 텍스트 메모를 만든다.
3. 본문 클릭 후 입력 커서 표시 여부, 영문 한 글자/Backspace, 한글 조합 순서로 관찰한다.
4. 독립 메모의 저장·숨김 복원 후 내용 확인을 수행한다. 실제 원본 연결 검사는 독립 임시 파일로만 한다.
5. 문제 메모 내용은 수집하지 않고 원문 보호 표시·파일 연결 유무·사용자가 선택한 모드·실행 출처만 확인한다.

커서 표시 질문은 팀장이 전달했으나 답변이 없다. 수면 중 추가 질문·음성·사용자 앱 조작을 요구하지 않는다. 2026-11-16 01:06–07:06 KST 야간 작업 범위에서 Astra 재평가 후, 새로운 독립 근거가 없으면 재현 미확정으로 차례를 반환한다. 배포·제품 최종커밋·push·병합은 계속 승인 범위 밖이다.
