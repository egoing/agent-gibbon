# Ctrl 단독 접촉 폴딩 — 2026-11-13

## 원인과 변경

기존 macOS 판정은 Control 포함 여부와 세 보조키 조합만 확인했고, Windows도 Ctrl 포함과 전체 접어두기 조합만 제외했다. 따라서 Ctrl+Shift, Ctrl+Option/Alt, Ctrl+Command/Win 등의 조합이 개별 폴딩을 실행할 수 있었다.

- `sizing.rs`: macOS의 device-independent flags가 Control과 정확히 일치할 때만 접촉 폴딩을 허용한다. Shift, Option, Command, Function, CapsLock, Help, NumericPad 상태를 제외하고 장치별 하위 플래그는 무시한다.
- `windows_contact.rs`: 양쪽 Shift를 포함한 현재 키 상태를 읽으며 Ctrl이 있고 Alt·Shift·Windows가 모두 없을 때만 허용한다. 기존 stash 조합 판정은 변경하지 않는다.
- `ui/app.js`: macOS 접기·펼치기 후 안내를 “Control만”으로 명확하게 했다.
- [디자인 시스템](../design-system.md)의 Ctrl 단독 접촉 기준을 갱신했다. 외형·배치는 바뀌지 않아 시안 변경은 없다.

## 입력 경로 확인

`ui/app.js`의 주기적 `poll_fold`, macOS의 전역/로컬 MouseMoved·Dragged·FlagsChanged 모니터, Windows의 16ms 네이티브 샘플링은 모두 `sizing::poll_fold`의 같은 조건을 거친다. Linux 등 미지원 플랫폼의 접촉 폴딩은 계속 비활성화다.

버튼의 `toggle_fold`/`chrome_fold`, 접힌 메모의 기본 클릭 펼치기는 보조키 접촉 제스처가 아니므로 기존 동작을 유지한다. 포커스된 창, 바깥→안쪽 재진입, 접힘 여백, capture/stash 억제 및 업데이트 안내 창 제외 조건도 유지한다. 다른 보조키를 창 안에서 놓아도 진입을 새로 만들지 않는다.

## 실행 환경과 검사

- 루트: `/Users/egoinglee/.codex/worktrees/02ec/agent-gibbon`
- 개발 인스턴스: `dev-f3ab8dc6a8d44e989b85031f`
- 바이너리: `/Users/egoinglee/.codex/worktrees/02ec/agent-gibbon/src-tauri/target/debug/agent-gibbon`
- macOS ARM64, Node v26.0.0, rustc 1.91.1.
- `cargo test --manifest-path src-tauri/Cargo.toml`: 215개 통과. 이번 macOS 추가 검사는 7개 추가 modifier의 128개 조합을 Ctrl 유무별로 확인하고 각 조합의 접촉·키 해제 후 지연 발동 방지·재진입을 검사한다.
- Windows의 실제 `Modifiers` 구조체·구현·테스트를 원본에서 임시 파일로 추출해 호스트 `rustc --test`로 실행: 2개 통과. Ctrl·Alt·Shift·Windows 16개 조합과 기존 stash 판정을 확인했다. Windows API 읽기·실행 검증을 대체하지 않는다.
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check`, `npm run check`(Clippy), `npm run dev:build`, `git diff --check`: 통과. 빌드만 수행했으며 앱 재시작은 하지 않았다.
- 최초 `npm run test:ui`: 병행 편집 중인 `memo-focus.test.mjs` 2개에서 `event.listen` mock 부재로 실패, 담당 세션에 전달. 이번 작업에서는 해당 파일을 수정하지 않았다. 병행 세션 수정 후 재실행에서 185개 전체 통과했다.

## 실제 앱 검증

병행 세션 GUI 사용 완료 후 최종 PID **83253**의 실행 파일 절대 경로를 `ps -p 83253 -o pid=,comm=`으로 확인했다. 앱 재시작·재빌드는 하지 않았다. 전용 메모 `ctrl-only-02ec-proof`, 고유 본문 `CTRL-ONLY-02EC-20261113 접촉 검증 전용`을 개발 CLI로 생성하고 `inspect`의 `ok:true`, 예상 instance, ID와 본문을 매 검사에서 대조했다. 같은 PID의 AX 창 제목 `Agent Gibbon [DEV detached@f343dbe · f3ab8dc6] — ctrl-only-02ec-proof`로 실제 창을 특정했다.

CoreGraphics 합성 키·포인터 이벤트를 실제 macOS 앱에 보내고 native modifier 상태와 개발 CLI의 `folded`·`focused`·크기·위치를 확인했다. 물리 키보드 수동 입력과 구분한다.

| 실제 입력 | 펼친 상태 | 접힌 상태 |
|---|---|---|
| Ctrl 단독 바깥→안 진입 | 접힘 | 펼침 |
| Ctrl+Shift 진입 | 유지 | 유지 |
| Ctrl+Option 진입 | 유지 | 유지 |
| Ctrl+Command 진입 | 유지 | 유지 |
| 각 추가 키를 창 안에서 해제 | 유지 | 유지 |

모든 경우 메모는 비활성이었고 접기·펼치기는 상태와 실제 native 크기에 반영됐다. 최종 앱에서 동작이 확인되어 이전 바이너리 결과로 대체하지 않았다. 안내 문구 `Control만`의 소스와 관련 UI 검사는 확인했으나 이번 실제 접촉 검증에서 안내 문구의 시각 표시는 별도로 검증하지 않았다.

검증 전 기존 메모들과 관리 창의 AX 위치·크기·두 편집 초안을 로컬에 스냅샷했다. 검증 메모만 별도 영역에서 사용하고 숨긴 뒤, 기존 창을 원래 기하학으로 복원하고 모든 기존 창의 위치·크기·초안이 스냅샷과 정확히 같은지 assert로 확인했다. 포인터 `(1060,475)`와 기존 frontmost PID `92046`도 복원·확인했다. 기존 앱 프로세스·설치 앱·사용자 메모 본문을 종료하거나 변경하지 않았다. 캡처는 생성하지 않았으며 [동작·출처 JSON](assets/ctrl-only-fold-20261113/evidence.json)에 전용 메모 결과만 저장했다. 사용자 초안 원문은 저장소 증거에 포함하지 않았다.

## 범위와 한계

macOS ARM64의 실제 개발 앱에서 합성 보조키·포인터 이벤트로 확인했다. Windows·Intel GUI와 물리 키보드 입력, macOS Function/CapsLock/Help/NumericPad의 실입력은 미검증이며 조합 판정 단위검사로 구분한다. 전체 플랫폼 빌드·CI 및 커밋·push·병합은 수행하지 않았다.

Conf **5**(macOS ARM64 실제 앱의 위 합성 입력 범위), Risk **Low**.
