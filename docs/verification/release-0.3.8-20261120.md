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
- [ ] PR 병합과 기본 main fast-forward 및 SHA 일치
- [ ] 공식 v0.3.8 새 SHA의 세 플랫폼 전체 검사·빌드·서명 성공
- [ ] Windows 동일 EXE/PDB GUID·age·Debug ID·해시 artifact 확인
- [ ] Draft 설치 파일 3개·updater 서명·manifest·체크섬·자산명 검증
- [ ] 정식 최신 공개 후 실제 공개 다운로드와 latest feed 검증
- [ ] 기본 체크아웃 최신 표준 dev:build/dev:info

실제 구버전→후보 설치/재시작 E2E, macOS 실기기 검증은 미실행이다. macOS 공증은 기존 사용자 일시 생략 정책을 유지하며 Developer ID/updater 서명은 필수다. Windows Authenticode는 구성하지 않는다. Windows 심볼은 저장소 Actions 권한을 따르는 별도 artifact에 보관하고 공개 Release 자산이나 Sentry에 업로드하지 않는다.


## 최종 로컬 결과

0.3.8 최종 버전에서 Node 24.22.3/Rust 1.100.0으로 UI 247 통과(기존 POSIX fixture 1 생략), Rust 311 통과, Rustfmt/Clippy all-targets -D warnings 통과. `npm run dev:build`도 성공했다. `dev:info`의 root는 위 작업 공간, id `dev-89449df4f6727d7ea014fbec`, binary `C:\Users\egoin\.codex\worktrees\release-0-3-6\agent-gibbon\src-tauri\target\debug\jajan.exe`. 개발 CLI `--instance-info`의 ok=true, instance/manifest_dir도 일치했다. 이는 바이너리 자체 출처 검사이며 서버 GUI/업데이트 E2E로 간주하지 않는다. 기존 사용자 설치 앱 PID 2832는 조회만 하고 보존했다.
