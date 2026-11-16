# #98 외부 웹 콘텐츠의 IPC 거절

2026-11-16 21:40–21:42 KST, Gibbon 팀장이 macOS 개발 앱에서 독립 검증했다. 별도 HTTP origin의 페이지는 JavaScript 실행 표식을 보냈고 앱 명령과 이벤트 등록을 각각 시도했다. 다음은 같은 제품 프로세스(PID 3614)에서 페이지가 받은 응답이다.

| 외부 페이지 요청 | 네이티브 응답 |
|---|---|
| 앱 명령 `local_flow_status` | `local_flow_status not allowed. Plugin not found` |
| 합성 이벤트의 `event.listen` | `event.listen not allowed`로 시작하며 해당 창·자식 WebView·외부 URL과 허용된 로컬 capability를 명시했다. |

첫 행은 원문이고, 둘째 행은 고유 창 ID와 capability 목록을 생략한 요약이다. 실행 당시 앱 명령은 [제품 핸들러에 등록](../../src-tauri/src/main.rs)돼 있었다. 제품의 두 응답만으로 앱의 레이블 검사까지 요청이 도달했다고 판정하지 않았다.

## 실행 조건과 대조

1. 검증 소스는 `7affa482d4bd12c4eda8e712d2bc1e8bccbc88fd`, 워크트리는 `5ea9`, 빌드는 `pnpm dev:build`의 debug 실행 파일이다. `pnpm dev:info`와 바이너리 `--instance-info`의 루트·manifest·인스턴스를 대조하고 실제 PID의 실행 경로를 확인했다. 인스턴스는 `dev-2d5f10299d2132f0992c78b8`, 바이너리 SHA-256은 `45068af2aee1976d81795bd681a1c4892831ad8bac54fbf22eadb3665814e2ca`였다.
2. 임시 HOME을 사용하고 제품 밖의 임시 서버를 loopback에만 바인딩했다. 개발 CLI로 고유 ID의 웹 창을 열고 `inspect` 응답의 ID·인스턴스를 대조했다. 같은 origin의 수집기는 각 페이지의 고유 표식, JavaScript 실행 여부, 두 요청의 성공·거절 결과를 받았다. 내부 대조를 포함한 실행의 포트는 64945였다.
3. 같은 PID의 신뢰된 합성 메모를 갱신했다. [내부 추적 코드](../../ui/local-flow.js)는 상태 응답이 `true`일 때 추적을 시작한다. 로컬 추적에서 같은 `note.refresh` 호출 번호 355의 `request`, `response`, `complete`를 확인했다. 내부 상태 명령의 성공은 이 조건과 실행 기록에서 추론했으며 반환값 자체는 캡처하지 않았다.
4. 기존 담당자는 자체 안전 범위 판단으로 제품 외부 호출을 수행하지 않았고, 별도 Tauri 시험본에서 신뢰된 화면의 상수 응답과 이벤트 등록·해제를 직접 확인했다. 위 표의 출처는 이후 팀장이 실행한 실제 제품이다.
5. 두 실행이 끝난 뒤 검증 PID 종료와 포트 64816·64945의 리스너 부재를 확인했다. 전면 앱 PID는 실행 전·중·후에 같았다. 제품 소스는 그대로였고, 추적은 로컬 파일에만 기록했다. 사용자 데이터나 비밀정보의 외부 전송은 없었다.

### 원본 근거의 SHA-256

| 보존한 원본 | SHA-256 |
|---|---|
| 외부 호출 실행 `product-result.json` | `5d126f1d54b7e53211c52e3114ca9ec7aa3ea3006be9e5be2c07014b00d0303e` |
| 내부 대조 포함 `product-control-result.json` | `429414a2083bd9e2004335a3e7373712661271d59b6c8349418cd1f435455066` |
| 팀장 판정 `verification-summary.json` | `597b49f1df7390d25f58caefa7769288eeaf0955a380bc48582b20b6f0b02989` |

## 이슈 완료 기준과 남은 범위

[이슈 #98](https://github.com/egoing/agent-gibbon/issues/98)의 별도 origin에서 앱·플러그인 명령 거절을 관측하는 기준과 제품 서버 추가·외부 전송 금지 조건을 충족했다. HTTP 페이지와 합성 메모 갱신을 확인했다. 팝업·다운로드 차단은 별도 시험본에서 관측했다. 제품에서 내부 origin으로 이동·리다이렉트할 때의 차단, 팝업·다운로드, HTTPS 허용과 기존 메모·터미널의 회귀 검증이 남아 있어 이슈를 열어 둔다.

**Conf 5 / Risk High**는 이 macOS 개발 제품에서 관측한 외부 호출 두 건과 해당 권한 경계의 실패 영향에 적용한다. 다른 명령·origin, Windows와 공식 서명 설치본은 미검증이다.
