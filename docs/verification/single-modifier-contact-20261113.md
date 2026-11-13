# 단독 보조키 접촉 제스처 — 2026-11-13 후속

## 변경

Option/Alt 접촉 이동도 키가 포함돼 있는지만 확인해서 Ctrl·Shift·Command/Windows가 섞여도 이동할 수 있었다. macOS의 단독 보조키 판정을 `warp::only_modifier_macos`로 공유하여 Ctrl 접기·펼치기와 Option 이동 모두 device-independent flags의 정확한 일치를 요구한다. Windows는 `Modifiers::alt_only`로 Ctrl·Shift·Windows 혼합을 제외한다. 양쪽 보조키 샘플링을 유지한다.

- `src-tauri/src/warp.rs`: Option 단독 판정과 128개 추가 보조키 조합·접촉 재진입 회귀검사.
- `src-tauri/src/sizing.rs`: 기존 Ctrl 정확 판정을 같은 macOS 함수로 연결.
- `src-tauri/src/windows_contact.rs`: Alt 단독 판정 및 Windows 16개 조합 검사 확장.
- `ui/app.js`: 이동 후 안내를 “Option만”으로 명확하게 변경.
- [디자인 시스템](../design-system.md)의 단독 보조키 접촉 기준을 갱신했다. 외형·배치 변경은 없다.

대상은 단독 보조키+포인터 제스처다. 버튼·일반 복사/저장/캡처 단축키와 여러 키 조합이 명세인 전체 접어두기·투명도 조절은 유지한다. 다른 보조키를 안쪽에서 놓아도 그 자리에서 이동하지 않으며 새 바깥→안쪽 진입이 필요하다. capture/stash·포커스 억제도 유지한다.

## 검사

- `cargo test --manifest-path src-tauri/Cargo.toml`: **216개 통과**. 기존 Ctrl 128조합과 추가 Option 128조합에서 다른 modifier 유무, 키 해제 후 지연 실행 방지와 새 재진입을 검사한다.
- Windows 원본 `Modifiers` 구조체·메서드·테스트를 임시 파일로 추출해 호스트 `rustc --test` 실행: **2개 통과**. Ctrl·Alt·Shift·Win 16조합에서 Ctrl 단독·Alt 단독과 기존 stash 판정을 확인한다. Windows API·GUI 실행 증거와 구분한다.
- `npm run test:ui`: **185개 통과**.
- Rustfmt·Clippy(`npm run check`)·개발 빌드(`npm run dev:build`): 통과.

## 실제 앱 출처와 입력 검증

- 작업 루트 `/Users/egoinglee/.codex/worktrees/02ec/agent-gibbon`, 인스턴스 `dev-f3ab8dc6a8d44e989b85031f`.
- 바이너리 `/Users/egoinglee/.codex/worktrees/02ec/agent-gibbon/src-tauri/target/debug/agent-gibbon`.
- macOS ARM64, Node v26.0.0, Rust 1.91.1.
- 이전 PID92340은 최종 빌드보다 먼저 시작돼 검증에 사용하지 않았다. 병행 세션의 GUI 해제 뒤 해당 PID의 절대 실행 경로를 확인하고 그 개발 앱만 재시작했다. 새 PID **96953**을 위 바이너리 경로와 대조했다. 설치 앱이나 다른 워크트리 프로세스를 종료하지 않았다.
- 전용 메모 `single-modifier-02ec-proof`, 본문 `SINGLE-MODIFIER-02EC-20261113 검증 전용`. 매 단계 `inspect`의 `ok:true`, 예상 instance·ID·고유 본문과 같은 PID의 AX 네이티브 창 제목을 대조했다.
- 실제 개발 앱에 CoreGraphics 합성 키·포인터 이벤트를 보내고 native modifier 상태, 실제 창 위치 및 `folded`를 확인했다. 물리 키보드 입력과 구분한다.

| 입력 | 결과 |
|---|---|
| Option+Ctrl 진입 | 위치·펼침 유지 |
| Option+Shift 진입 | 위치·펼침 유지 |
| Option+Command 진입 | 위치·펼침 유지 |
| 각 추가 키를 창 안에서 해제 | 위치 유지 |
| Option 단독 재진입 | 반대편 이동, 펼침 유지 |
| 다시 Option 단독 재진입 | 반대쪽 복귀 |
| Ctrl 단독 재진입 2회 | 접기·펼치기 정상 |

[실행 증거 JSON](assets/single-modifier-contact-20261113/evidence.json). 화면 캡처는 만들지 않았다.

## 사용자 상태 보존과 한계

재시작 직전의 모든 보이는 메모·관리 창 AX 위치·크기, 두 편집 초안과 Ctrl 검증 메모의 접힘 상태를 로컬에 저장했다. 재시작 후 초안·접힘을 복원하고, 관리 창은 트레이의 “메모 관리”로 복원했다. 첫 복원 시 관리 창 미생성과 AX 기하학의 일시적인 차이를 발견해 복원 완료로 보고하지 않았으며, 관리 창 재생성 및 재대조 후 모든 기존 창의 위치·크기·초안 일치와 접힘 상태 일치를 assert로 확인했다. 최종 검증 뒤 전용 메모만 숨기고 다시 전체 일치를 확인했다. 원래 포인터 `(1568.5,662)`와 frontmost PID15685도 복원했다. 사용자 초안 원문은 저장소 증거에 넣지 않았다.

Windows·Intel 실제 앱, 물리 키보드, macOS Function/CapsLock/Help/NumericPad의 실제 입력은 미검증이다. 여러 보조키 조합은 단위검사와 위 세 혼합 실앱 사례로 범위를 구분한다. 전체 플랫폼 CI·설치 파일 빌드·커밋·push·병합은 수행하지 않았다.

Conf **5**(macOS ARM64 실제 앱의 위 합성 입력 범위), Risk **Low**.

## 커밋 대상 분리 검증

병행 작업의 테두리·Feedback·hover 변경을 제외하고 스테이징한 트리만 임시 디렉터리로 추출해 검사했다. `cargo test` **215개**, `node scripts/test-ui.mjs` **183개**가 통과했다. 위 216/185개는 병행 작업을 포함한 당시 작업 사본의 수치이며, 이 분리 검사 결과와 구분한다. 관련 이슈는 [#145](https://github.com/egoing/agent-gibbon/issues/145)다.
