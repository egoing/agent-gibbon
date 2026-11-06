# #77 후속: 독립 메모 디자인·시스템 선택·두 샘플 (2026-11-06)

관리 창과 메모 디자인을 별도 선택으로 구성했다. 관리 창은 흰색/회색/검정/시스템, 메모는 흰색/회색/검정/컬러/시스템다. **메모 기본값은 회색**이며 관리 창과 연동하지 않는다. 각 그룹에서 시스템을 선택한 경우에만 주간 회색/야간 검정을 사용한다. 앞선 On/Off 연동안과 시스템이 없는 네 가지 메모 선택은 최신 사용자 요청으로 대체했다.

메모 선택 아래에는 실제 메모를 읽지 않는 `오늘 할 일` / `작업 메모` 고정 예시 두 개를 제공한다. 선택 즉시 해당 역할 세트를 적용하며 컬러에서는 노랑·파랑을 표시한다. 별도 시스템 버튼, 메모 하단 설명, 터미널 팔레트 안내는 제거했다.

[디자인 시스템](../design-system.md) · [최신 시안 SVG](../design/manager-themes.svg) / [PNG](../design/manager-themes.png) · [회색 샘플](issue77-memo-design/browser-preview-gray.png) · [컬러 샘플](issue77-memo-design/browser-preview-color.png) · [600px 화면](issue77-memo-design/browser-settings-600.png)

## 구현

- `ui/appearance.mjs`는 `agent-gibbon.memo-design`만 읽는다. 관리 창 key를 읽지 않으며 없는/잘못된/차단된 값은 회색으로 처리한다. 시스템 선택일 때만 OS media query를 읽는다. 미배포 중간 On/Off key는 사용하지 않는다.
- `ui/theme.mjs`의 공통 역할 매핑을 Memo·Feedback·미리보기에 사용한다. 컬러 선택은 기존 메모의 CLI/자동 팔레트 값을 표시하고 다른 디자인은 OTU 표면/본문/테두리/panel/error 역할을 사용한다. 원문과 저장 색상을 덮어쓰지 않는다.
- Memo/Feedback은 기존 조회에서 설정을 읽어 note revision이 같아도 CSS만 바꾼다. 외형 전환이 fit·이동·포커스·숨김·저장 명령을 발생시키지 않으며 초안과 Feedback 버튼 DOM을 유지한다. 캡처 이미지 픽셀·터미널 팔레트는 변경하지 않았다.
- `ui/manager-theme.js`는 별도 radio 두 그룹을 로컬 저장하고 실패 시 이전 선택을 유지한다. 관리 창의 시스템은 명시 색상 key를 제거하고 native 배경에 현재 OS의 고정 테마 이름만 보낸다. 메모 시스템 선택은 별도 key에 저장한다.
- `ui/memo-preview.js`는 고정 예시 두 개에 같은 역할 함수를 적용한다. 저장 메모 조회·IPC·네트워크 호출 없이 해당 선택과 OS 변화에 따라 CSS를 갱신한다. 세부 문구·checkbox 기호는 예시이며 실제 편집/저장 컨트롤이 아니다.
- 기본 Memo/Feedback CSS도 회색 역할을 사용하고 Toolbar의 키보드 초점에는 전경색 2px 외곽선을 표시한다. 테마/메모 디자인 선택은 동의·전송 상태를 변경하지 않는다.
- README·로컬 사용 스킬·디자인 시스템을 최종 다섯 선택에 맞췄다. 관리 창 시안 SVG/PNG를 갱신했고 Connected Feedback 시안은 컬러 예시로 명시하고 초점 표시를 갱신했다. 두 PNG를 렌더링해 직접 확인했다.

## 자동 검증

| 명령 | 결과 |
|---|---|
| `npm run test:ui` | **109 통과**. 기본 회색·네 가지 고정/컬러 디자인·시스템 낮/밤·관리 창 독립·저장 실패·원래 색상 복귀·초안 보존 |
| `cargo test --manifest-path src-tauri/Cargo.toml` | **155 통과** |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 통과 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | 통과 |
| `npm run dev:build` | 최신 JS/CSS·스킬을 포함한 debug 빌드 성공 |
| `node scripts/memo-appearance-smoke.mjs` | 실제 manager/index/feedback 자산의 별도 같은 origin 페이지. **메모 5종×관리 창 4종(20조합)**, 저장/reload, 시스템 선택 시 낮/밤, 고정 선택의 OS 독립, 두 샘플의 색상, 600px radio/preview·키보드·초안/초점/버튼 DOM 보존 통과 |
| `node scripts/manager-theme-smoke.mjs` | 관리 창 3색×2크기×5화면(30조합), 시스템 radio 추적/해제, 대비·저장 실패·기존 동의 경계 mock 회귀 통과 |
| `node scripts/settings-sidebar-smoke.mjs` | 기존 sidebar·초안·키보드 회귀 통과 (IPC mock) |
| `node scripts/skills-ui-smoke.mjs` | 기존 스킬 accordion/switch·대기/실패 복구·키보드 회귀 통과 (IPC mock) |
| `node --check ui/memo-preview.js`, `node --check ui/appearance.mjs`, `git diff --check` | 통과 |

