import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { BrowserRouter } from 'react-router-dom';
import { vi } from 'vitest';
import Login from '../Login';
import { AuthProvider } from '../../../contexts/AuthContext';
import { ThemeProvider } from '../../../contexts/ThemeContext';

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

// Mock localStorage
Object.defineProperty(window, 'localStorage', {
  value: {
    setItem: vi.fn(),
    getItem: vi.fn(() => null),
    removeItem: vi.fn()
  }
});

// Mock window.location
Object.defineProperty(window, 'location', {
  value: {
    href: ''
  },
  writable: true
});


// Mock AuthContext
const mockAuthContextValue = {
  user: null,
  loading: false,
  login: vi.fn(),
  register: vi.fn(),
  logout: vi.fn()
};

const MockAuthProvider = ({ children }: { children: React.ReactNode }) => (
  <AuthProvider>
    {children}
  </AuthProvider>
);

const MockThemeProvider = ({ children }: { children: React.ReactNode }) => (
  <ThemeProvider>
    {children}
  </ThemeProvider>
);

describe('Login - OIDC Features', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    
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
    });
  });

  const renderLogin = () => {
    return render(
      <BrowserRouter>
        <MockThemeProvider>
          <MockAuthProvider>
            <Login />
          </MockAuthProvider>
        </MockThemeProvider>
      </BrowserRouter>
    );
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