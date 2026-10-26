# 자동 릴리스

`.github/workflows/release.yml`은 `vX.Y.Z` 태그 push로 시작합니다. 소스의 버전만 변경하거나 일반 브랜치를 push하면 공개하지 않습니다. 브랜치 push와 `workflow_dispatch` 수동 실행은 검증·빌드 결과를 Actions artifact로 보관하며 릴리스를 만들지 않습니다. 수동 실행 버튼은 이 워크플로가 기본 브랜치에 병합된 뒤 사용할 수 있습니다.

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

macOS는 기존 애드혹 서명을 유지하며 Developer ID 서명·공증과 Windows 코드 서명은 구성하지 않았습니다. 사용자 설치 경고가 발생할 수 있습니다. 이 워크플로는 GitHub Releases 배포를 자동화하며 설치된 앱의 자동 업데이트 기능을 추가하지 않습니다.

공식 참고: [Tauri GitHub 배포](https://v2.tauri.app/distribute/pipelines/github/), [GitHub 호스팅 runner](https://docs.github.com/en/actions/reference/runners/github-hosted-runners), [GitHub CLI 릴리스 생성](https://cli.github.com/manual/gh_release_create).
