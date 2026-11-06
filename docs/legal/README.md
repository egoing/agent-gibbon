# 약관·개인정보 안내의 운영 조건

- [이용약관](terms.md)과 [개인정보·선택적 진단 안내](privacy.md)는 앱에 포함해 로컬에서 읽는다. 2026-11-06 사용자 승인으로 진단 원본 최대 30일, 백업 생성 후 30/90일, 오류 누적 통계 별도 고지, Release Health 수집 보류를 반영했다.
- `src-tauri/src/privacy/notice.json`에 운영자·문의처·저장 지역과 원본 보유기간 30일을 확정해 선택 동의를 활성화한다. 원본 보유기간 한 숫자가 백업·누적 통계까지 같은 날 완전히 삭제된다는 의미는 아니다. 실제 계정 조사와 고지를 일치시키며 API 키·DSN은 이 파일에 넣지 않는다.
- SDK 자동 세션을 끄고 envelope 경계에서도 세션·세션 집계 항목을 차단한다. 향후 도입 시 실제 보유기간을 확인하고 문서·동의를 갱신한다.
- 사용자 동의를 대신 저장하지 않는다. 동의 전 외부 전송 차단·동의 저장·철회와 저장 실패 처리를 검증한다. 이 문서 변경은 정식 법률 검토나 전 세계 배포 요건 충족을 뜻하지 않는다.
- debug 개발 인스턴스는 사용자 요청에 따라 약관의 기능 진입 제한만 기본 생략한다. 수락·전송 동의로 기록하지 않으며 SDK 초기화와 전송 허용에 사용하지 않는다. 명시적 약관 검증 실행과 배포 환경의 구분은 [검증 기록](../verification/dev-terms-skip-20261106.md)을 따른다.
- 정책 식별값은 약관·개인정보 안내·운영 정보의 SHA-256이다. 내용이 달라지면 기존 동의를 자동 승계하지 않는다. MIT License의 기존 권리는 유지한다.
- [디자인 기준](../design-system.md), [와이어프레임](../design/privacy-onboarding.png), [진단 구현](../diagnostics.md)을 함께 관리한다.

작성 시 확인한 기본 기준: [개인정보 보호법 제15조](https://www.law.go.kr/법령/개인정보보호법/제15조), [제22조](https://www.law.go.kr/법령/개인정보보호법/제22조), [제28조의8](https://www.law.go.kr/법령/개인정보보호법/제28조의8), [EDPB의 동의 기준](https://www.edpb.europa.eu/sme/be-compliant/process-personal-data-lawfully_en), [Sentry의 저장 지역 안내](https://www.sentry.help/en/articles/13965013-where-are-your-data-servers-located).
