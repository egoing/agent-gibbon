# #71 전체 메모 투명도 검증 (2026-11-06)

## 변경과 출처

- 이슈: https://github.com/egoing/agent-gibbon/issues/71
- 작업 루트: `/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/global-memo-opacity-wheel`
- 브랜치: `global-memo-opacity-wheel`, base `1383a64b8a070adfcf81bad2f4fa055a7b1add47` (`origin/main`). 원본 체크아웃의 미커밋 파일은 복사·편집하지 않았다.
- Orca ID: `da5d2674-a5c4-4f3d-99cb-5f69bdbabe9c::/Users/egoinglee/orca/workspaces/agent-gibbon/agent-gibbon/global-memo-opacity-wheel`. 생성 응답·후속 show에서 브랜치와 이슈 #71을 확인했으며 `--activate`로 열었다.
- `npm run dev:info`: instance `dev-67cc0e13d61c6b98b34c3f11`, binary `<작업 루트>/src-tauri/target/debug/agent-gibbon`.
- macOS ARM64, Node v24.21.0, npm 11.19.0, rustc 1.91.1, Tauri 2.12.2. Windows target와 cargo-xwin은 설치되어 있지 않았다.

## 자동 검사

| 명령 | 결과 |
|---|---|
| `npm test` | Rust 136개·UI 81개 통과 |
| `npm run check` | Clippy all-targets, warnings deny 통과 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | 통과 |
| `npm run dev:build` | 해당 워크트리 debug 앱 빌드 통과 |
| `git diff --check` | 통과 |
| `rsvg-convert docs/design/global-opacity-wheel.svg -o docs/design/global-opacity-wheel.png` | 2.62.3으로 렌더, PNG 시각 확인 |

추가 회귀는 물리 방향/자연스러운 스크롤·Windows signed wheel delta 정규화, 정밀 휠 누적/보조키 해제·중단 초기화, 10~100% 제한, 설정 원자 저장·재로딩·인스턴스 격리·손상 거부, 저장 실패 시 메모리 값 유지, 보조 창 제외, 휠 뒤 전체 접어두기 취소를 다룬다. 공유 입력 관찰 경로를 재사용하고 추가 외부 통신·키 내용 저장·새 CLI 변경 명령을 만들지 않았다.

## 네이티브 실행 확인

최종 제품 문서의 첫 실행 동의가 없는 새 개발 인스턴스에서 CLI `open`/`inspect`는 `onboarding_required:true`로 거절됐다. 이를 성공으로 보고하지 않는다. 실제 약관에 대신 동의하지 않고, 해당 워크트리의 약관·개인정보 문구만 법적 효력이 없는 `OPACITY71 TEST ONLY` 문서로 임시 교체하여 빌드했다. 이 테스트 문서 해시에만 유효한 합성 동의(`sharing:false`)와 이 인스턴스에 한정된 35% 설정 파일을 준비했다. 앱/입력/저장/창 로직과 인스턴스 경로는 그대로 사용했다. 테스트 후 문서 원문을 바이트 단위로 복원하고 합성 동의를 제거한 뒤 제품 문서로 전체 검사·개발 빌드를 다시 수행했다.

- 모든 메모 작업은 이 루트의 `./scripts/agent-gibbon-dev`로 실행했다. `opacity71-plan`, `opacity71-peer`의 `inspect`에서 `ok:true`, 예상 instance, 고유 본문과 불투명도 `0.35`를 대조했다.
- 실제 프로세스 명령이 위 절대 binary 경로와 `--serve`임을 PID 10346에서 확인했다. CoreGraphics 창 목록에서 같은 PID와 `— opacity71-plan`/`— opacity71-peer` 네이티브 제목으로 창 39167/39169를 특정했다. 둘의 실제 `kCGWindowAlpha`는 약 `0.349999994`였다. CLI 설정값만으로 실제 alpha를 판단하지 않았다.
- `screencapture -x -l 39167`로 [해당 창 캡처](issue71/native-plan.png)를 얻고 시각 확인했다. 창 단독 캡처는 합성된 데스크톱 투명도 판단용이 아니며 실제 alpha 증거는 CoreGraphics 값이다.
- peer를 `close` → `inspect` → `update` → `inspect`했다. 숨김·포커스·위치·크기·고정과 35%가 유지되며 고유 본문만 바뀜을 assert했다.
- 이번 작업이 시작한 정확한 PID만 종료하고 같은 개발 CLI로 peer를 열어 재시작했다. 재시작 PID 11355의 절대 실행 경로를 다시 확인했다. peer 본문과 35%가 유지됐고 plan은 숨긴 상태로 복원됐다. 새 창 목록의 PID·제목·네이티브 alpha를 다시 대조했다.
- [원시 필드를 줄인 실행 근거](issue71/evidence.json)에 인스턴스·메모·PID·창 번호·응답·본문·네이티브 alpha를 남겼다. 테스트 프로세스는 종료했으며 다른 앱이나 워크트리 프로세스를 종료하지 않았다. 테스트용 메모와 35% 설정은 개발 인스턴스 안에만 남는다.

