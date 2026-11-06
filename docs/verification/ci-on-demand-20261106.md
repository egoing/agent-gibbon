# 전체 CI 수동 요청 전환 검증 — 2026-11-06

- 이슈: [#74](https://github.com/egoing/agent-gibbon/issues/74)
- 작업 루트: `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/ci-on-demand`
- 브랜치: `ci-on-demand`
- 기준 커밋: `59d817e40e586b2e27dd56723db82194b4061c5c` (`origin/main`)
- 환경: macOS ARM64, Node v24.21.0, npm 11.19.0, Python PyYAML 6.0.2, GitHub CLI 2.89.0.

## 변경 범위

Release 워크플로에서 `on.push.branches: ['**']` 한 줄만 제거했다. `workflow_dispatch`와 `v*` 태그 push, 전체 검증·빌드·artifact 및 태그 push 전용 게시 조건을 유지했다. 프로젝트 지침에 전체 검사 요청 기준과 대상 SHA·실행 URL·결과 확인 규칙을 기록하고 README와 릴리스 안내를 일치시켰다.

## 실행 결과

| 검사 | 결과 |
|---|---|
| `node --test tests/release.test.mjs` (변경 전 baseline) | 3개 통과 |
| `npm run test:ui` (변경 후) | 릴리스 검사 포함 Node 테스트 95개 통과 |
| `npm run version:check` | `0.2.7`, 버전 일치 |
| `git diff --check` | 통과 |
| PyYAML `BaseLoader`로 변경 전후 workflow 파싱·비교 | 통과: 트리거는 `push.tags: ['v*']`와 `workflow_dispatch`만 존재하며 전체 jobs·permissions·concurrency 동일 |
| 변경 전후 workflow 원문 비교 | `HEAD`의 내용에서 브랜치 push 조건 한 줄만 제거된 것과 정확히 일치 |
| 로컬 문서 링크와 새 섹션 anchor 확인 | 통과 |

YAML 1.1 파서가 `on`을 boolean으로 해석하는 문제를 피하려고 `BaseLoader`로 파싱했다. 아래 표는 파싱한 설정과 보존된 게시 조건을 기준으로 평가한 **정적 검사**이며 GitHub 이벤트를 실제 발행한 결과가 아니다.

| 이벤트 | 전체 CI 실행 대상 | 게시 조건 충족 |
|---|---|---|
| `push`, `refs/heads/main` | 아니오 | 아니오 |
| `push`, `refs/heads/ci-on-demand` | 아니오 | 아니오 |
| `push`, `refs/tags/check-1` | 아니오 | 아니오 |
| `workflow_dispatch`, `refs/heads/ci-on-demand` | 예 | 아니오 |
| `push`, `refs/tags/v0.2.8` | 예 | 예 — 버전 일치·전체 빌드 성공·자산 검증이 추가로 필요 |

## 로컬 검사 당시 미검증과 후속 확인

- 커밋·push·PR 생성·실제 GitHub 수동 실행은 아직 수행하지 않았다. 기존 원격 실행 결과를 이번 변경의 실행 증거로 사용하지 않았다.
- 승인 후 커밋을 작업 브랜치에 push하고 [수동 검사 절차](../releases.md#필요할-때-전체-검사-요청)로 전체 CI를 요청한다. 실행 URL과 `headSha` 일치, 세 플랫폼 검사·설치 파일 빌드·artifact 성공 및 `publish`의 skipped 상태를 확인해야 한다.
- 태그 발행·공개 릴리스는 수행하지 않았다. 실제 배포 검증은 별도 승인된 배포에서 수행한다.
- Rust·앱 코드·UI 변경이 없어 로컬 Rust 빌드·실제 CLI·GUI 검증은 실행하지 않았다. 새 워크트리에 개발 바이너리가 없어 진행 메모용 개발 앱도 실행하지 않았다.
- 확신도: **2** (CI 실행 조건은 정적 검증), 위험도: **Low** (실행 조건과 문서의 국소 변경, 기존 게시 경계 유지).

## 사용자 요청에 따른 재검증

동일 작업 루트의 현재 미커밋 변경을 다시 검사했다. 앱 코드 변경이 없어 개발 앱 빌드·CLI·GUI 검증은 이번 범위에 포함하지 않았다.

| 검사 | 재검증 결과 |
|---|---|
| `npm run test:ui` | 95개 통과, 실패·생략 0개 |
| `npm run version:check` | `0.2.7`, 버전 일치 |
| `git diff --check` | 통과 |
| `actionlint -shellcheck= -pyflakes= .github/workflows/release.yml` | v1.7.12 통과: workflow 문법·표현식·runner 검사. 선택적 외부 ShellCheck·Pyflakes는 실행하지 않음 |
| YAML 파싱·원문 비교 | 브랜치 push 조건 한 줄만 제거. jobs·permissions·concurrency 동일, 세 플랫폼 유지 |
| 이벤트 조건 6개 평가 | 앞 표의 5개 조건과 태그 대상 `workflow_dispatch`를 검사. 태그를 대상으로 수동 실행해도 게시 조건은 불충족 |
| 문서 링크 | README·릴리스 안내·프로젝트 지침의 로컬 링크 대상 37개와 새 섹션 anchor 확인 |
| 원격 상태 조회 | 기본 브랜치는 `main`, 원격 `ci-on-demand` 브랜치는 없음. 기준 커밋은 기존과 동일 |

actionlint는 공식 `rhysd/actionlint`의 v1.7.12 macOS ARM64 배포 파일을 임시 디렉터리에 내려받아 배포 체크섬과 SHA256 일치를 확인한 뒤 실행했고, 검사 후 임시 파일을 제거했다. 검사한 workflow의 SHA256은 `6d2c4daa58b5ef351b08efaef0e93813eb15037ac53553be0b9b6e13358cf392`다.

실제 GitHub 이벤트 검증은 여전히 미실행이다. 원격 기준 커밋의 기존 성공 실행은 이번 미커밋 변경의 실행 근거가 아니다. 프로젝트 지침이 커밋 전 승인을 요구하므로, 승인을 받은 뒤 이번 변경 5개 파일만 커밋·push하여 브랜치 push 자동 실행 부재와 수동 전체 검사 결과를 확인해야 한다. 확신도 **2**, 위험도 **Low**를 유지한다.

## 커밋·원격 검사 착수

사용자가 커밋·병합·push·워크스페이스 삭제를 지시하여 승인 대기를 해제했다. 배포 설정 변경이므로 작업 브랜치 `ci-on-demand`에 검토할 커밋을 push한 뒤 `gh workflow run release.yml --ref ci-on-demand`로 전체 검사를 요청한다. 이 시점에는 원격 검사 결과를 아직 통과로 판단하지 않는다. 실제 대상 SHA·실행 URL·플랫폼별 결과는 완료 후 아래에 추가한다.
