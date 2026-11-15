# #191 웹·캡처 메모 전체 정렬 — 2026-11-15

## 판정

웹·터미널 누락은 코드에서 원인을 확인하고 수정했다. 캡처는 기존 수집 조건을 통과하며 이 인스턴스에서 이미지 상태·표시·일반 열기 배치 목적지를 확인했다. **사용자가 제보한 캡처의 명시적 정렬 누락은 실행 재현하지 못했고 해결로 판정하지 않는다.** native 두 정렬·5초 undo·숨김 회귀는 아래 도구·입력 소유권 제한으로 미검증이다. 사용자의 close 호출로 제품 커밋·push는 승인됐다. 캡처 재현과 native 완료 기준이 남아 병합·이슈 종료는 보류한다.

## 출처

- 작업 루트: `/Users/egoinglee/.codex/worktrees/c9f9/agent-gibbon`
- 기준 origin/main: `307f7fde87874763e6c5c5009fd903ae2a6949c0`
- 브랜치: `codex/mixed-note-arrangement`, 계획 HEAD: `2747c3e`
- 이슈 [#191](https://github.com/egoing/agent-gibbon/issues/191), Draft [#192](https://github.com/egoing/agent-gibbon/pull/192)
- `npm run dev:info`: instance `dev-ef9fadde855c7e2c93cbc288`, binary `/Users/egoinglee/.codex/worktrees/c9f9/agent-gibbon/src-tauri/target/debug/agent-gibbon`
- Node v24.21.0 / pnpm 11.25.0 / rustc 1.91.1 / macOS arm64。
- 팀장이 배정한 native 차례 안에서 전용 fixture만 생성했다. 다른 워크트리 실행 결과는 현재 실행 증거로 재사용하지 않았다.

## 원인과 수정

1. `managed_windows`는 외부 child를 가진 웹 host도 포함한다. 그러나 `arrange`는 텍스트·이미지 전용 `memo_id_from_window`로 다시 필터링하여 `terminal-*`, `webview-*`를 버렸다. 명시적 전체 정렬 전용 ID 분류로 네 유형을 포함하며 `webpage-*` child와 캡처 선택 overlay·관리·안내 창은 제외한다. 일반 열기 `reflow`의 대상과 수동 배치 제외 정책은 변경하지 않는다.
2. 웹·터미널에는 Markdown 콘텐츠 높이 측정값이 없어 기존 높이 fallback 48px을 사용하면 본문이 거의 사라진다. 펼친 두 유형에 최소 144·최대 420 논리 px를 사용하고, 바둑판 셀이 최소 높이를 확보하지 못하면 적용 전에 실패한다. 기존 이미지 프레임 비율·텍스트 cap·접힌 24px·화면 부족시 원상태 유지·음수 좌표와 배율 변환을 유지한다.
3. 생성 순서는 텍스트·캡처의 폴더 생성 시각, 생성형 터미널·웹 ID의 nanoseconds를 사용한다. 터미널 metadata는 위치 변경 때 원자 교체되어 file birth time을 쓰면 순서가 바뀔 수 있으므로 사용하지 않는다. 사용자 지정 웹 ID는 현재 세션 생성 시각을 host 교체에도 유지한다. 과거 사용자 지정 웹 ID의 원래 생성 시각은 저장 형식에 없으므로 재시작 후에는 세션 재생성 시각을 사용한다. 이 제한은 디자인 시스템에 명시했다.
4. 명시 정렬 적용시 지연된 본문 측정의 pending 배치를 취소한다. 수동 이동·크기 조절에 따른 자동 배치 제외 flag는 명시 정렬 수집에서 검사하지 않는다. 숨김·최소화·stash 소유 창은 기존대로 제외한다.
5. 실제 명령 매핑은 오른쪽=오른쪽 정렬, 위=바둑판, 왼쪽/Esc=취소다. 별도의 왼쪽 정렬 기능은 추가하지 않았다.

웹 child bounds는 기존 host resize callback, PTY fit은 기존 ResizeObserver/terminal_resize 경로를 따른다. 이 경로의 실제 resize·외부 child·실행 PTY 검증은 하지 않았다.

## 자동 검사

- Rust 전체 238개 통과. 유형별 host/child 구분, 네 유형 두 정렬·경계 내 배치·겹침 없음·이미지 비율·1/1.25/1.5/2배율·음수 좌표·작은 작업 영역의 사전 거절·접힌 viewport, 웹 생성 시각 검사를 포함한다. fixture에서 native 창·입력·숨김·undo를 실제 실행한 테스트는 아니다.
- UI 189개 통과. 기존 웹 Welcome·외부 chrome·터미널·공통 툴바 회귀 범위이며 native geometry 검증과 구분한다.
- Rustfmt, `git diff --check`, Clippy `--all-targets -- -D warnings`, `npm run dev:build` 통과.
- [SVG](../design/directional-arrangement.svg)를 `rsvg-convert`로 [PNG](../design/directional-arrangement.png) 렌더 후 시각 확인했다. 이는 설명용 시안이다.
- ui-ux-pro-max에서 actions feedback 권고와 데스크톱 quick-reference를 검토했다. 기존 취소 affordance·방향명·포커스 보존을 유지하고, 모바일 터치 수치를 데스크톱 창에 적용하지 않았다. 144px은 본 변경의 구현값이며 스킬이 제시한 수치가 아니다.

최종 미커밋 소스의 [파일 해시·검사 요약](assets/issue191/final-checks.json)을 남겼다. Rust 전체 검사는 기존 lifecycle 테스트의 임시 디렉터리/자식 셸 검사도 실행한다. native fixture 터미널에서는 PTY·셸·명령을 시작하지 않았다.

## 실제 CLI 확인과 native 제한

[중간 개발 빌드 CLI 증거](assets/issue191/cli-fixtures.json): PID `32257`의 실행 파일 절대 경로를 `ps`로 대조하고, 네 fixture 모두 `inspect.ok=true`, 예상 instance·ID를 확인했다.

| fixture | 유형 | 관측된 물리 위치·크기 | 상태 |
|---|---|---|---|
| arrange191-text | 텍스트 | 증거 JSON 참조 | 본문 고유 표식·visible, pending=false |
| arrange191-image | 이미지/캡처 fixture | 증거 JSON 참조 | capture 600×400·visible, pending=false, 일반 열기 목적지 존재 |
| terminal-1910001 | 터미널 Welcome | (900,240), 1000×800 | running=false, pid=null, PTY 미시작 |
| web-arrange191 | 웹 Welcome | (1448,414), 1200×800 | welcome=true, URL 없음 |

이미지 fixture는 테스트용 단색 PNG와 저장 메타데이터로 만들었으며 실제 화면 캡처 선택 과정은 실행하지 않았다. 프레임 비율은 툴바 공간을 포함한 이미지 창 크기를 기준으로 한다. CLI open 직후의 `animating`과 `layout_destination`은 일반 열기 예약이며 명시 정렬 완료 증거가 아니다. 이 CLI 실행 뒤 웹 생성 시각 처리를 추가했으므로 이 증거는 최종 소스 전체의 실환경 통과가 아니다.

`stash.shortcut`은 `Ctrl+Alt+Cmd 사용 중 · 트레이로 실행`으로, 전역 입력 소유권은 다른 인스턴스에 있었다. CUA `getApp`에 정확한 위 debug 경로를 전달했으나 `Invalid app`으로 거절됐다. 따라서 자기 native 창 제목·GUI 대상을 검증할 수 없었다. 전역키·CGEvent·AppleScript 등의 우회 입력이나 타인스턴스 종료·lease 변경은 하지 않았다. CUA로 읽은 시스템 설정은 Bluetooth 화면이었으며 설정 변경은 없었다.

네 fixture를 개발 CLI로 숨긴 뒤, PID 32257의 실행 파일 경로를 재확인하고 해당 프로세스만 종료했다. 사용자·타워크트리의 창·포커스·초안·모니터 설정을 조작하지 않았다. native 차례는 팀장에게 반환했다.

## 남은 검증과 위험

- 캡처 누락 제보의 자체 native 재현과 실제 원인 확정.
- 두 정렬의 실제 시작/최종 위치·크기, 이미지 frame 비율, web child bounds, 실행 PTY fit.
- 5초 undo 클릭·만료·중간 숨김/닫힘 및 수동 제외창의 명시적 재정렬.
- native 창 출처 확인과 GUI 대상 지정이 가능한 도구, 해당 인스턴스의 입력 차례가 필요하다. 기존 사용자 앱을 종료해 확보하지 않는다.
- Windows·Intel·실제 혼합 배율/모니터 변경·전체 플랫폼 CI·릴리스는 미실행이다.

Conf 3: 관련 자동 테스트 통과, 변경된 정렬의 native 실행은 미검증. Risk Medium: 일괄 위치·크기와 복원 상태에 영향. 2026-11-15 사용자의 close 호출로 커밋·push·병합 권한은 승인됐다. 다만 캡처 자체 재현·native 완료 기준 미충족으로 병합·이슈 종료를 보류한다.

## native 경로 후속 조사

[기존 경로 조사·조건부 수동 확인 준비안](issue191-native-confirmation-plan.md)에 트레이/CLI/기존IPC의 한계, 자체 캡처 snapshot 상태 분리와 실행 전 조건을 기록했다. 기존 snapshot의 이미지 `animating=true`를 완료 상태로 재해석하지 않는다. 새 앱 실행·사용자 실험 요청·제품 변경·자동 검사 반복은 하지 않았다.

## close 호출 후 보존

제품 소스 두 파일의 SHA256을 최종 검사 요약과 대조해 동일함을 확인했다. 이후 변경은 문서와 기존 증거 재분석뿐이므로 같은 자동 검사를 반복하지 않았다. 원격 main은 기준 307f7fd를 유지했다. 이번 작업 파일만 커밋·push하고 Draft PR·이슈는 미완료 상태로 유지한다. 병합이 없으므로 main 동기화와 워크트리 제거는 수행하지 않는다.
