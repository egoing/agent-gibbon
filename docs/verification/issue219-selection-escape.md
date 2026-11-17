# 영역 선택 마스크의 첫 Esc (#219)

- 작업 루트는 /Users/egoinglee/.codex/worktrees/9fcb/agent-gibbon, 브랜치는 codex/39-selection-escape, 기준 커밋은 16d1b2addff53526651441535b2d2b101a5f4f20이다.
- `pnpm dev:info`에서 인스턴스 `dev-636b68445600a44490a881a7`, 바이너리 `src-tauri/target/debug/agent-gibbon`을 확인했다.

## 입력 경로와 변경

네 종류의 단축키는 Pressed에서 같은 선택 화면을 연다. 모든 모니터의 화면이 준비되면 한 창을 키 창으로 올리지만 기존 코드는 이때 키 입력을 받을 WebView를 지정하지 않았다. macOS에서 포인터가 있는 화면의 마스크를 우선하고 앱 활성화를 요청한 후 키 창을 설정하고, 플랫폼 공통으로 해당 WebView에 포커스를 준다. 전역 Esc 단축키는 추가하지 않는다.

기존 입력 핸들러에서 드래그 중 Esc 직후 pointerup을 전달하면 cancel_capture와 finish_capture가 모두 호출되는 것을 재현했다. 이제 취소를 요청하기 전에 진행 중 플래그를 설정하고 포인터 캡처와 선택 좌표를 해제한다. 개별 화면이 보내는 취소 명령은 기존 세대 검사와 전체 마스크 정리를 거친다.

## 실행한 검증

- 기존 구현에 추가한 두 회귀 검사가 각각 반복 취소 요청 2회, 취소 직후 생성 요청 1회를 검출했다. 수정 후 `node --test tests/capture.test.mjs` 8개가 통과했다.
- `pnpm test` Rust 303개와 UI 217개 통과, Rust 1개 ignored. `cargo fmt --check --manifest-path src-tauri/Cargo.toml`, `pnpm check`, `git diff --check`, `pnpm dev:build` 통과.
- 최종 수정본은 `pnpm dev:build`, Rustfmt, Clippy와 캡처 UI 테스트 8개를 다시 통과했다.

## macOS 실기기 비교 (2026-11-17)

위 루트의 `scripts/agent-gibbon-dev open/inspect`로 인스턴스 ID와 검증 메모의 ID·본문을 대조했다. 각 PID의 실행 경로와 CGWindowList의 메모 제목을 확인했다. 최종 실행 파일은 `lsof`의 text inode 402206508과 디스크 inode가 일치했다. 전체 화면을 촬영하지 않고 해당 PID의 마스크 창 개수와 포커스 PID를 기록했다.

| 빌드 | 프로세스 및 메모 | 관찰 |
|---|---|---|
| 기준 16d1b2a, SHA-256 `409a838d940209ff9683812e8ce188fb78cbcb592ec7405e611f1c4211830b24` | PID 5163, proof-39-base, 본문 proof-39-base-origin | 트레이에서 캡처 시작 후 마스크 3개. CGEvent로 누름·뗌을 연속 전송한 첫 Esc에 3개가 남았다. 마스크 클릭 뒤 같은 Esc에 0개가 되고 원래 앱(PID 32566)으로 복귀했다. |
| WebView 포커스만 수정 | PID 7493, proof-39-fixed | AXWebArea 포커스를 확인했으나 같은 무간격 Esc에 마스크가 남았다. |
| 포인터 화면 우선 + WebView 포커스, SHA-256 `3253ecdd2ab54b5a7cd73614936a71875bfb05cfe608d659486ca13afbb73109` | PID 15439, proof-39-fixed, 본문 proof-39-pointer-origin | 트레이로 각각 연 캡처·영역 텍스트에서 클릭 전 첫 Esc(합성 HID, 65ms 누름), Ctrl·Shift 플래그를 준 첫 Esc에 마스크 3개가 모두 닫혔다. 각 검사 후 메모 ID 목록이 동일했고 원래 앱 PID 32566이 전면으로 복귀했다. 무간격 CGEvent에서는 마스크가 남았다. |

누름 간격과 CGEvent source가 다른 두 합성 입력을 사용했다. 첫 검증에서는 기준본에 65ms 입력을 보내지 않았다. 아래 보충 검증에서 동일 조건으로 비교했다. 무간격 입력은 수정본에서도 실패해 클릭 전 모든 Esc 문제의 해소나 단일 원인을 확정할 수 없다. 실제 Ctrl+Shift+숫자 단축키의 독점 소유권을 확인하지 못해 트레이로 진입했다. 수정키 플래그 전송은 실제 단축키 해제·반복 입력의 검증을 대신하지 않는다.

제3자 Snipaste 창(layer 102)이 실제 트레이 좌표를 덮은 것을 확인한 뒤 화면 입력을 중단했다. 종료 시 자기 마스크 0개, 검증 메모 숨김, 자기 서버 종료를 확인했다. 첫 검증에서 남은 드래그 경합은 아래 보충 검증에서 확인했다. 터미널·웹 생성, 다른 앱에서 일반 Esc 입력, Windows 실행은 미검증이다.

## 동일 조건 65ms 보충 검증

같은 세 모니터의 프레임·배율, 같은 전면 앱(PID 32566), 같은 인스턴스와 트레이 ‘영역 캡처’를 사용했다. 각 실행 전에 마스크 0개를 확인했다. 마스크 3개와 해당 앱의 활성 상태를 확인한 뒤 동일한 HIDSystemState 도우미로 키코드 53의 down/up을 65ms 간격으로 한 번 전송했다. 마스크를 클릭하지 않았다. 준비 관측부터 도우미 호출까지 양쪽 모두 255.02ms였다. 아래 시간은 호출부터 창 목록 관찰까지로, 렌더링 지연의 정밀 측정값이 아니다.

| 빌드 | 준비 상태 | 첫 Esc 결과 |
|---|---|---|
| 기준 PID 28936 | AXWindow, 마스크 3 | 2.03초 뒤에도 3개가 남고 기준 앱이 계속 전면이었다. 결과 기록 뒤 별도 클릭+Esc로 정리했다. |
| 수정 PID 29520 | AXWebArea, 마스크 3 | 278ms 관측에서 마스크 0개, 새 메모 0개, 원래 앱 포커스 복귀를 확인했다. |

같은 수정본에서 다시 영역 선택에 들어가 드래그→Esc→mouseup과 Ctrl·Shift 플래그를 준 Esc를 각각 검사했다. 두 경우 모두 마스크 0개, 새 메모 0개, 원래 전면 PID 32566 복귀를 확인했다. 마지막 추가 반복 직전 Snipaste가 트레이 클릭 좌표를 점유해 입력 전 중단했다. 완료된 세 번의 선택은 모두 트레이로 진입했다. 실제 전역 단축키의 누름·해제와 0ms 합성 입력은 이 통과 범위에 포함하지 않는다.

실행 파일과 PID는 `ps` 및 `lsof -a -p PID -d txt`의 inode로 대조했다(기준 402512265, 수정 402512391). 두 실행 모두 `scripts/agent-gibbon-dev inspect --id proof-39-paired`의 ok, 인스턴스, 본문(39-paired-base/fixed)과 PID 소유 창 제목을 확인했다. 종료 후 자기 서버 0개와 최종 수정 바이너리 해시 일치를 확인했다. CGWindowList와 AX 조회만으로 창·포커스 상태를 기록해 화면 캡처 파일은 없다.
