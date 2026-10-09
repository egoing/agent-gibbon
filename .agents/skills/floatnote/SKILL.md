---
name: floatnote
description: Floatnote CLI로 플로팅 Markdown 메모 창을 생성·조회·갱신·숨기고 창별 색상을 지정한다. 플로팅 메모 조작이 필요할 때 사용한다.
---

# Floatnote 사용

저장소 루트의 `scripts/floatnote`를 사용한다. 다른 디렉터리에서는 절대 경로로 호출한다. 실행 파일이 없으면 README의 빌드 절차를 수행한다. 셸 인수는 안전하게 인용하고, 긴 Markdown은 파일 또는 표준입력으로 전달한다.

```sh
./scripts/floatnote list
./scripts/floatnote open --id my-note --title '메모' --color '#fff1b8' --file examples/progress.md
./scripts/floatnote update --id my-note --status '갱신됨' --file examples/progress.md
./scripts/floatnote update --id my-note --color '#dcecff'
./scripts/floatnote inspect --id my-note
./scripts/floatnote close --id my-note
```

- `--id`는 영문·숫자·하이픈·밑줄 1~64자이며 대소문자를 구분한다. 생략하면 기본 창 `main`을 대상으로 한다. 같은 ID는 같은 창을 재사용하고 다른 ID는 별도 창을 만든다.
- `list`는 숨긴 창까지 ID순으로 제목·상태·색상·가시성·상단 고정을 반환한다. 본문은 `inspect --id ID`로 조회한다.
- `--color '#RRGGBB'`는 창 전체 배경색이다. 글자색은 자동 선택된다. 기본색은 `#20252e`, 생략하면 보존한다. 빈 문자열·색상 이름·알파값은 허용하지 않는다.
- 최초 표시와 사용자의 명시적인 복원 요청에만 `open`을 사용한다. `open`은 필요하면 앱과 해당 창을 생성한다. 없는 ID에 대한 `update`·`close`·`inspect`는 실패한다.
- 진행 변경은 `update`로 보낸다. 숨긴 창은 그대로 숨겨진 채 내용만 갱신된다. 갱신 실패를 `open`으로 자동 대체하지 않는다.
- 생략한 제목·본문·상태는 기존 값을 유지한다. 이 세 필드의 빈 문자열은 해당 필드를 비운다. 창별 내용·색상·위치·크기·숨김·상단 고정은 서로 독립적이다.
- `close`는 숨기기다. 프로세스가 유지되어 이후 갱신을 받을 수 있다.
- 종료 코드와 JSON의 `ok`를 확인한다. `inspect`로 가시성·위치·크기·포커스·상단 고정과 내용을 읽을 수 있다. 메모 원문이나 민감한 내용을 외부로 전송하지 않는다.
- 체크리스트는 읽기 전용이다. 사용자 답변, 자동 배치, 클라우드는 지원하지 않는다.
- 프로세스를 종료하면 모든 메모·색상·창 위치는 초기화된다. `close`는 창을 삭제하지 않으며 숨긴 창도 프로세스 메모리에 남는다. 앱이 실행 중인 동안에는 닫기·갱신·다시 열기로 위치를 바꾸지 않는다.

실제 사용 예제: 저장소의 `examples/progress.md`.

표시할 내용과 갱신 시점은 사용자 또는 호출한 워크플로우가 정한다. 이 스킬은 작업계획 작성이나 완료 체크를 강제하지 않는다.
