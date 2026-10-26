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
- 소켓·중복 실행 잠금은 `%LOCALAPPDATA%\AgentGibbon\installed` 또는 개발 인스턴스 디렉터리에 둡니다. 사용자 프로필의 기본 접근 권한을 사용합니다. 긴 프로필 경로는 Windows Unix socket 경로 제한에 걸릴 수 있습니다.
- 메모는 Windows 문서 폴더의 `agent-gibbon\docs`에 보관합니다. 개발 메모는 Tauri 앱 데이터 폴더 아래 개발 인스턴스로 분리합니다.
- `con`, `nul`, `com1` 같은 예약 이름도 메모 ID로 사용할 수 있도록 폴더명을 인코딩합니다.
- 편집 저장 단축키는 Ctrl+S입니다. macOS의 전역 보조키+포인터 접촉 동작과 창 투명도 효과는 Windows 지원 범위에 포함하지 않습니다. 이동·접기·숨기기는 화면 버튼을 사용합니다.
- 서명 인증서는 아직 구성하지 않았습니다. 배포용 서명 여부는 릴리스 안내에서 명시해야 합니다.
