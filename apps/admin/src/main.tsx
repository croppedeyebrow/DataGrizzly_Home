/**
 * 개념: 관리자 화면의 진입점.
 * 목적: 인증 자리표시자를 루트에 붙인다. API는 호출하지 않는다.
 * 사용처: apps/admin/index.html.
 */
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import App from './App.tsx'

const root = document.getElementById('root')
if (root) {
  createRoot(root).render(
    <StrictMode>
      <App />
    </StrictMode>,
  )
}
