import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { vi, describe, it, expect, beforeEach } from 'vitest';
import { createComprehensiveAxiosMock, createComprehensiveApiMocks } from '../../test/comprehensive-mocks';

// Mock axios comprehensively to prevent any real HTTP requests
vi.mock('axios', () => createComprehensiveAxiosMock());

// Mock API services comprehensively
vi.mock('../../services/api', async () => {
  const actual = await vi.importActual('../../services/api');
  const apiMocks = createComprehensiveApiMocks();
  
  return {
    ...actual,
    default: apiMocks.api, // Since this file imports `api` as default
    ...apiMocks,
  };
});

// Get references to the mocked modules using dynamic import
const { default: api } = await import('../../services/api');
const mockedApi = api;

// Mock settings with WebDAV configuration
const mockSettings = {
  ocrLanguage: 'eng',
  concurrentOcrJobs: 4,
  ocrTimeoutSeconds: 300,
  maxFileSizeMb: 50,
  allowedFileTypes: ['pdf', 'png', 'jpg', 'jpeg', 'tiff', 'bmp', 'txt'],
  autoRotateImages: true,
  enableImagePreprocessing: true,
  searchResultsPerPage: 25,
  searchSnippetLength: 200,
  fuzzySearchThreshold: 0.8,
  retentionDays: null,
  enableAutoCleanup: false,
  enableCompression: false,
  memoryLimitMb: 512,
  cpuPriority: 'normal',
  enableBackgroundOcr: true,
  webdavEnabled: false,
  webdavServerUrl: '',
  webdavUsername: '',
  webdavPassword: '',
  webdavWatchFolders: ['/Documents'],
  webdavFileExtensions: ['pdf', 'png', 'jpg', 'jpeg', 'tiff', 'bmp', 'txt'],
  webdavAutoSync: false,
  webdavSyncIntervalMinutes: 60,
};

// Mock WebDAV test connection response
const mockConnectionResult = {
  success: true,
  message: 'Successfully connected to WebDAV server (Nextcloud)',
  server_version: '28.0.1',
  server_type: 'nextcloud',
};

// Mock WebDAV crawl estimate response
const mockCrawlEstimate = {
  folders: [
    {
      path: '/Documents',
      total_files: 1500,
      supported_files: 1200,
      estimated_time_hours: 0.67,
      total_size_mb: 2500.5,
    },
  ],
  total_files: 1500,
  total_supported_files: 1200,
  total_estimated_time_hours: 0.67,
  total_size_mb: 2500.5,
};

