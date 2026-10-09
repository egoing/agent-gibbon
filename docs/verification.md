# 프로토타입 검증 기록

검증일: 2026-10-10 (Asia/Seoul)

## 환경

| 항목 | 확인 결과 |
|---|---|
| macOS | 15.8 (24H23) |
| CPU / 실행 파일 | Intel x86_64 / Mach-O 64-bit x86_64 |
| Rust / Cargo | 1.97.1 |
| Node.js / npm | 24.21.0 / 11.19.0 |
| Tauri Rust / CLI | 2.12.2 / 2.12.1 |
| 개발 도구 | Command Line Tools와 macOS SDK 설치됨 |
| 전체 Xcode | 미설치 (`xcodebuild -version` 실패) |
| origin | egoing/floatnote, private 확인 |

전체 Xcode 없이 설치된 CLT/SDK로 데스크톱 release 빌드와 `.app` 번들 생성에 성공했습니다. CLT 경로는 `/Library/Developer/CommandLineTools`입니다. Clippy는 누락되어 rustup으로 설치한 뒤 사용했습니다.

## 수행 결과

| 검증 | 결과 |
|---|---|
| Rust 단위 테스트 | 4개 통과: 안전한 Markdown, 부분 갱신, 거부된 요청, 잘못된 CLI 옵션 |
| Clippy | 모든 타깃, 경고를 오류로 처리하여 통과 |
| Rustfmt / JS 문법 | 통과 |
| release 빌드 / 앱 번들 | x86_64 성공, 앱 번들 약 11.83MiB |
| 실제 CLI | 자동 실행, JSON 응답/오류 종료 코드, 연속 갱신, 숨김, 재열기 통과 |
| 창 상태 보존 | 갱신 전후 실제 네이티브 위치·크기·포커스·가시성·상단 고정 비교 통과 |
| 실제 WebView | 접근성 트리에서 한글 제목·본문·☑/☐·갱신 시각 확인 |
| 앱 수명 / 시작 포커스 | 새 자동 실행과 별도 명령 세션에서 동일 프로세스 유지, Orca 포커스 보존 확인 |
| 실제 버튼 | 상단 고정 켜기/끄기, 닫기 버튼, 닫은 뒤 update로 숨김 유지 확인 |
| 마우스 드래그 | 제목 영역 드래그로 물리 좌표 (1540,348) → (1620,408) 변화 확인 |
| 마우스 크기 조절 | 우하단 드래그로 물리 크기 760×920 → 860×1000 변화 확인 |
| 화면 검토 | 아래 앱 창 캡처로 텍스트·체크리스트·상태·버튼 표시 확인 |
| 로컬 스킬 | 스킬 검증기 통과, 사용 예제 CLI 실행 확인 |

![실제 macOS WebView 화면](floatnote.png)

## 재현

```sh
npm ci
npm run build
npm test
npm run check
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
node --check ui/app.js
python3 scripts/smoke.py
python3 scripts/macos-ui-smoke.py
```

UI 검증은 macOS 자동화/손쉬운 사용 권한이 필요하며 실행 중 다른 앱에 포커스를 둡니다. 두 스크립트 모두 검증용 메모로 교체하고 마지막에 숨깁니다. 드래그·크기 조절은 실제 CGEvent 마우스 입력으로 별도 확인했습니다. 마우스 위치는 입력 후 복원하고 검증 뒤 Orca로 포커스를 돌렸습니다.

## 범위와 한계

- 확인한 실제 환경은 이 Intel Mac 한 대입니다. Apple Silicon, Linux, Windows 검증은 수행하지 않았습니다. 첫 버전의 IPC는 Unix 전용입니다.
- 배포 서명·공증·DMG·다른 Mac 설치 검증은 수행하지 않았습니다. 로컬 앱 번들만 생성했습니다.
- 메모 내용·창 위치는 프로세스 수명 동안만 유지됩니다. 영구 저장·재부팅 후 자동 실행은 없습니다.
- 체크리스트는 읽기 전용이며 이미지·링크 이동·원시 HTML 실행은 지원하지 않습니다.
- Conf 5: 이 환경에서 실제 앱과 CLI를 실행해 확인했습니다. Risk Low: 로컬 단일 메모와 개발 파일 범위이며 쉽게 되돌릴 수 있습니다.
