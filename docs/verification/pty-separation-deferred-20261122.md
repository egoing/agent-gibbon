# 셸·PTY 프로세스 분리 유예와 선별 제거

사용자는 Windows/Mac 분리 조사를 장기 과제로 보존하고 업데이트 시험 기능은 유지하도록 결정했다. 조사·실측·미완료·공통화 제안·재개 조건은 [장기 이슈 #279](https://github.com/egoing/agent-gibbon/issues/279)에 기록했다. old8405 GUI CPU100%·창 생성 정체는 [별도 #278](https://github.com/egoing/agent-gibbon/issues/278)이며 분리 기능과의 인과를 확정하지 않았다.

## 작업 출처와 선행 확인

- root: `C:\Users\egoin\.codex\worktrees\04fb\agent-gibbon`
- branch: `codex/defer-pty-separation`
- base: `859c9d9cdedc89ac9d2a7dd1b0ac6591dcc6acb7` (Windows PR #277 병합)
- 기존 구현: Windows `26aa1894c4d1fef734330f4b5bbc7c5aeb8121c8`, Mac `974197adbc34f2f72872eb4e6262e270db90ef2d`
- 단일 구현/PR 소유 배정과 Mac 보존·인계 gate 후 이 워크트리에서만 수정했다. 기존 `.tmp`는 보존했다. main 병합은 이 PR 준비 뒤 별도 순서 지정 단계다.
- MacBook 담당 인계 시각은 2026-11-22 08:23:04 KST다. 기본 checkout clean과 병합 PoC 인계는 확인했지만 다른 Mac 워크트리의 미커밋 변경은 존재한다. iMac은 접속 불가라 최신 상태가 미확인이다. 이는 모든 Mac이 clean이라는 뜻이 아니다. 전체 보존 목록과 마지막 관측은 #279에 남겼다.
- Mac의 Python 중계는 debug opt-in PoC만 해당한다. 일반/정식 Mac은 직접 셸 실행이었다. Mac은 AST/diff 확인만 인계됐고 동일 셸 재접속 실증·전용 실측 보고서·실제 lifecycle.log 경로/존재는 미검증이다.

## 선별 제거와 보존

| 제거 | 보존 |
|---|---|
| Windows owner 모듈·관리 status/enable API·CLI helper/실험 제어 | 일반 PTY 생성·셸/소유 작업 종료·창 생성/포커스 |
| Windows opt-in 환경변수·자동 reattach·remote Child 수명 분기 | 기존 terminal metadata·restore_plan·사용자 데이터 |
| 분리 세션의 업데이트 예외·owner checkpoint 판정 | 터미널 시작/실행/정리·Welcome·웹 창·저장/편집의 기본 업데이트 보호 |
| Mac debug Python client/owner PoC와 선택 환경변수 | 일반 Unix 셸 시작·프롬프트 hook·기존 자동 복원 정책 |
| 분리 전용 Windows 의존성과 owner 통합 시험 fixture | 일반 metadata fsync, verification instance 구분, debug verification updater 기반 |

과거 저장된 업데이트 snapshot의 터미널 창 복원은 남겼다. 분리 API와 자동 reattach가 없으므로 일반 Welcome UI로 돌아오며 새 셸·저장 명령을 자동 실행하지 않는다. 기존 owner cache·ticket·opt-in 파일이나 사용자 데이터를 지우는 migration은 수행하지 않았다. 현재 실행 앱·셸·owner를 종료하거나 옮기지 않았으며 Mac 로컬/iMac 미커밋 파일·브랜치를 수정하거나 강제 동기화하지 않았다. 과거 PIDs의 현재 생존을 주장하지 않는다.

## 업데이트 메뉴 담당에게 넘길 경계

PR #276의 인계 head는 `419d36db6659d393baa6049c0e22d199b0f17e2a`다. main의 이 제거 PR을 선행 통합한 뒤 다음 범위를 처리해야 한다. 이 PR에서는 다른 5170 워크트리나 그 PR을 수정하지 않았다.

- `terminal.rs`, `terminal/lifecycle.rs`, `terminal/persistence.rs`, `terminal/windows_owner.rs`의 PR277 이후 transport/worker/gate 진단·수신 복구를 충돌 해결로 다시 도입하지 않는다. 삭제된 owner 파일은 유지하지 않는다.
- `developer-updater.js`의 persistence 요청·refresh·5회 클릭 enable 호출과 `developer-terminal-status` UI만 제거한다. 5회 클릭 진입과 업데이트 버튼 자체는 유지한다.
- 분리 전용 `poc_control`은 제거하되 `updater_test_*`, `verification-roundtrip`, 서명·플랫폼·버전 바인딩·캐시·수동 대기·모든 창 ACK·기본 terminal 보호는 유지한다.
- #275는 업데이트 시험 범위로 정리하고 #279와 연결한다. 수정 설치 쌍의 실제 왕복과 최종 통합은 별도 검증하며 과거 독립 PTY 성공으로 대체하지 않는다.

## 검증

| 검증 | 실제 결과 |
|---|---|
| `npm test` | Rust315 통과, UI251 통과/2 skip. 시작 세션·정리 worker·닫기 중 업데이트 보류, legacy detached 표시가 있어도 자동 셸/reattach 실행 없음 확인. [로그](pty-separation-deferred-20261122/tests.log) |
| `npm run check` | Clippy all-targets `-D warnings` 통과. [로그](pty-separation-deferred-20261122/clippy.log) |
| `npm run dev:build` | 최종 변경에서 성공, 57.55초. [로그](pty-separation-deferred-20261122/dev-build.log) |
| `npm run dev:info` / `--instance-info` | root04fb, branch codex/defer-pty-separation, instance dev-9f4318a9190cf96216037515 및 해당 binary 출처 확인. [dev info](pty-separation-deferred-20261122/dev-info.log), [instance](pty-separation-deferred-20261122/instance-info.json) |
| 제거 CLI 실행 확인 | 개발 실행기로 `--poc-owner-info`, `poc-quit`를 호출해 exit1/사용법 오류로 거부됨 확인. 앱·owner를 생성하거나 종료하지 않았다. [결과](pty-separation-deferred-20261122/retired-cli.jsonl) |
| 선별 범위 정적 대조 | 일반 셸 생성은 dff06f8과 동일, 일반 lifecycle은974197a와 동일, verification instance 유지, metadata sync·과거 snapshot UI 복원 유지. 분리 진입/API/환경변수 참조는 실행 소스에서 없음. [대조](pty-separation-deferred-20261122/scope-audit.json) |
| `git diff --check` | 통과 |
| Mac 빌드/실기기 및 Windows 일반 셸 GUI 실기기 | 이번 작업에서 수행하지 않음. Mac PoC 검증으로 확대하지 않음 |
| PR276 업데이트 시험의 수정 설치 쌍 왕복·통합 배포 | 이 PR 범위에서 수행하지 않음. 메뉴 담당 후속 검증 대상 |

Windows owner 전용 실기기 시험 fixture는 삭제된 기능과 함께 제거했다. 과거 통과를 이번 제거 코드의 실행 결과로 계산하지 않았다. 사용자 설치 앱 교체·릴리스는 수행하지 않았고 현재 사용자 앱·셸·owner는 조작하지 않았다.

Conf **3**: 이 Windows 환경의 자동 회귀·실제 개발 바이너리 CLI/빌드 검증 범위. Mac은 정적 비교 범위이며 실기기 미검증. Risk **Medium**: 터미널 수명·복원·업데이트 보호 상태에 영향.

실행 명령:

```powershell
Set-Location 'C:\Users\egoin\.codex\worktrees\04fb\agent-gibbon'
npm test
npm run check
npm run dev:build
npm run dev:info
scripts/jajan-dev.cmd open
```

`open`은 자동 재빌드하지 않는다. 변경 감시는 `npm run dev`다. 개발 binary는 이 워크트리의 `src-tauri\target\debug\jajan.exe`, instance는 `dev-9f4318a9190cf96216037515`다. 다른 워크트리·설치본·release 파일로 대체하지 않는다.

이 작업은 장기 조사 근거를 폐기하는 revert가 아니다. 과거 Windows 검증 보고서·증거는 그대로 남기고 현재 기능 지원과 혼동하지 않도록 유예 안내를 추가했다. 기존 세션의 향후 이행·명시 종료·lease·checkpoint 정리 정책은 #279의 재개 조건이다.
