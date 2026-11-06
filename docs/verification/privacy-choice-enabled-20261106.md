# 진단 동의 선택 활성화 — 2026-11-06

## 반영 내용

사용자 승인에 따라 원본 최대 30일, 백업 생성 후 30/90일, 오류 이슈 누적 통계의 별도 보유를 개인정보 안내 2026-11-06.3에 반영했다. 세션 통계는 보유기간 확인 전 수집하지 않는다. `notice.json`의 원본 보유기간을 30일로 확정해 실제 선택을 허용했다. 선택은 기본 해제이며 기존 동의 기록은 정책 해시 변경으로 승계하지 않는다.

Release Health SDK 자동 세션을 끄고 envelope 재구성에서 SessionUpdate 및 허용하지 않은 집계 항목을 차단한다. SDK·런타임 상태의 release_health는 false다. 동의가 없는 상태에서는 SDK 초기화·전송 큐 허용을 하지 않는 기존 경계를 유지한다.

## 자동 검증

- macOS ARM64, Node v26.0.0, Rust/Cargo 1.91.1.
- `cargo test --manifest-path src-tauri/Cargo.toml -- --test-threads=1`: 123개 통과. 세션 envelope 차단, 미동의/철회/이전 세대 큐 차단 및 철회 전 시작된 HTTP 요청 대기 테스트 포함. HTTP 검증은 loopback 테스트이며 실제 Sentry 전송이 아니다.
- `npm run test:ui`: 81개 통과. 선택 가능·기본 해제·켜기 저장 전 무전송 선택·체크 해제 철회 검사 포함.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`: 통과.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`: 통과.
- `npm run diagnostics:build -- --official`: 현재 워크트리의 macOS ARM64 개발 빌드 성공. 실행기가 DSN을 빌드 프로세스 메모리로 전달하며 관리 토큰은 자식 환경에서 제거한다. 사용자 동의를 저장하지 않는다.
- Playwright Chromium에서 실제 manager HTML/CSS/privacy.js/manager.js와 모의 Tauri 경계로 첫 실행 기본 해제, 고지 열람, 약관만 수락, Settings 켜기 후 명시 저장, 끄기 즉시 철회, 600px/944px 가로 넘침 없음을 확인했다. 외부 요청 0개·페이지 오류 0개. 이 조작은 모의 브라우저 상태이며 실제 사용자의 동의를 대신 저장하지 않았다.
- 실제 HTML/CSS 브라우저 캡처: `/tmp/gibbon-consent-enabled-first-run.png`, `/tmp/gibbon-consent-enabled-settings.png`. `docs/design/privacy-onboarding.svg`를 PNG로 렌더해 시각 확인했다.

## 실제 개발 앱 실행·출처

- 작업 루트: `/Users/egoinglee/dev/project/agent-gibbon`.
- `node scripts/dev.mjs info`의 root는 위 루트, instance는 `dev-a5210ae16e79437d38490c8f`.
- 실행 파일: `/Users/egoinglee/dev/project/agent-gibbon/src-tauri/target/debug/agent-gibbon`.
- 실행 명령: `npm run diagnostics:open`. 초기 open 응답은 현재 정책 약관을 아직 수락하지 않았으므로 `onboarding_required:true`, 예상 instance를 반환했다. 앱 시작 실패로 해석하지 않는다.
- 새 서버 PID 59219, 인자 `--serve`. 실행 전 같은 절대 경로의 프로세스는 없었으며 다른 앱/워크트리를 종료하지 않았다.
- 개발 CLI `node scripts/diagnostics.mjs status`: `ok:true`, 위 instance 일치, `privacy.notice_ready:true`, `terms_accepted:false`, `sharing_allowed:false`, SDK `enabled:false`, `detailed:false`, `release_health:false`, journal 비활성, frontend_errors 0 확인.
- 빌드 revision 표식은 `02cefc78e203`이다. 이번 미커밋 변경은 표식에 포함되지 않으므로 표식만을 소스 일치 근거로 삼지 않는다. 이번 빌드 성공 로그와 새 프로세스·새 고지 버전·notice_ready 실제 동작을 함께 확인했다.
- Orca 네이티브 창 목록: PID 59219의 window ID 38569, `Agent Gibbon [DEV main · a5210ae1] — 메모 관리`, 944×480.
- `computer-use`의 window-id 기반 화면 캡처는 `window_not_found`로 실패했으므로 성공으로 기록하지 않는다. 같은 PID/window-index 0의 읽기 전용 AX 조회는 위 제목과 native window ID를 반환했다. 실제 고지 2026-11-06.3, 30일 안내, 선택 동의 체크박스(값 0, 비활성 표시 없음)를 확인했다. 사용자가 고지를 열람 중인 상태를 유지했다.

## 한계

- 실제 네이티브 동의 체크·저장·철회는 사용자를 대신 실행하지 않았다. 실제 Sentry 이벤트 신규 수신이나 패킷 캡처 검증도 수행하지 않았다. 대신 수집 경계의 loopback 테스트와 현재 실행 앱의 미동의/SDK 비활성 상태를 확인했다.
- 최초 검증 시점에는 이 변경을 커밋·푸시하거나 iMac/Windows에 배포하지 않았다. Windows 행 원인의 해결을 의미하지 않는다.
- 기존 package-lock 및 다른 작업의 디자인 문서·시안 변경을 보존했다.

확신도: 선택 UI/수집 경계 Conf 4(브라우저 및 loopback 자동 검증), 실제 앱의 선택 가능·미동의 상태 조회는 Conf 5. 위험도 Medium(개인정보 동의 및 외부 전송 제어).

## 후속 실제 저장 검증과 커밋

현재 사용자 요청으로 동의 활성화·세션 차단과 관련 고지/시안을 커밋 대상으로 정리했다. [실제 native 저장·재실행 검증](privacy-persistence-20261106.md)에서 동일한 주요 동의/전송 소스의 스냅샷으로 최초 수락·거부, 설정 켜기 명시 저장·끄기 즉시 저장, 재실행 및 손상 파일 차단을 확인했다. 테스트 약관·운영 정보·loopback DSN과 재실행 관찰용 Manager 창 설정을 사용했으며 실제 사용자의 동의나 공식 Sentry 수신 검증으로 해석하지 않는다.
