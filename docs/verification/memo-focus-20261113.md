# 활성 메모 테두리와 둥근 모서리 검증 — 2026-11-13

## 변경과 원인

- 회색 상시 선은 `#memo-surface`의 1px 실선과 Feedback 연결 시 별도로 그리는 SVG 외곽선이었다. 표면의 기존 1px 공간을 투명하게 유지하고 연결 외곽선을 숨겼다. native 창 포커스에만 3px inset shadow overlay를 표시한다. overlay는 포인터 이벤트와 레이아웃에 참여하지 않는다.
- 모서리는 표면의 `border-radius`·`overflow:hidden`만으로 하위 콘텐츠를 자르던 경로에 명시적인 둥근 `clip-path`를 추가했다. 기존 네이티브 투명 창과 html/body 투명 배경을 유지했다. 수정 후 실앱 캡처의 네 모서리 픽셀 알파가 모두 0이어서 사각 배경 없이 라운드를 유지할 수 있었다. 사용자 첨부의 사각 영역이 특정 WebKit 합성 레이어에서 기원했는지까지 분리 재현하지는 않았으므로 그 세부 원인을 확정하지 않는다.
- native 포커스 이벤트를 구독한 후 초기 상태를 조회한다. 조회 도중 더 최신 이벤트가 오면 초기 결과를 버린다. 초기 조회에 필요한 `core:window:allow-is-focused` 읽기 권한만 추가했다.
- 공유 표면을 사용하는 텍스트·이미지·터미널에 적용한다. 별도 네이티브 프레임의 웹 창에는 적용하지 않는다. Feedback 자체의 기존 테두리와 버튼 키보드 포커스 표시는 유지한다.

## 실행 출처

- 작업 루트: `/Users/egoinglee/.codex/worktrees/02ec/agent-gibbon`
- `npm run dev:info`: instance `dev-f3ab8dc6a8d44e989b85031f`, binary `/Users/egoinglee/.codex/worktrees/02ec/agent-gibbon/src-tauri/target/debug/agent-gibbon`
- Node v26.0.0, Rust 1.91.1, macOS ARM64. 최초 작업 트리에는 미커밋 변경이 없었다.
- 이 워크트리에서 `npm run dev:build` 후 `scripts/agent-gibbon-dev open`으로 시작했다. 권한 반영 재빌드 뒤 직접 시작한 이전 PID 59446만 종료하고 재시작했다. 최종 PID **60335**의 실행 파일 절대 경로를 `ps`로 확인했다.
- 검증 메모 `focus-02ec-proof`, 고유 본문 `FOCUS-02EC-20261113 모서리 검증`. `inspect`의 `ok:true`·예상 instance·ID·본문을 대조했다.
- Orca의 PID 지정 접근성 트리와 ScreenCaptureKit 창 ID **67086**, 네이티브 제목 `Agent Gibbon [DEV detached@f343dbe · f3ab8dc6] — focus-02ec-proof`를 연결했다. 캡처에도 고유 본문이 보인다. 기본 앱과 다른 워크트리에는 명령을 보내지 않았다.

## 결과

- `npm run dev:build`: 성공(최종 권한 포함 재빌드 성공).
- `npm run test:ui`: **185/185** 통과. 새 포커스 테스트 2개는 native 이벤트의 활성/비활성 반영과 초기 비동기 샘플의 역전 방지를 검사한다.
- `node --check ui/memo-focus.js`, `git diff --check`: 통과.
- `cargo test --manifest-path src-tauri/Cargo.toml`: **213/213** 통과.
- 실제 마우스 입력으로 메모를 활성화하고 CLI `focused:true`를 확인한 뒤 캡처했다. 이전 앱으로 포커스를 복원하고 `focused:false`를 확인한 뒤 비활성 캡처를 얻었다. GUI 도구의 background 클릭만으로는 활성화를 확인할 수 없어 native 마우스 입력을 사용했다. `screencapture` 직접 창 캡처는 실패했으며 Orca ScreenCaptureKit 경로로 캡처했다.
- 활성/비활성의 물리 좌표 `(3456,1662)`와 크기 `600×114`가 동일했다. 캡처에서 본문 위치가 같고 활성일 때만 3px 안쪽 선이 나타난다. 두 캡처 모두 네 모서리 RGBA 알파 **0,0,0,0**이다.
- 편집 상태에서도 활성 테두리를 시각 확인했다. Feedback 자체의 경계는 유지된다. 검증 후 편집을 취소했으며 본문은 바꾸지 않았다.
- SVG 두 개를 `rsvg-convert`로 PNG 렌더링하고 열어 확인했다.

[활성 실앱 캡처](assets/memo-focus-20261113/active.png) · [비활성 실앱 캡처](assets/memo-focus-20261113/inactive.png) · [출처·기하학 JSON](assets/memo-focus-20261113/evidence.json)

## 범위와 한계

- 실제 검증은 macOS ARM64 텍스트 메모와 편집 상태다. 이미지·터미널·접힌 메모·테마별 실화면, Windows·Intel은 미검증이다. OS별 전체 빌드·설치·배포는 실행하지 않았다.
- 사용자 첨부의 사각 영역을 원본 빌드에서 별도로 재현한 대조 캡처는 없다. 현재 수정본의 네 모서리 투명도와 실화면을 검증했다.
- 기존 앱·다른 워크트리 프로세스 및 사용자 메모는 보존했다. 전용 진행 메모는 완료 결과를 남겨 유지하고, 검증 메모만 숨긴다.
- Conf **5**(macOS 텍스트 실앱 범위), Risk **Low**. 커밋·푸시·병합·배포하지 않았다.