## 한계와 후속 수동 검증

- 다른 Agent Gibbon 인스턴스가 공통 전역 입력 잠금을 소유해 `Ctrl+Alt+Cmd 사용 중 · 트레이로 실행` 상태였다. 사용자 앱을 종료하거나 그 앱의 메모 투명도를 바꾸는 합성 전역 입력을 보내지 않았다. **물리 휠 → 네이티브 이벤트 관찰 → 저장까지의 실제 종단 입력은 미검증**이다. 입력 정규화·저장·단축키 취소는 자동 테스트, 설정 적용·복원은 위 네이티브 fixture로 각각 검증했다.
- 포인터 접촉·접힘·Feedback·터미널·전체 접어두기 전환 중 수동값 우선 적용은 구현했으나 이번 네이티브 검증은 일반 메모 2개와 숨김/재시작에 한정된다. 이 경로들의 실제 GUI 조합 검증은 남는다.
- Windows native hook·layered alpha 경로의 실제 컴파일/실행, Intel, 여러 입력 장치별 감도, 실제 사용자 약관 동의 후 제품 바이너리 종단 검증은 미실행이다. 과거 다른 워크트리 검증을 이번 실행 근거로 재사용하지 않았다.
- 수동 확인: 해당 버전이 전역 입력 담당인 상태에서 텍스트·이미지·접힌 메모·터미널을 열고 보조키+휠 양방향·상하한·hover·내용 갱신·키 해제 후 비접어두기를 확인한다. 숨김 메모 복원·새 메모·재실행에도 같은 값인지 확인하고 일반 휠/Shift 혼합이 조절하지 않는지 확인한다. 전체 접어두기 중 조절한 값은 복원 완료 시 적용되어야 한다.

전체 확신도 **Conf 3**, 위험도 **Medium**. 핵심 물리 입력과 Windows 미검증을 네이티브 설정 복원 성공으로 대체하지 않는다. 상호작용 기준과 설명용 시안은 [디자인 시스템](../design-system.md#전체-메모-투명도--71-2026-11-06)을 따른다.

## 구현 참고

- [Tauri WebviewWindow](https://docs.rs/tauri/latest/tauri/webview/struct.WebviewWindow.html)의 네이티브 창 접근을 사용한다.
- [Apple NSEvent direction inversion](https://developer.apple.com/documentation/appkit/nsevent/isdirectioninvertedfromdevice)에 따라 환경 설정으로 뒤집힌 델타를 물리 방향으로 정규화한다.
- Windows 전체 창 alpha는 [Microsoft SetLayeredWindowAttributes](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setlayeredwindowattributes)를 사용한다. 지원 플랫폼의 실제 실행을 대신하는 근거는 아니다.

## 사용자 재실행 세션의 추가 확인

- 사용자 보고 당시 이 워크트리의 PID 37748은 `Ctrl+Alt+Cmd 사용 중 · 트레이로 실행` 상태였고, `lsof`에서 다른 워크트리의 PID 72500이 공통 `modifier-stash.lock`을 보유했다. 이 상태에서 현재 앱의 휠 조절이 차단되는 것은 기존 단일 담당 규칙과 일치한다.
- 사용자의 재실행 승인 뒤 두 앱이 이미 종료된 것을 확인했다. 현재 워크트리를 `npm run dev:build`로 다시 빌드하고 `./scripts/agent-gibbon-dev open --id opacity-wheel-check`로 실행했다. 새 PID 11798의 절대 바이너리 경로·소켓·잠금 소유를 확인했고 `inspect` 응답의 instance·메모 ID·본문과 `stash.shortcut:Ctrl+Alt+Cmd`를 대조했다. 이때 약관 기본 생략 변경도 포함됐으며 사용자 동의 파일은 조작하지 않았다.
- 실행 직후 `global_opacity`·창 적용 값은 0.35였고 이후 사용자 확인 과정에서 둘 다 1.0으로 바뀌었다. 같은 PID·`— opacity-wheel-check` 제목의 네이티브 창 39665에서도 실제 alpha 1.0을 확인했다. [실행 근거](issue71/live-wheel-20261106.json)에 필드를 줄여 보관했다.
- 이 결과는 입력 담당 확보 후 실제 설정 변화·창 반영의 근거다. 물리 장치 이벤트 자체를 기록하거나 장치별 양방향·상하한·Shift 조합을 모두 검증한 것은 아니며, 위 Windows·다른 GUI 조합의 한계는 유지한다.
