# 업데이트 안내 정책과 복구 — 2026-11-20

## 조사 범위와 근거

운영 앱과 GUI 입력은 팀장 담당이다. 이 작업은 설치 파일의 읽기 전용 조사와 독립 워크트리의 코드·모의 IPC 브라우저 검증만 수행한다. 타 앱의 실제 업데이트나 설정 변경을 실행하지 않았다.

| 설치 앱 | Info.plist 버전 | 읽은 실제 자산 | 관찰한 정책 |
|---|---|---|---|
| Orca | 1.4.245 | `Contents/Resources/app.asar`의 `out/main/index.js`, preload | 메뉴 `Check for Updates…`, 확인·다운로드·quitAndInstall 분리, interactive 설치 모드·다운로드 상태·에러 처리, 자동 종료 설치 설정과 macOS staging 보호 |
| HyperFrames | 0.1.0 | 같은 asar의 `main/autoUpdate.mjs`, `updater.mjs`, `updateRestart.mjs`, `updateFailure.mjs`, `updateMenu.mjs`, `updateHandlers.mjs`와 renderer | 자동 업데이트 저장 선택은 기본 false로 읽음. 자동 모드는 전경·busy를 피하고 away 기간을 확인. 재시작을 막는 작업 목록을 기다림. 네트워크·서명·디스크·권한 등의 복구 문구와 로그 폴더·다운로드 동작 분리 |

Orca asar SHA-256: `ca0eb96e1ed0d38bf8018b24ccad1c36776f071e4186d7c6b484d65ecf0ce03c`.
HyperFrames asar SHA-256: `c573f1049d6013be21d55e1e88b82435b9731bfbb8433df2de8df8493118491e`.
버전은 앱 번들의 값이며 HyperFrames의 내부 배포 label·build SHA와 동일하다고 단정하지 않는다. 자산 코드는 로컬 조사용으로만 사용하고 저장소에 복제하지 않는다.

## 반영과 판단

짜잔의 현재 정책을 유지하면서 사용자가 선택할 행동과 결과를 구분했다. 자동 다운로드·5분 무입력 설치, 명시 재시작이 생략하는 대기, 작업 보호, 보류·오류 복구를 기존 설정 안에 설명한다. 작은 준비 안내의 이전 합의는 유지하고 ‘나중에’만 ‘안내 닫기’로 명확히 한다. 설치·다운로드 로직과 안전 검사, 재시도 간격은 변경하지 않는다. 다른 앱의 토글·백그라운드 정책을 그대로 도입하지 않는다.

`ui-ux-pro-max`의 `update deferred error recovery` 검색에서 오류 복구는 다음 행동을 제공하고 오류를 alert로 알리라는 피드백 기준을 확인했다. 기존 데스크톱 버튼·색·초점선·스크롤을 유지했다. 휴대폰 터치 영역 기준은 이 데스크톱 UI의 신규 요구로 적용하지 않았다.

## 실행 출처와 검증

- 루트: `/Users/egoinglee/.codex/worktrees/updater-policy-clarity/agent-gibbon`
- 개발 인스턴스: `dev-59b93e3b0e65fcf915f63d28`
- 개발 바이너리: 해당 루트 `src-tauri/target/debug/jajan`
- `npm run dev:info`의 root·id·binary, `scripts/jajan-dev inspect --id updater-policy-progress`의 `ok: true`·instance·체크리스트 본문 일치를 확인했다. 운영본 CLI는 사용하지 않았다.
- `pnpm run dev:build`: 성공. 전용 개발 CLI의 메모 생성·inspect 성공. 네이티브 GUI 입력·캡처는 수행하지 않았다.
- `pnpm run test`: Rust 328 통과·기존 1 ignored, UI 250 통과.
- `node --test tests/updater-ui.test.mjs tests/updater-notice.test.mjs`: 13 통과. 중복 설치 클릭·준비/오류 상태·재확인 행동·안내 닫기 의미와 기존 숨김 정책을 포함한다.
- `scripts/updater-compact-smoke.mjs`: 실제 HTML/CSS/JS, 모의 IPC의 Chromium에서 360×140 안내의 밝음/어두움·긴 오류와 버튼 유지, 설정 8상태×3테마×2크기 통과. 캡처를 열어 좁은 검정 테마와 두 시안 PNG의 배치·문구를 확인했다. 실제 업데이트 설치를 검증한 결과가 아니다.
- SVG 두 개를 `rsvg-convert`로 PNG 재생성하고 시각 검토했다. 설명용 시안이다.

복구 도움말을 키보드 Enter로 펼치고 좁은 화면에서 가로 넘침이 없는지 추가 확인했다.

최종 정적 검사·배포 빌드·CI·공개 검증 결과는 릴리스 기록에 남긴다. 실제 설치 후 포커스·메모 보존·실행 버전 확인은 팀장 검증 범위이며 이 기록의 통과에 포함하지 않는다.

로컬 Clippy에서 기존 `workspace.rs`의 `nonminimal_bool` 경고를 발견했다. 동일한 토큰 불일치 판정을 `is_none_or`로 정리했다. 설치 정책 변경은 아니다.

- `pnpm run check`: 0.3.11 기준 Clippy all-targets `-D warnings` 통과. `cargo fmt --check`와 `git diff --check` 통과.
- 위험도 Low: 안내 문구·접힌 도움말·진행 버튼과 동일 동작의 부정식 정리이며 설치 경계는 유지한다. 확신도 4는 모의 IPC 브라우저 UI에 한정하며 실환경 업데이트 성공을 의미하지 않는다.
