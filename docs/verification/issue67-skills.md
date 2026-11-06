# 스킬 관리 #67 — 2026-11-06

## 결과와 범위

사이드바는 #69 / PR #70으로 main `24f1ea3`에 병합·푸시했고 #69를 닫았다. 이후 현재 브랜치 `egoing/on-off`를 해당 main으로 fast-forward한 뒤 스킬 관리를 구현했다. 핵심 이슈 #67은 이번 후속 작업이다.

- 메모 관리 / 작업 진행 표시의 두 실제 리소스, 초기 Off, 아코디언·설명·On/Off·녹색 표시를 구현했다.
- Codex·Claude Code 감지, 포함 자원 설치, On 덮어쓰기·Off 제거·Rust 로컬 저장, 버전/자원 revision 갱신과 재시도, 저장 실패 rollback·중단 복구를 구현했다.
- 기본 사용 스킬의 지침은 바꾸지 않고 선택 가능한 진행 표시를 별도 자원으로 분리했다. 에이전트를 실행하거나 외부 진단을 전송하지 않는다.
- 스킬 상세 선택·설치 상태 관리 페이지·프로젝트 범위·저장 버튼·예시 안내 문구는 추가하지 않았다. 이슈 첨부 예시 SVG/PNG·HTML을 교체하거나 커밋하지 않았다. 저장소의 기존 설정 시안은 실제 두 항목의 구현에 맞게 갱신했다.

## 자동 검증

| 실행 | 결과 |
|---|---|
| `npm run test` | Rust 141개 + UI 86개 통과 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 통과 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | 통과 |
| `npm run dev:build` | macOS ARM64 최종 빌드 통과 |
| `node --check ui/skills.js`, `node --check ui/manager.js`, `git diff --check` | 통과 |
| skill-creator `quick_validate.py bundled-skills/agent-gibbon-progress` | 통과 |
| `scripts/settings-sidebar-smoke.mjs` | 기존 프라이버시 저장·즉시 철회·메뉴 전환 회귀 통과 |
| `scripts/skills-ui-smoke.mjs` | 944×480 / 600×360의 클릭·Enter·Space·Tab 기반 기본 버튼/아코디언 구조, 접근성 switch 상태, 독립 토글·접힌 활성 표시·오류·중복 실행 차단·가로 넘침 검사 통과 |

브라우저 스모크는 설치된 Chrome 및 기존 puppeteer-core를 `PUPPETEER_MODULE`로 지정해 실행했다. 실제 프런트엔드 자산을 열며 Tauri 호출은 mock이다. JavaScript 오류·외부 요청은 없었다. 상세 네이티브 설치는 아래와 별도로 확인했다. 브라우저 검사는 [펼친 944px](issue67-skills/expanded-944.png)·[600px](issue67-skills/expanded-600.png)·[접힌 944px](issue67-skills/collapsed-944.png)·[600px](issue67-skills/collapsed-600.png)에 남겼다.

Rust 스킬 검사 12개는 다음을 검증한다: 에이전트를 실행하지 않는 감지와 두 대상 설치, 기존 디렉터리·링크 교체 시 외부 원본 보존, 같은 revision의 로컬 수정 보존, 새 revision의 On만 덮어쓰기·Off 유지·실패 후 재시도, 새 에이전트 설치, Off의 소유권·범위, 손상 설정 보존, 다중 대상 저장 실패 시 모두 rollback, 파일 잠금, 중단된 최초 설치 rollback 및 커밋 전후 복구.

## 네이티브 검증 출처

사용자 앱을 재시작하거나 개인 스킬을 시험 설치하지 않기 위해 Orca로 현재 작업의 자식 검증 워크트리를 만들었다. 별도 에이전트는 시작하지 않았다.

