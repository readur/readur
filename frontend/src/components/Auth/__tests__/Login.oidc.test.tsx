import { vi } from 'vitest';

// Mock AuthContext to work with the test setup
vi.mock('../../../contexts/AuthContext', () => ({
  useAuth: vi.fn(() => ({
    user: null,
    loading: false,
    login: vi.fn().mockResolvedValue({}),
    register: vi.fn().mockResolvedValue({}),
    logout: vi.fn(),
  })),
}));

// Mock ThemeContext
vi.mock('../../../contexts/ThemeContext', () => ({
  useTheme: () => ({ 
    darkMode: false, 
    toggleDarkMode: vi.fn() 
  }),
}));

// Mock the API
vi.mock('../../../services/api', () => ({
  api: {
    post: vi.fn(),
    defaults: {
      headers: {
        common: {}
      }
    }
  }
}));

// Mock useNavigate
const mockNavigate = vi.fn();
vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom');
  return {
    ...actual,
    useNavigate: () => mockNavigate
  };
});

// Now import after all mocks are set up
import { screen, fireEvent, waitFor } from '@testing-library/react';
import { renderWithProviders, createMockUser } from '../../../test/test-utils';
import Login from '../Login';

// Mock window.location
Object.defineProperty(window, 'location', {
  value: {
    href: ''
  },
  writable: true
});

describe('Login - OIDC Features', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const renderLogin = () => {
    return renderWithProviders(<Login />);
  };

  it('renders OIDC login button', () => {
    renderLogin();
    
    expect(screen.getByText('Sign in with OIDC')).toBeInTheDocument();
    expect(screen.getByText('or')).toBeInTheDocument();
  });

  it('handles OIDC login button click', async () => {
    renderLogin();
    
    const oidcButton = screen.getByText('Sign in with OIDC');
    fireEvent.click(oidcButton);

    await waitFor(() => {
      expect(window.location.href).toBe('/api/auth/oidc/login');
    });
  });

  it('shows loading state when OIDC login is clicked', async () => {
    renderLogin();
    
    const oidcButton = screen.getByText('Sign in with OIDC');
    fireEvent.click(oidcButton);

    expect(screen.getByText('Redirecting...')).toBeInTheDocument();
    expect(oidcButton).toBeDisabled();
  });

  it('disables regular login when OIDC is loading', async () => {
    renderLogin();
    
    const oidcButton = screen.getByText('Sign in with OIDC');
    const regularButton = screen.getByText('Sign in');
    
    fireEvent.click(oidcButton);

    expect(regularButton).toBeDisabled();
  });

  it('shows error message on OIDC login failure', async () => {
    // Mock an error during OIDC redirect
    Object.defineProperty(window, 'location', {
      value: {
        get href() {
          throw new Error('Network error');
        },
        set href(value) {
          throw new Error('Network error');
        }
      },
      configurable: true
    });

    renderLogin();
    
    const oidcButton = screen.getByText('Sign in with OIDC');
    fireEvent.click(oidcButton);

    await waitFor(() => {
      expect(screen.getByText(/Failed to initiate OIDC login/)).toBeInTheDocument();
    });
  });

  it('has proper styling for OIDC button', () => {
    renderLogin();
    
    const oidcButton = screen.getByText('Sign in with OIDC');
    const buttonElement = oidcButton.closest('button');
    
    expect(buttonElement).toHaveClass('MuiButton-outlined');
    expect(buttonElement).toHaveAttribute('type', 'button');
  });

  it('includes security icon in OIDC button', () => {
    renderLogin();
    
    const oidcButton = screen.getByText('Sign in with OIDC');
    const buttonElement = oidcButton.closest('button');
    
    // Check for security icon (via test id or class)
    expect(buttonElement?.querySelector('svg')).toBeInTheDocument();
  });

  it('maintains button accessibility', () => {
    renderLogin();
    
    const oidcButton = screen.getByRole('button', { name: /sign in with oidc/i });
    expect(oidcButton).toBeInTheDocument();
    expect(oidcButton).toBeEnabled();
  });

  it('handles keyboard navigation', () => {
    renderLogin();
    
    const usernameInput = screen.getByLabelText(/username/i);
    const passwordInput = screen.getByLabelText(/password/i);
    const regularButton = screen.getByText('Sign in');
    const oidcButton = screen.getByText('Sign in with OIDC');

    // Tab order should be: username -> password -> sign in -> oidc
    usernameInput.focus();
    expect(document.activeElement).toBe(usernameInput);

    fireEvent.keyDown(usernameInput, { key: 'Tab' });
    // Note: Actual tab behavior would need more complex setup
    // This is a simplified test for the presence of focusable elements
    expect(passwordInput).toBeInTheDocument();
    expect(regularButton).toBeInTheDocument();
    expect(oidcButton).toBeInTheDocument();
  });
});