# 동의 항목 그룹핑·탐색 정리 — 2026-11-06

## 요청과 반영

- 약관 동의 체크박스를 약관 설명 카드의 하단으로 옮겼다.
- 진단 동의도 같은 카드의 하단으로 옮겨 제목 → 설명 → 문서 열람 → 동의 순서를 맞췄다. 동의 영역에는 같은 구분선을 적용했다.
- 운영 정보 미정으로 진단 체크박스가 비활성화된 이유를 같은 카드 안, 체크박스 바로 위에 표시했다. 미정 상태에서 전송을 막는 기존 정책은 유지했다.
- 최초 약관 수락 전에는 All Memos·Settings 탐색을 초기 HTML부터 숨긴다. 동의 상태를 읽지 못한 경우에도 숨기며, 수락 뒤 표시하는 동작은 UI 테스트로 확인했다.
- 실제 수신 조건·동의 저장 형식·약관 본문·기존 메모의 위치/숨김 정책은 변경하지 않았다.

## 자동 검사와 출처

- 루트 `/Users/egoinglee/dev/project/agent-gibbon`, instance `dev-a5210ae16e79437d38490c8f`.
- 기준 HEAD `cec17ce` + 이번 UI 변경으로 `npm run dev:build` 실행, 성공 로그 `/tmp/gibbon-consent-grouping-build.log`.
- `npm run test:ui`: **80개 통과**. 초기 탐색 숨김·수락 뒤 표시·상태 로드 실패 시 숨김·안내의 접근성 연결을 기존 동의 테스트에 포함했다. 로그 `/tmp/gibbon-consent-grouping-ui.log`.
- `git diff --check` 통과. Rust 소스 변경이 없어 Rust 테스트를 반복 실행하지 않았다.
- 새 앱 PID **97684**, 실행 파일 `/Users/egoinglee/dev/project/agent-gibbon/src-tauri/target/debug/agent-gibbon`.
- 같은 루트의 개발 CLI 응답에서 예상 instance와 `onboarding_required:true`를 확인했다. `/tmp/gibbon-consent-grouping-launch.json`, `/tmp/gibbon-consent-grouping-diagnostics.json`.
- 네이티브 창 ID **38296**, 제목 `Agent Gibbon [DEV main · a5210ae1] — 메모 관리`, 944×480. 정확한 PID/실행 파일과 native 창을 대조했다.

## 실제 UI 확인과 한계

Orca Computer Use의 해당 PID/창 상태에서 약관 체크박스가 Terms of Use 그룹 아래, 진단 체크박스가 Service improvement diagnostics 그룹 아래에 위치함을 확인했다. All Memos·Settings 탐색은 접근성 트리에서 사라졌다. [실제 화면 캡처](privacy-grouping-macos.png)에서 진단 카드 하단의 안내·선택 영역과 탐색 없는 헤더를 시각 확인했다.

체크박스나 Continue를 대신 눌러 사용자의 동의를 저장하지 않았다. 스크롤 요청은 synthetic input / unverified였으므로 스크롤 동작 검증을 통과로 기록하지 않는다. 캡처된 화면과 접근성 트리의 실제 표시 상태만 근거로 사용했다. 전체 동의 후 진입·키보드 이동은 이번 native 검사 범위가 아니다.

[와이어프레임 SVG](../design/privacy-onboarding.svg)와 [PNG](../design/privacy-onboarding.png)도 동일한 그룹핑으로 갱신해 rsvg-convert 렌더와 시각 확인을 마쳤다. [디자인 시스템](../design-system.md)에 기준을 반영했다.

이번 배치 변경의 Windows/Intel UI와 iMac 배포는 미검증이다. 이전 Windows 빌드 검증을 이번 UI의 실행 증거로 재사용하지 않는다.

확신도: 배치·탐색의 macOS native 표시 Conf 4, 수락 후 전환은 자동 검사 Conf 3. 위험도 Low(화면 그룹핑·탐색 표시의 국소 변경).
