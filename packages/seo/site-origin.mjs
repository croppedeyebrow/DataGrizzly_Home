/**
 * 개념: 공개 사이트의 기준 주소.
 * 목적: canonical과 수집 문서가 요청 호스트를 따라가지 않게, 환경에 적힌 값만 쓰게 한다.
 * 사용처: apps/web/src/layouts/Base.astro, apps/web/src/pages/sitemap.xml.ts, apps/web/src/pages/rss.xml.ts
 * 도메인은 미정이다. 빈 문자열이 그 명시값이며, localhost나 example.com으로 채우지 않는다.
 */
export const siteOrigin = ''
