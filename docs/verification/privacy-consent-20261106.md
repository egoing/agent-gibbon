# 첫 실행 약관·선택적 진단 동의 검증 — 2026-11-06

## 구현과 운영 상태

- 필수 약관과 선택적 서비스 개선 정보 전송은 분리되며 최초 선택은 모두 해제다. 전송 거부는 앱 기능을 제한하지 않는다.
- 현재 운영자·문의처·실제 저장 지역·보유기간은 미정이다. `src-tauri/src/privacy/notice.json`의 readiness가 false이므로 선택적 전송을 활성화할 수 없다. 문서/운영 정보가 바뀌면 정책 해시가 바뀌어 기존 동의는 승계되지 않는다.
- SDK 초기화 전과 실제 HTTP 요청 직전에 전송 허용을 확인한다. 철회는 새 요청을 차단하고, 이전 동의 세대의 SDK·로그·세션 큐는 재동의 시에도 전송하지 않는다. 이미 시작한 요청은 최대 3초의 요청 제한 안에서 끝날 수 있다. 서버에 이미 업로드된 자료는 별도 처리다.
- [약관](../legal/terms.md)·[개인정보 안내](../legal/privacy.md)는 운영 정보가 미정인 초안이다. 정식 법률 검토 또는 국제 배포 요건 충족을 의미하지 않는다.

## 실행 출처

- 작업 루트: `/Users/egoinglee/dev/project/agent-gibbon`
- `npm run dev:info` root는 위와 같고 ID는 `dev-a5210ae16e79437d38490c8f`다.
- binary: `/Users/egoinglee/dev/project/agent-gibbon/src-tauri/target/debug/agent-gibbon`
- 최신 소스 개발 빌드 후 실행한 PID: **48781**. `ps`의 실행 파일 절대 경로와 위 binary를 대조했다.
- 검증 시 기준 HEAD: `80b49b7282a0d0f15212f4139d29c8f13543b909` + 이 작업의 미커밋 변경. 앱 revision은 HEAD 값이므로 변경 포함 여부는 실제 재빌드 로그와 함께 확인했다. 이후 커밋의 빌드 증거로 임의 재사용하지 않는다.
- 검증 메모 ID: `privacy-no-consent-20261106`, 고유 본문: `CONSENT-GATE-LOCAL-ONLY`. 현재 루트의 개발 CLI open/close 응답에서 `ok:true`, 예상 instance, ID·본문을 대조했고 검증 후 숨겼다. 기존 사용자 메모는 식별용으로 바꾸지 않았다.
- 네이티브 GUI를 클릭·캡처해 검증한 기록은 없다. 아래 와이어프레임 PNG는 실제 앱 캡처가 아니다.

## 자동 검사

| 명령 | 결과 |
| --- | --- |
| `cargo test --manifest-path src-tauri/Cargo.toml` | Rust 122개 통과 |
| `npm run test:ui` | UI 80개 통과 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | 통과 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 통과 |
| `npm run dev:build` | macOS ARM64 debug 빌드 통과 |
| `git diff --check` | 통과 |

동의 누락/손상/이전 정책, 거부 기록의 원자적 저장과 재읽기, 미완성 운영 정보의 차단, 최초 선택 기본값·약관 요구, 즉시 철회, 꺼진 프런트 측정기와 과거 비동기 결과 배제, 오류·trace·log·session의 필터를 검사했다.

전송 통합 테스트는 `127.0.0.1` HTTP 수신기만 사용한다. 동의 없음 0건 → 허용 후 1건 → 철회 후 1건 유지 → 재허용해도 과거 client는 1건 유지 → 새 client만 2건을 확인한다. 테스트 payload는 고정된 합성 이벤트이며 실제 Sentry 프로젝트로 보내지 않는다.

## 실제 앱의 전송 거부 상태

빌드 시 가짜 loopback DSN을 주입하고 실행 시 `AGENT_GIBBON_DIAGNOSTICS=1`을 지정했다. 인증 토큰은 빌드·실행 자식 프로세스 환경에서 제거했고 실제 공식 DSN을 조회하거나 사용하지 않았다. 수신기는 127.0.0.1의 일회성 테스트 fixture이며 앱에 서버 기능을 추가하지 않았다.

- 이전 검사에서는 약관 미수락/전송 미동의 상태에서 open이 `onboarding_required:true`로 거절되고 진단 기능이 꺼져 있었다. `/tmp/gibbon-consent-runtime-final.json`, `/tmp/gibbon-consent-open-final.json`에 당시 응답이 있다.
- 최신 실행 시에는 로컬 약관 수락 기록이 존재했고 선택적 전송은 **false**였다. 이 검증이 사용자를 대신해 약관/전송 동의를 저장한 것은 아니며 기존 선택을 보존했다.
- 최신 CLI `diagnostics` 응답: `ok:true`, instance 일치, `terms_accepted:true`, `sharing_allowed:false`, `notice_ready:false`.
- SDK·detailed·logs·release_health·journal은 모두 false. 작업/heartbeat/frontend error 진단 카운터는 모두 0.
- `diagnostics-probe`는 `Sentry diagnostics are disabled`, `ok:false`로 거절됐다.
- loopback 수신기 관찰: **연결 0건, 수신 0바이트**. 수신기 시작 09:27:29 → 앱 open 09:28:09 → 수신기 종료 09:28:35 (KST)로 앱 재시작을 실제 관찰 구간에 포함했음을 파일 timestamp로 확인했다.
- 실행 증거: `/tmp/gibbon-consent-build-current.log`, `/tmp/gibbon-consent-open-current.json`, `/tmp/gibbon-consent-runtime-current.json`, `/tmp/gibbon-consent-probe-current.json`, `/tmp/gibbon-consent-network-check/result.json`.
- 검사 로그: `/tmp/gibbon-consent-rust-final.log`, `/tmp/gibbon-consent-ui-final.log`, `/tmp/gibbon-consent-clippy-final.log`, `/tmp/gibbon-consent-http-boundary.log`.

이 결과는 해당 loopback DSN과 관찰 구간에서의 진단 무전송 확인이다. 앱과 외부 도구·운영체제 전체의 모든 네트워크 통신을 패킷 수준으로 감사했다는 의미는 아니다.

## 시안과 남은 검증

[SVG](../design/privacy-onboarding.svg)를 `rsvg-convert`로 [PNG](../design/privacy-onboarding.png) 렌더링해 겹침·잘림을 시각 확인했다. 영문 레이블, 필수/선택 분리, 기본 해제, 초안 차단, 설정의 즉시 철회를 표현한다. 실제 Manager Window는 기존 크기에서 스크롤한다. [디자인 시스템](../design-system.md)의 해당 기준을 함께 갱신했다.

실제 네이티브 UI에서 최초 약관 수락·설정 변경·키보드 탐색, 운영 정보가 확정된 빌드의 실제 Sentry 수신, Windows/Intel 빌드·GUI는 미검증이다. iMac은 사용자가 요청한 절전을 유지하며 깨우지 않았다. Windows는 원격 연결 불가 상태로 이번 변경을 배포하지 않았다. 공개 릴리스·태그·설치 앱 업데이트는 수행하지 않았다.

확신도: 자동 동의/전송 경계 **Conf 3**, macOS 실제 거부 상태/loopback 무전송은 해당 관찰 범위에서 **Conf 5**. 위험도 **Medium**(동의 상태와 앱 첫 실행 경로 변경).
