import { Page, expect, type Locator } from '@playwright/test';
import { TEST_FILES } from './test-data';
import * as path from 'path';
import * as fs from 'fs';
import { fileURLToPath } from 'url';

// ES Module compatibility for __dirname
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

/** Files in TEST_FILES are relative to frontend/. */
export function resolveTestFile(filePath: string): string {
  return path.resolve(__dirname, '../..', filePath);
}

export type IntakeSection = 'upload' | 'connections' | 'watch' | 'attention' | 'ignored';

export interface WebdavSourceInput {
  name: string;
  serverUrl?: string;
  enabled?: boolean;
}

export class TestHelpers {
  constructor(private page: Page) {}

  // ---------------------------------------------------------------- API

  /** Bearer token of the signed-in user (the app keeps it in localStorage). */
  async getAuthToken(): Promise<string> {
    const token = await this.page.evaluate(() => localStorage.getItem('token'));
    if (!token) {
      throw new Error('No auth token found in localStorage. Ensure user is logged in.');
    }
    return token;
  }

  private async authHeaders(): Promise<Record<string, string>> {
    return { Authorization: `Bearer ${await this.getAuthToken()}` };
  }

  /** Upload a file from disk through the API. Returns the document id. */
  async uploadDocumentViaAPI(filePath: string): Promise<string> {
    const absolutePath = resolveTestFile(filePath);
    if (!fs.existsSync(absolutePath)) {
      throw new Error(`Test file not found: ${absolutePath}`);
    }
    const name = path.basename(absolutePath);
    return this.uploadBufferViaAPI(name, fs.readFileSync(absolutePath), this.getMimeType(name));
  }

  /** Upload in-memory content through the API under the given filename. Returns the document id. */
  async uploadBufferViaAPI(name: string, buffer: Buffer, mimeType = this.getMimeType(name)): Promise<string> {
    const response = await this.page.request.post('/api/documents', {
      headers: await this.authHeaders(),
      multipart: { file: { name, mimeType, buffer } },
      timeout: 60000,
    });
    if (!response.ok()) {
      throw new Error(`Failed to upload document via API: ${response.status()} - ${await response.text()}`);
    }
    const result = await response.json();
    return result.id || result.document_id;
  }

  async getDocumentViaAPI(documentId: string): Promise<any> {
    const response = await this.page.request.get(`/api/documents/${documentId}`, {
      headers: await this.authHeaders(),
      timeout: 10000,
    });
    if (!response.ok()) {
      throw new Error(`Failed to get document: ${response.status()}`);
    }
    return response.json();
  }

  /** Poll until OCR finishes (completed or failed). Returns the document. */
  async waitForOCRComplete(documentId: string, timeoutMs: number = 120000): Promise<any> {
    const start = Date.now();
    while (Date.now() - start < timeoutMs) {
      const doc = await this.getDocumentViaAPI(documentId);
      if (['completed', 'success', 'failed', 'error'].includes(doc.ocr_status)) {
        return doc;
      }
      await this.page.waitForTimeout(1000);
    }
    throw new Error(`OCR did not complete within ${timeoutMs}ms for document ${documentId}`);
  }

  async deleteDocumentViaAPI(documentId: string): Promise<void> {
    const response = await this.page.request.delete(`/api/documents/${documentId}`, {
      headers: await this.authHeaders(),
      timeout: 10000,
    });
    if (!response.ok()) {
      console.warn(`Failed to delete document ${documentId}: ${response.status()}`);
    }
  }

  async getOCRLanguagesViaAPI(): Promise<any[]> {
    const response = await this.page.request.get('/api/ocr/languages', {
      headers: await this.authHeaders(),
      timeout: 10000,
    });
    if (!response.ok()) {
      throw new Error(`Failed to get OCR languages: ${response.status()}`);
    }
    return response.json();
  }

  async getSettingsViaAPI(): Promise<Record<string, any>> {
    const response = await this.page.request.get('/api/settings', { headers: await this.authHeaders() });
    if (!response.ok()) throw new Error(`Failed to read settings: ${response.status()}`);
    return response.json();
  }

