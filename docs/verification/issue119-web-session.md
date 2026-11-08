# #119 웹 세션 도구 모음 검증

## 구현

- 웹 Welcome과 외부 웹 페이지에 부모 창에 연결된 로컬 핀/닫기 companion 창을 추가했다. 이동·크기 변경 시 위치를 맞추고, 숨김·최소화 상태도 부모와 동기화한다.
- 핀은 부모 웹 창과 도구 모음의 상단 고정을 함께 변경한다. CLI `close`, 네이티브 닫기, 도구 모음 닫기는 웹 세션과 두 창을 종료한다.
- 외부 페이지 DOM은 수정하지 않는다. 로컬 Welcome과 도구 모음만 업데이트 준비 이벤트를 수신하며, 외부 `webpage-*` 레이블은 권한에서 제외한다.

## 검증

- `node --test tests/web-welcome.test.mjs tests/updater-notice.test.mjs` — 통과.
- `cargo test --manifest-path src-tauri/Cargo.toml web::tests` — 로컬 unit tests 통과.
- 디자인 시안 SVG를 PNG로 렌더링하고 확인했다.
- macOS Intel 실앱 GUI 확인: 워크트리 `/Users/egoing/.codex/worktrees/imac-102/agent-gibbon`, 개발 인스턴스 `dev-08207552ee001b9318f23e11`, debug PID `74686`, 메모 `imac119-origin`의 inspect 본문 표식으로 출처를 확인했다. 네이티브 창 제목 `웹 도구 — web-imac119-test`에서 상단 고정을 눌러 `always_on_top`이 `true → false`로 바뀌는 것을 확인하고, 웹 세션 닫기 후 `list`에서 ID가 사라지는 것을 확인했다.
- 여러 화면·플랫폼에서 companion 위치/포커스, Welcome 페이지의 네이티브 종료는 별도 미검증이다.
