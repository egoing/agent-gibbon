# Sentry 저장 지역·보유기간 조사 — 2026-11-06

## 확인한 실제 설정

공식 앱의 빌드용 DSN을 메모리에서만 읽어 대상 프로젝트를 확인했다. API 응답에서 필요한 서비스 설정만 추렸으며 DSN·API 토큰·결제 정보·오류 원문은 기록하지 않았다. 모든 Sentry 호출은 읽기 전용 GET이며 서버 설정·요금제·자료 삭제·이벤트 전송은 변경하지 않았다.

- 조직 / 프로젝트: `egoing-personal / floatnote`.
- 조직 API의 `links.regionUrl`: `https://us.sentry.io`. 공식 ingest 주소의 US 리전 표기와도 일치한다.
- 공식 저장 지역 문서상 US 물리 위치: **미국 아이오와**.
- 구독 API의 `planDetails.name`: **Developer**, `isFree:true`, `onTrialPlan:false`.
- `planDetails.retentionDays`: **30일**.
- 실제 유효 보유기간: span 일반 30일·downsampled 30일, log 일반 30일·downsampled 30일.
- traceMetric API는 일반 30일·downsampled **396일**을 반환한다. 해당 프로젝트의 `hasTraceMetrics:false`다. 이 반환값을 현재 앱의 원본 span 보유기간이나 실제 집계 지표 수집 증거로 해석하지 않는다.
- 프로젝트 신호: `hasLogs:true`, `hasSessions:true`, `hasReplays:false`. 이는 서버의 기존 자료/기능 상태이며 현재 앱이 전송 중임을 뜻하지 않는다.

공개 문서와 실제 계정 설정이 일치하는 범위:

| 데이터 | 현재 프로젝트에 적용해 확인한 값 | 근거 |
| --- | --- | --- |
| 오류 이벤트 | 30일 | 실제 Developer 요금제 + 공식 오류 보유기간 |
| 성능 trace/span·transaction 원본 | 30일 | 실제 effectiveRetentions.span + 공식 표 |
| 구조화 로그 원본 | 30일 | 실제 effectiveRetentions.log + 공식 표 |
| traceMetric 집계 | API 일반 30일 / downsampled 396일, 프로젝트는 미사용 상태 | 실제 계정 API와 hasTraceMetrics:false |
| 원본 이벤트 백업 | 종류에 따라 생성 후 30일 또는 90일에 삭제 | Sentry 공식 백업 안내; 계정별 별도 백업 수명 API는 확인하지 않음 |

Sentry 공식 표에서 Business/Enterprise의 span 보존은 원본 30일 외 최대 13개월 sampled 보존이 있다. 현재 Developer 계정은 그 요금제가 아니므로 현재 원본 span에 13개월이 적용된다고 설명하지 않는다. 보유기간은 수집 당시 요금제에 따라 적용되어 나중에 요금제를 바꿔도 과거 이벤트의 기간이 함께 바뀌지 않는다.

## 안내 반영과 남은 범위

사용자가 정한 운영자 `egoing`, 개인정보 문의 `egoing+support@gmail.com`을 약관·개인정보 안내와 `notice.json`에 반영했다. 저장 지역은 실제로 확인한 미국 아이오와를 반영했다. 약관·개인정보 안내 버전은 `2026-11-06.2`이며 문서/운영 정보 해시가 바뀌므로 기존 동의를 자동 승계하지 않는다.

원본의 조회 보유기간, 백업 삭제 시점, 누적 이슈 집계·Release Health 세션 통계의 보유를 하나의 숫자로 단정하지 않았다. 최종 고지 정리 전 `retention_days`는 null로 유지했고 진단 전송을 활성화하지 않았다. Release Health 집계·이슈 누적 통계에 대해 위 원본 표와 같은 완전 삭제 시점을 계정별로 검증한 것은 아니다. 기존 약관 수락이나 운영자 입력이 사용자 전송 동의를 대신하지 않는다.

## 조회 근거와 재현

조회 API: `GET /api/0/projects/`, `GET /api/0/projects/egoing-personal/floatnote/`, `GET /api/0/organizations/egoing-personal/`, `GET /api/0/customers/egoing-personal/`. 마지막은 현재 SaaS UI용 구독 응답이며 공개 API의 안정성 보장은 가정하지 않는다. 응답 중 regionUrl·요금제 이름·유효 보유기간·사용 신호만 확인했다. 로컬 요약은 `/tmp/gibbon-sentry-retention-summary.json`이며 인증 토큰은 저장하지 않았다.

공식 문서:

- [Data Storage Location](https://docs.sentry.io/organization/data-storage-location/)
- [Data Retention Periods](https://docs.sentry.io/security-legal-pii/security/data-retention-periods/)
- [How Long Are Errors/Events Stored in Sentry?](https://www.sentry.help/en/articles/13964323-how-long-are-errors-events-stored-in-sentry)
- [How frequently is data backed up?](https://www.sentry.help/en/articles/13965019-how-frequently-is-data-backed-up)

확신도: 현재 조직·프로젝트·구독의 조회 값 Conf 5. 계정별 원본 외 자료의 전체 삭제 시점은 미확인. 위험도 Low(읽기 전용 조사·운영 정보 고지 변경, 전송은 차단 유지).

## 반영 검증

- `cargo test --manifest-path src-tauri/Cargo.toml privacy::tests`: 동의 저장·이전 정책 무효화·미완성 안내 차단 3개 통과.
- `npm run test:ui`: 80개 통과.
- `npm run dev:build`: macOS ARM64 개발 빌드 통과. 인증 토큰·공식 DSN·진단 플래그를 빌드 자식 환경에서 제거했다.
- SVG→PNG 렌더와 시각 확인, `git diff --check` 통과.
- 기존 개발 앱이 실행 중인 경우 해당 앱만 재실행해 번들에 포함된 새 안내를 반영했다. 사용자 동의를 대신 저장하지 않았다. 현재 앱의 privacy readiness와 SDK 비활성 응답은 `/tmp/gibbon-operator-runtime.json`에 확인했다.
- 실제 동의 버튼 조작·실제 Sentry 신규 이벤트 수신·Windows/iMac 배포는 이번 반영 검증 범위가 아니다.
