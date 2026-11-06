# #65 노트 갱신 모드와 포커스·롤오버 자동 확인

## 구현 범위

- 기본 `diff`, 명시적 `replace`, replace의 `none`/`newly-completed` 강조를 CLI·IPC에서 검증한다. 옵션은 본문이 있는 update에만 허용하며 중복·잘못된 조합을 거절한다.
- 별도 확인 버튼을 제거했다. 해당 메모창의 포커스·롤오버와 이미 접촉/포커스 중인 창의 새 revision 수신을 자동 확인한다. 편집 중·숨김 상태는 제외하고 접힌 메모는 포함한다. 연결된 Feedback만 접촉하면 확인하지 않는다.
- 저장 성공과 revision 일치 후에만 비교 기준·알림을 해제한다. 실패 시 표시를 유지하며 오류를 안내한다. 다른 오류·복사 결과의 수명은 자동 확인으로 바꾸지 않는다.
- replace는 새 전면 배치·불투명 알림·diff를 만들지 않는다. 최신 완료 강조는 다음 실제 본문 갱신 때 교체하고 동일 본문·메타데이터에는 유지한다. 포커스·롤오버로 지우지 않고 저장소에는 남기지 않는다. 완료 Markdown 자체는 저장한다.
- 체크 항목은 파서의 실제 task marker로 식별하며 코드 블록은 제외한다. 상태를 제외한 행이 양쪽에서 고유할 때 연결한다. 재정렬·동시 여러 완료·중첩 목록을 지원하며 중복·신규 완료 행·문구/들여쓰기 변경은 추측하지 않는다. 식별용 행별 인덱스를 사용해 반복 전체 탐색을 피한다.
- 미확인 diff와 replace 혼합은 같은 행 수에서 기존 미확인 변경과 겹치지 않는 행만 비교 기준에 반영한다. 겹치거나 행 수가 달라지면 요청 전체를 거절한다. 기존 diff가 우선 표시되며 확인 후 완료 강조가 나타난다. 이 제한을 README·스킬·디자인 시스템에 명시했다.
- 기본 사용 스킬의 원본은 `.agents/skills/agent-gibbon/SKILL.md`, 진행 스킬은 `bundled-skills/agent-gibbon-progress/SKILL.md`다. `skills.rs`가 두 원본을 바이너리에 포함하고 앱의 기존 설치 revision 정책으로 배포한다. 사용자 전역 설치본을 이 작업에서 직접 덮어쓰지 않았다.

## 출처

- 루트: `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/diff`
- 브랜치: `egoing/diff`
- `npm run dev:info`: instance `dev-b09222c3d68057de28b6eb9c`, binary `<루트>/src-tauri/target/debug/agent-gibbon`.
- macOS ARM64, Node v24.21.0, Rust 1.91.1. 기존 설치 앱·다른 워크트리는 변경하지 않았다.
- 최초에는 개발 바이너리가 없어 진행 메모를 만들지 않았다. 개발 빌드와 네이티브 검증 후 이 인스턴스에 `issue65-progress-b092`를 만들었다. 사용자가 숨긴 메모를 다시 여는 갱신은 하지 않는다.

## 검증

