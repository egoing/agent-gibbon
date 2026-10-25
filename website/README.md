# 홈페이지 배포

공식 주소: https://agentgibbon.com/  
호스팅: Vercel `egoings-projects/agent-gibbon`  
콘텐츠: `website/dist/`의 정적 HTML/CSS/JS. 앱 빌드와 별개이며 빌드 명령과 패키지 설치가 필요하지 않습니다.

## 배포

Vercel 프로젝트의 Git Root Directory는 `website`, Output Directory는 `dist`로 설정합니다. Git 연결 후 main 푸시는 프로덕션 배포, 다른 브랜치는 미리보기 대상입니다.

CLI를 쓰는 경우 저장소 루트에서 실행합니다.

```sh
npx vercel@latest deploy --prod --scope egoings-projects --project agent-gibbon
```

프로젝트 Root Directory가 `website`로 설정된 이후에는 CLI에도 저장소 루트를 전달합니다. `website` 안에서 중복으로 하위 경로를 선택하지 마세요.

Cloudflare는 DNS만 관리합니다. apex A 레코드는 `216.150.1.1`, `216.150.16.1`, www A 레코드는 `216.150.1.1`이며 DNS only입니다. 이는 2026-10-26 Vercel 검증 결과의 권장 주소로, 이후 변경 시 `vercel domains verify`가 안내하는 값을 사용합니다. www는 Vercel 프로젝트 Domains 설정에서 apex로 HTTP 308 영구 리다이렉트합니다.

이전 OpenAI Sites 배포 설정은 저장소에서 제거했습니다. 현재 배포 설정은 `vercel.json`입니다. 이전 호스팅의 콘텐츠 자체는 전환 직후 복구를 위해 보존했습니다.

## 검색과 AI 검색

- 한국어 `/`, 영어 `/en/`, 설치 예제 `/guide/`를 JavaScript 없이 읽을 수 있습니다.
- canonical, 언어별 hreflang, 제품 구조화 데이터, robots.txt, sitemap.xml을 제공합니다.
- robots.txt는 검색 크롤러를 허용합니다. 실제 검색 등록·노출·추천은 보장하지 않습니다.
- Google Search Console의 `sc-domain:agentgibbon.com` 소유권 확인과 `https://agentgibbon.com/sitemap.xml` 제출을 완료했습니다(2026-10-26). 처리 상태는 성공이며 3개 페이지를 발견했습니다. 이는 색인 완료를 뜻하지 않습니다. Cloudflare의 Google 소유권 확인 TXT 레코드는 유지하세요. Bing Webmaster Tools 등록은 아직 수행하지 않았습니다.
- 릴리스와 main의 기능 차이를 문서에 표시하고 다운로드 링크는 최신 릴리스 페이지를 사용합니다.

검증: 각 경로 HTTP 200, 없는 경로 HTTP 404, www 영구 리다이렉트, 검색 봇 User-Agent 응답, HTML 메타데이터와 내부 링크, 브라우저 데모/언어 이동을 확인합니다.

## 2026-10-26 이전 검증

Vercel과 Cloudflare DNS 전환 후 apex와 www 도메인의 Vercel 검증에 통과했습니다. 프로덕션 HTML과 CSS/JS/SVG, robots.txt, sitemap.xml은 인증 없이 HTTP 200을 반환합니다. Googlebot·bingbot·OAI-SearchBot·PerplexityBot User-Agent 요청도 200을 확인했습니다. 이는 요청 허용 검증이며 실제 검색 서비스 색인이나 추천 확인은 아닙니다. canonical, 페이지별 H1, 설명, 내부 링크와 사이트맵 XML을 확인하고 브라우저에서 데모 선택과 영어/설치 안내 이동을 검증했습니다. 없는 경로는 404, www는 apex의 같은 경로로 308을 반환합니다.
