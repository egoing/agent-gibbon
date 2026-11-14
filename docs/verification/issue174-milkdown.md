# #174 Milkdown 본문 통합 편집 구현·검증

**현재 저장 정책은 후속 사용자 승인에 따른 자동저장이다.** 아래 초기 명시저장 구현 기록은 작업 이력이며 맨 아래 자동저장 절이 최신 동작이다.

2026-11-14 · [PR #175](https://github.com/egoing/agent-gibbon/pull/175) · 조사에서 통합 편집·자동저장까지 확장한 작업 기록. 아래 미커밋·승인 대기 표현은 당시 진행 기록이며 최종 병합·동기화 결과는 PR과 이슈 #174에서 확인한다.

## 결과와 승인 범위

사용자의 ‘밀크다운으로 구현시작하자’ 승인에 따라 텍스트 본문을 항상 Milkdown으로 표시·선택·편집한다. 연필/textarea/별도 읽기 화면을 제거하고 툴바 저장·내부 Feedback 저장·Command S/Ctrl S를 기존 `save_note` 경로에 연결했다. 자동저장은 없다. 본문 drag는 선택, 툴바 빈 영역/접힌 제목 drag는 기존 native 창 이동이다. 최종 커밋·push·Ready·병합·배포는 하지 않았다.

- 작업 루트: `/Users/egoinglee/.codex/worktrees/0ae9/agent-gibbon`
- 브랜치: `codex/research-wysiwyg-markdown`
- 기반 제품 SHA: `91b17516f02df817abf2a21a04894f3ddcd6aaec`. 착수 시 `git ls-remote origin refs/heads/main`도 같은 SHA였다. 다른 작업의 미병합 제품 변경을 가져오지 않았다.
- 계획 커밋: `3924c45eca0f3e292430f05c23251e9a9cfd0c85`
- 개발 instance: `dev-cd65829f4f3c180fea8442f4`
- binary: `/Users/egoinglee/.codex/worktrees/0ae9/agent-gibbon/src-tauri/target/debug/agent-gibbon`
- 환경: macOS ARM64, Node `v24.21.0`, pnpm `11.25.0`, 앱 `0.2.11` 유지.

## 구현 경계

| 항목 | 실제 구현 |
|---|---|
| 의존성·번들 | Milkdown core/commonmark/gfm/prose/utils/history `7.23.0` 및 remark/unified를 고정하고 pnpm lockfile 갱신. esbuild `0.28.2`, 빌드 스크립트는 로컬 ESM과 실제 번들 의존성 라이선스 원문을 생성한다. CDN/서버 없음. |
| 개발·배포 진입 | `dev`, `dev:build`, `build.mjs`, Tauri beforeDev/beforeBuild, 릴리스 CI의 Cargo 검사 전 단계에 번들 생성 연결. 직접 Cargo 실행 시에는 먼저 `pnpm run build:editor`. 생성 디렉터리는 gitignore. |
| 저장·원문 | 수정 없는 원문은 재직렬화·쓰기하지 않는다. dirty 시에만 명시 저장하며 revision/연결 파일 원문 비교와 백엔드 원자적 교체를 재사용한다. 실제 편집 뒤 동일 의미의 목록 기호·표 공백 등 정규화는 가능하다. |
| 의미 손실 보호 | 입력 원문과 초기 Milkdown 재직렬화 결과를 Markdown AST로 비교한다. position/spread만 비교에서 제외한다. 참조 문법·코드 metadata 등의 AST 구조 보존을 확인하지 못하면 원문 전체를 읽기 전용 코드로 표시하고 저장 차단. 원문은 메모/연결 파일 모두 그대로 남는다. |
| 복구 | 보호된 문서는 **원문 복사**로 정확한 Markdown을 복사한다. 외부 편집기로 수정하고 `update --id ID --file 절대경로`로 다시 읽는다. 이미 입력한 초안에 외부 갱신/저장 실패가 발생하면 편집 DOM을 유지하며 초안 복사 후 취소로 최신 수신본을 확인한다. 자동 변환/자동 덮어쓰기 없음. |
| IME·이력·focus | composition 중 저장·취소·외부 문서 교체를 보류. 로컬 변경은 history undo/redo를 사용하고 clean 외부 본문 교체는 새 history를 만든다. 저장 잠금 때문에 입력 focus가 풀리면 다른 컨트롤/앱으로 옮기지 않은 경우만 복원한다. 영역 텍스트 생성의 명시적 초기 focus는 표시·펼침 후 한 번, 일반 CLI open은 focus 요청 없음. |
| 창·업데이트 | dirty/composition/save/export 상태만 updater busy로 보고한다. 첫 실제 입력만 기존 편집 높이 경로 요청. 접힘·숨김·수동 크기를 유지하며 내부 Feedback/viewport를 사용한다. |
| 보안 | raw HTML 문자 표시, 링크는 href 없는 span, 이미지는 src 없는 표식. ProseMirror가 생성하는 src 없는 separator img는 자체 편집용이며 네트워크 자산이 아니다. 붙여넣기 plain text만 허용, drop 차단. 기존 CSP·Rust sanitize·진단 동의 경계를 변경하지 않았다. |
| 변경 표시 | Rust가 만든 sanitized 변경 표시를 본문 밖 ProseMirror decoration으로 매핑한다. 삭제는 비편집 widget, 완료 체크박스는 node decoration. 저장 Markdown에 장식·삭제 이력이 들어가지 않는다. dirty/조합 중 수신 문서를 덮거나 변경을 자동 확인하지 않는다. |

장식은 렌더링 텍스트를 순서대로 매핑한다. 현재 회귀의 일반 문단·체크박스·삭제·표/코드를 확인했으나 모든 복합 Markdown 중첩의 정확한 위치 표시를 증명한 것은 아니다. 원문 위치 기반의 완전한 source map 구현은 하지 않았다. 원문 보호는 의미가 같은 참조 문법 변환도 차단할 수 있는 보수적 정책이며 모든 Markdown을 편집 가능하게 만드는 보장은 아니다.

## 실행 결과

| 검사 | 결과와 범위 |
|---|---|
| `pnpm install --frozen-lockfile` | 통과. esbuild postinstall만 workspace allowBuilds에서 명시 허용. |
| `pnpm run test` | Rust **223개**, UI **176개** 통과. 연결 파일 수정/삭제/readonly/stale revision 거절은 기존 Rust 저장 회귀 포함. |
| `pnpm run check` | Clippy all-targets `-D warnings` 통과. |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | 통과. |
| `pnpm run dev:build` | 최종 로컬 debug 빌드 성공. ARM64 호스트 한정. |
| `pnpm run version:check` | `0.2.11` 일치. 버전 변경 없음. |
| `PLAYWRIGHT_MODULE=… node scripts/milkdown-browser.mjs` | Chromium `158.0.8127.18`, 실제 생성 번들·HTML/CSS 사용, native IPC만 모의. 직접 입력/무편집 저장0회/명시저장/undo·redo/체크박스/외부 충돌/접힘숨김/이동 hit-test/조합 중 저장·Esc·undo·외부 수신/장식 비혼입/HTML·이미지·URL 보안/원문 보호·무손실 복사/144·300·480px/66,000자 문서 저장 통과. pageerror 0, 외부 네트워크 요청 0. |
| `python3 scripts/milkdown-cli-smoke.py` | 실제 binary/instance 응답과 본문·원본 연결·비포커스 시작, 숨긴 상태 CLI 갱신과 위치·크기 보존 확인. 전용 메모 숨기고 이번에 시작한 PID만 종료. |
| 시안·문서 | `rsvg-convert` PNG 렌더 및 육안 확인. `git diff --check`, 로컬 문서 링크 확인. |

이번 번들 실측은 minified JS **475,020 bytes**, gzip **145,957 bytes**, 동봉 라이선스 원문 **96,799 bytes**다. npm unpackedSize와 다른 실제 산출물 수치다. 여러 native WebView의 RSS·입력 지연·최소 macOS WebKit 성능은 측정하지 않았다. 기존 OS 최소 버전은 변경하지 않았고, 오래된 WebKit의 `Array.prototype.at` 누락에는 로컬 호환 처리를 추가했으나 오래된 실제 WebKit 검증은 남는다.

### UI 테스트 수 변경의 이유

기반 UI 테스트는 203개, 이번 결과는 176개다. 기존 editor VM **55개를 통합 표면 계약 26개로 재구성**했고 Markdown 의미 손실 테스트 **2개를 추가**했다(203 − 55 + 26 + 2 = 176). 단순히 실패한 테스트를 제외한 결과가 아니다.

| 제거·변경된 기존 계약 | 새 대응 회귀 |
|---|---|
| 연필 진입→textarea 복사→저장 후 읽기 화면 복귀 | 항상 편집 표면·clean 저장 요청0·dirty 저장 후 같은 editor 유지 |
| 읽기 Cmd/Ctrl+S 내보내기, 취소 후 내보내기 | 텍스트 항상 명시 저장, 이미지 내보내기는 유지 및 실패/재시도 |
| 편집 진입마다 높이 요청, region 진입 재실행 억제 | 첫 실제 변경 한 번만 높이 요청·접힘/숨김/취소 후 재요청 없음, region focus 한 번 |
| textarea draft/selection/fold와 기존 diff 확인 | dirty 초안/기준 revision 보존, 늦은 응답·포커스 이탈 시 새 수신 미확인, 실패 확인 재시도, 실제 DOM 선택/undo/장식 저장 비혼입 브라우저 회귀 |
| 편집 버튼/본문의 drag 제외 | 툴바 빈 공간만 native 이동, 본문·버튼·비주버튼은 이동 안 함 |

색상·글자 크기·숨김·복사·업데이트 보호·이미지·포인터 대상 구분 등 유지 계약도 새 VM 및 기존 독립 테스트에 남겼다. `editor-save-smoke.mjs`, `editor-entry-height-browser.mjs`, `inline-feedback-smoke.mjs`의 브라우저 진입점은 새 실제 번들 회귀로 연결했다. textarea 전용 native `editor-ui-smoke.py`/`editor-entry-height-smoke.py`는 새 UI를 잘못 조작하거나 통과로 오인하지 않도록 실행을 중단하고 아래 수동 절차를 안내한다. 이 두 native 스크립트는 통과 목록에 포함하지 않는다.

## 증거와 native 검증 한계

- [CLI 실제 응답](issue174/cli-result.json): 메모 `milkdown174-cli-048330c4`, PID **4368**, root/instance/binary와 명령별 결과. PID 실행 파일을 종료 직전 재확인하고 이번 프로세스만 종료했다.
- 최초 native 접근 시도: 메모 `milkdown174-cd658`, 고유 본문 `MILKDOWN174-CD658`, PID **89490**, 같은 개발 binary/instance를 CLI inspect와 `ps`로 확인했다. native 창 제목·GUI 캡처는 도구가 대상을 잡지 못해 확인하지 못했다.
- Orca는 `orca-runtime.json` 부재로 `runtime_unavailable`. Codex native CUA는 이 unbundled debug 실행 파일/PID/app-name을 `Invalid app`으로 거절했다. 설치 앱으로 대체하거나 다른 사용자의 창을 조작하지 않았다. 전용 메모를 숨기고 해당 PID만 종료한 뒤 팀장에게 GUI 슬롯을 반환했다.
- [브라우저 실행 결과](issue174/browser-result.json), [clean 300px](issue174/browser-clean-300.png), [dirty 300px](issue174/browser-dirty-300.png), [144×96](issue174/browser-144x96.png), [480×360](issue174/browser-480x360.png)는 **Chromium 모의 IPC 화면**이며 native 앱 캡처가 아니다.

**실제 OS 한글 IME 입력·조합 중 외부 갱신/저장/undo·native 선택/체크박스/이동·연결 파일 저장 UI·Intel/Windows/구형 WebKit는 미검증이다.** 브라우저 compositionstart/update/end는 이 항목의 통과 근거가 아니다. release CI/전체 플랫폼 설치 파일 빌드도 요청 범위에 따라 실행하지 않았다.

## 사용자가 직접 확인할 실행 방법

아래는 이 워크트리에서 실행한다. 다른 인스턴스/사용자 파일을 쓰지 않도록 별도 파일·ID를 사용한다.

```sh
pnpm install --frozen-lockfile
pnpm run dev:build
pnpm run dev:info
# 별도 파일에 한글 문단·task·표를 작성한 뒤:
./scripts/agent-gibbon-dev open --id milkdown174-manual --file /tmp/milkdown174-manual.md
./scripts/agent-gibbon-dev inspect --id milkdown174-manual
```

inspect의 `ok`, `instance=dev-cd65829f4f3c180fea8442f4`, 메모 ID·고유 본문을 대조한다. GUI 사용 시 실제 binary/PID와 `milkdown174-manual`을 포함하는 native 창 제목도 대조한다. 두벌식으로 문단/표/체크리스트에 받침·Backspace·선택 교체를 입력하고 **조합 중 Command S·Escape·undo**, 확정 후 저장·undo/redo를 확인한다. 조합 중 다음 명령으로 외부 revision을 보내 초안이 남는지 확인한다.

```sh
./scripts/agent-gibbon-dev update --id milkdown174-manual --file /tmp/milkdown174-remote.md
```

저장 충돌은 초안을 유지해야 하고, 복사 후 취소하면 최신 수신본이 보여야 한다. 연결 파일만 외부 편집기로 바꾼 뒤 저장하는 경우도 원본을 덮지 않아야 한다. 숨김/접힘/크기 변경과 선택 복사·체크박스·툴바 빈 영역 이동을 확인한다. 종료할 때는 다음으로 **검증 메모만** 숨긴다. CLI close는 앱 프로세스 종료가 아니다.

```sh
./scripts/agent-gibbon-dev close --id milkdown174-manual
```

원문 보호 복구 예: `[본문][ref]`와 참조 정의를 가진 전용 문서를 열어 원문 복사를 확인하고, 외부 편집기에서 일반 `[본문](URL)`로 명시적으로 바꿔 CLI로 다시 읽는다. 이 과정은 사용자가 선택한 변환이며 앱의 자동 변환이 아니다.

## 제출 파일·확신도

변경 범위는 편집기/정책/호환 모듈, `ui/app.js`·본문 HTML/CSS·Feedback/scrollbar, 고정 의존성과 번들/개발/CI 진입, 회귀 테스트·검증 스크립트, README·기본 사용 스킬·디자인 시스템·새 SVG/PNG·이 기록이다. 조사 문서는 이전 미커밋 파일을 보존했다. 제품 Rust 소스·네트워크 서버·진단 정책·버전은 변경하지 않았다.

Conf **4**: 실제 번들 브라우저 및 실제 로컬 CLI까지 확인한 범위. native IME는 미검증.
Risk **Medium**: 편집·원문·저장 상태 계약에 영향이 있다. 미확인 native 입력이 남아 있어 배포 준비 완료로 보지 않는다.

[디자인 시스템](../design-system.md) · [시안 SVG](../design/milkdown-editor.svg) · [시안 PNG](../design/milkdown-editor.png)

사용자의 후속 ‘실행’ 요청으로 최종 빌드의 `milkdown174-preview` 메모를 열었다. 이 사용 확인용 개발 앱과 메모는 종료하지 않고 유지한다.


## 후속: 선택 서식 메뉴

사용자가 ‘컨텍스트 메뉴’로 글자 선택 시의 굵게·제목 메뉴를 요청해 작은 floating toolbar를 추가했다. 굵게/기울임/취소선/코드/H1/H2/본문을 기존 ProseMirror selection과 history로 적용하며 저장은 기존 명시 저장만 사용한다. viewport 내부 배치·144px 가로 스크롤·Alt+F10/방향키·조합/보호/잠금 중 숨김을 구현했다. 실제 번들 Chromium에서 선택 유지·굵게·제목/본문 변경·저장·조합 중 숨김·좁은 창 및 키보드 접근을 확인했다. 실제 WKWebView 메뉴 동작은 별도 사용자 확인 대상이다.

서식 메뉴 소스는 `ui/editor-format-menu.js`이며 디자인 시스템과 기존 SVG/PNG를 함께 갱신했다. 사용자 실행 중 메모에 미저장 초안이 확인되어 개발 프로세스를 강제 재시작하지 않았다. 초안 저장 확인 후 새 빌드로 반영한다.

선택 서식 메뉴의 Chromium 화면: [300px](issue174/browser-selection-format-300.png) · [144px](issue174/browser-selection-format-144.png).

사용자가 개발환경의 미저장 변경 폐기를 명시적으로 허용해, 기존 전용 개발 PID 5354만 종료하고 새 빌드로 `milkdown174-preview`를 다시 열었다. CLI inspect의 instance·메모 ID·visible을 재확인했다. 새 프로세스와 미리보기 메모는 사용자 확인용으로 유지한다. 실제 native 서식 메뉴는 사용자 확인 대상이다.


## 후속: 자동저장 — 사용자 명시 승인

사용자 요청 ‘저장은 자동 저장으로 구현’에 따라 정상 입력의 저장/취소 버튼을 제거하고 **변경 후 600ms 자동저장**으로 전환했다. Cmd/Ctrl+S는 즉시 flush한다. 한글 조합은 확정 후 저장하며, 수정 없이 읽은 문서는 저장하지 않는다. 저장 중 입력을 잠그지 않고 전송한 문서 snapshot만 저장 기준으로 인정한다. 그 사이 입력·undo는 dirty로 유지해 성공 응답의 새 revision을 기준으로 다음 저장한다.

실패는 초안을 보존하고 자동 재시도를 중단하며 변경 취소·다시 저장을 제공한다. 외부 revision과 연결 파일 충돌 검사는 그대로 재사용한다. Escape/닫기는 저장을 기다리고 남은 변경을 flush한 뒤 실행하며, 실패하면 실행하지 않는다. 조합 중에는 해당 동작을 보류한다. 앱 강제 종료/충돌 시 마지막 성공 저장 이후 입력은 보장하지 않는다.

검증: UI **180개**, Rust **223개**, 실제 번들 Chromium의 600ms 자동저장·저장 버튼 없음·700ms 지연 IPC 중 추가 입력과 후속 revision·실패 후 자동 재시도 중단·명시 재시도·composition 중 저장0회와 확정 후 자동저장 회귀를 통과했다. 다른 기존 보안/선택메뉴/원문/기하/대용량 회귀도 유지한다. UI 재구성 수는 현재 203−55+30+2=180이다. 실제 OS IME/native UI는 여전히 미검증이다.

자동저장 뒤 사용자 확인용 개발 앱을 새 빌드로 다시 실행한다. 개발환경 초안 폐기는 사용자가 앞서 명시 허용했으며, 다른 워크트리/설치 앱은 종료하지 않는다. [자동저장 Chromium 화면](issue174/browser-autosave-300.png) · [최신 디자인 기준](../design-system.md).

자동저장 빌드의 전용 메모 `milkdown174-preview` 재열기와 instance/visible을 CLI로 확인했다. 앱은 사용자 확인을 위해 계속 실행한다.


## close 검증과 후속 범위

사용자 close 승인으로 현재 제품 변경을 커밋·병합한다. 마지막 코드 기준 frozen lockfile 설치, Rust 223개·UI 180개, Clippy, Rustfmt, 실제 번들 Chromium 회귀를 다시 확인했다. 전체 플랫폼 릴리스 CI는 실행하지 않는다. 이 저장소의 Release workflow는 태그/수동 실행이며 일반 PR의 필수 원격 검사 목록은 없다.

사용자가 요청한 이미지 본문 첨부·일반 파일 카드는 [#182](https://github.com/egoing/agent-gibbon/issues/182), H1/H2/코드 기호 표시 명칭 변경은 [#183](https://github.com/egoing/agent-gibbon/issues/183)에 기록했다. **두 이슈의 구현은 이번 변경에 포함하지 않는다.** 미지원 문법/실제 OS IME 등 기존 검증 한계도 계속 명시한다. Codex 워크트리이며 Orca 정리 대상이 아니다. 사용자 요청으로 실행한 개발 앱은 유지한다.
