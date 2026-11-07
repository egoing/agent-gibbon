# #108 업데이트 UI 축소

## 요구사항과 결정

사용자는 업데이트 UI를 간결하게 정리하고 준비 안내를 오른쪽 하단에 배치하도록 요청했다. 시안 검토 후 서명 검증 완료 표시·최근 입력/5분 대기 안내·설치 전 작업 보호 설명을 제거하도록 지정했다. 기본 안내는 제목·새 버전·두 행동만 표시한다. 사용자가 직접 설치를 시도했지만 진행할 수 없거나 상태 조회가 실패하면 행동 결과를 알린다. 보류 이유의 상시 확인은 설정에서 제공한다. 업데이트 확인·서명 검증·설치 보호·동일 버전 반복 억제의 정책과 저장은 변경하지 않는다.

디자이너 페르소나: 작업 방해를 최소화하는 데스크톱 유틸리티 디자이너. [디자인 시스템](../design-system.md)의 #108 기준과 연결된 SVG/PNG를 따른다.

## 출처와 baseline

- 작업 루트 `/Users/egoinglee/.codex/worktrees/0082/agent-gibbon`
- 브랜치 `codex/108-compact-update-ui`, 이슈 #108, PR #111
- 최초 base `e0b478d76ca08d153d7006733682a73a7b753b36`, `refs/is2/base/108`
- red `50ba47ce645111adc974ef41066afe372cb46ad6`, `refs/is2/red/108`
- macOS ARM64, Node 24.21.0, npm 11.19.0, Rust/Cargo 1.91.1
- 개발 ID `dev-ed6eff5da0a9343ac1fef5d3`, 개발 바이너리 `src-tauri/target/debug/agent-gibbon`（위 루트 기준）
- 진행 메모 `progress-108-update-ui`: 개발 CLI inspect 응답 ok와 instance 일치를 확인했다. 진행 메모는 구현 전 빌드 프로세스 PID 94685에서 표시했으며 새 안내창의 native 실행 증거로 사용하지 않는다.
- 최초 baseline: `npm test`, `cargo check --locked --manifest-path src-tauri/Cargo.toml --all-targets`, `npm run check`, `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`, `npm run dev:build` 모두 exit 0. 원본 `/tmp/gibbon-update-ui/baseline.json`, `baseline-{test,typecheck,lint,fmt,build}.log`. baseline 제외 0.

## TDD와 실행

- 구현 전 단위 red: 기본 설명 숨김과 준비되지 않은 설치 버튼 숨김 실패. 사용자 후속 마킹에 따라 미커밋 신규 테스트의 ‘서명 검증 표시’ 요구를 ‘검증 문구 제거’로 변경한 뒤 red를 확정했다. 기존 테스트 assertion은 제거·완화하지 않았다.
- 구현 전 브라우저 red: 실제 기존 UI가 360×140 viewport를 넘쳐 실패. 첫 호출의 Puppeteer 모듈 경로 오류는 red 증거로 세지 않고 설치된 경로를 확인해 다시 실행했다. 로그 `red-final-ui.log`, `red-final-browser.log`.
- 최초 구현 단위·회귀: Rust 207개, Node 172개 PASS. 별도 신규 UI 테스트 포함. `npm test`와 원본 `/tmp/gibbon-update-ui/final-test.log`.
- 타입 `cargo check --locked --manifest-path src-tauri/Cargo.toml --all-targets`, 린트 `npm run check`, Rustfmt, 개발 빌드 `npm run dev:build` 모두 PASS. 원본 `final.json`, `final-{typecheck,lint,fmt,build}.log`.
- Rust 좌표 테스트: 작업 영역의 오른쪽 하단·실제 외곽 크기·음수 원점·1/1.5/2 배율·창 크기 변경·작은 작업 영역의 위/왼쪽 경계 보존을 확인했다.
- 브라우저 명령: `PUPPETEER_MODULE=/tmp/gibbon-issue77/tools/node_modules/puppeteer-core/lib/puppeteer/puppeteer-core.js node scripts/updater-compact-smoke.mjs`. Chrome 실제 HTML/CSS/JS와 모의 업데이트 IPC를 사용했다. 밝음/어두움 360×140, 긴 오류/버전과 고정 버튼 접근, 설정 944×480/600×360 × 흰색/회색/검정 × 8상태를 확인했다. JavaScript 오류·가로 넘침 없음. 출력 `/tmp/gibbon-update-ui/browser/notice-{light,dark}.png`, `settings-{944,600}-{white,gray,black}.png`.
- 두 SVG를 `rsvg-convert`로 PNG 렌더하고 시안을 시각 확인했다. 브라우저 캡처는 실제 자산의 모의 IPC 화면이며 네이티브 제목줄은 포함하지 않는다. SVG/PNG는 설명용 시안이다.

## 범위와 남은 검증

사전·최종 위험도 Low: UI 구조·스타일·안내창 기하만 국소 변경하며 설치 정책·데이터·공개 API 변경이 없다. Conf 4: 브라우저 실행 및 Rust 좌표 단위 검증. 정식 앱의 실제 다운로드→안내 표시·OS 포커스·설치 재시작, Windows/Intel 실앱 및 다중 모니터 물리 이동은 이번 실행에서 검증하지 않았다. 정식 업데이트 실증 #102를 이 변경의 증거로 재사용하지 않는다.

최신 main `959143f4d3cab60e0cb761ef708017bbcb390b8b`에 rebase 후 Rust 208개·Node 172개, 타입·Clippy·Rustfmt·개발 빌드와 같은 브라우저 검사를 다시 통과했다. 원본 `/tmp/gibbon-update-ui/rebased.json`, `rebased-{test,typecheck,lint,fmt,build}.log`, `rebased-browser.log`. upstream #54의 변경은 이번 PR diff에 포함하지 않으며 최초 base/red/attempts ref를 보존했다.

전체 플랫폼 Release workflow_dispatch는 최종 작업 커밋에서 요청한다. 독립 감사와 SHA별 CI 결과는 PR에 기록한다. 공개 릴리스·배포 태그는 생성하지 않는다.