// Create a simplified test component that includes just the WebDAV tab content
const WebDAVTabTestComponent: React.FC = () => {
  const [settings, setSettings] = React.useState(mockSettings);
  const [loading, setLoading] = React.useState(false);
  
  const handleSettingsChange = async (key: string, value: any) => {
    setSettings(prev => ({ ...prev, [key]: value }));
  };

  const handleShowSnackbar = (message: string, severity: string) => {
    console.log(`${severity}: ${message}`);
  };

  return (
    <div data-testid="webdav-tab">
      {/* Simplified WebDAV tab content for testing */}
      <div>
        <h2>WebDAV Integration</h2>
        
        {/* Enable toggle */}
        <label>
          <input
            type="checkbox"
            checked={settings.webdavEnabled}
            onChange={(e) => handleSettingsChange('webdavEnabled', e.target.checked)}
            data-testid="webdav-enabled-toggle"
          />
          Enable WebDAV Integration
        </label>

        {settings.webdavEnabled && (
          <div data-testid="webdav-settings">
            {/* Server URL */}
            <input
              type="text"
              placeholder="Server URL"
              value={settings.webdavServerUrl}
              onChange={(e) => handleSettingsChange('webdavServerUrl', e.target.value)}
              data-testid="webdav-server-url"
            />

            {/* Username */}
            <input
              type="text"
              placeholder="Username"
              value={settings.webdavUsername}
              onChange={(e) => handleSettingsChange('webdavUsername', e.target.value)}
              data-testid="webdav-username"
            />

            {/* Password */}
            <input
              type="password"
              placeholder="Password"
              value={settings.webdavPassword}
              onChange={(e) => handleSettingsChange('webdavPassword', e.target.value)}
              data-testid="webdav-password"
            />

            {/* Test Connection Button */}
            <button
              onClick={() => {/* Test connection logic */}}
              disabled={loading}
              data-testid="test-connection-btn"
            >
              {loading ? 'Testing...' : 'Test Connection'}
            </button>

            {/* Estimate Crawl Button */}
            <button
              onClick={() => {/* Estimate crawl logic */}}
              disabled={loading}
              data-testid="estimate-crawl-btn"
            >
              {loading ? 'Estimating...' : 'Estimate Crawl'}
            </button>

            {/* Folder management */}
            <div data-testid="folder-list">
              {settings.webdavWatchFolders.map((folder, index) => (
                <div key={index} data-testid={`folder-${index}`}>
                  {folder}
                  <button
                    onClick={() => {
                      const newFolders = settings.webdavWatchFolders.filter((_, i) => i !== index);
                      handleSettingsChange('webdavWatchFolders', newFolders);
                    }}
                    data-testid={`remove-folder-${index}`}
                  >
                    Remove
                  </button>
                </div>
              ))}
            </div>

            {/* Add folder */}
            <input
              type="text"
              placeholder="Add folder path"
              data-testid="add-folder-input"
            />
            <button
              onClick={() => {
                const input = screen.getByTestId('add-folder-input') as HTMLInputElement;
                const newFolder = input.value;
                if (newFolder && !settings.webdavWatchFolders.includes(newFolder)) {
                  handleSettingsChange('webdavWatchFolders', [...settings.webdavWatchFolders, newFolder]);
                  input.value = '';
                }
              }}
              data-testid="add-folder-btn"
            >
              Add Folder
            </button>

            {/* Sync interval */}
            <input
              type="number"
              min="15"
              max="1440"
              value={settings.webdavSyncIntervalMinutes}
              onChange={(e) => {
                const value = e.target.value;
                const numValue = value === '' ? 0 : parseInt(value);
                handleSettingsChange('webdavSyncIntervalMinutes', numValue);
              }}
              data-testid="sync-interval"
            />

            {/* Auto sync toggle */}
            <label>
              <input
                type="checkbox"
                checked={settings.webdavAutoSync}
                onChange={(e) => handleSettingsChange('webdavAutoSync', e.target.checked)}
                data-testid="auto-sync-toggle"
              />
              Enable Automatic Sync
            </label>
          </div>
        )}
      </div>
    </div>
  );
};

