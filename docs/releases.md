# 자동 릴리스

`.github/workflows/release.yml`은 `v*` 태그 push 또는 `workflow_dispatch` 수동 요청으로 실행합니다. 일반 브랜치 push는 이 워크플로를 실행하지 않습니다. 수동 실행도 세 플랫폼의 검증·설치 파일·서명 updater 자산·`latest.json`·체크섬까지 만들고 완성 자산을 Actions artifact로 보관하지만 GitHub Release를 만들지는 않습니다. 수동 실행은 각 runner에서 일회용 updater 키를 만들어 빌드 검증에만 씁니다. 버전이 일치하는 `vX.Y.Z` 태그 push만 검증·빌드 성공 후 Draft를 준비합니다. 수동 실행을 요청하려면 이 워크플로가 기본 브랜치에 있어야 합니다.

## 필요할 때 전체 검사 요청

모든 변경에서 관련 로컬 검사를 수행합니다. 에이전트의 전체 검사 요청 기준은 [프로젝트 지침](../AGENTS.md#버전과-릴리스)에 따릅니다. 일반 개발·커밋·병합에서는 전체 플랫폼 빌드를 기본 실행하지 않습니다. 실제 배포 준비 또는 사용자가 명시적으로 전체 검사를 요청한 경우에만 실행하며, 그 외에는 변경에 맞는 로컬 검사와 미검증 범위를 보고합니다.

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

실행 중 새 커밋을 push했다면 기존 결과는 이전 SHA에만 유효합니다. 사용자가 새 커밋의 전체 검사를 명시적으로 요청했거나 실제 배포를 준비하는 경우에만 다시 실행합니다. 수동 검사는 세 플랫폼 테스트·검사·설치 파일 빌드를 모두 수행하고 artifact를 보관하지만, 공개용 체크섬 생성과 릴리스 게시 작업은 건너뜁니다. 검사용 태그를 만들지 않으며 `vX.Y.Z` 태그는 승인된 실제 배포에만 사용합니다.

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
- lockfile을 고정하여 Apple Silicon DMG, Intel DMG, Windows NSIS EXE와 세 플랫폼의 updater 자산을 빌드합니다. updater target은 `darwin-aarch64`, `darwin-x86_64`, `windows-x86_64`이며 Linux는 지원 범위에 포함하지 않습니다. CI는 화면 조작이나 실제 설치 검증을 대신하지 않습니다.
- 모든 빌드 성공 후 정확한 버전의 설치 파일과 updater 번들·서명 세트를 확인합니다. 서명은 Tauri updater가 읽는 Base64 minisign 형식이어야 하며, 실제 payload·trusted comment의 Ed25519 서명을 검증합니다. trusted comment는 manifest의 안정 버전·플랫폼 파일명과 일치해야 합니다. macOS updater 아카이브는 CPU별 이름으로 바꾼 뒤 다시 서명합니다. 앱은 `requireSignedVersion`을 켜 두어 실제 서명을 검증하고, 서명에 포함된 버전과 manifest 버전이 다르면 업데이트를 거부합니다. 자산이 모두 유효할 때 `latest.json`과 설치 파일·updater 번들·서명·플랫폼별 공개 키 파일의 `SHA256SUMS.txt`를 생성합니다.
- `latest.json`은 각 release에 업로드되는 정적 manifest입니다. `version`과 세 플랫폼별 `url`·Base64 `signature`를 담고, URL은 해당 `vX.Y.Z` GitHub Release의 updater 파일을 가리킵니다. 앱은 GitHub Releases의 `latest/download/latest.json`을 조회하므로 새 버전을 `--latest`로 공개하면 별도 업데이트 서버 없이 manifest와 번들을 가져옵니다.
- 게시 작업에만 `contents: write`를 부여합니다. Draft에 파일을 업로드하고 검증한 뒤 대기합니다. 태그 실행의 성공과 대상 버전을 확인하고, 사용자가 공개를 지시하면 `gh release edit vX.Y.Z --draft=false --latest`로 공개한 뒤 latest.json과 세 다운로드 URL을 확인합니다. 한 플랫폼이라도 실패하면 게시 작업이 실행되지 않습니다.
- 업로드 실패 시 남은 Draft는 자동 덮어쓰지 않습니다. 담당자가 실패 원인과 Draft를 확인하고 미공개 Draft를 정리한 후 실패 작업을 재실행합니다. 이미 공개한 버전은 새 버전으로 수정합니다.

## 자동 업데이트 서명 키와 최초 업데이트

자동 업데이트는 macOS ARM64, macOS Intel, Windows x64에서 지원합니다. 업데이트 확인은 정적 GitHub Release manifest를 사용하고 별도 서버를 추가하지 않습니다. updater 서명은 내려받은 번들의 출처와 manifest에 공지된 버전의 일치를 검증합니다. macOS Developer ID 서명·공증이나 Windows Authenticode 서명을 대신하지 않습니다.

공개 키와 개인 키는 서로 다른 용도로 관리합니다.

- GitHub Actions repository variable `TAURI_UPDATER_PUBLIC_KEY`에는 앱 바이너리에 컴파일할 Tauri updater 공개 키를 둡니다. 빌드 job은 이 값을 `TAURI_UPDATER_PUBLIC_KEY` 환경변수로 전달하고 Rust backend는 `option_env!`로 포함합니다. 공개 키는 `tauri.conf.json`에 비밀로 저장하지 않습니다.
- GitHub Actions repository secrets `TAURI_SIGNING_PRIVATE_KEY`와 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`는 updater 번들과 `.sig`를 생성하는 데 사용합니다. 개인 키는 소스, artifact, 로그, 캐시에 넣지 않습니다. 태그 릴리스는 개인 키와 공개 키가 설정되지 않으면 실패합니다. 암호 없는 키를 사용할 때 password secret은 빈 값이어도 됩니다.
- 장기 개인 키 생성과 보관은 릴리스 준비 작업입니다. 2026-11-07 사용자 승인으로 Mac Keychain(service `AGENT_GIBBON_TAURI_UPDATER`, account는 로컬 로그인 사용자)과 Actions secret/공개 variable에 등록했습니다. Enpass 백업 항목을 저장한 뒤 다시 열어 비밀 필드의 길이와 공개 키·복구 설명을 확인했습니다. 마스킹된 비밀 키의 원문 일치 및 실제 복구는 검증하지 않았습니다. 키를 잃으면 기존 설치본의 업데이트 서명을 만들 수 없으므로 두 백업의 접근성과 복구 가능성을 확인해야 합니다.
- `workflow_dispatch`는 각 플랫폼 runner에서 일회용 키를 만들고 빌드 및 manifest 검증 후 폐기합니다. 이 키는 공개되지 않고 다음 실행에도 재사용되지 않으므로 수동 실행 artifact는 실제 배포 키로 서명된 업데이트가 아닙니다.

`0.2.7` 이하 설치본에는 updater 공개 키가 포함되어 있지 않습니다. `0.2.8`은 다운로드 URL 오류가 있으므로, `0.2.8` 이하 사용자는 `0.2.9` 이상을 한 번 수동 설치해야 이후 버전부터 앱 내 업데이트를 받을 수 있습니다. 새 버전 공개 후에는 이전 정상 배포판에서 확인·다운로드·설치·재시작과 실제 실행 버전을 확인하고, 수행한 플랫폼과 미검증 플랫폼을 구분해 기록합니다.

## 오류 수집과 OS 서명

저장소 Actions secret 또는 variable `AGENT_GIBBON_SENTRY_DSN`에 공식 배포용 공개 수신 DSN을 설정하면 빌드 단계에만 주입합니다(secret 우선). 관리 API 토큰을 넣지 않습니다. 값이 없으면 경고를 표시하고 오류 수집이 꺼진 설치 파일을 만듭니다. 소스에는 개인 계정·서버 정보나 DSN을 넣지 않습니다.

오류·성능·로그·릴리스 상태·응답 감시는 별도 사용자 동의 전에는 켜지지 않는다. 운영 정보가 확정된 빌드에서 동의한 뒤에만 허용된 진단을 활성화하며, 개발 빌드는 명시적 진단 실행도 필요하다. 수집 끄기와 공헌자 빌드 경계는 [Sentry 진단 안내](diagnostics.md)를 따른다. 관리 토큰·진단 원문은 빌드 캐시나 릴리스 자산에 포함하지 않는다.

macOS 배포판은 기존 애드혹 코드 서명을 유지하며 Developer ID 서명·공증과 Windows 코드 서명은 구성하지 않았습니다. Tauri updater의 minisign 서명은 설치 경고를 없애거나 OS 코드 서명을 대체하지 않습니다. 사용자 설치 경고가 발생할 수 있습니다.

공식 참고: [Tauri GitHub 배포](https://v2.tauri.app/distribute/pipelines/github/), [GitHub 호스팅 runner](https://docs.github.com/en/actions/reference/runners/github-hosted-runners), [GitHub CLI 릴리스 생성](https://cli.github.com/manual/gh_release_create).

## 빌드 캐시

npm 캐시와 별도로 `Swatinem/rust-cache`를 Rust 도구 설치 뒤에 실행합니다. Cargo 다운로드와 `src-tauri/target`의 의존성 컴파일 결과를 GitHub Actions 캐시 저장 공간에 보관합니다. 캐시는 Git 소스나 릴리스 자산에 포함되지 않습니다.

OS·CPU·대상 triple·Rust 도구 버전·Cargo 매니페스트와 lockfile 등에 따라 캐시를 구분합니다. 앱 자체와 설치 파일은 매번 다시 빌드하며, 테스트·Clippy도 생략하지 않습니다. 캐시가 없거나 호환되지 않으면 전체 빌드를 수행하므로 첫 실행은 캐시를 채우는 시간이 추가될 수 있습니다.

캐시는 GitHub의 브랜치 접근 범위를 따릅니다. 다른 브랜치의 캐시를 항상 쓸 수 있는 것은 아니며, 기본 브랜치에서 생성한 캐시는 후속 브랜치·태그 실행이 재사용할 수 있습니다. 캐시 삭제·용량 제한·도구나 의존성 변경으로 다시 생성될 수 있습니다. 인증 정보·환경 파일·서명 키는 캐시 대상으로 추가하지 않습니다.

참고: [Rust cache 공식 설명](https://github.com/Swatinem/rust-cache), [GitHub Actions 캐시 접근 범위](https://docs.github.com/en/actions/using-workflows/caching-dependencies-to-speed-up-workflows#restrictions-for-accessing-a-cache).

## 첫 실행 동의와 운영 정보

DSN이 있는 공식 빌드도 현재 정책에 대한 별도 전송 동의가 없으면 SDK를 초기화하지 않는다. 공식 운영 고지는 원본 최대 30일·백업 생성 후 30/90일·오류 누적 통계의 별도 보유를 반영해 선택 동의를 활성화했다. Release Health 세션 수집은 보유기간 확인 전 차단한다. 포크/사용자 빌드에서 운영 정보가 미완성이면 `src-tauri/src/privacy/notice.json`의 readiness 검사로 전송을 막는다. 정식 운영 정보와 개인정보 안내를 함께 확정하고 서버의 보유기간·IP 설정을 실제로 확인한 뒤 새 정책 버전으로 동의를 받아야 한다. 약관 동의만으로 진단 전송을 허용하지 않는다.

## 0.2.8 공개 후 파일명 검증과 0.2.9

GitHub는 업로드 이름의 공백을 점으로 정규화했다. 0.2.8의 manifest는 공백을 인코딩한 URL을 생성해 세 플랫폼 모두 404가 발생했으며, 실제 공개 파일 다운로드 확인으로 발견했다. 기존 태그·자산을 덮어쓰지 않고 0.2.8을 사전 릴리스로 전환했다. 0.2.9부터는 `Agent.Gibbon_...` 형태를 빌드 자산·서명 trusted filename·manifest·앱 검증에 공통 적용한다. 배포할 때 GitHub에 올라간 실제 자산명과 manifest URL도 검증한다. 0.2.8을 설치한 경우에도 0.2.9를 한 번 수동 설치해야 한다.

### 설치 보호 실기기 시험

격리된 이전 버전과 후보를 모두 Cargo feature `verification-updater`로 빌드한다. 두 앱에 정식 배포와 다른 동일한 bundle identifier와 설치 경로를 사용한다. 이 feature는 검증용 메모 저장소와 CLI 인스턴스를 선택한다. `GIBBON_VERIFICATION_UPDATE_URL`은 빌드 시 주입하며 피드와 같은 출처에서 `v<버전>/<플랫폼 파일명>`을 제공한다. HTTP 시험은 기기 자체의 `127.0.0.1`에 한정하고 해당 검증 빌드에만 updater의 로컬 HTTP 허용 옵션을 지정한다.

이전 버전과 후보의 `TAURI_UPDATER_PUBLIC_KEY`와 Tauri 설정의 `plugins.updater.pubkey`에 같은 일회성 공개키를 주입한다. 후보는 정식 절차와 같은 플랫폼 파일명으로 바꾼 뒤 해당 키 쌍으로 다시 서명한다. 시험 중 후보를 바꾸면 이전 검증 캐시를 격리하고, 노출할 manifest 버전과 HTTP 응답 파일의 SHA-256을 대조한다.

기본 검증 간격은 최초 확인 15초, 이후 확인 60초, OS 무입력 30초, 설치 재시도 5초다. 최종 확인에도 무입력 30초를 사용한다. 배포판의 5분 대기 자체를 별도로 시험하려면 빌드 시 `GIBBON_VERIFICATION_STANDARD_TIMING=1`을 지정한다. 앱의 `diagnostics`에서 실제 시간값, 실행 버전과 설치 준비 응답을 확인한다. 이전 버전의 안정화된 메모 상태를 기록한 다음 피드에 후보 버전을 노출한다.

정식 빌드는 시작 15초 뒤 최초 확인을 시도하고, 성공·실패 여부와 관계없이 각 자동 확인 완료 후 15분 주기로 다시 확인한다. 설치 준비 재검사는 15초 간격이고 무입력 기준은 5분이다. 검증 프로필은 60초 확인·5초 재검사·30초 무입력을 사용한다.
