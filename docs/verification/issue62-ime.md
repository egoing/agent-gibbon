# #62 터미널 한글 조합 배경

2026-11-07. 아래는 최초 배경 수정의 검증 기록이다. 이후 사용자가 배경 해결을 확인하고 공백 확정 시 자간 변화를 제보했다. 이번 자간 보정과 최신 실행 결과는 [후속 기록](issue62-spacing.md)을 따른다. 전체 물리 키보드 수용 기준은 미완료다.

## 원인과 수정 범위

WebKit 보완 코드가 `textarea.value`의 단어 전체를 xterm `.composition-view`에 표시했다. 해당 요소의 기본 배경은 검정이므로 단어 앞부분까지 검은 배경으로 남았다. 정상 composition 이벤트의 처리·확정 전송은 [xterm 6.0.0 CompositionHelper](https://github.com/xtermjs/xterm.js/blob/6.0.0/src/browser/input/CompositionHelper.ts)가 담당하며 이번 수정에서 변경하지 않는다.

보완 경로에서만 별도 클래스를 적용한다. 보관한 단어의 앞부분은 기존 터미널 팔레트(`#20252e` / `#f0f0f0`), 마지막 코드 포인트는 기존 조합 팔레트(검정 / 흰색)를 사용한다. 현재 보완 경로가 다루는 현대 한글 음절·호환 자모가 대상이다. 고어 자모·여러 코드 포인트로 구성한 grapheme의 조합 범위는 확인하지 않았다. 단어 보관·PTY 전송 시점·받침 이동 보호·키 이벤트·xterm vendor 코드는 유지했다. 공백·구두점·방향키·Enter·blur·paste로 기존 확정 경로를 실행하면 별도 클래스와 조합 배경도 제거한다. Escape/Ctrl+C의 기존 취소 동작을 유지한다.

[디자인 기준](../design-system.md#한글-조합-배경--62-2026-11-07) · [설명용 SVG](../design/terminal-ime.svg) · [PNG](../design/terminal-ime.png).

## 환경과 출처

- 작업 루트: `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/issue-62`
- 기반 HEAD: `962301168cbac037310f47ca8e13fbf0bae646a8` + 이번 미커밋 변경.
- `npm run dev:info`: root=위 경로, id=`dev-3fbed16ec316807cb0b52c49`, binary=`<root>/src-tauri/target/debug/agent-gibbon`.
- macOS 15.6.1 (24G90), ARM64, Node v24.21.0, Rust 1.91.1, zsh 5.9, 앱 소스 0.2.7, 번들 xterm 6.0.0. 로컬 선택 입력기는 Apple 두벌식(`com.apple.inputmethod.Korean.2SetKorean`). 제보 당시의 셸·폰트·배율·앱 버전은 확인하지 않았다.
- 시험 글꼴은 제품 기본 Menlo/Monaco/monospace 13px다. 독립 WKWebView는 640×240 point, 캡처 영역 640×72 point, 출력 PNG는 Retina 2배다. 이는 제보 영상의 환경 확정이 아니다.
- 개발 CLI `inspect --id issue62-progress`: `ok=true`, instance가 예상 ID와 일치하고 ID·본문을 대조했다. PID 31285의 실행 파일 절대 경로는 위 binary와 일치한다. 이 프로세스는 수정 전 첫 개발 빌드로 시작했고 수정 후 재시작하지 않았다. CLI 결과는 통신 출처 검증이며 수정 후 실제 터미널 UI의 실행 증거가 아니다.
- 수정 후 개발 빌드 SHA256: `a76dc976a0bd5fa53352cb3363dfee0a7888202305d8cc61bbfd38d5b7768cb9`.

## 실행 결과

| 실행 | 결과 |
|---|---|
| `node --test tests/terminal-ime.test.mjs` | 6개 통과: 조합/받침 이동/삭제, 확정 경계, 취소, blur/paste, 정상 composition·영문·Chromium 위임 |
| `npm test` | Rust 182개, UI 126개 통과 |
| `npm run check` | Clippy 통과 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | 통과 |
| `npm run dev:build` | 최초 빌드 및 수정 후 빌드 통과 |
| `swift scripts/terminal-ime-webkit.swift "$PWD" <출력폴더>` | 수정 후 통과, 본문 `한글` 중 `글`만 검은 배경. 공백 확정은 단일 전송·표시 제거 확인 |
| 같은 Swift 검사 + 기반 HEAD의 `terminal-ime.js`·`terminal.css` | 의도한 실패: 전체 배경 `rgb(0,0,0)`, 현재 글자 전용 요소 없음 |
| SVG → PNG (`rsvg-convert`) | 렌더링 후 시각 확인 |
| `git diff --check` | 통과 |

WebKit 시험은 저장소의 실제 xterm·adapter·터미널 CSS를 독립 WKWebView에 로드한다. HTTP 서버나 제품 PTY를 만들지 않고 합성 InputEvent만 전달한다. 전송 문자열은 테스트 배열에 보관해 조합 중 미전송과 확정 시 단일 전송을 검사한다. 시험 창은 앱 활성화·키 창 요청 없이 생성하고 종료한다. 변경 전 입력은 `ㅎ → 하 → 한 → 한ㄱ → 한그 → 한글`로 동일하다.

- 수정 전: [캡처](issue62-ime/before/composition.png), [계산된 스타일](issue62-ime/before/render.json).
- 수정 후: [캡처](issue62-ime/after/composition.png), [계산된 스타일](issue62-ime/after/render.json).

Swift 스크립트는 별도 루트 인수를 받아 기반 소스 복사본에도 실행할 수 있다. 기준 소스 비교 시 `ui/vendor/xterm.js`·`xterm.css`는 같은 번들을 사용하고 adapter·terminal CSS만 기반 HEAD에서 가져온다. 단위 테스트와 WebKit 검사는 실제 입력기 키 조합 검증을 대체하지 않는다.

## 미검증과 다음 단계

독립 WKWebView에 네이티브 키를 합성 전달해 두벌식 composition 이벤트도 관찰했지만, 최초 입력과 백그라운드/전경 전달 경로가 달랐고 일관된 물리 키보드 재현이 아니다. 따라서 이 탐색을 정상 IME 회귀 통과로 집계하지 않는다. 탐색용 창은 해당 PID만 종료했고 기존 사용자 앱은 종료하지 않았다.

다음은 미완료다: 제보 환경 확정, 실제 키보드로 한글 조합/확정/공백/삭제/좌우 이동/줄바꿈, 한영 전환, 실제 PTY의 영문·붙여넣기 회귀, Windows·Intel 실제 앱. 고어·복합 grapheme·긴 단어의 줄 경계 개선은 이번 변경에 포함하지 않았다.

전체 플랫폼 release 워크플로는 미실행이다. WebKit 전용 스타일이지만 공통 터미널 자산이므로 플랫폼 간 영향을 확인할 필요가 있다. 저장소의 커밋 사전 승인 지침에 따라 사용자 승인 후 이번 변경을 커밋·push하고 해당 SHA로 수동 전체 검사를 요청해야 한다. 태그·배포·병합은 수행하지 않았다.

Conf 4: 실제 WebKit 렌더링의 합성 이벤트 시험 범위. 실제 IME 수용 기준 완료를 뜻하지 않는다. Risk Low: 보완 표시 DOM/CSS의 국소 변경이며 입력 전달 계약은 유지한다.
