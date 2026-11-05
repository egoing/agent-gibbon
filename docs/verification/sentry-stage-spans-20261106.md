# Sentry 단계 계측 세분화 — 2026-11-06

## 범위

사용자가 요청한 후속 작업으로 #19(터미널 단계별 계측)·#64(성능 진단)의 trace/span 세분화 부분을 반영한다. 모든 새 operation·stage label은 고정된 영어 이름이다. 작업 목록과 의미는 [진단 안내](../diagnostics.md#단계별-tracespan-이름)를 따른다.

캡처의 모니터 열거·화면 획득·Overlay 생성·픽셀 복사·선택 검증·PNG 인코딩·파괴 요청/대기, Windows 직렬화 잠금 대기, 터미널 창의 생성/배치/표시, 명령 준비·실제 탐색·PTY 할당·프로세스 생성·I/O 연결·세션 잠금·쓰기·출력 필터/IPC 전달을 분리했다. WebView의 초기 준비·첫 출력·xterm 파싱 큐 처리는 2초 단위 숫자 집계로 측정한다. xterm callback을 GPU 페인트 완료나 함수별 프로파일로 설명하지 않는다.

창 생성·프로세스 생성·파괴 대기에는 시작 로그를 남겨 끝나지 않은 span의 직전 단계를 찾을 수 있게 했다. 현재 Rust trace의 무작위 trace ID를 structured log와 연결하며, 메모/사용자 식별자는 추가하지 않았다. 기존 privacy envelope 필터·분당 한도·상세 모드 opt-in을 유지한다. 별도의 업로드 경로나 서버·브라우저 SDK를 추가하지 않았다.

## 검증

- MacBook root `/Users/egoinglee/dev/project/agent-gibbon`, instance `dev-a5210ae16e79437d38490c8f`, binary는 root의 `src-tauri/target/debug/agent-gibbon`이다.
- Rust 전체 116개와 UI 74개가 통과했다. 새 프런트 검사에서는 일반 모드의 측정 API 부재, 영어 stage 허용 목록, 원문·임의 이름·무한값 차단, 숨긴 창 제외를 검증했다.
- 코드·검사와 실제 native 실행·Sentry 수신은 구분한다. 최초 구현 커밋에서는 새 span의 실제 수신과 원격 배포를 통과로 기록하지 않는다. 재빌드·재시작 후 root/instance/PID/메모와 신규 release를 다시 확인해 후속 기록한다.

## 남은 범위

#19의 1·5·10개 창 부하 도구와 p50/p95·CPU/RSS·디스크 쓰기 기준, #64의 함수별 로컬 프로파일 수집·Windows Browser Profiling 지원 검증 전체를 완료한 것은 아니다. 두 이슈를 완료 처리하지 않는다. WebView 집계와 Rust 작업자 흐름이 하나의 end-to-end trace로 자동 연결된다고 주장하지 않는다.

Windows 반복 프리징의 실제 스택·Astra 독립 감사와 코드 보강은 [별도 검증 기록](windows-freeze-20261106.md)으로 관리한다. 운영 이름의 영어화는 Sentry operation·label에 적용하며 사용자 UI 문구나 프로젝트명을 임의로 바꾸지 않는다.
