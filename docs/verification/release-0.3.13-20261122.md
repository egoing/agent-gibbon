# 0.3.13 정식 배포 검증

## 범위와 기준

- 통합 기준: PR #276, `dc70d33e99d5eeb9dd31a58c6d97977d7724a3e3`. 실제 검증된 PR head의 tree와 병합 tree가 같다.
- 사용자 승인 범위: 업데이트 시험 기능 유지, 프로세스 분리 유예·선별 롤백. 분리 owner/backend/API/UI를 이번 릴리스에 포함하지 않는다. 장기 과제는 [#279](https://github.com/egoing/agent-gibbon/issues/279), 과거 전환 인과 미확정은 [#278](https://github.com/egoing/agent-gibbon/issues/278)에 보존한다.
- 버전 변경: npm/Cargo/Tauri/Cargo lock의 앱 버전을 `version:set`으로 0.3.12에서 미사용 0.3.13으로 함께 변경했다. 기존 태그·공개 자산은 변경하지 않는다.
- 작업 루트: `C:\Users\egoin\.codex\worktrees\release-0-3-6\agent-gibbon`, 브랜치 `codex/release-0.3.13`.

## 기능 검증과 한계

[최종 Windows 검증](developer-update-only-20261122.md)의 동일 소스 격리 서명 NSIS에서 동일 버전 재설치, 0.3.13→0.3.12→0.3.13 왕복, 매 설치의 창 8개 ACK, 본문·revision·안정 위치·크기·표시 보존을 확인했다. 349.516초/324.375초 무입력에도 수동 대기를 유지했다. 저장 실패 ACK 거부와 저장 복구, 숨긴 일반 실행 터미널의 설치 보류도 실제 확인했다.

이는 공식 공개 호환 쌍이나 macOS 실설치/UI 검증이 아니다. 공개된 복귀 가능 공식 쌍이 없으므로 일반 빌드의 이전/최신 왕복 버튼은 비활성이다. 5회 클릭은 숨김 업데이트 시험 메뉴만 표시하며 셸 분리를 켜지 않는다. 접힌 기존 패키지 준비 시험은 공식 최신의 동일 버전 재설치를 지원하고, 수동 준비 후 명시적 설치 전에는 자동 설치하지 않는다. 서명·플랫폼·버전 검증과 기존 작업 보호를 유지한다.

별도 일반 자동 신규버전의 무인 실설치는 이번 수동 시험에서 반복하지 않았다. 사용자 설치 앱을 배포 담당이 종료·교체하지 않는다. 0.3.5/0.3.7의 기존 실행 코드에 있는 ACK 누락은 새 릴리스 공개만으로 치유되지 않으므로 공식 설치 파일을 통한 데이터 보존 수동 업그레이드를 안내한다.

## 배포 게이트

- 로컬: Node 24.22.3/pnpm 11.25.0/Rust 1.100.0에서 버전 검사, Rust 318 통과, UI 257 통과/기존 2 건너뜀, Rustfmt 및 Clippy all-targets `-D warnings` 통과. 최종 버전 변경 후 표준 `npm run dev:build` 55.82초 성공, `dev:info`로 이 루트의 `dev-89449df4f6727d7ea014fbec`/`src-tauri\target\debug\jajan.exe` 확인. 이 작업에서 GUI를 시작하지 않았다.
- 공식 태그의 macOS ARM64/Intel·Windows x64 검사·빌드, macOS Developer ID 및 updater 서명 검증은 새 배포 SHA의 CI로 확인한다. 과거 버전의 CI 결과를 재사용하지 않는다.
- macOS 공증 제출·티켓 검사는 기존 사용자 일시 생략 정책을 유지한다.
- Windows 동일 빌드 EXE/PDB의 debug ID와 SHA-256을 확인하고 별도 artifact로 보존한다. 공개 릴리스 자산이나 Sentry에는 올리지 않는다.
- Draft 전체 자산/체크섬/manifest/서명 및 실제 파일명·URL을 검증한 뒤 공개한다. 공개 후 인증 없는 다운로드와 latest feed를 다시 검증한다.

## 공식 빌드와 공개 결과

- 버전 PR [#281](https://github.com/egoing/agent-gibbon/pull/281) 병합, 배포 태그 `v0.3.13`과 대상 SHA `77461beef7cc769b50ca7bb944b7aea6d792abac` 일치. 이 SHA에서 로컬 main/origin/main/실제 원격 main의 fast-forward 동기화를 확인했다. 기본 체크아웃의 기존 `.tmp/`는 보존했다.
- [공식 Actions 실행 44966829192](https://github.com/egoing/agent-gibbon/actions/runs/44966829192): validate, 세 플랫폼 build, assemble, draft 모두 completed/success.

| 플랫폼 | 검사·설치 파일 | 서명과 보관 |
|---|---|---|
| macOS ARM64 | Rust/UI/fmt/Clippy/DMG·updater 아카이브 성공 | Developer ID 팀·앱 식별자/strict 검증 성공, updater 서명 검증 성공 |
| macOS Intel | Rust/UI/fmt/Clippy/DMG·updater 아카이브 성공 | Developer ID 팀·앱 식별자/strict 검증 성공, updater 서명 검증 성공 |
| Windows x64 | Rust/UI/fmt/Clippy/NSIS 성공 | updater 서명 성공, 동일 빌드 EXE/PDB debug ID·SHA-256 일치 |

Windows 심볼의 Debug ID는 `5ad331b2-6912-4da3-9184-fc42ef208aab-1`이다. artifact `windows-symbols-77461beef7cc769b50ca7bb944b7aea6d792abac`와 로컬 증거에 별도 보존했다. Authenticode 서명이나 macOS 공증 성공으로 보고하지 않는다.

- Draft에서 자산 13개 전체를 다운로드해 체크섬 11개, 세 updater payload/trusted comment 서명, 공식 공개키, 버전/파일명/manifest URL/서명 문자열 일치를 검증했다.
- 2026-11-22 **09:49:36 KST** [정식 공개](https://github.com/egoing/agent-gibbon/releases/tag/v0.3.13). `isDraft=false`, `isPrerelease=false`, GitHub latest=0.3.13 확인.
- 공개 후 자산 13개를 인증 없는 `curl`로 다시 다운로드하여 같은 검사를 통과했다. 실제 API 자산 URL과 manifest의 세 플랫폼 URL도 일치했다. `latest/download/latest.json`의 SHA-256은 `65333f3c4900e5c7269c921c014e5f911f7e995acbc87aa2c5bc57f83ab86d14`이며 공개 manifest와 바이트 단위로 같다.

| 설치 파일 | SHA-256 |
|---|---|
| `Jajan_0.3.13_x64-setup.exe` | `96f39bc30af3c08fbd595e4a5d5fcfc89a70fee72a1199240eb24df1a16c4b21` |
| `Jajan_0.3.13_aarch64.dmg` | `e2f631baf563d3e308dfe759ef836aa1e90471da1e8f1d2c2537074ed607b403` |
| `Jajan_0.3.13_x64.dmg` | `152313c00dd3112ba50d6099455c4f9a2c0be3e479112ee3a4bee6daa042571c` |

기본 체크아웃 `C:\Users\egoin\dev\project\agent-gibbon`에서도 배포 버전의 표준 `dev:build` 49.43초 성공, `dev:info`로 `dev-fbc7d562e05aec4798036a95` 및 해당 root의 `src-tauri\target\debug\jajan.exe` 확인. 작업 워크트리와 별도 개발 인스턴스다. 이 작업은 기존 실행 앱을 종료하거나 사용자 설치본을 교체하지 않았으므로 실제 설치 앱의 자동 업그레이드 완료를 의미하지 않는다. 이미 실행 중인 앱은 재시작 전 이전 코드일 수 있다.

로컬 증거: `C:\Users\egoin\.codex\artifacts\release-0.3.13-20261122`(검사 로그, Actions 결과, 공개 자산, 릴리스 노트, 별도 Windows 심볼). 이 문서의 공개 후 결과 추가는 앱 코드를 바꾸지 않는다. 최종 문서 커밋 이후 main 동기화 SHA는 인계 기록에 별도 보존한다.

Conf **5**(Windows 격리 기능 실설치와 공식 공개 다운로드/서명 검증), Risk **Medium**(설치·재시작/캐시 상태). macOS 실설치와 사용자 설치 앱의 업그레이드는 이 확신도에 포함하지 않는다.

[디자인 기준](../design-system.md) · [설명용 시안 SVG](../design/developer-updater.svg) · [PNG](../design/developer-updater.png). 시안은 실제 앱 캡처가 아니다.
