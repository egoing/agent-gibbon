# 툴바 연결 편집 UI — #168 · 2026-11-15

## 구현 범위

[디자인 시스템](../design-system.md)의 연결 버튼 → 포인터 추적 → 대상 롤오버 → 클릭 확정 → 선택 레이블/화살표 설정 → 저장 흐름을 구현했다. [시안 SVG](../design/relation-editor.svg)·[PNG](../design/relation-editor.png)는 설명용이다.

- `relation-toolbar.js`를 세 공통 셸(텍스트/이미지, 터미널, 웹)에 공통 툴바보다 먼저 로드한다.12px/24×22 슬롯의 기존 CSS·페이지 이동·툴팁·hover 경로를 사용한다. Tabler의 MIT SVG 경로를 사용하고 `ui/licenses/tabler-icons.txt`에 라이선스를 포함했다.
- 명시 활성화된 개발 인스턴스에서만 버튼을 노출한다. trusted caller의 label과 실제 등록 host를 대조한다. 외부 `webpage-*`의 앱 IPC 차단은 그대로다. 입력 표면에 추가한 capability는 로컬 preview 이벤트 listen/unlisten뿐이다.
- 화면별 비포커스 입력 창은 연결 모드에만 존재한다. 곡선은 포인터 이동을 RAF 단위로 합쳐 전송하고 실제 host geometry를 조회한다. 롤오버 대상이 유일할 때 경계로 붙인다. 자기 자신·겹쳐 모호한 대상은 거부한다. 클릭 좌표를 다시 계산해 오래된 hover를 확정하지 않는다.
- 설정창은 레이블 생략·대상 방향 한쪽/양쪽/없음과 취소를 제공한다. 표시명은 사용자 제목이나 웹 host명이며 내부 ID를 화면 설명에 사용하지 않는다. 저장 중 중복 제출·취소를 막고 실패하면 입력을 유지하며 재시도할 수 있다.
- generation을 picker 고유 label·설정 저장/취소·출발 메모 Esc에 연결했다. 기존 세대의 응답으로 새 연결을 확정하거나 취소할 수 없다. 숨김·삭제·stash 시작은 임시 모드를 정리한다. 관계선 표시용 세대 관리도 유지한다.
- 동일 무방향 메모 쌍의 재연결은 그 항목만 새 from/to/label/arrow로 갱신한다. 다른 관계는 보존한다. 빈 레이블은 허용하고80자·제어문자·미등록 ID·최대16개/16KiB 제한을 유지한다.
- 사용자 저장 시 로컬 Store의 `.relations.json`과 기존 임시 live 파일을 각각 원자 교체한다. 두 파일 전체를 하나의 파일시스템 트랜잭션으로 묶지는 않는다. 재시작은 기존 live 파일을 보존하고 없으면 저장본에서 복구한다. 기존 임시 스크립트가 직접 바꾼 live 설정까지 자동 영구 저장하는 것은 아니다. 공개 CLI 편집 API·릴리스 기본 활성화·레이어/이력/부모 병합은 추가하지 않았다.

## 검증

작업 루트 `/Users/egoinglee/.codex/worktrees/67c9/agent-gibbon`, 예상 instance `dev-f3d1fc901f372efdf5d35b62`. 기존 main 병합과 미커밋 관계선 작업을 포함하는 현재 작업 트리 기준이다.

- Rust245개·UI196개 통과. unique target의 자기/경계/겹침 거부, 기존 JSON의 end 기본값, arrow roundtrip·잘못된 타입 거부, 같은 쌍/역순 갱신과 다른 항목 보존·개수 제한, toolbar 노출·source Escape 세대, pointer 최신 좌표·클릭 재판정, 선택 레이블·세 방향, 저장 실패 복구/중복 제출을 포함한다.
- Clippy all-targets `-D warnings`, 개발 빌드 통과. Rustfmt·diff 공백 검사 통과. 원시 로그는 `/tmp/gibbon168-motion/editor-{test,check,build}.log`에 임시 보관한다.
- 독립 headless Chrome/CDP에서 실제 설정 HTML/CSS/JS를 IPC mock과 함께380×330 viewport의 light/dark로 렌더했다. 입력 자동 포커스·폼/저장 버튼이 viewport 안에 있음을 검사하고 PNG를 직접 확인했다. `/tmp/gibbon168-motion/editor-preview/settings-{light,dark}.png`, `layout-{light,dark}.json`, `render.mjs`에 임시 보관했다. 실제 native 설정창 시험은 아니다.
- SVG/PNG 시안을 렌더 후 직접 확인했다. ui-ux-pro-max의 명시적 form label과 제출 피드백 지침을 적용했다. 모바일 터치 크기를 기존 desktop 툴바에 기계적으로 적용하지 않았다.

