# 설정 사이드바와 메뉴 등록 — 2026-11-06

이번 구현 범위는 설정 사이드바, 기존 프라이버시 기능의 메뉴 분리, 이슈 #67의 스킬 메뉴 진입점이다. 스킬 화면은 준비 중 상태이며 스킬 목록·On/Off·자동 설치·업데이트·영속 저장은 구현하지 않았다. 이슈 #67 전체 완료를 뜻하지 않는다.

## 수행 결과

- [x] 사이드바에 Privacy & Diagnostics / 스킬 등록, 선택 강조와 접근성 현재 페이지 상태 제공.
- [x] 프라이버시 UI를 기존 ID·저장 이벤트 그대로 별도 패널에 배치. 메뉴 전환 시 DOM·미저장 선택 유지.
- [x] 기본 버튼의 클릭·Enter·Space·Tab, 모든 메모 선택 및 설정 항목 유지 검사.
- [x] UI 테스트 82개 통과(`npm run test:ui`). 프라이버시의 동의 경계·저장·즉시 철회 회귀 검사 포함.
- [x] 실제 UI 자산의 headless Chrome 검사: 944×480 / 600×360에서 가로 넘침 없음, 하단 Save Changes 스크롤 접근, 메뉴 전환, 미저장 선택 유지, 저장 및 끄기 즉시 호출 확인. Tauri 호출은 메모리 mock으로 대체했다. JavaScript 오류 없음.
- [x] `npm run dev:build` 통과(최종 증분 빌드 포함), `node --check ui/manager.js`, `git diff --check` 통과.
- [x] 기존 설정 시안 SVG·PNG 갱신, rsvg-convert 렌더와 실제 브라우저 캡처 시각 확인.
- [ ] 네이티브 Manager UI·실제 저장 파일·실행 중 서버 inspect 검증 미수행. 개발 앱 미실행으로 list가 `ok:false / 앱이 실행 중이지 않습니다`를 반환했다. 해당 결과를 실행 중 서버의 출처 확인으로 간주하지 않는다.
- [ ] Windows·Intel·실제 스크린리더 읽기 미검증.

## 실행 출처

- 작업 루트: `/Users/egoinglee/orca/workspaces/agent-gibbon/floatnote/on-off`.
- `npm run dev:info`: root는 위 경로, instance `dev-1206da86e9acf14ddf9c123c`, binary는 해당 루트 `src-tauri/target/debug/agent-gibbon`.
- `scripts/agent-gibbon-dev --instance-info`: `ok:true`, instance 일치, manifest_dir는 해당 루트의 `src-tauri`.
- `scripts/agent-gibbon-dev list`: `ok:false`, 앱 미실행. 검증 메모·네이티브 PID·창 식별 근거는 없으며 브라우저 결과를 네이티브 검증으로 보고하지 않는다. 다른 워크트리나 설치 앱을 대체 실행하지 않았다.
- 환경: macOS ARM64, Node v24.21.0, Rust 1.91.1. 브라우저 자동화는 설치된 Chrome 및 기존 puppeteer-core 25.16.0 사용, 의존성 추가 없음.

재현 스크립트: `scripts/settings-sidebar-smoke.mjs`. 기존 puppeteer-core를 `PUPPETEER_MODULE`에 절대 모듈 경로로 지정하고 `node scripts/settings-sidebar-smoke.mjs`로 실행한다. 기본 Chrome 경로와 다른 환경에서는 `CHROME_BIN`도 지정한다. 임시 캡처는 `/tmp/gibbon-sidebar/`에 생성한다. 별도 서버나 외부 API를 호출하지 않는다.

## 화면 근거

다음은 실제 프런트엔드 자산을 Chrome에서 열어 촬영한 캡처이며 Tauri 앱 캡처가 아니다. mock 동의 상태로 동작하며 사용자 약관·진단 전송 동의는 변경하지 않았다.

- [944×480 프라이버시](settings-sidebar-20261106/privacy-944.png)
- [600×360 프라이버시](settings-sidebar-20261106/privacy-600.png)
- [스킬 진입점](settings-sidebar-20261106/skills-944.png)

디자인 기준은 [디자인 시스템](../design-system.md#설정-사이드바--67-진입점-2026-11-06), 설명용 시안은 [SVG](../design/manager-settings-wireframe.svg)·[PNG](../design/manager-settings-wireframe.png)를 따른다.

기존 `package-lock.json` 미커밋 변경은 작업 시작 전부터 존재했으며 보존했다. 커밋·병합·배포는 수행하지 않았다.

Conf 4: mock 호출을 사용하는 실제 자산의 브라우저 검증. Risk Low: 국소적인 메뉴·레이아웃 변경.

## 후속 실행·정리

사용자 요청으로 `npm run dev -- --no-watch`를 실행했다. PID 70065의 실행 파일은 현재 루트 `src-tauri/target/debug/agent-gibbon`과 일치한다. 최초 CLI 응답은 `onboarding_required:true`였으며, 이후 사용자 조작 뒤 만든 진행 메모 `issue67-work-20261106`의 `inspect`에서 `ok:true`, instance `dev-1206da86e9acf14ddf9c123c`, ID와 체크리스트 본문을 확인했다. 이 확인은 CLI 출처이며 Manager 화면의 native 검증을 대신하지 않는다.

사이드바 작업은 #69로 분리했고 사용자가 커밋·main 병합·푸시를 승인했다. 스킬 관리 핵심 기능은 #67로 이어진다.
