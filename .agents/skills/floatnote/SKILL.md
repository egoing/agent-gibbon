---
name: floatnote
description: 이 프로젝트의 Floatnote CLI로 작업 진행과 결과를 단일 플로팅 Markdown 메모에 표시한다. 사용자가 플로팅 메모 표시나 갱신을 요청할 때 사용한다.
---

# Floatnote 사용

저장소 루트의 `scripts/floatnote`를 사용한다. 다른 디렉터리에서는 절대 경로로 호출한다. 실행 파일이 없으면 README의 빌드 절차를 수행한다. 셸 인수는 안전하게 인용하고, 긴 Markdown은 파일 또는 표준입력으로 전달한다.

```sh
./scripts/floatnote open --title '현재 작업' --status '진행 중' --file examples/progress.md
./scripts/floatnote update --status '검증 중' --file examples/progress.md
./scripts/floatnote update --status '완료'
./scripts/floatnote inspect
./scripts/floatnote close
```

- 최초 표시와 사용자의 명시적인 복원 요청에만 `open`을 사용한다. `open`은 필요하면 앱도 실행한다.
- 진행 변경은 `update`로 보낸다. 숨긴 창은 그대로 숨겨진 채 내용만 갱신된다. 갱신 실패를 `open`으로 자동 대체하지 않는다.
- 생략한 제목·본문·상태는 기존 값을 유지한다. 빈 문자열은 해당 필드를 비운다.
- `close`는 숨기기다. 프로세스가 유지되어 이후 갱신을 받을 수 있다.
- 종료 코드와 JSON의 `ok`를 확인한다. `inspect`로 가시성·위치·크기·포커스·상단 고정과 내용을 읽을 수 있다. 메모 원문이나 민감한 내용을 외부로 전송하지 않는다.
- 체크리스트는 읽기 전용이다. 사용자 답변, 다중 메모, 자동 배치, 클라우드는 지원하지 않는다.
- 재실행 시 내용과 창 위치는 초기화된다. 앱이 실행 중인 동안에는 닫기·갱신·다시 열기로 위치를 바꾸지 않는다.

실제 사용 예제: 저장소의 `examples/progress.md`.
