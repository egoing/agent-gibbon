# 프로젝트 지침

- 한국어로 소통하고 커밋 제목과 본문을 한국어로 작성한다.
- README의 현재 지원 범위를 지킨다. 상세 설계는 실제 환경과 공식 Tauri 문서를 확인하여 결정한다.
- Rust 백엔드와 가벼운 WebView 프론트엔드를 사용한다. 창 ID 기반 다중 메모를 지원하되 서버는 추가하지 않는다. 앱과 기본 사용 스킬은 워크플로우 중립성을 유지한다.
- 로컬 CLI 통신에는 Unix socket 또는 원자적 파일 교환 등 로컬 방식만 사용한다. 비밀정보를 저장하거나 출력하지 않는다.
- 사용자 작업의 포커스, 창 위치, 숨김 상태를 보존한다.
- 설치된 도구 버전을 직접 확인한다. 환경은 Intel Mac일 수 있으므로 arm64를 가정하지 않는다.
- 구현 후 가능한 빌드와 실제 CLI 동작을 검증하고 실행하지 못한 검증은 명시한다.
- 이번 작업 파일만 명시적으로 스테이징한다. 사용자 승인 없이 공개 저장소로 변경하지 않는다.

## 워크트리별 개발 실행

- 개발 앱은 `npm run dev`로 실행하거나, `npm run dev:build` 후 `./scripts/agent-gibbon-dev open`으로 시작한다. 개발 메모 조회·갱신·숨기기에도 반드시 `scripts/agent-gibbon-dev`를 사용한다. 일반 `scripts/agent-gibbon`은 설치 앱의 기본 통신 경로를 사용하므로 개발 테스트에 대신 사용하지 않는다.
- 개발 실행기는 워크트리의 실제 절대 경로 해시로 인스턴스 ID를 정하고 Unix socket과 중복 실행 잠금을 분리한다. `npm run dev:info`로 ID와 실행 파일 경로를 확인한다. 환경변수는 실행기가 설정하며, 개발 CLI는 해당 워크트리에서 빌드한 debug 바이너리만 사용한다. 바이너리가 없거나 출처가 다르면 다시 빌드하며 설치본·release로 대체하지 않는다.
- 기존 설치 앱과 다른 워크트리의 프로세스를 종료하지 않는다. 검증에서 직접 시작한 프로세스만 정확한 바이너리 경로와 PID로 식별해 종료한다. `close`는 메모 숨기기이며 프로세스 종료가 아니다. 사용자 포커스·창 위치·숨김 상태를 보존한다.
- 같은 메모 ID는 인스턴스별로 독립적이지만 `--file`로 연결한 원본 파일은 공유될 수 있으므로 테스트 파일도 워크트리별로 분리한다.
- 격리 회귀는 두 워크트리를 각각 빌드한 뒤 `python3 scripts/isolation-smoke.py --peer /다른/워크트리/절대경로`로 확인한다. 두 개발 앱은 실행되지 않은 상태여야 하며 기존 기본 앱은 실행 중이어야 한다. 상세 실행법은 [README](README.md), CLI 사용법은 [로컬 스킬](.agents/skills/agent-gibbon/SKILL.md), 검증 범위와 한계는 [검증 기록](docs/verification.md)을 따른다.

## 버전과 릴리스

- 상세 절차와 현재 배포 조건은 [자동 릴리스 안내](docs/releases.md), 실행 설정은 `.github/workflows/release.yml`을 따른다.
- 버전은 `npm run version:set -- X.Y.Z`로 npm·Cargo·Tauri와 lockfile을 함께 올리고 `npm run version:check`로 확인한다. 버전 변경만으로 공개하지 않으며, 배포할 커밋에 일치하는 `vX.Y.Z` 태그를 push하면 자동 릴리스가 시작된다. 기존 태그·공개 릴리스 자산을 덮어쓰지 않는다.
- macOS ARM64·Intel DMG와 Windows x64 NSIS EXE의 테스트·Rustfmt·Clippy·빌드가 모두 성공하고 설치 파일 세 개와 체크섬이 준비된 뒤에만 공개한다. 일반 브랜치 push와 수동 실행은 검증·빌드만 수행한다.
- 플랫폼별 Rust 의존성 캐시는 GitHub Actions에 저장한다. 캐시 적중 여부와 관계없이 테스트·검사 및 앱·설치 파일 재빌드를 유지하며, 인증 정보·환경 파일·서명 키는 캐시에 넣지 않는다.
- 공식 오류 수집 DSN은 Actions secret/variable `AGENT_GIBBON_SENTRY_DSN`으로 빌드 시 주입한다. 누락 시 수집이 꺼진 빌드를 만들며, Developer ID 서명·공증·Windows 코드 서명은 별도 구성 사항이다. 실제 실행하지 않은 설치·UI·공개 배포 검증을 통과로 기록하지 않는다.
