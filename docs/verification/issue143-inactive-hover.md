# #143 비활성 창의 접힌 툴바 호버

## 기준과 구현

- [디자인 시스템](../design-system.md)의 최신 기준에 따라 활성/비활성 × 펼침/접힘 모두 버튼 배경을 전경색 16% 혼합으로 강조한다. 포커스 제한과 blur 시 강조 제거를 없앴다.
- 비활성 WKWebView가 DOM hover 이벤트를 전달하지 않는 경우를 보완하기 위해 기존 macOS 접촉 감지 → `sync_pointer` 경로에서 창별 `toolbar-pointer` 이벤트를 전달한다. AppKit의 논리 좌표와 콘텐츠 사각형으로 DOM 좌표를 계산하므로 주 모니터와 창의 backing scale을 섞지 않는다. 좌표가 바뀌었을 때만 해당 창에 전달하고 창 밖/숨김은 null이다. 좌표를 디스크에 저장하거나 외부 전송하지 않는다.
- 실제 DOM hit-test로 SVG 자식, 제목/드래그 영역, 숨김 도구와 비활성 버튼을 구분한다. native 활성화·입력 합성·첫 클릭 설정을 추가하지 않았다. 클릭·드래그 핸들러와 버튼/아이콘 크기·위치는 유지한다.
- [SVG](../design/toolbar-reserved-space.svg)와 [PNG](../design/toolbar-reserved-space.png)는 접힌 펼치기 버튼의 강조와 비활성 공통 정책을 표시한다. 설명용 시안이며 실제 앱 캡처가 아니다.

## 실행 출처

- 작업 루트: `/Users/egoinglee/.codex/worktrees/438f/agent-gibbon`
- 브랜치: `codex/143-inactive-collapsed-hover` (시작 기준 origin/main `eb5df329215e828a81028df5e4574465443adcf3`)
- `npm run dev:info`: root는 위 경로, instance `dev-46314bac3eb60389ce2c6b0f`, binary `/Users/egoinglee/.codex/worktrees/438f/agent-gibbon/src-tauri/target/debug/agent-gibbon`
- Node v26.0.0, rustc 1.91.1, macOS ARM64.
- 처음 실행 중이던 이전 개발 PID 68509는 작업 중 이미 종료된 상태임을 ps로 확인했다. 이번 작업에서 그 프로세스를 종료하지 않았다. 최신 코드의 `npm run dev:build` 성공 후 개발 CLI로 새 프로세스를 시작했다.
- 새 PID 62636의 `--serve` 실행 파일 절대 경로를 ps로 확인했다. 검증 전용 메모 `verify-143-438f`, 본문 고유 표식 `호버 검증 143-438f`와 CLI inspect의 `ok:true`·instance 일치를 확인했다.
- GUI 접근은 `orca computer list-windows --app pid:62636 --json`이 `windows:[]`를 반환했다. 이전 PID도 빈 목록이었고 CUA의 실행 파일 경로 선택은 `Invalid app`이었다. 창 제목과 PID를 연결하지 못했으므로 GUI 조작·전체 화면 캡처는 수행하지 않았다. 다른 워크트리나 설치 앱으로 대체하지 않았다.

## 실행 결과

| 검사 | 결과 |
| --- | --- |
| `npm run test:ui` | 183개 통과. DOM pointer 진입·이탈, 포커스 상실, native 좌표 이벤트의 활성/비활성 × 펼침/접힘, 버튼 간 전환·빈 영역·disabled·창 이탈 포함 |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 213개 통과. AppKit 좌표의 소수점·음수 화면 원점·접힌 24px 경계 포함 |
| `npm run check` | Clippy 경고 오류 처리 통과 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | 통과 |
| `npm run dev:build` | 통과. 최신 UI를 포함해 다시 빌드한 뒤 위 PID로 실행 |
| `git diff --check` | 통과 |
| SVG → PNG | `rsvg-convert` 렌더·육안 확인. 설명이 잘리지 않도록 시안 높이 조정 |
| 개발 CLI 상태 보존 | 전용 메모 update 전후 instance·focused(false)·visible·position·size·always_on_top·folded 일치. 종료 시 검증 메모만 close 후 visible:false 확인 |

Chromium은 `/tmp/gibbon-toolbar-browser/verify143.cjs`에서 현재 UI 자산을 route하고 Tauri 이벤트를 stub으로 전달해 검사했다. 메모·터미널·웹 창 각각에서 펼침 400×260과 접힘 400×24의 토글/닫기 슬롯 24×22, SVG 12×12, y=1 및 두 상태의 동일한 좌표를 확인했다. focus true/false stub 각각에서 native 좌표 → DOM hit-test → 배경 강조 → null 이탈 시 원래 배경 복귀를 확인했다. 물리 포인터와 실제 비활성 WebView의 실행 증거로 사용하지 않는다.

캡처(임시 경로): `/tmp/gibbon-toolbar-browser/143-index.html.png`, `143-terminal.html.png`, `143-web.html.png`.

## 한계와 수동 확인

- 실제 macOS 물리 포인터의 비활성 호버 진입·이탈, 최초 한 번 클릭의 펼치기/닫기, 접힌 창 드래그 및 다중 모니터 GUI는 미검증이다. 현재 도구에서 개발 창을 특정할 수 없었기 때문이다. CLI의 포커스 상태 보존은 이 GUI 검증을 대신하지 않는다.
- Intel/Windows의 실행·GUI 및 전체 플랫폼 설치 파일 빌드는 실행하지 않았다. 이번 요청은 일반 개발·병합 범위이며 별도 전체 플랫폼 검사나 배포 요청이 없다. 버전·태그·릴리스는 변경하지 않는다.
- 수동 확인: 펼친 메모의 버튼 hover → 접기 → 다른 앱 활성화 → 펼치기와 닫기에 차례로 포인터를 올리고 이탈 → 현재 앱 포커스 보존과 배경 복귀 → 한 번 클릭 펼치기 → 접힌 제목/빈 공간 드래그 순서로 확인한다.
- 코드/로컬 회귀 검증 Conf 3, 브라우저 레이아웃 검사 별도 통과, Risk Low(시각적 강조의 국소 변경). 검증 코드 SHA·PR·병합 결과는 이슈 #143과 PR에 기록한다.
