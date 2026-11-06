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

## 제품·독립 감사·CI

현재 제품 검증 진행 중. [실패 및 수정 재시도](issue86-attempts.md)를 별도 보존한다. Windows 제품 실기·전체 플랫폼 CI·최종 독립 감사는 완료 결과를 수신한 뒤 이 절에 기록하며 미실행을 PASS로 쓰지 않는다.

초기 제품 SHA `0a4c154611b598495b9a88fce8b5b1f42cb888df`의 독립 검토는 테스트 완화/스코프 이탈 미발동, Windows 제품 퇴행 증거 부족으로 판정했다. 지적한 직접 attach 닫기 경합, 동일 프로세스 HWND 재사용, 바로가기 시작 폴더 누락을 보완했으며 최종 SHA를 다시 감사한다.

## 디자인

[디자인 시스템](../design-system.md), [외부 앱 SVG](../design/external-apps.svg)·[PNG](../design/external-apps.png), [트레이 SVG](../design/tray-memo-list.svg)·[PNG](../design/tray-memo-list.png). 시안을 렌더해 열어 확인했다. 기존 터미널 Welcome 코드는 변경하지 않았다. 새 선택기는 별도 네이티브 제목줄을 사용하고 터미널의 선택 후 실행·행/콘텐츠 스타일을 참고한다.
