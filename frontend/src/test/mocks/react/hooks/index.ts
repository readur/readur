/**
 * React Hooks - Custom testing hooks for mock functionality
 * Centralized exports for all custom React hooks
 */

export { 
  useMockScenario,
  type ScenarioConfig,
  type UseTestScenarioReturn,
  type ScenarioPreview 
} from './useMockScenario'

export { 
  useMockNetworkCondition,
  type UseMockNetworkConditionReturn,
  type NetworkProfile,
  type NetworkTestResult,
  type NetworkStats 
} from './useMockNetworkCondition'

export { 
  useMockUser,
  type UseMockUserReturn,
  type UserProfile,
  type UserPreferences,
  type UserStatistics,
  type SessionInfo,
  type UserType,
  type UserAction,
  type ActivityRecord,
  type UserInsights 
} from './useMockUser'

export { 
  useMockDocuments,
  type UseMockDocumentsReturn,
  type DocumentFilter,
  type DocumentSort,
  type PaginationOptions,
  type DocumentTestScenario,
  type DocumentStatistics,
  type UsageMetrics 
} from './useMockDocuments'

export { 
  useMockSearch,
  type UseMockSearchReturn,
  type SearchQuery,
  type SearchFilters,
  type SearchSort,
  type SearchSuggestion,
  type SearchMetrics,
  type FullTextOptions,
  type SearchAnalytics,
  type QueryStats,
  type TrendData,
  type BenchmarkResults,
  type ValidationReport,
  type SearchConfig 
} from './useMockSearch'

export { 
  useMockUpload,
  type UseMockUploadReturn,
  type UploadFile,
  type UploadOptions,
  type FileValidationResult,
  type UploadStatistics,
  type ThroughputMetrics,
  type BenchmarkResult 
} from './useMockUpload'

// Combined hook for comprehensive testing
import { useMockScenario } from './useMockScenario'
import { useMockNetworkCondition } from './useMockNetworkCondition'
import { useMockUser } from './useMockUser'
import { useMockDocuments } from './useMockDocuments'
import { useMockSearch } from './useMockSearch'
import { useMockUpload } from './useMockUpload'

export interface ComprehensiveTestingHooks {
  scenario: ReturnType<typeof useMockScenario>
  network: ReturnType<typeof useMockNetworkCondition>
  user: ReturnType<typeof useMockUser>
  documents: ReturnType<typeof useMockDocuments>
  search: ReturnType<typeof useMockSearch>
  upload: ReturnType<typeof useMockUpload>
}

/**
 * useMockTestingKit - Comprehensive testing hook
 * Combines all testing hooks for complete test scenario management
 */
export const useMockTestingKit = (): ComprehensiveTestingHooks => {
  const scenario = useMockScenario({
    includeAuth: true,
    includeNotifications: true,
    includeWebSocket: true,
    resetOnChange: true,
  })
  
  const network = useMockNetworkCondition()
  const user = useMockUser()
  const documents = useMockDocuments()
  const search = useMockSearch()
  const upload = useMockUpload()

  return {
    scenario,
    network,
    user,
    documents,
    search,
    upload,
  }
}

// Quick setup functions for common testing patterns
export const useQuickTestSetup = () => {
  const kit = useMockTestingKit()

  const setupEmptyState = async () => {
    await kit.scenario.switchToEmpty()
    kit.user.logout()
    kit.documents.clearAllDocuments()
    kit.search.clearResults()
    kit.upload.resetUploads()
  }

  const setupActiveUser = async () => {
    await kit.scenario.switchToActive()
    await kit.user.simulateExperiencedUser()
    kit.network.setCondition('fast')
  }

  const setupSlowNetwork = async () => {
    kit.network.setCondition('slow')
    kit.network.simulateGradualDegradation(5000)
  }

  const setupErrorState = async () => {
    await kit.scenario.switchToProblematic()
    kit.network.simulateIntermittentConnection(3000)
  }

  const setupUploadTest = async (fileCount: number = 5) => {
    await setupActiveUser()
    
    // Create mock files
    const files = Array.from({ length: fileCount }, (_, i) => 
      new File([`test content ${i}`], `test-file-${i}.pdf`, { type: 'application/pdf' })
    )
    
    kit.upload.addFiles(files)
    return files
  }

  const setupSearchTest = async (documentCount: number = 20) => {
    await kit.documents.generateTestDocuments({
      name: 'Search Test',
      count: documentCount,
      withOcr: true,
    })
    
    return kit.documents.documents
  }

  return {
    ...kit,
    setupEmptyState,
    setupActiveUser,
    setupSlowNetwork,
    setupErrorState,
    setupUploadTest,
    setupSearchTest,
  }
}

// Performance testing utilities
export const usePerformanceTesting = () => {
  const kit = useMockTestingKit()

  const runDocumentPerformanceTest = async (documentCount: number) => {
    const startTime = performance.now()
    
    await kit.documents.generateTestDocuments({
      name: 'Performance Test',
      count: documentCount,
    })
    
    const creationTime = performance.now() - startTime
    
    // Test search performance
    const searchStart = performance.now()
    await kit.search.enhancedSearch('test')
    const searchTime = performance.now() - searchStart
    
    return {
      documentCount,
      creationTime,
      searchTime,
      memoryUsage: (performance as any).memory?.usedJSHeapSize || 0,
    }
  }

  const runUploadPerformanceTest = async (fileCount: number, fileSizeMB: number = 1) => {
    return kit.upload.stressTestUpload(fileCount, fileSizeMB)
  }

  const runNetworkPerformanceTest = async () => {
    const conditions = ['fast', 'slow', 'realistic'] as const
    const results: any[] = []
    
    for (const condition of conditions) {
      kit.network.setCondition(condition)
      const testResult = await kit.network.simulateSpeedTest()
      results.push({ condition, ...testResult })
    }
    
    return results
  }

  return {
    runDocumentPerformanceTest,
    runUploadPerformanceTest,
    runNetworkPerformanceTest,
  }
}

// Scenario automation utilities
export const useScenarioAutomation = () => {
  const kit = useMockTestingKit()

  const runUserJourney = async (journeyName: string) => {
    switch (journeyName) {
      case 'new_user_onboarding':
        await kit.user.simulateNewUser()
        await kit.scenario.switchToEmpty()
        // Simulate first upload
        const files = await kit.upload.simulateLargeFileUpload(0.1) // 100MB
        await kit.documents.simulateUploadProcess('welcome-document.pdf')
        break
        
      case 'power_user_workflow':
        await kit.user.simulateExperiencedUser()
        await kit.scenario.switchToActive()
        // Simulate bulk operations
        await kit.documents.addMultipleDocuments(50)
        await kit.search.enhancedSearch('invoice')
        await kit.upload.stressTestUpload(10, 5)
        break
        
      case 'error_recovery':
        await kit.scenario.switchToProblematic()
        kit.network.simulateIntermittentConnection(2000)
        await kit.upload.simulateNetworkInterruption('test-file', 3000)
        kit.network.simulateNetworkRecovery(5000)
        break
    }
  }

  const runDemoScenario = async () => {
    // Quick demo showing various features
    await kit.scenario.queueScenarioTransition([
      'EMPTY_SYSTEM',
      'NEW_USER_SETUP', 
      'ACTIVE_SYSTEM',
      'SYSTEM_UNDER_LOAD'
    ], 5000)
  }

  return {
    runUserJourney,
    runDemoScenario,
  }
}