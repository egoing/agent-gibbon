# Jajan 0.3.8 배포 검증 기록

## 범위·승인·작업 공간

- 사용자 승인: local 채팅 `01a1f8d4-7e8b-7410-a774-f6ca635f95f3`, userMessage `01a1f8f7-8a2f-7b72-80ec-a3fe5f164676`의 “$MERGE 하고 팀장에게 릴리즈 해달라고 요청해줘.”를 직접 조회했다. 팀장 배정 및 기존 공개 승인에 따라 버전/검증/커밋/push/PR/병합/태그/공식 빌드/검증 후 공개를 진행한다.
- root: `C:\Users\egoin\.codex\worktrees\release-0-3-6\agent-gibbon` (깨끗한 기존 배포 공간 재사용, 디렉터리 이름은 최초 배포 작업명).
- branch: `codex/release-0.3.8`, start/base `5d6faa8f5cfe5ccedcd3465f6589a964160582ab`.
- 기존 0.3.7 공개/기록 완료 main `5220c9402aa440e2f354255c239b177f1e1a1d88` 이후 PR263의 보조창 ACK 수정만 추가된 기준이다. 원격 main/태그/Release를 새로 확인해 미사용 0.3.8 선택. 기존 0.3.6/0.3.7 태그와 자산은 보존한다.
- AGENTS.md/AGENTS.override.md와 릴리스 문서/워크플로 전체 확인. President office.py의 Windows fcntl 미지원은 우회하지 않는다. 음성/진행 표시 도구의 기존 Windows 제한에 따라 문서 체크리스트와 대화에 기록한다.
- Node 24.22.3, pnpm 11.25.0, Rust/Cargo 1.100.0을 명령 환경에서 지정했다. 호스트 기본 Rust 1.99.1/Node 20과 혼동하지 않으며 전역 기본 도구 설정은 변경하지 않는다.

## 수정과 기존 사용자 경로

