export function loadPublicIndex(contentRoot?: string): {
  records: {
    path: string
    title?: string
    kind?: string
    updatedAt?: string
    draft?: boolean
  }[]
}
