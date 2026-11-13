# #140 체크박스 외곽 통일 검증

## 출처

- root: `/Users/egoinglee/.codex/worktrees/1123/agent-gibbon`
- branch: `codex/140-checkbox-size`, base: `9904f10770d552b8f90ceb8073415d9875c09062`
- 계획 커밋: `ebc4efe`, Draft PR: https://github.com/egoing/agent-gibbon/pull/157
- dev instance: `dev-16c8a614f5050c083c4ae6e3`
- binary: `/Users/egoinglee/.codex/worktrees/1123/agent-gibbon/src-tauri/target/debug/agent-gibbon`
- 환경: macOS ARM64, Node v24.21.0 / npm 11.19.0 / rustc·cargo 1.91.1.
- 최초 검증은 위 계획 커밋 이후 로컬 제품 변경에서 수행했다. 2026-11-14 사용자 승인에 따라 커밋·통합한다.

## 실행

- `npm test`: Rust 217개·UI 188개 통과. 기존 완료 강조·diff·sanitizer·파일 저장 검사와 새 marker 상태·생성 태그/사용자 HTML 분리 검사를 포함한다.
- `python3 scripts/checkbox-layout-smoke.py`: Chromium 64조합(12/14/16/18px × 흰색/회색/검정/컬러 × 일반/추가/수정/삭제) 통과. 상태 전환 전후 외곽 폭·높이·행 높이·본문 x/y·기준선 좌표가 동일하고 `currentPlainText()`의 복사 결과에 ☑/☐가 한 개씩 유지됨을 확인했다.
- 외곽 크기(CSS px): 12px→10.1875, 14px→11.890625, 16px→13.59375, 18px→15.296875. 두 상태는 동일하다.
- 최초 두 Chromium 실행은 프로세스 exit 2로 실패했다. 동일 검사 재실행은 통과했으며 앞선 실패를 통과로 계산하지 않는다.
- `rsvg-convert docs/design/checklist-checkbox.svg -o docs/design/checklist-checkbox.png`: 렌더 후 4개 크기·4개 테마를 육안 확인했다.

실제 개발 앱 실행 검증과 Clippy·빌드 결과는 아래에 기록했다. Windows·Intel GUI, 전체 플랫폼 CI·공개·배포는 실행하지 않는다.

## 최종 로컬 검사

- `npm run check`·`cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`·`git diff --check`: 통과.
- Clippy 최초 실행은 추가 테스트의 single-range slice 표기를 지적했다. `std::slice::from_ref`로 고쳐 재실행 통과했다. 제품 동작에는 영향이 없다.
- `cargo test --manifest-path src-tauri/Cargo.toml`: 최종 테스트 수정 후에도 217개 통과.
- `npm run dev:build`: 이 워크트리의 debug 빌드 통과. 전체 플랫폼은 실행하지 않았다.

## 실제 macOS 개발 앱

- 개발 CLI `open`·`inspect`: `ok: true`, instance 일치, 고유 메모 ID `checkbox140-native-20261113`와 본문 `140 고유 검증 1123`를 확인했다. [실제 응답](issue140/inspect.json).
- `ps -p 35993 -o comm=`으로 위 binary 절대 경로를 확인했다. Orca `list-windows --app pid:35993`는 빈 배열이었으나 `get-app-state --app pid:35993`는 창 ID **68502**, 제목 `Agent Gibbon [DEV codex/140-checkbox-size · 16c8a614] — checkbox140-native-20261113`, 동일 PID와 고유 본문을 반환했다. 이후 조작은 이 PID와 창 ID만 사용했다. [출처 근거](issue140/provenance.json).
- 실제 검정 테마에서 글자 크기 버튼을 12→14→16→18→12px로 순환하고 AX 이름·화면을 확인했다. 각 크기에서 체크·미체크 외곽은 같은 크기로 보였다. 14px 이상에서는 고정된 창 높이 때문에 뒤 행이 잘리므로 보이는 앞의 두 상태를 검증했다. 창 크기·위치 변경이나 테마 설정 변경은 하지 않았다.
- 실제 `update --change-mode replace --highlight newly-completed`에서 전환 행만 호박색 강조되고, `--change-mode diff`에서 삭제된 이전 상태와 새 상태의 체크박스 모두 각 강조색의 외곽과 체크를 유지했다. [완료 강조 응답](issue140/completion-inspect.json) · [diff 응답](issue140/diff-inspect.json).
- `본문 복사` 버튼을 native 배경 입력으로 눌러 NSPasteboard에서 **최신 본문**과 ☑ 두 개·☐ 두 개를 확인했다. 화면에 남은 삭제 diff는 복사되지 않았다. 기존 클립보드의 모든 데이터 타입을 메모리에만 보존하고 테스트 문자열·변경 카운트 일치 확인 뒤 복원했다. 클립보드 내용은 저장하지 않았다. 물리적 붙여넣기·스크린리더 낭독은 미검증이다.
- 실제 앱의 inspect HTML을 입력으로 `python3 scripts/checkbox-layout-smoke.py /tmp/gibbon140-evidence/inspect.json`도 64조합 통과했다. [측정 결과](issue140/layout.json). 테마별 실제 native 전환은 하지 않았으며 Chromium의 실제 제품 CSS 검사와 구분한다.
- 테스트 메모를 `close`한 뒤 [inspect의 visible=false](issue140/closed-inspect.json)를 확인했고, 정확한 경로를 재확인한 PID 35993만 종료했다. 다른 앱/메모는 변경·종료하지 않았다. GUI 슬롯 해제를 팀리더에 보고했다.

[실제 12px](issue140/native-12.png) · [14px](issue140/native-14.png) · [16px](issue140/native-16.png) · [18px](issue140/native-18.png) · [완료 강조](issue140/native-completion.png) · [diff](issue140/native-diff.png)

## 남은 한계와 승인

- Windows·Intel GUI·스크린리더 낭독·물리적 붙여넣기·테마별 native 전환·전체 플랫폼 CI·설치·공개 배포는 미실행이다. 파일 내보내기 다이얼로그를 열지는 않았으며, 원본 Markdown 보존·파일 저장 테스트를 통과한 범위와 구분한다.
- Conf **5**: macOS ARM64 실제 앱의 대상 표시·복사 확인. 모든 플랫폼/보조기술 검증을 뜻하지 않는다.
- Risk **Low**: 읽기 전용 기호의 국소적 표시 변경이며 원문·저장 단축키·입력 기능은 변경하지 않는다.
- 2026-11-14 사용자가 최종 제품 변경의 커밋·push·병합을 명시적으로 승인했다.