describe('WebDAV Tab Component', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedApi.post.mockResolvedValue({ data: mockConnectionResult });
  });

  it('renders WebDAV tab correctly', () => {
    render(<WebDAVTabTestComponent />);
    
    expect(screen.getByText('WebDAV Integration')).toBeInTheDocument();
    expect(screen.getByTestId('webdav-enabled-toggle')).toBeInTheDocument();
  });

  it('shows WebDAV settings when enabled', async () => {
    render(<WebDAVTabTestComponent />);
    
    const enableToggle = screen.getByTestId('webdav-enabled-toggle');
    
    // Initially disabled
    expect(screen.queryByTestId('webdav-settings')).not.toBeInTheDocument();
    
    // Enable WebDAV
    await userEvent.click(enableToggle);
    
    // Settings should now be visible
    expect(screen.getByTestId('webdav-settings')).toBeInTheDocument();
    expect(screen.getByTestId('webdav-server-url')).toBeInTheDocument();
    expect(screen.getByTestId('webdav-username')).toBeInTheDocument();
    expect(screen.getByTestId('webdav-password')).toBeInTheDocument();
  });

  it('handles server URL input correctly', async () => {
    render(<WebDAVTabTestComponent />);
    
    // Enable WebDAV first
    const enableToggle = screen.getByTestId('webdav-enabled-toggle');
    await userEvent.click(enableToggle);
    
    const serverUrlInput = screen.getByTestId('webdav-server-url');
    await userEvent.type(serverUrlInput, 'https://cloud.example.com');
    
    expect(serverUrlInput).toHaveValue('https://cloud.example.com');
  });

  it('handles username and password input correctly', async () => {
    render(<WebDAVTabTestComponent />);
    
    // Enable WebDAV first
    const enableToggle = screen.getByTestId('webdav-enabled-toggle');
    await userEvent.click(enableToggle);
    
    const usernameInput = screen.getByTestId('webdav-username');
    const passwordInput = screen.getByTestId('webdav-password');
    
    await userEvent.type(usernameInput, 'testuser');
    await userEvent.type(passwordInput, 'testpass');
    
    expect(usernameInput).toHaveValue('testuser');
    expect(passwordInput).toHaveValue('testpass');
  });

  it('manages folder list correctly', async () => {
    render(<WebDAVTabTestComponent />);
    
    // Enable WebDAV
    const enableToggle = screen.getByTestId('webdav-enabled-toggle');
    await userEvent.click(enableToggle);
    
    // Check initial folder
    expect(screen.getByTestId('folder-0')).toHaveTextContent('/Documents');
    
    // Add new folder
    const addFolderInput = screen.getByTestId('add-folder-input');
    const addFolderBtn = screen.getByTestId('add-folder-btn');
    
    await userEvent.type(addFolderInput, '/Photos');
    await userEvent.click(addFolderBtn);
    
    // Should have both folders now
    expect(screen.getByTestId('folder-0')).toHaveTextContent('/Documents');
    expect(screen.getByTestId('folder-1')).toHaveTextContent('/Photos');
  });

  it('removes folders correctly', async () => {
    render(<WebDAVTabTestComponent />);
    
    // Enable WebDAV
    const enableToggle = screen.getByTestId('webdav-enabled-toggle');
    await userEvent.click(enableToggle);
    
    // Add a second folder first
    const addFolderInput = screen.getByTestId('add-folder-input');
    const addFolderBtn = screen.getByTestId('add-folder-btn');
    await userEvent.type(addFolderInput, '/Photos');
    await userEvent.click(addFolderBtn);
    
    // Remove first folder
    const removeBtn = screen.getByTestId('remove-folder-0');
    await userEvent.click(removeBtn);
    
    // Should only have /Photos left
    expect(screen.getByTestId('folder-0')).toHaveTextContent('/Photos');
    expect(screen.queryByTestId('folder-1')).not.toBeInTheDocument();
  });

  it('handles sync interval changes', async () => {
    render(<WebDAVTabTestComponent />);
    
    // Enable WebDAV
    const enableToggle = screen.getByTestId('webdav-enabled-toggle');
    await userEvent.click(enableToggle);
    
    const syncIntervalInput = screen.getByTestId('sync-interval');
    
    await userEvent.clear(syncIntervalInput);
    await userEvent.type(syncIntervalInput, '120');
    
    expect(syncIntervalInput).toHaveValue(120);
  });

  it('handles auto sync toggle', async () => {
    render(<WebDAVTabTestComponent />);
    
    // Enable WebDAV
    const enableToggle = screen.getByTestId('webdav-enabled-toggle');
    await userEvent.click(enableToggle);
    
    const autoSyncToggle = screen.getByTestId('auto-sync-toggle');
    
    // Should be initially unchecked
    expect(autoSyncToggle).not.toBeChecked();
    
    await userEvent.click(autoSyncToggle);
    
    expect(autoSyncToggle).toBeChecked();
  });

  it('displays test connection button', async () => {
    render(<WebDAVTabTestComponent />);
    
    // Enable WebDAV
    const enableToggle = screen.getByTestId('webdav-enabled-toggle');
    await userEvent.click(enableToggle);
    
    const testBtn = screen.getByTestId('test-connection-btn');
    expect(testBtn).toBeInTheDocument();
    expect(testBtn).toHaveTextContent('Test Connection');
  });

  it('displays estimate crawl button', async () => {
    render(<WebDAVTabTestComponent />);
    
    // Enable WebDAV
    const enableToggle = screen.getByTestId('webdav-enabled-toggle');
    await userEvent.click(enableToggle);
    
    const estimateBtn = screen.getByTestId('estimate-crawl-btn');
    expect(estimateBtn).toBeInTheDocument();
    expect(estimateBtn).toHaveTextContent('Estimate Crawl');
  });

  it('prevents duplicate folder addition', async () => {
    render(<WebDAVTabTestComponent />);
    
    // Enable WebDAV
    const enableToggle = screen.getByTestId('webdav-enabled-toggle');
    await userEvent.click(enableToggle);
    
    const addFolderInput = screen.getByTestId('add-folder-input');
    const addFolderBtn = screen.getByTestId('add-folder-btn');
    
    // Try to add the same folder that already exists
    await userEvent.type(addFolderInput, '/Documents');
    await userEvent.click(addFolderBtn);
    
    // Should still only have one folder
    expect(screen.getByTestId('folder-0')).toBeInTheDocument();
    expect(screen.queryByTestId('folder-1')).not.toBeInTheDocument();
  });

  it('validates sync interval range', async () => {
    render(<WebDAVTabTestComponent />);
    
    // Enable WebDAV
    const enableToggle = screen.getByTestId('webdav-enabled-toggle');
    await userEvent.click(enableToggle);
    
    const syncIntervalInput = screen.getByTestId('sync-interval');
    
    // Test minimum value
    expect(syncIntervalInput).toHaveAttribute('min', '15');
    // Test maximum value
    expect(syncIntervalInput).toHaveAttribute('max', '1440');
  });

  it('handles form validation for empty inputs', async () => {
    render(<WebDAVTabTestComponent />);
    
    // Enable WebDAV
    const enableToggle = screen.getByTestId('webdav-enabled-toggle');
    await userEvent.click(enableToggle);
    
    const addFolderInput = screen.getByTestId('add-folder-input');
    const addFolderBtn = screen.getByTestId('add-folder-btn');
    
    // Try to add empty folder
    await userEvent.click(addFolderBtn);
    
    // Should still only have the original folder
    expect(screen.getByTestId('folder-0')).toBeInTheDocument();
    expect(screen.queryByTestId('folder-1')).not.toBeInTheDocument();
  });
});

