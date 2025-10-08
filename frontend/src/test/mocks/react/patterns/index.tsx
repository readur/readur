/**
 * Testing Patterns - Comprehensive testing pattern library
 * Centralized exports for all testing patterns and utilities
 */

export {
  FormTester,
  testForm,
  testFormField,
  testFormValidation,
  commonFormFields,
  type FormTestConfig,
  type FormFieldConfig,
  type FormTestResult,
} from './form-patterns'

export {
  AsyncOperationTester,
  testAsyncOperation,
  testLoadingStates,
  testErrorHandling,
  testTimeoutHandling,
  testConcurrentOperations,
  commonAsyncConfigs,
  type AsyncOperationConfig,
  type AsyncTestResult,
  type LoadingStateTest,
} from './async-patterns'

export {
  ErrorTester,
  testErrorHandling as testError,
  testErrorBoundary,
  testNetworkError,
  testValidationError,
  testPermissionError,
  commonErrorConfigs,
  type ErrorTestConfig,
  type ErrorTestResult,
  type ErrorBoundaryTest,
} from './error-patterns'

export {
  PerformanceTester,
  testComponentPerformance,
  testRenderPerformance,
  testMemoryUsage,
  testHookPerformance,
  analyzeBundleSize,
  commonPerformanceConfigs,
  type PerformanceTestConfig,
  type PerformanceTestResult,
  type PerformanceMetrics,
  type PerformanceThresholds,
} from './performance-patterns'

// Combined testing utilities
import type { EnhancedRenderResult } from '../render'
import { FormTester, type FormTestConfig } from './form-patterns'
import { AsyncOperationTester, type AsyncOperationConfig } from './async-patterns'
import { ErrorTester, type ErrorTestConfig } from './error-patterns'
import { PerformanceTester, type PerformanceTestConfig } from './performance-patterns'

export interface ComprehensiveTestConfig {
  form?: FormTestConfig
  async?: AsyncOperationConfig[]
  error?: ErrorTestConfig[]
  performance?: PerformanceTestConfig
  runAll?: boolean
  parallel?: boolean
}

export interface ComprehensiveTestResult {
  success: boolean
  duration: number
  results: {
    form?: any
    async?: any[]
    error?: any[]
    performance?: any
  }
  summary: {
    totalTests: number
    passedTests: number
    failedTests: number
    warnings: string[]
    criticalIssues: string[]
  }
}

/**
 * Comprehensive testing orchestrator
 */
export class ComprehensiveTester {
  private renderResult: EnhancedRenderResult
  private config: ComprehensiveTestConfig

  constructor(renderResult: EnhancedRenderResult, config: ComprehensiveTestConfig) {
    this.renderResult = renderResult
    this.config = config
  }

