# #160 메모 내부 Feedback 구현·검증

## 변경과 승인 경계

사용자가 별도 Feedback 창을 메모 내부 하단에 넣고 창 크기를 유지하도록 요청했다. Feedback 표시·숨김·안내 변경 자체는 native frame을 바꾸지 않는다. 6번의 편집 진입210 정책은 별도이며 이번 브랜치에서 구현하거나 폐기하지 않았다. 9번의 저장 명칭·파일 내보내기 제거·22px 툴바·padding0도 포함하지 않았다.

- 같은 memo-surface 안에서 toolbar → 본문 flex → Feedback footer 순으로 배치한다. 긴 안내와 좁은 창의 줄바꿈 버튼은 높이 상한 내에서 스크롤한다. 50% 상한은 구현값이다.
- 별도 native 창 생성, 100ms snapshot/내용 측정, 상하 부착·그림자·폭/위치·포커스·투명도 동기화를 제거했다. 전체 사각 외곽의 기존 resize 영역·native 입력 경로는 유지했다.
- 버튼은 DOM identity를 유지하며 상태만 갱신한다. 기존 backend의 발신 메모 검증·허용 action·revision 검사와 외부 웹 IPC 거부를 유지했다. present_feedback은 표시 상태의 inspect/action 검증용 기록만 수행한다.
- CLI inspect.feedback_surface는 null 또는 mode:inline/visible/focused/view를 반환한다. 더 이상 별도 창의 position/size/window_id를 제공하지 않는다. 기존 필드를 사용하는 외부 검증 도구는 이에 맞춰야 한다.
- [디자인 시스템](../design-system.md), [Feedback SVG](../design/feedback-drag.svg)·[PNG](../design/feedback-drag.png), [외곽선 SVG](../design/memo-focus.svg)·[PNG](../design/memo-focus.png)를 갱신하고 Chromium으로 PNG를 렌더링해 확인했다. 시안의 저장 명칭은 이 브랜치 기준이며 9번과 통합 시 갱신이 필요하다.

## 출처

| 항목 | 값 |
|---|---|
| 작업 root | /Users/egoinglee/.codex/worktrees/6eec/agent-gibbon |
| 브랜치 | codex/feedback-resize-research |
| 기준 main | 53ab3a710fb54f4702de2499fdc22b9dd8506087 |
| HEAD | eb404cfd3acfb8cb59ebd13e18ab8b5b9554b28b (계획 커밋; 구현 미커밋) |
| 개발 인스턴스 | dev-8d425074f682041f2185812f |
| binary | /Users/egoinglee/.codex/worktrees/6eec/agent-gibbon/src-tauri/target/debug/agent-gibbon |
| 실행한 PID | 24384, 위 binary의 --serve 경로와 대조 |
| 검증 note | inline7-verify, 고유 본문 표식 GIBBON7-INLINE-VERIFY |

## 실행 결과

- npm run test:ui: 197/197 통과. 내부 Feedback 렌더링·버튼 DOM 유지·최신 revision 전달·접힘 억제/복원, 기존 저장·충돌·초안·입력 회귀 포함.
- cargo test --manifest-path src-tauri/Cargo.toml: 220/220 통과. 초기 실행에서 보조창 opacity 제외 테스트가 실패하여 feedback-* 예약 label 제외를 유지하고 전체를 다시 통과했다.
- cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings, cargo fmt --manifest-path src-tauri/Cargo.toml -- --check, npm run dev:build: macOS ARM64에서 통과.
- scripts/inline-feedback-smoke.mjs: 실제 UI 파일 + Tauri IPC 모형을 Chromium 158.0.8127.18에서 실행했다. 144×210, 300×210, 600×420, 300×100에서 footer와 편집란 비겹침·가로 넘침 없음·줄바꿈 버튼 스크롤 접근·긴 오류·초안 유지·클릭 저장을 확인했다. 300×210에서는 Ctrl+S 경로도 검사한다. 접힘 복원과 디자인 전환 중 편집 DOM·초안·선택·포커스 보존도 확인한다. native frame 불변을 증명하는 시험이 아니라 browser viewport 기준 레이아웃 시험이다.
- 측정된 편집란 높이는 위 네 크기 순서대로 116/144/354/34 CSS px, 기본 footer 높이는 56/28/28/28 CSS px였다.
- CLI: ./scripts/agent-gibbon-dev open --id inline7-verify --title GIBBON7-INLINE-VERIFY --markdown <검증본문> 후 inspect. ok:true, instance 일치, note id/고유 본문, feedback_surface.mode:inline 확인. 그 시점 피드백 내용은 비어 있었으므로 visible:false였다. 실제 버튼/안내 표시를 native에서 확인한 결과는 아니다.
- CUA가 debug binary 절대 경로를 Invalid app으로 거부했고 앱 목록에도 해당 unbundled 프로세스가 없었다. 다른 설치 앱으로 대체하거나 GUI 클릭을 시도하지 않았다. 창 제목/PID를 연결한 native 화면 캡처도 확보하지 못했다.
- 검증 후 해당 CLI로 검증 note를 close하여 숨겼다. 실행 전 해당 경로 프로세스가 없음을 확인했고, 직접 시작한 PID24384의 전체 명령줄을 재확인한 뒤 SIGTERM을 보냈다. 다른 앱은 종료하지 않았다.

