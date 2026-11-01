/**
 * 개념: 브라우저에 남는 공개 웹 스크립트.
 * 목적: 좁은 화면 메뉴만 붙인다. 페이지 본문은 빌드된 HTML이다.
 * 사용처: 개발 서버와 dist/menu.js.
 */
import './styles.css'
import { setupSiteMenu } from './scripts/site-menu.ts'

setupSiteMenu(document.querySelector('#site-menu'))
