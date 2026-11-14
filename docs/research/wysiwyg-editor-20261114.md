# 텍스트 메모 위지윅 에디터 조사 — 구현 전 기록

2026-11-14 · 조사 이슈 [#174](https://github.com/egoing/agent-gibbon/issues/174) · 계획 Draft PR [#175](https://github.com/egoing/agent-gibbon/pull/175)

## 결론

**Milkdown의 최소 구성(CommonMark + GFM + history + listener)을 첫 실험 대상으로 권장한다.** Markdown을 중심으로 설계되어 있고 React/Vue 없이 현재 바닐라 WebView에 붙일 수 있으며, 자체 대형 툴바 없이 기존 메모 표면을 유지하기 쉽다. 후보는 **Milkdown, Tiptap, Vditor**로 좁힌다. Tiptap은 직접 제어할 범위가 커질 때의 차선, Vditor는 완성된 편집 기능을 빨리 비교하는 대조군이다. 이는 공식 자료와 현재 코드에 근거한 적합성 판단이며 실행 성능·입력 안정성의 순위가 아니다.

어느 후보도 현재 Markdown 원문을 바이트 단위로 보존한다고 확인하지 못했다. 원문 → 편집기 문서 모델 → Markdown 재직렬화에서 문법·공백이 달라질 수 있고, 이 앱은 그 차이를 외부 변경 강조·연결 파일 충돌에도 사용한다. **아무것도 수정하지 않은 열기/선택만으로 원문을 다시 저장하지 않는 조건과, 실제 수정 시 허용할 정규화 범위가 채택의 선행 조건**이다.

사용자 요청의 ‘표시·편집 구분 제거’는 한 표면에서 읽고 선택하고 입력하는 방향으로 이해한다. **자동저장·blur 저장·Esc 취소·저장 버튼 제거까지 확정된 것은 아니다.** ‘툴바를 클릭해서 이동’은 우선 빈 영역을 눌러 끄는 native 이동으로 검토하며, 단일 클릭 후 포인터 추종 같은 동작은 미확정이다. 제품 구현·패키지 설치·빌드·GUI 시험은 수행하지 않았다.

## 현재 앱에서 지켜야 하는 계약

기준 루트 `/Users/egoinglee/.codex/worktrees/0ae9/agent-gibbon`, 제품 코드 기준 `91b17516f02df817abf2a21a04894f3ddcd6aaec`. Node `v24.21.0`, pnpm `11.25.0`, 앱 `0.2.11`, Tauri 2. `package.json`의 pnpm 고정값을 확인했다. 프론트엔드 번들러 없이 `ui/`가 `frontendDist`이며 bare npm import를 현재 HTML에 바로 추가할 수 있는 구조는 아니다.

| 계약 | 실제 근거 | 위지윅 전환의 영향 |
|---|---|---|
| Markdown 원문이 저장·CLI 갱신의 기준 | `src-tauri/src/model.rs`의 `Note.markdown`, `apply`, `save_edit` | 편집기 JSON/HTML을 영속 원본으로 바꾸면 CLI/파일 계약까지 바뀐다. 원문을 정본으로 유지하는 어댑터가 필요하다. |
| GFM 체크리스트·취소선·표 및 일반 코드 | `model.rs`의 `render_styled`, `validate_markdown` | GFM 지원이라는 홍보만으로 동일 결과를 보장하지 않는다. `- [ ]` 검증, 혼합/중첩 목록, 코드 속 `[ ]` 예외까지 비교한다. |
| HTML은 실행 결과가 아니라 문자 | `render_styled`가 raw/inline HTML 이벤트를 text로 전환 | `<b>문자</b>`를 굵게 바꾸는 일반 HTML import는 기존 의미와 다르다. HTML·주석을 삭제하거나 실행하지 않아야 한다. |
| 이미지·링크 탐색도 제한 | `model.rs`의 ammonia: `img` 제거, `a.href` 제거, 생성된 mark/class 허용 | Rust sanitize가 편집기 내부 DOM을 대신 보호하지 않는다. 붙여넣기·URL·이미지·raw HTML의 편집 중 정책도 동일 경계로 구성한다. |
| 편집 초안과 수신 상태는 분리 | `ui/app.js`의 `beginEditing`, `refresh`, `editRevision` | 350ms snapshot이 최신 서버 상태를 받지만 textarea 초안을 덮지 않는다. 위지윅의 전체 setContent를 폴링마다 호출하면 선택·IME·undo가 손상될 수 있다. |
| 저장은 메모 revision과 연결 파일 원문을 확인 | `model.rs::save_edit`, `src-tauri/src/source.rs::save` | stale revision은 거절하고 파일의 외부 수정·삭제·readonly를 확인한다. 같은 디렉터리 임시 파일+rename이며 파일 write 실패 전 note를 교체하지 않는다. 다만 다른 프로그램의 write와 완전한 OS 수준 CAS는 아니다. |
| diff/quiet replace/새 완료 강조가 별개 | `model.rs`의 `change_base`, `rebase_quiet`, `completion_html`; `ui/app.js`의 표시 우선순위 | 새 완료 강조와 미확인 변경을 문서 본문·저장 원문에 섞지 않는다. 삭제 표시를 편집 가능한 텍스트로 import하면 삭제한 문장이 되살아날 수 있다. |
| 읽은 변경 자동 확인 | `ui/app.js`의 readRevision/acknowledge, 현재 편집 중 확인 차단 | 항상 편집 가능한 표면에서 기존 `editing` boolean을 그대로 true로 두면 확인이 막힌다. 초안 dirty/조합 중/실제 관찰 revision을 분리해야 한다. |
| 저장·내보내기·취소·업데이트 보호가 화면 모드에 결합 | `ui/app.js`, `ui/feedback-state.mjs`, `window.gibbonUpdateActivity` | 현재 Cmd/Ctrl+S는 편집 중 저장, 그 밖에는 내보내기이며 Esc는 편집 취소/숨김으로 나뉜다. `editing` 상시 true면 자동 업데이트의 busy도 계속 참이 된다. 단순 UI 제거로 끝나지 않는다. |
| 본문 선택은 현재 이동과 충돌 | `ui/style.css` body user-select:none/cursor:grab; `ui/app.js` 문서 mousedown의 preventDefault+drag_note | contenteditable만 추가해도 전역 핸들러가 native 이동을 시작할 수 있다. 본문을 선택 가능한 영역으로 만들고 이동 hit-test를 툴바로 좁혀야 한다. |
| native 이동과 자동 배치 제외 | `src-tauri/src/entrance.rs::drag_note` | 기존 start_dragging과 layout mark를 재사용하는 방향이 적절하다. 별도 JS 좌표 이동 루프는 제안하지 않는다. |
| 접힘·숨김·수동 크기·포커스 보존 | `ui/app.js`, `src-tauri/src/focus.rs`, `src-tauri/src/sizing.rs`, `src-tauri/src/session_restore.rs` 및 디자인 시스템 | 숨김 복원·외부 갱신이 입력 focus를 빼앗지 않아야 한다. 편집기 mount/update로 창을 다시 만들거나 자동 fit을 매 입력마다 강제하지 않는다. |

디자인 시스템은 문서 전체를 대상으로 관련 규칙을 확인하고, [저장·편집 시안 PNG](../design/editor-save-shortcut.png)와 [툴바 PNG](../design/toolbar-reserved-space.png)를 실제 열어 확인했다. 최신 내부 Feedback(#160), 저장/22px 편집 행(#164), 편집 진입 210px(#159), 툴바 전용 행 및 접힌 제목 드래그(#135/#141)가 관련된다. 문서의 과거 외부 Feedback 설명은 최신 내부 배치보다 우선하지 않는다. **위지윅 상시 표면에서 #159의 ‘진입’을 무엇으로 볼지, 22px/30px 행 중 어느 것을 적용할지는 후속 설계 판단**이며 이번 조사에서 바꾸지 않았다.

## 후보 비교

2026-11-14 npm `latest` 메타데이터 및 공식 저장소를 조회했다. 버전은 이번 조회 시점의 값이며 설치/동작 검증 버전이 아니다. 아래 MIT 표시는 코어·명시 패키지의 라이선스이고, 실제 채택 시 잠긴 transitive dependency와 배포 assets의 고지 목록도 따로 확인해야 한다.

| 후보·조회 버전 | 라이선스·프레임워크 | Markdown/GFM·원문 왕복 | 로컬 번들·제어 범위 | 판단 |
|---|---|---|---|---|
| **Milkdown 7.23.0** | MIT. ProseMirror+remark 기반, 프레임워크 독립 API | CommonMark에 GFM preset을 추가해 표·task·취소선 처리. serializer를 거치므로 원문 무손실 보장 아님 | 필요한 core/preset/history/listener를 선택하고 JS/CSS를 로컬 번들. Crepe의 전체 UI는 우선 제외 | **1순위 실험**. 기존 작은 표면에 맞추기 쉬운 구조이나 어댑터·장식 매핑은 직접 필요 |
| **Tiptap core/markdown 3.34.0** | 해당 OSS 패키지 MIT. Vanilla 지원, React/Vue 필수 아님. 유료 기능과 구분 | 공식 양방향 Markdown 확장이 현재 Beta. Marked→JSON→Markdown. 표·task는 대응 확장을 구성해야 하고 schema 밖 내용은 별도 처리 | core+Markdown+선택 확장만 번들. Pro/Cloud 없이 로컬 실행 가능 | **차선**. plugin/transaction 제어가 명확하나 Markdown 경계 사례와 HTML 문자 보존을 더 확인해야 함 |
| **Vditor 4.0.1** | MIT, native JS 지원. Lute 기반 | WYSIWYG/IR/분할 3모드 중 wysiwyg 고정 가능. GFM 기능과 Markdown API 제공, 원문 표기 보존 보장은 아님 | 배포 JS/CSS 외 Lute·테마·아이콘·lazy 렌더 자산까지 로컬 경로화 필요. cache·upload·렌더 옵션 점검 | **대조군**. 완성 UI는 빠르지만 자동 캐시/외부 자산/독자 DOM과 앱 상태 연결 부담 |
| TOAST UI Editor 3.2.2 | MIT, vanilla 생성 API | Markdown/WYSIWYG와 GFM 제공, 모드 스위치 숨김 가능 | 로컬 배포 가능하나 기본 UI·선택 기능을 줄이고 usageStatistics를 비활성화해야 함 | **신규 채택 제외 권고**. 공식 저장소가 2026-09-02 archive 상태여서 유지보수 책임이 커짐 |
| Lexical/markdown 0.53.0 | MIT, vanilla 지원. React는 선택 | Markdown transformer 방식. rich table 편집과 Markdown 표 왕복은 별개이며 앱 맞춤 매핑 확인 필요 | 필요한 패키지만 번들 가능. 자체 node/selection/history와 Rust 원문 간 어댑터 필요 | **이번 최종 후보에서 제외**. 범용 편집기로서는 가능하나 Markdown 중심 메모에 필요한 검증·조립 범위가 큼 |

근거: [Milkdown 공식 저장소](https://github.com/Milkdown/milkdown), [GFM API 원문](https://github.com/Milkdown/milkdown/blob/0655111f3bb8393e959ce3f3d107c83edce29c3a/docs/api/preset-gfm.md), [Tiptap Vanilla 설치](https://tiptap.dev/docs/editor/getting-started/install/vanilla-javascript), [Tiptap Markdown 및 명시 한계](https://tiptap.dev/docs/editor/markdown), [Vditor 공식 설명](https://github.com/Vanessa219/vditor/blob/0732967e1bd16de7716399b6926bf75ad9bfc5d8/README_en_US.md), [TOAST UI archive·MIT](https://github.com/nhn/tui.editor), [Lexical vanilla](https://lexicaljs.org/docs/getting-started/quick-start), [Lexical Markdown API](https://lexical.dev/docs/packages/lexical-markdown).

ProseMirror 직접 조립안도 Tiptap/Milkdown의 기반으로 비교했다. 완성된 Markdown/GFM 에디터가 아니어서 schema·parser/serializer·표/task·keymap·clipboard를 직접 통합하는 부담이 크므로 첫 실험에서는 제외한다. GitHub prosemirror-view의 archive는 저장소 이전 때문이므로 TOAST UI와 같은 유지보수 종료 근거로 쓰지 않는다. [이전 공지](https://github.com/ProseMirror/prosemirror-view/blob/master/README.md) · [공식 가이드](https://prosemirror.net/docs/guide/).

TOAST UI의 사용 통계 기본 활성화와 비활성화 옵션은 [패키지 공식 README](https://github.com/nhn/tui.editor/blob/master/apps/editor/README.md)에 명시되어 있다. 라이브러리 기본값은 앱의 별도 진단 전송 동의를 대체하지 않으므로, 실험에서도 비활성화하고 외부 요청이 없는지 확인한다.

### 용량 비교의 의미와 한계

다음은 npm registry의 `dist.unpackedSize`를 **바이트로 직접 조회한 값**이다. 소스·타입·source map 등을 포함할 수 있는 단일 패키지의 압축 해제 크기이며, 의존성 합계·앱 증가량·다운로드·런타임 메모리가 아니다. 숫자만으로 경량 순위를 매기지 않는다.

| 직접 조회한 패키지 | version | unpacked bytes |
|---|---|---:|
| `@milkdown/core` | 7.23.0 | 195,745 |
| `@milkdown/preset-gfm` | 7.23.0 | 293,962 |
| `@tiptap/core` | 3.34.0 | 3,336,096 |
| `@tiptap/markdown` | 3.34.0 | 493,708 |
| `@toast-ui/editor` | 3.2.2 | 3,274,089 |
| `vditor` | 4.0.1 | 23,638,046 |
| `lexical` | 0.53.0 | 3,745,927 |
| `@lexical/markdown` | 0.53.0 | 306,530 |

조회 출처는 [Milkdown core](https://registry.npmjs.org/@milkdown/core/7.23.0), [GFM](https://registry.npmjs.org/@milkdown/preset-gfm/7.23.0), [Tiptap core](https://registry.npmjs.org/@tiptap/core/3.34.0), [Markdown](https://registry.npmjs.org/@tiptap/markdown/3.34.0), [TOAST UI](https://registry.npmjs.org/@toast-ui/editor/3.2.2), [Vditor](https://registry.npmjs.org/vditor/4.0.1), [Lexical](https://registry.npmjs.org/lexical/0.53.0), [Markdown](https://registry.npmjs.org/@lexical/markdown/0.53.0)이다. 실제 조회에는 각 패키지의 `/latest` 응답을 사용했으며 위 링크는 확인된 버전의 메타데이터다.

채택 전에는 동일 기능 집합의 minified JS/CSS와 필수 자산 합계·gzip·네트워크 차단 cold start·1개/10개 메모 초기화 시간과 RSS를 측정한다. 현재 크기 상한과 성능 예산은 사용자 미확정이므로 임의 수치를 합격 기준으로 만들지 않는다. 현재 CSP는 script/style의 self와 IPC를 기본으로 하므로 CDN·eval·외부 스타일에 의존하지 않으며, CSP 완화를 도입 조건으로 삼지 않는다.

## Milkdown을 먼저 실험할 이유와 남는 손실

1. Markdown을 중심으로 표·task를 공식 preset으로 구성할 수 있어 현재 Rust 렌더러의 범위에 맞추기 쉽다.
2. ProseMirror의 transaction/selection/history와 장식을 사용할 여지가 있어 변경 강조를 저장 본문과 분리하기 좋다. [공식 가이드](https://prosemirror.net/docs/guide/) 다만 기존 HTML의 mark를 편집기 DOM에 그대로 삽입하는 방식은 피한다.
3. 조회한 HTML schema는 `value`를 문자로 표시하고 Markdown html node로 돌려주는 구조여서 기존 HTML 문자 표시와 방향이 맞는다. [확인한 구현](https://github.com/Milkdown/milkdown/blob/0655111f3bb8393e959ce3f3d107c83edce29c3a/packages/plugins/preset-commonmark/src/node/html.ts) 다만 atom이므로 태그 안을 일반 텍스트처럼 편집할 수 있다는 뜻은 아니며 block HTML을 포함한 보존은 미검증이다.

가장 큰 손실 후보는 같은 모습의 문서도 원문 표기가 달라질 수 있다는 점이다. `*`/`-`, `_`/`*`, Setext/ATX 제목, 참조 링크, 연속 빈 줄, 끝 공백, CRLF, 코드 fence·언어 info, 목록 시작 번호, 표의 공백/escape, HTML/주석을 확인해야 한다. 전체 재직렬화로 에이전트의 line diff가 대량 발생하는 문제는 외형의 미세 차이보다 중요하다.

Tiptap 공식 Markdown 문서는 Beta, comments 및 셀 내부 복수 자식 노드의 제한을 명시한다. 이 comments 제한을 곧바로 모든 HTML 주석의 소실이라고 단정하지 않고 fixture로 별도 확인한다. HTML 가져오기는 기존 schema 해석을 사용하므로 현재 문자 표시와 동일하지 않다. [공식 Markdown](https://tiptap.dev/docs/editor/markdown) Vditor는 기본값에 IR·cache 활성화·CDN 참조가 있으므로 wysiwyg 고정·자체 cache 비활성화·로컬 자산 경로가 실험 전제다. [확인한 Options](https://github.com/Vanessa219/vditor/blob/0732967e1bd16de7716399b6926bf75ad9bfc5d8/src/ts/util/Options.ts)

## 후속 실험 설계안 — 미실행

이 절은 승인 전 제안이며 제품의 확정 규칙이 아니다. Markdown 원문은 Rust 쪽 정본, 편집기 문서는 편집 중 파생 상태로 둔다. `baseRevision`, 입력 시작 원문, dirty, composition 중 여부, 수신 revision을 분리하고 저장 계약 결정 전에 blur·폴링에 자동저장을 추가하지 않는다.

| 검증 | 실험과 완료 조건 |
|---|---|
| 원문·의미 왕복 | 제목·혼합/중첩 task·표/pipe/alignment·fence/inline code·HTML/주석·참조 링크·끝 개행/CRLF를 fixture로 만든다. 무편집 open/select/blur는 저장 요청 0회·원문 동일. 실제 편집은 의미·Rust 표시 동등, 정규화 차이를 전부 나열해 허용 범위 승인. 조용한 누락 0건. |
| 한글 IME | 실제 macOS WKWebView에서 두벌식 연속 입력·받침·Backspace·선택 교체·확정 직전 Enter/Esc/Cmd+S, 표/task/code 내부, blur/재focus/외부 갱신을 조합한다. 중복·누락·임의 확정·조합 중 저장 0건. Windows WebView2도 별도로 실행하고 OS/IME/WebKit 버전을 기록한다. |
| 외부 갱신·충돌 | clean 상태의 CLI 갱신은 선택/scroll/비포커스 유지. dirty/IME 중에는 문서를 전체 교체하지 않고 로컬 초안과 수신본 보존. stale revision, 연결 파일 외부 수정/삭제/readonly에서 저장 거절·초안 유지. 자동 merge/LWW를 임의 도입하지 않는다. |
| 저장·undo/redo | 저장 전후 이력·task toggle·붙여넣기·외부 revision의 undo 경계를 확인한다. 사용자 undo가 외부 변경을 무심코 덮지 않아야 한다. 저장 중 추가 입력은 비활성화 또는 대기 초안으로 명시 처리. 실패 시 입력·선택·재시도 보존. |
| diff·완료 강조 | 추가·수정·삭제·quiet replace·새 task 완료를 각각 확인한다. 장식이 Markdown/undo에 혼입되지 않고 선택·입력만으로 수신 변경을 오확인하지 않는다. 읽은 뒤 포커스 이탈 확인과 dirty 중 보류를 구분한다. |
| 편집 DOM 안전성 | 붙여넣기 HTML/script/event handler, javascript URL, 외부 이미지 요청, raw HTML 입력. 코드 실행·외부 fetch·탐색 없음. 저장 후 Rust sanitize에서도 기존 계약 유지. 브라우저 안전성만으로 Tauri IPC 경계를 완화하지 않는다. |
| 선택과 이동 | 본문 click은 caret, drag는 선택, double/triple click·shift 선택·선택 복사 확인. 메모 이동은 툴바 빈 영역/접힌 제목으로 제한하는 제안을 시험한다. 버튼/SVG·표 resize·scrollbar·창 resize는 고유 기능 유지. 단일 클릭 이동은 별도 확정 전 구현하지 않는다. |
| 창 상태 | 고유 메모·instance·PID 출처를 확인하고 비포커스·접힘/펼침·숨김/복원·수동 resize·pin·내부 Feedback·너비 144px/300px를 시험한다. 배경 갱신이 포커스/위치/숨김/수동 크기를 바꾸지 않고 새 open이 포커스를 임의 탈취하지 않는다. |
| 업데이트 보호·단축키 | clean 상시 편집기로 updater busy가 영구 true가 되지 않아야 한다. dirty/composition/save 중 보호 유지. Cmd/Ctrl+C는 선택 복사 존중. Cmd/Ctrl+S/Esc는 확정된 저장 계약에 맞춰 검증한다. |
| 완전 오프라인·용량 | 네트워크 차단 상태에서 첫 시작·붙여넣기·task·표·코드가 동작하고 CSP 위반/외부 asset 요청 없음. 고정 version/lockfile로 재현. 필수 bundle·다중 메모 RSS/시작/입력/대형 문서 측정 후 결과를 사용자 판단에 제출한다. |

Tauri는 macOS에서 OS 제공 WebKit, Windows에서 WebView2를 사용하므로 Chromium E2E 통과를 WKWebView IME 통과로 대체하지 않는다. [Tauri 공식 설명](https://v2.tauri.app/reference/webview-versions/) 이번 후보 모두에서 이 앱의 실제 IME 적합성은 **미검증**이다. 기존 #27의 관리 창 IME도 이번 조사로 해결한 것으로 처리하지 않는다.

## 구현 전에 사용자가 결정할 사항

- 표시·편집 화면을 통합해도 명시적 저장·취소를 유지할지, 다른 저장 계약을 사용할지. 자동저장을 택하면 시점·실패·충돌 처리까지 별도 결정한다.
- ‘툴바를 클릭해서 이동’이 일반적인 누른 채 드래그인지, 클릭 뒤 포인터 추종인지. 우선 드래그를 권장하되 확정하지 않는다.
- 원문 문법 정규화를 어디까지 허용할지. 바이트 무손실이 필수라면 후보의 일반 serialize만으로 충족한다고 보지 않고 미편집 블록 원문 보존 등의 추가 설계를 검토한다.
- 기존 저장/내보내기 단축키·Esc·변경 자동 확인·편집 시작 높이를 상시 편집 표면에서 어떻게 정의할지.

## 검증과 제출 상태

- 확인: package/lockfile, Tauri/CSP, model/source, UI 편집·갱신·드래그·Feedback, focus/native drag, 디자인 시스템과 위 PNG, 후보 공식 문서/코드, npm latest/license/unpackedSize, GitHub archive 상태.
- 이슈 검색: WYSIWYG/위지윅/Milkdown/Tiptap/editor/마크다운/편집을 전체 상태에서 확인했다. 동일 선정 조사를 찾지 못해 #174를 만들었다. #15/#27/#31/#16/#65/#141/#159/#164는 인접 범위로 구분했다.
- 초기 clean detached HEAD에서 전용 `codex/research-wysiwyg-markdown`을 만들고 계획 커밋 `3924c45`만 push하여 Draft PR #175를 준비했다. 이 조사 문서는 요청대로 미커밋으로 제출한다.
- 미실행: 패키지 설치, bundle 측정, 단위/통합/E2E, 제품 빌드, 실제 앱 CLI/GUI/IME, Intel/Windows 실기기, 전체 전이 의존성 라이선스 감사. 문서 조사에 불필요한 빌드는 하지 않았다.
- 문서 검사: 후행 공백·로컬 링크 대상 존재·본문 언어 검사와 `git diff --check` 통과. untracked 조사 문서의 공백·링크는 별도 스크립트로 확인했다. Draft 상태와 계획 커밋만 포함된 PR 응답도 확인했다.
- Conf **1**: 코드·1차 자료 검토, 실행 검증 없음. Risk **Low**: 이번 조사 문서만 해당. 후속 구현은 원문·상태·저장 충돌에 영향이 있어 **Medium**으로 판단한다.
- [디자인 시스템](../design-system.md)의 확정 규칙과 SVG/PNG는 바꾸지 않았다. 채택 뒤 변경할 기준·UI·저장 계약은 구현과 같은 작업에서 문서와 필요한 시안을 갱신해야 한다.
