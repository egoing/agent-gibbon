# #86 Windows 외부 창 선행 실증 — PASS (제한 범위)

2026-11-07 KST, 전용 워크트리 `C:\Users\egoin\orca\workspaces\agent-gibbon\86-windows-poc`에서 직접 실행했다. **같은 사용자·같은 High integrity 토큰으로 실행한 전용 외부 WinForms 프로세스 두 개에 대해서 필수 네 항목을 모두 화면·문서·입력 이벤트로 확인했다.** 일반 설치 앱 전체, 낮은 권한 실행, 권한 경계, 다중 모니터를 보장하는 결과가 아니다. 제품 구현·병합·태그·릴리스·진단 업로드는 수행하지 않았다.

## 출처와 환경

- 시작 시 `git status --short` 출력 없음, 브랜치 `egoing/86-windows-poc`, 실제 Windows 소스 **`37dbfed28deac5e01f9764db9b352a8f2495bda4`**. 전달된 Mac 기준 `48041c5e3dcdb6db98e136d0098ab30a6343666a`와 다르며 그 SHA에서 실행했다고 주장하지 않는다.
- 기존 Windows main과 다른 워크트리를 건드리지 않았다. 이 전용 워크트리는 작업 시작 때 이미 분리되어 있었다. 변경 파일은 이 보고서 디렉터리의 PoC 스크립트·증거뿐이다.
- Windows 11 Home x64, `10.0.26200` / build `26200`, 단일 1920×1080 모니터, 대상 창 `GetDpiForWindow=120` (125%). Python의 OS 이름 `Windows-10`은 호환 버전 문자열이며 OS 명칭은 CIM 조회를 따른다.
- PowerShell `5.1.26100.9711`, Python `3.10.11` AMD64 + Pillow, .NET Framework C# compiler `4.8.9221.0`, Orca `1.4.250` / Windows provider `1.0.0`, Git `2.43.0.windows.1`, gh `2.96.0`, Node `20.11.0`, npm `10.2.4`, rustc/cargo `1.99.1`.
- Windows `whoami.exe /groups`의 Mandatory Label은 **High / `S-1-16-12288`**다. 테스트 앱은 `runas` 없이 자식 프로세스로 시작했으므로 같은 토큰을 상속했다. Medium·High 간 입력이나 UAC 승인은 시험하지 않았다.
- 저장소 `AGENTS.md`, README, 디자인 시스템, [이슈 본문/댓글](https://github.com/egoing/agent-gibbon/issues/86)을 읽었다. 조회 당시 댓글은 없었다. 원문은 [issue86.json](issue86.json), 환경은 [environment.txt](environment.txt), 기능 지원은 [orca-capabilities.json](orca-capabilities.json)에 보존했다.

성공 실행은 **[run-20261107-012815](run-20261107-012815/summary.json)**, UTC `2026-11-06 16:28:15`~`16:28:33` / KST `2026-11-07 01:28:15`~`01:28:33`이다. 아래 HWND는 이 실행에서만 유효했다.

| 대상 | 실제 PID | HWND | 네이티브 창 제목 |
|---|---:|---|---|
| A: 조작할 외부 편집기 | 25720 | `2491232` / `0x260360` | `ISSUE86-POC-A-task8f3bf6` |
| B: 겹치는 일반 창 | 7140 | `8914876` / `0x8807bc` | `ISSUE86-POC-B-task8f3bf6` |

두 프로세스의 실제 실행 경로는 `C:\Users\egoin\AppData\Local\Temp\gibbon-issue86-task8f3bf6-_tz5ea86\Issue86TestWindow.exe`다. 별도 C# WinForms EXE를 컴파일·실행했으며 Gibbon 창/WebView가 아니다. 대상 PID는 `Popen` 결과·준비 파일·`GetWindowThreadProcessId`로 대조하고 실행 경로는 `QueryFullProcessImageNameW`로 읽었다. 제목·HWND·PID는 Orca `list-windows` 결과로도 대조했다. 각 앱의 편집기/버튼 child HWND는 `A-ready.json`·`B-ready.json`에 있다.

## 필수 관측 결과: API 반환과 실제 증거 분리

좌표는 모두 물리 screen px, 사각형은 `[left, top, right, bottom]`이다. 초기 A는 `[80,100,720,520]` / 640×420, B는 `[140,140,960,720]` / 820×580, 둘 다 `showCmd=1`, visible=true, topmost=false였다. 전체 `WINDOWPLACEMENT` 원본 bytes도 로그에 저장했다.

| 항목 | API 반환 | 화면/문서/독립 조회 관측 | 판정 |
|---|---|---|---|
| 위치·크기 | `SetWindowPos(A, HWND_TOP,220,180,640,420,SWP_NOACTIVATE\|SWP_SHOWWINDOW)=1` | `GetWindowRect=[220,180,860,600]`; 실제 A 제목·편집기가 지정 영역에 표시됨 | PASS |
| 일반 창 활성화 뒤 상단 유지 | `SetWindowPos(A,HWND_TOPMOST,...,SWP_NOMOVE\|SWP_NOSIZE\|SWP_NOACTIVATE)=1`; B의 노출된 제목바에 `SendInput` 마우스 클릭 2개 전달 | `GetForegroundWindow=B`; B topmost=false, A topmost=true; 겹침점 `(300,300)`의 `WindowFromPoint→GetAncestor=A`; 화면에서 활성 B 위에 비활성 A가 남음 | PASS |
| 실제 클릭·키보드 | Orca click/type-text는 `ok:true`이나 **`unverified/synthetic_input`**; 별도 Win32 마우스 `SendInput=2`, Unicode 키보드 `SendInput=24` | Orca 입력 뒤 문서와 화면에 `ORCA_86_OK `; 네이티브 버튼 클릭 이벤트와 화면 `clicks=1`; 추가 키 입력 뒤 문서·화면 `ORCNATIVE_86_OKA_86_OK `; `KeyDown/KeyPress/TextChanged/MouseDown/Click` 이벤트 존재 | PASS |
| 해제·원상 복귀 | `SetWindowPos(A,HWND_NOTOPMOST,...)=1`; B 제목바 실제 클릭; 저장한 `WINDOWPLACEMENT`로 `SetWindowPlacement=1` | 해제 뒤 foreground=B, 겹침점 root=B, A topmost=false이며 화면에서 B가 A를 완전히 가림; 복원 뒤 A `[80,100,720,520]`, showCmd/placement/visible/topmost 초기값 완전 일치, 실제 A 화면 재확인; B도 초기값 완전 일치 | PASS |

네이티브 입력 문자열이 기존 문자의 중간에 들어간 것은 실제 편집기 클릭 위치에 생긴 caret 때문이다. 문자열을 API로 직접 설정하거나 결과 파일을 만들어 통과시키지 않았다. 앱이 `TextChanged` 이벤트에서 저장한 [A-document.txt](run-20261107-012815/A-document.txt)와 [A-events.tsv](run-20261107-012815/A-events.tsv)를 화면과 대조했다. 코드에는 `WM_SETTEXT`, UIA `set-value`, clipboard paste가 없다.

### 화면 증거

- [초기 A](run-20261107-012815/01-initial-A-client.png), [지정 영역의 A](run-20261107-012815/02-position-A-client.png)
- [활성 B 위에 남은 A](run-20261107-012815/03-topmost-B-active.png): B의 활성 제목바/입력 caret와 A의 비활성 제목바를 함께 볼 수 있다.
- [Orca 입력 결과](run-20261107-012815/04-orca-input.png), [네이티브 입력·clicks=1](run-20261107-012815/05-native-input.png)
- [해제 후 B가 A를 덮음](run-20261107-012815/06-release-B-above.png), [초기 위치·상태로 복원한 A](run-20261107-012815/07-restored-A-client.png)
- [공식 Orca 캡처](run-20261107-012815/00-orca-native-capture-client.png)

화면을 직접 열어 위 표시를 확인했다. 전체 화면은 저장·공개하지 않았다. A의 바깥 DWM 그림자에 기존 앱의 가장자리 픽셀이 비칠 수 있어 초기/이동/복원/Orca 캡처 네 장은 원본 640×420에서 `(12,2,628,408)`만 잘랐다. 크기 조절·합성·내용 수정은 없으며 [원본 해시·crop 좌표](run-20261107-012815/screenshot-provenance.json)를 남겼다. 초기 실패 시 생성된 미검증 스크린샷 두 장도 공개하지 않고 제거했다. `raw.jsonl`의 원래 캡처 파일명은 실행 당시 기록이며 공개본의 `-client.png`가 그 파생본이다. 나머지 네 장은 A/B 테스트 창 영역만 포함한 원본 crop이다.

## 포커스 제한, 실패 시도와 안전한 종료

첫 네 실행(`012554`, `012619`, `012652`, `012739`)은 포커스/노출 조건을 충족하지 못해 **CLOSED**다. `SetWindowPos`는 성공해도 실제 hit-test가 기존 Orca HWND를 가리켰고, `SetForegroundWindow=0` 및 실제 foreground 유지가 관측됐다. 입력 전 root HWND 검사를 통과하지 못한 지점에서는 마우스/키보드를 보내지 않았다. 이 실행들의 raw 로그·실패 summary를 보존하며 성공으로 계산하지 않는다.

성공 실행에서도 `AttachThreadInput` 자체는 1이지만 `SetForegroundWindow=0`이었다. Orca `get-app-state --restore-window`가 `ok:true`를 반환해도 실제 포커스 보장이 아니었고 첫 공식 클릭은 `window_not_focused`로 거부됐다. **자기 A만 잠시 TOPMOST로 노출하고 안전한 제목바 실제 클릭을 사용한 뒤**, A를 ordinary 상태로 돌려 초기 화면을 확인했다. 이후 B를 실제 활성화한 본 시험에서 네 항목을 검증했다. 따라서 백그라운드 프로세스가 임의의 앱 포커스를 항상 획득할 수 있다는 주장은 하지 않는다.

초기 세 실행은 DPI awareness 설정 전에 cursor를 저장하는 스크립트 오류가 있어 포인터 복원 좌표에 125% 변환 차이가 생겼다. 이후에는 DPI awareness를 먼저 설정하도록 고쳤다. 원래 작업 시작 시의 포인터 위치까지 정확히 보존했다고 주장하지 않으며 기존 창 위치·표시 상태를 변경한 것은 아니다. 성공 실행에서는 시작 cursor `[671,379]`와 종료 readback이 정확히 일치했다.

성공 실행의 종료 순간에는 원래 foreground HWND `1967202`로 복귀하고 실제 foreground 일치를 확인했다. 자기 A/B의 PID·실행 경로를 다시 확인하고 초기 placement/topmost로 복원한 뒤 **그 HWND에만 WM_CLOSE**를 보냈으며 PID 25720·7140이 exit=0으로 종료됐다. 다른 모든 시도에서 시작한 테스트 프로세스도 로그상 exit=0이다. 프로세스 이름 일괄 종료·기존 앱 종료·사용자 메모 변경은 하지 않았다.

후속 읽기 전용 [기존 창 보존 대조](run-20261107-012815/original-window-preservation.json)에서도 원래 Orca 창의 rect `[-9,-9,1929,1029]`, WINDOWPLACEMENT bytes, visible, topmost가 실행 전과 같았다. 후속 조회 시 foreground는 이미 다른 HWND였으므로 다시 바꾸지 않았다. 기존 앱 내용이나 제목은 기록하지 않았다.

## 제약과 미검증 범위

- **최소 크기 실측:** A에 100×80을 요청했지만 API는 1을 반환하고 실제 rect는 `[220,180,580,420]` / **360×240**였다. 이는 테스트 앱의 `MinimumSize`와 일치하며 요청 영역을 무조건 채울 수 없음을 입증한다.
- **DPI:** 단일 120 DPI 화면에서만 물리 좌표/스크린샷 크기를 대조했다. 컨트롤러는 Per Monitor V2, WinForms 호스트는 System DPI aware다. 혼합 DPI, 모니터 이동·분리, 음수 모니터 좌표, 다른 배율은 미검증이다.
- **권한:** High→High에서 실행했다. 표준 사용자 Medium→Medium, Medium→High, UAC secure desktop, UIAccess·보호 프로세스는 미검증이다. 공식 `SendInput` 문서상 UIPI 제한이 있으며 반환값만으로 UIPI 원인을 판별할 수 없다.
- **여러 창:** 서로 다른 프로세스 두 개를 PID+HWND로 구분했다. 같은 프로세스의 여러 최상위 창, 재사용 프로세스, 실행 지연·splash·modal/owned 창·재시작 시 HWND 재사용, UWP/관리자 앱·탐색기·브라우저 등 설치 앱 선택/실행 경로는 미검증이다.
- **복원:** 처음부터 visible/normal/non-topmost였던 두 창을 정확히 복원했다. 초기 최소화·최대화·숨김·기존 topmost 창과 다른 topmost 창 사이 우선순위는 시험하지 않았다. 전체 z-order의 초기 순서까지 복원했다고 주장하지 않는다.
- **앱 지원:** 전용 classic WinForms 편집기는 외부 네이티브 앱의 가능성 증거다. 제품의 영역 드래그·앱 탐색·실행·취소·에러 UX를 구현하거나 시험하지 않았다. Gibbon 빌드/CLI·Mac·ARM64·release.yml 검사는 이번 PoC 대상이 아니다.
- 제품 UI/용어/디자인 기준을 변경하지 않아 디자인 문서·SVG/PNG는 수정하지 않았다. 기존 기준은 [디자인 시스템](../../design-system.md)을 따른다.

## 재현과 원본 로그

```powershell
python docs/verification/issue86-windows-poc/run-poc.py
# 별도 새 run-* 폴더가 생김. 결과를 검토한 후 공개용 그림자 crop만 준비한다.
python docs/verification/issue86-windows-poc/prepare-evidence.py <새-run-폴더>
python docs/verification/issue86-windows-poc/check-preservation.py <새-run-폴더>
```

컴파일과 GUI 실행은 실제 완료했고 Python syntax compile도 통과했다. 재현 시 자기 전용 창에 포커스와 포인터 입력을 잠시 사용한다. 최초 상태·각 API 반환/오류·관측·스크린샷 좌표·종료는 [raw.jsonl](run-20261107-012815/raw.jsonl), Orca 명령과 반환은 같은 폴더의 `orca-*.stdout.txt`/`stderr.txt`, 실행 요약은 [summary.json](run-20261107-012815/summary.json)에 있다. 해당 폴더의 [sha256.json](run-20261107-012815/sha256.json)은 공개 증거의 무결성 확인용이다. 이 디렉터리의 `.gitattributes`는 원본 증거의 줄바꿈 bytes와 실제 입력의 후행 공백을 보존한다.

판단 근거로 확인한 공식 문서:

- [SetWindowPos](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowpos): TOPMOST/NOTOPMOST와 비활성 배치
- [SetWindowPlacement](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowplacement): show state/복원 위치 저장과 모니터 변경 시 보정
- [SendInput](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput): 입력 스트림과 UIPI/반환값 한계
- [SetForegroundWindow](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setforegroundwindow): Windows 포커스 획득 제한

**게이트 결론:** 같은 토큰으로 시작한 전용 외부 데스크톱 앱에서 위치/크기·상단 고정·실제 입력·해제/복원은 실증했다. 후속 구현의 검토 가능한 선행 근거로 제공하며, 위 미검증 조건을 지원 완료로 확대하지 않는다.
