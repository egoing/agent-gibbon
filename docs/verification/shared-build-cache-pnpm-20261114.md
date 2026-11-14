# 중앙 빌드 캐시·pnpm 비교

기준 SHA `40b440c` (제품은 `87aedf7`과 동일, 계획 문서만 추가). 두 실제 관리 워크트리는 `/Users/egoinglee/.codex/worktrees/ae2a/agent-gibbon`와 `/Users/egoinglee/.codex/worktrees/gibbon-cache-bench-peer/agent-gibbon`이다.

## 환경과 정의

- macOS arm64, 10코어, RAM 32GiB. cargo/rustc 1.91.1, Node24.21.0, npm11.19.0, pnpm11.25.0.
- 전용 측정 루트 `/tmp/gibbon-bench-20261114`. 기존 사용자 캐시·앱은 삭제하거나 종료하지 않았다.
- Cargo cold는 **중간·최종 산출물 폴더가 비어 있음**이다. 기존 registry 소스는 유지하고 `--offline --locked --jobs 4 --timings`를 동일 적용한다. OS 페이지 캐시는 비우지 않는다.
- JS cold는 빈 전용 npm cache/pnpm store 및 node_modules이다. registry.npmjs.org 네트워크를 사용한다. warm은 node_modules만 제거하고 동일 store/cache를 재사용하며 `--offline`로 설치한다. pnpm import 저장소는 설치 측정과 분리한다.
- JS cold n=1, warm n=3. Cargo 첫 빌드 각 n=1, warm n=3, 변경·동시 각 n=1로 먼저 비교한다. 작은 차이는 일반화하지 않는다.
- 기존 다른 작업이 실행 중인 공유 Mac이다. 시작 근처 load average(1/5/15분)는 9.09/20.15/19.27로, 스케줄러·열·페이지 캐시 및 네트워크 변동을 완전히 통제하지 않았다. JS 초기 설치는 첫 Cargo 빌드와 겹쳤다.
- Python monotonic wall 시간을 측정한다. 초기 independent-a 한 건만 Cargo Finished wall을 사용한다(출력 정밀도 1초).

## JS 설치 최소 비교

| 명령 | cold 1회 | warm 3회 | 중앙값 |
|---|---:|---|---:|
| npm ci --no-audit --no-fund | 0.844s | 0.451 / 0.411 / 0.462s | 0.451s |
| pnpm install --frozen-lockfile | 0.964s | 0.500 / 0.502 / 0.471s | 0.500s |

pnpm은 cold 14.2%, warm 중앙값 10.9% 느렸다. 이 프로젝트에서 설치 속도 향상을 확인하지 못했으며, pnpm 전환 자체는 사용자 요청 범위다. n=1 cold/공유 환경/작은 절대 차이이므로 일반적인 우열로 해석하지 않는다.

`pnpm import`는 1.440s(설치 시간에서 제외). import 결과 14개 package 이름·버전·integrity가 기존 lock과 모두 일치한다. 직접 의존성은 @xterm/addon-fit0.11.0, @xterm/xterm6.0.0, @tauri-apps/cli2.12.1이다.

논리 할당량(`du -sk`): npm node_modules 21,424KiB + cache9,436KiB; pnpm node_modules21,444KiB + store21,472KiB. pnpm은 hardlink 또는 APFS CoW clone으로 공유할 수 있어 이 합계는 고유 물리 점유량·절감량이 아니다. 실제 여유 공간 차이를 통제 측정하지 않았으며 공간 절감 수치는 주장하지 않는다.

## Cargo 결과

