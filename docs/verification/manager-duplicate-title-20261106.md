# 관리 창 텍스트 중복 제목 제거 — 2026-11-06

## 변경과 기준

[디자인 시스템](../design-system.md)의 텍스트 상세 기준에 따라, 텍스트 메모는 별도 상세 제목 없이 기존 Markdown HTML을 그대로 표시한다. 목록 제목 추출과 이미지·터미널·에이전트의 상세 제목은 유지한다. 원문과 저장 형식은 변경하지 않는다.

## 작업 출처

- 작업 루트: `/Users/egoinglee/orca/workspaces/agent-gibbon/floatnote/fix-manager-duplicate-title`
- 브랜치: `egoing/fix-manager-duplicate-title`
- 기준 커밋: `977037efde616040c8b585d0be49ca2ee98da1cc`
- `npm run dev:info`: root는 위 작업 루트, instance는 `dev-b43d989e93ab84560b176ec1`, binary는 위 작업 루트의 `src-tauri/target/debug/agent-gibbon`.
- 환경: macOS ARM64, Node v24.21.0, npm 11.19.0, rustc/cargo 1.91.1.

## 수행한 검증

- `npm run test:ui`: 81개 통과. 기존 관리 화면 검사는 제목이 있는 본문을 선택하고 설정에서 복귀해도 본문 하나만 표시하는지 확인하도록 변경했다.
- `node --check ui/manager.js`, `git diff --check`: 통과.
- `npm run dev:build`: 해당 워크트리의 개발 바이너리 빌드 성공.
- Chromium 156.0.8078.4에서 실제 `manager.html`, `manager.css`, `manager.js`와 모의 로컬 명령 응답으로 944×480, 600×360을 확인했다. 약관 화면과 진단 스크립트는 브라우저 검사 대상에서 제외했다. 실제 앱의 동의 파일은 변경하지 않았다.
- 각 크기에서 제목 있는 텍스트, 제목 없는 텍스트, 빈 텍스트, 이미지, 터미널, 에이전트의 6개 유형(총 12개 조합)을 확인했다. 텍스트 제목 중복 없음, 원본 본문 유지, 나머지 유형의 제목·이미지·동작 버튼 유지, 상세 패널 가로 넘침 없음. JavaScript 오류와 외부 HTTP 요청 0개.
- [944×480 브라우저 화면](manager-duplicate-title-944.png), [600×360 브라우저 화면](manager-duplicate-title-600.png). 모의 데이터 기반 캡처이며 네이티브 앱 화면이 아니다.
- [텍스트 시안 SVG](../design/manager-notes-wireframe.svg)를 rsvg-convert로 [PNG](../design/manager-notes-wireframe.png)로 렌더링하고 중복 제목 제거를 시각 확인했다. 기준 브랜치의 기존 앱 이름 배치는 유지했다.

## 실제 CLI와 한계

1. 개발 CLI `list`는 실행 전 앱 미실행 오류를 반환했다.
2. `./scripts/agent-gibbon-dev open --id duplicate-title-work-20261106 --title '관리 화면 중복 제목 제거' --file -`로 작업 체크리스트 생성을 시도했다. 새 개발 인스턴스가 시작됐지만 `ok: false`, `onboarding_required: true`, `instance: dev-b43d989e93ab84560b176ec1`로 차단했다.
3. 같은 CLI의 `inspect --id duplicate-title-work-20261106`와 `list`도 같은 인스턴스와 동의 필요 오류를 반환했다. 따라서 메모 생성·inspect 성공·본문 대조는 통과로 간주하지 않는다. 진행 메모는 생성하지 못했다.
4. 실제 서버 PID 39241을 `ps`와 `lsof -a -p 39241 -d txt -Fn`으로 확인했다. 실행 파일은 이 워크트리의 위 바이너리 경로와 일치했다. 검증 후 해당 PID와 명령이 일치함을 다시 확인하고 이 프로세스만 종료했다.

