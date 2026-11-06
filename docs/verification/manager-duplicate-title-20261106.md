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
