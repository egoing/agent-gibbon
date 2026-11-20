# Jajan 0.3.9 배포 검증 기록

## 범위·승인·공간

- 승인 원본: local 채팅 `01a1f8d4-7e8b-7410-a774-f6ca635f95f3`, userMessage `01a1f922-f7cd-7110-8098-b279e2ce54d1`의 트레이 아이콘·테마·표시 상태 수정/버전업/팀장 재배포 요청을 직접 조회했다. 기존 공개 승인과 팀장 배정에 따라 정식 공개를 진행한다.
- root: `C:\Users\egoin\.codex\worktrees\release-0-3-6\agent-gibbon` (기존 배포 공간이 clean임을 확인해 재사용).
- branch: `codex/release-0.3.9`, start/base/release 예정 head: `a18358980993906f2a9618309e46661979058675`.
- [PR266](https://github.com/egoing/agent-gibbon/pull/266) MERGED의 npm/Cargo/Tauri/lockfile 버전은 이미 0.3.9다. 원격 main SHA 일치, v0.3.9 미사용, 공개 latest v0.3.8을 새로 확인했다. 버전을 중복 증가하거나 기존 태그/자산을 덮어쓰지 않는다.
- AGENTS.md/로컬 완료 정책/president 팀원 지침/전체 릴리스 문서·워크플로 확인. Windows fcntl 기반 등록 CLI·음성 도구 및 진행 표시의 기존 제한은 우회하지 않고 이 기록과 대화로 상태를 남긴다.
- 기존 사용자 설치 앱 PID 7660의 `C:\Users\egoin\AppData\Local\Jajan\jajan.exe`는 관측만 하고 보존했다. 원 담당의 구현 워크트리/백업/실사용 데이터는 변경하지 않는다.

## 제품 변경

Windows 트레이 기능 7개에 Tabler 아이콘, Windows 앱 테마/고대비 메뉴 전경색을 따르는 RGB, 목록 우측 ‘표시 중’/‘숨김’ 문자가 포함된다. 숨김 선택·최근 순서·ID와 macOS template 표시는 유지한다. [디자인 시스템](../design-system.md), [원 구현 검증](windows-tray-theme-20261120.md), [SVG 시안](../design/windows-tray-menu.svg), [PNG 시안](../design/windows-tray-menu.png)을 따른다. 시안은 실행 캡처가 아니다. 콘솔 번쩍임 등 다른 기능 수정은 이번에 포함하지 않는다.

## 체크리스트

- [x] 승인/원격 상태/정책 확인 및 clean 배포 공간 재사용
- [x] version:check 0.3.9, frozen lockfile 설치
- [x] 최종 로컬 UI/Rust/fmt/Clippy/dev:build/dev:info
- [ ] 공식 v0.3.9 태그와 세 플랫폼 새 SHA의 검사·서명 빌드 성공
- [ ] Windows EXE/PDB GUID·age·Debug ID·해시 artifact 검증
- [ ] Draft 설치 파일 3개·updater 서명·manifest·체크섬·자산명 검증
- [ ] 정식 최신 공개 및 실제 공개 다운로드/URL/latest feed 확인
- [ ] 검증 기록 원격 반영과 기본 main ff/SHA 일치
- [x] 기본 checkout 표준 dev:build/dev:info

Node 24.22.3, pnpm 11.25.0, Rust/Cargo 1.100.0을 명령 환경에서 사용한다. 전역 기본 도구 설정을 바꾸지 않는다. 실제 native 메뉴 픽셀·Windows 테마 전환·고대비·macOS GUI, 기존 0.3.8→0.3.9 설치/재시작은 아직 미검증이다. 공개 뒤 사용자 앱 갱신/실메뉴 확인은 원 담당 채팅에서 이어갈 예정이며 이 배포 작업은 사용자 앱 종료·설치를 수행하지 않는다.

0.3.5/0.3.7의 준비 ACK 결함은 실행 중인 구버전 코드에 있어 새 공개만으로 복구되지 않는다. 시간 초과가 반복되면 메모/초안을 저장하고 앱을 직접 정상 종료한 뒤 공식 새 설치 파일을 기존 경로에 적용한다. 앱 제거/사용자 데이터·설정 폴더 삭제 없이 재실행 후 버전·기존 메모/배치를 확인한다. 현재 사용자의 0.3.8은 ACK 수정이 포함된 경로이며 이번 0.3.9로의 실제 설치 성공을 미리 주장하지 않는다.

macOS 공증은 기존 사용자 일시 생략 정책, Developer ID/updater 서명은 필수다. Windows Authenticode는 구성하지 않는다. Windows 심볼은 Actions artifact 권한을 따르는 별도 보관이며 공개 Release/Sentry에 업로드하지 않는다. 비밀·인증 원문은 기록하지 않는다.

## 로컬 최종 검사와 공식 태그

최종 head a18358980993906f2a9618309e46661979058675에서 UI 247 통과/기존 1 생략, Rust 314 통과, Rustfmt/Clippy all-targets -D warnings 및 npm run dev:build 성공. dev:info의 id는 `dev-89449df4f6727d7ea014fbec`, binary는 배포 root의 `src-tauri\target\debug\jajan.exe`. 해당 CLI --instance-info의 ok=true/instance/manifest_dir도 일치했다. 바이너리 자체 출처 확인이며 GUI 서버/메뉴 픽셀 검증과는 별개다.

공식 annotated tag v0.3.9를 위 SHA에 발행했고 원격 peeled SHA 일치를 확인했다. 태그 직전 실제 원격 main도 같은 SHA였다. 이전 릴리스의 CI 결과를 이번 head의 통과로 재사용하지 않는다.


[공식 Release 실행44746817414](https://github.com/egoing/agent-gibbon/actions/runs/44746817414)의 headSha는 a18358980993906f2a9618309e46661979058675로 공식 태그와 일치한다. 실행 완료/실패까지 추적하고 아래에 실제 결과를 기록한다.


기본 root `C:\Users\egoin\dev\project\agent-gibbon`의 0.3.9 표준 npm run dev:build도 성공했다. dev:info id `dev-fbc7d562e05aec4798036a95`, binary `C:\Users\egoin\dev\project\agent-gibbon\src-tauri\target\debug\jajan.exe`, CLI --instance-info의 ok=true/instance/manifest_dir 일치. 두 root의 개발 인스턴스는 별개이며 기존 사용자 앱을 재시작하거나 설치하지 않았다.

## 공식 첫 시도 실패

실행 44746817414 attempt 1의 macOS ARM64 job136324313147에서 `terminal::lifecycle_tests::quit_reclaims_confirmed_jobs_in_separate_groups_and_preserves_sentinel`이 `bounded_shutdown(...)` assert(src/terminal/lifecycle_tests.rs:357)로 실패했다. Rust 결과는 326 통과/1 실패/1 ignored다. 테스트 조건과 종료 제한시간은 변경하지 않는다. 트레이 관련 새 테스트들은 해당 로그에서 성공했다. terminal 경로는 v0.3.8 이후 diff가 없지만 이 사실만으로 환경 불안정이나 원인이 확정됐다고 주장하지 않는다. 첫 실패를 보존하고 동일 SHA의 실패 job 재실행으로 재현성을 확인한다. 새 SHA를 만든 경우에는 이 실행 결과를 재사용하지 않는다.

## 동시 릴리스 조율

팀장이 별도 v0.3.10 실행44747367976, head18eb259775ad3d59230d4be6c29355d9692f1855와 a1835898 대비 ahead3/behind0을 확인해 트레이 변경 포함을 전달했다. 해당 실행/태그는 이 배포 작업에서 취소·수정하지 않는다. 사용자 설치 앱 갱신·메뉴 확인은 원 담당 채팅에서만 진행한다. 0.3.10이 필수 게이트를 통과하고 먼저 정식 공개되면 0.3.9 공개를 보류하여 latest/feed를 되돌리지 않는다. 미공개이면 기존 0.3.9 검사와 동일 SHA 실패 job 재시도는 계속한다. 출처가 다른 SHA의 통과를 혼용하지 않는다.

## Windows 완료 결과

Windows job136324313473은 success: Rust 314 통과, UI 246 통과/기존 조건부 2 생략, fmt/Clippy/설치 파일/심볼 검증 성공. UI 생략은 POSIX fixture와 CI의 명시적 build-dir 환경에서 기본 캐시 정책 검사이며 로컬에서는 후자도 통과했다. 심볼 단위 검사 4개도 통과했다.


심볼 artifact ID14634600924, `windows-symbols-a18358980993906f2a9618309e46661979058675`를 다운로드해 revision/PE CodeView/PDB GUID·age·Debug ID와 해시를 직접 재검증했다. GUID `91c482a2-5033-4f6e-b927-b3448d5546c2`, age1, Debug ID `91c482a2-5033-4f6e-b927-b3448d5546c2-1` 일치. EXE 24593920바이트 SHA256 `50a627bcc01c3a52e416fca384cbc56dab5d1861b927099cb1a7fddd6022f577`, PDB 85454848바이트 SHA256 `c32149fdb85343ead57d11c8f6e3a2efd1f93886f1160af11e0eff35fae357ce` 일치. artifact 만료는 2026-12-20T13:53:52Z. 로컬 `%TEMP%\jajan-release-0.3.9-44746817414\symbols`에 보관하고 공개 Release/Sentry에는 올리지 않았다.

## 최종 배포 대상 변경 — 0.3.9 미공개 보존

팀장이 동일 사용자 재배포 승인을 근거로 배포 대상을 트레이 수정을 포함한 0.3.10으로 확정하고 기존 성공 Draft의 검증·공개를 이 배포 담당에게 재배정했다. 0.3.9는 공개하지 않고 태그/산출물/실패 로그를 보존한다. 진행 중인 Intel job이나 타 작업 run을 취소하지 않았으며 ARM64 재시도를 기다려 중복 릴리스를 만들지 않는다. 마지막 조회에서 44746817414는 여전히 in_progress, ARM64 failure/Windows success/Intel installer build 진행 중이다. 이 실행을 전체 통과나 완료로 보고하지 않는다. 향후 자동 생성되는 Draft가 있어도 공개하지 않는다.

최신 배포는 [0.3.10 공개 검증 기록](release-0.3.10-20261120.md)이다. 0.3.10의 별도 SHA 검증 결과를 0.3.9의 통과로 재사용하지 않는다.
