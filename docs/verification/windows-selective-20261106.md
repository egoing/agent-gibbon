# 최신 main에 Windows 수정 선별 통합 (2026-11-06)

- 기준: `origin/main`의 `1383a64`. 원본 Windows 작업은 `3f692bb` 및 `backup/window-test-before-selective-20261106`에 보존했다.
- 작업 브랜치: `egoing/windows-selective-20261106`. 전체 병합의 충돌 상태를 취소하고 최신 main에서 필요한 코드만 통합한다. 원래 브랜치와 원격 이력을 덮어쓰지 않는다.
- 유지: main의 개인정보 동의, 캡처 생성·트레이 교착 수정, 터미널, 관리 창, 자동 배치, 전체 접어두기.
- 선별 대상: Windows 네이티브 창 크기·이미지 비율·캡처 client 좌표 보정, 비활성 표시와 포커스 보존, 창 반투명도, 보조키 접촉 감지, 로컬 IPC 경로 오류 안내 및 회귀 검사.
- 제외: 기존 세 보조키+포인터의 개별 숨김, 구형 캡처 완료 구현, 과거 검증 기록의 일괄 병합. 기존 기록은 이번 실행 증거로 사용하지 않는다.

## 실행 환경과 출처

- 루트: `C:\Users\egoin\orca\workspaces\agent-gibbon\window-test`
- 개발 인스턴스: `dev-9267983a444c0e3310b457f3`
- 바이너리: 루트의 `src-tauri\target\debug\agent-gibbon.exe`
- Node 20.11.0, Rust/Cargo 1.99.1 MSVC, Python 3.10.11.
- 최신 동의 정책을 우회하거나 사용자 대신 약관·진단 동의를 저장하지 않는다. 미동의 환경의 전체 GUI 회귀는 별도 범위다.

## 검증

- `cargo test --manifest-path src-tauri/Cargo.toml`: 126개 통과. 실제 숨긴 HWND에서 8방향 이미지 비율, 음수 좌표의 정확한 client 영역, 프레임 변경 후 크기 복원, 수동/프로그램 크기 변경 구분, 반투명 스타일 재설정 및 반복 외곽 크기 적용을 검사한다.
- `npm run test:ui`: 80개 통과, POSIX 실행 파일 fixture 1개는 Windows에서 제외. 최신 main의 Editor·관리 창·동의·터미널 테스트를 유지했다.
- `npm run check`: Clippy 전 타깃 `-D warnings` 통과.
- Rustfmt, `npm run version:check`(0.2.7), `git diff --check`, Node 검증 스크립트 구문 검사 통과.
- 최신 main의 캡처 worker를 유지하기 위해 HWND subclass·스타일·크기 접근을 소유 스레드로 전달한다. 설치된 `tauri-runtime-wry 2.12.1`의 `Context::send_user_message`가 메인 스레드 호출을 즉시 실행함을 소스로 확인했다.

- `npm run dev:build`: Windows x64 개발 바이너리 빌드 통과.
- `node scripts/windows-runtime-smoke.mjs --consent-check`: 통과. 위 절대 경로의 실행 프로세스 PID `28948`과 진단 응답의 인스턴스 ID를 대조했다. 긴 UTF-8 IPC 경로의 실행 전 오류, 미동의 진단 응답, 비활성 개발 probe, 메모 open/list/inspect/update/close 차단, 동의 파일 불변 및 테스트 메모 미생성을 확인했다. 해당 테스트가 시작한 프로세스만 정리했다.

실제 캡처 드래그·Editor·포커스 전환·stash 복원·다중 모니터/혼합 DPI·다른 운영체제의 이번 GUI 회귀는 미검증이다. 미동의 환경이므로 테스트 메모의 성공한 `inspect`와 화면 대조를 통한 GUI 출처 확인도 실행하지 않았다. `windows-capture-smoke.py`는 기존 테스트를 최신 빈 메모 창 제목에 맞춰 복원한 것이며, 파일을 추가한 사실을 실행 통과로 간주하지 않는다. 설치 파일 빌드·앱 설치·릴리스 공개는 이번 작업 범위에 포함하지 않는다.
