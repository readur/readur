/**
 * Central export for all factory functions
 * Provides a convenient single import for all mock data generation
 */

// Document factories
export {
  createMockDocument,
  createMockDocuments,
  createMockDocumentWithScenario,
  createDocumentScenarios,
  resetDocumentFactorySeed,
} from './document'

// User factories
export {
  createMockUser,
  createMockUsers,
  createMockUserWithScenario,
  createUserScenarios,
  getDefaultTestUser,
  getDefaultAdminUser,
  resetUserFactorySeed,
} from './user'

// Source factories
export {
  createMockSource,
  createMockSources,
  createMockSourceWithScenario,
  createSourceScenarios,
  resetSourceFactorySeed,
} from './source'

// Search and response factories
export {
  createMockSearchResponse,
  createMockSearchRequest,
  createMockEnhancedDocument,
  createMockSearchSnippet,
  createSearchScenarios,
  resetSearchFactorySeed,
} from './search'

// OCR and queue factories
export {
  createMockQueueStats,
  createMockOcrResponse,
  createMockRetryStats,
  createMockSyncProgress,
  createQueueScenarios,
  resetQueueFactorySeed,
} from './queue'

// Label factories
export {
  createMockLabel,
  createMockLabels,
  createMockLabelWithScenario,
  createLabelScenarios,
  createDefaultLabels,
  resetLabelFactorySeed,
} from './label'

// Combined factory utilities
export {
  createCompleteTestDataset,
  resetAllFactorySeeds,
  generateScenarioDataset,
} from './combined'