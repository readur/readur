/**
 * Combined factory utilities for generating complete test datasets
 * Provides high-level functions to create coherent test scenarios
 */

import { MockState } from '../api/types'
import { 
  createMockDocuments, 
  createDocumentScenarios,
  resetDocumentFactorySeed 
} from './document'
import { 
  createMockUsers, 
  createUserScenarios,
  getDefaultTestUser,
  getDefaultAdminUser,
  resetUserFactorySeed 
} from './user'
import { 
  createMockSources, 
  createSourceScenarios,
  resetSourceFactorySeed 
} from './source'
import { 
  createDefaultLabels,
  createLabelScenarios,
  resetLabelFactorySeed 
} from './label'
import { 
  createMockQueueStats,
  createQueueScenarios,
  resetQueueFactorySeed 
} from './queue'
import { 
  createSearchScenarios,
  resetSearchFactorySeed 
} from './search'

/**
 * Reset all factory seeds for consistent test results
 */
export const resetAllFactorySeeds = (seed: number = 12345) => {
  resetDocumentFactorySeed(seed)
  resetUserFactorySeed(seed)
  resetSourceFactorySeed(seed)
  resetLabelFactorySeed(seed)
  resetQueueFactorySeed(seed)
  resetSearchFactorySeed(seed)
}

/**
 * Create a complete test dataset with all entities
 */
export const createCompleteTestDataset = (): MockState => {
  resetAllFactorySeeds()

  const defaultUser = getDefaultTestUser()
  const adminUser = getDefaultAdminUser()
  const users = [defaultUser, adminUser, ...createMockUsers(3)]
  
  const documents = createMockDocuments(50, { 
    overrides: { user_id: defaultUser.id } 
  })
  
  const sources = createMockSources(5, { 
    overrides: { user_id: defaultUser.id } 
  })
  
  const labels = createDefaultLabels().map(label => ({
    ...label,
    user_id: defaultUser.id,
  }))

  return {
    documents,
    users,
    sources,
    labels,
    searchResults: new Map(),
    syncProgress: new Map(),
    queueStats: createMockQueueStats(),
    scenarios: {},
  }
}

/**
 * Generate a dataset for a specific scenario
 */