| 명령 | 결과 |
|---|---|
| `cargo test --locked --manifest-path src-tauri/Cargo.toml` | Rust 161개 통과 |
| `npm run test:ui` | UI 98개 통과 |
| `cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 통과 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | 통과 |
| `npm run dev:build` | debug 개발 빌드 통과 |
| `python3 scripts/note-updates-smoke.py` | macOS 실제 개발 앱 통과, 아래 근거 |
| `python3 -m py_compile scripts/note-updates-smoke.py` | 통과 |
| `rsvg-convert docs/design/feedback-preview.svg -o docs/design/feedback-preview.png` | 렌더·PNG 시각 확인 |
| `git diff --check` | 통과 |

Rust 신규 검사는 A→B 연속 완료, 같은 본문 재전송, 체크 해제, 동시 여러 완료, 재정렬·중복·중첩·추가·코드 블록, 혼합 갱신과 충돌의 원자적 거절, 옵션·CLI 파싱, 재실행 시 일시 강조 제외/완료 상태 저장을 다룬다. 기존 revision 격리·저장 실패·누적 diff 회귀를 유지한다. UI 검사는 버튼 부재·롤오버/포커스 자동 확인·편집 초안 유지·숨김·이미 포커스/접촉 중 갱신·완료 강조의 독립성을 확인한다.

### 실제 창과 CLI

- 최종 실행 PID **49605**, 메모 `updates-2fcf3f89-a`·`updates-b5e3dce0-b`. 동일 PID의 메모 제목과 창 번호는 아래 JSON에 기록했다. 최종 실행은 `passed:true`, `test_server_stopped:true`다.

[실행 근거 JSON](issue65/evidence.json)은 CLI 응답의 `ok`, instance, 메모 ID와 고유 본문, lsof의 실행 파일 절대 경로, 서버 PID 및 동일 PID 창의 정확한 네이티브 제목·창 번호를 기록한다. 전체 화면 대신 출처가 확인된 창만 캡처했다.

- 검증용 메모 두 개에 diff를 만들고 확인 버튼이 없는 Feedback을 확인했다.
- A 롤오버로 A만 확인되고 B의 미확인 revision은 남았다. 전면 앱 PID·위치·크기·핀·가시성·포커스는 접촉 전후 같았다.
- A를 replace로 갱신한 뒤 새 알림·일시 불투명 효과 없이 이번 완료 B만 강조됨을 확인했다. 롤오버 후에도 강조와 완료 본문을 유지하고 Feedback은 숨겨졌다. [해당 창 캡처](issue65/completion.png)를 시각 확인했다.
- 테스트 메모에 포커스를 준 뒤 포인터를 밖으로 옮기고 본문을 갱신했다. 실제 focused 상태에서 새 변경이 자동 확인됐다.
- 숨긴 B를 갱신해 숨김과 미확인 변경이 유지됨을 확인했다.
- 스크립트는 기존 해당 서버가 있으면 중단하며, 자신이 시작한 정확한 실행 파일/PID만 종료하고 원래 포인터·전면 앱을 복원한다. 약관·진단 동의 기록과 전역 입력을 변경하지 않는다.

## 한계와 후속

- Windows·Intel의 실제 실행과 GUI, 실제 편집·접힘·Feedback 전용 접촉·원본 파일 저장 충돌의 모든 조합은 이번 네이티브 검사에 포함하지 않았다. 해당 범위는 자동 회귀 또는 기존 동작 보존이며 현재 실환경 통과로 보고하지 않는다.
- 미확인 diff와 조용한 구조 변경/중첩된 변경을 자동 병합하지 않는다. 사용자 확인 후 replace하거나 중요한 변경을 diff로 갱신한다.
- 커밋·push·원격 전체 플랫폼 검사는 아직 실행하지 않았다. AGENTS.md의 커밋 승인 지침에 따라 로컬 변경과 검증을 먼저 제시하고 승인을 받는다. 승인 후 대상 브랜치/SHA를 push하고 창 동작·상태 모델 변경에 대한 전체 release 수동 CI를 요청한다. 태그·공개 배포는 별도 범위다.
- **Conf 5: 위 macOS 실제 검증 범위, Risk Medium: 비교 기준과 상태 전이 변경.**

[디자인 시스템](../design-system.md) · [SVG 시안](../design/feedback-preview.svg) · [PNG 시안](../design/feedback-preview.png).

## 사용자 요청에 따른 사례별 재실행

- 같은 작업 루트에서 `npm run dev:build`를 다시 확인하고 개발 CLI로 앱을 시작했다. instance `dev-b09222c3d68057de28b6eb9c`, PID **17144**의 실제 바이너리 경로를 lsof로 확인했다.
- 고유 접두사 `demo65-c0eb10`의 안내 메모와 사례 5개를 표시했다. 일반 diff, 강조 없는 replace, 동시 두 항목 완료, A→B 연속 완료, 미확인 diff와 완료 체크 혼합을 실제 CLI로 생성했다.
- 각 inspect의 instance·ID·본문·가시성·highlight revision·completion HTML을 확인하고 동일 PID 및 정확한 네이티브 제목으로 창을 특정해 개별 캡처했다. 다섯 캡처를 시각 확인했다.
- [라이브 사례 출처](issue65/live-demo.json), [일반 변경](issue65/01-diff.png), [조용한 교체](issue65/02-plain.png), [동시 완료](issue65/03-complete.png), [연속 완료](issue65/04-next.png), [혼합](issue65/05-mixed.png).
- 사용자 확인을 위해 앱과 여섯 메모를 표시한 채 유지했다. 원래 포인터·전면 앱은 보존했다. 사용자가 사례 ①·⑤에 포커스/롤오버하면 해당 diff가 해제되며, 이 사용자 후속 조작은 준비 시점의 캡처와 구분한다.
