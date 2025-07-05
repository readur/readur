import { vi } from 'vitest';
import React from 'react';

// Create stable mock functions
const mockLogin = vi.fn().mockResolvedValue({});
const mockRegister = vi.fn().mockResolvedValue({});
const mockLogout = vi.fn();

// Mock the auth context module completely
vi.mock('../../../contexts/AuthContext', () => ({
  useAuth: vi.fn(() => ({
    user: null,
    loading: false,
    login: mockLogin,
    register: mockRegister,
    logout: mockLogout,
  })),
  AuthProvider: ({ children }: { children: React.ReactNode }) => React.createElement('div', null, children),
}));

// Mock axios comprehensively to prevent any real HTTP requests
import { createComprehensiveAxiosMock, createComprehensiveApiMocks } from '../../../test/comprehensive-mocks';

vi.mock('axios', () => createComprehensiveAxiosMock());

// Create the mock API object
const mockApi = {
  get: vi.fn().mockResolvedValue({ data: { token: 'default-token' } }),
  post: vi.fn().mockResolvedValue({ data: { success: true } }),
  put: vi.fn().mockResolvedValue({ data: { success: true } }),
  delete: vi.fn().mockResolvedValue({ data: { success: true } }),
  patch: vi.fn().mockResolvedValue({ data: { success: true } }),
  defaults: {
    headers: {
      common: {}
    }
  }
};

// Mock the services/api file
vi.mock('../../../services/api', () => ({
  api: mockApi,
  default: mockApi,
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

// Now import after mocks
import { screen, waitFor, fireEvent } from '@testing-library/react';
import { renderWithProviders } from '../../../test/test-utils';
import OidcCallback from '../OidcCallback';

// Mock window.location
Object.defineProperty(window, 'location', {
  value: {
    href: ''
  },
  writable: true
});

describe('OidcCallback', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.resetModules();
    window.location.href = '';
    // Clear API mocks
    mockApi.get.mockClear();
    // Reset API mocks to default implementation
    mockApi.get.mockResolvedValue({ data: { token: 'default-token' } });
  });

  const renderOidcCallback = (search = '') => {
    // Mock the URL search params for the component
    const url = new URL(`http://localhost/auth/oidc/callback${search}`);
    Object.defineProperty(window, 'location', {
      value: { search: url.search },
      writable: true
    });
    
    // Use renderWithProviders to get auth context
    return renderWithProviders(<OidcCallback />);
  };

  it('shows loading state initially', async () => {
    // Mock the API call to delay so we can see the loading state
    mockApi.get.mockImplementation(() => new Promise(() => {})); // Never resolves
    
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

    mockApi.get.mockResolvedValueOnce(mockResponse);

    renderOidcCallback('?code=test-code&state=test-state');

    await waitFor(() => {
      expect(mockApi.get).toHaveBeenCalledWith('/auth/oidc/callback?code=test-code&state=test-state');
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
    mockApi.get.mockRejectedValueOnce(error);

    renderOidcCallback('?code=test-code&state=test-state');

    await waitFor(() => {
      expect(screen.getByText('Authentication Error')).toBeInTheDocument();
      expect(screen.getByText('Invalid authorization code')).toBeInTheDocument();
    });
  });

  it('handles invalid response from server', async () => {
    mockApi.get.mockResolvedValueOnce({
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
    mockApi.get.mockRejectedValueOnce(new Error('Network error'));

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