독립 headless 미리보기에서 실제 SVG의 곡선·화살촉·hover 테두리도 렌더하고 visible/path를 검사했다. `/tmp/gibbon168-motion/editor-preview/pick-preview.png`·`pick-layout.json`은 mock 대상 좌표의 브라우저 렌더이며 native 메모 hit-test 증거가 아니다.

## 실행 앱과 남은 검증

팀장에게 자기 인스턴스 시작 차례를 배정받은 뒤 앱 반영을 진행했다. 직전 CLI list는 ‘앱이 실행 중이지 않습니다’를 반환했고 libproc 실행 경로 대조에서도 자기 앱 PID가 없었다. 따라서 기존 프로세스를 종료한 재시작이 아니라, 저장 상태를 백업하고 현재 빌드를 새로 시작한 것이다. 다른 작업 앱·모니터 설정은 조작하지 않았다.

개발 CLI `--serve`에 기존 활성화 환경변수를 지정해 PID81735를 시작했다. 정확한 바이너리 경로·루트·instance와 4종류 예제의 native 창 제목·고유 본문 ‘01 · 기획’을 대조했다. 기존 live 관계 JSON은 바이트까지 동일한12개였다. 시작 전 저장 journal의44개 창에 대해 현재 inspect의 위치·크기·visible을 비교해 전부 일치했다.17개 메모가 표시됐고, 전체 숨김 active는 없었다. 이는 저장 상태와의 비교이며 직전 실행 앱이 없었으므로 ‘실화면의 현재 기하를 그대로 재현했다’는 별도 관측은 아니다.

검사 소스·binary SHA256, dev info, 원본 session/live 관계, 시작 후 inspect·native와 geometry 비교는 `/tmp/gibbon168-motion/editor-live/`에 임시 보관했다. 시작 후 기존 웹 메모 web-relation-20261115의 focused=true가 관측됐다. 시작 전 외부 전경 앱을 기록하지 않아 포커스 완전 보존 여부는 판정하지 않는다. 이 실행에서 명시적으로 메모/입력 표면을 focus하거나 포인터·전역키를 조작하지 않았다. 기존 관계선 표시면11개가 native 표시 상태인 것은 확인했지만 새 연결 편집 흐름이나 픽셀 끝점 실검증으로 사용하지 않는다.

실행 앱에는 새 binary/UI를 반영했으나 연결 모드를 활성화해 사용자 화면을 덮거나 실제 메모 클릭을 자동 실행하지 않았다. 툴바 버튼의 native 실제 렌더·선택 흐름은 아래 남은 검증과 구분한다. 자기 앱 시작 차례는 팀장에게 반환했고 예제와 사용자 앱은 유지했다.

실제 Mac에서 버튼 노출, 비포커스 표면의 첫 클릭 수신/출발 창 Esc, 화면 간 포인터·native 좌표, 양끝 숨김/삭제·모드 종료, 실제 SVG arrow 방향·클릭 통과·저장 후 재시작 복구는 미검증이다. 웹 external child 본문/터미널 본문의 포커스, Windows·Intel·혼합 DPI도 별도다. CUA는 앞선 검증에서 debug 바이너리를 앱으로 선택하지 못했으며 다른 입력 기술로 우회하지 않았다. 자기 앱 반영은 완료했으며 실제 선택 흐름 확인은 별도다.

Conf3 / RiskMedium(포인터 입력 표면·관계 저장·포커스 전환). 제품 커밋·push·병합은 하지 않았다. 기존 무관한 package-lock.json과 main 병합 전 백업은 보존했다.

## 모니터 복구 main 통합 — 2026-11-15 후속

