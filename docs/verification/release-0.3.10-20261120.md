# Jajan 0.3.10 정식 공개 검증 기록

## 승인·범위·출처

- 사용자 재배포 승인 원본: local 채팅 `01a1f8d4-7e8b-7410-a774-f6ca635f95f3`, userMessage `01a1f922-f7cd-7110-8098-b279e2ce54d1`의 트레이 아이콘·테마·표시 상태 수정, 버전업과 팀장 재배포 요청을 직접 읽었다. 버전 고정 없는 승인 및 기존 공개 승인에 따라 팀장이 이번 배포 대상을 0.3.10으로 확정하고 기존 Draft 검증·공개를 이 담당에게 명시 배정했다.
- 기존 0.3.10 담당 채팅 `01a1f90a-6125-76f3-9964-a7962420cae3`의 Draft 생성 완료 보고를 읽기 전용으로 확인했다. 해당 담당의 Draft-only 범위를 변경하거나 그 담당을 사칭하지 않고, 별도 팀장 배정에 따라 이 담당이 공개했다. 다른 채팅에 메시지/지시를 보내지 않았다.
- 공개 tag/head: `v0.3.10`, `18eb259775ad3d59230d4be6c29355d9692f1855`.
- [PR266](https://github.com/egoing/agent-gibbon/pull/266)의 Windows 트레이 개선 a1835898이 위 head의 ancestor임을 직접 확인했다. 이후 [PR267](https://github.com/egoing/agent-gibbon/pull/267)의 macOS 로그인 자동 실행 로컬 진단 개선을 포함한다.
- 원격 tag와 실제 main도 공개 직전 위 head와 일치했다. 새 태그/버전/공식 재빌드나 자산 덮어쓰기는 하지 않았다. [0.3.9 첫 시도](release-0.3.9-20261120.md)는 ARM64 테스트 실패와 동시 릴리스 조율 후 미공개로 보존한다.
- root: `C:\Users\egoin\.codex\worktrees\release-0-3-6\agent-gibbon`, branch `codex/release-0.3.10`, start/base `18eb259775ad3d59230d4be6c29355d9692f1855`. 이전 0.3.9 검증 문서는 untracked 상태 그대로 보존해 함께 기록했다. 제품 코드는 수정하지 않았다.
- Windows fcntl 기반 President 등록/음성 도구와 진행 표시의 기존 제약을 우회하지 않고 문서·대화로 기록했다. 사용자 설치 앱/원 담당 구현 공간·백업을 변경하거나 프로세스를 종료하지 않았다.

## 작업 체크리스트

- [x] 새 공개 배정과 기존 성공 Draft/태그/SHA/main 관계 확인
- [x] 세 플랫폼 공식 test/fmt/Clippy/설치 빌드·서명 단계 재확인
- [x] 동일 Windows EXE/PDB GUID·age·Debug ID·해시 실물 검증
- [x] Draft 13개 자산·설치 파일 3개·updater 서명·manifest·체크섬 검증
- [x] 0.3.10 정식 최신 공개 및 공개 파일/피드 실제 다운로드 검증
- [x] 최종 작업 공간 표준 dev:build/dev:info
- 검증 기록 원격 PR 병합과 최종 main SHA 일치는 이 문서의 병합 PR 및 최종 보고에서 확인한다.
- [x] 최종 기본 체크아웃 표준 dev:build/dev:info

## 공식 CI — 모두 같은 0.3.10 SHA

[Release 실행44747367976](https://github.com/egoing/agent-gibbon/actions/runs/44747367976)은 completed/success, headSha `18eb259775ad3d59230d4be6c29355d9692f1855`. validate, 아래 세 플랫폼, assemble136335412032, draft136335585281이 모두 success임을 직접 조회했다. 이전 0.3.8/0.3.9 결과를 이번 head의 통과로 재사용하지 않았다.

| 플랫폼 | 공식 job | Rust | UI | 검사·빌드·서명 |
|---|---|---|---|---|
| macOS Intel | [136326177831](https://github.com/egoing/agent-gibbon/actions/runs/44747367976/job/136326177831) | 328 통과, 기존 1 ignored | 247 통과, 기존 1 생략 | fmt/Clippy/DMG·app archive/Developer ID 성공 |
| macOS ARM64 | [136326177852](https://github.com/egoing/agent-gibbon/actions/runs/44747367976/job/136326177852) | 328 통과, 기존 1 ignored | 247 통과, 기존 1 생략 | fmt/Clippy/DMG·app archive/Developer ID 성공 |
| Windows x64 | [136326179373](https://github.com/egoing/agent-gibbon/actions/runs/44747367976/job/136326179373) | 314 통과 | 246 통과, 기존 조건부 2 생략 | fmt/Clippy/NSIS·updater·심볼 검사 성공 |

로그에서 Rust/UI 수치와 macOS codesign deep/strict·Identifier=app.jajan·정해진 Team/Developer ID identity 검증 성공을 확인했다. Windows 심볼 단위 검사도 4개 통과했다. Windows UI 생략은 POSIX fixture 및 CI 명시적 build-dir에서의 기본 캐시 정책 검사다. Windows updater 서명을 Authenticode 검증으로 보고하지 않는다. macOS 공증 제출/티켓 검사는 기존 사용자 일시 생략 정책을 유지했다.

## Windows 심볼 실물

artifact `windows-symbols-18eb259775ad3d59230d4be6c29355d9692f1855`, ID14634327418을 내려받아 PE CodeView/MSF PDB 파서와 SHA-256으로 직접 검사했다. revision은 release head와 같다. GUID `62a8c2d3-2ea3-4557-aeba-74e28d915f41`, age `1`, Debug ID `62a8c2d3-2ea3-4557-aeba-74e28d915f41-1` 일치.

| 파일 | 바이트 | SHA-256 |
|---|---:|---|
| jajan.exe | 24593920 | d5757d40a1e75e6bd9e42d467461b0f4a281ee100d142aa3f1e29f273cf56c3c |
| jajan.pdb | 85454848 | b886989adaedefde68ada35e1505425a8e755d5953b0996562f2b0dcf06021a0 |

보관 만료 2026-12-20T13:58:47Z, 저장소 Actions artifact 권한을 따른다. 공개 Release/Sentry에 심볼을 업로드하지 않았다.

## Draft와 공개 실물 검증

- 공개 전 실제 Draft 자산 13개 다운로드. 설치 파일 3개와 updater bundle/signature/pubkey 세트의 파일 집합, 11개 파일 SHA-256, Base64 minisign/Ed25519 payload 및 trusted comment 버전/파일명 검증 성공. 모든 공개 키는 Actions의 공식 TAURI_UPDATER_PUBLIC_KEY와 대조했다.
- Draft의 임시 untagged URL은 공개 전 정식 manifest URL/실제 자산명과 구분했다. 공개 후에는 API 실제 tag 다운로드 URL과 manifest URL이 정확히 일치함을 검사했다.
- [Jajan v0.3.10](https://github.com/egoing/agent-gibbon/releases/tag/v0.3.10) 공개 시각 2026-11-20 23:10:17 KST. isDraft=false/isPrerelease=false, latest API도 v0.3.10 확인. 0.3.9는 공개하지 않았다.
- 공개 URL에서 curl로 인증 없이 자산 13개를 전부 다시 다운로드해 위 서명/체크섬/이름/URL 검증을 반복 통과했다. [최신 피드](https://github.com/egoing/agent-gibbon/releases/latest/download/latest.json)의 바이트 해시도 이번 공개 latest.json과 일치했다.
- 공개 자산에는 PDB·심볼 EXE·심볼 manifest가 없으며 Windows 공개 EXE는 NSIS 설치 패키지다. 기존 공개 자산/태그를 수정하거나 덮어쓰지 않았다.
- 검사 파일은 `%TEMP%\jajan-release-0.3.10-44747367976` 아래 draft/public/symbols로 분리 보관했다. 비밀 키와 인증 원문을 저장·출력하지 않았다.

### 공식 설치 파일

| 플랫폼 | 다운로드 | SHA-256 |
|---|---|---|
| Windows x64 | [EXE](https://github.com/egoing/agent-gibbon/releases/download/v0.3.10/Jajan_0.3.10_x64-setup.exe) | e522c9e9dde71a599bd5213e79c90bd9d6c69d2d097e21921c9bf085bafe423b |
| macOS ARM64 | [DMG](https://github.com/egoing/agent-gibbon/releases/download/v0.3.10/Jajan_0.3.10_aarch64.dmg) | 6ec0ecea360de950ab4ff7c456852aed5cfc62c076e6462bee16c14e863e5c40 |
| macOS Intel | [DMG](https://github.com/egoing/agent-gibbon/releases/download/v0.3.10/Jajan_0.3.10_x64.dmg) | 50b6b4b8e02f206aa28f39e09f600c0ef53c88a0390cba1aa1d8aa121bd7e73c |

[SHA256SUMS.txt](https://github.com/egoing/agent-gibbon/releases/download/v0.3.10/SHA256SUMS.txt)에 updater 자산과 공개 키 해시도 있다.

## 사용자 확인과 한계

Windows 실제 메뉴 픽셀/테마 전환/고대비와 macOS 트레이 GUI, 실제 로그아웃 후 자동 실행, 이 담당의 기존 설치본→0.3.10 설치/재시작 E2E는 미검증이다. 원 Mac 담당이 별도 실행 앱에서 자동 실행 상태1을 보고했지만 실제 로그인 성공과 구분한다. Windows 사용자 설치 앱 갱신·메뉴 확인은 원 담당 local01a1f8d4-7e8b-7410-a774-f6ca635f95f3에서 이어간다. 이 담당은 사용자 앱 설치/종료를 실행하지 않았다.

0.3.8에는 보조창 ACK 수정이 포함돼 있다. 0.3.5/0.3.7의 준비 시간 초과가 반복되면 앱의 기존 코드는 새 공개만으로 바뀌지 않으므로 메모·초안을 저장하고 직접 정상 종료한 뒤 공식 0.3.10 설치 파일을 기존 경로에 적용한다. 제거/데이터·설정 폴더 삭제 없이 재실행 후 버전·기존 메모/배치를 확인한다. 해당 데이터 보존 설치 E2E의 완료를 주장하지 않는다.

[디자인 기준](../design-system.md) · [Windows 트레이 검증](windows-tray-theme-20261120.md) · [설명용 SVG](../design/windows-tray-menu.svg) · [설명용 PNG](../design/windows-tray-menu.png). 시안은 실제 메뉴 캡처가 아니다.

Conf 4: 동일 공식 SHA의 CI와 서명·심볼·실제 공개 다운로드/피드 검증. Risk Medium: 정식 배포/업데이트 대상. 실제 사용자 앱 설치와 네이티브 메뉴 GUI는 별도 확인이 남아 있다.


## 로컬 표준 개발 바이너리

배포 작업 root에서 version:check 0.3.10과 최종 npm run dev:build가 성공했다(Node24.22.3/Rust1.100.0). dev:info의 id `dev-89449df4f6727d7ea014fbec`, binary `C:\Users\egoin\.codex\worktrees\release-0-3-6\agent-gibbon\src-tauri\target\debug\jajan.exe`, --instance-info의 ok=true/instance/manifest_dir 일치. 해당 root에서 `scripts\jajan-dev.cmd open`으로 실행하며 자동 재빌드는 하지 않는다.


기본 root `C:\Users\egoin\dev\project\agent-gibbon`도 0.3.10 표준 npm run dev:build에 성공했다. dev:info id `dev-fbc7d562e05aec4798036a95`, binary `C:\Users\egoin\dev\project\agent-gibbon\src-tauri\target\debug\jajan.exe`, CLI --instance-info의 ok=true/instance/manifest_dir 일치. 이 root에서도 `scripts\jajan-dev.cmd open`을 사용한다. 배포 워크트리와 별도 인스턴스이며 이미 실행 중인 앱에는 새 빌드가 자동 반영되지 않는다. 사용자 설치 앱의 실제 갱신은 원 담당에게 맡겼다.
