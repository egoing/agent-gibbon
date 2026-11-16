# 서식 메뉴와 취소한 초안 복구 결합 검증

[이슈 #183](https://github.com/egoing/agent-gibbon/issues/183)의 서식 버튼 세 개와 [이슈 #15](https://github.com/egoing/agent-gibbon/issues/15)의 초안 복구를 같은 편집기에서 검증했다. 기준 main은 `22dc7de`, 전용 워크트리의 계획 커밋은 `41c19a3`이다. 개별 검토본의 PNG를 복사하면 다른 변경이 사라지므로 두 SVG 변경을 함께 적용해 1040×840 PNG를 새로 렌더했다. 서식 메뉴와 복구 패널을 모두 육안 확인했다. [디자인 시스템](../design-system.md) · [SVG](../design/milkdown-editor.svg) · [PNG](../design/milkdown-editor.png)

| 경로 | 결합 소스에서 확인한 결과 |
|---|---|
| 정상 서식 적용 | 버튼 문구·도구 설명·접근성 이름이 ‘큰제목’, ‘작은제목’, ‘코드’로 일치한다. 선택한 범위에 제목 수준 1·2 또는 인라인 코드를 적용하고 해당 Markdown을 저장한다. |
| 같은 revision의 초안 | 세 서식 각각을 적용하고 저장 실패 후 취소·복구했다. undo는 원문을, redo는 서식을 복원했다. 추가 서식 변경은 정상 자동저장됐다. |
| 외부 revision과 충돌한 초안 | 세 서식 각각에서 최신 원문으로 undo한 뒤 redo·서식 변경을 수행했다. 750ms 동안 자동 쓰기가 없었고 명시 저장은 초안의 최초 revision으로 거절돼 최신 외부 원문이 유지됐다. |
| redo에 남은 충돌 초안 | ProseMirror의 직접 비이력 변경과 비이력 부모에 덧붙인 변경에서 redo 이력 한 건이 유지됐다. redo 후에도 자동 쓰기 0, 명시 저장은 옛 revision으로 거절됐다. |
| 144px 키보드 | Alt+F10으로 선택 메뉴에 진입하고 메뉴 내부 가로 스크롤과 방향키로 서식을 선택했다. 복구 후 작은제목·코드 적용, Ctrl/Cmd+S 저장 실패 후 재취소·재복구, Feedback의 Tab·Enter 접근을 확인했다. |
| 자동저장 및 기존 입력 보호 | 저장 중 새 입력, 저장 실패·재시도, 합성 조합 이벤트, 원문 보호, 숨김·복원, 유지 중인 버튼의 오래된 이벤트, 별도 WebView 격리, Esc 저장 후 숨김을 확인했다. |

`node scripts/cancel-draft-browser.mjs`는 기존 23개와 결합 경로 7개를 실행한다. `node scripts/milkdown-browser.mjs --format-menu-only`와 전체 모드도 결합 소스로 실행했다. 전체 모드의 툴바 검사는 제품의 현재 `pointerdown` 이동 계약에 맞춰 바로잡았다. 본문 클릭과 툴바 3px 이동은 드래그가 없고 4px에서 `chrome_drag`가 한 번 호출된다. 이전 개별 제안에서 실패한 옛 `mousedown` 검사 결과는 그대로다. #202의 네이티브 텍스트 입력은 미검증이다.

2026-11-16 macOS arm64의 독립 Chromium 158에서 실제 편집기·선택 메뉴·Feedback을 실행했다. Tauri 저장소·창·포커스 이벤트는 대역이며, 두 비이력 시험은 테스트 페이지에서 실제 ProseMirror 트랜잭션을 주입한다. 브라우저 오류와 외부 요청은 0건이었다. UI 198개와 Rust 263개가 통과했다. 사용자 로그인 셸 프로필을 실행할 수 있는 대화형 셸 시험 4개는 제외했다. rustfmt, Clippy, 개발 빌드 및 전용 바이너리의 인스턴스 식별이 성공했다.

공유 Cargo 캐시는 전용 APFS 복제본을 만들었다. 복제 안의 앱 crate와 build script 산출물을 재생성했고, 새 build-script의 root-output과 최종 바이너리 경로·해시를 확인했다. 작업 루트는 `/Users/egoinglee/.codex/worktrees/a15c/agent-gibbon`, instance는 `dev-746209b8f15ed327664fcfa9`다. 설치 앱·개발 앱 GUI, 실제 한글 IME, Windows·Intel, 전체 플랫폼 CI와 배포는 이번 검증 범위 밖이다.
