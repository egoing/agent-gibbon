# 개발 인스턴스 약관 확인 생략 — 2026-11-06

## 동작과 범위

- debug 빌드이면서 유효한 개발 인스턴스 ID가 있는 실행만 약관의 기능 진입 제한을 기본 생략한다. 설치 앱·기본 인스턴스·release 빌드의 약관 제한은 유지한다.
- 실제 `terms_accepted`와 `terms_skipped`를 구분한다. 생략 경로는 동의 파일을 생성·변경하지 않으며, 진단 허용은 기존의 현재 정책 수락·별도 전송 동의 조건을 그대로 사용한다.
- 사용자가 약관 검증을 요청하면 시작 전에 `AGENT_GIBBON_REQUIRE_TERMS=1`을 설정한다. 실행 중인 앱에는 소급 적용하지 않고 현재 정책의 기존 수락 기록도 지우지 않는다. 예: `AGENT_GIBBON_REQUIRE_TERMS=1 npm run dev`.
- 관리 화면은 개발 기본 실행에서 일반 탐색을 허용한다. 설정의 최초 진단 전송 저장은 기존 약관·선택 동의 화면을 명시적으로 거친다. 생략을 실제 수락 체크로 표시하거나 설정 저장으로 수락 기록을 자동 생성하지 않는다.
- 외형·배치 변경은 없으며 기존 SVG·PNG를 유지한다. [디자인 시스템](../design-system.md#개발-인스턴스-약관-확인-생략-2026-11-06)을 함께 갱신했다.

## 검증 환경과 출처

- 소스 루트: `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/global-memo-opacity-wheel`
- `npm run dev:info` ID: `dev-67cc0e13d61c6b98b34c3f11`
- 빌드 바이너리: 위 루트의 `src-tauri/target/debug/agent-gibbon`
- macOS ARM64, Node v24.21.0, npm 11.19.0, Rust 1.91.1.
- 기존 앱과 열린 메모를 유지하기 위해 동의 파일을 조작하거나 기존 프로세스를 재시작하지 않았다. 실행 중인 기존 앱에는 이번 빌드가 소급 반영되지 않는다.

## 자동 검사

| 명령 | 결과 |
| --- | --- |
| `npm test` | Rust 137개·UI 84개 통과 |
| `npm run check` | 전체 target Clippy 경고 없음 |
| `npm run dev:build` | 현재 워크트리 debug 빌드 성공 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | 통과 |
| `git diff --check` | 통과 |

Rust 검사는 debug/개발 인스턴스의 조합과 명시적 약관 검증 설정을 확인한다. 기존 동의·손상·이전 정책·진단 전송 경계 검사도 함께 통과했다. UI 검사는 개발 생략 시 일반 탐색 허용·약관 체크 해제·동의 저장 없음, 최초 진단 선택 시 명시적 약관 수락, 생략 해제 시 Welcome 표시를 확인했다.

## 실제 CLI 검증

기존 워크트리의 사용자 동의를 건드리지 않도록 임시 루트에 현재 `dev.mjs`·`dev-instance.mjs`를 복사하고 `src-tauri`는 이 소스 루트로 연결했다. 임시 개발 실행기의 `info`로 별도 ID를 계산했다. 모든 메모 명령은 이 실행기를 사용하며 바이너리 출처 검사는 심볼릭 링크를 해소한 현재 워크트리의 `manifest_dir`와 대조했다. 빌드·소스는 현재 워크트리이고 동의·메모·소켓 namespace만 별도인 실행이다. 별도 워크트리 격리 회귀를 수행한 것으로 간주하지 않는다.

- 실행: 임시 루트의 `node scripts/dev.mjs cli --serve` 후 같은 실행기로 `diagnostics`, `open`, `inspect`, `close`.
- 임시 루트·ID·바이너리·각 서버 PID·응답: [JSON 실행 근거](dev-terms-skip-evidence-20261106.json).
- 메모 ID: `dev-terms-smoke`, 본문 표식: `DEV-TERMS-SKIP-20261106`.
- 두 실행 모두 `diagnostics.instance`를 예상 ID와 대조하고 서버 자식 PID의 `lsof -d txt` 실행 경로를 현재 워크트리 바이너리와 대조했다. 기본 생략 실행의 `inspect`는 `ok:true`, 동일 ID, 메모 ID와 본문 표식을 확인했다.

| 실행 | 약관 수락 | 생략 | 기능 진입 | 진단 전송 허용·동의 파일 생성 |
| --- | --- | --- | --- | --- |
| 기본 debug 개발 | false | true | open·inspect·close 성공 | false·없음 |
| `AGENT_GIBBON_REQUIRE_TERMS=1` | false | false | open이 `onboarding_required:true`로 차단 | false·없음 |

두 실행의 SDK 상태는 비활성이었다. 실행 환경의 `AGENT_GIBBON_TELEMETRY=off`도 사용했으므로 이 관측만으로 유효 DSN 환경의 무전송까지 입증했다고 간주하지 않는다. 별도 전송 동의 판정이 false인 것과 기존 전송 경계 자동 검사를 함께 확인했다. 테스트 프로세스 그룹만 종료하고 해당 임시 namespace의 메모·IPC 파일과 임시 루트를 정리했다.

## 한계

- 실제 native Manager 화면 조작·캡처, macOS Intel·Windows 실행, release 바이너리 실행은 수행하지 않았다. 해당 분기는 단위·UI 검사와 코드 검토 범위다.
- 외부 진단 서버로 전송하거나 사용자 수락·전송 동의를 대신 저장하지 않았다.
- Conf 5: macOS ARM64 실제 CLI의 기본 생략·명시적 제한. Manager 표시·선택 동의 화면 전환은 Conf 3(UI 자동 검사). Risk Low: debug 개발 namespace의 진입 변경이며 배포 제한·진단 동의 조건은 유지한다.
