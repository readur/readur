import { test, expect } from './fixtures/auth';
import type { Page } from '@playwright/test';
import { TEST_FILES } from './utils/test-data';
import { TestHelpers } from './utils/test-helpers';

/** ⌘K / Ctrl+K palette: destinations, document search, recent searches. */
test.describe('Command palette', () => {
  const palette = (page: Page) => page.getByRole('dialog', { name: 'Command palette' });
  const input = (page: Page) => palette(page).getByRole('searchbox', { name: 'Search' });

  test('opens with the keyboard shortcut and closes with Esc', async ({ dynamicUserPage: page }) => {
    await page.goto('/home');
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();

    await page.keyboard.press('ControlOrMeta+k');
    await expect(palette(page)).toBeVisible();
    await expect(input(page)).toBeFocused();

    // With nothing typed it offers the destinations
    const goTo = palette(page).getByRole('group', { name: 'Go to' });
    for (const d of ['Home', 'Search', 'Library', 'Intake', 'Sources', 'Settings']) {
      await expect(goTo.getByRole('menuitem', { name: d, exact: true })).toBeVisible();
    }

    await page.keyboard.press('Escape');
    await expect(palette(page)).toBeHidden();
  });

  test('opens from the quick-find field in the sidebar', async ({ dynamicUserPage: page }) => {
    await page.goto('/home');
    const trigger = page.getByRole('button', { name: 'Search documents' });
    await expect(trigger).toHaveAttribute('aria-keyshortcuts', /K/i);
    await trigger.click();
    await expect(palette(page)).toBeVisible();
  });

  test('typing searches documents and Enter opens the top result', async ({ dynamicUserPage: page }) => {
    const helpers = new TestHelpers(page);
    const id = await helpers.uploadDocumentViaAPI(TEST_FILES.test1);
    await helpers.waitForOCRComplete(id);

    await page.goto('/home');
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
    await page.keyboard.press('ControlOrMeta+k');
    await input(page).fill('some text from text');

    const documents = palette(page).getByRole('group', { name: 'Documents' });
    const hit = documents.getByRole('menuitem', { name: /test1\.png/ });
    await expect(hit).toBeVisible();
    // The best snippet is shown with the result
    await expect(hit).toContainText('This is some text from text 1');
    await expect(documents.getByRole('menuitem', { name: /Show all results for “some text from text”/ })).toBeVisible();

    // The document opens in the drawer over the page the palette was opened on.
    await page.keyboard.press('Enter');
    await expect(page).toHaveURL(new RegExp(`[?&]document=${id}`));
    await expect(page.getByRole('dialog', { name: 'test1.png' })).toBeVisible();
    await page.keyboard.press('Escape');

    // The search is remembered as a recent search
    await page.keyboard.press('ControlOrMeta+k');
    await expect(palette(page).getByRole('group', { name: /Recent/ })).toContainText('some text from text');
  });

  test('Enter on a destination navigates there', async ({ dynamicUserPage: page }) => {
    await page.goto('/home');
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
    await page.keyboard.press('ControlOrMeta+k');
    await input(page).fill('connections');

    const item = palette(page).getByRole('group', { name: 'Go to' }).getByRole('menuitem', { name: 'Connections' });
    await expect(item).toBeVisible();
    await page.keyboard.press('Enter');
    await expect(page).toHaveURL(/\/sources\?section=connections/);
    await expect(palette(page)).toBeHidden();
  });

  test('"Show all results" opens the Search page', async ({ dynamicUserPage: page }) => {
    await page.goto('/home');
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
    await page.keyboard.press('ControlOrMeta+k');
    await input(page).fill('quarterly');
    await palette(page).getByRole('menuitem', { name: /Show all results for “quarterly”/ }).click();
    await expect(page).toHaveURL(/\/search\?q=quarterly/);
    await expect(page.getByRole('searchbox').first()).toHaveValue('quarterly');
  });
});
