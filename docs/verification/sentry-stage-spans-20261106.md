# Sentry 단계 계측 세분화 — 2026-11-06

## 범위

사용자가 요청한 후속 작업으로 #19(터미널 단계별 계측)·#64(성능 진단)의 trace/span 세분화 부분을 반영한다. 모든 새 operation·stage label은 고정된 영어 이름이다. 작업 목록과 의미는 [진단 안내](../diagnostics.md#단계별-tracespan-이름)를 따른다.

캡처의 모니터 열거·화면 획득·Overlay 생성·픽셀 복사·선택 검증·PNG 인코딩·파괴 요청/대기, Windows 직렬화 잠금 대기, 터미널 창의 생성/배치/표시, 명령 준비·실제 탐색·PTY 할당·프로세스 생성·I/O 연결·세션 잠금·쓰기·출력 필터/IPC 전달을 분리했다. WebView의 초기 준비·첫 출력·xterm 파싱 큐 처리는 2초 단위 숫자 집계로 측정한다. xterm callback을 GPU 페인트 완료나 함수별 프로파일로 설명하지 않는다.

창 생성·프로세스 생성·파괴 대기에는 시작 로그를 남겨 끝나지 않은 span의 직전 단계를 찾을 수 있게 했다. 현재 Rust trace의 무작위 trace ID를 structured log와 연결하며, 메모/사용자 식별자는 추가하지 않았다. 기존 privacy envelope 필터·분당 한도·상세 모드 opt-in을 유지한다. 별도의 업로드 경로나 서버·브라우저 SDK를 추가하지 않았다.

## 검증

- MacBook root `/Users/egoinglee/dev/project/agent-gibbon`, instance `dev-a5210ae16e79437d38490c8f`, binary는 root의 `src-tauri/target/debug/agent-gibbon`이다.
- Rust 전체 116개와 UI 74개가 통과했다. 새 프런트 검사에서는 일반 모드의 측정 API 부재, 영어 stage 허용 목록, 원문·임의 이름·무한값 차단, 숨긴 창 제외를 검증했다.
- 코드·검사와 실제 native 실행·Sentry 수신은 구분한다. 최초 구현 커밋에서는 새 span의 실제 수신과 원격 배포를 통과로 기록하지 않는다. 재빌드·재시작 후 root/instance/PID/메모와 신규 release를 다시 확인해 후속 기록한다.

## 실제 실행과 수신

구현 커밋은 `5aaadd3c938d6e8c5edb0fde4a3c7f7750b2c166`이다. GitHub main에 push한 뒤 MacBook과 iMac에서 같은 커밋을 빌드·재시작했다. iMac root `/Users/egoing/dev/project/agent-gibbon`, instance `dev-e467b83ad9082909fc66aba7`, binary는 root의 `src-tauri/target/debug/agent-gibbon`, PID `26302`를 확인했다. iMac Rust 116개·UI 74개·Clippy·진단 빌드가 통과했고, 두 호스트 모두 status revision `5aaadd3c938d`, detailed true, frontend_errors 0과 heartbeat 증가를 확인했다.

다음 이벤트는 로컬 flush 결과 외에 Sentry API로 실제 수신을 확인했다. release는 모두 `agent-gibbon@0.2.7+5aaadd3c938d`다.

| 호스트 | probe event ID |
| --- | --- |
| MacBook | `c8956301d8044987880d749ed4800f4f` |
| iMac | `a2710ef5f89d49569b137a37cb18737b` |

MacBook에서는 테스트 전용 terminal metadata `terminal-202611060001`을 대상 앱이 종료된 상태에서 만들고, 다시 시작한 PID `32101`의 개발 CLI로 새 터미널 열기·inspect·숨김·재열기를 확인했다. 논리 원점 `(120,120)`·크기 `(480,260)`에 대해 native 응답은 Retina의 물리 원점 `(240,240)`·크기 `(960,520)`였고 instance가 일치했다. `running:false`인 Welcome만 사용해 실제 셸/모델/사용자 작업은 시작하지 않았다. 종료 후 테스트 metadata 파일 하나만 소유 표식을 대조해 제거했고, 앱을 다시 실행한 최종 PID는 `35067`이다. 두 Mac 화면은 잠겨 있었으며 AX 조회도 막혔으므로 GUI 클릭·렌더링 회귀 통과로 기록하지 않는다.

