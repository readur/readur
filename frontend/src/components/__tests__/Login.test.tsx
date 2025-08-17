import { render, screen } from '@testing-library/react'
import { vi, describe, it, expect, beforeEach } from 'vitest'
import Login from '../Login'

const mockLogin = vi.fn()

// Mock the AuthContext with a simple mock
vi.mock('../../contexts/AuthContext', () => ({
  useAuth: () => ({
    login: mockLogin,
    logout: vi.fn(),
    register: vi.fn(),
    user: null,
    loading: false,
  }),
}))

// Mock react-router-dom
vi.mock('react-router-dom', () => ({
  Link: ({ to, children }: { to: string; children: React.ReactNode }) => (
    <a href={to}>{children}</a>
  ),
}))

describe('Login', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders login form elements', () => {
    render(<Login />)

    expect(screen.getByText('Sign in to Readur')).toBeInTheDocument()
    expect(screen.getByPlaceholderText('Username')).toBeInTheDocument()
    expect(screen.getByPlaceholderText('Password')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Sign in' })).toBeInTheDocument()
  })

  it('renders signup link', () => {
    render(<Login />)
    
    expect(screen.getByText("Don't have an account? Sign up")).toBeInTheDocument()
  })
})