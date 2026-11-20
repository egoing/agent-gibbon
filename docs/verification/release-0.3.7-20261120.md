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
- [x] PR 병합과 main 동기화
- [x] 새 공식 태그/SHA의 macOS ARM64·Intel·Windows x64 전체 성공
- [x] Windows 동일 EXE/PDB 심볼 artifact와 Debug ID 검증
- [x] Draft 서명·체크섬·자산명·manifest URL 검증
- [x] 공개 및 실제 공개 다운로드·latest feed 검증
- [x] 기본 체크아웃 최신 표준 dev:build와 dev:info

## 중요한 미검증 범위

OS 실제 등록/로그인, macOS 실제 GUI와 이전 공식 버전에서 자동 업데이트 설치·재시작은 이번 배포 CI가 대신하지 않는다. macOS 공증은 현재 사용자 일시 생략 정책, Developer ID/updater 서명은 필수다. Windows Authenticode는 구성하지 않는다. 0.3.5 과거 panic의 PDB 복구나 근본 해결을 주장하지 않는다. 심볼 artifact는 저장소 Actions 권한에 따르며 공개 Release/Sentry에 업로드하지 않는다.

[디자인 시스템](../design-system.md)과 [자동 실행 검증](autostart-settings-20261120.md)의 동작 기준을 유지한다.


## 로컬 최종 결과

Rust 1.99.1에서 fmt, Clippy all-targets -D warnings, Rust 311개 테스트, 최종 `npm run dev:build` 성공. UI 소스는 변경하지 않았으며 동일 소스의 앞선 UI 240 통과/기존 1 생략 결과를 구분해 사용한다. 추가로 `RUSTUP_TOOLCHAIN=1.100.0 node scripts/build.mjs cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`가 전체 통과했다. 기본 toolchain은 바꾸지 않았다. 1.100.0 설치 후 rustup 자체 갱신은 실행 파일 잠금으로 실패했지만 해당 컴파일러 설치와 실제 Clippy 성공은 별도로 확인했다.

## 공식 실행

