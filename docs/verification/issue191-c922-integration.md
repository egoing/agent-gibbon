# #191 표시 후속 c922 main 통합 — 2026-11-16

## 통합과 분리 보존

작업 root `/Users/egoinglee/.codex/worktrees/c9f9/agent-gibbon`, branch `codex/mixed-note-arrangement`.

1. HEAD `5aea837`의 진행 중 f6a08ca 병합 index와 새 UI working tree를 분리했다. 새 UI14개 파일의 원본/해시, 추적6개 index blob, staged main patch와 unstaged UI patch를 공용 gitdir의 `agent-gibbon/task-backups/issue191-20261116-c922`에 보존했다.
2. index tree `4b87f06814c2379e9342a0ae540951f8a7899d64`가 `git merge-tree --write-tree HEAD MERGE_HEAD` 결과와 동일함을 확인하고 main통합만 `870ea2f1d9f9b0847d7e6194f411e5ce35b22b7d`로 커밋했다. 새 UI는 포함하지 않았다.
3. 추적 UI6개를 백업 해시 대조 후 잠시 기준으로 복원하고 신규8개 파일은 유지한 채 `c922f7976fcece0dea3a58125d3c7c745d1e2a63`을 실제 병합했다. index tree `613768c6e3ede59f710d9161e42fb9eee1eef505`도 자동 병합 결과와 일치하여 main통합만 `f7d17ce80dff517dce77274d9044acbdad56a596`으로 커밋했다.
4. UI patch를 3-way 재적용했다. 제품 소스5개는 자동 적용됐고 디자인 시스템 말미의 추가 문단1건만 충돌했다. 관계선의 최신 설명과 #191 눈·불투명 기준을 모두 남겼다.
5. main.rs/stash.rs에서 새 UI의 추가·삭제 행이 백업과 정확히 같고, 다른11개 UI 파일은 byte hash까지 같음을 확인했다. index는 비웠고 UI제품은 unstaged/untracked 상태로 유지했다. 강제 reset·stash·사용자 파일 삭제는 사용하지 않았다.

[보존 대조와 소스 해시](assets/issue191/c922-integration-preservation.json). 원본 백업과 두 index tree 대조 기록은 위 공용 gitdir에 보존한다.

## 보존한 경로

- main의 `sample_memo` 불투명·hover 재진입 정책과 새 relation handler 등록·native event·설치/개발 CLI launcher 분기.
- stash의 다음 동작에 맞춘 눈 아이콘 분기, 관계선 보조창 snapshot 제외, `transition_started` 연결 상태 훅.
- 모니터 복구의 수동 기록·보조창 제외·정렬/undo `RecoveryPoint` 연결.
- 기존 relation의 데이터·picker·settings 경로. 사용자 앱을 열어 관계 데이터를 다시 쓰는 검증은 하지 않았다.

## 검사와 한계

기존 Rust253/UI190은 f6 통합 기준 이력이다. 이번 c922 통합 후의 새 실행 결과는 아래 최종 검사 절에 별도로 기록한다. 새 UI의 native alpha·눈 아이콘 전환은 여전히 실환경 미검증이며 이전20e07f0 정렬·취소 실증을 이번 수정에 재사용하지 않는다.

이번 범위는 source 통합과 관련 검사다. 사용자 앱·PTY·포커스·기하를 조작하거나 앱을 재시작하지 않았다. 새 UI 최종 제품 commit/push·PR192 병합도 수행하지 않았다. 미확정 그라데이션·바둑판 보기 닫기·새 취소 배치는 구현하지 않았다. 새 binary 실증은 팀장에게 별도 차례를 요청한다.

## 최종 검사 — c922 통합 working tree

- `npm test`: Rust **266개**, UI **198개** 통과. opacity 교차 포인터·동일fade, stash 관계 보조창 제외, monitor 관계 보조창 제외, relation/toolbar/guide 검사 포함.
- `npm run check`: Clippy all-targets `-D warnings` 통과.
- `npm run dev:build`: c9f9 최종 debug 빌드 통과(새 프로세스 실행 아님).
- Rustfmt·`git diff --check` 통과. c922가 현재 HEAD의 ancestor임을 확인했다.
- 최종 index는 비어 있고 진행 중 merge는 없다. 추적 UI6개와 신규 UI/검증 문서는 working tree에 보존했다. 두 통합 커밋은 로컬에만 있고 push하지 않았다.

현재 source 통합·자동 검사 범위 Conf3 / Risk Medium(모니터·관계 상태 경로 통합 포함). 새 눈·불투명 UI의 실환경 판정은 별도다. 실행 앱 반영 차례를 배정받기 전에는 새 효과가 사용자 화면에 적용됐다고 보고하지 않는다.

## 최종 제출 전 상태 갱신

팀장 채팅의 사용자 명시 ‘21번 병합’ 승인으로 합의된 두 UI제품의 커밋·push·PR병합을 진행한다. 위 미커밋/미push 표시는 통합 정리 당시 기록이다. 새표시효과의 수동native확인은 답변이없어 미실행이며 자동검사와 분리한다. 앱 정상종료·기존인스턴스복원은 후속기록에 명시한다.
