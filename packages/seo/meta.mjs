/**
 * 개념: 공개 메타와 수집 문서.
 * 목적: 제목, 설명, canonical, OG, 작성자, 날짜, sitemap, RSS를 한 경로에서 만든다.
 * 사용처: apps/web/src/layouts/Base.astro, apps/web/src/pages/sitemap.xml.ts, apps/web/src/pages/rss.xml.ts, scripts/check-m1-seo.mjs
 * 사이트 주소가 비어 있거나 localhost, example.com이면 절대 URL을 만들지 않는다.
 */

const LOCAL_HOSTS = new Set(['localhost', '127.0.0.1', '0.0.0.0', '::1'])
const DATE = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/
const WEEKDAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat']
const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec']

export function resolveSiteOrigin(value) {
  if (value == null || String(value).trim() === '') return { ok: false, reason: 'unset', origin: '' }
  let url
  try {
    url = new URL(String(value).trim())
  } catch {
    return { ok: false, reason: 'invalid', origin: '' }
  }
  const host = url.hostname.replace(/^\[|\]$/g, '').toLowerCase()
  if (url.username || url.password || url.search || url.hash || (url.pathname !== '/' && url.pathname !== '')) {
    return { ok: false, reason: 'invalid', origin: '' }
  }
  if (url.protocol !== 'https:') return { ok: false, reason: 'not-https', origin: '' }
  if (LOCAL_HOSTS.has(host) || host.endsWith('.localhost')) return { ok: false, reason: 'localhost', origin: '' }
  if (host === 'example.com' || host.endsWith('.example.com')) return { ok: false, reason: 'placeholder', origin: '' }
  return { ok: true, reason: 'explicit', origin: url.origin }
}

export function normalizeDescription(value) {
  const text = String(value ?? '').replace(/\s+/g, ' ').trim()
  if (text.length <= 180) return text
  return `${text.slice(0, 177).trimEnd()}...`
}

export function buildMeta({ title, description, path, date, siteOrigin }) {
  const origin = resolveSiteOrigin(siteOrigin)
  const safePath = typeof path === 'string' && path.startsWith('/') && !path.includes('://') && !path.includes('?') ? path : ''
  const canonical = safePath ? (origin.ok ? `${origin.origin}${safePath}` : safePath) : ''
  const safeDate = typeof date === 'string' && DATE.test(date) ? date : ''
  const safeTitle = String(title || 'DataGrizzly').replace(/\s+/g, ' ').trim() || 'DataGrizzly'
  const safeDescription = normalizeDescription(description)
  return {
    title: safeTitle,
    description: safeDescription,
    canonical,
    canonicalIsAbsolute: Boolean(safePath && origin.ok),
    originReason: origin.reason,
    author: 'DataGrizzly',
    date: safeDate,
    og: {
      title: safeTitle,
      description: safeDescription,
      url: canonical,
      type: safeDate ? 'article' : 'website',
    },
  }
}

function pathReason(record) {
  if (record?.draft === true) return 'draft'
  const route = record?.path
  if (typeof route !== 'string' || !route.startsWith('/') || route.startsWith('//')) return 'invalid'
  if (route.includes('?') || route.includes('#') || route.includes('://') || route.includes('\\') || route.includes(' ')) return 'invalid'
  if (
    route === '/404' ||
    route.startsWith('/404/') ||
    route.startsWith('/admin') ||
    route.startsWith('/lab') ||
    route.startsWith('/labs') ||
    route.startsWith('/experiments') ||
    route.startsWith('/signup') ||
    route.startsWith('/register') ||
    route.startsWith('/api/')
  ) {
    return 'excluded'
  }
  return ''
}

export function uniquePublicPaths(records) {
  const urls = []
  const rejected = []
  const seen = new Set()
  for (const record of records ?? []) {
    const reason = pathReason(record)
    if (reason) {
      rejected.push({ path: record?.path ?? '', reason })
      continue
    }
    if (seen.has(record.path)) {
      rejected.push({ path: record.path, reason: 'duplicate' })
      continue
    }
    seen.add(record.path)
    urls.push({
      path: record.path,
      title: String(record.title || 'DataGrizzly'),
      kind: record.kind === 'entry' ? 'entry' : 'page',
      updatedAt: typeof record.updatedAt === 'string' ? record.updatedAt : '',
    })
  }
  return { urls, rejected }
}

export function escapeXml(value) {
  return String(value)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
}

export function rfc822(iso) {
  if (!DATE.test(iso ?? '')) return ''
  const date = new Date(iso)
  if (Number.isNaN(date.getTime())) return ''
  const [, year, month, day, hour, minute, second] = DATE.exec(iso)
  return `${WEEKDAYS[date.getUTCDay()]}, ${day} ${MONTHS[Number(month) - 1]} ${year} ${hour}:${minute}:${second} GMT`
}

export function buildSitemap(records, siteOrigin) {
  const origin = resolveSiteOrigin(siteOrigin)
  const unique = uniquePublicPaths(records)
  const locs = origin.ok ? unique.urls.map((item) => `${origin.origin}${item.path}`) : []
  const body = locs.map((loc) => `  <url><loc>${escapeXml(loc)}</loc></url>`).join('\n')
  const note = origin.ok ? '' : '<!-- absolute URLs omitted: site origin is not a public https origin -->\n'
  const xml = `<?xml version="1.0" encoding="UTF-8"?>\n${note}<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${body}${body ? '\n' : ''}</urlset>\n`
  return { xml, absolute: origin.ok, originReason: origin.reason, urls: unique.urls, rejected: unique.rejected }
}

export function buildRss(records, siteOrigin) {
  const origin = resolveSiteOrigin(siteOrigin)
  const unique = uniquePublicPaths(records)
  const entries = origin.ok ? unique.urls.filter((item) => item.kind === 'entry') : []
  const items = entries
    .map((item) => {
      const link = `${origin.origin}${item.path}`
      const published = rfc822(item.updatedAt)
      const date = published ? `\n      <pubDate>${escapeXml(published)}</pubDate>` : ''
      return `    <item>\n      <title>${escapeXml(item.title)}</title>\n      <link>${escapeXml(link)}</link>\n      <guid>${escapeXml(link)}</guid>${date}\n    </item>`
    })
    .join('\n')
  const channelLink = origin.ok ? `${origin.origin}/` : '/'
  const description = origin.ok ? '공개된 글의 주소만 담는다.' : '사이트 주소가 정해지지 않아 절대 링크를 넣지 않았다.'
  const xml = `<?xml version="1.0" encoding="UTF-8"?>\n<rss version="2.0">\n  <channel>\n    <title>DataGrizzly</title>\n    <link>${escapeXml(channelLink)}</link>\n    <description>${escapeXml(description)}</description>\n${items}${items ? '\n' : ''}  </channel>\n</rss>\n`
  return { xml, absolute: origin.ok, originReason: origin.reason, items: entries, rejected: unique.rejected }
}
