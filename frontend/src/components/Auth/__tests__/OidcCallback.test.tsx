import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
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
    window.location.href = '';
    // Clear API mocks
    (api.get as any).mockClear();
    // Reset API mocks to default implementation
    (api.get as any).mockResolvedValue({ data: { token: 'default-token' } });
    
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

  const renderOidcCallback = (search = '') => {
    return render(
      <MemoryRouter initialEntries={[`/auth/oidc/callback${search}`]}>
        <MockAuthProvider>
          <Routes>
            <Route path="/auth/oidc/callback" element={<OidcCallback />} />
            <Route path="/login" element={<div>Login Page</div>} />
          </Routes>
        </MockAuthProvider>
      </MemoryRouter>
    );
  };

  it('shows loading state initially', async () => {
    // Mock the API call to delay so we can see the loading state
    (api.get as any).mockImplementation(() => new Promise(() => {})); // Never resolves
    
    renderOidcCallback('?code=test-code&state=test-state');
    
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

    renderOidcCallback('?code=test-code&state=test-state');

    await waitFor(() => {
      expect(api.get).toHaveBeenCalledWith('/auth/oidc/callback?code=test-code&state=test-state');
    });

    expect(localStorage.setItem).toHaveBeenCalledWith('token', 'test-jwt-token');
    expect(window.location.href).toBe('/dashboard');
  });

  it('handles authentication error from URL params', () => {
    renderOidcCallback('?error=access_denied&error_description=User+denied+access');

    expect(screen.getByText('Authentication Error')).toBeInTheDocument();
    expect(screen.getByText('Authentication failed: access_denied')).toBeInTheDocument();
  });

  it('handles missing authorization code', () => {
    renderOidcCallback('');

    expect(screen.getByText('Authentication Error')).toBeInTheDocument();
    expect(screen.getByText('No authorization code received')).toBeInTheDocument();
  });

  it('handles API error during callback', async () => {
    const error = {
      response: {
        data: {
          error: 'Invalid authorization code'
        }
      }
    };
    (api.get as any).mockRejectedValueOnce(error);

    renderOidcCallback('?code=test-code&state=test-state');

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

    renderOidcCallback('?code=test-code&state=test-state');

    await waitFor(() => {
      expect(screen.getByText('Authentication Error')).toBeInTheDocument();
      expect(screen.getByText('Invalid response from authentication server')).toBeInTheDocument();
    });
  });

  it('provides return to login button on error', async () => {
    (api.get as any).mockRejectedValueOnce(new Error('Network error'));

    renderOidcCallback('?code=test-code&state=test-state');

    await waitFor(() => {
      expect(screen.getByText('Return to Login')).toBeInTheDocument();
    });

    // Test clicking return to login
    const returnButton = screen.getByText('Return to Login');
    fireEvent.click(returnButton);
    
    // Check if navigation to login page occurred by looking for login page content
    await waitFor(() => {
      expect(screen.getByText('Login Page')).toBeInTheDocument();
    });
  });
});