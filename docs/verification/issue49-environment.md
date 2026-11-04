# #49 활성 화면이 없는 환경에서 open 비가시

2026-11-05, 기준 `f1eb03222bbc087f28f73e84474ee3ae95891298`, macOS ARM64. **제품 수정 없이 환경 원인을 재현·확인했다.** 두 연결 화면이 asleep=1/active=0인 동안 Tauri available_monitors는 빈 목록이며 배치가 ‘배치할 화면이 너무 작습니다’로 실패했다. Swift의 CGGetActiveDisplayList 직접 호출도 성공코드0·개수0이었다. AppKit NSScreen에는 2개가 남아 있어 저장 상태·프런트 준비 문제와 구분했다. tao 0.37.1의 화면 조회가 같은 CoreGraphics 함수를 쓰는 것도 로컬 의존성 소스로 확인했다.

감독자 승인으로 일시적 `caffeinate -u`를 실행하자 active=2/asleep=0으로 바뀌고 **원본 코드의 신규 open이 visible:true**가 됐다. 짧은 깨우기 종료 뒤 재절전을 관측해 필요한 동안만 120초로 연장했다. 검증 시 활성 화면은 1개였으므로 물리 다중 화면 회귀를 주장하지 않는다. 영구 전원 설정·잠금해제·권한은 바꾸지 않았다. 진단 출력은 제거하고 원본으로 재빌드했다. [화면 전후·진단 근거](issue49-environment/display-evidence.json).

## 출처와 실제 검증

- root `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/issue-36-first-approach`, instance `dev-8fbfdca2d77ba80c9f021e8f`, binary는 이 root의 `src-tauri/target/debug/agent-gibbon`이다. 개발 CLI만 사용했고 lsof txt로 PID별 절대 실행 파일을 대조했다. 진단 PID18037의 [inspect 고유 본문·출처](issue49-environment/before-origin.json)·[비가시 창 목록](issue49-environment/before-windows.json)을 남겼다.
- 원본 재빌드 PID **21283**: [새 생성·반복 open 위치/크기 보존·숨김 복원·이전 프로세스 저장 복원 4검사 통과](issue49-environment/open-report.json). 매 응답 ok/instance와 메모 본문을 대조했다. `issue49-verified-new`, 본문 `ISSUE49 VERIFIED NEW UNIQUE`를 [PID·네이티브 창 제목/번호](issue49-environment/new-source.json)와 연결해 [실제 화면](issue49-environment/new.png)을 캡처하고 육안 확인했다. Orca 창 목록은 비어 있어 해당 결과만으로 GUI 출처를 주장하지 않았으며 CoreGraphics의 같은 PID/정확한 제목을 사용했다.
- 같은 원본 PID의 `python3 /tmp/gibbon-issue49/stash-smoke.py after` **통과**: 기존 `stash-first-approach-smoke.py`의 root/출력 폴더/PID 선택만 이 실행에 맞춘 임시 복사본이다. 메모 `issue36-first-approach-66779502`, [실제 최초접근·안내 보호·이탈/재진입·추격 방지·복원 보고](issue49-first-approach/report.json), [도착 캡처](issue49-first-approach/arrived.png)와 [PID/네이티브 창 출처](issue49-first-approach/arrived-source.json). 전면 PID427 유지, 포인터 원위치 복원과 검증 메모 숨김을 확인했다. 최초접근 순간 moving=true/tip=false를 상태 샘플로 확인했으며 캡처 자체를 이동 중 프레임으로 해석하지 않는다.
- `npm run dev:build`, `npm test` **Rust84/UI53**, `npm run check`, Rustfmt check, version:check(0.2.7) 통과. 로그는 `/tmp/gibbon-issue49/{final-build,tests,clippy}.log`에 있다. 제품 변경이 없어 새 테스트를 추가하지 않았다.

**Conf 5 / Risk Low**: 이 호스트에서 절전 전후의 원인과 원본 동작을 확인했다. 화면이 실제로 없는 상태의 앱 UX/CLI 오류 전달 정책 변경은 하지 않았으며 Windows·Intel·물리 다중 화면 회귀는 미실행이다. [정리](issue49-environment/cleanup.json): 자체 앱과 자체 caffeinate만 종료했고 사용자 메모·설치 앱·다른 프로세스는 건드리지 않았다. 원격 push·PR·병합·이슈 닫기는 실행하지 않았다. #36 통합 GUI 차단은 이번 검증에서 해소됐으며 최종 병합/CI 판단은 감독자가 한다.

[디자인 시스템](../design-system.md) · [Apple 활성 화면 API](https://developer.apple.com/documentation/coregraphics/cggetactivedisplaylist(_:_:_:)) · [온라인 화면 API](https://developer.apple.com/documentation/coregraphics/cggetonlinedisplaylist(_:_:_:))