| 상황 | 독립 build-dir | 공유 build-dir | 해석 |
|---|---:|---:|---|
| A 첫 빌드 | 88.0s | 92.882s | 공유 첫 빌드 자체는 빨라지지 않음 |
| B 첫 빌드 | 86.083s | 15.031s | 82.54% 단축, 5.73배 |
| A 무변경 3회 | 3.713 / 0.413 / 0.503s | 14.120 / 0.700 / 0.396s | 중앙값 0.503 / 0.700s |
| A 작은 변경 | 2.354s | 2.532s | 공유가 7.6% 느림, 절대 차이 작음 |
| A/B 동시 완료 | 3.499s | 15.340s | 공유가 4.38배 오래 걸림 |

작은 변경은 `src-tauri/src/main.rs` 끝에 동일 주석 한 줄을 추가하고 빌드한 뒤 원래 바이트·mtime을 복원했다. 각 조건은 별도 cache를 쓰며 동일 source SHA와 옵션을 유지했다. warm 첫 회에 앱 재컴파일이 포함되어 있으므로 이후 두 회만의 완전 안정 no-op 측정과 구분한다. 최초 빌드 각 n=1이므로 개선율은 이 실행의 관측치이며 통계적 보장으로 해석하지 않는다.

첫 A와 독립 B는 각각 `Compiling` 369개, 공유 B는 1개(앱)였다. 즉 로그상 의존성 368/368개를 재사용했고 전체 컴파일 항목 중 재사용 비율은 368/369=99.73%다. 공유 B의 binary는 자신의 manifest 경로를 내장했고 A/B 최종 파일 경로와 inode는 서로 다르다. 중앙에 모이는 것과 실제 재사용을 이 근거로 구분했다.

추가 `-vv` + `CARGO_LOG=cargo::core::compiler::fingerprint=info` 진단에서 A/B 전환 시 `RerunIfChangedOutputPathsChanged`와 `UnitDependencyInfoChanged`를 확인했다. build script의 Git HEAD 및 Tauri `tauri.conf.json`·`capabilities` 절대 경로가 워크트리별로 달라 앱을 재컴파일한다. 경로 정보를 제거하거나 fingerprint를 강제로 속이지 않았다. 원본 진단 로그는 `/tmp/gibbon-bench-20261114/fingerprint-switch-a.log`다.

동시 빌드 시 독립 조건은 Cargo registry의 `package cache` 잠금, 공유 조건은 추가로 `build directory` 잠금 메시지가 있었다. 두 조건 모두 종료 코드0이며 각 최종 경로를 유지했다. 개별 동시 프로세스 시간은 순차 wait 관측의 상한이므로 비교에는 두 프로세스의 전체 완료 시간을 사용한다. 차이는 잠금뿐 아니라 경로 의존 앱 재컴파일도 포함하며 잠금만의 비용으로 단정하지 않는다.

첫 두 빌드 직후 `du` 할당량: 독립 cache 합계5,716,058,112B, 공유 cache3,043,479,552B(논리적 차이 약2.49GiB/46.8%). B 추가 시 공유 cache 증가184,336,384B(약175.8MiB). 각 최종 target 약68–69MiB. 이후 변경·교차 빌드로 cache가 더 증가했다. APFS clone/하드링크와 다른 앱의 동시 디스크 사용을 통제하지 않았으므로 **실제 물리 절감량은 미측정**이다.

[환경](shared-build-cache-pnpm-20261114/metadata.json), [Cargo 원시 수치](shared-build-cache-pnpm-20261114/cargo-results.json), [JS 원시 수치](shared-build-cache-pnpm-20261114/js-results.json), [lock 비교](shared-build-cache-pnpm-20261114/lock-comparison.json). 전체 명령·compiler 로그는 로컬 측정 루트에, Cargo timings HTML은 그 안의 `timings/`에 남긴다. 측정 완료 후 전용 중간/최종 바이너리 폴더8개만 제거했으며 기존 사용자 캐시를 정리한 것이 아니다.

## 기존 결합 target 보완 측정

