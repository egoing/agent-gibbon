# 약관·개인정보 안내 초안의 운영 조건

- [최소 이용약관](terms.md)과 [개인정보·선택적 진단 안내](privacy.md)는 운영자와 문의처가 반영된 검토용 초안이다. 정식 법률 검토나 전 세계 배포 요건 충족을 의미하지 않는다.
- 현재 `src-tauri/src/privacy/notice.json`에는 운영자 egoing, 문의처 egoing+support@gmail.com, 저장 지역 미국 아이오와를 반영했다. 최종 보유기간 필드만 null이며 원본·집계·백업별 고지는 [실제 계정 조사](../verification/sentry-data-retention-20261106.md)를 근거로 확정한다. 명시적 동의가 있어도 readiness 검사를 통과하지 못하므로 전송하지 않는다. 문서를 고쳐 확정할 때도 실제 Sentry 설정과 맞춰야 하며 API 키·DSN은 이 파일에 넣지 않는다.
- SDK 송신 정보·IP 처리와 저장·국외 이전 관계를 확인하고 초안에 미정으로 표시한 부분을 함께 갱신한다. 자동으로 국가나 보유기간을 추측하지 않는다. 자료가 사용자 기기를 떠나기 전에 동의를 받고 철회 경로도 제공한다.
- 정책 식별값은 약관·개인정보 안내·운영 정보의 SHA-256이다. 내용이 달라지면 기존 동의를 자동 승계하지 않는다. MIT License의 기존 권리는 유지한다.
- [디자인 기준](../design-system.md), [와이어프레임](../design/privacy-onboarding.png), [진단 구현](../diagnostics.md)을 함께 관리한다.

작성 시 확인한 기본 기준: [개인정보 보호법 제15조](https://www.law.go.kr/법령/개인정보보호법/제15조), [제22조](https://www.law.go.kr/법령/개인정보보호법/제22조), [제28조의8](https://www.law.go.kr/법령/개인정보보호법/제28조의8), [EDPB의 동의 기준](https://www.edpb.europa.eu/sme/be-compliant/process-personal-data-lawfully_en), [Sentry의 저장 지역 안내](https://www.sentry.help/en/articles/13965013-where-are-your-data-servers-located).