| 항목 | 출처 |
|---|---|
| 실제 작업 루트 | `/Users/egoinglee/orca/workspaces/agent-gibbon/floatnote/on-off` |
| 실제 작업 instance | `dev-1206da86e9acf14ddf9c123c` |
| 실제 작업 binary | 위 루트 `src-tauri/target/debug/agent-gibbon` |
| 검증 앱 루트 | `/Users/egoinglee/orca/workspaces/agent-gibbon/floatnote/issue67-native-audit-20261106` |
| 검증 앱 instance | `dev-52f22b348ebf00ad3dacc61e` |
| 검증 binary | 검증 앱 루트 `src-tauri/target/debug/agent-gibbon` |
| 실제 PID | 최초 24534, 같은 버전 재실행 29658, 이전 revision 모의 업데이트 재실행 30558 |
| Manager 창 | PID 24534의 창 39224, PID 30558의 창 39274, 제목 `SKILLS AUDIT — TEST ONLY` |
| 검증 메모 | `skills-audit-provenance-67`, 본문 `SKILLS-AUDIT-67-20261106 — TEST ONLY` |
| 설치 fixture | `/tmp/gibbon-skills-native-fixture-20261106` (정규화 후 `/private/tmp/…`) |

두 루트의 `npm run dev:info`를 기록했다. 검증 루트의 `scripts/agent-gibbon-dev inspect --id skills-audit-provenance-67`에서 `ok:true`·예상 instance·메모 ID·본문을 확인했다. 시작·재실행마다 실제 PID의 실행 파일 절대 경로를 `lsof -a -p PID -d txt -Fn`으로 확인했고, `orca computer list-windows --app pid:PID`의 창 ID·제목으로 Manager를 특정했다. 네이티브 스킬 동작을 원래 작업 앱에서 실행했다고 주장하지 않는다.

제품의 설치·상태·UI·기본 및 진행 표시 리소스는 원래 작업 코드와 SHA-256이 일치한다([파일별 근거](issue67-skills/source-hashes.json)). fixture에서 바꾼 사항은 법적 효력 없는 테스트 약관/안내, 진단 전송 비활성용 운영자 null, 관찰용 Manager 시작 창, debug 전용 `AGENT_GIBBON_SKILLS_TEST_ROOT`다. 원래 사용자 동의 기록을 대신 수락하거나 공식 Sentry를 대상으로 전송하지 않았다. Release는 이 환경변수의 분기를 포함하지 않는다.

검증 폴더의 `bin/codex`·`bin/claude`는 실행하면 실패하는 파일이다. 후보를 실행하지 않고 감지한 뒤 실제 파일시스템의 두 사용자 스킬 루트에 설치되는 것을 검증했으며, 실제 Codex/Claude 세션의 재로딩·인증·스킬 작업 실행을 시험했다는 뜻은 아니다.

## 네이티브 실행 결과

| 순서 | UI·저장·파일 교차 확인 |
|---|---|
| 초기 | 두 summary가 Off. 미확정 테스트 운영자로 진단 체크 비활성·전송 Off |
| 메모 관리 On | 두 대상의 미리 만든 `OLD LOCAL SKILL`을 실제 배포 SKILL.md로 덮어씀. JSON enabled:true, 두 placements·revision, native switch Value:1 일치 |
| 진행 표시 On / 메모 관리 Off | 진행 표시 유지, 메모 관리 두 사본만 제거. 무관한 `unrelated-skill/SKILL.md` 내용 유지 |
| 같은 버전 재실행 | Off/On 선택 유지. 두 진행 표시 사본에 넣은 로컬 수정 문구를 덮어쓰지 않음 |
| 이전 revision 모의 업데이트 | 앱 중지 후 On 항목의 설치 revision만 이전 값으로 바꾸고 재실행. On 두 사본이 현재 포함 리소스로 교체되고 revision 갱신. Off 파일·무관 파일 내용 동일 |
| 설치 경로 오류 | 테스트 Claude 스킬 폴더를 일시적으로 파일로 막고 메모 관리 On 시도. native 오류 Feedback·Off Value:0, 저장 Off 유지, 첫 대상의 기존 파일도 보존 |
| 오류 해소 후 재시도 | 테스트 폴더 복구 후 같은 On이 성공. Feedback 해제·두 스위치 Value:1, 파일·저장 상태 일치 |

중단 복구·저장 실패 rollback은 Rust의 실제 임시 파일시스템 검사로 확인했으며, 네이티브 프로세스를 트랜잭션 중간에 강제 중단한 검증은 하지 않았다. 모의 업데이트는 기록의 이전 revision을 주입한 것으로 실제 배포 파일 교체·서명·설치 관리자 업그레이드 검증과 구분한다.

