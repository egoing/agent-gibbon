# Windows panic·멈춤 조사 — 2026-11-20

아래 상태·SHA·실행 결과는 최초 조사 종료 시점의 기록이다. 후속 사용자 승인에 따른 병합은 심볼 보관 준비와 조사 기록만 대상으로 하며, 멈춤 해결 완료를 뜻하지 않는다. 병합 결과와 기본 브랜치 동기화는 연결된 PR 기록을 따른다.

## 현재 결론

**멈춤 해결 완료가 아니다.** Sentry FLOATNOTE-9의 배포 바이너리와 Debug ID가 일치하는 EXE를 확보했다. 스택의 반환 주소 `+0x82ac8d` 바로 앞 호출이 `flush_paint_messages` assertion 경로이며, 해당 바이너리의 panic location 자료는 Tao 0.37.1 `src/platform_impl/windows/event_loop.rs:2295:11`이다. 이는 정확한 배포본의 기계어·문자열·내장 위치 자료를 대조한 결과다. 누락 PDB를 복원하거나 전체 스택을 심볼화한 결과는 아니다.

Jajan의 어떤 작업이 그 상태를 만들었는지, 동일 trace의 WebView 중단 및 Native UI 중단이 같은 원인인지, 모든 과거 멈춤이 이 panic으로 설명되는지는 미확정이다. 앱·watchdog·동의·필터·의존성을 추측으로 바꾸지 않았다. 실제 수정은 앞으로 배포 심볼이 유실되지 않도록 EXE/PDB를 검증해 별도 Actions artifact로 보관하는 준비에 한정한다. Sentry 쓰기·심볼 업로드·워크플로 실행·커밋·push·병합·배포는 하지 않았다.

## 작업 출처와 범위

- root: `C:\Users\egoin\.codex\worktrees\windows-panic-investigation\agent-gibbon`
- branch: `codex/windows-panic-investigation`
- start/base/현재 HEAD: `9753ed98d24ef0d679a2272543f4a93d8a11e870` + 본 작업 미커밋 diff.
- 기본 checkout 및 그 `.tmp`는 수정·삭제하지 않았다. 숨기기 담당 `windows-stash-stall` 워크트리에서는 전달받은 설치 파일을 읽기만 했다.
- president 팀원 지침을 읽었으나 `office.py`의 POSIX `fcntl` 제약 때문에 Windows 등록·report·통합 check를 실행하지 않았다. 우회 등록·가짜 상태·다른 채팅 전송은 하지 않았다. 현재 채팅 제목을 별도로 조회하지 않았으며 제목을 추측해 등록하지 않았다.
- agent-gibbon 진행 표시 스킬은 지정 fallback 경로에 없었다. voice-guide도 `ModuleNotFoundError: fcntl`; Windows에 macOS `say`가 없으므로 대화로 진행을 안내했다.
- Node 기본 설치 v20.11.0 / npm 10.2.4. 첫 의존성 설치 시 engine 경고가 있었으며 실제 개발 빌드·검증에는 npm registry의 임시 Node **v22.23.5**를 사용했다. 매니페스트·lockfile·시스템 Node를 변경하지 않았다.
- rustc 1.99.1, Cargo 1.99.1, Windows MSVC x86_64. Python 3.10으로 심볼 도구를 검증했다. CI에 지정한 Python 3.12의 원격 실행은 미검증이다.

## Sentry 직접 재조회

