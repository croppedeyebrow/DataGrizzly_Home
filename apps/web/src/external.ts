/**
 * 개념: 저장소의 예시 선택과 SEO 함수를 공개 웹 타입으로 연다.
 * 목적: 페이지가 fixture 규칙을 다시 만들지 않게 한다.
 * 사용처: apps/web/src/prerender.ts
 */
export { loadPublicHome } from '../../../scripts/public-examples.mjs'
export { loadPublicProjects } from '../../../scripts/public-projects.mjs'
export { loadPublicResearch } from '../../../scripts/public-research.mjs'
export { loadPublicTaxonomy } from '../../../scripts/public-taxonomy.mjs'
export { loadPublicIndex } from '../../../scripts/public-index.mjs'
export { buildMeta, buildRss, buildSitemap } from '../../../packages/seo/meta.mjs'
export { siteOrigin } from '../../../packages/seo/site-origin.mjs'