약관 동의를 대리하거나 다른 인스턴스로 대체하지 않았다. 관리 창의 네이티브 GUI·실제 저장 메모 표시·Windows 실행은 미검증이다. CLI 격리와 앱 창 출처를 완전히 검증했다고 주장하지 않는다. 브라우저 검증은 실제 IPC·네이티브 상태 보존 검증을 대체하지 않는다.

Conf 4 (모의 응답을 사용한 실제 UI의 브라우저 검증), Risk Low (관리 창의 텍스트 표시만 변경).


## main 병합·재시작·실환경 확인 — 후속 검증

- 사용자 승인으로 구현 커밋 `279c0fe030eb06f779ab138c5e43844dcc6301e0`를 `main`에 fast-forward 병합하고 `origin/main`으로 push했다.
- 작업 루트: `/Users/egoinglee/dev/project/agent-gibbon`. `npm run dev:info`의 root 일치, instance `dev-a5210ae16e79437d38490c8f`, binary `/Users/egoinglee/dev/project/agent-gibbon/src-tauri/target/debug/agent-gibbon`.
- main에 있던 미커밋 변경 18개를 임시 백업했다. 이 작업과 겹치는 디자인 문서·텍스트 시안 SVG/PNG 3개에는 기존 중앙 정렬 변경과 새 중복 제거 기준을 함께 보존했다. 나머지 15개 파일은 SHA-256 일치를 확인했고, 기존 변경은 커밋에 포함하지 않았다.
- `npm run test:ui`: 81개 통과. `npm run dev:build`: 성공. `git diff --check`: 통과.
- 재시작 전 PID 59219의 실행 명령을 정확한 위 바이너리 경로와 대조하고 해당 프로세스만 종료했다. 같은 루트의 `scripts/agent-gibbon-dev open --id main`으로 재시작했다. 새 PID 70637의 실행 파일을 `ps`와 `lsof -a -p 70637 -d txt -Fn`으로 확인했다.
- 같은 개발 CLI의 `inspect --id compact-feedback-20261028`에서 `ok: true`, 예상 instance, 메모 ID, 본문 첫 줄 `# 피드백바·툴팁 보정`, 숨김 상태를 확인했다. 사용자 메모의 본문은 변경하지 않았다.
- `orca computer list-windows --app pid:70637 --json`으로 PID에 속하는 관리 창 ID 38866과 네이티브 제목을 확인했다. 새 관리 창은 기존 위치 `{422, -730}`와 크기 `944×481`로 복원했다.
- `orca computer get-app-state --app pid:70637 --window-id 38866 --no-screenshot --query '피드백바·툴팁 보정' --json`의 목록 버튼을 선택했다. 반환된 접근성 트리에서 상세 패널의 같은 제목 heading이 하나뿐임을 확인하고 [실제 관리 창 캡처](manager-duplicate-title-main-native.png)에서도 확인했다. 캡처 원본은 `/var/folders/ds/zm2jvd217ss_q5jy2bjcbb9c0000gn/T/orca-computer-use/ce18535b-bcf5-4559-8865-e76739cb62e4-screenshot.png`이며 창 ID 38866, scale 2다.
- 재시작 전후 전체 메모·터미널의 표시/숨김 상태가 일치했다. `main` 메모의 본문·물리 위치 `{3496, 698}`·크기 `600×96`도 일치했다. 앱 재시작으로 이전 터미널 프로세스는 종료됐으며 창 복원은 PTY 세션 보존을 뜻하지 않는다.
- 현재 main 작업 폴더의 기존 미커밋 CSS 중앙 정렬도 포함해 재빌드했다. 원격에는 이번 중복 제목 변경과 검증 기록만 반영하며, 다른 미커밋 변경을 배포했다고 간주하지 않는다.
- Conf 5 (macOS main 개발 앱의 실제 저장 메모·CLI·네이티브 화면 확인), Risk Low. Windows 실행은 미검증이다. 위의 최초 새 워크스페이스에서 동의로 차단된 결과와 이번 기존 main 인스턴스의 성공 결과는 서로 다른 실행 근거다.
