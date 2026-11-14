# 웹 메모 native 배치와 모바일 표시 개선

이슈: #177. 기준 main `91b1751`. 브랜치 `codex/web-mobile-chrome`.

## 완료 단계

- [x] 관련 이슈·디자인 시스템·연결 SVG/PNG·첨부 화면 확인
- [x] 설치된 Tauri/Wry 소스와 공식 API에서 native 배치·모바일 기능·스크롤 제약 확인
- [x] 외부 콘텐츠가 앱 툴바를 침범하지 않도록 배치 수정, 모바일 UA와 지원 native 기능 적용
- [ ] 외부 IPC 거부·HTTP(S) 제한 유지 및 관련 Rust/UI 검사·개발 빌드
- [ ] 실제 HTTP UA와 navigator/viewport 및 Welcome 전환·탐색·resize·접힘·숨김 확인
- [ ] 디자인 시스템·SVG/PNG·검증 기록 갱신, 한계와 Conf/Risk 보고

## 경계

#158 신규 탐색 버튼, #119 닫기 계약, 15번 terminal/image, 17번 공통 toolbar 더블클릭은 별도다. 17번은 web_chrome match action만 추가하며 web.rs 배치와 모바일 설정은 16번이 소유한다. 임의 사이트의 JS 권한이나 host bridge를 확대하지 않는다. 사이트별 모바일 판정은 보장할 수 없으며 3px overlay는 native WebView가 제공하는 제약을 확인해 가장 근접한 fallback과 근거를 제시한다.

첫 계획 커밋·push·Draft PR은 승인 범위다. 제품 최종 커밋·push·병합은 별도 승인 대기한다. 전체 플랫폼 CI·배포는 실행하지 않는다. GUI는 팀장 슬롯 승인 후 출처를 확인하고 사용자 상태를 보존한다.

## 현재 검증 상태

Rust223/UI204·Clippy·개발 빌드와 Chromium toolbar15조합·presentation 검사를 통과했다. 실제 native에서 Welcome→redirect 뒤 toolbar 유지·HTTP/navigator 모바일UA 일치·second/back을 확인했다. [검증 기록](../verification/issue177-web-mobile-chrome.md)과 [디자인 시스템](../design-system.md)에 기준·스크롤 한계 및 미검증을 기록했다.

사용자 피드백에 따른 팀장의 추가 Orca 실행/재기동·native GUI 조작 중단 지시를 준수한다. 공개 NSScrollView Overlay 후보와 보안probe/resize/drag/fold/reload/권한은 아직 native 검증 전이며 완료로 표시하지 않는다. GUI 검증 방식 결정 뒤 이어간다. 최종 제품 커밋·push·병합은 하지 않았다.

사용자 dev시작 패닉 제보 대응: nullable WKWebView URL을 강제로 읽던 inspect 경로를 교체하고 실제관측 URL/허용탐색/요청주소를 구분했다. session_restore의 null fallback과 회귀를 추가했으며 Rust224/UI204·Clippy·빌드·실제 dev시작·오프라인nil·온라인302후 실제URL을 확인했다. 보안 probe는 hidden 복원 페이지 자동실행으로 permission계층 거부를 확인했고 추가 GUI/Orca는 사용하지 않았다. 상세 기록은 위 검증 기록을 따른다.
