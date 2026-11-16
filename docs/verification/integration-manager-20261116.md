# 웹 세션 종료와 관리 창 검색 통합 검증

[#119](https://github.com/egoing/agent-gibbon/issues/119), [#184](https://github.com/egoing/agent-gibbon/issues/184), [#30](https://github.com/egoing/agent-gibbon/issues/30)을 main `22dc7de7309f8d47772bb0cf1f2c88768f742ef8` 위에서 함께 검증했다. 브랜치는 `codex/issue30-manager-search`, 계획 커밋은 `3ee80bf59b9b50b0e8547946deeb464729367609`다. [Draft #211](https://github.com/egoing/agent-gibbon/pull/211)은 야간 검증 당시 계획만 담고 있었다. [Draft #205](https://github.com/egoing/agent-gibbon/pull/205)와 [Draft #206](https://github.com/egoing/agent-gibbon/pull/206)은 이후 main에 병합됐다. #211은 그 위에 검색과 세션별 갱신을 적용한다.

## 통합 동작

- 툴바 X와 OS 닫기는 웹 세션을 종료한다. 관리 창과 CLI의 숨기기는 세션을 유지한다. 관리 목록은 종료 요청 중인 웹을 제외하며, 종료 실패로 되돌아간 레코드는 같은 세션으로 조회한다. 업데이트 전 검사에서는 Destroyed까지 호스트가 살아 있다. 같은 ID의 새 호스트는 새 수명 번호를 가진다.
- 웹 행에는 전체 지정 URL 또는 Welcome, 호스트 생성 시의 로컬 원래 제목, 표시 상태, 수명 번호가 전달된다. 검색은 로컬 두 제목을 문자열로 비교한다. 관리 창의 조회는 원격 본문·탐색 기록·파일을 읽거나 웹 콘텐츠에 IPC를 보내지 않는다. 웹 종류 라벨은 목록 제목과 독립적으로 보인다.
- 관리 화면은 종류·ID·웹 수명으로 행과 선택을 식별한다. 동일 세션의 Welcome→URL 전환은 기존 행을 유지한다. 새 수명이나 다른 종류가 재사용한 ID에는 이전 선택·표시 요청을 넘기지 않는다. 웹 파괴 및 Windows 관리 동작과의 순서를 맞추기 위해 Windows의 비동기 명령은 실제 조회·동작을 메인 스레드에 전달한다. 웹 표시 명령은 등록과 수명을 재검증하고 일반 메모 생성 경로로 넘어가지 않는다. 일반 메모와 터미널도 실제 라우팅 직전 예상 종류를 확인한다.
- 텍스트 본문은 해당 ID의 캐시 토큰이 일치할 때 생략한다. 이미지 재요청·디코딩 후 선택·스크롤을 보존하고, 웹과 터미널 행은 본문을 색인하지 않는다. 표시나 종료는 사용자가 해당 동작을 직접 실행할 때만 일어난다.

## 시작 기록 경합

수집한 창 목록의 파일 쓰기가 웹 종료 처리 뒤로 밀리는 순서를 제어해, 처음 검토본에서 닫힌 행이 실제 임시 시작 기록 파일에 되살아나는 것을 확인했다. 체크포인트가 사용하는 수집 콜백에 임시 파일의 합성 창 상태를 제공했고, 수집 및 파일 저장 사이에 실제 종료 기록 제거를 실행했다. OS 창 수집과 파괴는 대역이다.

현재는 수집 시작 때의 세대와 파일 쓰기 직전의 세대를 대조한다. 종료가 끼어든 체크포인트를 폐기하고 닫기 처리와 실패 복구 중에는 새 수집을 보류한다. 다음 정상 체크포인트는 다시 저장한다. 네이티브 파괴 요청 동안 파일 mutex를 풀며, 실패한 경우 해당 웹 행과 해당 모니터 기록만 복구한다. 재진입한 체크포인트가 수집을 시작하지 않고 반환하며, 그 사이 다른 웹의 종료가 성공하면 첫 세션의 rollback 뒤에도 두 번째 행이 사라진 채로 남는 것을 확인했다.

## 실행 결과

| 검사 | 결과와 경계 |
|---|---|
| macOS ARM64 15.6.1, Rust 1.91.1, Node 24.21.0 | Rust 270개 통과·실제 셸 4개 제외, UI 205개 통과. Rustfmt·Clippy(all targets, 경고 오류 처리)·워크트리 개발 빌드 통과 |
| 실제 Rust 및 임시 파일 | 종료 실패·중복 종료·두 개 열린 웹/마지막 Destroyed의 업데이트 보류, 같은 ID의 지연 콜백 격리, 시작 기록 삭제·복구·경합을 검사. 시작 기록 관련 10개 통과 |
| Rust 호스트 대역 | 제품의 Session, finish_destroy, manager_entries, 가시성·종류 검사 함수를 추출해 컴파일·실행. 종료 중 show/hide 거절, 실패 복구, 상태 보류, stale 종류·이전 수명 거절, 최소화 해제 경로 확인. 네이티브 API·최종 일반 메모 apply는 대역 |
| 실제 관리 HTML/JS·Chromium 157 | 검색 회귀 15그룹, 웹 혼합 관리 17그룹 통과. Rust 호스트 대역에서 실제 제품 함수가 반환한 6단계 행 배열을 관리 화면에 순서대로 공급했다. 실패 복구·종료·재생성, 늦은 목록·이미지, 900KiB 본문, 한글 조합·포커스·스크롤, 비활성 외부 마크업 색인, 600×360 세 테마 확인. 외부 요청·페이지 오류 0 |
| 웹 툴바와 시안 | 실제 web.html에서 핀 두 번, 종료 실패 뒤 재시도·성공 전달 확인. Tauri IPC는 대역. 텍스트·이미지·터미널·웹 SVG를 1000×660 PNG로 렌더해 검색 입력과 웹 라벨을 확인 |
| 검색 비용 재측정 | 동일 1,000행의 실제 관리 JS와 합성 IPC에서 무변경 왕복 6,105,693→259,528 bytes. 타 본문 변경은 6,105,838→265,578 bytes, 목록/상세 DOM 변경과 선택 이미지 재요청 0. 최초 목록 조회는 6,105,693→6,211,571 bytes. CPU·RSS·네이티브 IPC 실측은 별도 |

Rust 전체 검사는 대화형 셸 rc 실행을 피하도록 아래 네 이름을 각각 `--skip`했다.

- `bash_prompt_commands_and_blank_lines_do_not_replace_the_user_command`
- `real_shell_prompt_builtin_busy_and_child_cleanup`
- `restore_runs_edited_command_in_real_shell_and_reports_program`
- `shell_command_exit_returns_to_same_pty_and_cwd_is_observed`

공유 Cargo 캐시에 다른 워크트리의 빌드 스크립트가 남아 첫 컴파일이 현재 소스에 없는 C 파일을 요구했다. 검증용 APFS 복제 캐시에서 이 패키지의 fingerprint를 새로 생성해 재실행했다. 다른 프로젝트의 파일이나 공유 캐시는 삭제하지 않았다.

작업 루트는 `/Users/egoinglee/.codex/worktrees/27b9/agent-gibbon`, 인스턴스는 `dev-de0f8268143b7edecc650ec7`이다. 개발 바이너리는 해당 루트의 `src-tauri/target/debug/agent-gibbon`에 빌드했다. GUI를 띄우거나 네이티브 OS 닫기·작업표시줄 포커스·실제 원격 탐색/재시작/설치·OS 한글 입력기·Windows·Intel을 실행하지 않았다. 브라우저의 수명 이벤트와 Tauri IPC는 대역으로 구분한다. **Conf 4, Risk Medium**(창 수명·시작 기록·관리 상태).

시안과 동작 기준은 [디자인 시스템](../design-system.md#웹-세션의-관리-목록과-상세-184)을 따른다.