최신 `origin/main`의 `57231c3d36e5507dcdd21593f678f56c0d784c46`을 관계선 브랜치에 통합했다. 병합 커밋은 `0eb4c12`이며 연결 편집 UI 제품 변경은 이 커밋에 포함하지 않고 계속 미커밋으로 남겼다. main으로의 역병합·push는 하지 않았다.

현재 변경32개 파일을 tar/전체 binary patch/index 사본/개별 SHA256으로 `/tmp/gibbon-monitor-main-integration-20261115-212428/`에 분리 백업하고 tar 내용의 해시를 대조했다. index는 원래 staged 변경이 없었다. 추적 파일만 임시로 HEAD 내용으로 돌려 main 병합을 완료한 뒤 백업 patch를3-way 적용했다. 자동 stash·강제 reset·무관 제품 커밋은 사용하지 않았다.

복원 즉시32개 원본이 전부 존재하며29개는 바이트까지 동일했다. 차이가 있는 디자인 시스템·main.rs·web.rs3개는 main 변경과 기존 UI를 합친 부분이었다. 무관한 package-lock.json은 원본 해시와 같고, 미추적 파일도 원위치에 유지했다. index는 통합 후 기존 제품 작업을 모두 unstage하여 원래의 ‘제품 변경 미스테이징’ 상태로 복원했다.

충돌은 main.rs의 native event와 sizing manual_resize의 두 훅을 함께 유지해 해결했다. UI patch 재적용 시 main.rs의 설정창 취소 처리와 monitor native_event를 함께 보존하고, web drag/resize에서 relation changed와 monitor manual을 모두 유지했다. 초기 `Targets::initial()`·Ready 소비, 확대 복원·이미지 툴바·자동 배치 제외·모니터 journal 필드도 main 그대로 보존했다.

추가로 monitor manual/native_event의 시작 지점에서 등록 대상 분류를 확인해 relation-prototype/pick/settings 같은 보조 창을 수동 이동 기록에 넣지 않게 했다. 기존 모니터 재배치 대상 필터도 이 창들을 제외한다. 보조 창 제외 및 실제 메모 유형 포함의 회귀 검사를 추가했다. 이 방어 코드와 검사는 UI 작업과 함께 미커밋이다.

최신 통합 작업 트리의 Rust258개·UI196개·Clippy all-targets `-D warnings`·Rustfmt·개발 빌드·diff 공백 검사를 통과했다. 검사한 제품 소스와 새 binary 해시·병합 부모·lock 보존·index 상태는 같은 백업 디렉터리의 `final-verification.json`에 기록했다. 원시 로그는 `/tmp/gibbon168-motion/integrated-{test,check,build}.log`다. 제품 소스 검증 후 문서 기록만 추가했으며 코드·시안 외형은 바꾸지 않았다.

실행 중인 PID81735는 이전 UI 적용 때 시작한 구빌드다. 새 빌드 파일과 시작 시 보관한 binary 해시가 다르며, 이번 통합에서는 앱·PTY를 종료하거나 재시작하지 않았다. 현재 창 기하·숨김·관계 설정을 바꾸는 native/GUI 명령도 실행하지 않았다. 따라서 위 결과는 최신 소스의 자동 검사·빌드이며 최신 모니터 복구+연결 선택 흐름의 실환경 통과는 아니다. 기존 UI에 남아 있던 실제 포인터 선택·첫 클릭·설정 저장·복구 검증 한계도 그대로 남는다.

## 권한 메뉴 main 증분 통합 — 최종 검토 기준

`f6a08cadd51ab0056ce5bf4ddcbb30d3240f1364`의 권한 메뉴 수정6파일을 `269eaea`로 병합했다. 부모는 `0eb4c12`와 `f6a08ca`다. 코드 충돌은 없었고 미커밋 문서 복원 시 끝에 추가된 두 문서 영역을 함께 보존했다. 이번 병합 커밋에는 main에서 온6파일만 포함했으며 연결 UI 제품을 섞지 않았다.

