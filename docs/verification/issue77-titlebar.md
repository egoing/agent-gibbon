# #77 후속: macOS 제목 표시줄 통일 (2026-11-06)

> 현재 최종 범위는 독립 메모 디자인·시스템 radio·두 샘플을 포함하며 [최종 후속 기록](issue77-memo-design.md)을 따른다. 이 문서는 당시 구현의 검증 근거로 보존한다.


사용자 화면의 흰색 네이티브 제목줄이 회색 본문과 분리되어 보이던 부분을 수정했다. 관리 창의 제목줄을 transparent로 하고 해당 창 배경을 OTU 세 테마와 동기화한다. 중복되는 긴 제목은 화면에서 숨기지만 네이티브 창 제목과 개발 인스턴스 식별 정보는 유지한다. 표준 창 장식과 네이티브 이동 영역을 유지한다.

[실제 회색 화면](issue77-titlebar/native-final-gray.png) · [검정](issue77-titlebar/native-black.png) · [흰색](issue77-titlebar/native-white.png) · [시안 SVG](../design/manager-titlebar-themes.svg) / [PNG](../design/manager-titlebar-themes.png) · [디자인 시스템](../design-system.md)

## 구현과 근거

- [Tauri 공식 Window Customization](https://tauri.app/learn/window-customization/#macos-transparent-titlebar-with-custom-window-background-color)의 transparent titlebar 방식을 참고하고 설치된 tauri 2.12.2 / tao 0.37.1 소스를 확인했다. macOS에서 app-wide인 `set_theme`는 사용하지 않고, 현재 manager의 네이티브 Window 배경만 바꾼다. 커스텀 닫기 버튼이나 드래그 영역은 만들지 않는다.
- `src-tauri/src/manager.rs`: macOS builder에 Transparent·hidden_title을 적용한다. `manager_set_theme`는 manager 창만 허용하고 `white/gray/black` enum을 각 원본 배경색에 매핑한다. 메모·창 위치·동의·네트워크는 다루지 않는다. macOS 외에서는 스타일 명령이 no-op이며 기존 제목줄을 유지한다.
- `src-tauri/src/main.rs`: 스타일 명령을 등록한다. 동의 화면에도 같은 외형을 제공하기 위해 이 명령만 동의 전 허용 목록에 추가했다. 명령 내부의 manager 제한과 고정 테마 검사를 유지하며 다른 기능의 약관 경계를 완화하지 않는다.
- `ui/manager-theme.js`: 고정된 테마 이름만 로컬 IPC로 전달한다. 느린 이전 응답이 최신 선택을 덮지 않도록 호출을 직렬화한다. 네이티브 적용 실패는 저장된 테마를 되돌리지 않고 별도 상태 문구로 안내·재시도한다. 메모 원문을 읽거나 외부로 전송하지 않는다.
- 디자인 기준·제목줄 시안 SVG/PNG를 갱신하고 rsvg-convert 2.62.3으로 렌더링한 결과를 직접 열어 확인했다. 기존 client 영역 시안의 구성은 유지한다.

## 자동 검사

| 명령 | 결과 |
|---|---|
| `npm run test:ui` | 101 통과. 네이티브 동기화의 직렬 실행·최신 선택·고정 payload·실패 시 저장 유지/재시도 2개 회귀 추가 |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 155 통과 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 통과 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | 통과 |
| `npm run dev:build` | 최신 UI와 네이티브 변경 포함 debug 빌드 성공 |
| `node scripts/manager-theme-smoke.mjs` | 3테마×2크기×5화면, 저장·시스템 추적·오류·키보드·대비 회귀 통과. native IPC는 mock |
| `node scripts/settings-sidebar-smoke.mjs` | 기존 설정 경로 회귀 통과 (IPC mock) |
| `node scripts/skills-ui-smoke.mjs` | 기존 스킬 경로 회귀 통과 (IPC mock) |
| `git diff --check` | 통과 |

브라우저는 이전 기록과 같은 `PUPPETEER_MODULE=/tmp/gibbon-issue77/tools/node_modules/puppeteer-core/lib/puppeteer/puppeteer-core.js`를 사용했다. 테마 화면 출력은 `THEME_OUTPUT=/tmp/gibbon-issue77/titlebar-browser`. Node/npm/Rust/Cargo/gh/Orca/macOS ARM64 도구 환경은 [최초 기록](issue77-themes.md)과 같다.

## 실제 실행 출처와 결과

- root: `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/issue-77-otu-themes`
- `npm run dev:info`: id `dev-f1d9811e5ba88a2ec84a8faa`, binary `위 root/src-tauri/target/debug/agent-gibbon`.
- 기존 이 작업의 PID 65330만 절대 실행 경로를 재확인하고 종료했다. 새 PID **11157**도 `ps -p 11157 -o comm=`으로 같은 binary를 확인했다. 다른 워크트리/설치 앱을 종료하지 않았다.
- CLI는 이 root의 `./scripts/agent-gibbon-dev`만 사용했다. 고유 메모 `issue77-titlebar-probe`를 inspect해 ok=true, 예상 instance, 본문 표식 `TITLEBAR77-f1d9811e`를 확인했다. 같은 PID의 네이티브 창 제목 `… — issue77-titlebar-probe`와 화면 본문을 대조했다.
- 관리 창은 PID 11157의 native window **41763**, AX 제목 `Agent Gibbon [DEV egoing/issue-77-otu-themes · f1d9811e] — 메모 관리`로 지정했다. 시각적으로 숨긴 제목도 AX 창 제목에 유지됐다. 세 radio checked 값과 저장 문구·실제 화면을 확인했다. native 스타일 오류 상태는 표시되지 않았다.
- retina PNG(1888×960)에서 `(700,20)`(제목줄)과 `(700,150)`(상단 본문)의 실제 픽셀을 대조했다. 흰색은 두 지점 모두 `(248,248,248)`, 회색 `(223,226,221)`, 검정 `(38,38,39)`로 정확히 일치했다. native 초점 없는 상태에서도 흰색 띠가 남지 않았다.
- 표준 close/full screen/minimize 버튼이 native AX tree에 유지됐다. background drag 입력은 전달됐지만 실제 위치가 바뀌지 않아 **실제 드래그 통과로 판정하지 않았다**. 닫기·최소화·최대화 실제 실행도 이번 범위에서 미검증이다. 커스텀 프레임으로 대체하지 않은 구현 근거와 이 실행 한계를 구분한다.
- 관리 창을 사용자가 옮겨 둔 위치·크기 `x=-1329, y=354, 944×480`으로 복원하고 최종 frame을 대조했다. 초기 재시작 때 숨겨진 진행 메모를 잘못 다시 표시해 즉시 close로 숨김을 복원하고 사용자에게 알렸다. 이후 진행 메모는 update만 사용했다. 최종 list에서 기존 두 메모와 새 probe 모두 hidden이다.
- 테마 선택은 검증 후 기존 시스템 추적 상태로 복귀했다. 최종 diagnostics는 terms_accepted=false, terms_skipped=true, sharing_allowed=false다. 약관·전송 동의 기록은 만들지 않았다.

[출처·UI/Rust 자산 SHA256·픽셀·최종 상태 JSON](issue77-titlebar/provenance.json)을 남겼다. 최초 테마 구현의 native 캡처를 최신 네이티브 변경의 실행 근거로 재사용하지 않았다.

Conf 5는 위 macOS 색상·제목·실행 출처와 상태 복원 확인 범위다. Risk Low는 관리 창의 국소 외형 변경 기준이다. Windows·Intel·실제 OS 테마 변경·native 동의 전 실행·전체 플랫폼 CI는 미실행이다. 네이티브 변경이므로 전체 플랫폼 검사가 필요하지만 커밋/push 권한이 없어 요청하지 않았다. 승인 후 검토 커밋의 SHA에 연결해 실행해야 한다. 커밋·push·PR·병합·릴리스는 실행하지 않았다.
