# 개발자 메뉴 버전 왕복·터미널 분리 연결 — 2026-11-21

## 현재 완료 기준

- [x] 기존 준비/설치 기능 보존과 두 왕복 버튼 구현
- [x] 공식 호환 쌍 부재 시 임의 과거 버전 설치 차단
- [x] 지정 격리 쌍·정상 서명·캐시·재시작 수동 대기 구현
- [x] 전용 NSIS 설치본에서 실제 0.3.13 → 0.3.12 교체·재시작 확인
- [x] 이전 버전에서 무입력 자동 복귀 차단과 0.3.13 복귀 확인
- [x] 터미널 분리 API 계약 연결과 미포함 API 실패 표시 구현
- [ ] PTY PR 병합 후 실제 분리 ON·신규 세션·동일 셸 업데이트 재접속 통합 검증
- [ ] 최종 테스트·개발 빌드·PR 순차 병합·main 동기화

최초 최신 배포판 준비/동일 버전 검증은 [선행 기록](developer-updater-20261121.md)에 분리한다. 이 문서는 사용자의 후속 왕복·분리 연동 요구와 실제 수행 범위를 기록한다.

## 격리와 설계

- 작업 루트 `C:\Users\egoin\.codex\worktrees\5170\agent-gibbon`, 브랜치 `codex/developer-update-test`, 최초 기준 SHA `974197adbc34f2f72872eb4e6262e270db90ef2d`다. 현재 저장소 버전을 올리거나 공식 릴리스를 변경하지 않았다.
- 공식 배포판에 개발자 메뉴와 복귀 기능이 포함된 호환 버전 쌍이 아직 없다. 기본 빌드에서는 두 왕복 버튼을 비활성으로 두고 이유를 표시한다. 아무 과거 버전으로 내려가는 기능은 제공하지 않는다.
- `verification-roundtrip` feature에만 같은 소스·데이터 형식·메뉴의 `0.3.12 ↔ 0.3.13` 쌍을 정의했다. 두 설치 패키지에는 실제 해당 버전 메타데이터를 넣었고 현재 버전 비교 값을 위조하지 않았다. 앱·Cargo·Tauri·lockfile 버전 변경은 임시 복사본에서만 수행했다.
- `scripts/prepare-roundtrip-verification.mjs`는 프로젝트 소스와 필요한 로컬 스킬을 새로운 임시 폴더에 복사하고 두 NSIS 패키지와 loopback 검증 피드를 만든다. 비밀 키는 메모리에만 생성·보관하고 파일·출력에 저장하지 않는다. 패키지와 trusted comment의 버전·플랫폼 파일명에 정상 Ed25519 prehash 서명을 적용한다. 정식 배포 키로 서명한 공식 자산으로 표현하지 않는다.
- 식별자 `app.jajan.roundtrip5170`, 제품명 `Jajan Roundtrip Verification 5170`, 실행 파일 `jajan-roundtrip-5170.exe`를 사용한다. 기본 Jajan의 바이너리·제거 레지스트리·설치 경로와 분리했다. NSIS 기본 downgrade 허용 동작 안에서 앱이 명시적인 호환 쌍만 승인한다. 서명·창 ACK·native 작업 보호를 우회하지 않는다.
- 실제 패키지의 NSIS 스크립트에서 별도 제품명·실행 파일명·제거 레지스트리 키를 확인했다. 설치 대상이 전용 임시 작업공간 내부인지 확인하고 패키지 SHA-256을 대조한 뒤 `/S /NS /D=<전용 경로>`로 최초 설치했다. 업데이트 버튼은 기존 plugin의 NSIS 설치·재시작 절차를 사용한다. 사용자 설치 앱은 종료하지 않았다.
- 이 고정 쌍 준비 스크립트는 소스 기준 버전 0.3.12만 허용한다. 이후 공개 버전이 바뀌면 새로운 호환 쌍을 검토해 정의해야 하며 임의로 과거 버전 메타데이터를 맞추지 않는다.
- 재시작해도 캐시가 없어지거나 새 버전 피드가 보인다는 이유로 자동 복귀하지 않도록 `.developer-update-trial.json`으로 별도 시험 대기를 저장한다. 서명된 최신 쌍 패키지를 현재 실행 버전에서 확인한 후에만 일반 자동 업데이트 정책을 복원한다.
- 5회 클릭 진입 시 UI가 `terminal_persistence_enable`, 조회 시 `terminal_persistence_status`를 호출한다. `supported/enabled/applies_to=new_sessions`를 확인할 때만 켜짐을 표시한다. `backend/persisted/reason`을 함께 표시하고 기존 셸을 재실행하지 않는다. 상태를 따로 만들어 낙관적으로 켜졌다고 표시하지 않는다.
- Windows backend와 관리 창 전용 API는 PTY 담당 브랜치가 소유한다. 이 메뉴 브랜치에는 아직 API가 없으며, API 미포함·실패는 분리 미지원/통합 필요로 표시한다. Mac release의 debug-only 경로 활성화를 주장하지 않는다. 실제 ON과 재접속은 PTY PR 통합 이후 별도 검증한다.
- 신규 생성 opt-in은 현재 GUI 실행에만 적용한다. 재시작 후 `enabled=false`와 기존 셸의 `reconnect_persisted=true`가 함께 반환되어도 새 세션을 ON으로 표시하지 않는 UI 회귀 검사를 통과했다. 기존 셸 재접속 권한과 신규 생성 활성화를 구분하며 backend 상태를 별도로 복제하지 않는다.
- 다음 통합 패키지 준비부터 임시 작업공간별 고유 `JAJAN_VERIFICATION_INSTANCE`를 두 버전에 동일하게 주입하고 계획 파일에 기록한다. 해당 compile-time override가 포함되지 않은 소스는 준비 전에 중단한다. 이 추가 경로의 실제 빌드·namespace 응답 확인은 PTY 통합 뒤 수행하며, 이전 독립 시험은 당시 고정 검증 ID의 증거로 보존한다.

