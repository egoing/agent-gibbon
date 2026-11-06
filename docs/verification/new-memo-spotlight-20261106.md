# 새 메모 스포트라이트 보정 — 2026-11-06

## 원인과 변경

- 기존 `entrance::active`는 새 메모의 커서 옆 대기·등장 이동과 기존 메모의 자동 재배치를 모두 포함했다. 투명도 계산이 이를 모두 강조로 취급하여 기존 메모도 선명해졌다.
- 저장한 수동 투명도는 기존 등장 강조보다 우선했다. 따라서 수동값이 있으면 새 메모가 등장 중에도 반투명으로 표시됐다.
- 등장 종료 때 투명도를 재계산하지 않아 마지막 마우스 이벤트 이후 추가 입력이 없으면 불투명 상태가 남을 수 있었다. 숨긴 WebView 측정 중 이미 진행된 fade도 표시 시점에 초기화하지 않았다.
- 새 메모 등장만 `presenting`으로 판별하고, 등장·hover → 수동값 → 변경 알림 → 접힘/기본값 순으로 목표값을 선택한다. 기존 재배치 전환은 현재 투명도를 높이지 않는다. 수동 설정 자체와 변경 알림의 기존 우선순위는 유지한다.
- 공통 비활성 표시 경로에서 숨긴 메모의 fade를 100%로 초기화한다. 기존에 보이는 창·관리 창·Capture Overlay·Feedback·Stash Indicator에는 초기화를 적용하지 않는다. 등장 완료와 위치 불일치 종료 시 투명도를 동기화하여 추가 입력 없이도 2초 대기·600ms 복귀를 시작한다.
- 저장된 메모·포커스·핀·숨김·자동 배치 규칙은 유지한다. 미커밋으로 존재했던 오른쪽 하단 배치 변경은 보존하며 이 보정의 원인·변경과 구분한다.