로그·브라우저 캡처는 /tmp/gibbon7-*에 남겼다. 주요 파일: /tmp/gibbon7-ui-final.log, /tmp/gibbon7-rust-final.log, /tmp/gibbon7-clippy-final.log, /tmp/gibbon7-build-final.log, /tmp/gibbon7-browser-final.json, /tmp/gibbon7-before.json, /tmp/gibbon7-close.json 및 /tmp/gibbon7-inline-browser/inline-*.png. 임시 폴더 자료는 영구 보관을 보장하지 않는다.

## 한계와 통합 시 주의

- 실제 macOS Feedback 버튼 클릭·native 가장자리 drag·비활성 첫 클릭·물리 초안/IME·복수 모니터·혼합 DPI·터미널·이미지 실환경, Intel 및 Windows는 미검증이다. 전체 플랫폼 CI·릴리스·설치 앱 교체는 수행하지 않았다.
- 높이가 극단적으로 작은 창에서는 toolbar가 남은 공간 대부분을 차지한다. 모든 높이에서 충분한 편집 영역을 제공한다는 보장은 하지 않는다. Feedback을 나타내기 위해 창을 자동 확대하지 않으며 사용자가 resize하거나 별도 편집 진입 높이 정책을 적용한다.
- 구형 companion 창 전용 스모크 진입점은 내부 footer 스모크로 전환하고 appearance 스모크도 내부 footer를 가진 메모 페이지를 대상으로 바꿨다. appearance 스모크 전체 matrix는 이번에 실행하지 않았다.
- 사용자가 2026-11-14 이 작업 채팅에서 `클로즈 스킬`을 직접 호출하여 현재 범위의 최종 커밋·push·병합·main 동기화를 승인했다. 구현 커밋은 `45838a6`, 최신 main의 캡처 권한 안내와 충돌을 해결한 통합 커밋은 `360237e4d030e3fdb5f57f3af0fd423142612d8b`다. PR #161에 최종 구현을 반영한다.

Conf 4: 단위 테스트·브라우저 실자산 레이아웃 회귀 통과, native CLI 출처 확인. native GUI 완료를 뜻하지 않는다. Risk Medium: 창 구조·상태 표시 및 inspect 응답 구조를 변경한다.

## close 통합 검증 — 2026-11-14

- origin/main `64915018a7fbabab00d895b285538f1f73520750`을 통합했다. main.rs의 창 이벤트 대상 조건 한 곳이 충돌했으며, 새 권한 안내 창을 제외하는 `memo_id_from_window(...).is_none()`를 유지하고 폐기된 companion 조건만 제거했다.
- 통합 제품 SHA `360237e4d030e3fdb5f57f3af0fd423142612d8b`에서 UI197, Rust221, Clippy, Rustfmt, macOS ARM64 개발 빌드와 Chromium 네 크기 회귀를 다시 수행했다. 이전 제품 커밋의 검사 결과로 대체하지 않았다. 이 절 이후 변경은 검증 기록뿐이다.
- close 로그: /tmp/gibbon7-close-ui.log, /tmp/gibbon7-close-rust.log, /tmp/gibbon7-close-clippy.log, /tmp/gibbon7-close-build.log, /tmp/gibbon7-close-browser.json. 실제 native GUI 및 타 플랫폼의 미검증 한계는 유지한다.
- 이번 워크트리와 기본 main 워크트리의 기존 package-lock.json 변경은 커밋에서 제외하고 보존한다. 병합 SHA와 로컬·원격 main 일치, 이슈 종료는 GitHub 이슈 #160의 최종 기록을 따른다.
