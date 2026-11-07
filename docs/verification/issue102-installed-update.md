# #102 실제 설치 앱 업데이트 검증

## 범위와 출처

- 사용자 요청: 새 워크스페이스에서 작은 변경 후 정식 릴리스, 이 컴퓨터의 실제 앱 업데이트 확인.
- 작업 루트: `/Users/egoinglee/.codex/worktrees/issue-102-update-verification/agent-gibbon`
- 브랜치: `codex/102-update-verification`; 개발 ID: `dev-48fc08799f16fff8bfceaf8a`.
- 환경: macOS 15.6.1 (24G90), Apple Silicon ARM64, Node v24.21.0, Rust 1.91.1.
- 기존 실행 PID 14633은 다른 워크트리 개발 앱으로 유지한다. 정식 설치 앱이 없어 공식 v0.2.9를 최초 설치했다.
- 대상: 운영 서명 키를 포함한 정식 v0.2.9 → v0.2.10. 개발 앱이나 수동 검사 artifact를 실제 업데이트 증거로 쓰지 않는다.
- 작은 변경: 최초 수동 설치 안내의 0.2.8 URL 결함 누락을 바로잡고 0.2.10으로 버전 필드를 일치시킨다. main에 병합된 웹 창 기능도 새 릴리스에 포함된다.

## 최초 설치

`gh release download v0.2.9 --repo egoing/agent-gibbon --pattern 'Agent.Gibbon_0.2.9_aarch64.dmg' --pattern SHA256SUMS.txt --dir /tmp/agent-gibbon-102-bootstrap`.

DMG SHA-256 `cae038cd3bfaf8b106c314f13ec90c7b93cbe83e566a7082b4098c636b74330d`는 공개 자산 digest와 일치한다. hdiutil 자체 체크섬 확인 후 `/Applications/Agent Gibbon.app`에 복사했다. 설치된 앱은 없었으므로 기존 앱을 덮어쓰지 않았다.

실행 PID 64758의 경로 `/Applications/Agent Gibbon.app/Contents/MacOS/agent-gibbon`를 ps와 sample로 대조했다. diagnostics 응답은 `instance:null`, `updater.enabled:true`, `current_version:0.2.9`, 빌드 revision `1e7b30eb31a2`다. 약관은 미수락(`onboarding_required:true`), 선택 진단은 비활성이다. 동의 파일을 대신 만들거나 진단 전송을 켜지 않는다.

네이티브 관리 창은 PID 64758, window ID 45920, 제목 `Agent Gibbon — 메모 관리`로 식별했다. Orca 접근성 조회는 두 번 시간 초과했고 UI 검증 통과로 계산하지 않는다. sample의 main thread는 일반 이벤트 루프 대기여서 이 결과만으로 앱 교착을 단정하지 않는다.

## 진행 상태

- [x] 새 워크스페이스와 버전·플랫폼·설치본 출처 확인
- [x] 공식 v0.2.9 최초 설치와 updater 활성 상태 확인
- [x] Rust 206개·Node UI 169개, Rustfmt·Clippy·버전 일치·diff 공백 검사와 개발 빌드 통과
- [ ] 작업 커밋·전체 플랫폼 검사와 정식 v0.2.10 공개
- [ ] 실제 확인·다운로드·서명 검증·안내 및 작업 보호
- [ ] 설치·재시작·실행 버전과 메모 상태 대조

이 기록 작성 시점에는 업데이트 전 과정 성공을 확인하지 않았다. Intel Mac·Windows의 실제 설치 업데이트, 자동 idle 설치와 버튼 설치 양쪽, 오프라인·변조·중복 요청의 실앱 검증은 별도 범위이며 미실행 항목을 구분한다. 디자인 기준은 [디자인 시스템](../design-system.md)의 자동 업데이트 절을 따른다.

로컬 로그: `/tmp/issue102-local-test.log`, `/tmp/issue102-ui.log`, `/tmp/issue102-clippy.log`. 실제 배포 전이라 원격 검사·실제 업데이트 성공으로 확대 해석하지 않는다.

개발 CLI 확인: `npm run dev:info`의 root/id/binary와 `scripts/agent-gibbon-dev inspect --id issue-102-release-progress` 응답의 `ok:true`, `instance:dev-48fc08799f16fff8bfceaf8a`, 메모 ID·본문을 대조했다. 개발 빌드 로그는 `/tmp/issue102-dev-build.log`이며 진행 메모를 설치 앱과 분리했다.

## 사용자 확인 후 사전 상태

사용자의 약관 확인 완료와 커밋·push·릴리스 계속 승인을 받았다. 설치 앱 diagnostics에서 `terms_accepted:true`, `terms_skipped:false`, `onboarding_required:false`, `updater.enabled:true`, 현재 버전 0.2.9, phase idle(최신 버전)을 확인했다. `sharing_allowed:true`도 관찰했으며 이는 사용자가 앱에서 선택한 상태다. 에이전트는 동의 설정을 변경하지 않았다.