- [PR261](https://github.com/egoing/agent-gibbon/pull/261) MERGED, head `9659f621b33900f378b10646d03b46c3a664d0c3`, merge SHA `bcc290983480f52e02baba6618ca52679172bc6b`.
- 병합 직전 main 추가 변경 없음. 병합 후 기본 체크아웃 fast-forward 및 local main = origin/main = GitHub main SHA 일치 확인.
- 공식 태그 `v0.3.7`은 위 merge SHA에 발행. [Release 실행 44703881105](https://github.com/egoing/agent-gibbon/actions/runs/44703881105)의 headSha 일치, 진행 중.


기본 root의 최종 `npm run dev:build` 성공(0.3.7). `npm run dev:info`의 ID는 `dev-fbc7d562e05aec4798036a95`, binary는 `C:\Users\egoin\dev\project\agent-gibbon\src-tauri\target\debug\jajan.exe`. `scripts\jajan-dev.cmd --instance-info`도 ok=true 및 기본 root manifest_dir 일치를 확인했다. 워크트리 ID는 `dev-89449df4f6727d7ea014fbec`로 구분한다. GUI 서버 검증이나 이미 실행 중인 프로세스 교체를 의미하지 않는다.

## Windows 심볼 실물 검증

공식 실행의 `windows-symbols-bcc290983480f52e02baba6618ca52679172bc6b` artifact(ID 14614896197, 만료 2026-12-20T08:46:55Z)를 다운로드해 동일 스크립트의 PE CodeView/MSF PDB identity 파서와 SHA-256으로 직접 재검증했다. revision은 release head와 일치, GUID `5108061a-b305-4925-bf1a-ceb90a4ef8a2`, age `1`, Debug ID `5108061a-b305-4925-bf1a-ceb90a4ef8a2-1` 일치.

| 파일 | 크기 | SHA-256 |
|---|---:|---|
| jajan.exe | 24408064 | 9ee685c8f5a7e5660cc44eb28b0d571b005fff83d675f9193dcb27c9218cdb59 |
| jajan.pdb | 85233664 | 0338f4fbc8ca8ef4655751b023deb9d017b4e5bc5efc8423c2b67b911c7bfd57 |

CI Windows job 136178651342의 빌드·심볼 검사·artifact 업로드 성공 확인. 로컬 검증 보관 경로는 `%TEMP%\jajan-release-0.3.7-44703881105\symbols`이며 공개 자산/Sentry 전송은 하지 않았다.


## 최종 공식 CI 결과

Release 실행 `44703881105`는 completed/success. 아래 결과는 모두 tag `v0.3.7`, head `bcc290983480f52e02baba6618ca52679172bc6b`에만 해당한다. 버전 validate, assemble, Draft 업로드/자산명 검증도 모두 성공했다.

| 플랫폼 | 실행 근거 | Rust | UI | fmt / Clippy / 설치 빌드 / 서명 |
|---|---|---|---|---|
| macOS ARM64 | [job136178651419](https://github.com/egoing/agent-gibbon/actions/runs/44703881105/job/136178651419) | 324 통과, 기존 1 ignored | 240 통과, 기존 1 생략 | 전부 성공, Developer ID 검증 성공 |
| macOS Intel | [job136178651438](https://github.com/egoing/agent-gibbon/actions/runs/44703881105/job/136178651438) | 324 통과, 기존 1 ignored | 240 통과, 기존 1 생략 | 전부 성공, Developer ID 검증 성공 |
| Windows x64 | [job136178651342](https://github.com/egoing/agent-gibbon/actions/runs/44703881105/job/136178651342) | 311 통과 | UI 단계 성공 | 전부 성공, updater 서명 및 심볼 일치 검사 성공 |

macOS의 codesign deep/strict 검증과 Identifier=app.jajan·정해진 Team/Developer ID identity 검사 성공을 job 로그에서 확인했다. Windows는 updater 서명이며 Authenticode를 검증했다고 보고하지 않는다. 이전 `44702390360`은 completed/failure로 종료됐고 assemble/Draft는 미실행이다.

## 공개 전후 자산 검증

- [정식 릴리스](https://github.com/egoing/agent-gibbon/releases/tag/v0.3.7) 공개: 2026-11-20 20:47:26 KST, isDraft=false, isPrerelease=false. GitHub latest API도 v0.3.7 일치.
- Draft에서 자산 13개 다운로드 후 파일 집합·설치 파일 3개·11개 파일 SHA-256·세 플랫폼 Base64 minisign/Ed25519 payload 및 trusted comment(버전/파일명) 검증 성공. 공개 키는 Actions의 공식 TAURI_UPDATER_PUBLIC_KEY와 대조했다.
- GitHub Draft의 browser URL은 임시 untagged 주소이므로, 공개 전에는 manifest의 정식 v0.3.7 URL과 실제 자산명을 검증했다. 공개 후에는 manifest URL과 실제 API 자산 URL까지 완전히 일치함을 확인했다.
- 공개 후 curl로 인증 없이 자산 13개 전부 실제 다운로드했다. 같은 서명·체크섬·파일명·URL 검증을 반복하여 성공했고 [최신 업데이트 피드](https://github.com/egoing/agent-gibbon/releases/latest/download/latest.json)의 바이트 해시도 이번 latest.json과 일치했다.
- 공개 자산에는 PDB, Windows 심볼 EXE 또는 심볼 manifest를 포함하지 않는다. Windows 설치 EXE는 NSIS 패키지다.
- 원본 검사 파일: `%TEMP%\jajan-release-0.3.7-44703881105\draft`, 공개 재다운로드: 같은 경로의 `public`, 심볼은 `symbols`로 분리했다.

### 직접 설치

- [Windows x64 설치 파일](https://github.com/egoing/agent-gibbon/releases/download/v0.3.7/Jajan_0.3.7_x64-setup.exe)
- [macOS ARM64 DMG](https://github.com/egoing/agent-gibbon/releases/download/v0.3.7/Jajan_0.3.7_aarch64.dmg)
- [macOS Intel DMG](https://github.com/egoing/agent-gibbon/releases/download/v0.3.7/Jajan_0.3.7_x64.dmg)
- [SHA256SUMS.txt](https://github.com/egoing/agent-gibbon/releases/download/v0.3.7/SHA256SUMS.txt)

기본 개발 실행: `cd C:\Users\egoin\dev\project\agent-gibbon` 후 `scripts\jajan-dev.cmd open`. 배포 워크트리는 앞서 기록한 root에서 같은 명령을 사용하며 별도 인스턴스다. 명령은 자동 재빌드하지 않는다. 이미 실행 중인 앱은 이전 코드일 수 있으므로 출처를 확인하고 사용자가 해당 인스턴스를 재시작해야 한다. 이번 작업에서 다른 앱이나 사용자의 프로세스를 종료하지 않았다.

Conf 4: 세 플랫폼 CI와 실제 공개 자산/업데이트 피드 다운로드 검증 완료. Risk Medium: 정식 배포·업데이트 대상이며 실제 OS 로그인과 설치/업데이트 재시작은 미검증이다.
