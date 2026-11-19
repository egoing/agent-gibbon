# macOS 앱 전환 시 관리 창 복원

## 구현과 이벤트 구분

`NSApplicationDidBecomeActiveNotification`을 Ready 이후 한 번 관찰한다. Tao의 delegate 교체나 전역 키보드 감시·입력 권한 추가는 하지 않는다. 기존 `manager::open`의 고정 창 label을 재사용하므로 창이 있으면 복원하고 없으면 생성한다. 기존 페이지·선택 상태는 유지한다.

활성화 당시 마우스 버튼이 눌렸으면 제외한다. 메인 큐의 다음 실행 시점까지 로컬 클릭·키 입력, 비활성화 또는 포커스 보존 처리로 세대가 바뀌면 요청을 취소한다. CLI 창 생성 전에 시작하고 표시 후 끝내는 기존 focus 경로가 이 억제 처리를 호출한다. 시작 시점의 활성화는 관찰하지 않는다.

최소화 애니메이션 중 Tao의 set_focus가 생략되는 경우 native 완료 알림에서 관리 창을 key로 만든다. 새 입력·앱 전환 후에는 취소하며 타이머 재시도는 없다. updater 설치 예약과 충돌하는 자동 열기도 수행하지 않는다.

### 구분할 수 없는 범위

Apple의 활성화 알림에는 `userInfo`가 없으며 ⌘Tab 출처를 식별하는 필드가 없다. 이 구현은 **⌘Tab 전용 검출이 아니라 비마우스 앱 활성화 처리**다. 다른 앱의 activate 요청이나 시스템 재활성화를 모두 구분하지 못한다. 이미 활성인 앱을 선택해 활성화 알림이 발생하지 않는 경우는 처리할 수 없다. Dock Reopen은 별개이며 기존 메모 표시 동작을 유지한다.

직접 클릭과 활성화의 실제 이벤트 순서, WKWebView의 비동기 활성화와 focus 복원 순서는 아직 실환경에서 검증하지 않았다. 따라서 모든 포커스 회귀가 방지됐다고 보증하지 않는다. 특히 다음 main queue 실행 뒤에 발생하는 별도 비동기 프로그램 활성화는 CLI 억제 범위를 벗어날 수 있다.

공식 근거: [활성화 알림](https://developer.apple.com/documentation/appkit/nsapplication/didbecomeactivenotification), [Reopen Apple event](https://developer.apple.com/documentation/coreservices/apple_events/1527223-event_id_constants), [마우스 버튼 상태](https://developer.apple.com/documentation/appkit/nsevent/pressedmousebuttons). 로컬 Tauri 2.12.2의 Reopen은 applicationShouldHandleReopen이며, Tao 0.37.1은 최소화 상태에서 set_focus를 생략한다.

## 실환경 시험 상태

### 최종 개발 빌드 출처

- 워크트리: `/Users/egoinglee/.codex/worktrees/567c/agent-gibbon`
- 브랜치: `codex/cmd-tab-manager`
- HEAD: `d29579230ddb9ae75bae6872ac2e16447fdb1a65`
- HEAD 위의 이번 미커밋 제품 변경을 포함하여 빌드했다. HEAD만의 산출물이 아니다.
- 인스턴스: `dev-5fee2b32fc1a5fe9eb5b292d`
- 바이너리: `/Users/egoinglee/.codex/worktrees/567c/agent-gibbon/src-tauri/target/debug/jajan`
- 바이너리 SHA-256: `d14b94cdec6da5e875f0833dc6d7855f5ae7b43bfb485c2f8303b445b8863808`
- 명령: `npm run dev:build` — 최종 실행 성공, 경고 없음. 첫 실행의 esbuild 미설치는 pnpm 11.25.0의 frozen-lockfile 설치로 해결했다. lockfile 변경 없음.
- 실행 앱에 자동 반영됐다고 보지 않는다. 이 작업에서는 시험 앱을 시작하지 않았다. 이전 개발 프로세스가 별도로 있다면 정확한 출처 확인 후 정상 재시작이 필요하다.

```sh
cd /Users/egoinglee/.codex/worktrees/567c/agent-gibbon
./scripts/agent-gibbon-dev open
```

### 미실행 범위

- 작업 Mac: arm64, macOS 15.6.1 (24G90), Node 24.21.0, pnpm 11.25.0.
- Computer Use 로컬 앱 목록 조회 가능. 실제 시험 앱 조작은 하지 않았다.
- 지정 iMac GUI 시험: **BLOCKED** — 자동 시험 중지 상태를 존중해 요청·실행하지 않았다. 다른 호스트로 대체하지 않았다.
- 단위·통합 테스트, 정적 검사, 모의 브라우저 테스트: **NOT RUN** — AGENTS.override.md의 시험 범위에 따라 생략.
- 진행 메모: 착수 시 이 워크트리 개발 바이너리가 없어 표시 불가. 설치 앱으로 대체하지 않았다.

## 시험 재개 시 확인 기준

1. 다른 앱에서 ⌘Tab으로 선택: 관리 창 신규 생성·키 포커스.
2. 관리 창 표시·숨김·최소화 상태에서 반복: 같은 native 창 재사용·최소화 복원·키 포커스, 선택·설정 상태 유지.
3. 비활성 메모 직접 클릭 및 빠른 클릭: 해당 메모 입력 유지, 관리 창이 앞을 가로채지 않음.
4. 다른 앱에서 CLI 새 메모 생성·기존 메모 표시·갱신: 원래 앱 포커스 유지, 관리 창 자동 열림 없음.
5. 활성화 직후 다른 앱 선택·메모 클릭·키 입력: 대기 자동 열기 및 최소화 완료 포커스 취소.
6. 시작 시 포커스 보존, Dock Reopen 기존 동작, updater 설치 예약 중 자동 열기 차단.

사용자는 2026-11-19에 “윈도우에서는 잘 동작하는 걸 확인했어”라고 확인하고 커밋·병합을 승인했다. 이는 사용자 확인 기록이며, macOS의 세부 회귀 항목 전체를 에이전트가 검증한 근거로 확대하지 않는다. Windows 제품 코드는 이번 작업에서 변경하지 않았다.

macOS 변경에 대한 에이전트 검증은 빌드·코드 검토 범위이므로 Conf 1, Risk Medium(앱 수준 포커스·창 상태 영향, 코드 복구 가능)이다. 지정 iMac 자동 시험은 중지 상태를 유지한다. 공개 배포는 승인 범위에 포함하지 않는다. 관련 이슈 #120의 양 플랫폼·핀 조건별 확인은 미완료 범위를 남겨 둔다.
