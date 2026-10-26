# Windows 개발·빌드

Windows 10/11 x64용 개발 경로입니다. Windows 실제 검증 결과는 `docs/verification.md`에 기록합니다. 설치 파일 공개 배포 여부와 빌드 성공은 별도입니다.

## 개발 도구

[Tauri Windows 사전 요구 사항](https://v2.tauri.app/start/prerequisites/#windows)에 따라 Node.js/npm, Rust MSVC stable, Visual Studio Build Tools의 Desktop development with C++ 및 Windows SDK, WebView2 Runtime이 필요합니다.

```powershell
winget install --id Microsoft.VisualStudio.2022.BuildTools --exact --source winget --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
winget install --id Rustlang.Rustup --exact --source winget
```

설치 후 새 터미널을 열어 `rustc -Vv`, `cargo --version`, `node --version`을 확인합니다. Rust 호스트는 `x86_64-pc-windows-msvc`입니다.

## 격리 개발 앱

```powershell
npm.cmd ci
npm.cmd run dev:info
npm.cmd run dev:build
.\scripts\agent-gibbon-dev.cmd open --id windows-test --markdown '# Windows 메모'
.\scripts\agent-gibbon-dev.cmd inspect --id windows-test
.\scripts\agent-gibbon-dev.cmd close --id windows-test
```

개발 실행기는 현재 워크트리에서 빌드한 debug 실행 파일만 사용합니다. 개발 인스턴스의 소켓과 저장소는 설치 앱과 분리됩니다. 숨긴 메모를 변경할 때는 `open` 대신 `update`를 사용합니다.

```powershell
npm.cmd test
npm.cmd run check
npm.cmd run build:windows
```

설치 파일은 `src-tauri\target\release\bundle\nsis\`에 생성합니다. NSIS는 현재 사용자 설치를 사용하며 한글·영문 설치 화면을 제공합니다. [Tauri Windows 설치 파일 안내](https://v2.tauri.app/distribute/windows-installer/)를 따릅니다.

## 플랫폼 차이

- Windows에서도 HTTP 서버 없이 로컬 Unix domain socket으로 CLI와 앱이 통신합니다. Windows 10 1803 이상의 AF_UNIX 기능을 사용합니다.
- 소켓·중복 실행 잠금은 `%LOCALAPPDATA%\AgentGibbon\installed` 또는 개발 인스턴스 디렉터리에 둡니다. 사용자 프로필의 기본 접근 권한을 사용합니다. 전체 소켓 경로가 UTF-8 107바이트를 넘으면 앱 실행 전에 오류를 반환합니다. 한글 등 다중 바이트 문자가 포함된 긴 프로필 경로도 이 제한에 해당합니다.
- 메모는 Windows 문서 폴더의 `agent-gibbon\docs`에 보관합니다. 개발 메모는 Tauri 앱 데이터 폴더 아래 개발 인스턴스로 분리합니다.
- `con`, `nul`, `com1` 같은 예약 이름도 메모 ID로 사용할 수 있도록 폴더명을 인코딩합니다.
- 편집 저장 단축키는 Ctrl+S입니다. Ctrl을 누른 채 포인터를 노트 밖에서 안으로 옮기면 접기·펼치기, Alt는 반대편으로 이동, Ctrl+Alt+Win은 숨기기입니다. 다른 앱에 포커스가 있을 때 동작하며 숨기기 조합이 접기·이동보다 우선합니다. 화면 버튼도 사용할 수 있습니다.
- 키보드 포커스가 있는 메모는 모든 접촉 단축키를 무시합니다. 메모 안에서 보조키만 누르거나 해제·재누름하거나 포커스를 잃어도 실행하지 않습니다. 다른 앱으로 포커스를 옮기고 포인터를 실제로 밖으로 옮겼다가 다시 접촉해야 실행됩니다. Ctrl+A·Ctrl+S와 화면 버튼은 그대로 사용할 수 있습니다.
- 포인터가 밖에 있으면 펼친 노트는 불투명도 50%, 접힌 노트는 80%입니다. 포인터를 올리거나 등장·변경 알림이 진행 중이면 100%입니다. 표시·재열기·반투명 갱신은 사용자의 포커스를 유지합니다.
- Windows 접촉 감지는 WebView 타이머와 별도로 네이티브 키 상태를 주기적으로 확인합니다. 키 입력을 가로채지 않으므로 운영체제나 다른 앱에 설정된 단축키도 그대로 적용됩니다. 화면 잠금·보안 데스크톱에서는 접촉 감지를 보장하지 않습니다.
- Ctrl+Shift+1은 영역 캡처 이미지 메모, Ctrl+Shift+2는 영역을 지정하는 빈 편집기를 엽니다. Escape로 선택을 취소하며, 다른 인스턴스가 단축키를 점유하면 트레이 메뉴에서 실행할 수 있습니다. Windows에서는 macOS 화면 기록 권한 설정을 사용하지 않습니다.
- 이미지 메모는 네이티브 가장자리·모서리 드래그에서 원본 비율을 유지합니다. 접기 중에는 비율 제약을 해제하고 펼치면 복구하며, 일반 텍스트 메모의 크기 조절은 제약하지 않습니다.
- 서명 인증서는 아직 구성하지 않았습니다. 배포용 서명 여부는 릴리스 안내에서 명시해야 합니다.

## 실제 동작 회귀

먼저 `npm.cmd run dev:build`를 실행하고 해당 워크트리의 개발 앱을 종료합니다. 각 검증은 고유 메모를 만들며 자신이 시작한 정확한 개발 바이너리의 PID와 생성 파일만 정리합니다. 설치 앱에는 접근하지 않습니다.

```powershell
# CLI·저장·재시작, 선택적으로 두 개발 워크트리 격리
node scripts/windows-runtime-smoke.mjs '다른 워크트리의 절대 경로'
# 실제 포인터·보조키 접촉
python scripts/windows-contact-smoke.py
# 네이티브 반투명도·포커스·반복 표시, 실제 버튼 조작 포함
python scripts/windows-visual-smoke.py --interact
# 실제 연필 편집·한글 저장·가장자리 크기 조절과 복원
python scripts/windows-editor-smoke.py --interact
# 실제 영역 선택·빈 편집기·접힌 메모 클릭·이미지 비율 유지
python scripts/windows-capture-smoke.py --interact
```

포인터·키보드 검증은 화면 잠금을 해제한 상태에서 하나씩 실행하고 실행 중 사용자 입력을 멈춥니다. 테스트 시작 시 눌린 보조키가 있으면 실행하지 않습니다. 두 워크트리 검증은 양쪽 개발 바이너리를 먼저 각각 빌드해야 합니다.

캡처 검증은 주 모니터를 불투명한 흑백 패턴 테스트 창으로 덮고 그 안의 영역만 PNG로 저장합니다. 원본 픽셀·좌표·크기를 검사한 뒤 생성 파일을 정리하며 클립보드는 사용하지 않습니다.