통제 비교의 독립 조건도 `build-dir`와 `target-dir`를 각자 분리했으므로, 기존 기본 Cargo 배치와 동일한 결합 target 조건도 추가 확인했다. `CARGO_BUILD_BUILD_DIR`를 제거하고 새 `/tmp/gibbon-bench-20261114/original-layout`만 `--target-dir`로 지정했다. 나머지 옵션은 같으며 Rust/frontend의 Git diff는 비어 있었다(계획 SHA40b440c, JS 실행기·패키지 메타데이터만 로컬 변경).

[결합 target 원시 수치](shared-build-cache-pnpm-20261114/original-layout-results.json): 첫99.967s(n=1), 무변경[2.705,0.383,0.368]s(중앙값0.383s). 이 보완 측정 후반에는 두앱 기능 검증이 겹쳤고 다른 팀 CPU 작업도 있어 통제 비교와 직접 동일 조건의 속도 비율로 사용하지 않는다. 새 워크트리의 의존성 컴파일을 생략하는 효과와 원래 방식의 안정 no-op가 이미 빠르다는 점을 함께 확인한다.

## 적용 범위와 선택

- package의 dev/build/check/test 실행기는 Git common-dir의 `agent-gibbon/build-cache`를 기본 중간 폴더로 사용한다. 최종 `CARGO_TARGET_DIR`는 워크트리 `src-tauri/target`으로 고정한다.
- 명시적인 `CARGO_BUILD_BUILD_DIR`는 별도 캐시·CI를 위해 유지한다. Git 없는 소스는 로컬 target을 사용한다. Cargo1.91 미만은 잘못된 공유 성공으로 보이지 않도록 빌드 전에 오류를 낸다. 직접 Cargo 호출은 실행기를 거치지 않으므로 이 정책 적용 범위가 아니다.
- pnpm11.25.0과 Node22.13+를 고정하고 CI는 pnpm/action-setup + pnpm store cache + frozen lockfile을 사용한다. 단일 체크아웃 CI는 build-dir를 기존 Rust cache action의 target 경로에 맞춘다.
- pnpm lock에는 앱 자체 버전 필드가 없으므로 version:set은 package/Cargo/Tauri 버전만 갱신하고 pnpm 의존성 해석은 보존한다.
- 기존 target·사용자 캐시 자동 이동/삭제, 앱 중단, 전역 pnpm 설치, 배포·전체 플랫폼 CI는 실행하지 않았다.

## 구현 후 검증