  /**
   * Run all configured tests
   */
  async runAllTests(): Promise<ComprehensiveTestResult> {
    const startTime = performance.now()
    const results: any = {}
    const warnings: string[] = []
    const criticalIssues: string[] = []
    let totalTests = 0
    let passedTests = 0

    try {
      if (this.config.parallel) {
        // Run tests in parallel where possible
        const promises = []

        if (this.config.form) {
          promises.push(this.runFormTest())
          totalTests++
        }

        if (this.config.performance) {
          promises.push(this.runPerformanceTest())
          totalTests++
        }

        const parallelResults = await Promise.allSettled(promises)
        
        parallelResults.forEach((result, index) => {
          if (result.status === 'fulfilled') {
            passedTests++
            if (index === 0 && this.config.form) results.form = result.value
            if (index === 1 && this.config.performance) results.performance = result.value
          } else {
            criticalIssues.push(`Parallel test failed: ${result.reason}`)
          }
        })

        // Run sequential tests (async and error tests need to be sequential)
        if (this.config.async) {
          const asyncResults = []
          for (const asyncConfig of this.config.async) {
            try {
              const result = await this.runAsyncTest(asyncConfig)
              asyncResults.push(result)
              if (result.success) passedTests++
              totalTests++
            } catch (error) {
              criticalIssues.push(`Async test failed: ${error}`)
              totalTests++
            }
          }
          results.async = asyncResults
        }

        if (this.config.error) {
          const errorResults = []
          for (const errorConfig of this.config.error) {
            try {
              const result = await this.runErrorTest(errorConfig)
              errorResults.push(result)
              if (result.success) passedTests++
              totalTests++
            } catch (error) {
              criticalIssues.push(`Error test failed: ${error}`)
              totalTests++
            }
          }
          results.error = errorResults
        }

      } else {
        // Run tests sequentially
        if (this.config.form) {
          try {
            results.form = await this.runFormTest()
            if (results.form.success) passedTests++
            totalTests++
          } catch (error) {
            criticalIssues.push(`Form test failed: ${error}`)
            totalTests++
          }
        }

        if (this.config.async) {
          const asyncResults = []
          for (const asyncConfig of this.config.async) {
            try {
              const result = await this.runAsyncTest(asyncConfig)
              asyncResults.push(result)
              if (result.success) passedTests++
              totalTests++
            } catch (error) {
              criticalIssues.push(`Async test failed: ${error}`)
              totalTests++
            }
          }
          results.async = asyncResults
        }

        if (this.config.error) {
          const errorResults = []
          for (const errorConfig of this.config.error) {
            try {
              const result = await this.runErrorTest(errorConfig)
              errorResults.push(result)
              if (result.success) passedTests++
              totalTests++
            } catch (error) {
              criticalIssues.push(`Error test failed: ${error}`)
              totalTests++
            }
          }
          results.error = errorResults
        }

        if (this.config.performance) {
          try {
            results.performance = await this.runPerformanceTest()
            if (results.performance.success) passedTests++
            totalTests++
          } catch (error) {
            criticalIssues.push(`Performance test failed: ${error}`)
            totalTests++
          }
        }
      }

      // Collect warnings from all test results
      Object.values(results).forEach((result: any) => {
        if (Array.isArray(result)) {
          result.forEach(r => {
            if (r.warnings) warnings.push(...r.warnings)
          })
        } else if (result?.warnings) {
          warnings.push(...result.warnings)
        }
      })

      const duration = performance.now() - startTime

      return {
        success: criticalIssues.length === 0 && passedTests === totalTests,
        duration,
        results,
        summary: {
          totalTests,
          passedTests,
          failedTests: totalTests - passedTests,
          warnings,
          criticalIssues,
        },
      }

    } catch (error) {
      criticalIssues.push(`Test orchestration failed: ${error}`)
      
      return {
        success: false,
        duration: performance.now() - startTime,
        results,
        summary: {
          totalTests,
          passedTests,
          failedTests: totalTests - passedTests,
          warnings,
          criticalIssues,
        },
      }
    }
  }

  private async runFormTest() {
    const tester = new FormTester(this.renderResult, this.config.form!)
    return await tester.testFormWorkflow()
  }

  private async runAsyncTest(config: AsyncOperationConfig) {
    const tester = new AsyncOperationTester(this.renderResult, config)
    return await tester.testAsyncOperation()
  }

  private async runErrorTest(config: ErrorTestConfig) {
    const tester = new ErrorTester(this.renderResult, config)
    return await tester.testErrorHandling()
  }

  private async runPerformanceTest() {
    const tester = new PerformanceTester(this.renderResult, this.config.performance!)
    return await tester.runPerformanceTest()
  }
}

/**
 * Quick comprehensive testing function
 */
export const runComprehensiveTests = async (
  renderResult: EnhancedRenderResult,
  config: ComprehensiveTestConfig
): Promise<ComprehensiveTestResult> => {
  const tester = new ComprehensiveTester(renderResult, config)
  return await tester.runAllTests()
}

/**
 * Pre-configured test suites for common scenarios
 */
