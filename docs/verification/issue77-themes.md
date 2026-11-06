# #77 관리 창 OTU 테마 검증 (2026-11-06)

> 현재 최종 범위는 독립 메모 디자인·시스템 radio·두 샘플을 포함하며 [최종 후속 기록](issue77-memo-design.md)을 따른다. 이 문서는 당시 구현의 검증 근거로 보존한다.


> 최초 구현의 검증 기록이다. 이후 macOS 제목줄을 추가 통일했으며, 현재 네이티브 구현·고정 로컬 IPC·UI 101개 검사 결과는 [후속 기록](issue77-titlebar.md)을 따른다. 아래 초기 범위·자산 해시는 당시 증거로 보존한다.

관리 창에 흰색·회색·검정 역할별 토큰과 Settings → 테마를 적용했다. 선택은 로컬 자동 저장하며 명시 선택 전에는 시스템 light→gray / dark→black을 추적한다. 첫 paint 전에 저장값을 적용하고 시스템 추적 복귀·손상/차단된 저장소·다른 문서의 storage 이벤트를 처리한다. 저장 실패는 이전 선택을 유지하고 오류를 표시한다. 같은 시스템 색을 클릭해도 명시 선택으로 저장한다.

디자인 기준은 [디자인 시스템](../design-system.md), 세 테마 비교는 [SVG](../design/manager-themes.svg) / [PNG](../design/manager-themes.png)에 있다. 기존 [메모](../design/manager-notes-wireframe.svg) / [이미지](../design/manager-image-wireframe.svg) / [터미널](../design/manager-terminal-wireframe.svg) / [설정](../design/manager-settings-wireframe.svg) / [동의](../design/privacy-onboarding.svg) SVG·PNG도 흰색 기준으로 갱신했다. `rsvg-convert`로 여섯 쌍을 렌더링하고 직접 열어 겹침·잘림·색상을 확인했다. 시안은 실행 캡처와 구분한다.

## 기준과 출처

- 작업 루트: `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/issue-77-otu-themes`
- 초기 HEAD: `30a39e9354b0c7298048d7e9b98af65b29c92012` (초기 clean). 부모 main `59d817e40e586b2e27dd56723db82194b4061c5c`는 이 HEAD의 조상이며 `git merge-base --is-ancestor 59d817e… HEAD` 성공. 추가 병합 없이 최신 관리 창·스킬·개발 약관 생략 기능을 포함했다.
- `npm run dev:info`: root는 위 경로, id `dev-f1d9811e5ba88a2ec84a8faa`, binary는 위 루트의 `src-tauri/target/debug/agent-gibbon`.
- OTU `/Users/egoinglee/dev/project/otu.ai.web/app/globals.css` 앞쪽 팔레트·`docs/core-ui/theme.md`·`app/RootLayoutProvider.tsx`를 읽었다. 사용자 첨부 `orca-paste-1793971774968-cbc6c550-0c7a-47dc-8b59-d9e8aaa5236f.png`(검정), `orca-paste-1793971811244-c98b3c24-2c70-4c9a-ba7a-c30b7a919e4e.png`(회색)을 직접 열어 확인했다. OTU 원본은 수정하지 않았다.
- 환경: macOS ARM64, Node v24.21.0, npm 11.19.0, Rust/Cargo 1.91.1, gh 2.89.0, Orca 1.4.245, rsvg-convert 2.62.3. Chromium 검증은 설치된 Google Chrome과 임시 `/tmp/gibbon-issue77/tools`의 puppeteer-core 25.16.0 사용. 저장소 의존성·lockfile은 변경하지 않았다.

## 자동 검사

