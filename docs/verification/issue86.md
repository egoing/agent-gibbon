# #86 Windows 외부 앱 검증

## 범위와 기준

- 이슈: [#86](https://github.com/egoing/agent-gibbon/issues/86), PR: [#88](https://github.com/egoing/agent-gibbon/pull/88).
- Mac 작업 루트: `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/86-windows-floating-apps`, 브랜치 `86-windows-floating-apps`.
- 최초 base ref: `refs/is2/base/86` = `48041c5e3dcdb6db98e136d0098ab30a6343666a`.최신 origin 값으로 덮어쓰지 않았다.
- red ref: `refs/is2/red/86` = `edad8f93ef5badb881322c3a68703573344d0c43`.
- macOS 15.6.1 ARM64, Node 24.21.0 / npm 11.19.0, rustc/cargo 1.91.1, gh 2.89.0, Orca 1.4.245. `npm ci` 수행.
- 개발 인스턴스: `dev-6e5267484ea1b5b980acd2b4`, binary는 작업 루트의 `src-tauri/target/debug/agent-gibbon`.
- 위험도 Medium: 외부 프로세스의 창 상태·전역 단축키를 다룬다. 자동 병합하지 않는다. 태그·공개 릴리스는 수행하지 않는다.

## 구현 전 baseline과 TDD

구현 전 최초 SHA에서 아래 모두 exit 0. baseline 제외 0개다. 원본 로그와 metadata/results JSON은 `/Users/egoinglee/.codex/artifacts/agent-gibbon-is2-86/baseline/`에 보존했다.

| 항목 | 실제 명령 | 결과 |
|---|---|---|
| 회귀 | `npm test` | PASS |
| 타입 | `cargo check --locked --manifest-path src-tauri/Cargo.toml --all-targets` | PASS |
| 린트 | `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` 및 `cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | PASS |
| 빌드 | `npm run build` | PASS, macOS app 생성; 공증은 별도 구성 |

신규 CLI 테스트 3개 중 2개는 `app` 명령 미지원 때문에 실제 assertion 실패했으며 구성/의존성 오류가 아니었다. `/tmp/is2-86-red.log`에 원본을 보존했다. red 커밋은 테스트와 등록만 포함한다. 구현 후 같은 테스트 3개가 모두 통과했고 외부 앱 UI 행동 테스트 5개와 영역 유효성 테스트를 추가했다.

## Windows 선행 게이트

[PoC 전체 보고서](issue86-windows-poc/README.md)와 원시 로그·실제 시험 창 캡처를 대조했다. 별도 소스 SHA `37dbfed28deac5e01f9764db9b352a8f2495bda4`, High→High, 단일 125% 화면의 외부 WinForms 두 프로세스에서 위치/크기·일반 창 활성화 후 topmost·실제 클릭/키보드·해제/복원 네 항목 PASS. 해당 실행과 실패 네 실행은 구분하여 보존했다. 제품 구현은 그 뒤에 시작했다. 이 증거를 제품/UI/표준 권한·모든 앱 지원 증거로 확대하지 않는다.

## 최종 제품 검증과 출처

검증용 제품 소스는 `27e5321049ad42b2e75848221ac27f9990df3055`다. 시도 이력 `3009e0b619fdbfc5ed38f065ad7b5ef05ef9e7f7`을 ref에 보존한 뒤 최초 기준으로 한 번 squash하고 최신 main `047858207aa36f31baac3870bb1c14a03f887242`에 rebase했다. 충돌은 디자인 문서 선두 두 절을 모두 보존하여 해결했다. 외부 앱·캡처·Welcome 소스는 에스컬레이션 수정본과 byte 동일하며 #84의 관리 창 활성화 변경/관련 문서·증거만 upstream에서 유입됐다.

| 항목 | Mac 부모 재실행 | Windows 실기 작업자 |
|---|---|---|
| 신규 테스트 | CLI/영역 4개 + UI 5개 PASS | Windows 전용 수명 감시 테스트와 제품 4종 실증 PASS |
| 회귀 | Rust 173 / UI 124 PASS | Rust 178 / UI 123 PASS, 기존 1 SKIP |
| 타입 | cargo check --locked --all-targets PASS | cargo check --locked PASS |
| 린트 | Rustfmt / Clippy -D warnings PASS | Rustfmt / Clippy -D warnings PASS |
| 빌드 | macOS app / dev-build PASS | Windows dev-build PASS; 설치 파일은 전체 플랫폼 CI에서 검사 |

Mac 원본 명령/exit/log는 `/Users/egoinglee/.codex/artifacts/agent-gibbon-is2-86/rebased/results.json` 및 같은 폴더에 있다. Windows의 정확한 OS/도구/소스 SHA/instance/PID/exe/HWND, 위치·입력·복원 관측과 캡처는 [전체 제품 보고서](issue86-product-windows/README.md) 및 `final-result.json`을 따른다. API 반환과 실제 관측을 구분한다. 이전 SHA의 실행을 최신 실행으로 재사용하지 않는다.

Windows 기존 1 SKIP은 TAP 12, `tests/dev-instance.test.mjs:26`의 `development CLI rejects missing, old and foreign binaries without release fallback`이며 원래 `POSIX executable fixtures; Windows uses native CLI smoke tests` 조건이다. 신규 테스트를 skip한 것이 아니다. baseline 제외는 0개다. [실패·수정·단일 에스컬레이션](issue86-attempts.md) 및 [에스컬레이션 원본](issue86-escalation/README.md)을 보존한다.

Mac 진행 메모 `issue86-progress`는 최초 baseline 개발 빌드에서 시작한 PID 65061 / 위 워크트리 binary에 속한다. inspect의 instance·ID·본문을 확인하고 update만 사용했다. 사용자 위치·숨김을 보존하기 위해 앱을 재시작하지 않았으며, 이 진행 메모를 최신 제품 GUI 실행 증거로 사용하지 않는다. Mac의 새 외부 앱 CLI 오류/비활성 트레이 실제 UI는 별도 실행하지 않았고 소스 분기·타입·빌드만 검사했다.

## 독립 감사와 한계

[독립 감사 보고서](issue86-audit.md)의 검토자는 구현 이력을 상속하지 않고 원문 요구사항, base/red/attempts ref·전체 diff·실행 로그·시안을 검토했다. 테스트 완화·스코프 이탈은 미발동이며 발견한 기능 경합/시작 폴더/복원 대기 문제를 보완한 후 재검토했다. 최신 Windows 원본 관측과 전체 플랫폼 CI도 별도로 대조하며 최종 판정/실행 URL/headSha는 [PR #88의 검증 표](https://github.com/egoing/agent-gibbon/pull/88)에 기록한다. CI 미완료를 PASS로 표시하지 않는다.

Windows 제품 필수 네 항목 및 Welcome의 선택 후 실행·기존 창 재선택·한 줄 작업 버튼을 확인했다. 같은 High 토큰·단일 125% 화면의 전용 WinForms 앱 범위다. 실제 트레이 해제/종료 클릭, 실제 HWND/PID 재사용, 직접 attach와 닫기의 경합 중첩, 혼합 DPI/복수 모니터·Medium 권한·UAC 경계 및 기본 설치 인스턴스와 동시 실행은 별도 미검증이다. 소유 표식 불일치는 전용 창에서 시뮬레이션했으며 실제 handle 재사용 재현으로 부풀리지 않는다. 타인 Gibbon 인스턴스 대조 시 실행 수가 0→0이었으므로, 설치/다른 debug 인스턴스가 실행 중인 동시 격리·보존을 통과했다고 주장하지 않는다. 잔여 Win32 TOCTOU는 디자인 시스템에 명시했다.

Medium이므로 성공하더라도 자동 병합하지 않고 사람의 리뷰를 기다린다. 릴리스 태그와 공개 배포는 하지 않는다.

## is2 커버리지 대조

원본 coverage-checklist의 기존 검증됨/미검증 표를 대조했다. 이번 실행은 Medium 외부 창 상태 제어, 실제 Windows 선행 게이트, red/green, 2회 인라인 재시도 후 1회 격리 에스컬레이션, 독립 감사 지적 보완, main 충돌 해결/최초 ref 보존을 포함한 실사용 이력이다(2026-11-07, #86/PR #88, Codex; 세부 모델 버전 미노출). 전체 플랫폼 CI·최종 리뷰 상태는 PR의 실제 결과로 구분한다. 이 프로젝트의 관측만 기록하며 전역 스킬 정책/기존 coverage 판정을 임의 변경하지 않는다. 자동 병합·롤백·워크트리 삭제 경로를 시험한 것으로 표시하지 않는다.

## 디자인

[디자인 시스템](../design-system.md), [외부 앱 SVG](../design/external-apps.svg)·[PNG](../design/external-apps.png), [트레이 SVG](../design/tray-memo-list.svg)·[PNG](../design/tray-memo-list.png). 시안을 렌더해 열어 확인했다. 기존 터미널 Welcome 코드는 변경하지 않았다. 새 선택기는 별도 네이티브 제목줄을 사용하고 터미널의 선택 후 실행·행/콘텐츠 스타일을 참고한다.