이번 소규모 통합 직전33개 기존 변경의 SHA를 기록하고 docs/design-system 원본·patch만 별도 백업했다(`/tmp/gibbon-permission-main-integration-20261115-213150/`). 다른32파일은 바이트까지 동일했으며, 디자인 시스템의 기존 전문도 복원된 문서에 그대로 존재함을 검증했다. index는 원래처럼 제품 미스테이징 상태이며 package-lock 원본도 보존했다. 자동 stash·reset·사용자 앱 재시작은 없었다.

새 기준의 증분 검사는 Rust tray::tests7개(권한 완료 기록·재조회·실제 차단 후 재노출 포함), relation_12개(등록 타입·세대/표시·보조 창 분류·저장 모델), web::3개 및 관계/렌더/웹 chrome UI8개가 통과했다. Clippy all-targets `-D warnings`·개발 빌드·Rustfmt·diff 공백 검사도 통과했다. 이전57231 기준의 전체258/196 검사와 구분하며 새 기준 전체 검사를 실행했다고 보고하지 않는다. 로그는 `/tmp/gibbon168-motion/permission-{tray,relation,web,ui}-tests.log`, `permission-{check,build}.log`다.

permission-setup-complete marker와 관계 저장 파일은 별개다. picker가 capture::begin을 호출하지 않아 권한 안내 재노출 흐름으로 들어가지 않으며 external webpage IPC 차단도 보존했다. 이 경계는 소스 검토·관련 검사이며 실제 시스템 권한 회수/복원 시험을 실행하지 않았다.

[최종 검토 자료](relation-editor-review-20261115.md)에 코드·문서·시안 묶음, 지원 유형, 저장 한계, 남은 native 검증과 승인 경계를 모았다. 현재 실행 앱81735는 앞서 시작한 구빌드이며 이번 최신 source/binary의 실환경 증거가 아니다. 사용자 앱·PTY·관계·창 기하를 변경하는 명령은 실행하지 않았다. 최신 binary 적용과 실제 연결 클릭 검증에는 별도 실행 차례가 필요하다. 제품 최종 커밋·push·PR169 병합은 하지 않았다.

## 최신 binary 적용 차례 — 2026-11-15 21:39 후속

팀장이 자기 앱 시작/읽기 상태 대조 차례를 배정했다. 직전 list는 앱 미실행, exact 바이너리 경로의 PID도 없었다. 현재 live/durable 관계와 journal을 `/tmp/gibbon168-motion/editor-live-latest-269eaea/`에 백업하고 최종 검토 manifest의 코드/binary 해시 일치를 검사한 뒤 CLI `--serve`로 시작했다. 다른 앱·PTY를 종료하지 않았고, 새 fixture·포인터 입력·연결 모드 활성화·관계 저장을 하지 않았다.

시작 PID20701의 inspect 응답에서 instance 일치·기존 예제 ‘01 · 기획’ 고유 본문·visible을 확인했다. 현재 live JSON은 시작 직전 백업과 바이트까지 동일한12개였고17개 메모가 visible이었다. 과거12개 백업을 덮어쓴 것이 아니라 이번 직전 live 파일을 그대로 유지한 비교다.

저장된44프레임 중43개는 xywh/visible 일치했다. terminal-202611150011만 y1510→1466, 크기848×838·x3160·visible은 같았다. 저장 frame의 아래 끝2348이 현재 주 화면 아래 끝2304보다44물리px 아래이며 새 frame은2304에 맞는다. 모니터 복구의 화면 경계 보정과 일치하는 정황이나, 실제 monitor runtime 진단을 뒤이어 수집하기 전에 앱이 종료돼 해당 경로의 직접 판정까지 확보하지 못했다. 이전 위치로 강제 복원하지 않았다.

native 메타데이터에서는 같은 PID 창29개가 있었지만 제목 필드는 모두 누락돼 이번에는 native 제목을 대조했다고 주장하지 않는다. inspect source/instance/binary 근거와 native 제목 검증의 한계를 구분한다. 시작 후 기존 웹 메모의 focused=true가 있었으며 시작 전 외부 전경 앱을 기록하지 않아 포커스 완전 보존은 판정하지 않는다.

차례 수행 뒤 턴이 중단됐고, 후속 read-only list/diagnostics는 앱 미실행, PID20701도 없었다. runtime.log에는 시작 시 macOS 입력기 메시지 한 줄만 있으며 종료 원인을 확정할 로그는 없다. 사용자 종료·중단 영향·제품 종료 중 하나로 단정하지 않는다. 별도로 다시 시작하지 않았다. 이 결과는 최신 binary가 실제 시작해 상태를 반환한 증거이며 현재 계속 실행 중이라는 뜻은 아니다.

