# Jajan 0.3.7 배포 검증 기록

[0.3.6 시도](release-0.3.6-20261120.md)의 동일 공개 승인·작업 공간·포함 범위를 이어받는다. 0.3.6은 공식 Windows Clippy 실패로 미공개이며 기존 태그를 보존한다.

## 작업 공간

- root: `C:\Users\egoin\.codex\worktrees\release-0-3-6\agent-gibbon` (기존 배포 공간을 계속 사용)
- branch: `codex/release-0.3.6` (브랜치명은 최초 준비 버전이며 최종 제품 버전은 0.3.7)
- 이번 수정 base: `594ae511260e53f111abfb768b76d2208d8a76c4`.
- President office.py 및 진행 표시/음성의 Windows 미지원 한계는 앞 기록과 동일하다. 도구를 우회하지 않고 이 문서·대화에 누적 기록한다.

## 수정과 게이트

`login_launch`의 Windows `if true / false`를 직접 bool 값 반환으로 바꾸고, macOS는 기존과 동일하게 명시 인자 OR OS 로그인 이벤트를 단락 평가한다. Clippy 억제나 배포 게이트 완화 없이 Rust 1.100 lint에 대응한다. 의존성·자동 실행 동의 정책·시작 동작은 바꾸지 않는다.

- [x] 원격 태그 조회 후 미사용 0.3.7 선택, version:set/check 일치
- [x] 로컬 최종 검사와 표준 개발 빌드
- [ ] PR 병합과 main 동기화
- [ ] 새 공식 태그/SHA의 macOS ARM64·Intel·Windows x64 전체 성공
- [ ] Windows 동일 EXE/PDB 심볼 artifact와 Debug ID 검증
- [ ] Draft 서명·체크섬·자산명·manifest URL 검증
- [ ] 공개 및 실제 공개 다운로드·latest feed 검증
- [ ] 기본 체크아웃 최신 표준 dev:build와 dev:info

## 중요한 미검증 범위

OS 실제 등록/로그인, macOS 실제 GUI와 이전 공식 버전에서 자동 업데이트 설치·재시작은 이번 배포 CI가 대신하지 않는다. macOS 공증은 현재 사용자 일시 생략 정책, Developer ID/updater 서명은 필수다. Windows Authenticode는 구성하지 않는다. 0.3.5 과거 panic의 PDB 복구나 근본 해결을 주장하지 않는다. 심볼 artifact는 저장소 Actions 권한에 따르며 공개 Release/Sentry에 업로드하지 않는다.

[디자인 시스템](../design-system.md)과 [자동 실행 검증](autostart-settings-20261120.md)의 동작 기준을 유지한다.


## 로컬 최종 결과

Rust 1.99.1에서 fmt, Clippy all-targets -D warnings, Rust 311개 테스트, 최종 `npm run dev:build` 성공. UI 소스는 변경하지 않았으며 동일 소스의 앞선 UI 240 통과/기존 1 생략 결과를 구분해 사용한다. 추가로 `RUSTUP_TOOLCHAIN=1.100.0 node scripts/build.mjs cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`가 전체 통과했다. 기본 toolchain은 바꾸지 않았다. 1.100.0 설치 후 rustup 자체 갱신은 실행 파일 잠금으로 실패했지만 해당 컴파일러 설치와 실제 Clippy 성공은 별도로 확인했다.
