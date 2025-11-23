/**
 * Async Operation Testing Patterns - Comprehensive async testing utilities
 * Provides reusable patterns for testing async operations with realistic scenarios
 */

import { waitFor, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import type { EnhancedRenderResult } from '../render'

export interface AsyncOperationConfig {
  triggerSelector?: string
  triggerTestId?: string
  loadingIndicatorSelector?: string
  loadingText?: string | RegExp
  successSelector?: string
  successText?: string | RegExp
  errorSelector?: string
  errorText?: string | RegExp
  timeout?: number
  expectedDuration?: number
  retryable?: boolean
  maxRetries?: number
}

export interface AsyncTestResult {
  success: boolean
  duration: number
  loadingDetected: boolean
  successDetected: boolean
  errorDetected: boolean
  retryCount: number
  errors: string[]
  warnings: string[]
  performanceMetrics: {
    initialRenderTime: number
    loadingStateTime: number
    completionTime: number
    totalTime: number
  }
}

export interface LoadingStateTest {
  hasLoadingIndicator: boolean
  hasProgressBar: boolean
  hasSkeletonLoader: boolean
  hasSpinner: boolean
  showsProgressPercentage: boolean
  disablesInteraction: boolean
}

/**
 * Comprehensive async operation tester
 */
export class AsyncOperationTester {
  private renderResult: EnhancedRenderResult
  private config: AsyncOperationConfig
  private user: ReturnType<typeof userEvent.setup>
  private startTime: number = 0
  private loadingStartTime: number = 0
  private completionTime: number = 0

  constructor(renderResult: EnhancedRenderResult, config: AsyncOperationConfig) {
    this.renderResult = renderResult
    this.config = {
      timeout: 10000,
      expectedDuration: 2000,
      retryable: false,
      maxRetries: 3,
      loadingText: /loading|processing|submitting/i,
      successText: /success|complete|done/i,
      errorText: /error|failed|wrong/i,
      ...config,
    }
    this.user = userEvent.setup()
  }

  /**
   * Test complete async operation workflow
   */
  async testAsyncOperation(): Promise<AsyncTestResult> {
    const errors: string[] = []
    const warnings: string[] = []
    let retryCount = 0
    
    this.startTime = performance.now()
    const initialRenderTime = this.startTime

    try {
      // 1. Test loading state
      const loadingResult = await this.testLoadingState(errors, warnings)

      // 2. Test completion (success or error)
      const completionResult = await this.testCompletion(errors, warnings)

      // 3. Test retries if applicable
      if (this.config.retryable && completionResult.errorDetected) {
        const retryResult = await this.testRetryMechanism(errors, warnings)
        retryCount = retryResult.retryCount
      }

      const totalTime = performance.now() - this.startTime

      return {
        success: errors.length === 0,
        duration: totalTime,
        loadingDetected: loadingResult.loadingDetected,
        successDetected: completionResult.successDetected,
        errorDetected: completionResult.errorDetected,
        retryCount,
        errors,
        warnings,
        performanceMetrics: {
          initialRenderTime: this.loadingStartTime - initialRenderTime,
          loadingStateTime: this.completionTime - this.loadingStartTime,
          completionTime: this.completionTime - this.startTime,
          totalTime,
        },
      }
    } catch (error) {
      errors.push(`Async operation test failed: ${error}`)
      return {
        success: false,
        duration: performance.now() - this.startTime,
        loadingDetected: false,
        successDetected: false,
        errorDetected: true,
        retryCount,
        errors,
        warnings,
        performanceMetrics: {
          initialRenderTime: 0,
          loadingStateTime: 0,
          completionTime: 0,
          totalTime: performance.now() - this.startTime,
        },
      }
    }
  }

  /**
   * Test loading state behavior
   */
  private async testLoadingState(
    errors: string[],
    warnings: string[]
  ): Promise<{ loadingDetected: boolean }> {
    // Trigger the async operation
    await this.triggerAsyncOperation()
    this.loadingStartTime = performance.now()

    // Check for loading indicators
    const loadingChecks = await this.checkLoadingIndicators()
    
    if (!loadingChecks.hasLoadingIndicator && !loadingChecks.hasSpinner) {
      warnings.push('No loading indicator detected during async operation')
    }

    if (!loadingChecks.disablesInteraction) {
      warnings.push('UI does not prevent interaction during loading')
    }

    // Wait for loading to complete or timeout
    try {
      await waitFor(
        () => {
          const loadingElement = this.findLoadingElement()
          if (loadingElement) {
            throw new Error('Still loading')
          }
        },
        { timeout: this.config.timeout }
      )
      this.completionTime = performance.now()
    } catch (error) {
      errors.push('Async operation timed out')
      this.completionTime = performance.now()
    }

    return { loadingDetected: loadingChecks.hasLoadingIndicator || loadingChecks.hasSpinner }
  }

  /**
   * Test operation completion (success or error)
   */
  private async testCompletion(
    errors: string[],
    warnings: string[]
  ): Promise<{ successDetected: boolean; errorDetected: boolean }> {
    let successDetected = false
    let errorDetected = false

    // Check for success indicators
    if (this.config.successSelector || this.config.successText) {
      const successElement = this.config.successSelector
        ? this.renderResult.container.querySelector(this.config.successSelector)
        : screen.queryByText(this.config.successText!)

      if (successElement) {
        successDetected = true
      }
    }

    // Check for error indicators
    if (this.config.errorSelector || this.config.errorText) {
      const errorElement = this.config.errorSelector
        ? this.renderResult.container.querySelector(this.config.errorSelector)
        : screen.queryByText(this.config.errorText!)

      if (errorElement) {
        errorDetected = true
      }
    }

    // Validate completion state
    if (!successDetected && !errorDetected) {
      warnings.push('No clear success or error state detected after async operation')
    }

    // Check operation duration
    const actualDuration = this.completionTime - this.loadingStartTime
    if (this.config.expectedDuration && Math.abs(actualDuration - this.config.expectedDuration) > 1000) {
      warnings.push(
        `Operation duration ${actualDuration.toFixed(0)}ms differs significantly from expected ${this.config.expectedDuration}ms`
      )
    }

    return { successDetected, errorDetected }
  }

  /**
   * Test retry mechanism
   */
  private async testRetryMechanism(
    errors: string[],
    warnings: string[]
  ): Promise<{ retryCount: number }> {
    let retryCount = 0
    const maxRetries = this.config.maxRetries || 3

    while (retryCount < maxRetries) {
      // Look for retry button
      const retryButton = screen.queryByText(/retry|try again/i) ||
                         screen.queryByRole('button', { name: /retry|try again/i })

      if (!retryButton) {
        if (retryCount === 0) {
          warnings.push('No retry mechanism found after error')
        }
        break
      }

      // Click retry button
      await this.user.click(retryButton)
      retryCount++

      // Wait for operation to complete
      try {
        await waitFor(
          () => {
            const loadingElement = this.findLoadingElement()
            if (loadingElement) {
              throw new Error('Still loading')
            }
          },
          { timeout: this.config.timeout }
        )

        // Check if retry was successful
        const successElement = this.config.successSelector
          ? this.renderResult.container.querySelector(this.config.successSelector)
          : screen.queryByText(this.config.successText!)

        if (successElement) {
          break // Retry successful
        }
      } catch (error) {
        warnings.push(`Retry attempt ${retryCount} timed out`)
      }
    }

    if (retryCount >= maxRetries) {
      warnings.push('Maximum retry attempts reached without success')
    }

    return { retryCount }
  }

  /**
   * Trigger the async operation
   */
  private async triggerAsyncOperation(): Promise<void> {
    let triggerElement: HTMLElement | null = null

    if (this.config.triggerTestId) {
      triggerElement = this.renderResult.getByTestId(this.config.triggerTestId)
    } else if (this.config.triggerSelector) {
      triggerElement = this.renderResult.container.querySelector(this.config.triggerSelector)
    } else {
      // Try to find common trigger elements
      triggerElement = 
        this.renderResult.container.querySelector('button[type="submit"]') ||
        this.renderResult.container.querySelector('button') ||
        this.renderResult.container.querySelector('[role="button"]')
    }

    if (!triggerElement) {
      throw new Error('No trigger element found for async operation')
    }

    await this.user.click(triggerElement)
  }

  /**
   * Check for various loading indicators
   */
  private async checkLoadingIndicators(): Promise<LoadingStateTest> {
    // Give a moment for loading state to appear
    await new Promise(resolve => setTimeout(resolve, 50))

    const hasLoadingIndicator = !!this.findLoadingElement()
    const hasProgressBar = !!this.renderResult.container.querySelector('[role="progressbar"], .progress, progress')
    const hasSkeletonLoader = !!this.renderResult.container.querySelector('.skeleton, [data-testid*="skeleton"]')
    const hasSpinner = !!this.renderResult.container.querySelector('.spinner, .loading-spinner, [data-testid*="spinner"]')
    
    // Check for progress percentage
    const progressText = screen.queryByText(/\d+%/) || screen.queryByText(/\d+\/\d+/)
    const showsProgressPercentage = !!progressText

    // Check if interaction is disabled during loading
    const buttons = this.renderResult.container.querySelectorAll('button:not([disabled])')
    const inputs = this.renderResult.container.querySelectorAll('input:not([disabled])')
    const disablesInteraction = buttons.length === 0 && inputs.length === 0

    return {
      hasLoadingIndicator,
      hasProgressBar,
      hasSkeletonLoader,
      hasSpinner,
      showsProgressPercentage,
      disablesInteraction,
    }
  }

  /**
   * Find loading element using various strategies
   */
  private findLoadingElement(): HTMLElement | null {
    // Try loading indicator selector first
    if (this.config.loadingIndicatorSelector) {
      const element = this.renderResult.container.querySelector(this.config.loadingIndicatorSelector)
      if (element) return element as HTMLElement
    }

    // Try loading text
    if (this.config.loadingText) {
      const element = screen.queryByText(this.config.loadingText)
      if (element) return element
    }

    // Try common loading indicators
    const selectors = [
      '[data-testid*="loading"]',
      '[data-testid*="spinner"]',
      '.loading',
      '.spinner',
      '[role="progressbar"]',
      '.progress',
    ]

    for (const selector of selectors) {
      const element = this.renderResult.container.querySelector(selector)
      if (element) return element as HTMLElement
    }

    return null
  }
}

/**
 * Quick async operation testing utilities
 */
export const testAsyncOperation = async (
  renderResult: EnhancedRenderResult,
  config: AsyncOperationConfig
): Promise<AsyncTestResult> => {
  const tester = new AsyncOperationTester(renderResult, config)
  return await tester.testAsyncOperation()
}

/**
 * Test loading states specifically
 */
export const testLoadingStates = async (
  renderResult: EnhancedRenderResult,
  triggerConfig: Pick<AsyncOperationConfig, 'triggerSelector' | 'triggerTestId'>
): Promise<LoadingStateTest> => {
  const tester = new AsyncOperationTester(renderResult, triggerConfig)
  
  await tester['triggerAsyncOperation']()
  return await tester['checkLoadingIndicators']()
}

/**
 * Test error handling patterns
 */
export const testErrorHandling = async (
  renderResult: EnhancedRenderResult,
  config: AsyncOperationConfig & {
    simulateError?: () => void
    expectRetry?: boolean
  }
): Promise<{ errorHandled: boolean; retryAvailable: boolean; errors: string[] }> => {
  const errors: string[] = []
  const warnings: string[] = []

  // Simulate error if function provided
  if (config.simulateError) {
    config.simulateError()
  }

  const tester = new AsyncOperationTester(renderResult, config)
  
  try {
    await tester['triggerAsyncOperation']()
    
    // Wait for error to appear
    await waitFor(
      () => {
        const errorElement = config.errorSelector
          ? renderResult.container.querySelector(config.errorSelector)
          : screen.queryByText(config.errorText || /error|failed/i)
        
        if (!errorElement) {
          throw new Error('Error not yet displayed')
        }
      },
      { timeout: config.timeout || 5000 }
    )

    const errorHandled = true
    
    // Check for retry mechanism
    const retryButton = screen.queryByText(/retry|try again/i)
    const retryAvailable = !!retryButton

    if (config.expectRetry && !retryAvailable) {
      errors.push('Expected retry mechanism not found')
    }

    return { errorHandled, retryAvailable, errors }
  } catch (error) {
    errors.push(`Error handling test failed: ${error}`)
    return { errorHandled: false, retryAvailable: false, errors }
  }
}

/**
 * Test timeout scenarios
 */
export const testTimeoutHandling = async (
  renderResult: EnhancedRenderResult,
  config: AsyncOperationConfig & {
    simulateTimeout?: () => void
    expectedTimeoutMessage?: string | RegExp
  }
): Promise<{ timeoutHandled: boolean; errors: string[] }> => {
  const errors: string[] = []

  // Simulate timeout if function provided
  if (config.simulateTimeout) {
    config.simulateTimeout()
  }

  const tester = new AsyncOperationTester(renderResult, {
    ...config,
    timeout: 2000, // Short timeout for testing
  })

  try {
    const result = await tester.testAsyncOperation()
    
    if (result.success) {
      errors.push('Operation should have timed out but succeeded')
      return { timeoutHandled: false, errors }
    }

    // Check for timeout message
    const timeoutMessage = config.expectedTimeoutMessage || /timeout|took too long/i
    const timeoutElement = screen.queryByText(timeoutMessage)
    
    if (!timeoutElement) {
      errors.push('Timeout message not displayed')
    }

    return { timeoutHandled: !!timeoutElement, errors }
  } catch (error) {
    errors.push(`Timeout handling test failed: ${error}`)
    return { timeoutHandled: false, errors }
  }
}

/**
 * Test concurrent async operations
 */
export const testConcurrentOperations = async (
  renderResult: EnhancedRenderResult,
  configs: AsyncOperationConfig[]
): Promise<{
  allCompleted: boolean
  completionOrder: number[]
  errors: string[]
}> => {
  const errors: string[] = []
  const completionTimes: number[] = []
  const user = userEvent.setup()

  try {
    const startTime = performance.now()
    
    // Trigger all operations simultaneously
    const triggerPromises = configs.map(async (config, index) => {
      let triggerElement: HTMLElement | null = null

      if (config.triggerTestId) {
        triggerElement = renderResult.getByTestId(config.triggerTestId)
      } else if (config.triggerSelector) {
        triggerElement = renderResult.container.querySelector(config.triggerSelector)
      }

      if (!triggerElement) {
        errors.push(`Trigger element not found for operation ${index}`)
        return
      }

      await user.click(triggerElement)
    })

    await Promise.all(triggerPromises)

    // Wait for all operations to complete
    const completionPromises = configs.map(async (config, index) => {
      try {
        await waitFor(
          () => {
            const successElement = config.successSelector
              ? renderResult.container.querySelector(config.successSelector)
              : screen.queryByText(config.successText || /success/i)
            
            const errorElement = config.errorSelector
              ? renderResult.container.querySelector(config.errorSelector)
              : screen.queryByText(config.errorText || /error/i)

            if (!successElement && !errorElement) {
              throw new Error(`Operation ${index} not yet completed`)
            }
          },
          { timeout: config.timeout || 10000 }
        )
        
        completionTimes[index] = performance.now() - startTime
      } catch (error) {
        errors.push(`Operation ${index} failed to complete: ${error}`)
      }
    })

    await Promise.allSettled(completionPromises)

    // Determine completion order
    const completionOrder = completionTimes
      .map((time, index) => ({ time, index }))
      .filter(({ time }) => time > 0)
      .sort((a, b) => a.time - b.time)
      .map(({ index }) => index)

    return {
      allCompleted: completionOrder.length === configs.length,
      completionOrder,
      errors,
    }
  } catch (error) {
    errors.push(`Concurrent operations test failed: ${error}`)
    return {
      allCompleted: false,
      completionOrder: [],
      errors,
    }
  }
}

/**
 * Common async operation configurations
 */
export const commonAsyncConfigs = {
  fileUpload: (triggerTestId: string): AsyncOperationConfig => ({
    triggerTestId,
    loadingText: /uploading|processing/i,
    successText: /uploaded|complete/i,
    errorText: /upload failed|error/i,
    expectedDuration: 2000,
    retryable: true,
  }),

  formSubmission: (triggerTestId: string): AsyncOperationConfig => ({
    triggerTestId,
    loadingText: /submitting|saving/i,
    successText: /saved|success|submitted/i,
    errorText: /submission failed|error/i,
    expectedDuration: 1000,
    retryable: false,
  }),

  dataFetch: (triggerTestId: string): AsyncOperationConfig => ({
    triggerTestId,
    loadingText: /loading|fetching/i,
    successSelector: '[data-testid="data-loaded"]',
    errorText: /failed to load|error/i,
    expectedDuration: 500,
    retryable: true,
  }),

  search: (triggerTestId: string): AsyncOperationConfig => ({
    triggerTestId,
    loadingText: /searching/i,
    successSelector: '[data-testid="search-results"]',
    errorText: /search failed|no results/i,
    expectedDuration: 300,
    retryable: true,
  }),

  deletion: (triggerTestId: string): AsyncOperationConfig => ({
    triggerTestId,
    loadingText: /deleting|removing/i,
    successText: /deleted|removed/i,
    errorText: /delete failed|error/i,
    expectedDuration: 500,
    retryable: false,
  }),
}