## 실제 독립 왕복 시험

- 준비 디렉터리: `C:\Users\egoin\AppData\Local\Temp\jajan-roundtrip-5170-Ms4WyJ`.
- 설치 실행 파일: `...\installed\jajan-roundtrip-5170.exe`. [설치 계획과 패키지 해시](developer-roundtrip-20261121/installation-plan.json)를 남겼다. 로컬 feed는 `127.0.0.1`에만 바인딩했다.
- 실제 최초 설치 파일 정보·NSIS 등록 버전은 `0.3.13`이었다. 최초 실행 PID 29716, 파일 SHA-256 `3C4D6189F5F777C32729CBB9A87004E8CDBBC5E32A907BACA01E92B7A6CF3153`이다. 선택 진단 전송은 false다.
- [소스 해시 대조](developer-roundtrip-20261121/source-provenance.json)는 독립 설치 시험 시점의 소스를 기록한다. 그 뒤 `updater.rs`에 별도 시험 대기가 기존 자동 캐시·이미 진행 중인 자동 다운로드보다 우선하는 두 경계 검사를 추가했고, `developer-updater.js`에는 PTY API와 실행 한정 수명 표시를 추가했다. 따라서 독립 설치 쌍을 최종 통합 코드의 실행 증거로 재사용하지 않는다. JSON의 `matches_current`로 현재 코드 커밋 668432e와의 차이를 구분했다. 시험 서명 검증용 [공개 키](developer-roundtrip-20261121/verification-public.key)를 보존했고 개인 키는 저장하지 않았다.
- 지원 Computer Use의 반환 창에서 전용 실행 경로와 네이티브 관리 창 제목으로 선택했다. 실제 설치 앱과 혼용하지 않았다. 5회 클릭 후 ‘이전 테스트 버전으로 교체’를 실행했다.
- [이전 설치 전](developer-roundtrip-20261121/before-down.json)과 [교체 후](developer-roundtrip-20261121/after-down.json): 실제 실행·파일 버전이 `0.3.13 → 0.3.12`, PID가 `29716 → 21488`로 바뀌었다. 준비 기록은 source `0.3.13`, target `0.3.12`, **8개 창 ACK 모두 true**, pending 0이다. 시험 메모 `roundtrip-proof-5170`의 sentinel 일치·revision 1·크기·가시성이 유지됐다. 최초 좌표는 생성 직후 자동 배치 안정 여부를 확인하지 않은 snapshot으로 `(1346,927)`이며 하향 후 `(3390,1582)`와 다르다. 첫 하향 구간의 좌표 보존은 미검증으로 정정한다. 통합 시험에서 `animating=false`·`layout_pending=false`와 안정된 좌표를 확인하고 다시 비교한다.
- 이전 버전도 5회 클릭으로 진입할 수 있고 최신 복귀 버튼이 활성이다. [이전 버전 화면](developer-roundtrip-20261121/previous-ready.jpg). `manual_test=true`, `roundtrip_active=true`, 공식 최신 피드 대신 지정 검증 최신 피드에 `0.3.13`이 있는 상태다. 기존 API 미포함 상태의 순수 업데이트 시험이며 터미널 분리 ON 시험과 혼동하지 않는다.
- [이전 버전 무입력 실측](developer-roundtrip-20261121/previous-idle-held.json): Windows 무입력 **325.406초**, 임계값 300초와 15초 retry 주기를 넘긴 뒤에도 같은 PID 21488/버전 0.3.12와 수동 시험 대기를 유지했다. 실제 최신 피드는 0.3.13을 제공하고 있었으며 입력 시간을 조작하지 않았다.
- ‘최신 버전으로 복귀’를 명시적으로 실행해 실제 0.3.13/PID 18688로 재시작했다. [복귀 결과](developer-roundtrip-20261121/after-return.json)의 source 0.3.12 → target 0.3.13과 모든 창 ACK true, sentinel/revision/창 위치·크기·가시성 보존을 확인했다. 최종 파일 SHA-256이 처음 설치한 최신 바이너리와 같은 `3C4D6189F5F777C32729CBB9A87004E8CDBBC5E32A907BACA01E92B7A6CF3153`이다.
- 최신 복귀 후 `manual_test=false`, `roundtrip_active=false`, `phase=idle`, 별도 시험 대기 파일 부재를 확인했다. 메시지는 ‘최신 버전 복귀를 확인했습니다. 일반 자동 업데이트 정책을 다시 적용합니다.’다. 최신 실행 확인 전에 시험 대기를 해제하지 않았다.

