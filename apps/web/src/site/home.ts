/**
 * 개념: 홈이 fixture에서 읽는 값.
 * 목적: 대표 프로젝트, 최근 과정, 노트와 리서치 링크의 필드 이름을 고정한다.
 * 사용처: apps/web/src/site/pages.tsx, apps/web/src/external.d.ts
 */
export type HomeLog = {
  title: string
  slug: string
  stage: string
  createdAt: string
  displayOrder: number
}

export type HomeTopic = {
  title: string
  slug: string
  kind: 'note' | 'paper' | 'news'
}

export type HomeProject = {
  title: string
  slug: string
  summary: string
  imageAlt: string
  imageName: string
}

export type HomeData = {
  loadError: boolean
  project: HomeProject | null
  logs: HomeLog[]
  topics: HomeTopic[]
  contact: string | null
}