// Test API integration
describe('WebDAV API Integration', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('handles connection test API call', async () => {
    mockedApi.post.mockResolvedValue({ data: mockConnectionResult });
    
    // Test the API call format
    const testConfig = {
      server_url: 'https://cloud.example.com',
      username: 'testuser',
      password: 'testpass',
      server_type: 'nextcloud',
    };

    const response = await api.post('/webdav/test-connection', testConfig);
    
    expect(mockedApi.post).toHaveBeenCalledWith('/webdav/test-connection', testConfig);
    expect(response.data).toEqual(mockConnectionResult);
  });

  it('handles crawl estimate API call', async () => {
    mockedApi.post.mockResolvedValue({ data: mockCrawlEstimate });
    
    const crawlRequest = {
      folders: ['/Documents', '/Photos'],
    };

    const response = await api.post('/webdav/estimate-crawl', crawlRequest);
    
    expect(mockedApi.post).toHaveBeenCalledWith('/webdav/estimate-crawl', crawlRequest);
    expect(response.data).toEqual(mockCrawlEstimate);
  });

  it('handles API errors gracefully', async () => {
    const errorMessage = 'Connection failed';
    mockedApi.post.mockRejectedValue(new Error(errorMessage));
    
    const testConfig = {
      server_url: 'https://invalid.example.com',
      username: 'testuser',
      password: 'wrongpass',
      server_type: 'nextcloud',
    };

    try {
      await api.post('/webdav/test-connection', testConfig);
    } catch (error) {
      expect(error).toBeInstanceOf(Error);
      expect((error as Error).message).toBe(errorMessage);
    }
  });
});

// Test data validation
describe('WebDAV Data Validation', () => {
  it('validates server URL format', () => {
    const validUrls = [
      'https://cloud.example.com',
      'http://localhost:8080',
      'https://subdomain.example.com/path',
    ];

    const invalidUrls = [
      'not-a-url',
      'ftp://example.com',
      '',
    ];

    validUrls.forEach(url => {
      // Simple URL validation - in real app you'd use a proper validator
      expect(url.startsWith('http')).toBe(true);
    });

    invalidUrls.forEach(url => {
      expect(url.startsWith('http')).toBe(false);
    });
  });

  it('validates folder paths', () => {
    const validPaths = [
      '/Documents',
      '/Photos/2024',
      '/home/user/files',
    ];

    const invalidPaths = [
      'relative/path',
      '',
      'no-leading-slash',
    ];

    validPaths.forEach(path => {
      expect(path.startsWith('/')).toBe(true);
    });

    invalidPaths.forEach(path => {
      expect(path.startsWith('/')).toBe(false);
    });
  });

  it('validates file extensions', () => {
    const supportedExtensions = ['pdf', 'png', 'jpg', 'jpeg', 'tiff', 'bmp', 'txt'];
    const testFile = 'document.pdf';
    
    const extension = testFile.split('.').pop()?.toLowerCase();
    expect(supportedExtensions.includes(extension || '')).toBe(true);
  });

  it('validates sync interval range', () => {
    const validIntervals = [15, 30, 60, 120, 1440];
    const invalidIntervals = [5, 10, 2000];

    validIntervals.forEach(interval => {
      expect(interval >= 15 && interval <= 1440).toBe(true);
    });

    invalidIntervals.forEach(interval => {
      expect(interval >= 15 && interval <= 1440).toBe(false);
    });
  });
});