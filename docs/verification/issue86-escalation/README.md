# #86 Windows 단일 에스컬레이션

- 작업 루트: `C:/Users/egoin/orca/workspaces/agent-gibbon/86-windows-escalation` (다른 워크트리와 분리된 기존 전용 작업 공간, 시작 시 clean).
- 기준: `origin/86-windows-floating-apps`를 fetch한 뒤 정확히 `5b45811936745b24da4b8d92dc908139132f250b`에서 새 브랜치 `egoing/86-windows-escalation-fix` 생성.
- Windows 11 Home x64 `10.0.26200`, Rust/Cargo `1.99.1`, Node `20.11.0`, npm `10.2.4`, Python `3.10.11`, Chrome `156.0.8078.80`, Orca `1.4.250`. 2026-11-07 KST 실행.
- `npm run dev:info`: ID `dev-d83e74df06998bfab2fb836c`, binary `<작업 루트>/src-tauri/target/debug/agent-gibbon.exe`. 앱을 실행하거나 기존 프로세스/창을 조작하지 않았다. 이 ID는 빌드 경로 확인이며 실행 중 서버 출처 검증을 뜻하지 않는다.

## 원인과 수정 경계

기준 SHA의 Windows Clippy를 직접 실행하여 `items_after_test_module`과 exit 101을 재현했다([원문](before-clippy.log)). `guard_selection`과 `clear_claim` 아래로 테스트 모듈을 이동했다. 테스트 본문·기대값과 나머지 Rust 내용은 빈 줄을 제외하고 동일하다([내용 비교](source-integrity.json)). 네이티브 동작·CLI·timeout·lint 설정을 변경하지 않았다.

`ui/style.css`의 전역 `button { width: var(--toolbar-target) }`에서 `--toolbar-target:24px`를 상속한 것이 실제 폭 결함이다. 기존 외부 앱 CSS는 padding만 지정하여 새로 고침·취소가 약 25.6px 폭이 됐다. Chromium에서도 새로 고침 네 줄·취소 두 줄을 재현했다. `.actions`의 다른 CSS 선언 충돌은 없었으며, 소유 범위를 분명히 하도록 `.external-app-actions`로 변경했다. 해당 세 버튼만 `width:auto; min-width:64px; flex:0 0 auto; white-space:nowrap`를 적용하고 기존 행의 `flex-wrap:wrap`을 유지했다. 세션 해제 버튼·다른 UI의 규칙은 변경하지 않았다.

정확한 세 소스 파일의 전후 diff는 [source.diff](source.diff)다. 버전·태그·릴리스·의존성·lockfile·공통 스킬·진단 동의/전송을 변경하지 않았다. 의존성은 기존 lockfile로 `npm ci`만 실행했다.

## 실제 브라우저 레이아웃 회귀

새 의존성 없이 Python 표준 라이브러리와 설치 Chrome을 사용하는 [검증 스크립트](layout-check.py)를 추가했다. 실제 HTML과 네 CSS 파일을 읽고 네이티브 IPC bootstrap만 공개 예시 행을 만드는 코드로 교체한다. 사설 임시 프로필·headless 모드·외부 호스트 차단을 사용하며 키보드/마우스/전경 창을 조작하지 않는다. Chrome 네이티브 최소 창 폭과 제목줄 inset 때문에 iframe 안 문서의 실제 viewport를 지정하고, 요청값과 `innerWidth/innerHeight`가 같은지도 검사한다.

```powershell
python docs/verification/issue86-escalation/layout-check.py --label before --ref 5b45811936745b24da4b8d92dc908139132f250b
python docs/verification/issue86-escalation/layout-check.py --label after
```

125% DPR에서 480×460, 384×368, 320×240, 240×240 CSS px를 검사했다. 수정 전 네 크기 모두 FAIL(exit 1), 수정 후 모두 PASS(exit 0)다. 각 버튼의 실제 64×32px 이상 target, Range 기준 한 줄 문구 및 내부 수용, 버튼 간 겹침 없음, 가로 overflow 없음, 스크롤 후 viewport 내 접근성을 검사한다. 240px 폭에서는 버튼 단위 줄바꿈도 확인한다. CSS 문자열과 구현을 대조하는 검사가 아니다.

- 원시 측정: [수정 전](before-layout.json), [수정 후](after-layout.json), 콘솔 [전](before-layout.log)/[후](after-layout.log).
- 480×460 전후: [수정 전](before-480x460.png), [수정 후](after-480x460.png).
- 240×240 전후: [수정 전](before-240x240.png), [수정 후](after-240x240.png). 384×368·320×240도 같은 폴더에 보존했다.
- PNG는 실제 페이지 문서와 측정 결과만 담은 headless 브라우저 산출물이다. 사용자 화면/메모/설치 앱 캡처가 아니며 480×460·240×240 수정 후 렌더를 직접 열어 확인했다.

## Windows 게이트

실행 명령과 종료 코드는 [gates.txt](gates.txt)에 보존한다. 각 검사는 이 작업 루트의 소스에 대해 실행했다. 이전 Windows 작업자의 통과 기록을 이번 수정의 결과로 재사용하지 않았다.

| 검사 | 결과 | 원본 출력 |
| --- | --- | --- |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | exit 0 | [fmt](after-fmt.log), 정상 시 빈 출력 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | exit 0, 178 PASS / 0 ignored | [test](after-test.log) |
| `cargo check --manifest-path src-tauri/Cargo.toml --locked` | exit 0 | [check](after-check.log) |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | exit 0 | [clippy](after-clippy.log) |
| `npm run test:ui` | exit 0, 123 PASS / 기존 1 SKIP | [UI](after-ui.log) |
| `npm run dev:build` | exit 0 | [개발 빌드](after-dev-build.log) |

기존 SKIP ID는 TAP `12`, `tests/dev-instance.test.mjs:26`의 `development CLI rejects missing, old and foreign binaries without release fallback`이다. 사유는 `POSIX executable fixtures; Windows uses native CLI smoke tests`다. 기존 테스트를 수정하거나 건너뛰지 않았다.

PowerShell 5의 stderr 병합 출력에는 성공한 cargo 진행 메시지도 `NativeCommandError` 형식으로 포함된다. 로그 본문을 지우지 않았으며 위 결과는 실제 프로세스 `$LASTEXITCODE`와 test 요약/Finished 행으로 확인했다. 텍스트 로그는 열람을 위해 UTF-16 콘솔 캡처를 내용 변경 없이 UTF-8로 변환했다.

## 디자인 기준과 남은 검증

[디자인 시스템](../../design-system.md)의 기존 13px 본문·32px 이상 행과 버튼·4px 모서리·선택 후 실행 구조 및 [외부 앱 시안](../../design/external-apps.png)의 한 줄 텍스트 버튼을 적용하는 결함 수정이다. 새 시각 기준을 만들지 않았고 시안은 변경하지 않았다. 소유권이 세 소스 파일과 이 검증 폴더로 제한되어 디자인 시스템 파일을 수정하지 않았으므로, 통합 담당자가 적용·검증 상태 링크를 추가해야 한다.

실제 Tauri/WebView2의 Windows 125% Welcome, 단축키/취소/플로팅 기능의 재확인은 기존 Windows 제품 검증자가 최종 통합 SHA에서 별도로 수행한다. 이 작업은 실제 GUI 성공을 주장하지 않는다. 부모의 최종 통합 SHA 재검증·독립 감사·전체 플랫폼 workflow 검증·병합은 남아 있으며, 이 브랜치에서 릴리스 workflow 실행이나 공개 배포를 하지 않는다.
