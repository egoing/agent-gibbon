# #164 편집 저장·툴바 여백 검증

2026-11-14 (Asia/Seoul). [이슈 #164](https://github.com/egoing/agent-gibbon/issues/164) · [Draft PR #165](https://github.com/egoing/agent-gibbon/pull/165).

## 변경과 출처

- root `/Users/egoinglee/.codex/worktrees/21c1/agent-gibbon`, branch `codex/editor-save-layout`, 기준 main `53ab3a7`, 계획 커밋 `d685c77` 위의 미커밋 구현.
- `npm run dev:info`: instance `dev-04a8bd2447daf04ae0f34710`, binary `/Users/egoinglee/.codex/worktrees/21c1/agent-gibbon/src-tauri/target/debug/agent-gibbon`.
- Node 24.21.0, Rust 1.91.1, macOS arm64. #159의 편집 높이 변경은 포함하지 않는다.

Feedback은 취소·저장 두 버튼으로 바꾸고 저장 중/완료 문구도 통일했다. 편집 중 상단 내보내기를 hidden 처리하며 exportNote 진입에서 편집 상태를 확인해 오래된 Feedback 요청까지 무시한다. 읽기·이미지 export는 기존 null markdown 경로를 유지한다.

녹색 표시 여백은 공통 30px 툴바 행의 버튼 아래 8px와 viewport 상단 패딩 3px다. 편집 상태에서만 22px 행·상단 패딩 0으로 변경한다. 버튼 크기·가장자리·내부 입력 패딩과 native 드래그 처리 코드는 유지한다.

## 실행 결과

- `npm run test:ui`: 196개 통과. 저장 단축키, revision 충돌, 읽기/이미지 내보내기, stale Feedback 및 숨긴 toolbar 내보내기 방어, 편집 툴바 빈 공간 드래그를 포함한다.
- `PLAYWRIGHT_MODULE=/tmp/gibbon-playwright/node_modules/playwright/index.mjs node scripts/editor-save-smoke.mjs`: Chromium 158.0.8127.18 통과. Feedback 144/300px의 두 버튼·Tab·접근성 이름·단축키·오류 후 focus, 편집 144/300/600px의 toolbar 22px 및 textarea top과 row bottom 간격 0px, 이미지 행 30px 유지 확인. IPC mock 검사다. 캡처 `/tmp/gibbon16-browser/editor-{144,300,600}.png`.
- `cargo test --manifest-path src-tauri/Cargo.toml`: 222개 통과.
- `npm run dev:build`, `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`, `git diff --check` 통과.
- SVG를 `rsvg-convert`로 PNG 렌더링하고 시안 및 브라우저 캡처를 육안 확인했다.

## 실제 앱 확인

팀장 GUI 슬롯 승인 후 전용 CLI로 `issue164-21c1`과 `/tmp/gibbon164-note.md`만 생성했다. inspect의 ok·instance·메모 ID·고유 본문을 대조했다. 실제 프로세스 PID 99402의 정확한 binary 경로를 확인하고 AX/CG에서 native title `Agent Gibbon [DEV codex/editor-save-layout · 04a8bd24] — issue164-21c1`, 본문 window 71413·Feedback 71421을 특정했다.

Orca는 runtime metadata 부재로 사용 불가여서 저장소의 PID-bound AX helper `scripts/design-system-ax.swift`를 사용했다. 읽기 AX tree는 숨긴 요소를 포함하므로 내보내기 제거의 시각 판단에는 사용하지 않았다.

1. AX 메모 편집 버튼 실행 후 [실제 창 캡처](assets/issue164-editor-native.png)를 `screencapture -x -l 71413`으로 얻고 육안 확인했다. 본문+연결 Feedback이 포함되며 내보내기 아이콘 없음, 밀착 편집기, 취소·저장 두 버튼을 확인했다.
2. AX draft로 `issue164-21c1 저장된 본문`을 입력하고 Feedback `저장 · Command S` 버튼 실행. inspect revision 1→2, 저장했습니다 안내와 연결 파일의 동일 본문 확인.
3. 다시 편집하여 `issue164 충돌 초안` 입력 후 외부에서 전용 원본 파일 변경. 같은 저장 버튼 실행 시 원본 외부 변경 오류, 메모 revision 2 유지·외부 파일 보존 및 AX textarea의 초안 유지 확인.
4. 취소 후 개발 CLI close로 검증 메모·Feedback 숨김을 확인하고 정확한 PID 99402만 종료했다. 팀장에게 GUI 슬롯 해제를 보고했다. 다른 인스턴스와 사용자 메모는 변경하지 않았다. 입력은 AX만 사용했고 포인터·전역 키·시스템 볼륨은 변경하지 않았다.

## 한계

실제 키보드 Cmd/Ctrl+S·실제 toolbar drag·읽기/이미지 native export 완성은 이번에 재실행하지 않았다. 해당 동작의 자동 회귀검사와 실제 저장 버튼·원본 충돌 검증을 구분한다. Intel·Windows GUI, 전체 플랫폼 CI·설치·배포는 미실행이다. 최종 구현 커밋·push·병합은 승인 전이며 Draft PR에는 계획 커밋만 있다.

Conf 5: macOS 화면·저장 버튼·원본 충돌 / Conf 3~4: 나머지 자동 검사. Risk Low: 국소 UI·진입 제한이며 저장 백엔드는 변경하지 않음.

## 편집 스크롤바 후속

사용자 후속 요청에 따라 같은 작업에서 편집 textarea의 스크롤바를 읽기 overlay로 통일했다. 공통 컨트롤의 대상만 바꾸며 textarea 기본 입력·커서·선택·scrollTop을 유지한다. input·selectionchange·resize를 반영하고 전환/접힘 시 기존 드래그를 종료한다. 접근성 제어 대상·명칭도 textarea/viewport에 맞춰 바뀐다. 네이티브 scrollbar 열을 제거하고 forced-colors에서는 복귀한다.

- 위 Chromium 스크립트를 확장해 144/300/600px 각각 긴 글 입력, Control+End 커서 끝 이동·추가 입력과 selectionEnd, scrollbar Home·thumb drag로 textarea만 이동, 짧은 글에서 숨김, 읽기로 전환 후 viewport End 이동, forced-colors의 scrollbar 숨김·textarea native 복귀를 검사했다. 출력 `/tmp/gibbon164-scroll`, 실제 pointer drag는 Chromium에서 실행했다.
- UI 196개 및 수정 모듈 syntax 검사 통과. Rust 코드는 변경하지 않았으며 앞선 222개 결과는 후속 JS 이전 결과다. 개발 빌드 재실행 통과(6.06초). 최종 드래그 종료 가드 보강은 이후 브라우저 재검증으로 확인했다.
- 실제 앱을 다시 시작한 PID 5660의 정확한 binary 경로와 `dev-04a8bd2447daf04ae0f34710` inspect를 재대조했다. 전용 파일 `/tmp/gibbon164-scroll-note.md`, 메모 `issue164-scroll-21c1`, 고유 본문 `issue164-21c1 편집 스크롤 검증`, native title `Agent Gibbon [DEV codex/editor-save-layout · 04a8bd24] — issue164-scroll-21c1`과 CG window 71429를 확인했다.
- PID-bound AX helper로 편집 진입, AX의 `편집기 세로 스크롤` 노출을 확인했다. [실제 편집 화면](assets/issue164-editor-scroll-native.png)은 window 71429의 screencapture이며 넓은 native scrollbar 열이 제거된 상태를 육안 확인했다. 비활성 fade 상태여서 thumb는 보이지 않는 캡처다. thumb의 표시·드래그와 입력/커서 추적은 Chromium 증거로 구분한다.
- AX로 짧은 글을 넣고 취소한 뒤 전용 CLI close 및 이번 PID 5660만 종료하고 GUI 슬롯을 반납했다. 실제 물리 입력·native thumb 드래그·IME 조합 입력은 미실행이다. Windows/Intel GUI 및 전체 플랫폼 CI도 미실행이다.
- SVG/PNG의 편집기 thumb 시안을 갱신·렌더링·육안 확인했다. 최종 제품 변경은 계속 미커밋으로 보존한다.

후속 스크롤 동작 Conf 4(Chromium), 실제 native 화면 Conf 5 / Risk Low.

## 기본 설명 제거 후속

사용자 요청으로 편집 시 `원본 파일에 저장` 기본 안내를 제거했다. 연결 파일 유무에 관계없이 기본 Feedback은 취소·저장 버튼만 있으며 실제 오류·저장 결과는 유지한다. 최종 개발 빌드 통과(5.25초), UI 197개 통과(연결 메모 기본 message 빈 값 및 실제 notice 유지 검사 추가). 시안은 이미 버튼만 표시하므로 추가 형상 변경은 없고 디자인 시스템에 명시했다.

위 native 캡처 두 개는 이 안내 제거 **이전**의 실행 증거다. 안내 제거 후 native 화면을 다시 캡처한 것으로 간주하지 않는다. 이번 최종 기본 안내는 UI 검사와 source 검토로 확인했다.
