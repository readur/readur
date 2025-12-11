import { describe, it, expect } from 'vitest';

/**
 * Unit tests for the URL Preview feature in SourcesPage
 *
 * The URL preview shows users an example of the sync URL that will be constructed
 * based on their inputs (server URL, server type, username, watch folders).
 *
 * This tests the URL construction logic that mirrors what the backend does
 * in src/services/webdav/config.rs
 */

// Helper function that mirrors the buildExampleSyncUrl logic from SourcesPage
// Extracted here for direct unit testing
interface UrlPart {
  text: string;
  type: 'server' | 'path' | 'folder' | 'file';
}

interface FormData {
  source_type: 'webdav' | 'local_folder' | 's3';
  server_url: string;
  username: string;
  server_type: 'nextcloud' | 'owncloud' | 'generic';
  watch_folders: string[];
  bucket_name: string;
  region: string;
  endpoint_url: string;
  prefix: string;
}

function buildExampleSyncUrl(formData: FormData): { parts: UrlPart[] } | null {
  const exampleFile = 'document1.pdf';
  const firstFolder = formData.watch_folders.length > 0 ? formData.watch_folders[0] : '/Documents';

  if (formData.source_type === 'webdav') {
    if (!formData.server_url) return null;

    let serverUrl = formData.server_url.trim();
    // Add https:// if no protocol specified
    if (!serverUrl.startsWith('http://') && !serverUrl.startsWith('https://')) {
      serverUrl = `https://${serverUrl}`;
    }
    serverUrl = serverUrl.replace(/\/+$/, ''); // Remove trailing slashes

    let webdavPath = '';
    if (formData.server_type === 'nextcloud') {
      // Nextcloud uses /remote.php/dav/files/{username}
      if (!serverUrl.includes('/remote.php/dav/files/')) {
        webdavPath = `/remote.php/dav/files/${formData.username || 'username'}`;
      }
    } else if (formData.server_type === 'owncloud') {
      // ownCloud uses /remote.php/webdav
      if (!serverUrl.includes('/remote.php/webdav')) {
        webdavPath = '/remote.php/webdav';
      }
    }
    // For generic, use the URL as-is

    const cleanFolder = firstFolder.replace(/^\/+/, ''); // Remove leading slashes

    return {
      parts: [
        { text: serverUrl, type: 'server' },
        { text: webdavPath, type: 'path' },
        { text: `/${cleanFolder}`, type: 'folder' },
        { text: `/${exampleFile}`, type: 'file' },
      ],
    };
  } else if (formData.source_type === 's3') {
    if (!formData.bucket_name) return null;

    const endpoint = formData.endpoint_url?.trim() || `https://s3.${formData.region || 'us-east-1'}.amazonaws.com`;
    const cleanEndpoint = endpoint.replace(/\/+$/, '');
    const prefix = formData.prefix?.trim().replace(/^\/+|\/+$/g, '') || '';
    const cleanFolder = firstFolder.replace(/^\/+|\/+$/, '');

    const parts: UrlPart[] = [
      { text: cleanEndpoint, type: 'server' },
      { text: `/${formData.bucket_name}`, type: 'path' },
      { text: `/${cleanFolder}`, type: 'folder' },
      { text: `/${exampleFile}`, type: 'file' },
    ];
    // Insert prefix after bucket if present
    if (prefix) {
      parts.splice(2, 0, { text: `/${prefix}`, type: 'path' });
    }

    return { parts };
  } else if (formData.source_type === 'local_folder') {
    if (formData.watch_folders.length === 0) return null;

    return {
      parts: [
        { text: firstFolder, type: 'folder' },
        { text: `/${exampleFile}`, type: 'file' },
      ],
    };
  }

  return null;
}

// Helper to join URL parts into a single string for easier assertion
function joinUrlParts(result: { parts: UrlPart[] } | null): string {
  if (!result) return '';
  return result.parts.map(p => p.text).join('');
}

