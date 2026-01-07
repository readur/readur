import React, { Suspense } from 'react'
import ReactDOM from 'react-dom/client'
import { BrowserRouter } from 'react-router-dom'
import App from './App'
import './index.css'
import { AuthProvider } from './contexts/AuthContext'
import { FeatureFlagsProvider } from './contexts/FeatureFlagsContext'
import './i18n/config'

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <Suspense fallback={<div>Loading...</div>}>
      <BrowserRouter>
        <AuthProvider>
          <FeatureFlagsProvider>
            <App />
          </FeatureFlagsProvider>
        </AuthProvider>
      </BrowserRouter>
    </Suspense>
  </React.StrictMode>,
)