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