describe('SourcesPage URL Preview - WebDAV', () => {
  const baseWebdavForm: FormData = {
    source_type: 'webdav',
    server_url: '',
    username: '',
    server_type: 'generic',
    watch_folders: ['/Documents'],
    bucket_name: '',
    region: 'us-east-1',
    endpoint_url: '',
    prefix: '',
  };

  describe('Nextcloud server type', () => {
    it('should construct URL with /remote.php/dav/files/{username} path', () => {
      const formData: FormData = {
        ...baseWebdavForm,
        server_url: 'https://cloud.example.com',
        username: 'john',
        server_type: 'nextcloud',
        watch_folders: ['/Documents'],
      };

      const result = buildExampleSyncUrl(formData);
      const url = joinUrlParts(result);

      expect(url).toBe('https://cloud.example.com/remote.php/dav/files/john/Documents/document1.pdf');
    });

    it('should use "username" placeholder when username is empty', () => {
      const formData: FormData = {
        ...baseWebdavForm,
        server_url: 'https://cloud.example.com',
        username: '',
        server_type: 'nextcloud',
      };

      const result = buildExampleSyncUrl(formData);
      const url = joinUrlParts(result);

      expect(url).toContain('/remote.php/dav/files/username/');
    });

    it('should not duplicate path if URL already contains /remote.php/dav/files/', () => {
      const formData: FormData = {
        ...baseWebdavForm,
        server_url: 'https://cloud.example.com/remote.php/dav/files/john',
        username: 'john',
        server_type: 'nextcloud',
      };

      const result = buildExampleSyncUrl(formData);

      // Should not have double /remote.php/dav/files/
      expect(result?.parts.filter(p => p.text.includes('/remote.php/dav/files/')).length).toBeLessThanOrEqual(1);
    });

    it('should handle server URL without trailing slash', () => {
      const formData: FormData = {
        ...baseWebdavForm,
        server_url: 'https://cloud.example.com',
        username: 'john',
        server_type: 'nextcloud',
      };

      const url = joinUrlParts(buildExampleSyncUrl(formData));

      // Should not have double slashes (except in https://)
      expect(url.replace('https://', '')).not.toContain('//');
    });

    it('should handle server URL with trailing slash', () => {
      const formData: FormData = {
        ...baseWebdavForm,
        server_url: 'https://cloud.example.com/',
        username: 'john',
        server_type: 'nextcloud',
      };

      const url = joinUrlParts(buildExampleSyncUrl(formData));

      // Should not have double slashes
      expect(url.replace('https://', '')).not.toContain('//');
    });
  });

  describe('ownCloud server type', () => {
    it('should construct URL with /remote.php/webdav path', () => {
      const formData: FormData = {
        ...baseWebdavForm,
        server_url: 'https://owncloud.example.com',
        username: 'john',
        server_type: 'owncloud',
        watch_folders: ['/Documents'],
      };

      const result = buildExampleSyncUrl(formData);
      const url = joinUrlParts(result);

      expect(url).toBe('https://owncloud.example.com/remote.php/webdav/Documents/document1.pdf');
    });

    it('should not duplicate path if URL already contains /remote.php/webdav', () => {
      const formData: FormData = {
        ...baseWebdavForm,
        server_url: 'https://owncloud.example.com/remote.php/webdav',
        username: 'john',
        server_type: 'owncloud',
      };

      const result = buildExampleSyncUrl(formData);

      // Should not have double /remote.php/webdav
      expect(result?.parts.filter(p => p.text.includes('/remote.php/webdav')).length).toBeLessThanOrEqual(1);
    });
  });

  describe('Generic WebDAV server type', () => {
    it('should use server URL as-is without adding WebDAV path', () => {
      const formData: FormData = {
        ...baseWebdavForm,
        server_url: 'https://webdav.example.com/dav',
        username: 'john',
        server_type: 'generic',
        watch_folders: ['/Documents'],
      };

      const result = buildExampleSyncUrl(formData);
      const url = joinUrlParts(result);

      expect(url).toBe('https://webdav.example.com/dav/Documents/document1.pdf');
    });

    it('should not add /remote.php paths for generic servers', () => {
      const formData: FormData = {
        ...baseWebdavForm,
        server_url: 'https://custom.webdav.com',
        server_type: 'generic',
      };

      const result = buildExampleSyncUrl(formData);
      const url = joinUrlParts(result);

      expect(url).not.toContain('/remote.php');
    });
  });

  describe('Protocol handling', () => {
    it('should add https:// when no protocol is specified', () => {
      const formData: FormData = {
        ...baseWebdavForm,
        server_url: 'cloud.example.com',
        server_type: 'generic',
      };

      const result = buildExampleSyncUrl(formData);
      const url = joinUrlParts(result);

      expect(url).toStartWith('https://');
    });

    it('should preserve http:// when explicitly specified', () => {
      const formData: FormData = {
        ...baseWebdavForm,
        server_url: 'http://local.webdav.com',
        server_type: 'generic',
      };

      const result = buildExampleSyncUrl(formData);
      const url = joinUrlParts(result);

      expect(url).toStartWith('http://');
      expect(url).not.toStartWith('https://');
    });

    it('should preserve https:// when explicitly specified', () => {
      const formData: FormData = {
        ...baseWebdavForm,
        server_url: 'https://secure.webdav.com',
        server_type: 'generic',
      };

      const result = buildExampleSyncUrl(formData);
      const url = joinUrlParts(result);

      expect(url).toStartWith('https://');
    });
  });

  describe('Watch folder handling', () => {
    it('should use first watch folder in the URL', () => {
      const formData: FormData = {
        ...baseWebdavForm,
        server_url: 'https://webdav.example.com',
        server_type: 'generic',
        watch_folders: ['/Photos', '/Documents', '/Videos'],
      };

      const result = buildExampleSyncUrl(formData);
      const url = joinUrlParts(result);

      expect(url).toContain('/Photos/');
      expect(url).not.toContain('/Documents/');
    });

    it('should handle watch folder with leading slash', () => {
      const formData: FormData = {
        ...baseWebdavForm,
        server_url: 'https://webdav.example.com',
        server_type: 'generic',
        watch_folders: ['/Documents'],
      };

      const url = joinUrlParts(buildExampleSyncUrl(formData));

      // Should not have double slashes around folder
      expect(url).toContain('/Documents/');
      expect(url.replace('https://', '')).not.toContain('//');
    });

    it('should handle watch folder without leading slash', () => {
      const formData: FormData = {
        ...baseWebdavForm,
        server_url: 'https://webdav.example.com',
        server_type: 'generic',
        watch_folders: ['Documents'],
      };

      const url = joinUrlParts(buildExampleSyncUrl(formData));

      expect(url).toContain('/Documents/');
    });

    it('should default to /Documents when watch_folders is empty', () => {
      const formData: FormData = {
        ...baseWebdavForm,
        server_url: 'https://webdav.example.com',
        server_type: 'generic',
        watch_folders: [],
      };

      const url = joinUrlParts(buildExampleSyncUrl(formData));

      expect(url).toContain('/Documents/');
    });
  });

  describe('Edge cases', () => {
    it('should return null when server_url is empty', () => {
      const formData: FormData = {
        ...baseWebdavForm,
        server_url: '',
      };

      const result = buildExampleSyncUrl(formData);

      expect(result).toBeNull();
    });

    it('should handle whitespace in server_url', () => {
      const formData: FormData = {
        ...baseWebdavForm,
        server_url: '  https://webdav.example.com  ',
        server_type: 'generic',
      };

      const url = joinUrlParts(buildExampleSyncUrl(formData));

      expect(url).toStartWith('https://');
      expect(url).not.toContain('  ');
    });
  });
});

