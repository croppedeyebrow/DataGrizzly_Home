/**
 * 개념: 노트와 리서치 페이지가 읽는 글 값.
 * 목적: 목록과 상세가 같은 블록 종류를 쓰게 한다.
 * 사용처: apps/web/src/pages/notes, apps/web/src/pages/research, apps/web/src/components/BlockList.astro
 */
export type TextPart = { kind: 'text'; text: string } | { kind: 'link'; href: string };

export type ViewBlock = {
  kind: string;
  parts?: TextPart[];
  level?: number;
  text?: string;
  anchor?: string;
  headingPath?: string[];
  ordered?: boolean;
  items?: string[];
  citation?: string;
  caption?: string;
  header?: string[];
  rows?: string[][];
  alt?: string;
  imageName?: string;
  missing?: boolean;
  language?: string;
  filename?: string | null;
  state?: string;
  tex?: string;
  message?: string;
  tone?: string;
  href?: string | null;
  title?: string;
};

export type PublicArticle = {
  slug: string;
  title: string;
  kind: 'note' | 'paper' | 'news';
  createdAt: string;
  updatedAt: string;
  projectId: string | null;
  summary: string;
  source: string;
  claim: string;
  interpretation: string;
  reproduction: string;
  eventDate: string;
  publicationDate: string;
  blocks: ViewBlock[];
};

export type ResearchSite = {
  loadError: boolean;
  notes: PublicArticle[];
  research: PublicArticle[];
};
