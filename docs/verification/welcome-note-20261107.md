# 최초 메모 사용 안내 검증 — 2026-11-07

## 변경 범위

- 기존 대기 문구를 일반 Markdown 샘플로 교체했다. 캡처·메모·터미널·브라우저를 다른 창 위에 띄워 참조하는 목적, 메뉴/영역 선택으로 시작하는 방법, 이동·크기·편집과 숨긴 창 복원을 안내한다. 사용자 피드백에 따라 개발자용 스킬 설명을 제거했다.
- 일반 메모의 렌더러와 스타일을 재사용한다. 기존 저장 메모 마이그레이션이나 강제 교체는 추가하지 않는다. 정적 WebView의 이전 대기 문구는 제거했다.
- [디자인 기준](../design-system.md#최초-메모-사용-안내--2026-11-07)과 [본문 시안](../design/welcome-note.png)을 함께 갱신했다. SVG를 `rsvg-convert`로 PNG로 렌더링해 글자와 잘림 여부를 확인했다. 툴바를 생략한 설명용 시안이다.

## 실행 환경과 출처

- 작업 루트: `/Users/egoinglee/.codex/worktrees/1297/agent-gibbon`
- `npm run dev:info` root는 작업 루트와 일치, id는 `dev-f0828dabec18050d84018f5f`.
- binary: `/Users/egoinglee/.codex/worktrees/1297/agent-gibbon/src-tauri/target/debug/agent-gibbon`
- macOS ARM64, Node v24.21.0, Cargo 1.91.1.
- `npm run dev:build`로 빌드 후 `./scripts/agent-gibbon-dev open`으로 실행. 해당 절대 바이너리 경로의 PID `78032`를 확인했다.
- 기본 메모 `main`을 개발 CLI로 `inspect`하여 `ok: true`, 응답 instance 일치, 본문과 `src-tauri/src/welcome.md`의 일치를 확인했다. 다른 인스턴스의 실행 증거는 사용하지 않았다.

## 검증 결과

- `npm run dev:build`: 통과.
- `npm test`: Rust 206개, UI 169개 통과.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`: 통과.
- `npm run check` (Clippy, 전체 타깃·경고 오류 처리): 통과.
- `git diff --check`: 통과.
- 개발 CLI `inspect --id main`: 일반 HTML과 변경 표시 HTML 동일, 최초 변경 강조와 Feedback 없음, 콘텐츠 높이 상한 273 논리 px 확인. 실제 GUI의 시각적 판정은 별도 미검증이다.
- 검증 전용 `welcome-preserve-1297`에 사용자 본문을 전달한 뒤 close/open하여 본문 보존 확인. 이어 명시적 빈 문자열로 갱신하고 다시 open하여 빈 본문 보존 확인. 검증 후 이 메모만 숨겼다.
- `main`은 새 안내를 살펴볼 수 있게 유지했다. 작업 진행 메모는 `welcome-note-progress-1297`을 사용한다. 설치 앱과 다른 워크트리의 메모·프로세스는 변경하지 않았다.

## 한계

GUI 버튼 조작·실제 첫 약관 수락 흐름·저장 메모의 프로세스 재시작 복원·Windows/Intel 실제 실행은 이번 변경에서 검증하지 않았다. 기존 저장/편집/렌더링 회귀 테스트는 통과했다. 문구와 기본 본문 변경으로 OS별 코드·입력·창 제어·의존성·빌드 설정은 변경하지 않았으며 전체 플랫폼 CI는 요청하지 않았다. 커밋·push·배포는 하지 않았다.

## 사용자 피드백 반영 후 재검증

- 변경은 `welcome.md`의 문구와 연결된 디자인 문서·SVG/PNG에 한정한다. 위 Rust/UI·정적 검사 결과는 최초 구현 단계의 결과이며 문구 보완 후 전체 테스트를 다시 실행하지 않았다.
- `npm run dev:build` 재통과. 이전 검증에서 시작한 PID 78032의 실행 파일 절대 경로를 다시 대조한 뒤 해당 프로세스만 종료하고 개발 CLI로 재시작했다.
- 재시작 후 동일 절대 바이너리 경로의 PID는 `89579`다.
- 새 기본 메모 `welcome-preview-1297-v2`를 `open`/`inspect`하여 `ok: true`, 예상 instance와 일치, 최신 원문과 본문 일치, 스킬 언급 없음, 일반 HTML과 변경 HTML 동일·최초 Feedback 없음 확인.
- 기존 저장된 `main`은 이전 샘플을 포함해 덮어쓰지 않았다. 최신 기본 문구는 새 검증 메모에서 확인했다. 새 검증 메모는 사용자가 확인할 수 있도록 표시 상태로 남긴다. 재시작으로 숨김 복원된 진행 메모를 자동으로 다시 열지 않는다.

## 마무리 검증 — 2026-11-08

최종 문구 상태에서 `npm test`(Rust 206개·UI 169개), `npm run check`, Rustfmt, `git diff --check`를 다시 실행해 통과했다. 이 시점 개발 앱은 실행 중이 아니므로 진행 메모를 다시 열거나 다른 인스턴스로 대체하지 않았다. 위 2026-11-07 실제 CLI 검증은 당시의 실행 기록이며 이번 마무리에서 재실행한 것으로 간주하지 않는다.