describe('SourcesPage URL Preview - S3', () => {
  const baseS3Form: FormData = {
    source_type: 's3',
    server_url: '',
    username: '',
    server_type: 'generic',
    watch_folders: ['/documents'],
    bucket_name: '',
    region: 'us-east-1',
    endpoint_url: '',
    prefix: '',
  };

  describe('AWS S3', () => {
    it('should construct URL with default AWS endpoint when endpoint_url is empty', () => {
      const formData: FormData = {
        ...baseS3Form,
        bucket_name: 'my-bucket',
        region: 'us-west-2',
        watch_folders: ['/documents'],
      };

      const result = buildExampleSyncUrl(formData);
      const url = joinUrlParts(result);

      expect(url).toBe('https://s3.us-west-2.amazonaws.com/my-bucket/documents/document1.pdf');
    });

    it('should use us-east-1 as default region', () => {
      const formData: FormData = {
        ...baseS3Form,
        bucket_name: 'my-bucket',
        region: '',
        watch_folders: ['/documents'],
      };

      const result = buildExampleSyncUrl(formData);
      const url = joinUrlParts(result);

      expect(url).toContain('s3.us-east-1.amazonaws.com');
    });
  });

  describe('S3-compatible storage (MinIO)', () => {
    it('should use custom endpoint_url when provided', () => {
      const formData: FormData = {
        ...baseS3Form,
        bucket_name: 'my-bucket',
        endpoint_url: 'https://minio.example.com',
        watch_folders: ['/documents'],
      };

      const result = buildExampleSyncUrl(formData);
      const url = joinUrlParts(result);

      expect(url).toBe('https://minio.example.com/my-bucket/documents/document1.pdf');
    });

    it('should handle endpoint_url with trailing slash', () => {
      const formData: FormData = {
        ...baseS3Form,
        bucket_name: 'my-bucket',
        endpoint_url: 'https://minio.example.com/',
        watch_folders: ['/documents'],
      };

      const url = joinUrlParts(buildExampleSyncUrl(formData));

      // Should not have double slashes
      expect(url.replace('https://', '')).not.toContain('//');
    });
  });

  describe('Prefix handling', () => {
    it('should include prefix in URL when provided', () => {
      const formData: FormData = {
        ...baseS3Form,
        bucket_name: 'my-bucket',
        prefix: 'uploads/2024',
        watch_folders: ['/documents'],
      };

      const result = buildExampleSyncUrl(formData);
      const url = joinUrlParts(result);

      expect(url).toContain('/uploads/2024/');
    });

    it('should handle prefix with leading/trailing slashes', () => {
      const formData: FormData = {
        ...baseS3Form,
        bucket_name: 'my-bucket',
        prefix: '/uploads/2024/',
        watch_folders: ['/documents'],
      };

      const url = joinUrlParts(buildExampleSyncUrl(formData));

      // Should normalize slashes
      expect(url.replace('https://', '')).not.toContain('//');
    });

    it('should not include prefix segment when prefix is empty', () => {
      const formData: FormData = {
        ...baseS3Form,
        bucket_name: 'my-bucket',
        prefix: '',
        watch_folders: ['/documents'],
      };

      const result = buildExampleSyncUrl(formData);

      // Should only have 4 parts: server, bucket, folder, file (no prefix)
      expect(result?.parts.length).toBe(4);
    });
  });

  describe('Part types', () => {
    it('should correctly type each URL part', () => {
      const formData: FormData = {
        ...baseS3Form,
        bucket_name: 'my-bucket',
        endpoint_url: 'https://s3.example.com',
        prefix: 'prefix',
        watch_folders: ['/documents'],
      };

      const result = buildExampleSyncUrl(formData);

      expect(result?.parts[0].type).toBe('server'); // endpoint
      expect(result?.parts[1].type).toBe('path');   // bucket
      expect(result?.parts[2].type).toBe('path');   // prefix
      expect(result?.parts[3].type).toBe('folder'); // watch folder
      expect(result?.parts[4].type).toBe('file');   // example file
    });
  });

  describe('Edge cases', () => {
    it('should return null when bucket_name is empty', () => {
      const formData: FormData = {
        ...baseS3Form,
        bucket_name: '',
      };

      const result = buildExampleSyncUrl(formData);

      expect(result).toBeNull();
    });
  });
});