브라우저는 기존 Chrome과 임시 puppeteer-core 25.16.0을 사용했다. `PUPPETEER_MODULE=/tmp/gibbon-issue77/tools/node_modules/puppeteer-core/lib/puppeteer/puppeteer-core.js`, `APPEARANCE_OUTPUT=/tmp/gibbon-issue77/memo-design-final-browser`로 실행했다. 서버 없이 interception으로 로컬 자산만 제공했다. headless 탭 간 대기는 frame 대신 100ms polling을 사용하고 조작/캡처 시 해당 탭을 선택한다. 테마 스모크의 외부 요청과 note/consent 쓰기는 0이며 IPC는 mock이다. 실제 약관/진단 동의를 생성한 검사가 아니다.

[흰색 메모](issue77-memo-design/browser-memo-white.png) · [회색](issue77-memo-design/browser-memo-gray.png) · [검정](issue77-memo-design/browser-memo-black.png) · [시스템](issue77-memo-design/browser-memo-system.png) · [컬러](issue77-memo-design/browser-memo-color.png)

## 실제 앱과 출처 — 최종 UI 검증과 구분

- 작업 루트: `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/issue-77-otu-themes`
- `npm run dev:info`의 id는 `dev-f1d9811e5ba88a2ec84a8faa`, binary는 위 root의 `src-tauri/target/debug/agent-gibbon`이다. 모든 개발 메모 명령은 이 root의 `./scripts/agent-gibbon-dev`를 사용했다.
- 네 가지 독립 디자인 단계(PID **88357**, 관리 창 **42270**, 메모 **42264**, ID `issue77-independent-probe`)에서 native radio checked 값과 흰색/회색/검정/원래 컬러의 실제 표시를 확인했다. 관리 창을 검정→회색으로 바꿔도 흰색 메모가 유지됐다. 전환 전후 note 전체·저장 색상·visible·position·size·focused·always_on_top·opacity가 같았다. 당시 메모 opacity는 0.5라 캡처 RGB는 원본 색상과 소폭 차이가 있어 정확한 픽셀 일치로 보고하지 않는다.
- [위 단계의 흰색](issue77-memo-design/native-white.png) · [회색](issue77-memo-design/native-gray.png) · [검정](issue77-memo-design/native-black.png) · [컬러](issue77-memo-design/native-color.png)는 **시스템/최종 미리보기 추가 전의 실행 근거**다. 최종 시스템 radio·샘플 UI의 native 통과로 재사용하지 않았다.
- 해당 실행 전에는 각 메모의 편집 Action Bar를 검사해 초안이 있으면 재시작하지 않도록 방어했다. 별도 중단 시점의 재시작 가드는 visible 메모가 없다는 조건에서 종료해 PID 43466을 종료하지 않았으며, 이후 all-hidden 상태에서는 고유 probe만 새로 열었다. 다른 워크트리/설치 앱 프로세스를 종료하지 않았다.
- 최종 UI 빌드 후 기존 개발 앱이 종료돼 있음을 확인했다. 최신 PID **24337**을 시작하고 `ps`의 절대 binary 경로·CLI inspect ok/instance/ID·native 제목/본문으로 출처를 확인했다. ID `issue77-system-preview`, 표식 `SYSPREVIEW77-f1d9811e`, native 메모 창 **42432**. [최신 메모 캡처](issue77-memo-design/native-latest-probe.png).
- **최신 관리 창의 native GUI는 미검증이다.** 해당 PID의 status item 좌표와 AXPress를 확인했으나 메뉴가 열리지 않아 manager 창을 특정·캡처하지 못했다. CGEvent 재확인과 AXPress 대안도 실패했으며, 다른 인스턴스로 바꾸지 않았다. 따라서 최종 시스템 radio/두 미리보기 native 통과나 실제 OS 변경을 주장하지 않는다.
- 최종 diagnostics는 terms_accepted=false, terms_skipped=true, sharing_allowed=false다. 테스트 동의를 만들지 않았다. 개발 메모와 UI 조회가 성공했다는 사실은 native 관리 창 검증을 대신하지 않는다.