| 실행 명령 | 결과 |
|---|---|
| `npm run test:ui` | 99 통과. 신규 4개 상태 회귀: 초기 시스템/실시간 추적, 세 선택의 reload 유지와 reset, 손상·읽기/쓰기 실패, storage 동기화 |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 155 통과 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | 통과 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 통과 |
| `npm run dev:build` | 최신 UI를 포함한 debug 빌드 성공 |
| `node --check ui/manager-theme.js` | 통과 |
| `node scripts/manager-theme-smoke.mjs` | 실제 manager 자산, 3테마×2크기×5화면(30조합). 저장·reload·시스템 이벤트·reset·저장 실패·Space/화살표 조작과 focus, 600px overflow 검사 통과 |
| `node scripts/settings-sidebar-smoke.mjs` | 두 크기·기존 사이드바 키보드·privacy 초안 보존 경로 통과 (IPC mock) |
| `node scripts/skills-ui-smoke.mjs` | 기존 스킬 아코디언·키보드·독립 전환·설치 대기/실패 복구 회귀 확인. 이전 고정 녹색 기대값은 새 success 표면에 맞게 갱신 |
| `git diff --check` | 통과 |

브라우저 명령은 `PUPPETEER_MODULE=/tmp/gibbon-issue77/tools/node_modules/puppeteer-core/lib/puppeteer/puppeteer-core.js`로 실행했고, 테마 스모크에는 `THEME_OUTPUT=/tmp/gibbon-issue77/browser`를 지정했다. HTTP 서버 없이 요청 interception으로 로컬 자산만 제공했다. 테마 스모크의 `terms_accepted=false, terms_skipped=true` IPC mock은 실제 동의 파일을 만들지 않는다. 첫 실행 화면은 mock의 gate만 전환하며 약관·선택 전송 미체크와 Continue 비활성을 확인한다. 테마 스모크에서 `privacy_save`, `manager_set_visibility`, 외부 요청은 0이다. 기존 sidebar/skills 스모크의 IPC도 mock이며 실제 동의·설치가 아니다.

### 대비

OTU 본문/배경/구분선/선택 표면/modal 값은 유지했다. 흰색의 #b0b0b0를 일반 본문에 적용하지 않고 보조·비활성·오류·성공·초점과 조작 경계에 별도 역할 토큰을 썼다. 비활성 회색이 초기 계산에서 4.28:1이었던 부분은 더 어둡게 보완한 뒤 재검사했다.

| 테마 | 본문 최소 | 보조 최소 | 비활성 최소 | focus 최소 | 오류 | 성공 |
|---|---:|---:|---:|---:|---:|---:|
| 흰색 | 12.97 | 5.74 | 5.17 | 12.97 | 7.54 | 5.57 |
| 회색 | 7.47 | 4.90 | 4.69 | 7.47 | 7.54 | 5.57 |
| 검정 | 8.84 | 6.02 | 4.75 | 8.84 | 7.85 | 7.21 |

배경·선택 표면·문서 표면 대비의 최솟값이며 오류/성공은 각 상태 표면 기준이다. 조작 테두리는 배경과 선택 표면에 3:1 이상이다. 원본 구분선은 장식 경계이며 이 기준과 구분한다. native 폼 장식의 픽셀 대비까지 자동 측정했다는 뜻은 아니다. [전체 계산값](issue77-themes/contrast.json)을 남겼다.

[흰색 설정](issue77-themes/browser-settings-white-944.png) · [회색 최소 크기](issue77-themes/browser-settings-gray-600.png) · [검정 설정](issue77-themes/browser-settings-black-944.png) · [메모](issue77-themes/browser-notes-gray-944.png) · [스킬](issue77-themes/browser-skills-black-944.png) · [동의](issue77-themes/browser-welcome-black-600.png) · [문서](issue77-themes/browser-document-gray-944.png)

## 실제 개발 앱과 CLI/GUI 출처

모든 개발 메모 명령은 이 루트의 `./scripts/agent-gibbon-dev`를 사용했다. `open --id issue77-theme-plan`으로 이번 작업의 고유 계획을 만들었고, 이후에는 같은 ID에 `update`만 사용한다. `inspect --id issue77-theme-plan`에서 `ok=true`, 예상 instance, ID와 체크리스트 본문을 대조했다. 이 CLI 성공은 GUI 검증과 별도 근거다.

실제 실행 파일을 `ps -p PID -o comm=`로 확인했다. 검증에서 시작한 PID 46303 → 최신 빌드 PID 59593 → 저장 유지 재실행 PID 65330은 모두 위 binary 절대 경로였다. 종료 전에도 경로를 확인하고 해당 PID만 SIGTERM으로 종료했다. 기존 다른 워크트리 PID 31185는 종료하지 않았다.

