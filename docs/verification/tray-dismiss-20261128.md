# 트레이 메뉴 닫기 확인 — 2026-11-28

macOS 15.6.1 ARM64에서 수정 전후 같은 워크트리의 개발 바이너리를 실행했다. 기존 경로는 tray-icon의 보조 뷰가 마우스 입력을 받고 상태 버튼의 클릭을 합성한다. 수정본은 같은 메뉴와 액션을 사용하며 실제 클릭 대상은 AppKit 상태 버튼이다.

| 항목 | 결과 |
|---|---|
| 최종 개발 빌드 | PASS · `npm run dev:build` |
| 개발 앱과 메뉴 출처 | PASS · 검증 메모 `tray-8b91`의 inspect 응답, 인스턴스와 실제 창 제목 대조 |
| 메뉴 연결과 열기 | PASS · PID 27088의 상태 항목에서 기존 메뉴 행을 확인했다. 닫힌 메뉴는 0×0, 열린 메뉴는 (1098, 25)에서 340×278pt였다. 열린 동안 메인 스레드는 AppKit 상태 버튼의 네이티브 메뉴 추적 루프에 있었다. |
| Esc로 닫기 | PASS · 같은 바이너리를 다시 실행한 PID 29430에서 열린 메뉴(340×278pt)에 Computer Use로 Escape를 보냈다. 메뉴는 0×0이 되었고 앱은 실행 상태를 유지했다. |
| 수정 전 재현, 수정 후 메뉴 밖 클릭·항목 실행 | BLOCKED · Computer Use가 시험 메모 창에는 입력을 전달했으나 상태 항목 클릭은 `windowNotFoundAtPosition`으로 거부했다. |
| Windows | NOT RUN · 잠긴 의존성 tray-icon 0.25.1에서 전경 창 설정, TrackPopupMenu, WM_NULL 게시 경로를 확인했다. macOS의 클릭 합성 코드를 사용하지 않는다. |

시험 ID는 `tray-8b91`, 워크트리는 `/Users/egoinglee/.codex/worktrees/8b91/agent-gibbon`, HEAD는 `c6d7c24c1212e9540a9e110fe80d94d897cc92f4`이다. 개발 인스턴스는 `dev-b73a140c3e480bbdccff24bc`다. 원본 바이너리는 이 워크트리의 `src-tauri/target/debug/jajan`이며 Computer Use 식별을 위해 시험 앱 번들에 같은 파일을 하드링크했다. 기준 빌드의 SHA-256은 `724fcc85d6ff2edfb79a868c4dc9c87bef8321194582f853bbfb12615b306813`, 미커밋 수정을 포함한 최종 빌드는 `2aea15322923c3c5c0ecf1d497a903ca1357b17d3b0700c866282d42d1019016`이다.

각 시험은 기본 체크아웃의 정책을 다시 읽은 뒤 공통 파일 잠금을 보유한 상태에서 진행했다. 기준 빌드 PID 25110, 수정 빌드 PID 27088과 29430을 순서대로 하나씩 실행했다. 마지막 시험 앱을 정상 종료하고 Jajan 프로세스 0개를 확인한 뒤 잠금을 반환했다.
