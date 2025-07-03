import React from 'react'
import { render, RenderOptions } from '@testing-library/react'
import { BrowserRouter } from 'react-router-dom'
import { vi } from 'vitest'

interface User {
  id: string
  username: string
  email: string
  role?: string
}

interface MockAuthContextType {
  user: User | null
  loading: boolean
  login: (username: string, password: string) => Promise<void>
  register: (username: string, email: string, password: string) => Promise<void>
  logout: () => void
}

// Test data factories for consistent mock data across tests
export const createMockUser = (overrides: Partial<User> = {}): User => ({
  id: '1',
  username: 'testuser',
  email: 'test@example.com',
  role: 'user',
  ...overrides
})

export const createMockAdminUser = (overrides: Partial<User> = {}): User => ({
  id: '2',
  username: 'adminuser',
  email: 'admin@example.com',
  role: 'admin',
  ...overrides
})

// Centralized API mocking to eliminate per-file duplication
export const createMockApiServices = () => {
  const mockDocumentService = {
    enhancedSearch: vi.fn().mockResolvedValue({ documents: [], total: 0 }),
    bulkRetryOcr: vi.fn().mockResolvedValue({ success: true }),
    getDocument: vi.fn().mockResolvedValue({}),
    uploadDocument: vi.fn().mockResolvedValue({}),
    deleteDocument: vi.fn().mockResolvedValue({}),
    updateDocument: vi.fn().mockResolvedValue({}),
  }

  const mockAuthService = {
    login: vi.fn().mockResolvedValue({ token: 'mock-token', user: createMockUser() }),
    register: vi.fn().mockResolvedValue({ token: 'mock-token', user: createMockUser() }),
    logout: vi.fn().mockResolvedValue({}),
    getCurrentUser: vi.fn().mockResolvedValue(createMockUser()),
  }

  const mockSourceService = {
    getSources: vi.fn().mockResolvedValue([]),
    createSource: vi.fn().mockResolvedValue({}),
    updateSource: vi.fn().mockResolvedValue({}),
    deleteSource: vi.fn().mockResolvedValue({}),
    syncSource: vi.fn().mockResolvedValue({}),
  }

  const mockLabelService = {
    getLabels: vi.fn().mockResolvedValue([]),
    createLabel: vi.fn().mockResolvedValue({}),
    updateLabel: vi.fn().mockResolvedValue({}),
    deleteLabel: vi.fn().mockResolvedValue({}),
  }

  return {
    documentService: mockDocumentService,
    authService: mockAuthService,
    sourceService: mockSourceService,
    labelService: mockLabelService,
  }
}

// Setup global API mocks (call this in setup files)
export const setupApiMocks = () => {
  const mockServices = createMockApiServices()
  
  vi.mock('../../services/api', () => ({
    documentService: mockServices.documentService,
    authService: mockServices.authService,
    sourceService: mockServices.sourceService,
    labelService: mockServices.labelService,
    api: {
      defaults: {
        headers: {
          common: {}
        }
      }
    }
  }))

  return mockServices
}

// Create a mock AuthProvider for testing
export const MockAuthProvider = ({ 
  children, 
  mockValues = {} 
}: { 
  children: React.ReactNode
  mockValues?: Partial<MockAuthContextType>
}) => {
  const defaultMocks = {
    user: null,
    loading: false,
    login: vi.fn(),
    register: vi.fn(),
    logout: vi.fn(),
    ...mockValues
  }

  // Mock the useAuth hook
  const AuthContext = React.createContext(defaultMocks)
  
  return (
    <AuthContext.Provider value={defaultMocks}>
      {children}
    </AuthContext.Provider>
  )
}

// Enhanced provider wrapper with theme and notification contexts
const AllTheProviders = ({ 
  children, 
  authValues,
  routerProps = {}
}: { 
  children: React.ReactNode
  authValues?: Partial<MockAuthContextType>
  routerProps?: any
}) => {
  return (
    <BrowserRouter {...routerProps}>
      <MockAuthProvider mockValues={authValues}>
        {children}
      </MockAuthProvider>
    </BrowserRouter>
  )
}

// Enhanced render functions with better provider configuration
export const renderWithProviders = (
  ui: React.ReactElement,
  options?: Omit<RenderOptions, 'wrapper'> & {
    authValues?: Partial<MockAuthContextType>
    routerProps?: any
  }
) => {
  const { authValues, routerProps, ...renderOptions } = options || {}
  
  const Wrapper = ({ children }: { children: React.ReactNode }) => (
    <AllTheProviders authValues={authValues} routerProps={routerProps}>
      {children}
    </AllTheProviders>
  )
  
  return render(ui, { wrapper: Wrapper, ...renderOptions })
}

export const renderWithMockAuth = (
  ui: React.ReactElement,
  mockAuthValues?: Partial<MockAuthContextType>,
  options?: Omit<RenderOptions, 'wrapper'>
) => {
  return renderWithProviders(ui, { ...options, authValues: mockAuthValues })
}

// Render with authenticated user (commonly used pattern)
export const renderWithAuthenticatedUser = (
  ui: React.ReactElement,
  user: User = createMockUser(),
  options?: Omit<RenderOptions, 'wrapper'>
) => {
  return renderWithProviders(ui, {
    ...options,
    authValues: {
      user,
      loading: false,
      login: vi.fn(),
      register: vi.fn(),
      logout: vi.fn(),
    }
  })
}

// Render with admin user (commonly used pattern)
export const renderWithAdminUser = (
  ui: React.ReactElement,
  options?: Omit<RenderOptions, 'wrapper'>
) => {
  return renderWithAuthenticatedUser(ui, createMockAdminUser(), options)
}

// Mock localStorage consistently across tests
export const createMockLocalStorage = () => {
  const storage: Record<string, string> = {}
  
  return {
    getItem: vi.fn((key: string) => storage[key] || null),
    setItem: vi.fn((key: string, value: string) => { storage[key] = value }),
    removeItem: vi.fn((key: string) => { delete storage[key] }),
    clear: vi.fn(() => Object.keys(storage).forEach(key => delete storage[key])),
    key: vi.fn((index: number) => Object.keys(storage)[index] || null),
    length: Object.keys(storage).length,
  }
}

// Setup function to be called in test setup files
export const setupTestEnvironment = () => {
  // Mock localStorage
  Object.defineProperty(window, 'localStorage', {
    value: createMockLocalStorage(),
    writable: true,
  })

  // Mock sessionStorage
  Object.defineProperty(window, 'sessionStorage', {
    value: createMockLocalStorage(),
    writable: true,
  })

  // Mock window.matchMedia
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    value: vi.fn().mockImplementation(query => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  })

  return setupApiMocks()
}

// re-export everything
export * from '@testing-library/react'