# is2 독립 감사

2026-11-08, 구현 대화를 상속하지 않은 `/root/audit117`(Codex, 세부 모델 버전 미노출)이 읽기 전용으로 검토했다. 별도 터미널·워크스페이스는 생성하지 않았다.

대상 HEAD `4f2035ea6ef0b20d227f24d88c64155339bafaf9`, base `aa75157316a380b4f933db2d2fc41dc07a4c5fe3`, red `7f6b9dbacee3d4fbd1f3557339ffa987f7b9396e`와 스크롤 후 AX 추가·문서 링크 정정.

| 검사 | 판정 | 근거 |
|---|---|---|
| 테스트 완화 | 미발동 | red는 테스트만 7줄 추가. RED→HEAD assertion 변화 없이 판정 구현만 변경. sizing.rs contact_tests, red.log assertion 실패, green.log 통과 |
| 퇴행 | 미발동 | poll_fold와 macOS monitor 모두 같은 판정. 일반 메모 true 유지, GUI 높이 136→24→136 및 위치 복원 |
| 스코프 이탈 | 미발동 | 안내 제외 및 NOTICE_LABEL 가시성, 관련 문서·검증 증거뿐. 업데이트 주기·설치 정책·#112 변경 없음 |

- Honesty clean, Risk Low, Conf 4/5.
- 임시 fixture는 증거 diff에만 있으며 최종 updater.rs에는 남지 않았다. 실제 show_ready_notice/NOTICE_LABEL 생성 경로를 사용했다.
- instance·inspect·AX·캡처의 출처 연결을 확인했다. binary↔PID는 문서의 ps 기록에 근거하며 별도 원본 ps 파일은 없다.
- 스크롤 후 baseline 안내 노드와 Ctrl 접촉 AX 3회가 완전히 동일하다. 캡처에서 보류 문구·두 버튼을 확인했다.
- 버튼 클릭 성공 자체가 아닌 AX geometry로 클릭 영역 유지를 확인한 범위다. 실제 업데이트 설치·Windows/Intel GUI는 미검증이다.
- fmt 빈 로그는 단독으로 exit code 복원이 불가능하므로 최종 SHA 재검증에서 별도 exit code 기록을 보완한다.
- 병합 전 최종 SHA Release dispatch의 URL/headSha 및 세 플랫폼 통과 확인이 필요하다.
