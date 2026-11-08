# #116 iMac 수동 설치와 세션 복원

Intel iMac에서 2026-11-08에 로컬 피드의 서명된 0.2.11 → 0.2.12 검증 앱을 실행했다. 테스트 조건은 무입력 30초, 확인 주기 60초, 재시도 5초다. 후보의 SHA-256은 `c39f2ebd6030e02e5b9d3b746afa0011322d70c31eec17f38c01f026b2254ad1`. 이 후보는 #116·#118·#119 작업을 합친 로컬 검증 빌드다.

- 워크트리: `/Users/egoing/.codex/worktrees/imac-102/agent-gibbon`. 전용 검증 앱·자료는 `/private/var/folders/mk/dtctp5sn72nbvtvlmbj4ztd00000gn/T/imac102-combined-4hkd4zpx/verification/manual` 아래에 뒀다. 실행 파일은 그 안의 `Run.app/Contents/MacOS/agent-gibbon`, 인스턴스는 `dev-001020010200102001020010`. 메모 `imac102-signed`를 inspect하여 표식과 인스턴스를 대조했다.
- PID 5561의 네이티브 창 ‘Agent Gibbon · 업데이트’에서 터미널 두 개와 서로 다른 로컬 URL의 웹 창 두 개가 각각 ‘보기’ 버튼을 제공했다. 숨긴 셸의 ‘보기’를 눌러 visible=false → true, running=true를 확인했다. 재시작 확인 화면에서 취소한 뒤에도 셸 PID 6141이 유지됐다.
- 같은 앱에서 Ctrl을 누르고 준비 안내 위를 지나도 창 크기는 유지됐고, 일반 메모는 300×48 → 300×12 → 300×48로 접히고 복원됐다. 로그는 `/tmp/102-hover-result.log`. 접혀 있던 main 메모는 펼친 뒤 설치했다.
- 한 번은 feedback 창에 진행 중인 요청 1개가 남아 설치를 보류했다. 사용자가 다시 확인하는 경로로 재시도해 모든 참여 창이 준비된 후 설치됐다. 보류 후 취소나 설치 실패로 프로세스를 미리 종료하지 않는다.
- 재시작 PID 9865에서 현재 버전 0.2.12와 prepared 기록, 설치된 바이너리와 후보의 바이트 일치를 확인했다. `before.json`·`after.json`에서 메모 본문, 원래 웹 창 ID와 각 URL, 위치·크기·고정·표시 상태가 일치했다. 두 터미널은 각각 같은 창으로 복원됐고 실행 중이던 셸 PID 6141·6163은 모두 null, running=false였다. 하나는 숨김, 다른 하나는 표시 상태였다.
- Chromium에서 실제 UI 파일로 360×140 준비 안내의 보류 목록과 ‘보기’, 종료 영향 안내, 취소를 검증했다. 설치 IPC는 모의 응답을 사용했다. UI 회귀는 편집 중 설치 차단, 확인 도중 PID 변경, 복원 미지원 후보 차단을 포함한다.