[최초 On 화면](issue67-skills/native-on.png) · [독립 상태 화면](issue67-skills/native-independent.png) · [실패 Feedback](issue67-skills/native-failure.png) · [성공 후 두 On](issue67-skills/native-both-on.png). 캡처는 실제 WebKit 앱이며 [창·트리·설정·inspect 근거](issue67-skills/native-evidence.json)와 연결한다. AXPress의 `unverified` 응답 자체를 성공으로 판정하지 않고 뒤의 native Value·JSON·파일을 대조했다.

## 조사 근거와 한계

- Orca 설치본 `/Applications/Orca.app/Contents/Resources/app.asar.unpacked/out/main/chunks/service-CWrioPGP.js`의 실행 파일 감지·에이전트별 사용자 루트·로컬 stage/backup 배치 구조를 확인했다. Orca 전체 지원 목록을 이 앱이 지원한다고 표시하지 않는다.
- `DISABLE_TELEMETRY=1 DO_NOT_TRACK=1 npx --yes skills --help`로 agent/global/yes/copy 등의 현재 도움말을 확인했다. 앱의 런타임에는 사용하지 않으며 npx를 이용한 설치·덮어쓰기 지원을 검증했다고 주장하지 않는다. 공식 설치 위치와 오프라인 배포 결정은 [스킬 안내](../skills.md)에 연결한다.
- 현재 작업 소스는 macOS ARM64에서 빌드·검사했다. 사이드바 PR #70의 Windows·Intel CI 통과를 #67 후속 코드의 검증으로 재사용하지 않는다. #67의 Windows·Intel 실행/설치, 실제 VoiceOver 읽기, 실제 에이전트 스킬 재탐색, 배포 업데이트는 미검증이다.
- 초기 검증에서는 원래 개발 앱(PID 70065)과 다른 앱/워크트리를 종료하지 않았다. 이후 사용자 요청으로 다른 인스턴스 종료와 현재 워크트리의 최신 앱 실행을 수행했다. 후속 실행 근거는 아래와 같다.
- 구현 검증 시 기존 `package-lock.json` 미커밋 변경을 보존했다. 최종 정리 요청에 따라 이 파일의 원본·patch·SHA-256을 워크스페이스 밖에 별도로 백업하고 기능 커밋에서 제외한다. 최종 커밋·병합 상태는 #67에 연결된 PR을 따른다.

Conf 4 / Risk Medium. 네이티브 로컬 파일 경로는 실제 확인했고 패키지 업그레이드·플랫폼·실제 에이전트 세션 경계는 위와 같이 남아 있다.

검사 종료 시 native UI로 두 항목을 모두 Off로 전환하고 두 대상의 관리 사본 제거·무관 파일 보존을 확인했다. 정확한 실행 파일 경로로 식별한 fixture PID 30558만 종료했으며 검증 소스·증거는 보존했다.

## 사용자 요청에 따른 최신 앱 실행

사용자가 다른 인스턴스를 종료하고 현재 구현을 실행하라고 명시했다. 실행 파일 절대 경로를 확인한 PID 22648(memo-show-common)·70065(이전 on-off)·70637(기본 체크아웃)을 종료하고, 현재 루트에서 `npm run dev:build` 후 `npm run dev -- --no-watch`로 최신 앱을 실행했다. 이후 실행 파일 조회에서 Agent Gibbon은 현재 루트의 PID 17134 하나만 남았다.

`inspect --id issue67-work-20261106`의 `ok:true`, instance `dev-1206da86e9acf14ddf9c123c`, ID·본문을 대조했다. Manager는 같은 PID의 창 39476, 제목 `Agent Gibbon [DEV egoing/on-off · 1206da86] — 메모 관리`로 특정했다. 설정 → 스킬의 두 실제 summary를 native 트리와 [현재 워크트리 실행 화면](issue67-skills/current-worktree-running.png)에서 확인했다. On/Off와 개인정보 선택은 조작하지 않았다. 기존에 숨긴 main과 진행 메모는 숨김을 유지했다. 이 후속 확인은 최신 UI의 실제 작업 루트 실행이며 설치·업데이트 동작 검증은 위의 격리 fixture 범위를 따른다.