describe('SourcesPage URL Preview - Local Folder', () => {
  const baseLocalForm: FormData = {
    source_type: 'local_folder',
    server_url: '',
    username: '',
    server_type: 'generic',
    watch_folders: [],
    bucket_name: '',
    region: 'us-east-1',
    endpoint_url: '',
    prefix: '',
  };

  it('should show local path with example file', () => {
    const formData: FormData = {
      ...baseLocalForm,
      watch_folders: ['/home/user/Documents'],
    };

    const result = buildExampleSyncUrl(formData);
    const url = joinUrlParts(result);

    expect(url).toBe('/home/user/Documents/document1.pdf');
  });

  it('should use first watch folder', () => {
    const formData: FormData = {
      ...baseLocalForm,
      watch_folders: ['/var/data', '/home/user/Documents'],
    };

    const result = buildExampleSyncUrl(formData);
    const url = joinUrlParts(result);

    expect(url).toContain('/var/data/');
    expect(url).not.toContain('/home/user/Documents');
  });

  it('should return null when watch_folders is empty', () => {
    const formData: FormData = {
      ...baseLocalForm,
      watch_folders: [],
    };

    const result = buildExampleSyncUrl(formData);

    expect(result).toBeNull();
  });

  it('should correctly type parts for local folder', () => {
    const formData: FormData = {
      ...baseLocalForm,
      watch_folders: ['/home/user/Documents'],
    };

    const result = buildExampleSyncUrl(formData);

    expect(result?.parts.length).toBe(2);
    expect(result?.parts[0].type).toBe('folder');
    expect(result?.parts[1].type).toBe('file');
  });
});

