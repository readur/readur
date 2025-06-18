import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { vi } from 'vitest'
import Login from '../Login'

const mockLogin = vi.fn()

// Mock the useAuth hook
vi.mock('../../contexts/AuthContext', () => ({
  useAuth: () => ({
    login: mockLogin,
    user: null,
    loading: false,
    logout: vi.fn(),
  }),
}))

// Mock react-router-dom
vi.mock('react-router-dom', () => ({
  Link: ({ to, children }: { to: string; children: React.ReactNode }) => (
    <a href={to}>{children}</a>
  ),
}))

const renderLogin = () => {
  return render(<Login />)
}

describe('Login', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  test('renders login form', () => {
    renderLogin()

    expect(screen.getByText('Sign in to Readur')).toBeInTheDocument()
    expect(screen.getByPlaceholderText('Username')).toBeInTheDocument()
    expect(screen.getByPlaceholderText('Password')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Sign in' })).toBeInTheDocument()
    expect(screen.getByText("Don't have an account? Sign up")).toBeInTheDocument()
  })

  test('handles form submission with valid credentials', async () => {
    mockLogin.mockResolvedValue(undefined)

    renderLogin()

    const usernameInput = screen.getByPlaceholderText('Username')
    const passwordInput = screen.getByPlaceholderText('Password')
    const submitButton = screen.getByRole('button', { name: 'Sign in' })

    fireEvent.change(usernameInput, { target: { value: 'testuser' } })
    fireEvent.change(passwordInput, { target: { value: 'password123' } })
    fireEvent.click(submitButton)

    await waitFor(() => {
      expect(mockLogin).toHaveBeenCalledWith('testuser', 'password123')
    })
  })

  test('displays error message on login failure', async () => {
    const errorMessage = 'Invalid credentials'
    mockLogin.mockRejectedValue({
      response: { data: { message: errorMessage } },
    })

    renderLogin()

    const usernameInput = screen.getByPlaceholderText('Username')
    const passwordInput = screen.getByPlaceholderText('Password')
    const submitButton = screen.getByRole('button', { name: 'Sign in' })

    fireEvent.change(usernameInput, { target: { value: 'testuser' } })
    fireEvent.change(passwordInput, { target: { value: 'wrongpassword' } })
    fireEvent.click(submitButton)

    await waitFor(() => {
      expect(screen.getByText(errorMessage)).toBeInTheDocument()
    })
  })

  test('shows loading state during submission', async () => {
    mockLogin.mockImplementation(() => new Promise(() => {})) // Never resolves

    renderLogin()

    const usernameInput = screen.getByPlaceholderText('Username')
    const passwordInput = screen.getByPlaceholderText('Password')
    const submitButton = screen.getByRole('button', { name: 'Sign in' })

    fireEvent.change(usernameInput, { target: { value: 'testuser' } })
    fireEvent.change(passwordInput, { target: { value: 'password123' } })
    fireEvent.click(submitButton)

    await waitFor(() => {
      expect(screen.getByText('Signing in...')).toBeInTheDocument()
      expect(submitButton).toBeDisabled()
    })
  })

  test('requires username and password', () => {
    renderLogin()

    const usernameInput = screen.getByPlaceholderText('Username')
    const passwordInput = screen.getByPlaceholderText('Password')

    expect(usernameInput).toBeRequired()
    expect(passwordInput).toBeRequired()
  })
})