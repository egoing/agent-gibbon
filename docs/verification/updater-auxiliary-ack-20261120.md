# 보조창 업데이트 준비 응답 수정 — 2026-11-20

## 결과와 범위

방향 안내(direction-guide), 정렬 취소(layout-undo), 모아보기 배경(memo-backdrop), 메모 모아보기(memo-overview)가 기존 updater-guard.js를 로드하고 설치 준비/해제에 참여하도록 했다. 네 창에 로컬 event listen/unlisten 권한만 추가했다. 대기 대상 전체, 3초 제한, 모든 창의 safe ACK 요구, 기존 저장·초안·터미널·웹 세션 보호는 유지한다.

정렬 취소와 모아보기의 프론트엔드 명령은 jajanInvoke를 사용하여 pending에 집계된다. 진행 중인 변경은 safe:false를 보내며 준비 이후 새 입력/명령은 차단한다. JS를 거치지 않는 네이티브 hover-click 정렬 취소도 updater::activity 보호를 적용하여 frozen 상태에서 저장된 Undo를 소비하거나 배치를 변경하지 않는다.

UI 문구·외형·배치는 변경하지 않았다. [디자인 시스템](../design-system.md)의 자동 업데이트 항목에 기존 설치 보호 기준의 보조창 적용을 기록했다. 상세 원인과 실제 0.3.5 실패 증거는 [분석 기록](../analysis/windows-update-ack-timeout-20261120.md)에 있다.

## 작업 체크리스트

- [x] 기존 정책·변경 상태 확인, 분석 전용 워크트리 재사용
- [x] 실제 HTML/권한/스크립트를 읽는 회귀 테스트 작성: 수정 전 7/7 실패 확인
- [x] 네 보조창 준비/해제 참여와 명령 보호 구현
- [x] 수정 후 새 테스트 7/7 및 기존 관련 테스트 포함 27/27 통과
- [x] 전체 UI·Rust 테스트, Rustfmt·Clippy 통과
- [x] 최신 개발 빌드 및 root/id/binary 확인
- [x] 개발 앱 CLI open/inspect로 실행 출처 확인
- [x] 테스트 메모 숨김 및 이번에 실행한 개발 PID만 종료
- [ ] 커밋·push·병합: 사용자 승인 전 미실행
- [ ] 격리 업데이트 설치·재시작 E2E 및 정식 배포: 미실행

## 실행 검증

환경: Windows, Cargo 1.99.1. 호스트 기본 Node 20.11.0은 프로젝트 엔진 범위 미달이라 검증/빌드는 `npm exec --yes --package=node@24 -- ...`로 Node 24.22.3을 사용했다. 글로벌 Node 설정은 변경하지 않았다. `pnpm install --frozen-lockfile`은 lockfile 변경 없이 성공했으며 그 설치 명령은 호스트 Node 버전 경고를 출력했다.

| 실행 | 실제 결과 |
|---|---|
| node --test tests/updater-auxiliary.test.mjs (수정 전) | 7 실패: 각 페이지에 prepare listener 없음 |
| Node 24 관련 4개 테스트 파일 | 27 통과 |
| Node 24 scripts/test-ui.mjs | 248개 중 247 통과, 1 skip, 0 실패 |
| Node 24 scripts/build.mjs cargo test --manifest-path src-tauri/Cargo.toml | 311 통과, 0 실패 |
| cargo fmt --manifest-path src-tauri/Cargo.toml -- --check | 통과 |
| Node 24 scripts/build.mjs cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings | 통과 |
| npm run dev:build (Node 24 환경) | 성공, 43.55초 |
| git diff --check | 통과 |

새 회귀는 guard 함수만 떼어 호출하지 않고 배포할 네 HTML의 script 목록을 읽어 실제 연결된 스크립트를 VM에서 실행한다. 해당 label에 실제 capability의 event 권한이 없으면 listener 등록을 거절하는 mock을 사용한다. 각 보조창의 safe ACK, 입력 차단, release 후 명령 재개, 다음 epoch 재시도를 확인한다. 정렬 취소·모아보기·배경의 실제 핸들러로 시작한 명령을 지연시켜 pending=1/safe:false를 확인하고, 완료·해제 뒤 재시도에서는 safe:true를 확인한다. 기존 테스트가 dirty/저장, pending 드레인, stale epoch, ACK 실패의 설치 차단도 계속 검증한다.

VM 테스트는 실제 WebView2 event 전달/숨김 타이머 검증이나 설치 E2E가 아니다. capability는 Tauri 개발 빌드에서도 정상 컴파일되었다. 실제 새 버전 다운로드·서명·설치·재시작은 하지 않았으며 macOS 실기기 동작도 미검증이다.

## 개발 앱 출처와 정리

- root: `C:\Users\egoin\.codex\worktrees\update-ack-analysis\agent-gibbon`
- branch: `codex/update-ack-analysis`
- HEAD/start/base: `5220c9402aa440e2f354255c239b177f1e1a1d88` (아직 미커밋)
- dev:info id: `dev-064b569be88189ba4c163c22`
- binary: `C:\Users\egoin\.codex\worktrees\update-ack-analysis\agent-gibbon\src-tauri\target\debug\jajan.exe`
- CLI: `scripts\jajan-dev.cmd open --id updater-ack-check --markdown '업데이트 보조창 응답 수정 검증 전용 메모'`, 이어서 같은 root의 `inspect --id updater-ack-check`.
- inspect: `ok:true`, instance가 위 id와 일치, id 및 전용 본문 일치, `focused:false`. 저장소는 `AppData\Roaming\app.jajan\notes\dev-064b569be88189ba4c163c22\docs\updater-ack-check\index.md`로 설치 앱 저장소와 분리.
- Get-CimInstance: PID 31536의 ExecutablePath가 위 binary와 정확히 일치.
- 검증 종료: 같은 CLI로 검증 메모를 close(숨김)하고 해당 PID의 executable 경로를 다시 확인한 뒤 이번에 시작한 프로세스만 종료. 사용자 앱은 종료하지 않았다. 전용 검증 메모는 해당 개발 인스턴스에 숨김으로 보존했다.

바로 개발 앱을 실행하려면 PowerShell에서:

```powershell
Set-Location 'C:\Users\egoin\.codex\worktrees\update-ack-analysis\agent-gibbon'
scripts\jajan-dev.cmd open
```

이 명령은 자동 재빌드를 하지 않는다. 위 최신 개발 바이너리를 준비했으며 프로세스는 종료된 상태다. debug 앱은 실제 자동 업데이트를 실행하지 않으므로 위 실행만으로 설치 경로 검증을 대신할 수 없다.

## 배포와 기존 설치본

현재 사용자 설치본 0.3.5는 이 워크트리 수정으로 바뀌지 않는다. 결함이 기존 앱의 준비 단계에 있으므로 수정판 공개만으로 이미 실행 중인 0.3.5의 ACK 기능을 고칠 수 없다. 배포 승인 시 격리 시험과 설치 파일 준비, 기존 사용자 데이터 보존을 전제로 한 수동 업그레이드 경로를 함께 검토해야 한다. 사용자 앱을 강제 종료하거나 실제 업데이트를 실행하지 않았다.

Conf 3(전체 단위/VM 회귀 통과 + 개발 앱 CLI 출처 확인). Risk Medium(업데이트 설치와 배치 변경의 상태 보호 경계). 커밋·push·PR·병합·공개는 수행하지 않았다. president 등록/보고 CLI는 Windows fcntl 미지원으로 미사용이며 자기 채팅의 현재 제목은 별도 조회하지 않아 최신값으로 기록하지 않았다.
