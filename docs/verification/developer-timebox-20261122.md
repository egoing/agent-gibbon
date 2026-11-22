# 개발자 메뉴 통합 — 20분 마감 기록

> 현재 업데이트 전용 범위와 최신 검증은 [2026-11-22 기록](developer-update-only-20261122.md)을 따른다. 아래는 분리 연동을 포함했던 과거 회차의 증거다.

사용자 마감: 2026-11-22 00:18:16 KST. 이 기록은 완료 보고가 아니다. PR #276은 Draft이며 미병합·미배포다.

- 작업 root: C:\Users\egoin\.codex\worktrees\5170\agent-gibbon, branch codex/developer-update-test.
- HEAD bef3303은 서버 accepted socket의 blocking 복구, 완전 응답/제로수신 Reset의 제한된 read/status 복구, 검증 전용 gate 기록과 관련 테스트를 포함한다. write/attach/stop은 재전송하지 않는다. Windows 관련 회귀 9개 통과.
- 그 뒤 변경은 Clippy가 지적한 두 map_err 기록 함수를 inspect_err로 바꾼 동작 동일 변경이다. post-commit-diff.patch 참조. 이 변경 후 Clippy 재실행은 마감 내 미완료다.
- 실제 f196 GUI에서 ConnectionReset/raw10054 → reader 종료(cancelled=false) → 세션1/worker1을 계측했다. 완전 응답 복구안은 received_len=0/complete_object=false를 안전하게 거부했고, 이것만으로 해결됐다고 주장하지 않았다.
- 수정 GUI9220에서는 동일 인증 owner35576/셸7588에 대해 empty-reset-retry 실제 실행 뒤 reader 생존과 실제 설치 요청의 sessions1/workers2/gate ready를 확인했다. 숨긴 Welcome는 별도 보호로 설치를 보류했다. 고정 변수731은 수정 후 재초기화 없이 최종 조회에 성공했다. restore_plan 실제 값은 function이며 전후 보존했다.
- 서버 accept의 nonblocking 상속으로 지연 요청이 WouldBlock/0 bytes가 되는 Windows 재현을 확인했고 새 owner는 blocking 전환·정상 응답 shutdown을 적용했다. 살아있는 owner35576은 기존 바이너리라 클라이언트 호환 복구를 사용한다.
- read가 서버에서 이미 drain된 뒤 응답을 잃은 경우 화면 출력 누락 가능성은 프로토콜2에서 완전히 복원하지 못한다. 셸/변수 보존은 전체 화면·출력 무손실 보장이 아니다. read/status는 최대3회, read 복구 사이 건강 status도 각각 최대3회이므로 최대9 RPC이며 단순 6초 상한으로 표현하지 않는다. 부분 응답/Timeout/인증 거절/영구 오류는 실패·보류한다.
- 수정된 release .12/.13 NSIS 쌍은 fixed-release-plan.json의 fenSVI에 준비했다. 새 임시 서명 키는 메모리에만 있었고 공개 키만 저장했다. 같은 f196 instance/제품/기존 설치 경로를 유지하므로 이후 GUI 교체 시 새 공개 키와 cache 재검증을 고려해야 한다. 해당 수정 쌍의 실제 양방향 설치는 아직 실행하지 않았다.

## 미완료

실제 수정 release 쌍의 5회 클릭 연동 양방향 NSIS·같은 셸/731/계획·안정 좌표 보존, 재시작 신규 OFF 통합 확인, 동일 버전 재설치, 최종 Clippy·개발 빌드 결과 확인, PR276 ready/병합·main 최종 동기화·기본 checkout 개발 빌드·배포 인계가 남았다. 과거 독립 왕복과 PTY 독립 release 검증은 현재 수정 쌍의 실행 증거로 대체하지 않는다.

## 실행 중 및 재개

- 사용자 설치본과 old trial은 조작하지 않았다. 현재 전용 GUI13684, 기존 owner35576/셸7588을 보존했다. 절전은 팀장이 처리하며 이 작업은 실행하지 않는다.
- 실제 NSIS 설치/파일 교체는 진행 중이지 않다. fenSVI와 htC7ol의 loopback feed는 대기 중이다. 신규 작업은 마감 후 시작하지 않는다.
- 최종 개발 빌드는 5170에서 npm run dev:build, session25842, htC7ol/deadline-dev-build.log로 실행했다. 마감 시 완료 여부를 별도로 확인해야 한다. 개발 instance는 dev-0e8f09996ee653e1d6855ad6, binary는 C:\Users\egoin\.codex\worktrees\5170\agent-gibbon\src-tauri\target\debug\jajan.exe다.
- 재개: npm run check → npm run dev:build → npm run dev:info. 전용 앱 상태는 htC7ol\installed\jajan-roundtrip-5170-f19639c4.exe inspect --id terminal-1795271052106808300 으로 확인한다. owner/셸을 새로 생성하거나 종료하지 않는다.
- 보호 시험용 미실행 Welcome terminal-1795273780833127600은 창 닫기 후에도 기록이 남아 설치를 막는다. 관리 창에서 해당 시작 화면임을 확인해 삭제해야 하며 살아있는 terminal-1795271052106808300은 보존한다.
- 재실험은 fenSVI의 새 key가 포함된 최신 GUI를 출처 확인 후 전용 경로에서 시작하고, 기존 캐시의 서명 재검증/수동 대기를 유지한 채 수행한다. 사용자 설치본 교체는 금지다.

Conf5: 이 Windows 기기의 reader 실패·복구/worker2/같은 셸731 보존 범위. 전체 기능의 최종 NSIS 통합은 미완료. Risk Medium.

## 마감 직후 종료 상태 확인

7f5636ff8d967cf32a00f40d083cfc5d81bf5edd를 작업 브랜치에 push했고, 이후 새 구현·검사·설치는 시작하지 않았다. 이미 진행 중이던 최종 5170 개발 빌드(session25842)는 exit0, 55.99초로 완료됐다. deadline-dev-build.log를 완료 로그로 갱신했고 final-dev-info.log에 실제 root/id/binary를 보존했다. cargo11968/rustc32264와 makensis/setup 설치 프로세스는 더 이상 조회되지 않았다. loopback feed는 대기 중이며 NSIS 설치는 진행 중이지 않다. GUI13684·owner35576·셸7588은 유지한다. 절전은 팀장이 수행한다.

Clippy manual_inspect 두 건은 inspect_err로 수정했으나 그 후 재실행은 하지 않았다. 최종 양방향 NSIS·후속 보호/좌표 검증·main 병합 및 배포는 미완료다. 최신 개발 실행은 이 워크트리에서 scripts/jajan-dev.cmd open이며, 자동 재빌드는 하지 않는다.
