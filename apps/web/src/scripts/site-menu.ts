/**
 * 개념: 좁은 화면에서만 접히는 사이트 메뉴.
 * 목적: Escape로 메뉴를 닫고 포커스를 메뉴 버튼에 되돌린다.
 * 사용처: apps/web/src/layouts/Base.astro 의 details#site-menu
 * 넓은 화면에서는 메뉴가 항상 보이므로 포커스를 옮기지 않는다.
 */
const narrowMenu = window.matchMedia('(max-width: 47.99rem)')

export function setupSiteMenu(menu: Element | null): void {
  if (!(menu instanceof HTMLDetailsElement)) return
  const summary = menu.querySelector('summary')
  const firstLink = menu.querySelector<HTMLAnchorElement>('.site-nav a')
  if (!summary || !firstLink) return

  menu.addEventListener('toggle', () => {
    if (!narrowMenu.matches || !menu.open) return
    firstLink.focus()
  })

  document.addEventListener('keydown', (event) => {
    if (event.key !== 'Escape' || !narrowMenu.matches || !menu.open) return
    menu.open = false
    summary.focus()
  })
}
