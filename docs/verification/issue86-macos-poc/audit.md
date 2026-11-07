# Mac PoC 독립 증거 감사 — 2026-11-07

검토자: 별도 컨텍스트 Codex 작업자 `/root/macos_evidence_audit`. 구현 대화 이력 없이 사용자 요구사항·작업 루트·기준 SHA/ref와 이번 증거 파일만 전달했다. GUI 재실행·파일 수정·커밋은 하지 않았다.

## 판정

**현재 Mac PoC의 topmost FAIL 판정 수용.** 테스트 완화·오류 은폐·지원 범위 과장은 발견하지 못했다. Mac 전체가 불가능하다는 증명으로 확대하지 않는다. 제품 지원 게이트는 닫힌 상태다.

- manifest 원본 18개 해시 및 최종/attempt1~3 `sourceHash`가 보존 Swift 소스와 일치한다. 캡처별 JSON도 result의 해당 관측과 일치한다.
- 세 PNG를 직접 열어 A 입력 문자열 및 B가 A를 가리는 화면을 확인했다. `raiseResult=0`에도 frontPID는 B이고 CG 순서 B→A다 (`result.json:742`, `:786`, `:834`).
- `probe.swift:60`의 CGEvent 마우스/Unicode key 전송, 별도 프로세스 NSTextField 값·버튼 action 횟수를 대조했다. AXValue 직접 쓰기로 대체하지 않았다 (`probe.swift:15`, `run.py:43`).
- attempt1/2의 실제 foreground PID assertion 실패가 INCOMPLETE로 보존됐다. attempt3→최종은 assertion을 유지하면서 B 활성화 후 raise 및 노출 부분 클릭을 추가했다.
- 자신의 Popen 프로세스만 종료한다 (`run.py:33`, `:66`). 최종 두 PID 종료 -15와 원래 frontPID 16242/포인터 `(708,1002)` 일치를 확인했다 (`result.json:972`, `:985`).
- pin/unpin SKIP과 Intel/다른 macOS/임의 앱 전체 미검증을 구분했다.

## 지적과 반영

1. 저장소 밖으로 cd해 실행하면 `git rev-parse HEAD`가 실패한다. **반영:** README를 저장소 cwd에서 절대경로 빌드·실행하는 예시로 변경했다.
2. 실행기 topmost는 즉시 관측으로만 계산하고 1초 뒤 관측은 기록만 한다. status는 실패 문자열 고정이므로 향후 PASS 환경의 범용 판정에 부적합하다. **반영:** 원본 소스를 바꾸지 않고 README에 제한을 명시했다. 이번 기록은 두 시점 모두 FAIL이어서 판정 영향 없다.
3. 당시 독립 빌드 로그/시작 시 바이너리 해시 부재로 소스→바이너리 연결을 완전히 증명하지 못한다. **반영:** 이 제한을 README에 명시하고 현재 보존 바이너리의 사후 해시/mtime/명령을 provenance.json에 추가했다. 사후 해시를 시작 시점 증명으로 취급하지 않는다.

이 감사는 Mac 추가 PoC 증거 검토이며 기존 Windows 제품 감사·게이트·CI의 새 실행이 아니다. 파일 줄 번호는 감사 당시 원본 기준이다.
