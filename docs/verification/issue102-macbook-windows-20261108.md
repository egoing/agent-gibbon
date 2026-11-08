# MacBook 및 Windows Desktop 업데이트 검증 · 2026-11-08

- [x] 현재 MacBook의 네이티브 커널 아키텍처 arm64 확인.
- [x] 공개 v0.2.11의 latest.json에서 darwin-aarch64 및 windows-x86_64 자산 URL 확인.
- [x] Windows mywin에서 작업 세션 실행 확인. 초기 Orca 세션 ID는 codex_c3d49d8a_2372_4649_ab81_7400714ec86f.
- [ ] Windows Codex Desktop에서 작업 수신 및 대상 프로세스 확인.
- [ ] Windows 기본 무입력 300초, 설치와 보호·복원 증거 확인.
- [ ] MacBook 격리 프로필의 이전/후보 서명과 버전 확인.
- [ ] MacBook 읽기·편집·터미널·수동 설치 결과 기록.
- [ ] 실패 및 복구 시험 결과 기록.

- [x] GitHub 공개 릴리스에서 ARM64 updater 번들과 Windows x64 설치 파일을 내려받아 SHA256SUMS.txt의 두 값과 각각 일치함을 확인.
- [x] M1 Pro 운영 앱 v0.2.10에서 v0.2.11이 준비됐고 웹 세션 보호 사유가 표시됨. 별도로 진단 전송 시험 명령도 호출되어 event_id 및 queue_flushed=true가 반환됨. 외부 수신은 미확인.
- [x] MacBook 격리용 v0.2.10 및 v0.2.11 arm64 번들 생성. 검증 전용 ID org.egoing.floatnote.verification.issue102. 실행 증거 작업 폴더 /tmp/issue102-arm64-e2e-20261108.
- Windows Orca 작업 세션은 이전 후보 캐시의 조기 적용을 관찰. 이 시도는 300초 판정에서 제외. 실제 Windows Desktop에서 화면 공유로 확인한 채팅은 과거 설치 시험 채팅.
- [x] Windows Codex Desktop 새 채팅 ‘이슈 #102 Windows 실기기 검증’에서 인계 수신 및 표준 시간값 별도 빌드 준비 확인. 컴퓨터 사용 승인 창 대기.
- MacBook 검증 앱의 필수 약관 선택을 사용자가 승인. 실제 클릭 직전 Mac 잠금으로 입력 거부. 선택 진단 미체크 상태, 설치 결과 대기.

### 잠금 상태에서 실행한 검증

- [x] npm test 종료 코드 0. Rust 208개 및 Node 173개 통과.
- [x] 127.0.0.1:18453 피드 연결 거절 확인 후 검증용 서버 재시작. HTTP manifest 및 후보 다운로드 성공.
- [x] scripts/release-assets.mjs의 verifySignature와 시험 공개키를 사용해 ARM64 원본 통과, HTTP에서 받은 1바이트 변조 후보 거부, 원본 재검증 통과 확인.
- 이 검사는 릴리스 서명 검증기에 대한 결과다. 앱 자체의 거부 안내·재시도 및 설치는 별도 실기기 시험 항목이다.

- [x] MacBook 잠금 해제 후 격리 검증 앱에서 필수 약관만 동의. terms_accepted=true, sharing_allowed=false 확인.
- [x] 실제 v0.2.10 앱에서 변조 번들 다운로드 후 오류 안내 및 기존 버전 유지. 정상 번들 복구 후 수동 재확인으로 v0.2.11 ready 확인.
- [x] Windows Desktop에서 인계 지시 수신 및 표준 빌드 완료. 화면 제어 승인 대기, 300초 시험 시작 전.

### Apple Silicon 수동 설치 및 복원 재시험

- [x] 수정 전 설치 클릭 두 회에서 숨겨진 메모의 poll_fold, poll_warp, snapshot 조회로 pending_request 발생. 미저장 편집은 없었다.
- [x] 수정 후 19:53 KST 격리 검증 앱 v0.2.10에서 HID 무입력 159초에 설치 버튼 실행. 자동 설치 기준은 300초. v0.2.11 실행 확인.
- [x] 검증 메모 A는 보임·고정, 좌표 (1638,1024), 크기 600×96 유지. B는 숨김·고정, 좌표 (1748,494), 크기 600×480 유지. 두 메모 모두 note 객체 전체 일치.
- 증거 파일은 /tmp/issue102-arm64-e2e-20261108/fixed-run-preclick.json 및 fixed-run-{pre,post}-{a,b}.json. Windows는 이번 재시험에서 승인 대기 유지.