첫 사용자 확인은 앱이 실행된 상태에서 고유 본문 ‘01 · 기획’ 메모의 플러그 버튼을 누르고, 다른 예제 쪽으로 이동해 곡선/대상 테두리만 확인한 뒤 Esc 또는 화면 ‘취소 · Esc’로 종료하는 것이다. 대상 클릭·설정 저장은 이 단계에 포함하지 않는다. 실제 첫 클릭/취소가 확인된 뒤 저장·방향·재시작 지속을 별도 단계로 확인해야 한다.

21:41(+09:00) 후속 출처 대조에서도 자기 CLI list/inspect는 모두 미실행이었다. app.sock/app.lock 파일은 남아 있었지만 lsof 소유 PID가 없었다. 파일 존재만으로 서버가 실행 중이라고 판단하지 않는다. 실행 중인 유일한 agent-gibbon PID28742는 libproc 및 lsof cwd/txt 모두 permission-tray-visibility 워크트리의8번 앱이었다. 다른 앱으로 대체하거나 종료하지 않았다. 자기 live 관계12개는 그대로였다. `process-followup.json`에 조회 시각·파일/소유 PID·정확한 바이너리 경로를 기록했다.

## 일반 개발 실행의 연결 기본값 수정

사용자가 버튼과 선이 보이지 않는다고 다시 보고했다. 그 시점에는67c9 앱PID95684/watcher95287이 실행 중이었지만 연결 환경옵션이1이 아니었다. ‘다른 워크트리 앱이 실행됨’만으로 현재 현상을 설명할 수 없었으며, 일반 개발 실행 시 옵션을 주입하지 않는 재발 경로를 확인했다.

scripts/dev-instance.mjs의 devEnvironment에서 AGENT_GIBBON_RELATION_PROTOTYPE을 기본1로 주입한다. 명시적0은 유지하며 buildEnvironment에는 기본값을 추가하지 않는다. Rust backend의 debug/개발 instance 조건은 그대로여서 설치/release를 기본 활성화하지 않는다. Node dev-instance/관계 editor/render11개 검사를 통과했다. Rust/UI 바이너리 코드는 이 수정에서 바꾸지 않았다.

최초 재실행 guard에서 stash.active=true여서 어떤 프로세스도 종료하지 않았다. 사용자 ‘전체 복원했음’ 응답 후 모든 inspect의 stash inactive·진입/측정 대기 없음·실행 터미널 없음·저장 버튼 초안 없음과 두 번의 안정된 위치/크기/표시/revision을 확인했다. 현재 live/durable/journal을 백업하고 정확한 자기 watcher95287/앱95684만 종료했다.

새 프로세스에는 연결 옵션을 명시하지 않고 개발 CLI --serve로 시작해 실행기의 기본값이 적용됨을 검증했다. 새PID11761의 exact binary 경로와 실제 환경에서 옵션1을 확인했다. instance 응답 일치·‘01 · 기획’ 본문을 확인했고 현재48개 메모의 위치·크기·숨김·접힘이 전부 직전 snapshot과 같았다. 현재 live12관계는 바이트까지 동일했다. 다른 앱과 PTY·모니터 설정은 조작하지 않았다.

증거는 `/tmp/gibbon168-motion/default-connection-fix/{before,after,result}.json`, 현재 관계/저장 원본, runtime.log와 `default-connection-tests.log`다. 미커밋으로 유지하며 native 실제 버튼 클릭·포인터 미리보기·Esc 취소/저장 확인은 사용자 시험 전이다. 기존 실제 선택 native 한계를 통과로 바꾸지 않는다.

## 텍스트·캡처 툴바 DOM 초기화 수정

사용자는 웹·터미널과 달리 텍스트·캡처에서 플러그가 없다고 보고했다. index.html은 relation-toolbar.js를 일반 script로 head에서 로드했지만 이 스크립트는 즉시 toolbar-items를 찾아 append하므로 body 파싱 전 null로 실패했다. 웹·터미널은 같은 스크립트를 body 끝에서 실행해 이 문제가 없었다. 앞선 VM 검사에서는 DOM을 미리 만들어 원인을 놓쳤다.

