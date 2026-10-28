/**
 * 개념: 공개 주소 사이트맵.
 * 목적: 지금 공개된 경로만 XML로 낸다. 사이트 주소가 없으면 절대 URL을 비운다.
 * 사용처: 공개 경로 `/sitemap.xml`. 관리 화면, 미리보기, 초안, 404는 넣지 않는다.
 */
import { buildSitemap } from '../../../../packages/seo/meta.mjs';
import { siteOrigin } from '../../../../packages/seo/site-origin.mjs';
import { loadPublicIndex } from '../../../../scripts/public-index.mjs';

export const prerender = true;

type IndexRecord = {
  path: string;
  title?: string;
  kind?: string;
  updatedAt?: string;
  draft?: boolean;
};

export function GET() {
  const index = loadPublicIndex('../../fixtures/content') as { records: IndexRecord[] };
  const result = buildSitemap(index.records, siteOrigin) as { xml: string };
  return new Response(result.xml, {
    headers: { 'Content-Type': 'application/xml; charset=utf-8' },
  });
}
