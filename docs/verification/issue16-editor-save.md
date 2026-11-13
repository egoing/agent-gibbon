# #16 편집 저장 단축키 검증

2026-11-13 (Asia/Seoul). 관련 [이슈 #16](https://github.com/egoing/agent-gibbon/issues/16) · [Draft PR #156](https://github.com/egoing/agent-gibbon/pull/156).

## 작업 출처와 승인 경계

- root: `/Users/egoinglee/.codex/worktrees/3e31/agent-gibbon`
- branch: `codex/editor-save-shortcut`
- 기준 origin/main: `9904f10770d552b8f90ceb8073415d9875c09062`
- 첫 계획 HEAD: `d7ddbd8d8035bd9ca91884e9a04925ff7d605aaa` (계획만 push). 아래 구현 검증은 이 HEAD 위의 로컬 미커밋 변경에 해당한다.
- `npm run dev:info`: instance `dev-0d8853e5e61d9b4182d09a6a`, binary `/Users/egoinglee/.codex/worktrees/3e31/agent-gibbon/src-tauri/target/debug/agent-gibbon`.
- 최종 구현 커밋·push·병합·공개·전체 플랫폼 CI는 실행하지 않는다. 다른 워크트리의 코드·앱·빌드는 변경하지 않는다.

## 변경과 로컬 검사

본문과 Feedback의 저장 단축키는 메모 창의 현재 편집 상태에 따라 기존 `save_note` 또는 `export_note`를 호출한다. Feedback의 저장 단축키는 별도 `save-shortcut` 이벤트로 전달하여 지연된 Action Bar 상태로 저장 여부를 결정하지 않는다. 저장·원본 갱신·revision 및 원본 파일 충돌 검사는 기존 `save_edit`를 그대로 사용한다.

편집 Action Bar는 `파일 내보내기`·`취소`·`메모에 반영`을 표시한다. 내보내기는 초안의 별도 파일이며 메모 반영과 구분한 완료 문구를 표시한다. 저장·내보내기 중 중복 요청과 취소를 막으며 실패 시 편집기·초안을 유지한다. IME·반복 키·Alt/Shift 혼합 입력은 저장하지 않는다. 렌더러·체크박스 로직은 변경하지 않는다.

설치 도구: macOS ARM64, Node v24.21.0, npm 11.19.0, rustc/cargo 1.91.1. `npm ci` 성공, lockfile 변경 없음.

- `npm run test:ui`: 194개 통과. 본문/Feedback의 저장·내보내기 경로, 플랫폼별 Command/Ctrl 표시, 취소 후 읽기와 이미지 내보내기, IME/반복/추가 보조키, 실패·재시도 초안 보존 및 내보내기 중 저장·취소 차단 검사.
- `PLAYWRIGHT_MODULE=/tmp/gibbon-playwright/node_modules/playwright/index.mjs node scripts/editor-save-smoke.mjs`: Chromium 158.0.8127.18에서 144/300px 통과. 세 버튼 범위·Tab 순서·접근성 이름·Feedback 단축키 전달·IME/반복 차단·긴 오류 갱신 후 포커스 보존 확인. 브라우저의 IPC는 mock이며 실제 OS 저장/파일 검증을 대신하지 않는다.
- [144px 브라우저 캡처](assets/issue16-feedback-144.png) · [300px 브라우저 캡처](assets/issue16-feedback-300.png). 144px 줄바꿈 렌더를 육안 확인했다.
- `rsvg-convert docs/design/editor-save-shortcut.svg -o docs/design/editor-save-shortcut.png`: 성공. PNG 렌더 육안 확인. 설명용 시안이며 실앱 캡처가 아니다.
- `npm run dev:build`: 성공. 전용 debug 바이너리 빌드.
- `cargo test --manifest-path src-tauri/Cargo.toml`: 최종 217개 통과. `model::tests::editor_saves_source_and_rejects_stale_or_invalid_changes`의 실제 연결 원본 쓰기·stale revision/외부 변경 거부, export의 실제 UTF-8/PNG bytes 쓰기 및 실패 시 대상 보존 검사 포함.
- `npm run check`: Clippy `-D warnings` 통과. Rustfmt 및 `git diff --check` 통과.

## macOS ARM64 native 검증

팀리더의 #140 GUI 해제 뒤 전용 슬롯을 사용했다. PID `39599`의 실행 파일 절대 경로가 위 binary와 일치하며 `scripts/agent-gibbon-dev inspect`가 `ok: true`, 예상 instance 및 고유 ID·본문을 반환하는 것을 확인했다. 네이티브 제목은 `Agent Gibbon [DEV codex/editor-save-shortcut · 0d8853e5] — issue16-3e31-source` 및 `… — issue16-3e31-independent`, Feedback은 같은 제목의 ` · Feedback` 접미사다. 각 메모 창 번호 `68553`/`68576`, Feedback `68564`/`68577`을 PID·inspect·native 제목과 대조했다. [검증 상태·AX 원문](assets/issue16-native.json).

| 검증 | 실행·결과 |
|---|---|
| 연결 원본 저장 | `/tmp/gibbon16-native-3e31/source.md`를 전용 CLI `open --id issue16-3e31-source --file …`로 연결. 편집 AXSetValue 뒤 본문 창에 `orca computer hotkey --app pid:39599 --window-id 68553 --key CmdOrCtrl+S` 전달. inspect revision 1→2 및 Markdown, 실제 테스트 원본 파일의 동일 내용 갱신 확인 |
| 외부 원본 충돌 | 테스트 원본만 외부 변경 후 본문 Command S. revision 2와 저장 본문 유지, 원본 외부 변경 유지, 편집 초안 `issue16-3e31 충돌 초안` 유지 및 원본 변경 오류 확인 |
| 독립 메모 | `open --id issue16-3e31-independent --markdown …` 후 본문 Command S. source_path null, revision 1→2 및 새 본문, `메모에 반영했습니다` 안내 확인. 초안의 ‘Feedback 반영’ 문자열은 테스트 데이터이며 Feedback 단축키 성공을 뜻하지 않음 |
| CLI revision 충돌 | 편집 중 전용 CLI `update --id issue16-3e31-independent --markdown …`로 revision 변경 후 본문 Command S. CLI 본문/revision 3을 보존하고 메모 외부 갱신 오류, 기존 편집 초안 유지 확인 |
| 취소 후 읽기 내보내기 | 본문 Escape로 편집 종료 후 Command S. AX에서 `파일로 저장` native save sheet와 Markdown 파일명 확인. Cancel로 닫힌 뒤 읽기 본문/revision 유지 |
| 명시적 초안 내보내기 | `swift scripts/design-system-ax.swift 39599 '<정확한 source Feedback 제목>' click '파일 내보내기'` 사용. 부모 창의 native save sheet 생성 확인 후 Cancel. AXTextArea의 초안과 기존 메모/revision·테스트 원본 외부 변경 유지 확인. 실제 내보내기 파일 완성은 미실행 |

[원본 충돌 Feedback 실제 캡처](assets/issue16-native-conflict.png)는 window `68564`의 단독 캡처이며 원본 변경 오류·세 버튼·접근성 이름을 확인했다. PNG 육안 확인 완료. `orca computer list-windows`는 빈 배열이었지만 get-app-state의 CG fallback으로 부모 및 큰 오류 Feedback의 PID·창번호·제목을 확인했다.

Feedback 조작은 초기에 `background_unavailable`, 작은 독립 Feedback은 `window_not_found`가 발생했다. 큰 source Feedback의 정확한 제목·창번호로 재선택한 합성 Command S도 저장 결과의 변화를 입증하지 못했으므로 **Feedback native 단축키는 통과로 보고하지 않는다**. 기존 PID/정확한 제목 AX helper로 명시 내보내기 버튼만 검증했다. Feedback 단축키 라우팅은 Chromium·VM·Rust 테스트에서 검증한 범위다.

모든 Orca 입력은 background였으며 foreground 복원·포인터 이동·클립보드 조작을 하지 않았다. 시작 전 frontmost 기준 조회는 실패했으므로 포커스 동일성을 확정하지 않는다. 종료 후 frontmost 조회는 ChatGPT였다. 다른 앱·워크트리와 사용자 파일·메모는 수정하지 않았다. 두 테스트 메모의 `close`와 `visible: false`를 확인한 뒤 정확한 실행 파일/PID를 재조회하고 이번에 시작한 PID `39599`만 TERM으로 종료했다. GUI 슬롯 해제를 팀리더에게 보고했다. 테스트 데이터는 위 `/tmp` 경로에 보존한다.

## 디자인 기준

[디자인 시스템](../design-system.md#파일-저장과-복사-통합), [시안 SVG](../design/editor-save-shortcut.svg) · [PNG](../design/editor-save-shortcut.png). 편집 단축키 계약, Action Bar의 세 버튼·줄바꿈, 툴팁/접근성 및 결과 문구를 갱신했다. 과거 시안 전체를 다시 그리지 않고 이번 변경 범위를 독립 시안으로 연결했다.

## 한계

Windows·Intel native GUI, 물리 키보드 IME 입력, Feedback native 저장 단축키, 이미지 메모 native 내보내기 및 실제 native 내보내기 파일 완성은 미검증이다. 이미지 읽기 단축키는 VM, PNG 쓰기는 Rust 테스트에서 확인했다. 브라우저/VM 검사는 실제 WebView 키 입력을 대신하지 않는다. 전체 플랫폼 CI·릴리스·설치 검사는 이번 범위에서 실행하지 않는다. 최종 구현은 미커밋으로 보존한다.

확신도: 검증한 macOS 본문 저장·충돌 보호·시트/취소 경로 Conf 5, Feedback 단축키·이미지 경로는 자동 검사 범위 Conf 3~4. 위험도: Medium(저장 단축키와 연결 원본 반영 동작).

## 2026-11-14 커밋·병합 승인

사용자가 저장 단축키 변경의 커밋·push·병합을 승인했다. 위 승인 대기 표기는 최초 검증 당시 상태다. 새 package-lock.json의 정렬 변경은 이번 작업에서 제외하고 보존한다. 최신 main 통합 후 관련 회귀 검사를 다시 수행한다.