  async updateSettingsViaAPI(settings: Record<string, any>): Promise<void> {
    const response = await this.page.request.put('/api/settings', {
      headers: { ...(await this.authHeaders()), 'Content-Type': 'application/json' },
      data: settings,
      timeout: 10000,
    });
    if (!response.ok()) {
      throw new Error(`Failed to update settings: ${response.status()}`);
    }
  }

  /** Create a WebDAV connection through the API (the server is never contacted on create). */
  async createWebdavSourceViaAPI(input: WebdavSourceInput): Promise<{ id: string; name: string }> {
    const response = await this.page.request.post('/api/sources', {
      headers: await this.authHeaders(),
      data: {
        name: input.name,
        source_type: 'webdav',
        enabled: input.enabled ?? true,
        config: {
          server_url: input.serverUrl ?? 'http://127.0.0.1:9/dav/',
          username: 'e2e',
          password: 'e2e-password',
          watch_folders: ['/Documents'],
          file_extensions: ['pdf', 'png'],
          auto_sync: false,
          sync_interval_minutes: 60,
          server_type: 'generic',
        },
      },
    });
    if (!response.ok()) {
      throw new Error(`Failed to create source: ${response.status()} - ${await response.text()}`);
    }
    const body = await response.json();
    return { id: body.id, name: body.name };
  }

  async deleteSourceViaAPI(sourceId: string): Promise<void> {
    await this.page.request.delete(`/api/sources/${sourceId}`, { headers: await this.authHeaders() });
  }

  /** A unique name for data created by a test. */
  uniqueName(base: string): string {
    return `${base}_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`;
  }

  private getMimeType(fileName: string): string {
    const ext = path.extname(fileName).toLowerCase();
    const mimeTypes: Record<string, string> = {
      '.pdf': 'application/pdf',
      '.png': 'image/png',
      '.jpg': 'image/jpeg',
      '.jpeg': 'image/jpeg',
      '.gif': 'image/gif',
      '.doc': 'application/msword',
      '.docx': 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
      '.txt': 'text/plain',
    };
    return mimeTypes[ext] || 'application/octet-stream';
  }

  // ---------------------------------------------------------------- UI

  async waitForApiCall(urlPattern: string | RegExp, timeout = 10000) {
    return this.page.waitForResponse(
      (resp) => (typeof urlPattern === 'string' ? resp.url().includes(urlPattern) : urlPattern.test(resp.url())),
      { timeout },
    );
  }

  /** The toast region; toasts are alertdialogs named "<Tone>: <title>". */
  toasts(): Locator {
    return this.page.getByRole('region', { name: 'Notifications' });
  }

  async waitForToast(message?: string | RegExp): Promise<Locator> {
    const toast = message
      ? this.toasts().getByRole('alertdialog', { name: message })
      : this.toasts().getByRole('alertdialog').first();
    await expect(toast).toBeVisible({ timeout: 10000 });
    return toast;
  }

  /** Wait for skeletons to go (they carry role=status with a "Loading" name). */
  async waitForLoadingToComplete() {
    await this.page.waitForLoadState('domcontentloaded');
    await expect(this.page.locator('[aria-busy="true"]')).toHaveCount(0, { timeout: 15000 }).catch(() => undefined);
  }

  async navigateToPage(pathname: string) {
    await this.page.goto(pathname);
    await expect(this.page.getByRole('heading', { level: 1 })).toBeVisible({ timeout: 15000 });
  }

  async openIntake(section: IntakeSection) {
    await this.page.goto(`/intake?section=${section}`);
    await expect(this.page.getByRole('heading', { level: 1, name: 'Intake' })).toBeVisible({ timeout: 15000 });
  }

  /** The Library grid. */
  documentsGrid(): Locator {
    return this.page.getByRole('grid', { name: 'Documents' });
  }

  /** Body rows of the Library grid (header row excluded). */
  documentRows(): Locator {
    return this.documentsGrid().getByRole('rowgroup').nth(1).getByRole('row');
  }

  /**
   * The OCR text on the document page. Wide screens show preview and text side by side;
   * narrower ones put them behind "Preview" / "Text" tabs.
   */
  async extractedText(): Promise<Locator> {
    await expect(this.page.getByRole('heading', { level: 1 })).toBeVisible();
    const tab = this.page.getByRole('tab', { name: 'Text' });
    if (await tab.isVisible()) await tab.click();
    const region = this.page.getByRole('region', { name: 'Extracted text' });
    await expect(region).toBeVisible();
    return region;
  }

