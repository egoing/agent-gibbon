# 선택 서식 메뉴의 한국어 명칭 — #183

2026-11-16, 작업 25의 병합 검증. 루트는 `/Users/egoinglee/.codex/worktrees/49d3/agent-gibbon`, 브랜치는 `codex/183-format-labels`다. 기준 main `0fee5c354bb5744534a5a41280df17e7d8b12616`의 빈 메모 입력 수정(#202)과 웹 창 종료 수정(#119)을 유지했다.

코드·큰제목·작은제목의 화면 문구, 도구 설명과 접근성 이름을 일치시켰다. 실제 동작은 inline code, heading level 1·2 그대로다. 버튼은 최소 너비 24px와 높이 22px를 유지하며 글자 길이에 맞게 늘어난다. 좁은 창에서는 메뉴 내부 스크롤로 접근한다. 디자인 시스템과 설명 SVG·PNG도 같은 명칭을 표시한다.

## 수행한 검증

- macOS ARM64, Node 26.0.0, pnpm 11.25.0, Cargo 1.91.1. 플랫폼은 직접 조회했다.
- `pnpm test:ui`: 200개 통과, 실패·생략 0.
- `PLAYWRIGHT_MODULE=/tmp/gibbon-playwright/node_modules/playwright/index.mjs node scripts/milkdown-browser.mjs`: Chromium 158.0.8127.18의 전체 9묶음 통과. 네이티브 IPC는 대역이며 실제 앱을 실행하지 않았다.
- 큰제목/작은제목의 Markdown 수준, 인라인 코드, 선택 유지·undo/redo·명시저장, 144px 메뉴의 키보드 진입·이동·스크롤·본문 복귀를 확인했다. 300px 및 144px 실제 브라우저 캡처와 설명 PNG를 시각 확인했다.
- #202의 빈 영역 클릭→첫 입력→저장→저장 문서 다시 읽기 검사와 인쇄 가능한 키 입력을 보존했다. 기존 자동저장·원문 보호·66k 문서 검사가 통과했고 브라우저 오류 및 외부 요청은 0이었다.
- 기준 main 대비 Rust 변경은 없다. 이번에는 Rust 전체 테스트와 Intel/Windows·실제 macOS OS 키입력·물리 한글 IME 검사를 실행하지 않았다.

로그와 캡처는 `/Users/egoinglee/Documents/Codex/2026-11-16/issue183-merge-side/final-main0fee/`에 보존했다. 공유 Cargo 캐시를 변경하지 않고 개인 APFS 복제 캐시로 개발 빌드를 수행하며, 실행 파일의 instance·manifest_dir를 확인한다. 해당 결과는 같은 위치의 `build-report.json`을 따른다.

Conf 4 / Risk Low. 앱 재시작·설치·공개 배포는 이번 작업에서 실행하지 않는다.

[디자인 시스템](../design-system.md) · [설명 SVG](../design/milkdown-editor.svg) · [PNG](../design/milkdown-editor.png)

## 개발 빌드 결과

`pnpm dev:build`는 2026-11-16T10:48:21+09:00에 종료 코드 0으로 완료됐다. 바이너리 `--instance-info`에서 인스턴스 `dev-d7e98ec7ea551c0e723710fe`와 이 워크트리의 `manifest_dir`가 일치했다. 실행 파일 SHA-256은 `b85172627503e852a3339a0a8982bdcda571bdc066a726e2a7275c9df3bd0e48`이며 실행 권한을 확인했다. 개인 중간 캐시만 정리하고 해당 워크트리의 실행 파일을 보존했다. 실제 앱은 시작하지 않았다.