로그인된 Chrome에서 이 작업 전용 탭으로 [FLOATNOTE-9](https://egoing-personal.sentry.io/issues/7857896400/?project=4512315426799616)를 읽었다. 기존 담당자의 탭은 점유되어 새 탭으로 조회했고 외부 상태는 바꾸지 않았다.

- event `deaa713a71ff4185b9fd432c997a6323`, 2026-11-20 **12:36:19.165 KST**.
- fatal/unhandled `panic`, 원문 `Error details withheld for privacy`.
- release `jajan@0.3.5+c1c89d5a82b8`, Windows 10.0.26200, x86_64.
- processing error: required debug information file missing, `jajan.exe`, Debug ID `ad314f0a-07ac-458d-841f-cd1d45e16ba1-1`.
- 스택은 앞부분 `+0xa074db` … `+0xfb273e`, 중간 `+0x82ac8d`, `USER32 DispatchMessageWorker` 중첩, 뒤쪽 EmbeddedBrowserWebView 프레임을 포함한다.
- export_note 28580.8ms breadcrumb는 **09:34:11**, panic보다 약 3시간 전이다. export의 저장 대화상자 사용자 대기를 포함하는 전체 측정만으로 원인으로 단정하지 않는다.
- 같은 trace `25cbb93716c897a4d1a2d40d2ce3fabe`에 Native UI stalled / WebView stalled가 표시된다. 화면의 63/305는 누적 수이며 이 작업의 최근 24시간 수로 취급하지 않았다.
- Native 이벤트 `5f51af6290db4d55a9f3f3c802c1ddc5` 12:36:30.098 / 12001.574ms와 WebView 이벤트 `d83bfd18699e4898aae0a78d6628da06` 12:26:51.939 / 30840.0978ms의 상세 값은 인계 자료다. 이 작업에서 두 이벤트를 개별 재조회하지 않았으므로 독립 확인 값과 구분한다.

## 일치하는 배포 바이너리 분석

GitHub API로 공식 [release run 44491026740](https://github.com/egoing/agent-gibbon/actions/runs/44491026740)의 success, head `c1c89d5a82b8540070e3237048324e583c415978` 및 artifact 목록을 다시 확인했다. artifact는 세 installer와 complete-release-assets이며 PDB 전용 artifact가 없다. 현재 workflow에도 심볼 보관이 없었다.

기존 담당자의 `Jajan_0.3.5_x64-setup.exe`를 **설치/실행하지 않고** 이 워크트리의 `.tmp/release035`에 풀었다. 추출 도구는 [7-Zip 공식 다운로드](https://www.7-zip.org/download.html)에서 받은 7zr 및 26.06 x64이다. 원본 설치 파일은 변경하지 않았다.

| 항목 | 실제 값 |
|---|---|
| EXE SHA-256 | `0f5bd83cc1585a44ffb72ade1cb59bb99b5e364f0ba9202dec7329793e55716c` |
| PE CodeView GUID / age | `ad314f0a-07ac-458d-841f-cd1d45e16ba1` / `1` |
| Sentry Debug ID와 일치 | 예 |
| 내장 PDB basename | `jajan.pdb` |
| 원본 PDB 확보 | 아니오 |

PE debug directory를 파싱해야 한다. EXE에서 첫 `RSDS` 바이트만 검색하면 명령어 속 우연한 바이트를 오인하므로 그 방식은 폐기했고 새 검사에 회귀 사례를 넣었다.

일치하는 EXE를 pefile 2024.8.26 / Capstone 5.0.9로 읽은 국소 명령은 다음과 같다. 두 패키지는 로컬 `.tmp/python`에만 설치했고 제품 의존성에는 추가하지 않았다.

```text
RVA        bytes                instruction
0x82ac75   488d0db4538b00       lea rcx, [rip + 0x8b53b4] -> RVA 0x10e0030
0x82ac7c   4c8d05fd538b00       lea r8,  [rip + 0x8b53fd] -> RVA 0x10e0080
0x82ac83   ba49000000           mov edx, 0x49
0x82ac88   e893e18000           call 0x141038e20
0x82ac8d                        Sentry stack return address
```

RVA `0x10e0030`의 73바이트 문자열은 `assertion failed: flush_paint_messages(None, &userdata.event_loop_runner)`다. RVA `0x10e0080`의 Rust panic Location 자료에서 경로의 crate 이하와 행·열을 읽으면 `tao-0.37.1\src\platform_impl\windows\event_loop.rs`, 2295, 11이다. 빌드 머신 경로 전체를 기록하지 않았다. 이 국소 자료는 `.tmp/panic-binary-evidence.json`에도 남겼다.

배포 SHA와 현재 lockfile 모두 Tao **0.37.1**이다. 로컬 registry 소스에서 `flush_paint_messages`는 이미 redrawing 중이면 false를 반환하고, thread event target의 WM_PAINT 처리기가 이를 assert한다. 따라서 이 panic 지점은 페인트 처리 재진입/상태 문제로 좁혀진다. 어떤 작업이 재진입을 유발했는지는 이 정보만으로 정하지 않는다.

공식 [Tao #1180](https://github.com/tauri-apps/tao/issues/1180)에는 같은 assertion과 다른 event-loop 상태 panic들이 보고돼 있다. 그러나 확인한 [PR #1188](https://github.com/tauri-apps/tao/pull/1188)은 미병합이고 `Destroyed` 상태의 별도 panic을 다룬다. 이를 이 assertion의 검증된 해결책으로 적용하지 않았다. 최신 0.37.3 release 설명도 이 assertion의 수정 증거가 아니므로 무조건 업그레이드하지 않았다.

## 배제·보류한 코드 가설

- `windows_window::on_owner_thread`의 recv만으로 main-thread 자기 교착을 결론내리지 않았다. 설치된 runtime-wry 2.12.1의 `Context::send_user_message`는 메인 스레드에서 inline으로 실행한다.
- watchdog의 상태 guard 안에 visibility query와 diagnostic 호출이 있다. 다만 Windows visibility의 최종 호출은 `IsWindowVisible`이며, 실제 재진입 스택이나 재현 없이 이를 이번 panic 원인으로 지정하지 않았다.
- Sentry 진단 callback, 앱 잠금, WebView lifecycle, 장시간 process-scope trace는 별도 조사 대상이다. 늦은 watchdog 경고만으로 원인·결과 방향을 확정하지 않았다.
- Swatinem/rust-cache의 정리 코드는 profile 최상위 파일과 workspace 산출물을 정리한다. 캐시 목록에 v0.3.5 Windows cache가 있다는 사실을 원본 앱 PDB 보존 증거로 간주하지 않았다. 대용량 캐시 다운로드나 외부 cache 변경은 하지 않았다.

## 기존 로컬 크래시 덤프의 별도 사건

`%LOCALAPPDATA%\CrashDumps\jajan.exe.25852.dmp`가 기존에 존재했고 파일 수정 시각은 2026-11-20 09:29:08 KST였다. 원본을 수정·복사·업로드하거나 메모리 내용을 출력하지 않고 minidump module/exception stream 메타데이터만 읽었다. Jajan module base `0x7ff66e5e0000`, Debug ID는 같은 공식 배포본 `ad314f0a-07ac-458d-841f-cd1d45e16ba1-1`이다. 예외 스레드 32648, 예외 코드 **`0xc0000374`(STATUS_HEAP_CORRUPTION)**, 예외 주소 `0x7ff91fd02fc5`였다. 요약은 `.tmp/preexisting-dump-summary.json`이다.

이는 12:36 fatal panic과 **시각·예외 종류가 다른 별도 사건**이다. 같은 배포본에 원인 미상의 힙 손상도 관측된다는 근거이며, panic assertion이 힙 손상을 일으켰다거나 힙 손상이 그 panic을 일으켰다는 결론은 아니다. 덤프 원본의 PDB·힙 파괴 지점·올바른 unwind stack은 확보하지 못했다. 예외 주소 하나만으로 손상 책임 모듈을 지정하지 않는다.

## 검토 가능한 변경과 검증

- `.github/workflows/release.yml`: Windows 설치 빌드에만 release debug=1. 같은 빌드 EXE/PDB를 검증한 별도 30일 artifact 추가. 공개 릴리스 조립 glob·Sentry 설정은 변경하지 않았다. 원격 실행은 하지 않았다.
- `scripts/windows-symbols.py`: 표준 라이브러리만 사용해 PE CodeView와 PDB MSF GUID/age 비교. 검증한 동일 바이트와 전체 SHA/Debug ID/파일 해시 manifest 보관. 불일치·누락·기존 출력은 실패. 소스 경로를 manifest에 넣지 않는다.
- `scripts/test_windows_symbols.py`: 4개 테스트 통과. 가짜 첫 RSDS, GUID/age 불일치, 잘린 입력, 원본 바이트 보존, 출력 덮어쓰기 거부를 포함한다. 처음 잘못된 하이픈 파일명으로 discover가 0개 수집한 것은 통과로 세지 않았고, 밑줄 파일명으로 고쳐 4개 실행을 확인했다.
- 실제 이 워크트리의 debug EXE/PDB: GUID `4512cb14-47ba-48ec-85a7-d9ae06d868cd-1` 일치·보관 성공. 이 ID는 0.3.5 배포본의 ID와 다르며 원본 역산에 쓰지 않았다.
- 별도 `rustc -C opt-level=3 -C debuginfo=1` 최소 fixture의 실제 EXE/PDB도 일치·보관 성공. 공식 0.3.5 EXE + 현재 debug PDB 조합은 실제 mismatch 오류로 거부하고 출력 폴더도 만들지 않았다.
- Windows 개발 빌드 통과. Rustfmt 통과. Rust 테스트 **304 통과**, Clippy `--all-targets -- -D warnings` 통과. UI 테스트 **229 통과/1 제외**. 새 앱 코드 변경 없이 기존 검사를 실행한 결과다.
- macOS 빌드/실행, 공식 release 재빌드, 원격 Python 3.12/Actions artifact 업로드, Sentry 심볼화, 장시간/실제 사용자 입력으로 동일 panic 재현은 미검증이다.

## 개발 앱 실제 CLI 확인

`npm run dev:info` root는 위 작업 루트, instance는 `dev-2e34b32b5503419cc7c205d4`, binary는 작업 루트의 `src-tauri\target\debug\jajan.exe`였다. `JAJAN_TELEMETRY=off`로 해당 root의 `scripts/jajan-dev.cmd open`을 실행했다.

- 검증 메모 `panic-investigation-20261120`, 본문 `PANIC-INVESTIGATION-2E34B32B-20261120`의 inspect에서 `ok:true`, 예상 instance, 동일 본문을 확인했다. `.tmp/live-inspect.json`.
- CIM의 **PID 29628**, 실행 파일 절대 경로가 위 binary와 일치했다. GUI를 조작·캡처하지 않았으므로 HWND/화면 기반 검증은 주장하지 않는다.
- diagnostics: `enabled:false`, `detailed:false`, `sharing_allowed:false`, `terms_accepted:false`, `terms_skipped:true`. 동의를 대신 생성하지 않았다.
- 고유 메모 3개의 open/update/inspect/close/inspect **15회**에서 모두 성공과 instance 일치를 확인했다. 기존 사용자 메모·설치 앱·다른 워크트리 앱은 변경/종료하지 않았다. 이는 짧은 정상 CLI 확인이며 과거 panic의 재현·해결 증거가 아니다.
- 16:02 KST 후속 조회에서 PID 29628은 더 이상 없었고, 해당 root의 inspect는 `앱이 실행 중이지 않습니다`로 실패했다. 에이전트가 이 PID에 종료 명령을 보내지 않았다. 조회 시 다른 담당 워크트리의 PID 31588만 실행 중이었으며 조작하지 않았다. 최근 25분 Application 1000/1001에서 Jajan 오류를 찾지 못했고 새 crash dump도 관측하지 못했다. 사용자 종료·외부 종료·앱 오류 중 무엇인지 증거가 없어 미확정으로 남긴다. 앞선 15회 성공을 장시간 안정성이나 종료 원인 검증으로 확대하지 않는다. 재시작/다른 앱 fallback은 하지 않았다.

## 남은 작업과 직접 확인

1. 원본 Debug ID에 일치하는 PDB가 별도 보관돼 있다면 확보해 전체 스택을 복원한다. 현재 artifact/캐시 관측만으로 존재한다고 가정하지 않는다.
2. 같은 사용자 동작에서 멈춤이 재현되면 재시작 전 동일 PID의 UI·작업자 스택과 로컬 단계 기록을 수집한다. 사용자 메모·동의·Sentry 필터를 바꾸지 않는다.
3. 앱의 유발 경로나 Tao의 최소 재현을 확보한 후 이벤트 루프 상태·메시지 전달 규칙을 유지하는 수정을 검증한다. assertion 삭제·panic 삼키기·오류 필터링만으로 해결 처리하지 않는다.

심볼 도구를 직접 확인하려면 이 root에서 `python -m unittest discover -s scripts -p test_windows_symbols.py -v`를 실행한다. 실제 앱 쌍은 `python scripts/windows-symbols.py --binary src-tauri/target/debug/jajan.exe --pdb src-tauri/target/debug/jajan.pdb --output <새 빈 폴더> --revision 9753ed98d24ef0d679a2272543f4a93d8a11e870`로 검증한다. 출력의 Debug ID와 두 SHA-256이 있어야 하고 잘못된 PDB는 비정상 종료해야 한다.

UI 기준·외형·배치·상호작용을 변경하지 않았다. [디자인 시스템](../design-system.md)의 기존 규칙을 유지하며 SVG/PNG 수정·렌더 확인은 이번 변경 대상이 아니다. Conf **3**(심볼 도구 자동 검사 및 실제 파일 검증), Risk **Low**(앱 런타임 변경 없음). Windows 실제 CLI 확인은 별도 성공 사실이며 Sentry 멈춤 수정의 Conf로 올리지 않는다.