  async takeScreenshotOnFailure(testName: string) {
    await this.page.screenshot({ path: `test-results/screenshots/${testName}-${Date.now()}.png`, fullPage: true });
  }

  /** Upload one file through Intake → Add documents and wait for it to finish uploading. */
  async uploadTestDocument(fileName: string = 'test1.png') {
    await this.openIntake('upload');
    const filePath = fileName === 'test1.png' ? TEST_FILES.test1 : `../tests/test_images/${fileName}`;
    await this.page.locator('input[type="file"]').first().setInputFiles(filePath);
    const queue = this.page.getByRole('grid', { name: 'Files to upload' });
    await expect(queue.getByRole('row', { name: new RegExp(fileName) })).toBeVisible();
    const upload = this.waitForApiCall('/api/documents', 30000);
    await this.page.getByRole('button', { name: /^Upload all/ }).click();
    await upload;
  }

  /** Make sure the signed-in user has at least one document (uploads test1.png through the API). */
  async ensureTestDocumentsExist() {
    const response = await this.page.request.get('/api/documents?limit=1', { headers: await this.authHeaders() });
    const body = response.ok() ? await response.json() : null;
    const count = Array.isArray(body) ? body.length : (body?.documents?.length ?? 0);
    if (count === 0) {
      const id = await this.uploadDocumentViaAPI(TEST_FILES.test1);
      await this.waitForOCRComplete(id);
    }
  }

  /**
   * Create a connection through Intake → Connections → Add connection.
   * The page must already be on the Connections section. Returns the unique name used.
   */
  async createTestSource(
    baseName: string,
    type: 'webdav' | 'local_folder' | 's3',
    options?: { uniqueSuffix?: string; serverUrl?: string },
  ): Promise<string> {
    const suffix = options?.uniqueSuffix || Math.random().toString(36).substring(7);
    const sourceName = `${baseName}_${Date.now()}_${suffix}`;

    await this.page.getByRole('button', { name: 'Add connection' }).first().click();
    const dialog = this.page.getByRole('dialog', { name: 'Add connection' });
    await expect(dialog).toBeVisible();
    await dialog.getByRole('textbox', { name: 'Name', exact: true }).fill(sourceName);

    if (type === 'webdav') {
      await dialog.getByRole('radio', { name: /^WebDAV/ }).check({ force: true });
      await dialog.getByRole('textbox', { name: 'Server URL' }).fill(options?.serverUrl ?? 'http://127.0.0.1:9/dav/');
      await dialog.getByRole('textbox', { name: 'Username' }).fill('testuser');
      await dialog.getByRole('textbox', { name: 'Password' }).fill('testpass');
    } else if (type === 's3') {
      await dialog.getByRole('radio', { name: /^S3-compatible/ }).check({ force: true });
      await dialog.getByRole('textbox', { name: 'Bucket name' }).fill('test-bucket');
      await dialog.getByRole('textbox', { name: 'Access key ID' }).fill('AKIATESTKEY0000');
      await dialog.getByRole('textbox', { name: 'Secret access key' }).fill('test-secret-key');
    } else {
      await dialog.getByRole('radio', { name: /^Local folder/ }).check({ force: true });
    }

    const create = this.page.waitForResponse(
      (r) => /\/api\/sources$/.test(r.url()) && r.request().method() === 'POST',
      { timeout: 10000 },
    );
    await dialog.getByRole('button', { name: 'Add connection' }).click();
    const response = await create;
    expect(response.ok(), `create source returned ${response.status()}`).toBe(true);
    await expect(dialog).toBeHidden();
    await expect(this.connectionRow(sourceName)).toBeVisible();
    return sourceName;
  }

  /** A row of the Intake → Connections board. */
  connectionRow(name: string): Locator {
    return this.page.getByRole('grid', { name: 'Connections' }).getByRole('row', { name: new RegExp(escapeRegExp(name)) });
  }

  /** Open a connection's detail panel from its row. Returns the panel. */
  async openConnection(name: string): Promise<Locator> {
    await this.connectionRow(name).click();
    const panel = this.page.getByRole('dialog', { name });
    await expect(panel).toBeVisible();
    return panel;
  }
}

export function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}