- 실제 기본 중앙 경로 `/Users/egoinglee/dev/project/agent-gibbon/.git/agent-gibbon/build-cache`로 A/B 개발 빌드 성공: A 1m31s, B14.96s(끝부분 A/B build-directory 잠금 포함).
- A instance `dev-f0f4ac7ca763829096124cec`, B instance `dev-f403a583b693ae5b1f09960a`. 각각 자신의 `src-tauri/target/debug/agent-gibbon`만 사용하고 `--instance-info`의 manifest_dir가 자신의 src-tauri인 것을 확인했다. [바이너리 출처](shared-build-cache-pnpm-20261114/integration-provenance.json).
- `pnpm install --frozen-lockfile` 두 워크트리 성공. `pnpm test`: Rust223개+Node204개 통과. `pnpm run check`: Clippy 모든 target -D warnings 통과. Rustfmt·버전 일치·diff 공백·Python 문법 검사 통과. workflow YAML 파싱 성공.
- `CARGO_BUILD_JOBS=4 pnpm run build`: macOS ARM64 release app 번들 성공(컴파일2m45s, 번들19.42MiB), `codesign --verify --deep --strict` 통과. Developer ID 서명·공증·설치·release 앱 실행·다른 플랫폼 빌드와 전체 CI는 미실행이다.
- `python3 scripts/isolation-smoke.py --peer /Users/egoinglee/.codex/worktrees/gibbon-cache-bench-peer/agent-gibbon --without-default`: 종료0, 대안 범위 통과. [실제 inspect와 결과](shared-build-cache-pnpm-20261114/integration-isolation.jsonl). note ID `isolation-c516ee1c`, A PID65351/B PID66093의 실행파일 vnode를 lsof로 각 binary에 연결하고 응답 instance·본문 `worktree A`/`worktree B`를 대조했다.
- CLI 자동 시작과 pnpm dev가 각각 자기 namespace를 사용하고, 동일 ID 본문·중복 실행 잠금·원본 파일이 독립적이었다. 숨김 갱신에서 위치/크기/포커스 유지, A 종료·재시작 중 B 유지, 네이티브 활성앱 유지도 통과했다.
- 기본 설치 앱은 처음부터 ConnectionRefused로 미실행이었고 끝까지 부재를 유지했다. `installed_app_state_verified:false`이며 기존 설치 앱 메모·창 상태 보존은 **미검증**이다. 원래 with-default isolation 전체 통과로 보고하지 않는다. 화면 수동 조작·스크린샷은 수행하지 않았다.
- 종료 후 lsof로 두 개발 binary에 남은 PID가 없음을 확인했다. 기존 사용자 앱을 종료하지 않고 시험이 직접 시작한 PID만 종료했으며 팀장에게 GUI 슬롯을 반환했다.
- [검증 대상 로컬 파일 해시](shared-build-cache-pnpm-20261114/integration-source.json)로 미커밋 구현을 식별한다.
- 최초 검증 당시에는 계획 커밋40b440c와 로컬 구현을 사용했다. 이후 사용자의 close 호출로 제품 커밋·병합이 승인되었으며 최종 SHA와 검사·동기화 결과는 PR171/이슈170에 기록한다.


## 공식 자료

- [Cargo build-dir 설정](https://doc.rust-lang.org/cargo/reference/config.html#buildbuild-dir)·[build cache](https://doc.rust-lang.org/cargo/reference/build-cache.html): 중간/최종 산출물 분리. 로컬1.91.1 설치 문서에서도 환경변수 지원을 확인했다.
- [pnpm import](https://pnpm.io/cli/import): 변환 결과가 일대일 보장이 아니므로 실제 버전·integrity를 대조했다.
- [pnpm 저장 방식](https://pnpm.io/motivation): hardlink/CoW clone 가능.
- [설치 호환성](https://pnpm.io/installation)·[CI setup](https://github.com/pnpm/action-setup): pnpm11의 Node22.13+ 요구에 맞춰 CI는 Node를 먼저 설정하고 pnpm 설치 후 pnpm cache를 복원한다. Intel macOS에 pnpm11 standalone 바이너리를 가정하지 않는다.


## 명령 재현

측정 루트와 두 Git 워크트리 루트는 환경 JSON에 기록했다. 각 Cargo 비교는 `CARGO_BUILD_BUILD_DIR=<비어 있는 independent-a|independent-b|shared> cargo build --offline --locked --jobs 4 --manifest-path <root>/src-tauri/Cargo.toml --target-dir <별도 target> --timings`를 사용한다. shared만 A/B가 같은 build-dir를 쓰며 최종 target은 다르다. 동시는 Python Popen으로 A/B를 연속 시작하고 모두 종료할 때까지 monotonic 시간을 잰다.

JS는 전용 폴더에 package.json/package-lock을 복사해 npm은 `npm ci --no-audit --no-fund --cache <새 전용 cache>`, pnpm은 별도 import store로 `pnpm import --config.store-dir=<import-store>` 후 `pnpm install --frozen-lockfile --store-dir <새 전용 store>`를 실행한다. warm은 해당 시험 폴더의 node_modules만 제거하고 `--offline`로 각 3회 반복한다. 기존 사용자 cache나 Cargo registry를 비우지 않는다.

검증 전용 peer 워크트리는 두앱 시험과 출처 기록 완료 후 호스트 archive_worktree로 보관했다. 주 작업 루트 ae2a의 구현과 중앙 캐시는 유지한다.
