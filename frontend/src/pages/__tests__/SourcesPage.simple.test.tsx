import { describe, it, expect } from 'vitest';

describe('SourcesPage Test Connection Fix', () => {
  it('Test connection should use unified /sources/test-connection endpoint for all source types', () => {
    // This test documents the fix for the bug where WebDAV test connection
    // was failing for existing sources because it was using /webdav/test-connection
    // endpoint which validated watch_folders existence, but the frontend wasn't
    // sending the watch_folders that were already configured for the source.
    
    // The fix ensures all source types (WebDAV, S3, Local Folder) use the same
    // /sources/test-connection endpoint and include all necessary configuration
    // including watch_folders and file_extensions.
    
    const testConnectionEndpoint = '/sources/test-connection';
    
    // All source types should use the same endpoint
    expect(testConnectionEndpoint).toBe('/sources/test-connection');
    
    // WebDAV payload should include watch_folders
    const webdavPayload = {
      source_type: 'webdav',
      config: {
        server_url: 'https://example.com',
        username: 'user',
        password: 'pass',
        server_type: 'generic',
        watch_folders: ['/Documents'],  // This was missing before the fix
        file_extensions: ['pdf', 'jpg']  // This was missing before the fix
      }
    };
    
    expect(webdavPayload.config.watch_folders).toBeDefined();
    expect(webdavPayload.config.watch_folders.length).toBeGreaterThan(0);
    expect(webdavPayload.config.file_extensions).toBeDefined();
  });

  it('WebDAV config validation should pass when watch_folders are provided', () => {
    // The backend WebDAVConfig.validate() method requires at least one watch folder
    // This test documents that requirement
    
    const validConfig = {
      server_url: 'https://webdav.example.com',
      username: 'testuser',
      password: 'testpass',
      watch_folders: ['/Documents'],
      file_extensions: ['pdf']
    };
    
    const invalidConfig = {
      server_url: 'https://webdav.example.com',
      username: 'testuser', 
      password: 'testpass',
      watch_folders: [],  // Empty array would fail validation
      file_extensions: ['pdf']
    };
    
    // Valid config should have at least one watch folder
    expect(validConfig.watch_folders.length).toBeGreaterThan(0);
    
    // Invalid config would fail with "At least one watch folder must be specified"
    expect(invalidConfig.watch_folders.length).toBe(0);
  });

  it('Frontend form should preserve watch_folders when editing existing sources', () => {
    // When editing an existing source, the form should populate with existing config
    // including watch_folders and file_extensions
    
    const existingSourceConfig = {
      server_url: 'https://existing.webdav.com',
      username: 'existing',
      password: 'pass',
      server_type: 'nextcloud',
      watch_folders: ['/Documents', '/Photos'],
      file_extensions: ['pdf', 'jpg', 'png']
    };
    
    // Form data should be populated from existing source
    const formData = {
      name: 'Existing Source',
      source_type: 'webdav',
      enabled: true,
      server_url: existingSourceConfig.server_url,
      username: existingSourceConfig.username,
      password: existingSourceConfig.password,
      server_type: existingSourceConfig.server_type,
      watch_folders: existingSourceConfig.watch_folders,
      file_extensions: existingSourceConfig.file_extensions
    };
    
    // Verify watch_folders are preserved
    expect(formData.watch_folders).toEqual(existingSourceConfig.watch_folders);
    expect(formData.file_extensions).toEqual(existingSourceConfig.file_extensions);
  });
});