index.html의 해당 script에 defer를 추가했다. 실제 문서 파싱 순서를 유지한 독립 headless Chrome/CDP fixture에서 text-note/image-note 각각 기존 script는 버튼 부재, defer 적용은 버튼 존재·hidden=false·실제 rect 너비>0임을 확인했다.4개 비교 결과와 렌더는 `/tmp/gibbon168-motion/toolbar-dom/results.json`, `{text,capture}-fixed.png`에 있다. 실제 앱 native 버튼 증거와 구분한다. 관련 Node editor/render7개 검사와 개발 빌드는 통과했다. 코드는미커밋이다.

실앱 적용 직전 stash.active=true를 관측했을 때 중단했고, 이후 inactive에서 재시도했으나 자기 앱의 terminal-202611150022(‘03 구현 · 터미널’)에 running=true/PTY pid16265가 있어 종료 전 다시 중단했다. 앱·watcher·PTY를 종료하지 않았다. 사용자에게 셸 종료를 동반한 즉시 반영 또는 터미널 작업 후 반영 중 선택을 요청했다. 수정 source/binary가 현재 프로세스에 반영됐다고 보고하지 않는다.

사용자가 터미널 작업 후 반영을 선택한 뒤 ‘재 실행’을 명시했다. 적용 직전 자기 앱은 이미 미실행이어서 실행 중 PTY나 앱을 종료하지 않고 현재 저장 상태를 백업해 수정 빌드로 시작했다. PID35440/예상 instance·binary 경로와 기본 연결 옵션1을 확인했다. 현재 live 관계는 사용자가 추가한 것까지14개였고 직전 백업과 바이트까지 동일했다. 저장된44프레임과 시작 후 inspect가 모두 일치하고17개 메모가 표시됐다. 예전12개로 덮어쓰지 않았다. 증거는 `/tmp/gibbon168-motion/toolbar-restarted-approved/`이며 실제 플러그 클릭·연결 미리보기는 사용자 확인 전이다.

## 제품 기본 기능 전환·배포 준비

사용자가 현재 구현의 제품 반영과 환경변수 활성화 제거를 요청했다. install의 환경변수·debug_assertions·개발 instance 전용 return 조건을 모두 제거해 정상 앱 setup에서 관계 state/editor를 초기화한다. devEnvironment의 연결 옵션 기본값 주입도 제거했다. 기존 환경변수의0/1 값은 활성화 판단에 사용하지 않는다. 외부 webpage IPC 차단·privacy gate·등록 host 검증·관계 파일·세대 검사는 그대로다.

최신 소스에서 Rust259개·UI197개·Clippy all-targets를 통과했다. `pnpm build`로 현재 Mac의 release app bundle을 생성했다. 경로는 워크트리 `src-tauri/target/release/bundle/macos/Agent Gibbon.app`이며 단순 컴파일/번들 생성이지 설치·실행·공식 배포 검증은 아니다. 원시 로그는 `/tmp/gibbon168-motion/always-on-{test,check,release-build}.log`에 있다. 이 준비 단계에서는 사용자 앱·PTY를 종료하거나 설치 앱을 교체하지 않았다.

데스크톱 앱에는 별도 실서버가 없으므로 ‘실서버 반영’이 이 Mac의 설치 앱 업데이트·main 병합·공식 릴리스 중 어느 범위인지 사용자에게 확인 요청했다. 답변 전에는 제품 최종 커밋·push·main 병합·태그/공개·설치 교체를 실행하지 않는다. 기존 모든 미커밋과 무관한 package-lock.json은 보존한다.

별도 사용자 요청3개는 #168의 실제 서브 이슈 #198/#199/#200으로 생성하고 부모→자식 목록 및 자식→부모 응답을 모두 검증했다. 첨부된 화살촉 화면은 로컬 `assets/issue168/arrow-artifact-user-20261115.png`에 보존했으며 아직 미커밋/미게시다. 이 이슈화와 이번 기본 기능 전환은 해당 후속 구현 완료를 뜻하지 않는다.
