import { render, screen, waitFor } from '@testing-library/react';
import { BrowserRouter } from 'react-router-dom';
import { vi } from 'vitest';
import OidcCallback from '../OidcCallback';
import { AuthProvider } from '../../../contexts/AuthContext';
import { api } from '../../../services/api';

// Mock the API
vi.mock('../../../services/api', () => ({
  api: {
    get: vi.fn(),
    defaults: {
      headers: {
        common: {}
      }
    }
  }
}));

// Mock useNavigate and useSearchParams
const mockNavigate = vi.fn();
const mockUseSearchParams = vi.fn(() => [new URLSearchParams('code=test-code&state=test-state')]);

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom');
  return {
    ...actual,
    useNavigate: () => mockNavigate,
    useSearchParams: mockUseSearchParams
  };
});

// Mock localStorage
Object.defineProperty(window, 'localStorage', {
  value: {
    setItem: vi.fn(),
    getItem: vi.fn(),
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

describe('OidcCallback', () => {
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

  const renderOidcCallback = () => {
    return render(
      <BrowserRouter>
        <MockAuthProvider>
          <OidcCallback />
        </MockAuthProvider>
      </BrowserRouter>
    );
  };

  it('shows loading state initially', () => {
    renderOidcCallback();
    
    expect(screen.getByText('Completing Authentication')).toBeInTheDocument();
    expect(screen.getByText('Please wait while we process your authentication...')).toBeInTheDocument();
  });

  it('handles successful authentication', async () => {
    const mockResponse = {
      data: {
        token: 'test-jwt-token',
        user: {
          id: '123',
          username: 'testuser',
          email: 'test@example.com'
        }
      }
    };

    (api.get as any).mockResolvedValueOnce(mockResponse);

    renderOidcCallback();

    await waitFor(() => {
      expect(api.get).toHaveBeenCalledWith('/auth/oidc/callback?code=test-code&state=test-state');
    });

    expect(localStorage.setItem).toHaveBeenCalledWith('token', 'test-jwt-token');
    expect(window.location.href).toBe('/dashboard');
  });

  it('handles authentication error from URL params', () => {
    // Mock useSearchParams to return error
    mockUseSearchParams.mockReturnValueOnce([
      new URLSearchParams('error=access_denied&error_description=User+denied+access')
    ]);

    renderOidcCallback();

    expect(screen.getByText('Authentication Error')).toBeInTheDocument();
    expect(screen.getByText('Authentication failed: access_denied')).toBeInTheDocument();
  });

  it('handles missing authorization code', () => {
    // Mock useSearchParams to return no code
    mockUseSearchParams.mockReturnValueOnce([
      new URLSearchParams('')
    ]);

    renderOidcCallback();

    expect(screen.getByText('Authentication Error')).toBeInTheDocument();
    expect(screen.getByText('No authorization code received')).toBeInTheDocument();
  });

  it('handles API error during callback', async () => {
    (api.get as any).mockRejectedValueOnce({
      response: {
        data: {
          error: 'Invalid authorization code'
        }
      }
    });

    renderOidcCallback();

    await waitFor(() => {
      expect(screen.getByText('Authentication Error')).toBeInTheDocument();
      expect(screen.getByText('Invalid authorization code')).toBeInTheDocument();
    });
  });

  it('handles invalid response from server', async () => {
    (api.get as any).mockResolvedValueOnce({
      data: {
        // Missing token
        user: { id: '123' }
      }
    });

    renderOidcCallback();

    await waitFor(() => {
      expect(screen.getByText('Authentication Error')).toBeInTheDocument();
      expect(screen.getByText('Invalid response from authentication server')).toBeInTheDocument();
    });
  });

  it('provides return to login button on error', async () => {
    (api.get as any).mockRejectedValueOnce(new Error('Network error'));

    renderOidcCallback();

    await waitFor(() => {
      expect(screen.getByText('Return to Login')).toBeInTheDocument();
    });

    // Test clicking return to login
    screen.getByText('Return to Login').click();
    expect(mockNavigate).toHaveBeenCalledWith('/login');
  });
});