Sentry Explore의 spans 데이터셋에서 새 stage를 조회하고 trace `74b70ae843ed58002ceb6ea49a44d276`의 실제 부모 관계를 확인했다. 조회 필드는 `parent_span`이며, `span.parent_id`/`parent_span_id`와 혼동하지 않는다.

| operation | span ID | parent span |
| --- | --- | --- |
| `ipc.open` | `b514a716c0f60f95` | 없음 |
| `terminal.window.open` | `3189e4402ad787fc` | `b514a716c0f60f95` |
| `terminal.window.create` | `e3ff8a9ac226ce67` | `3189e4402ad787fc` |
| `terminal.window.position` | `af63b7459aed724e` | `3189e4402ad787fc` |
| `terminal.window.show` | `adb366ed261c6c3a` | `3189e4402ad787fc` |

이 실제 수신은 창 생성 흐름의 증거다. 새 capture/PTY/input/output stage를 전부 실제로 실행·수신했다고 확대하지 않는다. SDK가 종료된 span만 전송하므로 별도 시작 stage 로그는 완료 이벤트가 아니라 시작 지점의 증거다.

## Windows와 실행 한계

Windows 작업자 직렬화·관리 창 재열기 보강은 앞선 커밋 `2d1b3b4`에 있으며 해당 코드의 Windows 실기 Rust 102개와 Clippy가 통과했다. 다만 실행 중이던 PID `20796`는 그 추가 보강 전 바이너리였다. 최신 커밋을 동기화하려던 시점에 Windows Orca(`192.168.0.100:6768`)가 연결되지 않아 명령은 전달되지 않았다. 따라서 Windows에 `5aaadd3`가 반영됐거나 최종 반복 GUI 검증이 통과했다고 기록하지 않는다.

Windows 원격 GUI 요청은 paired client에 `desktop-control` 권한이 없어 forbidden이었다. 사용자에게 권한 또는 직접 반복 시험 결과를 요청했다. 세부 계측의 Windows CI 검사·빌드는 실기 배포와 별개의 증거다. Windows 절전은 원인·해결 검증이 끝난 뒤라는 사용자 조건이 있으므로 연결 복구와 최종 검증 전에는 실행하지 않는다.

[CI 42075231357](https://github.com/egoing/agent-gibbon/actions/runs/42075231357)의 validate·macOS Intel/ARM64·Windows x64 검사와 설치 파일 빌드가 모두 성공했다. publish는 일반 main push이므로 건너뛰었다. 공개 태그·릴리스는 생성하지 않았다. Windows의 저장된 대상 주소로 깨우기 패킷을 한 번 보냈으나 이후 TCP 연결도 시간 초과였으며, 절전 또는 전원 종료 중 무엇인지 판정하지 않았다.

현재 증거는 MacBook `/tmp/gibbon-stage-*.json`, `/tmp/gibbon-stage-spans-*.log`, iMac `/tmp/gibbon-stage-spans-20261106/`에 있다. 원문 로그나 임시 terminal metadata는 저장소에 추가하지 않았다.

## 남은 범위

#19의 1·5·10개 창 부하 도구와 p50/p95·CPU/RSS·디스크 쓰기 기준, #64의 함수별 로컬 프로파일 수집·Windows Browser Profiling 지원 검증 전체를 완료한 것은 아니다. 두 이슈를 완료 처리하지 않는다. WebView 집계와 Rust 작업자 흐름이 하나의 end-to-end trace로 자동 연결된다고 주장하지 않는다.

Windows 반복 프리징의 실제 스택·Astra 독립 감사와 코드 보강은 [별도 검증 기록](windows-freeze-20261106.md)으로 관리한다. 운영 이름의 영어화는 Sentry operation·label에 적용하며 사용자 UI 문구나 프로젝트명을 임의로 바꾸지 않는다.
