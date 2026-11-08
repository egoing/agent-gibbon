# #125 서명 업데이트 장애 복구 실기기 기록

실행기 경로 정규화 변경(`82873f4`) 뒤 Intel macOS 검증 인스턴스와 loopback 피드(`127.0.0.1:49382`)로 확인했다. 원래 `verification/manual` 증거와 키 파일은 보존했다. 회차별 원본 HTTP 로그·Store·앱 번들은 검증 실행기의 임시 증거 경로에 있고, 재현 절차는 로컬 재현 절차는 `/tmp/issue125-gui-free-recovery.md`에 기록했다.

| 시나리오 | 관찰 | 판정 |
| --- | --- | --- |
| 서명 변조 (r4) | manifest와 archive GET은 200. 서명 확인은 `last_check_failure=download`로 실패, 버전은 0.2.11 유지, 설치되지 않음. | 보호 확인 |
| 오프라인 (r5) | manifest GET 503, `last_check_failure=manifest`, 버전 0.2.11 유지. | 기존 버전 보호 확인 |
| 정상 후보 복구 (r6, fast profile) | 60초 확인 주기. 오프라인 503 이후 피드 복구 63초 뒤 manifest/archive 200. 서명된 0.2.12 설치 후 앱이 한 번 재시작했고 버전이 0.2.12가 됨. 업데이트 캐시와 `.last-update-attempt.json`의 `prepared` 기록이 남았고, 다음 실행에서도 이를 읽어 정상 기동했다. | 자동 복구 확인 |
| 다운로드 도중 연결 종료 (r8) | manifest 200, archive는 7,208,594바이트 중 2,402,864바이트에서 연결 종료. `last_check_failure=download`, `phase=error`, 버전 0.2.11 유지. | 불완전 다운로드 설치 방지 확인 |

r6에서 격리 Store의 읽기 메모는 업데이트 전후 제목과 revision 1, Markdown이 같았다. 앱 프로세스는 82702에서 85554로 바뀌었고, 단일 `restart` 이벤트와 실행 번들의 `CFBundleShortVersionString=0.2.12`를 확인했다. 설치 상태 응답은 `sharing_allowed=false`였다. 빌드에 Sentry DSN을 주입하지 않았으며, 외부 진단 트래픽은 패킷 캡처로 확인하지 않았다. 동시 자동·수동 설치 요청 예약은 Rust 단위 테스트 `simultaneous_automatic_and_manual_requests_reserve_only_one_install`에서 통과했다.

화면이 잠겨 모든 테스트를 `--headless`로 실행했다. CLI `inspect`는 격리 인스턴스와 Store, 메모 내용을 확인했지만 `visible=false`, `focused=false`를 반환했다. 따라서 사용자가 실제 메모를 읽는 화면 상태에서 30초 무입력 자동 설치가 진행됐다고 판정하지 않는다. `idle_seconds`가 30초를 넘은 것은 관찰했으나 visible-read 증거와 구분한다.

## 로컬·원격 검증

- `node --check scripts/issue102-update-runner.mjs`
- `node --test tests/issue102-update-runner.test.mjs`: 7개 통과
- verification-updater Rust 테스트: 210개 통과
- 자동·수동 동시 설치 예약 Rust 테스트: 통과
- verification-updater Clippy (`-D warnings`) 및 `cargo fmt --check`: 통과
- GitHub Release workflow run [42457208947](https://github.com/egoing/agent-gibbon/actions/runs/42457208947), SHA `0a6ac2582f21e19dacdbdbc7a935ebe83f89b1df`: validate, Intel/ARM macOS, Windows, assemble 모두 통과; Draft job은 skip. 중복 workflow run `42457223968`은 같은 SHA의 pending dispatch임을 확인한 뒤 취소했다.

서명 검증 및 앱 실행은 로컬 전용 시험이었다. 태그, 공개 릴리스, 공개 배포는 수행하지 않았다.