[PR263](https://github.com/egoing/agent-gibbon/pull/263)은 direction-guide/layout-undo/memo-backdrop/memo-overview에 기존 준비/해제 guard와 local listen/unlisten 권한을 연결하고, 정렬/취소 명령 pending 및 네이티브 정렬 취소 activity를 보호한다. 응답 대상 제외나 timeout 연장은 하지 않는다. 이번 배포 준비의 제품 변경은 버전 필드 네 곳뿐이다.

실제 설치본 0.3.5의 과거 실패 진단은 ACK 8개 중 4개 null이며 당시 label 자체는 기록되지 않았다. 원인 창의 정적 구현/현재 관측과 과거 label 직접 관측을 구분한다. [분석](../analysis/windows-update-ack-timeout-20261120.md)과 [수정 검증](updater-auxiliary-ack-20261120.md), [디자인 기준](../design-system.md)을 따른다.

**0.3.5와 0.3.7의 실행 중 준비 코드는 새 릴리스 공개만으로 바뀌지 않는다.** 자동 업데이트가 준비 응답 시간 초과로 막히면 공식 0.3.8 설치 파일을 받아 수동 업그레이드해야 한다. 메모/초안을 저장하고 사용자가 직접 자잔을 정상 종료한 뒤 같은 사용자·기존 설치 경로에 새 설치 파일을 적용한다. 앱 제거, 사용자 데이터/설정 폴더 삭제, 새 프로필 초기화는 하지 않는다. 다시 시작해 버전 0.3.8과 기존 메모/배치를 확인한다. 실제 기존 데이터 보존 설치 E2E는 이번에 아직 실행하지 않았으며 사용자 앱 강제 종료·실설치·메모 삭제는 수행하지 않는다.

## 체크리스트

- [x] 승인 원본·원격 상태·현재 정책 확인, clean 워크트리 재사용
- [x] version:set/check 0.3.8 및 frozen lockfile 설치 (해석 변경 없음)
- [x] 최종 로컬 Rust/UI/fmt/Clippy와 표준 dev:build/dev:info
- [x] PR 병합과 기본 main fast-forward 및 SHA 일치
- [x] 공식 v0.3.8 새 SHA의 세 플랫폼 전체 검사·빌드·서명 성공
- [x] Windows 동일 EXE/PDB GUID·age·Debug ID·해시 artifact 확인
- [x] Draft 설치 파일 3개·updater 서명·manifest·체크섬·자산명 검증
- [x] 정식 최신 공개 후 실제 공개 다운로드와 latest feed 검증
- [x] 기본 체크아웃 최신 표준 dev:build/dev:info

실제 구버전→후보 설치/재시작 E2E, macOS 실기기 검증은 미실행이다. macOS 공증은 기존 사용자 일시 생략 정책을 유지하며 Developer ID/updater 서명은 필수다. Windows Authenticode는 구성하지 않는다. Windows 심볼은 저장소 Actions 권한을 따르는 별도 artifact에 보관하고 공개 Release 자산이나 Sentry에 업로드하지 않는다.


## 최종 로컬 결과

0.3.8 최종 버전에서 Node 24.22.3/Rust 1.100.0으로 UI 247 통과(기존 POSIX fixture 1 생략), Rust 311 통과, Rustfmt/Clippy all-targets -D warnings 통과. `npm run dev:build`도 성공했다. `dev:info`의 root는 위 작업 공간, id `dev-89449df4f6727d7ea014fbec`, binary `C:\Users\egoin\.codex\worktrees\release-0-3-6\agent-gibbon\src-tauri\target\debug\jajan.exe`. 개발 CLI `--instance-info`의 ok=true, instance/manifest_dir도 일치했다. 이는 바이너리 자체 출처 검사이며 서버 GUI/업데이트 E2E로 간주하지 않는다. 기존 사용자 설치 앱 PID 2832는 조회만 하고 보존했다.


## 공식 태그와 실행

- [PR264](https://github.com/egoing/agent-gibbon/pull/264) MERGED. version commit `75023b2967e8e0b54ac34081620dc508e195fea1`, merge/release head `42907267ead9438b29b23ace307581f759d2cbe0`.
- 병합 직전 원격 main은 base와 동일했다. 병합 후 기본 체크아웃 fast-forward 및 local main = origin/main = 실제 원격 main SHA 일치를 확인했다. 기존 `.tmp`는 보존했다.
- 공식 annotated tag `v0.3.8`의 peeled SHA도 위 release head와 일치한다. [Release 실행44738817018](https://github.com/egoing/agent-gibbon/actions/runs/44738817018)의 headSha 일치 확인, 결과는 실제 완료 후 아래에 누적한다. 이전 릴리스 검사 결과를 이 SHA의 통과로 재사용하지 않는다.


기본 체크아웃 `C:\Users\egoin\dev\project\agent-gibbon`에서도 최종 0.3.8의 `npm run dev:build`가 성공했다. `dev:info` id는 `dev-fbc7d562e05aec4798036a95`, binary는 `C:\Users\egoin\dev\project\agent-gibbon\src-tauri\target\debug\jajan.exe`. 해당 CLI `--instance-info`의 ok=true 및 instance/manifest_dir 일치 확인. 배포 워크트리와 기본 root는 서로 다른 개발 인스턴스다. 이미 실행 중인 사용자 설치 앱은 새 개발 빌드로 교체되지 않으며 사용자가 수동 업그레이드/재시작해야 한다.

## Windows 심볼 실물 검증

`windows-symbols-42907267ead9438b29b23ace307581f759d2cbe0` artifact ID `14630268102`를 다운로드해 PE CodeView/MSF PDB 파서와 SHA-256으로 직접 재검증했다. revision은 release head와 일치. GUID `9602105a-0208-4a40-a232-8c65d5f972bb`, age `1`, Debug ID `9602105a-0208-4a40-a232-8c65d5f972bb-1`이 일치한다. 보관 만료는 2026-12-20T12:59:47Z이며 저장소의 Actions artifact 접근 권한을 따른다.

| 파일 | 바이트 | SHA-256 |
|---|---:|---|
| jajan.exe | 24588800 | c71e29208a9f887f28517579145cb22f69c766209816d9cb76ffa18f6999c3ca |
| jajan.pdb | 85446656 | 5b4f4cba32d67bc64a53779f65f1db7869ce2d92764c55c0c8bff955aa23caec |

로컬 보관: `%TEMP%\jajan-release-0.3.8-44738817018\symbols`. 공개 Release 자산이나 Sentry에 업로드하지 않았다.

## 공식 플랫폼 결과

아래 검증은 모두 공식 tag v0.3.8의 head `42907267ead9438b29b23ace307581f759d2cbe0`에서 실행했다.

| 플랫폼 | 공식 job | Rust | UI | 필수 검사·설치 파일 |
|---|---|---|---|---|
| macOS ARM64 | [136296986756](https://github.com/egoing/agent-gibbon/actions/runs/44738817018/job/136296986756) | 324 통과, 기존 1 ignored | 247 통과, 기존 1 생략 | fmt/Clippy/build/Developer ID/자산 수집 성공 |
| macOS Intel | [136296986702](https://github.com/egoing/agent-gibbon/actions/runs/44738817018/job/136296986702) | 324 통과, 기존 1 ignored | 247 통과, 기존 1 생략 | fmt/Clippy/build/Developer ID/자산 수집 성공 |
| Windows x64 | [136296986717](https://github.com/egoing/agent-gibbon/actions/runs/44738817018/job/136296986717) | 311 통과 | 246 통과, 기존 조건부 2 생략 | fmt/Clippy/build/심볼 일치/자산 수집 성공 |

Windows UI의 생략은 POSIX executable fixture와 기본 캐시 정책 검사다. CI의 명시적 CARGO_BUILD_BUILD_DIR 환경 때문에 두 번째가 생략됐으며 로컬에서는 해당 검사도 통과했다. 새 보조창 ACK 회귀 검사는 생략하지 않았다. Windows 심볼 단위 검사도 4개 통과했다.

macOS job 로그에서 codesign deep/strict 및 Identifier=app.jajan, 정해진 Team/Developer ID identity 검사 성공을 확인했다. Windows updater 서명을 Authenticode 코드 서명으로 보고하지 않는다. [assemble job136308336126](https://github.com/egoing/agent-gibbon/actions/runs/44738817018/job/136308336126)의 서명 검증·manifest·체크섬 생성도 성공했다.


## 최종 공개와 다운로드 검증

- 공식 Release 실행 `44738817018`은 headSha `42907267ead9438b29b23ace307581f759d2cbe0`, completed/success. validate/세 플랫폼/assemble/[Draft job136308509378](https://github.com/egoing/agent-gibbon/actions/runs/44738817018/job/136308509378) 모두 성공.
- [Jajan v0.3.8 정식 릴리스](https://github.com/egoing/agent-gibbon/releases/tag/v0.3.8)는 2026-11-20 22:13:35 KST 공개. isDraft=false, isPrerelease=false, GitHub latest API의 tag_name도 v0.3.8.
- Draft 자산 13개를 내려받아 실제 이름 집합, 설치 파일 3개, 11개 payload의 SHA-256, 세 플랫폼 minisign/Ed25519 payload 및 trusted comment의 버전/파일명 서명을 검증했다. 모든 플랫폼 공개 키는 공식 Actions TAURI_UPDATER_PUBLIC_KEY와 일치했다.
- Draft는 GitHub가 임시 untagged URL을 표시하므로 공개 전에는 실제 자산명과 manifest의 정식 v0.3.8 URL을 대조했다. 공개 후에는 API의 실제 공개 URL과 manifest URL까지 정확히 대조했다.
- 공개 URL에서 curl로 인증 없이 자산 13개를 전부 다시 다운로드했고 위 서명/체크섬/이름/URL 검증이 반복 통과했다. [최신 업데이트 피드](https://github.com/egoing/agent-gibbon/releases/latest/download/latest.json)의 바이트 해시도 이번 release latest.json과 같았다.
- 자산명에는 공백/변형 파일명이 없고 공개 자산에 PDB·심볼 EXE·심볼 manifest는 포함하지 않는다. Windows 공개 EXE는 NSIS 설치 패키지다. 기존 0.3.6/0.3.7 태그와 자산을 덮어쓰지 않았다.
- 검사 파일은 `%TEMP%\jajan-release-0.3.8-44738817018` 아래 draft/public/symbols로 분리 보관했다. 키/인증 비밀은 기록하지 않았다.

### 직접 설치·개발 실행

[Windows x64 설치 파일](https://github.com/egoing/agent-gibbon/releases/download/v0.3.8/Jajan_0.3.8_x64-setup.exe), [macOS ARM64 DMG](https://github.com/egoing/agent-gibbon/releases/download/v0.3.8/Jajan_0.3.8_aarch64.dmg), [macOS Intel DMG](https://github.com/egoing/agent-gibbon/releases/download/v0.3.8/Jajan_0.3.8_x64.dmg), [체크섬](https://github.com/egoing/agent-gibbon/releases/download/v0.3.8/SHA256SUMS.txt)을 제공한다. 릴리스 노트에도 이 링크와 데이터 폴더 삭제 없이 기존 설치를 수동 업그레이드하는 절차를 넣었다. 실제 데이터 보존 설치/재시작 E2E를 수행했다고 주장하지 않는다.

기본 root는 `C:\Users\egoin\dev\project\agent-gibbon`, 배포 root는 `C:\Users\egoin\.codex\worktrees\release-0-3-6\agent-gibbon`이다. 각 root에서 `scripts\jajan-dev.cmd open`으로 해당 표준 개발 바이너리를 실행한다. 두 root는 별도 인스턴스다. 이 명령은 자동 재빌드하지 않으며 이번 최종 코드 이후 두 root의 표준 dev:build를 준비했다. 이미 실행 중인 사용자 앱이나 다른 개발 앱에는 새 빌드가 자동 적용되지 않는다. 사용자 설치 앱은 작업 내내 보존했다.

Conf 4: 새 SHA의 세 플랫폼 CI와 공개 설치/updater 파일·서명·피드 실물 다운로드 검증. Risk Medium: 업데이트 설치/상태 보호 경계. 실제 구버전→후보 설치·재시작 E2E 및 macOS 실기기는 미검증이다. 새 공개가 실행 중 구버전의 ACK 코드를 고치지 않는다는 한계를 유지한다.
