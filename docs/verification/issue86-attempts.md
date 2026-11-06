# #86 검증 시도 기록

## Windows 제품 시도 1 — FAIL

- 대상 SHA: `953778b81a0ee171853ed35e0aaa28ae471a9a26`.
- Windows 11 x64 / Rust 1.99.1, Orca 작업자 `ctx_936019c41a9a`, 전용 루트 `C:/Users/egoin/orca/workspaces/agent-gibbon/86-product-verification`.
- `cargo test --locked`, `cargo check --locked`, `cargo clippy --locked --all-targets -- -D warnings`, `npm run dev:build` 모두 `E0282`로 FAIL. `windows.rs`의 창 목록 `Vec`에서 `.extend(rows.iter().map(|w| w.identity.hwnd))` 원소 타입 추론 실패.
- 원본 로그는 위 Windows 루트의 `docs/verification/issue86-product-windows/953778b-gates/{cargo-test,cargo-check,clippy,dev-build}.log`에 보존. 이후 제품 검증 보고서에 함께 수록한다.
- 이전 바이너리를 제품 검증에 사용하지 않았다. 테스트/기대값/timeout을 변경하지 않고 `Vec<Window>` 타입을 명시하여 첫 수정 재시도를 진행한다.
- 앞선 초기 구현 SHA `0a4c154611b598495b9a88fce8b5b1f42cb888df`의 Windows 검사 성공은 이 실패 SHA의 성공으로 사용하지 않는다.

## Windows 제품 시도 2 — FAIL (첫 수정 재시도)

- 대상 SHA: `f6279d80ff2d2a89dbdb5bb27a641531eca747da`.
- Rustfmt·Rust test 178개·cargo check·UI(123 PASS / 기존 플랫폼 전용 1 SKIP)·dev:build는 성공했으나 Clippy는 FAIL.
- 원인: 미사용 `PeekMessageW`, `collapsible_if`, 기존 stash_input의 `GetMessageW`와 다른 `MSG` 타입을 재선언한 `clashing_extern_declarations`.
- 원본: Windows 검증 루트 `docs/verification/issue86-product-windows/f6279d8-gates/clippy.log` 및 같은 폴더의 나머지 검사 로그.
- 두 번째 수정 재시도: 메시지 큐 생성에 PeekMessageW를 실제 사용하고 조건을 합쳤다. MSG 구조체와 GetMessageW 선언을 공용 모듈 하나로 옮겨 기존 전역 입력 감시와 동일 ABI를 공유한다. 테스트나 린트 규칙을 완화하지 않았다.
