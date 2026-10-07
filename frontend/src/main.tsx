import React, { Suspense } from 'react'
import ReactDOM from 'react-dom/client'
import { BrowserRouter } from 'react-router-dom'
import { RacRouterBridge } from './app/RacRouterBridge'
import App from './App'
import '@fontsource-variable/plus-jakarta-sans'
import '@fontsource-variable/jetbrains-mono'
import './styles/tokens.css'
import './styles/base.css'
import { ThemeModeProvider } from './theme/ThemeProvider'
import { AuthProvider } from './contexts/AuthContext'
import { FeatureFlagsProvider } from './contexts/FeatureFlagsContext'
import './i18n/config'

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <Suspense fallback={<div>Loading...</div>}>
      <ThemeModeProvider>
      <BrowserRouter>
        <RacRouterBridge>
        <AuthProvider>
          <FeatureFlagsProvider>
            <App />
          </FeatureFlagsProvider>
        </AuthProvider>
        </RacRouterBridge>
      </BrowserRouter>
      </ThemeModeProvider>
    </Suspense>
  </React.StrictMode>,
)