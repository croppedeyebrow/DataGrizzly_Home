/**
 * 개념: 주제와 시리즈 페이지가 읽는 분류 값.
 * 목적: 목록 항목의 주소와 순서를 페이지가 직접 지어내지 않게 한다.
 * 사용처: apps/web/src/pages/topics/[slug].astro, apps/web/src/pages/series/[slug].astro
 */
export type TaxonomyEntry = {
  entryId: string;
  slug: string;
  title: string;
  path: string;
  position?: number;
};

export type TaxonomyPage = {
  kind: 'topic' | 'series';
  slug: string;
  title: string;
  entries: TaxonomyEntry[];
};

export type TaxonomySite = {
  loadError: boolean;
  topics: TaxonomyPage[];
  series: TaxonomyPage[];
};
