# 자동 릴리스

`.github/workflows/release.yml`은 `v*` 태그 push 또는 `workflow_dispatch` 수동 요청으로 실행합니다. 일반 브랜치 push는 이 워크플로를 실행하지 않습니다. 수동 실행은 전체 플랫폼 검증·빌드 결과를 Actions artifact로 보관하며 릴리스를 만들지 않습니다. 버전이 일치하는 `vX.Y.Z` 태그 push만 검증·빌드 성공 후 공개로 이어집니다. 수동 실행을 요청하려면 이 워크플로가 기본 브랜치에 있어야 합니다.

## 필요할 때 전체 검사 요청

모든 변경에서 관련 로컬 검사를 수행합니다. 에이전트의 전체 검사 요청 기준은 [프로젝트 지침](../AGENTS.md#버전과-릴리스)에 따릅니다. OS별 코드·전역 입력·창 제어·의존성·빌드 및 배포 설정 변경 등에 전체 검사가 필요하며, 문서·단순 문구 변경은 관련 로컬 검사로 마칠 수 있습니다.

검토할 커밋을 작업 브랜치에 push한 뒤 요청합니다. 아래 `ci-on-demand`는 실제 작업 브랜치로 바꿉니다.

```sh
git rev-parse HEAD
git push -u origin ci-on-demand
gh workflow run release.yml --ref ci-on-demand
gh run list --workflow release.yml --branch ci-on-demand --event workflow_dispatch --limit 10 --json databaseId,url,headSha,status,conclusion
```

요청 이유·브랜치·커밋 SHA를 남기고, 목록에서 해당 요청의 실행 URL과 `headSha`가 대상 커밋과 일치하는지 확인합니다. 요청 수락만으로 검증 통과로 보고하지 않습니다. 확인한 실행 ID로 완료와 플랫폼별 결과를 조회합니다.

```sh
gh run watch <실행-ID> --exit-status
gh run view <실행-ID> --json url,headSha,status,conclusion,jobs
```

실행 중 새 커밋을 push했다면 기존 결과는 이전 SHA에만 유효합니다. 여전히 전체 검사가 필요한 변경이면 다시 요청합니다. 수동 검사는 세 플랫폼 테스트·검사·설치 파일 빌드를 모두 수행하고 artifact를 보관하지만, 공개용 체크섬 생성과 릴리스 게시 작업은 건너뜁니다. 검사용 태그를 만들지 않으며 `vX.Y.Z` 태그는 승인된 실제 배포에만 사용합니다.

참고: [GitHub 수동 실행 안내](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/manually-run-a-workflow).

## 버전 올리기

Windows 지원 및 릴리스 워크플로를 기본 브랜치에 병합한 뒤, 새 버전을 준비합니다.

```sh
npm run version:set -- 0.2.8
npm run version:check
```

스크립트는 `package.json`, `package-lock.json`의 루트·앱 항목, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`의 앱 항목, `src-tauri/tauri.conf.json`을 함께 변경합니다. 현재보다 높은 안정 버전만 허용합니다. 변경을 검토하고 테스트한 뒤 버전 커밋을 push하고 그 커밋에 태그를 붙입니다.

```sh
git tag -a v0.2.8 -m 'Agent Gibbon 0.2.8 배포'
git push origin v0.2.8
```

예시 버전은 실제 다음 버전에 맞춰 바꿉니다. 태그와 모든 버전 필드가 일치하지 않으면 빌드를 시작하지 않습니다. 기존 태그를 이동하거나 기존 릴리스 파일을 덮어쓰지 않습니다.

## 검증과 공개 조건

- macOS ARM64(`macos-15`), macOS Intel(`macos-15-intel`), Windows x64(`windows-2022`)에서 각각 Rust·Node 테스트, Rustfmt, Clippy를 실행합니다.
- lockfile을 고정하여 Apple Silicon DMG, Intel DMG, Windows NSIS EXE를 빌드합니다. CI는 화면 조작이나 실제 설치 검증을 대신하지 않습니다.
- 모든 빌드 성공 후 정확히 해당 버전의 설치 파일 세 개가 있는지, 파일이 비어 있지 않은지 확인하고 `SHA256SUMS.txt`를 생성합니다.
- 게시 작업에만 `contents: write`를 부여합니다. Draft에 파일을 모두 업로드한 뒤 공개합니다. 한 플랫폼이라도 실패하면 게시 작업이 실행되지 않습니다.
- 업로드 실패 시 남은 Draft는 자동 덮어쓰지 않습니다. 담당자가 실패 원인과 Draft를 확인하고 미공개 Draft를 정리한 후 실패 작업을 재실행합니다. 이미 공개한 버전은 새 버전으로 수정합니다.

## 오류 수집과 서명

저장소 Actions secret 또는 variable `AGENT_GIBBON_SENTRY_DSN`에 공식 배포용 공개 수신 DSN을 설정하면 빌드 단계에만 주입합니다(secret 우선). 관리 API 토큰을 넣지 않습니다. 값이 없으면 경고를 표시하고 오류 수집이 꺼진 설치 파일을 만듭니다. 소스에는 개인 계정·서버 정보나 DSN을 넣지 않습니다.

오류·성능·로그·릴리스 상태·응답 감시는 별도 사용자 동의 전에는 켜지지 않는다. 운영 정보가 확정된 빌드에서 동의한 뒤에만 허용된 진단을 활성화하며, 개발 빌드는 명시적 진단 실행도 필요하다. 수집 끄기와 공헌자 빌드 경계는 [Sentry 진단 안내](diagnostics.md)를 따른다. 관리 토큰·진단 원문은 빌드 캐시나 릴리스 자산에 포함하지 않는다.

macOS는 기존 애드혹 서명을 유지하며 Developer ID 서명·공증과 Windows 코드 서명은 구성하지 않았습니다. 사용자 설치 경고가 발생할 수 있습니다. 이 워크플로는 GitHub Releases 배포를 자동화하며 설치된 앱의 자동 업데이트 기능을 추가하지 않습니다.

공식 참고: [Tauri GitHub 배포](https://v2.tauri.app/distribute/pipelines/github/), [GitHub 호스팅 runner](https://docs.github.com/en/actions/reference/runners/github-hosted-runners), [GitHub CLI 릴리스 생성](https://cli.github.com/manual/gh_release_create).

## 빌드 캐시

npm 캐시와 별도로 `Swatinem/rust-cache`를 Rust 도구 설치 뒤에 실행합니다. Cargo 다운로드와 `src-tauri/target`의 의존성 컴파일 결과를 GitHub Actions 캐시 저장 공간에 보관합니다. 캐시는 Git 소스나 릴리스 자산에 포함되지 않습니다.

OS·CPU·대상 triple·Rust 도구 버전·Cargo 매니페스트와 lockfile 등에 따라 캐시를 구분합니다. 앱 자체와 설치 파일은 매번 다시 빌드하며, 테스트·Clippy도 생략하지 않습니다. 캐시가 없거나 호환되지 않으면 전체 빌드를 수행하므로 첫 실행은 캐시를 채우는 시간이 추가될 수 있습니다.

캐시는 GitHub의 브랜치 접근 범위를 따릅니다. 다른 브랜치의 캐시를 항상 쓸 수 있는 것은 아니며, 기본 브랜치에서 생성한 캐시는 후속 브랜치·태그 실행이 재사용할 수 있습니다. 캐시 삭제·용량 제한·도구나 의존성 변경으로 다시 생성될 수 있습니다. 인증 정보·환경 파일·서명 키는 캐시 대상으로 추가하지 않습니다.

참고: [Rust cache 공식 설명](https://github.com/Swatinem/rust-cache), [GitHub Actions 캐시 접근 범위](https://docs.github.com/en/actions/using-workflows/caching-dependencies-to-speed-up-workflows#restrictions-for-accessing-a-cache).

## 첫 실행 동의와 운영 정보

DSN이 있는 공식 빌드도 현재 정책에 대한 별도 전송 동의가 없으면 SDK를 초기화하지 않는다. 공식 운영 고지는 원본 최대 30일·백업 생성 후 30/90일·오류 누적 통계의 별도 보유를 반영해 선택 동의를 활성화했다. Release Health 세션 수집은 보유기간 확인 전 차단한다. 포크/사용자 빌드에서 운영 정보가 미완성이면 `src-tauri/src/privacy/notice.json`의 readiness 검사로 전송을 막는다. 정식 운영 정보와 개인정보 안내를 함께 확정하고 서버의 보유기간·IP 설정을 실제로 확인한 뒤 새 정책 버전으로 동의를 받아야 한다. 약관 동의만으로 진단 전송을 허용하지 않는다.
