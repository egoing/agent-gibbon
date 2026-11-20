# Jajan 0.3.6 배포 검증 기록

## 작업 공간과 승인

- 역할: Windows president 배포 전담 팀원. 등록 CLI는 POSIX fcntl 의존으로 Windows 미지원이며 실행하거나 우회하지 않았다.
- 사용자 공개 승인 원본: local 채팅 `01a1f7f0-822b-73f2-8c50-1c498a1c4b1e`, userMessage `01a1f7ff-e571-7262-a07b-f356d866c583` 및 `01a1f800-35d7-7670-b199-d6c372bf2aba`를 직접 조회했다.
- root: `C:\Users\egoin\.codex\worktrees\release-0-3-6\agent-gibbon`
- branch: `codex/release-0.3.6`
- start/base: `cd09acd52a711709e4523dec34f8aee3454ee16c` (조회 시 local main = origin/main = 원격 main).
- 버전: 원격 태그·Release 확인 후 미사용 안정 버전 `0.3.6` 선택. 기존 태그와 공개 자산은 덮어쓰지 않는다.
- 로컬 `AGENTS.override.md` 자동 복사 확인. 기존 기본 체크아웃 `.tmp`와 타 작업 프로세스는 보존한다.

## 포함 범위

기존 v0.3.5 이후 main에 병합된 변경 전체를 배포한다. 로그인 자동 실행/설정 즉시 적용(PR259), Windows 숨김 안내창의 메모 재표시 방지(PR257), Windows 동일 EXE/PDB 검증·보관(PR258), 닫기 강조·메모 경계·그림자·관리 창 복원 및 앞선 메모/터미널 안정화가 포함된다. 이번 배포 작업의 제품 변경은 버전 필드 네 개뿐이다. 디자인 기준은 [디자인 시스템](../design-system.md), 자동 실행 검증은 [기록](autostart-settings-20261120.md)을 따른다.

## 검증 계획과 상태

- [x] 배포 절차·워크플로 전체, 원격 버전/태그, 공개 승인 확인
- [x] 전용 워크트리/브랜치 준비 및 version:set/check 0.3.6
- [x] frozen lockfile 의존성 설치 (Node 24.22.3, pnpm 11.25.0)
- [x] 최종 로컬 UI/Rust/fmt/Clippy 및 표준 dev:build
- [x] 배포 PR 병합, 기본 체크아웃 fast-forward와 원격 SHA 일치
- [ ] 승인된 공식 태그로 세 플랫폼 테스트/fmt/Clippy/서명 빌드
- [ ] 동일 Windows EXE/PDB GUID·age·Debug ID artifact 확인
- [ ] Draft 설치 파일 세 개/updater 서명/manifest/체크섬/실제 자산명 검증
- [ ] 공개 후 latest feed 및 공개 다운로드/체크섬 재검증
- [x] 기본 체크아웃 표준 dev:build/dev:info 준비

## 출처와 한계

- 배포 워크트리 개발 ID: `dev-89449df4f6727d7ea014fbec`
- 개발 binary: `C:\Users\egoin\.codex\worktrees\release-0-3-6\agent-gibbon\src-tauri\target\debug\jajan.exe`
- CI는 설치·로그인·GUI 시험을 대신하지 않는다. 실제 OS 시작 등록/로그인 및 이전 공식 버전에서 자동 업데이트 설치는 이번에 아직 검증하지 않았다.
- macOS 공증은 사용자가 재개를 요청할 때까지 생략하는 현재 정책을 따른다. Developer ID 서명과 updater 서명은 필수다. Windows Authenticode 서명은 현 워크플로에서 구성하지 않는다.
- Windows 심볼은 별도 Actions artifact이며 공개 Release 자산 또는 Sentry에 업로드하지 않는다. 저장소 Actions artifact 권한을 따르며 비공개를 가정하지 않는다. 0.3.5 과거 panic의 PDB 복구/원인 해결을 의미하지 않는다.
- 실행 URL·head SHA·플랫폼별 결과·공개 검증은 실제 확인 후 아래에 누적한다.

## 로컬 실행 결과 (2026-11-20)

- `npm run version:check`: 0.3.6 일치, 의존성 해석 변경 없음.
- `npm run test:ui`: 240 통과, 기존 POSIX fixture 검사 1 생략, 실패 0.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`: 통과.
- `node scripts/build.mjs cargo test --locked --manifest-path src-tauri/Cargo.toml`: Rust 311 통과.
- `npm run dev:build`: 성공, 최종 버전 변경 이후 표준 경로 debug 바이너리 생성.
- `scripts\jajan-dev.cmd --instance-info`: ok=true, 위 manifest_dir 및 instance 일치. 서버/GUI 출처 검증과는 별개인 바이너리 자체 검사다.

- `npm run check`: Clippy all-targets -D warnings 통과.


## 원격 병합·공식 실행

- 배포 커밋: `d0e05cb5e9f9d46a0da7af1cd81cf57b04035cc0`.
- [PR260](https://github.com/egoing/agent-gibbon/pull/260) MERGED, merge SHA `594ae511260e53f111abfb768b76d2208d8a76c4`.
- 병합 직전 main은 base와 동일했다. 기본 root에서 `git merge --ff-only origin/main` 후 local main = origin/main = GitHub 원격 SHA가 위 merge SHA임을 확인했다. 기존 `.tmp`는 보존했다.
- `v0.3.6` 공식 annotated tag의 peeled SHA도 위 merge SHA와 같다.
- [공식 Release 실행 44702390360](https://github.com/egoing/agent-gibbon/actions/runs/44702390360), headSha `594ae511260e53f111abfb768b76d2208d8a76c4`. 버전/태그 validate 성공, 세 플랫폼 진행 중.
- 최초 Git push는 Credential Manager의 egoing2 계정 선택으로 HTTP403. 전역 인증 설정을 바꾸지 않고 해당 명령에만 활성 gh 계정 인증 도우미를 지정해 egoing 계정으로 push 성공.
- 기본 root `C:\Users\egoin\dev\project\agent-gibbon`에서도 `npm run dev:build` 성공. dev:info ID `dev-fbc7d562e05aec4798036a95`, binary `C:\Users\egoin\dev\project\agent-gibbon\src-tauri\target\debug\jajan.exe`.
- 두 개발 바이너리는 서로 다른 인스턴스다. 이 작업에서 기존 앱/타 작업 프로세스를 종료하거나 새 GUI 앱을 시작하지 않았다. 이미 실행 중인 앱은 새 빌드로 자동 교체되지 않는다.

## 공개 중단과 후속 버전

Windows CI의 Clippy(Rust 1.100.0)가 `autostart.rs:161`의 `needless_bool`을 오류로 판정했다. 로컬 1.99.1 검사와 달라 공식 게이트가 실패했다. Windows 설치/심볼 단계 및 assemble/Draft/공개는 진행할 수 없으므로 0.3.6을 공개하지 않는다. 기존 태그를 이동하거나 덮어쓰지 않고 수정본은 0.3.7에서 새 SHA로 전체 검증한다. 위 0.3.6 체크리스트의 미완료 항목은 이 실패 시도의 상태이며, 후속 결과는 [0.3.7 기록](release-0.3.7-20261120.md)을 따른다.
`44702390360` 최종 상태는 completed/failure이며 macOS 두 빌드는 성공했지만 Windows Clippy 실패로 assemble/Draft는 건너뛰었다. 0.3.6은 공개하지 않았다.
