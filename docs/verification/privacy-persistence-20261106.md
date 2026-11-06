# 실제 동의 저장·설정 반영 검증 — 2026-11-06

## 결과

macOS ARM64의 실제 Tauri 앱에서 체크박스·Continue·Settings·Save Changes를 조작하고, 로컬 JSON·백엔드 진단 상태·재실행한 UI를 대조했다. 테스트 약관과 loopback 수신기를 사용했으며 사용자의 실제 약관·전송 동의를 대신 저장하지 않았다. 공식 Sentry로 진단 이벤트를 보내지 않았다.

| 검증 | 실제 결과 |
| --- | --- |
| 최초 상태 | 두 체크박스 해제, Continue 비활성, 동의 파일 없음, HTTP 0건 |
| 약관만 체크하고 Continue | terms:true / sharing:false 저장, 일반 기능 진입, SDK 꺼짐 |
| 거부 후 재실행 | 같은 파일 유지, Settings 체크 해제와 SDK 꺼짐 복원 |
| 설정에서 전송 체크만 켜기 | UI만 체크됨; 저장 파일은 false, SDK 꺼짐, HTTP 0건 유지 |
| Save Changes | sharing:true 저장; 비동기 초기화 완료 후 Sharing enabled·SDK 활성 일치 |
| 수락 후 재실행 | 파일 내용 유지, Settings 체크됨·sharing_allowed:true·SDK 활성 복원 |
| 설정 체크 끄기 | Save Changes 없이 sharing:false 즉시 저장; Not sharing·SDK 비활성 일치 |
| 철회 후 전송 검사 | 4초 관찰에서 loopback 요청 25→25건, 바이트 동일; probe disabled |
| 철회 후 재실행 | 같은 파일 유지, Settings 체크 해제·SDK 비활성 복원 |
| 손상된 동의 파일 | 앱을 중지한 뒤 검증 파일만 손상시켜 재실행; 재동의 UI·경고·SDK 꺼짐, 요청 증가 없음 |
| 첫 동의에서 두 항목 모두 수락 | 유효 JSON으로 대체되어 terms:true / sharing:true, 정책 해시와 선택 시각 저장 |
| 검사 종료 상태 | 테스트 전송을 다시 끄고 검증 앱과 수신기를 종료 |

`format:1`, 현재 문서 SHA-256, 약관·전송 boolean, 양수 Unix 초 시각을 파일에서 확인했다. 실제 정책 해시는 fixture 약관·안내·운영 정보의 계산 결과와 일치했다. 파일 mode 0600, 부모 디렉터리 0700도 확인했다. 재실행만으로 파일이나 선택 시각이 바뀌지 않았다.

사용자 본래 개발 인스턴스 `dev-a5210ae16e79437d38490c8f`의 consent.json은 시작·종료 SHA-256이 같았다. 검사 명령은 검증용 개발 CLI와 검증 앱의 PID/창에만 적용했다.

## 대상·재현 근거

- 검증 루트: `/Users/egoinglee/orca/workspaces/agent-gibbon/floatnote/privacy-persistence-audit-20261106`.
- instance: `dev-d715a196521bf7c65b1aaa05`.
- binary: 해당 루트의 `src-tauri/target/debug/agent-gibbon`. `npm run dev:info`의 root/id/binary와 `--instance-info` 기반 개발 CLI 검사, 실제 프로세스 실행 파일 경로, 진단 응답 instance를 대조했다.
- 최초 PID 81424, 재실행 PID 83045·87606·89041·90025. 최초 네이티브 창 ID 38676이며 재실행마다 창을 다시 조회했다. 제목은 `PRIVACY AUDIT — TEST ONLY`다.
- 기준 HEAD `02cefc78e20347968ada041422b105e2080c0e93`와 당시 진행 중인 동의 선택 활성화/세션 비활성화 변경을 복사한 소스 스냅샷이다. 검증 종료 시 주요 동의·프런트·전송 코드 6개 파일은 원래 작업 트리와 SHA-256이 같았다. [선택 기록·상태·코드 해시](privacy-persistence-evidence-20261106.json)에 근거를 저장했다.
- [실제 Settings 체크 상태 캡처](privacy-persistence-macos.png). 저장 파일, 재실행 응답, native UI의 Value:0/1과 텍스트를 독립적으로 확인했다. 접근성 AXPress 호출의 unasserted 표시는 그 자체를 성공으로 삼지 않고 후속 UI 읽기·디스크·상태를 확인해 판정했다.
- 로컬 상세 증거: `/tmp/gibbon-consent-audit-evidence/`, 빌드 로그 `/tmp/gibbon-consent-audit-build.log`.

## 격리와 한계

Orca로 별도 워크트리를 만들고 현재 소스를 스냅샷으로 복사했다. 앱/동의/전송 로직은 바꾸지 않았다. 테스트 전용 변경은 약관·개인정보 안내를 법적 효력이 없는 테스트 문서로 교체하고, 운영 정보를 localhost fixture로 바꾸고, 재실행 후 Manager를 관찰할 수 있게 Tauri 창 설정에 Manager를 추가한 것이다. 이 창 설정은 제품의 시작 시 Manager 자동 복원 검증을 의미하지 않는다. 기본 메모 창 설정은 유지했다.

빌드용 DSN은 오직 `127.0.0.1` 수신기를 가리켰다. 관리 토큰과 공식 DSN을 자식 프로세스 환경에서 제거하고 proxy도 제거했다. SDK 초기화 이후 합성 probe와 실제 앱 진단 요청이 loopback에 도달하는 것을 확인해 켜기 경로도 검사했다. 공식 Sentry 처리·수신 성공은 이번 검사 범위가 아니다. fixture 변경이나 테스트 동의는 제품 소스·사용자 동의 기록에 반영하지 않았다.

첫 재실행의 Settings 조작에서 macOS 접근성 제공자가 3초 시간 초과를 반환했다. CLI 상태 응답은 정상이고 후속 조회에서 Settings 전환 완료를 확인했으므로 같은 클릭을 중복 실행하지 않았다. 프로세스 sample에서는 메인 이벤트 루프 대기와 이벤트 처리가 관찰됐고 이후 접근성 조회가 회복됐다. 원인을 제품 교착으로 확정하지 않았으며 이 시간 초과 자체를 성공으로 숨기지 않는다. 또한 파일 저장 직후·SDK 비동기 초기화 완료 전의 중간 상태는 최종 활성 상태와 구분했다.

Windows/Intel·실제 사용자 약관 수락·디스크 쓰기 실패·백업 복구·공식 Sentry 수신은 미검증이다. 제품 코드 수정은 필요하지 않았다. 기존 단위 검사와 별개로 이번 기록은 실제 native UI→저장→재시작의 동작 근거다.

확신도 **Conf 4**(독립 테스트 데이터와 수신기를 사용하는 실제 native 앱), 위험도 **Low**(검증 인스턴스만 조작).