## 검사와 남은 작업

- 메뉴 단독 변경의 Rust 전체 317개, UI 전체 256개 통과(기존 2 건너뜀), Clippy all-targets 통과. API 미포함 시 낙관적 ON을 표시하지 않는 UI 테스트와 쌍 밖의 버전 거부 검사를 포함한다.
- 마지막 `npm run dev:build`는 해당 워크트리의 기존 debug PID 10636 파일 잠금(`os error 5`)으로 실패했다. 이 프로세스의 출처를 확인하지 못해 종료하지 않았고 팀장에게 확인을 요청했다. 이 실패를 최종 개발 빌드 통과로 기록하지 않는다.
- 이후 PID 10636이 더 이상 조회되지 않아 직접 종료하지 않고 개발 빌드를 다시 실행했다. 최종 빌드 결과는 통합 검증 뒤 갱신한다.
- 통합 전 메뉴 단독 빌드는 이후 통과했다. `npm run dev:info`의 root/id/binary를 확인했으며 [현재 개발 앱 증거](developer-roundtrip-20261121/current-dev-probe.json)에서 실제 instance·검증 sentinel·실행 파일 해시를 대조했다. 통합 후에는 다시 빌드·검증한다.
- [이슈 #275](https://github.com/egoing/agent-gibbon/issues/275)에 현재 완료 기준을 기록했다. PTY PR 통합 후 검증과 순차 병합·main 동기화가 끝나기 전에는 이슈를 닫지 않는다.

## 통합 의존성·범위

사용자가 후속으로 두 작업의 커밋·push·순차 병합·main 동기화를 승인했다. PTY PR을 먼저 검토·병합한 후 메뉴 브랜치에 최신 main을 통합해 실제 분리·업데이트 왕복을 검증한다. 릴리스 버전업·태그·공개는 별도 지정된 배포 담당자에게 넘긴다. 이 승인은 사용자 설치본의 임의 교체를 허용하지 않는다.

현재 기록은 진행 중이며 남은 검증이나 병합을 완료로 보고하지 않는다.
