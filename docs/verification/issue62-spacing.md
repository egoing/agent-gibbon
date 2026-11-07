# #62 후속: 공백 확정 전후 한글 자간

2026-11-07. 사용자가 배경 문제의 해결을 확인하고, 공백으로 단어를 확정하면 글자 간격이 넓어지는 현상을 제보했다. 배경 수정은 유지하면서 조합 중 표시의 폭을 터미널 격자에 맞췄다. 아래 실행 검증은 독립 WKWebView의 합성 입력이며 물리 키보드 확인과 구분한다.

## 원인과 변경

조합 중 표시에는 브라우저의 자연 글자 폭을 사용했다. 반면 확정 후 xterm은 현대 한글 음절·호환 자모를 두 셀에 배치하고 글자 간격을 보정한다. 같은 글꼴 이름과 크기만 지정해도 두 배치 방식의 폭은 일치하지 않는다. [xterm 6.0.0의 DOM 렌더러](https://github.com/xtermjs/xterm.js/blob/6.0.0/src/browser/renderer/dom/DomRendererRowFactory.ts)는 셀 폭과 글리프 실측 폭의 차이로 letter-spacing을 계산한다.

기존 코드의 WebKit 합성 입력에서 13px `한글`의 조합 폭은 22.5 CSS px, 확정 후 터미널 네 칸의 폭은 31.3333 CSS px로 차이를 재현했다. `안녕하세요`처럼 길어질수록 차이가 누적됐다. 이는 사용자가 관찰한 현상과 부합하는 재현이며, 제보 당시 앱·물리 키보드 세션을 계측한 결과는 아니다.

보완 경로의 각 글자를 별도 inline-block으로 만들고 화면 폭/열 수로 구한 셀 폭을 사용한다. 현대 한글 음절·호환 자모는 두 칸, ASCII는 한 칸이다. 이 어댑터의 기존 한글 범위에 대해서 번들 xterm의 기본 Unicode 6 폭 규칙을 따른다(선행 자모 두 칸, 결합 중·종성 자모 0칸, 확장 범위 기본 한 칸). 고어·복합 grapheme의 전체 동작 보장은 추가하지 않는다.

시작 위치와 줄 높이도 같은 화면 격자로 맞추고, 공개 `onRender` 알림에서 보류 중인 글자가 있을 때만 다시 표시한다. 글자 크기·리사이즈·비동기 출력으로 바뀐 위치를 따라가며 강제 확정이나 포커스 이동은 하지 않는다. 임시 단어 보관·PTY 전송 시점·취소/붙여넣기·정상 composition 이벤트의 xterm 위임은 유지한다. CSS는 보완 경로 클래스에 한정한다.

## 검증과 결과

| 검사 | 결과 |
|---|---|
| `node --test tests/terminal-ime.test.mjs` | 7개 통과. 기존 6개 + 혼합 폭/보류 중 격자·글자 크기 갱신 검증 |
| `npm test` | Rust 182개, UI 127개 통과 |
| `npm run check` | Clippy 통과 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | 통과 |
| `npm run dev:build` | 통과 |
| `swift scripts/terminal-ime-webkit.swift "$PWD" <출력폴더>` | 배경·단일 확정 전송·격자 위치와 실제 렌더 Range 위치 비교 20개 통과 |
| `git diff --check` | 통과 |
| `rsvg-convert docs/design/terminal-ime.svg -o docs/design/terminal-ime.png` | 렌더링·시각 확인 |

20개 조건: 글자 크기 13/14/16/18px × `한글`, `안녕하세요`, `하나`, `ㅎㅏ`, `한a글`. 14px는 47열, 나머지는 60열로 화면 폭/열 수의 분수 셀 폭도 검사했다. 각 조건에서 `> ` 뒤에 보완 입력을 표시하고, 공백 확정 후 실제 번들 xterm에 단일 전송 문자열을 echo해 공개 buffer의 셀 수 및 `.xterm-rows`의 실제 글자 Range 위치를 비교했다. 실제 셸/PTY echo를 대신했다고 주장하지 않는다.

| 측정 | 수정 전 최대 절댓값 | 수정 후 최대 절댓값 |
|---|---:|---:|
| 조합 표시 폭 − 확정 문자열 셀 폭 | 30.474 px | 0.0521 px |
| 마지막 조합 셀 시작 위치 − 확정 셀 시작 위치 | 24.397 px | 0.0521 px |
| 마지막 조합 셀 시작 위치 − 실제 렌더 글자 Range 시작 위치 | 23.720 px | 0.84375 px |

CSS 소수 폭의 양자화와 xterm의 병합 span/letter-spacing으로 측정 Range에 작은 잔차가 있다. 격자 폭·마지막 셀 위치는 0.1 CSS px 미만, 실제 렌더 글자 Range 위치는 1 CSS px 미만을 허용한다. 완전한 픽셀 동일성으로 보고하지 않는다. 공백 자체 한 칸은 단어 폭 비교에서 제외한다.

- 수정 전 [20개 측정값](issue62-ime/spacing-before/spacing.json). 배경 수정까지 적용한 직전 코드에 같은 검사로 자간 실패를 확인했다.
- 수정 후 [20개 측정값](issue62-ime/spacing-after/spacing.json).
- 수정 후 [조합 중 캡처](issue62-ime/spacing-after/spacing-pending.png) / [확정 후 캡처](issue62-ime/spacing-after/spacing-committed.png). 비교를 위해 시험 스크립트가 확정 문자열을 다음 줄에도 echo한다. 제품의 공백 입력이 줄바꿈한다는 뜻이 아니다.
- [디자인 기준](../design-system.md#한글-조합확정-자간-일치--62-후속-2026-11-07), [설명용 SVG](../design/terminal-ime.svg), [PNG](../design/terminal-ime.png).

## 출처·제한·남은 검증

작업 루트 `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/issue-62`, 기반 HEAD `962301168cbac037310f47ca8e13fbf0bae646a8` + 배경 수정 및 이번 자간 미커밋 변경. macOS 15.6.1 ARM64, Node v24.21.0, Rust 1.91.1, 번들 xterm 6.0.0, Menlo/Monaco/monospace. 시험 창은 640×240 point이며 자체 WKWebView snapshot만 캡처한다. 사용자 앱 활성화나 키보드 입력을 요청하지 않는다. 재실행 시 새 시험 창·새 xterm을 생성하고 시험 프로세스는 스스로 종료한다.

`npm run dev:info`의 root는 위 경로, instance는 `dev-3fbed16ec316807cb0b52c49`, binary는 `<root>/src-tauri/target/debug/agent-gibbon`이다. 개발 CLI `inspect --id issue62-progress`에서 `ok=true`, 예상 instance와 ID·본문을 대조했다. 해당 노트는 사용자가 숨긴 상태를 유지한다. 실행 PID 40551의 실제 txt 경로도 위 바이너리와 일치함을 `lsof`로 확인했다. 이 CLI 출처 확인은 최신 터미널 GUI 코드 반영 여부의 증거가 아니다. 이번 작업에서 사용자 개발 앱·기존 터미널 프로세스를 종료하거나 재시작하지 않았다.

최종 개발 빌드 SHA256은 `3e337663f99cfd30f885ee25a86dd59432945e2ce9e7c54fed16f9f1eb3376e7`이다. 기존 `package-lock.json` 변경은 보존하고 이번 자간 작업에서 수정하지 않았다.

실제 키보드에서 공백 확정 전후 자간·한영 전환·셸/PTY 전체 회귀, Windows/Intel, 긴 단어의 줄 경계·고어 자모는 미검증이다. 전체 플랫폼 검사는 여전히 커밋 승인 후 해당 SHA의 push 및 workflow_dispatch가 필요하다. 커밋·push·배포·병합은 수행하지 않았다.

Conf 4: 실제 WebKit의 합성 입력·렌더링 검사 범위. Risk Low: 조합 표시 DOM/CSS와 렌더 알림 갱신의 국소 변경.