export const generateScenarioDataset = (scenarioName: string): Partial<MockState> => {
  resetAllFactorySeeds()

  switch (scenarioName) {
    case 'empty_system':
      return {
        documents: [],
        users: [getDefaultTestUser()],
        sources: [],
        labels: [],
        queueStats: createMockQueueStats({ 
          pending_count: 0, 
          processing_count: 0, 
          failed_count: 0 
        }),
      }

    case 'new_user_setup':
      const newUser = getDefaultTestUser()
      return {
        documents: [],
        users: [newUser],
        sources: [],
        labels: createDefaultLabels().map(label => ({
          ...label,
          user_id: newUser.id,
          document_count: 0,
        })),
        queueStats: createMockQueueStats({ 
          pending_count: 0, 
          processing_count: 0, 
          failed_count: 0 
        }),
      }

    case 'active_system':
      const activeUser = getDefaultTestUser()
      return {
        documents: createMockDocuments(100, { 
          overrides: { user_id: activeUser.id } 
        }),
        users: [activeUser, getDefaultAdminUser()],
        sources: createMockSources(8, { 
          overrides: { user_id: activeUser.id } 
        }),
        labels: createDefaultLabels().map(label => ({
          ...label,
          user_id: activeUser.id,
        })),
        queueStats: createMockQueueStats({ 
          pending_count: 15, 
          processing_count: 2, 
          failed_count: 5,
          completed_today: 85,
        }),
      }

    case 'system_under_load':
      const heavyUser = getDefaultTestUser()
      return {
        documents: createMockDocuments(500, { 
          overrides: { user_id: heavyUser.id } 
        }),
        users: [heavyUser, getDefaultAdminUser(), ...createMockUsers(10)],
        sources: createMockSources(15, { 
          overrides: { user_id: heavyUser.id } 
        }),
        labels: createDefaultLabels().map(label => ({
          ...label,
          user_id: heavyUser.id,
        })),
        queueStats: createMockQueueStats({ 
          pending_count: 200, 
          processing_count: 5, 
          failed_count: 50,
          avg_wait_time_minutes: 45,
          oldest_pending_minutes: 120,
        }),
      }

    case 'multi_user_system':
      const users = [getDefaultTestUser(), getDefaultAdminUser(), ...createMockUsers(5)]
      const allDocuments = users.flatMap(user => 
        createMockDocuments(20, { overrides: { user_id: user.id } })
      )
      const allSources = users.flatMap(user => 
        createMockSources(2, { overrides: { user_id: user.id } })
      )
      const allLabels = users.flatMap(user => 
        createDefaultLabels().map(label => ({ ...label, user_id: user.id }))
      )

      return {
        documents: allDocuments,
        users,
        sources: allSources,
        labels: allLabels,
        queueStats: createMockQueueStats({ 
          pending_count: 30, 
          processing_count: 3, 
          failed_count: 12 
        }),
      }

    case 'problematic_system':
      const problemUser = getDefaultTestUser()
      const failedDocuments = createMockDocuments(25, {
        overrides: { 
          user_id: problemUser.id,
          ocr_status: 'failed',
          has_ocr_text: false,
        }
      })
      
      return {
        documents: [
          ...failedDocuments,
          ...createMockDocuments(10, { overrides: { user_id: problemUser.id } })
        ],
        users: [problemUser],
        sources: createMockSources(3, { 
          overrides: { 
            user_id: problemUser.id,
            sync_status: 'error',
          } 
        }),
        labels: createDefaultLabels().map(label => ({
          ...label,
          user_id: problemUser.id,
        })),
        queueStats: createMockQueueStats({ 
          pending_count: 5, 
          processing_count: 0, 
          failed_count: 25 
        }),
      }

    default:
      console.warn(`Unknown scenario: ${scenarioName}`)
      return createCompleteTestDataset()
  }
}

/**
 * Predefined scenario configurations
 */
export const SCENARIO_CONFIGURATIONS = {
  empty_system: {
    name: 'Empty System',
    description: 'Clean system with no documents or sources',
  },
  new_user_setup: {
    name: 'New User Setup',
    description: 'Fresh user account with default labels',
  },
  active_system: {
    name: 'Active System',
    description: 'Normal system with active processing',
  },
  system_under_load: {
    name: 'System Under Load',
    description: 'Heavy usage with many documents and processing',
  },
  multi_user_system: {
    name: 'Multi-User System',
    description: 'Multiple users with their own documents and sources',
  },
  problematic_system: {
    name: 'Problematic System',
    description: 'System with failed OCR and sync errors',
  },
} as const

/**
 * Create a minimal dataset for performance testing
 */
export const createMinimalDataset = (): MockState => ({
  documents: createMockDocuments(5),
  users: [getDefaultTestUser()],
  sources: createMockSources(1),
  labels: createDefaultLabels().slice(0, 3),
  searchResults: new Map(),
  syncProgress: new Map(),
  queueStats: createMockQueueStats({ 
    pending_count: 0, 
    processing_count: 0, 
    failed_count: 0 
  }),
  scenarios: {},
})

/**
 * Create a comprehensive dataset for integration testing
 */
export const createIntegrationDataset = (): MockState => {
  const dataset = createCompleteTestDataset()
  
  // Add comprehensive scenario data
  dataset.scenarios = {
    documents: createDocumentScenarios(),
    users: createUserScenarios(),
    sources: createSourceScenarios(),
    labels: createLabelScenarios(),
    search: createSearchScenarios(),
    queue: createQueueScenarios(),
  }

  return dataset
}