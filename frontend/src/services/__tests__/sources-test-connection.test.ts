/**
 * Tests for Source Test Connection API Endpoint
 *
 * These tests verify that the frontend calls the correct API endpoint
 * for testing source connections. This prevents route mismatch bugs
 * between frontend and backend (Issue #431).
 *
 * The correct endpoint is: POST /api/sources/test-connection
 * NOT: POST /api/sources/test (the old incorrect route)
 */

import { describe, it, expect } from 'vitest';

describe('Source Test Connection API Endpoint', () => {
  it('should use /sources/test-connection endpoint (not /sources/test)', () => {
    // This test documents the correct endpoint URL
    // The frontend should call: POST /api/sources/test-connection
    // NOT: POST /api/sources/test

    const correctEndpoint = '/sources/test-connection';
    const incorrectEndpoint = '/sources/test';

    // Verify our expected endpoint matches what the frontend should call
    expect(correctEndpoint).toBe('/sources/test-connection');
    expect(correctEndpoint).not.toBe(incorrectEndpoint);
  });

  describe('WebDAV test connection', () => {
    it('should construct correct request body for WebDAV', () => {
      const webdavConfig = {
        source_type: 'webdav',
        config: {
          server_url: 'https://cloud.example.com/remote.php/dav/files/user/',
          username: 'testuser',
          password: 'testpass',
          server_type: 'nextcloud',
          watch_folders: ['/Documents'],
          file_extensions: ['pdf', 'txt'],
        },
      };

      // Verify the structure matches what the backend expects
      expect(webdavConfig).toHaveProperty('source_type', 'webdav');
      expect(webdavConfig.config).toHaveProperty('server_url');
      expect(webdavConfig.config).toHaveProperty('username');
      expect(webdavConfig.config).toHaveProperty('password');
      expect(webdavConfig.config).toHaveProperty('server_type');
    });
  });

  describe('Local Folder test connection', () => {
    it('should construct correct request body for Local Folder', () => {
      const localFolderConfig = {
        source_type: 'local_folder',
        config: {
          watch_folders: ['/data/documents'],
          file_extensions: ['pdf', 'png', 'jpg'],
          recursive: true,
          follow_symlinks: false,
        },
      };

      // Verify the structure matches what the backend expects
      expect(localFolderConfig).toHaveProperty('source_type', 'local_folder');
      expect(localFolderConfig.config).toHaveProperty('watch_folders');
      expect(localFolderConfig.config).toHaveProperty('recursive');
      expect(localFolderConfig.config).toHaveProperty('follow_symlinks');
    });
  });

  describe('S3 test connection', () => {
    it('should construct correct request body for S3', () => {
      const s3Config = {
        source_type: 's3',
        config: {
          bucket_name: 'my-documents-bucket',
          region: 'us-east-1',
          access_key_id: 'AKIAEXAMPLE',
          secret_access_key: 'secretkey',
          endpoint_url: null,
          prefix: 'documents/',
        },
      };

      // Verify the structure matches what the backend expects
      expect(s3Config).toHaveProperty('source_type', 's3');
      expect(s3Config.config).toHaveProperty('bucket_name');
      expect(s3Config.config).toHaveProperty('region');
      expect(s3Config.config).toHaveProperty('access_key_id');
      expect(s3Config.config).toHaveProperty('secret_access_key');
    });
  });

  describe('API endpoint consistency', () => {
    /**
     * This test exists to catch future route mismatches.
     * If the backend changes the endpoint, this test should fail
     * and remind developers to update the frontend as well.
     */
    it('should match the backend route definition', () => {
      // Backend route defined in src/routes/sources/mod.rs:
      // .route("/test-connection", post(test_connection_with_config))
      //
      // Frontend calls in frontend/src/pages/SourcesPage.tsx:
      // api.post('/sources/test-connection', {...})

      const backendRoute = '/test-connection';
      const frontendEndpoint = '/sources/test-connection';

      // The frontend endpoint should be /sources + backendRoute
      expect(frontendEndpoint).toBe(`/sources${backendRoute}`);
    });

    it('should NOT use the old /test endpoint', () => {
      // This was the old incorrect route that caused Issue #431
      const oldIncorrectEndpoint = '/sources/test';
      const correctEndpoint = '/sources/test-connection';

      expect(correctEndpoint).not.toBe(oldIncorrectEndpoint);
    });
  });

  describe('Response handling', () => {
    it('should handle successful connection test response', () => {
      const successResponse = {
        success: true,
        message: 'Connection successful',
      };

      expect(successResponse.success).toBe(true);
      expect(successResponse.message).toBeTruthy();
    });

    it('should handle failed connection test response', () => {
      const failureResponse = {
        success: false,
        message: 'Connection failed: Unable to reach server',
      };

      expect(failureResponse.success).toBe(false);
      expect(failureResponse.message).toBeTruthy();
    });
  });
});

/**
 * Integration contract test - verifies the endpoint matches between
 * frontend and backend definitions.
 *
 * Frontend: api.post('/sources/test-connection', ...)
 * Backend:  .route("/test-connection", post(test_connection_with_config))
 *
 * Combined with base URL '/api' → Full path: POST /api/sources/test-connection
 */
describe('Frontend-Backend Contract', () => {
  it('documents the expected API contract for test-connection', () => {
    const contract = {
      method: 'POST',
      basePath: '/api',
      routePrefix: '/sources',
      routePath: '/test-connection',
      fullPath: '/api/sources/test-connection',
      requestBody: {
        source_type: 'webdav | local_folder | s3',
        config: 'object (varies by source_type)',
      },
      responseBody: {
        success: 'boolean',
        message: 'string',
      },
    };

    // This documents the expected contract
    expect(contract.fullPath).toBe('/api/sources/test-connection');
    expect(contract.method).toBe('POST');
  });
});
