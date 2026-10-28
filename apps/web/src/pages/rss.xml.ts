/**
 * 개념: 공개 글 RSS.
 * 목적: 공개된 글만 피드에 담는다. 사이트 주소가 없으면 항목의 절대 링크를 넣지 않는다.
 * 사용처: 공개 경로 `/rss.xml`. 목록 페이지와 초안은 항목이 아니다.
 */
import { buildRss } from '../../../../packages/seo/meta.mjs';
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
  const result = buildRss(index.records, siteOrigin) as { xml: string };
  return new Response(result.xml, {
    headers: { 'Content-Type': 'application/xml; charset=utf-8' },
  });
}
