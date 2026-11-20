# Windows 트레이 메뉴 테마·상태 표시 — 2026-11-20

## 결과와 계획

- [x] 공식 0.3.8 설치 파일과 SHA256SUMS 일치 확인
- [x] 사용자 문서 폴더를 별도 로컬 경로에 백업하고 기존 설치 앱 업데이트
- [x] 재실행 앱 diagnostics의 current_version=0.3.8, 최신 버전 응답 확인
- [x] Windows 기능 아이콘·테마 착색·메모 표시/숨김 문자 구현
- [x] 0.3.9로 npm/Cargo/Tauri/lockfile 함께 버전업
- [x] Rust 314개 및 UI 247개 통과(1개 skip), Clippy 통과
- [x] 밝음·어두움 SVG/PNG 시안 렌더링 및 직접 이미지 확인
- [x] 최종 개발 빌드 및 CLI 출처 확인
- [ ] 커밋·병합·팀장 재배포 요청
- [ ] 공개 0.3.9 설치 후 사용자 환경 확인

## 변경

Windows의 메모 관리/영역 캡처/영역 텍스트 메모/영역 터미널/영역 웹 창/숨김/종료를 IconMenuItem으로 만든다. Tabler v3.35.0(MIT)의 layout-grid, crop, browser, eye-off, logout SVG 및 18×18 PNG를 추가하고 기존 file-text/terminal-2를 재사용한다. [원본 출처](https://github.com/tabler/tabler-icons/tree/v3.35.0/icons/outline)의 기존 Tabler LICENSE를 유지한다. 임시 npm exec @resvg/resvg-js로 렌더링했으며 제품 의존성은 추가하지 않았다.

Windows 앱 테마(AppsUseLightTheme)를 읽어 어두움 #f0f0f0 / 밝음 #202020으로 아이콘 RGB를 착색한다. 고대비 모드는 SystemParametersInfoW와 GetSysColor(COLOR_MENUTEXT)의 읽기 전용 조회를 우선한다. 투명 배경·안티앨리어싱을 보존하고 메뉴 생성·갱신 시 테마를 다시 읽는다. OS 설정을 변경하지 않으며 관리 화면의 웹 테마와 OS 메뉴 테마를 구분한다.

메모 제목 오른쪽 tab 열에는 실제 창 is_visible에 따른 `표시 중` / `숨김`을 표시한다. PTY 실행 여부와 독립적이며 숨긴 항목도 선택 가능하다. 기존 숨김 아이콘 45% 알파와 최근 순서·30개 제한·ID·복원 동작은 유지한다. macOS는 기존 template 아이콘과 제목 알파를 유지하며 기능 아이콘 추가 및 상태 문자는 Windows에 한정한다.

## 실제 검증과 한계

Windows, Node 24.22.3(npm exec node@24), Cargo 1.99.1.

| 실행 | 결과 |
|---|---|
| Rust 전체 테스트 | 314 통과, 0 실패 |
| UI 전체 테스트 | 247 통과, 1 skip, 0 실패 |
| Clippy --all-targets -- -D warnings | 통과 |
| 버전 일치 검사 | 0.3.9 |
| 최종 npm run dev:build | 성공, 16.89초 |
| Rustfmt --check / git diff --check | 통과 |
| 밝음·어두움 시안 렌더/이미지 열람 | 완료 |

새 테스트는 밝음·어두움·고대비 색의 RGBA 전경·원본 알파·숨김 알파·투명 픽셀과 macOS RGB 보존, Windows 상태 문자를 검증한다. 기능 7개 PNG를 실제 디코딩하여 크기·투명 배경·유효한 선과 알 수 없는 ID의 미매칭을 검증한다. 기존 최근 순서·표시 제목·메뉴 잠금 회귀도 전체 Rust 테스트에 포함된다.

[Windows 시안 SVG](../design/windows-tray-menu.svg) · [PNG](../design/windows-tray-menu.png)는 실행 캡처가 아니다. 실제 네이티브 메뉴 크기·여백·선택 강조는 OS를 따른다.

실제 메뉴 GUI 확인은 미완료다. computer-use와 CUA의 앱/창 목록에 Jajan 도구창·트레이가 노출되지 않았다. orca-computer-use는 runtime_unavailable, 스킬의 앱 시작 시도는 runtime_timeout을 반환해 중단했다. 실제 Windows 테마 전환·고대비·네이티브 메뉴 화면과 macOS 실기기는 미검증이다.

## 설치본 업데이트 근거

[공식 0.3.8 설치 파일](https://github.com/egoing/agent-gibbon/releases/download/v0.3.8/Jajan_0.3.8_x64-setup.exe)과 SHA256SUMS를 비교했다. 다운로드는 `C:\Users\egoin\.codex\artifacts\jajan-install-038`, 업데이트 전 문서 백업은 `C:\Users\egoin\.codex\artifacts\jajan-before-038`에 로컬 보존했다.

설치 전 PID 27236/0.3.7을 확인했다. 공식 NSIS를 /S /UPDATE로 실행한 뒤 파일 버전 0.3.8을 확인하고 재실행했다. PID 7660의 경로는 `C:\Users\egoin\AppData\Local\Jajan\jajan.exe`, diagnostics는 current_version 0.3.8 / phase idle / 최신 버전입니다 / target null을 반환했다. 사용자 메모 원문을 출력하지 않았고 별도로 강제 종료하지 않았다. 실행 중 앱 갱신은 설치 프로그램이 처리했다.

앱 재실행·아이콘 다운로드·렌더링을 묶은 최초 명령은 자동 승인 검토에서 차단되어 실행되지 않았다. 각각의 허용된 작업으로 분리하여 완료했으며 보안 설정을 변경하지 않았다. 긴 문서 작성 셸 명령도 거절되어 전용 파일 편집 도구로 기록했다.

## 작업 출처

- root: `C:\Users\egoin\.codex\worktrees\update-ack-analysis\agent-gibbon`
- branch: `codex/windows-tray-theme`
- base/start: origin/main의 0324bdc (0.3.8 공개 검증 병합)
- dev instance: `dev-064b569be88189ba4c163c22`
- binary: 위 root의 `src-tauri\target\debug\jajan.exe`

Conf 3, Risk Low. 설치·공개 릴리스 결과는 개별 확인한다. 공개 태그·전체 플랫폼 빌드·배포는 팀장에게 연결하며 현재 체크리스트에는 실제 완료한 항목만 표시한다.

개발 CLI로 tray-theme-visible / tray-theme-hidden 전용 메모를 생성하고 두 번째만 숨겼다. inspect는 모두 ok:true, 위 dev instance 일치, 각각 visible:true/false였다. diagnostics는 current_version 0.3.9, phase disabled를 반환했다. PID 12988의 실행 경로가 위 binary와 일치하고 기존 설치 앱 PID 7660/0.3.8과 구분됨을 확인했다. 검증 뒤 두 메모를 숨기고 실행 경로를 다시 확인해 이번 개발 PID만 종료한다. CLI 확인은 실제 메뉴 픽셀 검증을 대신하지 않는다.
