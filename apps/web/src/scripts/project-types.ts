/**
 * 개념: 프로젝트 페이지가 mjs 선택 결과에서 읽는 값.
 * 목적: 목록, 설명, 과정 페이지가 같은 필드 이름을 쓰게 한다.
 * 사용처: apps/web/src/pages/projects, apps/web/src/pages/logs
 */
export type RelatedLink = {
  title: string;
  href: string | null;
  kind: string;
};

export type LogLink = {
  slug: string;
  title: string;
};

export type PublicLog = {
  slug: string;
  title: string;
  entryId: string;
  displayOrder: number;
  createdAt: string;
  stage: string;
  paragraphs: string[];
  callout: string;
  projectSlug: string;
  projectTitle: string;
  related: RelatedLink[];
  previous: LogLink | null;
  next: LogLink | null;
};

export type PublicProject = {
  slug: string;
  title: string;
  summary: string;
  status: string;
  updatedAt: string;
  callout: string;
  problem: string;
  goal: string;
  scope: string;
  choice: string;
  judgment: string;
  result: string;
  limit: string;
  imageAlt: string;
  imageName: string;
  topic: null;
  github: null;
  logs: PublicLog[];
  related: RelatedLink[];
};

export type ProjectSite = {
  loadError: boolean;
  projects: PublicProject[];
  logs: PublicLog[];
};