[디자인 시스템](../design-system.md#새-메모-스포트라이트-보정-2026-11-06) · [설명용 SVG](../design/global-opacity-wheel.svg) · [PNG](../design/global-opacity-wheel.png).

## 출처

- 작업 루트: `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/bottom-left-memo-placement`
- 기준 HEAD: `30a39e9` (`npm run dev:info`의 현재 브랜치 이름은 `egoing/bottom-right-memo-placement`). 기존 배치 관련 미커밋 변경이 있는 상태에서 이번 세 Rust 파일을 수정했다.
- 원본 개발 instance: `dev-dab6365756cd9981d7d0ff67`
- 원본 binary: `<작업 루트>/src-tauri/target/debug/agent-gibbon`
- 환경: macOS ARM64, Node v24.21.0, rustc 1.91.1. 설치 앱과 다른 워크트리는 종료하지 않았다.
- 초기에는 이 작업에서 시작하지 않은 PID 58138이 실행 중이어서 재시작 승인을 질문했다. 답변을 기다리는 동안 별도 소스 사본으로 검증했다. 이후 원래 PID가 더 이상 실행 중이지 않음을 확인하고, 비어 있는 원본 인스턴스에서도 테스트가 직접 시작한 서버만 사용했다. 원래 PID를 이 작업에서 종료하지 않았다.

## 검사

- `npm test`: Rust 157개·UI 95개 통과. 새 회귀는 수동 미설정·10/35/80/100%, 접힘 여부, 10초 이상 등장 유지, 다른 창의 idle alpha 유지, 도착 후 지연 복귀를 검사한다.
- `npm run check`: Clippy 경고 오류 처리 통과.
- `npm run dev:build`: 원본 워크트리 debug 빌드 통과.
- Rustfmt, Python 구문 검사, SVG XML·렌더·PNG 시각 확인과 diff 검사 통과.

### 실제 창 회귀

`scripts/new-memo-spotlight-smoke.py`는 실행 중인 대상 인스턴스가 있으면 중단한다. 개발 CLI만 사용하고 PID·실행 명령·lsof의 실행 파일을 대조한다. `inspect`의 instance·ID·고유 본문과 같은 PID의 정확한 네이티브 창 제목을 연결한다. **네이티브 alpha는 `inspect` 전에 읽는다.** 실제 alpha 근거는 CoreGraphics 값이며 창 단독 PNG는 콘텐츠·창 식별용이다.

검사 범위: 새 메모 100%, 3초 정지 중 100% 유지, 입력 없이 도착 후 idle 복귀, 다음 메모를 열 때 기존 메모의 반투명 유지, 숨김 갱신·복원, 빠른 연속 열기에서 마지막 메모만 강조. 자동 50%와 수동 35% 두 가지를 확인한다. 포인터·전면 앱·핀·포커스·설정 파일 보존을 확인하고, 원래 설정과 포인터를 복원하며 자신이 시작한 서버만 종료한다. 테스트 메모는 숨긴 상태로 남는다.

- 초기 별도 소스 사본은 `/private/var/folders/ds/zm2jvd217ss_q5jy2bjcbb9c0000gn/T/gibbon-spotlight-3o2lv329`다. root·instance·binary와 PID는 [초기 실행 근거](new-memo-spotlight/evidence.json)에 있다. 자동/35% 각각 8단계를 통과했다. 이후 원본 워크트리에서도 [중간 빌드 검증](new-memo-spotlight/worktree-initial-build/evidence.json)을 통과했다. 이 두 결과는 공통 표시 경로로 초기화를 옮기기 전 결과이며 최종 변경의 근거로 대신 사용하지 않는다.
- 별도 사본의 첫 시도는 연속 열기 중 CLI 오류로 중단했다. 반환 stdout을 보존하지 못해 원인은 미확정이다. [실패 기록](new-memo-spotlight/first-attempt.json)의 `passed:false`와 종료·설정 복원 결과를 보존했다. 재시도 성공과 구분한다.
- 원본의 첫 시도는 startup 후 포인터가 메모 영역 안에 있어 hover 없음 조건에 실패했다. [실패 기록](new-memo-spotlight/worktree-initial-build/first-attempt.json)을 보존했다. 테스트에 준비용 메모를 열고 숨기는 단계를 추가해 저장 메모 복원 완료 후 포인터를 고정했으며 alpha assertion은 완화하지 않았다. 포인터 위치도 이후 근거에 기록한다.

### 최종 빌드의 원본 워크트리 결과

`python3 scripts/new-memo-spotlight-smoke.py --report-dir docs/verification/new-memo-spotlight/worktree`를 최종 빌드에 실행했다. [최종 근거](new-memo-spotlight/worktree/evidence.json)는 `passed:true`, alpha 불일치 0건, 서버 종료·원래 설정 복원 성공을 기록한다. 자동 50%의 PID **69285**, 수동 35%의 PID **70782**에서 각각 8단계를 통과했다. 각 샘플의 PID에 속한 정확한 창 제목·창 ID·CLI instance·본문을 대조했다. 최종 세 Rust 소스와 binary의 SHA-256도 근거에 기록했다.

- 자동 상태: 새 메모 **1.0**, 기존 메모 **0.5**.
- 수동 상태: 새 메모 **1.0**, 기존 메모 **0.35**.
- 두 상태 모두 3초 정지 대기 중 등장 상태 유지, 도착 후 입력 없이 idle 복귀, 숨김 복원과 연속 열기의 마지막 메모 강조를 통과했다.
- [자동 상태 창 단독 PNG](new-memo-spotlight/worktree/auto-new.png) · [수동 상태 창 단독 PNG](new-memo-spotlight/worktree/manual-new.png)를 시각 확인했다. 단독 PNG는 투명 합성 측정의 대체 근거가 아니다.

검사 뒤 원래 투명도 설정으로 원본 개발 앱을 새로 실행했다. PID **73070**의 확인용 메모 `new-memo-spotlight-result-20261106`가 `ok:true`, 예상 instance, 고유 본문, 표시 직후 opacity **1.0**을 반환했다. [현재 실행 기록](new-memo-spotlight/final-running.json)에 출처를 남겼다. 기존 진행 메모는 숨김 상태를 유지하며 완료 결과와 커밋 승인 후 세 플랫폼 CI 대기를 갱신했다. 다른 메모를 자동으로 다시 열지 않았다. 이후 [동일 PID의 네이티브 창](new-memo-spotlight/final-native.json)에서 창 ID 41619와 도착 후 정상 idle alpha 0.5를 확인했다.

### 이전 코드 재현

같은 별도 소스 사본에서 이번 세 Rust 파일만 기준 HEAD로 되돌려 다시 빌드하고, 나머지 배치 변경은 그대로 유지했다. `--diagnostic-continue`로 실패를 수집했으며 최종 종료 코드는 의도대로 1이다. [기준 코드 실행 근거](new-memo-spotlight/baseline/evidence.json)는 `passed:false`이며 수동 35%의 새 메모·3초 대기·숨김 복원·연속 열기에서 실제 alpha가 모두 약 **0.35**였다. 자동 투명도에서 기존 메모가 숨김 복원에 따른 재배치 중 **1.0**으로 선명해지는 것도 기록했다. 기준 코드가 새 동작을 통과한 것으로 보고하지 않는다.

최종 빌드 재검사 중 전면 앱의 시작 전후 차이([기록](new-memo-spotlight/worktree/startup-front-failure.json))와 검사 중 실제 포인터 이동([기록](new-memo-spotlight/worktree/pointer-moved-during-hold.json))으로 중단한 시도가 있다. 포인터 이동 기록에서는 새 메모가 이미 목적지에 도착해 fade 중이었다. 제품의 정지 대기 실패와 구분하며, 전면 앱 비교의 기준을 준비용 메모를 닫은 뒤로 옮겼다. 정지 대기 단계는 등장 상태가 여전히 활성인지도 확인하도록 강화했다.

## 한계

- 실제 GUI 범위는 macOS ARM64의 일반 텍스트 메모다. Windows·Intel, 메뉴/Dock 직접 복원·영역 생성·이미지·터미널·전체 접어두기 조합의 GUI는 미검증이다. 해당 코드 경로 전체를 통과로 간주하지 않는다.
- 기존 사각형 기반 hover 판정과 여러 화면의 좌표 정책은 변경하지 않았다.
- 커밋 승인을 받기 전이므로 원격 push와 세 플랫폼 수동 CI는 미실행이다. 창 제어 변경이므로 커밋·push 후 대상 SHA의 전체 검사가 필요하다. 배포 태그와 공개 릴리스는 생성하지 않는다.
- Conf 5는 기록된 macOS 실제 창 범위에 한정하며 Risk Low다.

## 통합 진행 — 사용자 승인 후

사용자가 이 워크스페이스의 커밋·병합·push·이슈 기록·종료를 승인했다. 배치 변경과 스포트라이트 보정을 별도 커밋으로 보존한 뒤 같은 작업 브랜치에서 전체 플랫폼 검사를 요청한다. 앞의 승인 대기 설명은 최초 로컬 검증 시점의 상태이며, 원격 검사 결과와 대상 SHA는 후속 통합 기록으로 구분한다.