[출처·privacy 상태·최종 UI 자산 SHA256](issue77-memo-design/provenance.json)에 최신 실행과 앞선 부분 검증을 구분했다. 도구 환경은 앞선 기록과 같은 macOS ARM64 / Node v24.21.0 / npm 11.19.0 / Rust·Cargo 1.91.1 / gh 2.89.0 / Orca 1.4.245 / rsvg-convert 2.62.3이다.

**Conf 4(최종 UI는 실제 자산 Chromium 검증), Risk Low(로컬 표시 설정이며 원문·저장 색상은 보존).** 네 가지 기본/컬러 디자인의 앞선 macOS 확인은 Conf 5 범위로 별도 기록했다. Windows·Intel·최종 native 관리 화면·실제 OS 설정 변경·최종 디자인의 native 프로세스 재실행 후 선택 유지·전체 플랫폼 CI는 미검증이다. 커밋/push 권한이 없어 전체 플랫폼 검사를 요청하지 않았으며 커밋·push·PR·병합·릴리스도 실행하지 않았다.

## 시스템 항목의 마지막 배치와 오른쪽 정렬 후속

관리 창과 메모의 `시스템`을 모두 DOM/화면/키보드 순서의 마지막에 배치했다. 메모의 컬러는 시스템 앞이다. `margin-inline-start:auto`와 약한 세로 구분선·16px 안쪽 여백으로 고정 선택과 분리한다. 공간이 부족해 줄바꿈돼도 마지막 줄 오른쪽 정렬을 유지한다.

이번 변경은 HTML/CSS 배치다. UI 109개와 실제 자산 브라우저 검증에서 944×480/600×360 두 크기의 그룹별 순서·시스템 오른쪽 경계(내용 영역 끝과 2px 미만 차이)·앞 항목 뒤 배치·동일 행의 간격·overflow 및 기존 키보드/저장/시스템/샘플 동작을 확인했다. Rust 코드는 변경하지 않았으므로 앞선 155개 통과를 유지하며 재실행하지 않았다. 최신 개발 빌드를 수행했다. 최종 native 관리 화면의 미검증 범위는 앞 절과 같다.

[944px 결과](issue77-memo-design/system-right-944.png) · [600px 결과](issue77-memo-design/system-right-600.png). 시안 SVG·PNG를 같은 기준으로 갱신하고 렌더링한 결과도 직접 확인했다.

## 사용자 요청 재실행 후 native 확인 (2026-11-07)

사용자의 재실행 요청으로 이 워크트리 PID 24337만 절대 binary 경로와 편집 초안 부재를 확인한 뒤 종료하고, 최신 빌드를 PID **51969**로 실행했다. 다른 워크트리 프로세스는 종료하지 않았다. 표시 중이던 `issue77-system-preview`만 다시 표시했으며 기존 숨김 상태는 유지했다. 본문·저장 색상·제목·상태·원본 연결과 native 위치/크기·포커스·고정을 비교해 보존을 확인했다. CLI open에 따른 revision/updated_at 갱신은 원문 변경과 구분했다.

관리 창 **42550**을 이전 위치/크기(147,144 / 944×508)로 복원하고 Settings → 테마를 열었다. 이번에는 실제 status 메뉴 열기가 성공했다. CLI inspect ok/예상 instance와 같은 PID/절대 binary/native 제목을 대조했으며, 관리 창 회색 선택과 메모 시스템 선택이 재실행 후 유지된 것을 읽었다. [두 그룹의 시스템 마지막·오른쪽 정렬](issue77-memo-design/native-restart-system-right.png), [독립 메모 그룹과 두 샘플](issue77-memo-design/native-restart-previews.png)을 실제 화면에서 확인했다. 이 범위의 이전 native 관리 화면 미검증은 해소됐다. 실제 OS 낮/밤 변경·600px native·Windows·Intel·전체 CI의 미검증은 유지한다. 현재 약관 수락/전송 허용은 false다. Conf 5는 이번 macOS 표시·재실행·복원 확인 범위다.

### 세로 구분선 제거 후속 (2026-11-07)

사용자 요청으로 시스템 앞 세로 구분선을 제거했다. 마지막 순서·오른쪽 정렬·16px 여백은 유지한다. SVG/PNG를 갱신해 렌더링 확인했고, 944px/600px 실제 자산 브라우저 회귀와 개발 빌드로 확인한다. 실행 중 앱에는 다음 재실행 때 반영된다.