Orca computer-use로 각 PID의 메모 창 제목 `Agent Gibbon [DEV egoing/issue-77-otu-themes · f1d9811e] — issue77-theme-plan`과 같은 체크리스트 본문을 읽어 CLI 출처와 연결했다. 관리 창은 같은 PID에 속한 `… — 메모 관리`, native window ID 41281(PID 59593), 41416(PID 65330)으로 지정했다. 최초 메뉴 열기는 해당 PID의 status item 좌표/메뉴를 확인해 실행했으며 포인터와 사용자의 원래 foreground 앱을 복원했다. 이후 창 내부 조작은 background 방식으로 수행했다.

- PID 59593 / 창 41281: 시스템 회색을 다시 선택해 명시 저장되는 경계, 흰색·회색·검정 radio checked 값·저장 문구·실제 색상·키보드 초점 표시를 확인했다. provider AXPress의 성공값만으로 판정하지 않고 후속 tree와 PNG를 읽었다. [회색](issue77-themes/native-gray.png) · [검정](issue77-themes/native-black.png) · [흰색](issue77-themes/native-white.png)
- 전환 전후 고유 메모 inspect를 비교해 note 전체(본문·배경색 포함), visible/focused/position/size/always_on_top이 동일했다. 테마 파일은 메모 원문을 읽거나 IPC·네트워크를 호출하지 않는다. floating memo/Feedback/terminal 자산은 변경하지 않았다. 네 유형 전체의 native 재검증을 의미하지는 않는다.
- PID 65330 / 창 41416: 프로세스 재실행 뒤 첫 화면에 흰색이 적용됐고, 설정에서 white checked=1 및 명시 선택 상태를 확인했다. [재실행 흰색](issue77-themes/native-restart-white.png)
- 같은 창에서 검정 테마의 앱 내 로컬 Terms를 열고 본문·표면을 확인한 뒤 닫았다. [검정 문서](issue77-themes/native-document-black.png)
- 검증 후 시스템 추적 복귀 버튼을 눌러 초기 선호 상태로 돌렸다. 회색 checked=1, 시스템 추적 문구를 다시 읽었다. [시스템 복귀](issue77-themes/native-system-reset.png)
- 최종 diagnostics: terms_accepted=false, terms_skipped=true, onboarding_required=false, sharing_allowed=false. 테스트 약관 수락·전송 동의·진단 전송을 생성하지 않았다.

[출처 및 UI 자산 SHA256](issue77-themes/provenance.json)에 루트·base SHA·instance·binary·PID/창 ID·privacy 상태를 남겼다. 모든 native PNG는 해당 창 캡처이며 다른 워크트리의 과거 캡처를 재사용하지 않았다.

## 범위와 한계

- Conf 5: macOS ARM64에서 세 선택, 저장 후 프로세스 재실행, 로컬 문서, 메모 상태 보존을 실제 확인한 범위. 시스템 색 변화는 Chromium media emulation으로 확인했고 실제 macOS 시스템 설정은 바꾸지 않았다.
- Risk Low: 관리 WebView의 색상·선호와 사이드바 한 항목으로 국소적이며 되돌릴 수 있다. 외부 API·Rust 데이터 저장 구조·메모 팔레트·창 제어 경로는 변경하지 않았다.
- 실제 첫 실행 동의 화면은 이번 native 실행에서 강제 활성화하지 않았다. 동의 화면의 세 테마는 Chromium mock 검증이며 실제 약관 수락/전송·설치 앱·release·Windows·Intel·스크린 리더·장시간 OS 전환은 미검증이다.
- 커밋·push·PR·병합·릴리스 권한이 없어 실행하지 않았으며 전체 플랫폼 CI도 미실행이다. 향후 원격 검사를 실행한다면 새 커밋 SHA에 연결해 결과를 확인해야 한다. 이번 로컬 검사를 원격 CI 통과로 표시하지 않는다.
