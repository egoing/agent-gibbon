# #188 컬러 팔레트·공통 메모 색상 검증

## 출처와 범위

- root: `/Users/egoinglee/.codex/worktrees/92bf/agent-gibbon`
- branch: `codex/color-palette-memo-types`
- 기준 main: `2b0cfb8e34e80cc0447a51892c72ef3ce9acccce`
- 계획 커밋: `2eecf36` · Draft PR [#189](https://github.com/egoing/agent-gibbon/pull/189) · 이슈 [#188](https://github.com/egoing/agent-gibbon/issues/188)
- 아래 결과는 이 워킹트리의 제품 변경에 대한 검사이며 계획 커밋만의 검사 결과를 뜻하지 않는다. 2026-11-15 사용자가 native 표시색·종료 원인 미확인 한계를 고지받고 제품 커밋·push·병합을 승인했다.
- Node 24.21.0 / npm 11.19.0 / pnpm 11.25.0 / Rust 1.91.1 / macOS ARM64. `pnpm install --frozen-lockfile`로 전용 node_modules를 준비했다. 새 의존성·lock 변경은 없다.

## 참고 이미지 샘플

세 참고 PNG를 직접 열어 확인했다. 좌표는 각 이미지의 원래 픽셀 기준이며 영역의 RGB 채널별 중앙값이다. 앱 런타임에서 샘플링하지 않는다.

| 참고 파일 식별자 | 영역 (left, top, right, bottom) | 샘플 |
|---|---|---|
| c9843190-f998-4735-9bb4-e770fc9de35a | 300,100,410,140 | 라임 `#bdd657` |
| c9843190-f998-4735-9bb4-e770fc9de35a | 280,280,370,320 | 차콜 `#4d4d4b` |
| d86dfd00-2166-414a-b581-c1ee41b0c588 | 620,60,900,110 | 노랑 `#dec755` — 제외 |
| d86dfd00-2166-414a-b581-c1ee41b0c588 | 140,360,360,400 | 코랄 `#db704f` |
| d86dfd00-2166-414a-b581-c1ee41b0c588 | 140,70,300,110 | 회녹 `#c1c3b0` — 제외 |
| 9e06c614-db1f-4fe4-a531-275688b7c067 | 220,210,340,240 | 민트 `#6baa92` |
| 9e06c614-db1f-4fe4-a531-275688b7c067 | 220,340,340,370 | 아이보리 `#e5e4d2` |
| 9e06c614-db1f-4fe4-a531-275688b7c067 | 220,500,340,530 | 세이지 `#697063` — 제외 |

사진·작은 화면에서 서로 가까운 녹색/중성색을 여러 개 넣는 대신 색상과 밝기가 구분되는 5색을 사용한다. 라임/노랑의 ΔOK 5.72보다 최종 최소값 15.51이 크다. `node scripts/color-palette-report.mjs`가 모든 쌍·역할색·대비를 [metrics.json](color-palette/metrics.json)에 생성하고 이번 선택 기준 최소15를 검사한다. 후보/전경/아이콘/보조색·오류·상태 정책은 [디자인 시스템](../design-system.md#컬러-팔레트와-공통-메모-색상--188)을 따른다.

## 자동 검사

| 검사 | 결과 |
|---|---|
| `npm run test:ui` | 187개 통과. 기존 편집/테마 경로와 새 opaque secondary·border 대비, 프론트/백엔드 후보 일치, trusted chrome 저장색/이벤트/다른 모드 보존 포함 |
| `node scripts/build.mjs cargo test --manifest-path src-tauri/Cargo.toml` | 232개 통과. 신규/명시색·업데이트 보존, legacy 기본색·원자적 색 저장/재생성 보존·ID/HEX 검증 포함 |
| `node scripts/build.mjs cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 통과 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 통과 |
| `npm run dev:build` | 이 root의 debug 바이너리 빌드 통과 |
| `git diff --check` | 통과 |

최초 UI 실행은 node_modules 미설치로 실패했다. 설치 후 실행에서 DOM mock의 dataset 누락으로 5개가 실패했고, optional dataset 접근으로 고친 뒤 전체 187개가 통과했다. 실패를 통과로 합산하지 않는다.

## 실제 프론트 렌더

`PLAYWRIGHT_MODULE=/tmp/gibbon-playwright/node_modules/playwright/index.mjs node scripts/color-palette-smoke.mjs`

- Chromium의 실제 `index.html`/Milkdown·캡처 표시·xterm·웹 HTML/CSS를 사용하고 Tauri IPC만 mock한다. 물리 포인터·전역키·사용자 Chrome 탭은 사용하지 않는다.
- 4유형 × 5색 × 3크기(240×160 / 320×300 / 600×400) × 4상태(normal/hover/focus/disabled) = **240건 통과**. 실제 계산된 본문·버튼·보조/상태·오류 Feedback의 RGB와 배경 대비 4.5 이상, opacity1 및 가로 넘침 없음을 확인했다. 유형별 forced-colors 초점의 시스템 Highlight 복귀도 확인했다. 에러는 실제 실패 응답의 UI 경로로 표시한다. disabled는 DOM 상태를 지정해 계산된 스타일을 검사한다.
- PTY 테마 `#20252e`/`#f0f0f0` 보존을 검사했다. 캡처 fixture 원색 `#35688a`와 원본 비율이 유지되며 프레임/letterbox만 메모색이다. 실제 외부 페이지·ANSI 프로그램은 native에서 실행하지 않았다.
- [브라우저 결과 JSON](color-palette/browser-results.json) · [텍스트 코랄](color-palette/text-1.png) · [터미널 민트](color-palette/terminal-2.png) · [웹 차콜](color-palette/web-4.png) · [캡처 코랄](color-palette/image-1.png). 20개 이미지와 팔레트·관리/터미널 SVG→PNG를 생성하고 대표 렌더를 육안 확인했다.
- 별도 `scripts/terminal-theme-smoke.mjs`의 기존 14개 테마/크기 회귀도 실행했다. 설정 storage·OS 변경·DOM/선택 보존·키보드 초점·실행과 xterm 고정 팔레트를 확인하는 headless 검사다.
- 설명 SVG·스크린샷과 native 화면 검증은 구분한다. 실제 native WebView에서 이 글자색이 보였다고 주장하지 않는다.

## 개발 CLI 및 native 한계

`npm run dev:info` 출처:

- instance: `dev-cee63ced4725619c6df21545`
- binary: `/Users/egoinglee/.codex/worktrees/92bf/agent-gibbon/src-tauri/target/debug/agent-gibbon`
- 첫 PID `72893`, 최신 빌드 실행 PID `92494`. `ps`의 실행 파일 절대 경로와 각 inspect의 `ok:true`/instance/메모 ID·본문·저장색을 대조했다.
- 사용자의 “예제 메모창을 켜줘” 요청으로 `palette19-{lime,yellow,coral,mint,ivory,charcoal}` 6개를 생성했다. 노랑은 비교용 제외색이며 자동 후보에는 없다. 6개 모두 처음 `visible:true`와 명시색을 확인했고 숨김/종료하지 않았다. 사용자 이동 후 저장된 좌표가 복원된 것은 [native-restored.json](color-palette/native-restored.json)에 기록했다. 다만 재시작 직전 snapshot은 확보하지 못해 직전 기하·숨김과의 완전 일치 검증으로 판정하지 않는다.
- 최신 빌드에서 `web --id web-palette19` 생성의 자동색이 후보에 속했고, `update --color '#123456'` → inspect → close/다시 열기 뒤 명시색 유지 및 instance 일치를 확인했다. [native-web.json](color-palette/native-web.json). 웹의 현재 외부 URL은 null이며 외부 페이지를 열지 않았다. 추가 `palette19-auto`도 자동 후보 `#4d4d4b`를 반환했다.
- 진단 응답은 `terms_accepted:false`, `terms_skipped:true`, `onboarding_required:false`, `sharing_allowed:false`였다. 약관 수락/진단 전송 동의는 생성하지 않았다.
- **도구 한계:** CUA가 미번들 debug 실행 파일을 앱으로 선택하지 못했다. 따라서 native 화면 캡처·실제 표시색·설정의 컬러 선택·터미널 새 창의 native 실행·포커스 실기기 확인은 미검증이다. 다른 앱으로 대체하거나 AppleScript/전역키 조작을 하지 않았다.
- **종료 관찰:** 두 실행 모두 에이전트가 kill/앱 종료 명령을 보내기 전에 종료됐다. PID92494의 프로세스 종료 결과는 exit0이고 새 crash report는 없었다. 사용자 직접 종료 여부를 질문했으며 답변 전 반복 재실행을 멈췄다. 이 결과로 종료 원인·기하/숨김 보존·재시작 안정성을 확정하지 않는다. 추가 close 요청은 앱 미실행 오류였으므로 숨김 성공으로 기록하지 않는다.
- 다른 워크트리·설치 앱의 종료·이동·재시작은 수행하지 않았다. 전체 플랫폼 CI·Intel/Windows·release/설치·배포는 범위 밖으로 미실행이다.

## 판정

구현·로컬 자동 검사·실제 Chromium 렌더를 완료했다. **Conf4 / Risk Medium**: 색 저장·타입별 전달과 CLI 색상 변경을 추가하므로 상태/API 범위의 복구 가능한 위험이 있다. native 실제 표시와 종료 원인 등 위 한계를 남긴다. 2026-11-15 사용자 승인에 따라 제품 커밋·push·병합을 진행한다. 승인 범위에 앱 추가 실행·공개 배포는 포함하지 않으며 native 미확인 한계는 유지한다.