describe('SourcesPage URL Preview - URL Part Types', () => {
  it('WebDAV should have correct part types', () => {
    const formData: FormData = {
      source_type: 'webdav',
      server_url: 'https://cloud.example.com',
      username: 'john',
      server_type: 'nextcloud',
      watch_folders: ['/Documents'],
      bucket_name: '',
      region: 'us-east-1',
      endpoint_url: '',
      prefix: '',
    };

    const result = buildExampleSyncUrl(formData);

    expect(result?.parts[0].type).toBe('server'); // https://cloud.example.com
    expect(result?.parts[1].type).toBe('path');   // /remote.php/dav/files/john
    expect(result?.parts[2].type).toBe('folder'); // /Documents
    expect(result?.parts[3].type).toBe('file');   // /document1.pdf
  });

  it('Generic WebDAV should have empty path part', () => {
    const formData: FormData = {
      source_type: 'webdav',
      server_url: 'https://webdav.example.com',
      username: 'john',
      server_type: 'generic',
      watch_folders: ['/Documents'],
      bucket_name: '',
      region: 'us-east-1',
      endpoint_url: '',
      prefix: '',
    };

    const result = buildExampleSyncUrl(formData);

    // For generic, webdavPath is empty string
    expect(result?.parts[1].text).toBe('');
    expect(result?.parts[1].type).toBe('path');
  });
});

// Custom matcher for startsWith
expect.extend({
  toStartWith(received: string, expected: string) {
    const pass = received.startsWith(expected);
    return {
      message: () =>
        pass
          ? `expected ${received} not to start with ${expected}`
          : `expected ${received} to start with ${expected}`,
      pass,
    };
  },
});

declare global {
  namespace jest {
    interface Matchers<R> {
      toStartWith(expected: string): R;
    }
  }
}
