declare module '../../../scripts/public-examples.mjs' {
  export function loadPublicHome(contentRoot?: string): import('./site/home').HomeData
}

declare module '../../../scripts/public-projects.mjs' {
  export function loadPublicProjects(contentRoot?: string): import('./scripts/project-types').ProjectSite
}

declare module '../../../scripts/public-research.mjs' {
  export function loadPublicResearch(contentRoot?: string): import('./scripts/research-types').ResearchSite
}

declare module '../../../scripts/public-taxonomy.mjs' {
  export function loadPublicTaxonomy(contentRoot?: string): import('./scripts/taxonomy-types').TaxonomySite
}

declare module '../../../scripts/public-index.mjs' {
  export function loadPublicIndex(contentRoot?: string): {
    records: {
      path: string
      title?: string
      kind?: string
      updatedAt?: string
      draft?: boolean
    }[]
  }
}

declare module '../../../packages/seo/meta.mjs' {
  export function buildMeta(input: {
    title: string
    description?: string
    path?: string
    date?: string
    siteOrigin?: string
  }): {
    title: string
    description: string
    canonical: string
    author: string
    date: string
    og: { title: string; description: string; url: string; type: string }
  }
  export function buildSitemap(
    records: { path: string; title?: string; kind?: string; updatedAt?: string; draft?: boolean }[],
    origin: string,
  ): { xml: string }
  export function buildRss(
    records: { path: string; title?: string; kind?: string; updatedAt?: string; draft?: boolean }[],
    origin: string,
  ): { xml: string }
}

declare module '../../../packages/seo/site-origin.mjs' {
  export const siteOrigin: string
}