export const testSuites = {
  /**
   * Basic component validation suite
   */
  basic: (renderResult: EnhancedRenderResult): Promise<ComprehensiveTestResult> =>
    runComprehensiveTests(renderResult, {
      performance: {
        name: 'Basic Performance',
        iterations: 50,
        measurementTypes: ['render'],
        thresholds: { maxRenderTime: 100 },
      },
    }),

  /**
   * Form component test suite
   */
  form: (renderResult: EnhancedRenderResult, formConfig: FormTestConfig): Promise<ComprehensiveTestResult> =>
    runComprehensiveTests(renderResult, {
      form: formConfig,
      error: [
        {
          errorType: 'validation',
          triggerMethod: 'interaction',
          expectedErrorDisplay: 'inline',
          recoveryMechanism: 'none',
        },
      ],
      performance: {
        name: 'Form Performance',
        iterations: 30,
        measurementTypes: ['render', 'interaction'],
        thresholds: { maxRenderTime: 50, maxInteractionTime: 16 },
      },
    }),

  /**
   * Interactive component test suite
   */
  interactive: (renderResult: EnhancedRenderResult): Promise<ComprehensiveTestResult> =>
    runComprehensiveTests(renderResult, {
      async: [
        {
          triggerSelector: 'button',
          loadingText: /loading/i,
          successText: /success/i,
          expectedDuration: 1000,
        },
      ],
      error: [
        {
          errorType: 'async',
          triggerMethod: 'interaction',
          expectedErrorDisplay: 'toast',
          recoveryMechanism: 'retry',
        },
      ],
      performance: {
        name: 'Interactive Performance',
        iterations: 100,
        measurementTypes: ['render', 'update', 'interaction'],
        thresholds: {
          maxRenderTime: 50,
          maxUpdateTime: 30,
          maxInteractionTime: 16,
        },
      },
    }),

  /**
   * Data-heavy component test suite
   */
  dataHeavy: (renderResult: EnhancedRenderResult): Promise<ComprehensiveTestResult> =>
    runComprehensiveTests(renderResult, {
      async: [
        {
          triggerSelector: '[data-testid="load-data"]',
          loadingText: /loading|fetching/i,
          successSelector: '[data-testid="data-loaded"]',
          expectedDuration: 500,
        },
      ],
      performance: {
        name: 'Data Heavy Performance',
        iterations: 100,
        measurementTypes: ['render', 'memory'],
        thresholds: {
          maxRenderTime: 100,
          maxMemoryUsage: 100 * 1024 * 1024, // 100MB
        },
      },
    }),

  /**
   * Critical component test suite (comprehensive)
   */
  critical: (renderResult: EnhancedRenderResult, config: Partial<ComprehensiveTestConfig> = {}): Promise<ComprehensiveTestResult> =>
    runComprehensiveTests(renderResult, {
      async: [
        {
          triggerSelector: 'button',
          loadingText: /loading/i,
          successText: /success/i,
          errorText: /error/i,
          expectedDuration: 1000,
          retryable: true,
        },
      ],
      error: [
        {
          errorType: 'render',
          triggerMethod: 'immediate',
          expectedErrorDisplay: 'boundary',
          recoveryMechanism: 'retry',
        },
        {
          errorType: 'async',
          triggerMethod: 'interaction',
          expectedErrorDisplay: 'toast',
          recoveryMechanism: 'retry',
        },
      ],
      performance: {
        name: 'Critical Performance',
        iterations: 200,
        warmupIterations: 20,
        measurementTypes: ['render', 'update', 'interaction', 'memory'],
        thresholds: {
          maxRenderTime: 50,
          maxUpdateTime: 30,
          maxInteractionTime: 16,
          maxMemoryUsage: 50 * 1024 * 1024,
          minFPS: 30,
        },
      },
      parallel: true,
      ...config,
    }),
}

/**
 * Test pattern builder for creating custom test configurations
 */
export class TestPatternBuilder {
  private config: ComprehensiveTestConfig = {}

  static create(): TestPatternBuilder {
    return new TestPatternBuilder()
  }

  withForm(formConfig: FormTestConfig): TestPatternBuilder {
    this.config.form = formConfig
    return this
  }

  withAsync(asyncConfigs: AsyncOperationConfig[]): TestPatternBuilder {
    this.config.async = asyncConfigs
    return this
  }

  withError(errorConfigs: ErrorTestConfig[]): TestPatternBuilder {
    this.config.error = errorConfigs
    return this
  }

  withPerformance(performanceConfig: PerformanceTestConfig): TestPatternBuilder {
    this.config.performance = performanceConfig
    return this
  }

  parallel(enabled: boolean = true): TestPatternBuilder {
    this.config.parallel = enabled
    return this
  }

  build(): ComprehensiveTestConfig {
    return this.config
  }

  async run(renderResult: EnhancedRenderResult): Promise<ComprehensiveTestResult> {
    return runComprehensiveTests(renderResult, this.config)
  }
}

// Utility functions for quick pattern creation
export const createFormTestPattern = (fields: any[], submitExpected: 'success' | 'error' = 'success') =>
  TestPatternBuilder.create()
    .withForm({
      fields,
      submitExpectedResult: submitExpected,
      enableAccessibilityChecks: true,
    })
    .withPerformance({
      name: 'Form Performance',
      iterations: 50,
      measurementTypes: ['render', 'interaction'],
    })

export const createAsyncTestPattern = (configs: AsyncOperationConfig[]) =>
  TestPatternBuilder.create()
    .withAsync(configs)
    .withError([
      {
        errorType: 'async',
        triggerMethod: 'async',
        expectedErrorDisplay: 'toast',
        recoveryMechanism: 'retry',
      },
    ])
    .withPerformance({
      name: 'Async Performance',
      iterations: 30,
      measurementTypes: ['render', 'interaction'],
    })

export const createErrorTestPattern = (errorConfigs: ErrorTestConfig[]) =>
  TestPatternBuilder.create()
    .withError(errorConfigs)
    .withPerformance({
      name: 'Error Handling Performance',
      iterations: 20,
      measurementTypes: ['render'],
    })