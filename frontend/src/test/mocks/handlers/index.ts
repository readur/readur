/**
 * Central export for all mock handlers
 * Combines all API endpoint handlers for MSW
 */

import { documentHandlers } from './documents'
import { authHandlers } from './auth'
import { searchHandlers } from './search'
import { queueHandlers } from './queue'
import { sourceHandlers } from './sources'
import { labelHandlers } from './labels'
import { userHandlers } from './users'
import { ocrHandlers } from './ocr'
import { settingsHandlers } from './settings'

/**
 * All mock handlers combined
 * These are used by MSW to intercept HTTP requests
 */
export const handlers = [
  ...documentHandlers,
  ...authHandlers,
  ...searchHandlers,
  ...queueHandlers,
  ...sourceHandlers,
  ...labelHandlers,
  ...userHandlers,
  ...ocrHandlers,
  ...settingsHandlers,
]

// Export individual handler groups for selective mocking
export {
  documentHandlers,
  authHandlers,
  searchHandlers,
  queueHandlers,
  sourceHandlers,
  labelHandlers,
  userHandlers,
  ocrHandlers,
  settingsHandlers,
}