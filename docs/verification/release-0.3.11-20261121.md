# Jajan 0.3.11 배포 검증 — 2026-11-21

## 대상

- 업데이트 설치 조건·보류·복구 안내 개선. [변경 설계와 로컬 검증](updater-policy-clarity-20261120.md), [디자인 시스템](../design-system.md).
- PR: https://github.com/egoing/agent-gibbon/pull/269
- 배포 태그: `v0.3.11`
- 대상 SHA: `3a7ceebe911883ca6efa47beb23cbf0d5937b952`
- CI: https://github.com/egoing/agent-gibbon/actions/runs/44757931639

## 로컬

- 0.3.11 최종 `pnpm run test`: Rust 328 통과·기존 1 ignored, UI 250 통과.
- Clippy all-targets `-D warnings`, Rustfmt, 버전 일치, diff whitespace 검사 통과.
- 실제 UI 자산과 모의 IPC로 Chromium 밝음/어두움 안내·긴 오류, 설정 8상태×3테마×2크기, 키보드 복구 도움말 열기·가로 넘침 방지 통과. 실제 설치 검증은 아니다.
- `pnpm run build`: macOS ARM64 0.3.11 앱 번들 성공. `codesign --verify --deep --strict`와 Info.plist 0.3.11 확인. 로컬은 ad-hoc 서명이며 공식 CI의 Developer ID 서명과 구분한다.
- PR 병합 뒤 기본 checkout의 local main을 fast-forward하고 원격 main과 대상 SHA 일치를 확인했다.

## CI와 공개 결과

- CI의 validate, macOS ARM64·Intel, Windows x64, assemble, draft 모두 success. 실행 headSha가 위 태그 SHA와 일치한다. 각 플랫폼의 테스트·Rustfmt·Clippy·설치 파일 빌드와 공식 서명 자산 생성 성공을 확인했다.
- Draft 자산 13개를 실제로 다운로드했다. SHA256SUMS의 11개 파일을 대조했고, 공개 키를 Actions variable `TAURI_UPDATER_PUBLIC_KEY`와 비교했다. 세 payload·trusted comment의 서명, 버전·플랫폼 파일명을 다시 검증하고 manifest와 checksum을 재생성해 다운로드 원본과 byte 일치를 확인했다.
- 2026-11-21 00:18:23 KST에 `gh release edit v0.3.11 --draft=false --latest`로 정식 공개했다. `isDraft=false`, `isPrerelease=false`를 조회했다. [정식 릴리스](https://github.com/egoing/agent-gibbon/releases/tag/v0.3.11).
- 인증 없이 공개 latest/download/latest.json을 GET해 HTTP 200과 0.3.11 manifest 원문 일치를 확인했다. SHA-256은 `bce4a032a43d2b8db783d4adec7405d216fe54aec8a85b6eb0440b34e608f010`.
- manifest의 darwin-aarch64·darwin-x86_64·windows-x86_64 업데이트 URL 모두 인증 없이 HEAD HTTP 200. 별도로 두 DMG와 Windows EXE의 공개 주소도 HEAD HTTP 200을 확인했다.
- 공개 자산을 덮어쓰거나 기존 태그를 이동하지 않았다. macOS 공증 제출·티켓 검사는 현재 운영 지시에 따라 생략했다.

## 실환경 검증 경계

운영 설치본의 0.3.10 → 0.3.11 업데이트, 실제 재시작 후 버전·메모·포커스 보존은 팀장 검증 범위다. 설치된 짜잔과 GUI 입력은 이 작업에서 조작하지 않았다. Windows·Intel 설치 실행도 수행하지 않았다. CI와 공개 다운로드·서명 검증이 실제 설치 성공을 뜻하지 않는다.

Conf 4: 실제 UI 자산의 모의 IPC 브라우저 검증 기준. 공개 전달·서명은 실제 자산으로 검증했다. Risk Low: 기존 설치 보호 로직을 유지한 안내·복구 문구 개선이다.
