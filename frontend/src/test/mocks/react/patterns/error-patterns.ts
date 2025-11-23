/**
 * Error Handling Testing Patterns - Comprehensive error testing utilities
 * Provides reusable patterns for testing error boundaries, error states, and error recovery
 */

import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import type { EnhancedRenderResult } from '../render'

export interface ErrorTestConfig {
  errorType: 'render' | 'async' | 'network' | 'validation' | 'permission' | 'timeout' | 'custom'
  triggerMethod: 'immediate' | 'interaction' | 'async' | 'prop_change'
  triggerSelector?: string
  triggerTestId?: string
  errorMessage?: string
  expectedErrorDisplay?: 'boundary' | 'inline' | 'toast' | 'modal' | 'console'
  recoveryMechanism?: 'retry' | 'refresh' | 'fallback' | 'redirect' | 'none'
  shouldReportError?: boolean
  timeout?: number
}

export interface ErrorTestResult {
  errorTriggered: boolean
  errorDisplayed: boolean
  errorCaught: boolean
  recoveryAvailable: boolean
  errorReported: boolean
  userCanRecover: boolean
  accessibilityCompliant: boolean
  errors: string[]
  warnings: string[]
  errorDetails: {
    errorMessage: string | null
    errorType: string | null
    stackTrace: string | null
    componentStack: string | null
    errorBoundaryTriggered: boolean
    fallbackDisplayed: boolean
  }
}

export interface ErrorBoundaryTest {
  catchesErrors: boolean
  displaysErrorUI: boolean
  providesRetryMechanism: boolean
  logsErrors: boolean
  reportsErrors: boolean
  isolatesError: boolean
}

/**
 * Comprehensive error testing class
 */
export class ErrorTester {
  private renderResult: EnhancedRenderResult
  private config: ErrorTestConfig
  private user: ReturnType<typeof userEvent.setup>
  private errorLog: any[] = []
  private originalConsoleError: typeof console.error

  constructor(renderResult: EnhancedRenderResult, config: ErrorTestConfig) {
    this.renderResult = renderResult
    this.config = {
      timeout: 5000,
      expectedErrorDisplay: 'boundary',
      recoveryMechanism: 'retry',
      shouldReportError: true,
      ...config,
    }
    this.user = userEvent.setup()
    this.originalConsoleError = console.error
    this.setupErrorCapture()
  }

  /**
   * Test complete error handling workflow
   */
  async testErrorHandling(): Promise<ErrorTestResult> {
    const errors: string[] = []
    const warnings: string[] = []
    let errorDetails = {
      errorMessage: null as string | null,
      errorType: null as string | null,
      stackTrace: null as string | null,
      componentStack: null as string | null,
      errorBoundaryTriggered: false,
      fallbackDisplayed: false,
    }

    try {
      // 1. Trigger the error
      const triggerResult = await this.triggerError()
      
      // 2. Check error detection
      const detectionResult = await this.checkErrorDetection(errors, warnings)
      errorDetails = { ...errorDetails, ...detectionResult.errorDetails }

      // 3. Check error display
      const displayResult = await this.checkErrorDisplay(errors, warnings)

      // 4. Check recovery mechanisms
      const recoveryResult = await this.checkRecoveryMechanisms(errors, warnings)

      // 5. Check error reporting
      const reportingResult = await this.checkErrorReporting(errors, warnings)

      // 6. Check accessibility
      const accessibilityResult = await this.checkErrorAccessibility(errors, warnings)

      this.restoreErrorCapture()

      return {
        errorTriggered: triggerResult.triggered,
        errorDisplayed: displayResult.displayed,
        errorCaught: detectionResult.caught,
        recoveryAvailable: recoveryResult.available,
        errorReported: reportingResult.reported,
        userCanRecover: recoveryResult.userCanRecover,
        accessibilityCompliant: accessibilityResult.compliant,
        errors,
        warnings,
        errorDetails,
      }
    } catch (testError) {
      this.restoreErrorCapture()
      errors.push(`Error test execution failed: ${testError}`)
      return {
        errorTriggered: false,
        errorDisplayed: false,
        errorCaught: false,
        recoveryAvailable: false,
        errorReported: false,
        userCanRecover: false,
        accessibilityCompliant: false,
        errors,
        warnings,
        errorDetails,
      }
    }
  }

  /**
   * Trigger error based on configuration
   */
  private async triggerError(): Promise<{ triggered: boolean }> {
    switch (this.config.triggerMethod) {
      case 'immediate':
        return this.triggerImmediateError()
      case 'interaction':
        return this.triggerInteractionError()
      case 'async':
        return this.triggerAsyncError()
      case 'prop_change':
        return this.triggerPropChangeError()
      default:
        throw new Error(`Unknown trigger method: ${this.config.triggerMethod}`)
    }
  }

  private async triggerImmediateError(): Promise<{ triggered: boolean }> {
    // For immediate errors, they should already be triggered during render
    return { triggered: true }
  }

  private async triggerInteractionError(): Promise<{ triggered: boolean }> {
    let triggerElement: HTMLElement | null = null

    if (this.config.triggerTestId) {
      triggerElement = this.renderResult.getByTestId(this.config.triggerTestId)
    } else if (this.config.triggerSelector) {
      triggerElement = this.renderResult.container.querySelector(this.config.triggerSelector)
    } else {
      // Try to find common interactive elements
      triggerElement = 
        this.renderResult.container.querySelector('button[data-error-trigger]') ||
        this.renderResult.container.querySelector('[data-testid*="error"]') ||
        this.renderResult.container.querySelector('button')
    }

    if (!triggerElement) {
      throw new Error('No trigger element found for interaction error')
    }

    await this.user.click(triggerElement)
    return { triggered: true }
  }

  private async triggerAsyncError(): Promise<{ triggered: boolean }> {
    // Trigger async operation that will fail
    await this.triggerInteractionError()
    
    // Wait for async error to manifest
    await new Promise(resolve => setTimeout(resolve, 100))
    return { triggered: true }
  }

  private async triggerPropChangeError(): Promise<{ triggered: boolean }> {
    // This would require re-rendering with error-inducing props
    // Implementation depends on specific test setup
    return { triggered: true }
  }

  /**
   * Check if error was detected/caught
   */
  private async checkErrorDetection(
    errors: string[], 
    warnings: string[]
  ): Promise<{ caught: boolean; errorDetails: any }> {
    let errorCaught = false
    let errorDetails = {
      errorMessage: null as string | null,
      errorType: null as string | null,
      stackTrace: null as string | null,
      componentStack: null as string | null,
      errorBoundaryTriggered: false,
      fallbackDisplayed: false,
    }

    // Check console errors
    if (this.errorLog.length > 0) {
      errorCaught = true
      const lastError = this.errorLog[this.errorLog.length - 1]
      errorDetails.errorMessage = lastError.message
      errorDetails.errorType = lastError.name || this.config.errorType
      errorDetails.stackTrace = lastError.stack
    }

    // Check for error boundary activation
    const errorBoundaryElement = this.renderResult.container.querySelector('[data-testid="error-boundary-fallback"]')
    if (errorBoundaryElement) {
      errorCaught = true
      errorDetails.errorBoundaryTriggered = true
      errorDetails.fallbackDisplayed = true
    }

    // Check for error state in components
    const errorStateElement = screen.queryByText(/error|failed|wrong/i) ||
                             this.renderResult.container.querySelector('[data-error="true"]') ||
                             this.renderResult.container.querySelector('.error-state')

    if (errorStateElement) {
      errorCaught = true
    }

    if (!errorCaught) {
      warnings.push('Expected error was not detected or caught')
    }

    return { caught: errorCaught, errorDetails }
  }

  /**
   * Check error display mechanisms
   */
  private async checkErrorDisplay(
    errors: string[], 
    warnings: string[]
  ): Promise<{ displayed: boolean }> {
    let errorDisplayed = false

    switch (this.config.expectedErrorDisplay) {
      case 'boundary':
        errorDisplayed = await this.checkErrorBoundaryDisplay(errors, warnings)
        break
      case 'inline':
        errorDisplayed = await this.checkInlineErrorDisplay(errors, warnings)
        break
      case 'toast':
        errorDisplayed = await this.checkToastErrorDisplay(errors, warnings)
        break
      case 'modal':
        errorDisplayed = await this.checkModalErrorDisplay(errors, warnings)
        break
      case 'console':
        errorDisplayed = this.errorLog.length > 0
        break
    }

    if (!errorDisplayed) {
      errors.push(`Expected error display type '${this.config.expectedErrorDisplay}' not found`)
    }

    return { displayed: errorDisplayed }
  }

  private async checkErrorBoundaryDisplay(errors: string[], warnings: string[]): Promise<boolean> {
    const errorBoundary = this.renderResult.container.querySelector('[data-testid="error-boundary-fallback"]')
    
    if (!errorBoundary) {
      return false
    }

    // Check error boundary content
    const hasErrorMessage = errorBoundary.textContent?.includes('error') || 
                           errorBoundary.textContent?.includes('wrong')
    
    if (!hasErrorMessage) {
      warnings.push('Error boundary displayed but without clear error message')
    }

    // Check for error ID or tracking
    const hasErrorId = errorBoundary.querySelector('[data-error-id]') ||
                      errorBoundary.textContent?.match(/error.?id|reference/i)
    
    if (!hasErrorId) {
      warnings.push('Error boundary does not display error tracking information')
    }

    return true
  }

  private async checkInlineErrorDisplay(errors: string[], warnings: string[]): Promise<boolean> {
    const inlineError = this.renderResult.container.querySelector('.error-message, .field-error, [data-testid*="error"]')
    
    if (!inlineError) {
      return false
    }

    // Check error message content
    if (!inlineError.textContent?.trim()) {
      warnings.push('Inline error element found but without error text')
    }

    return true
  }

  private async checkToastErrorDisplay(errors: string[], warnings: string[]): Promise<boolean> {
    const toastError = this.renderResult.container.querySelector('[data-testid*="notification"], .toast, .alert') ||
                      screen.queryByRole('alert')
    
    if (!toastError) {
      return false
    }

    // Check if toast contains error information
    const hasErrorContent = toastError.textContent?.toLowerCase().includes('error') ||
                           toastError.classList.contains('error') ||
                           toastError.getAttribute('data-type') === 'error'
    
    if (!hasErrorContent) {
      warnings.push('Toast notification found but does not appear to be an error notification')
    }

    return true
  }

  private async checkModalErrorDisplay(errors: string[], warnings: string[]): Promise<boolean> {
    const modalError = this.renderResult.container.querySelector('[role="dialog"], .modal, [data-testid*="modal"]')
    
    if (!modalError) {
      return false
    }

    // Check modal accessibility
    const hasTitle = modalError.querySelector('[role="heading"], h1, h2, h3') ||
                    modalError.getAttribute('aria-labelledby')
    
    if (!hasTitle) {
      warnings.push('Error modal lacks proper heading or aria-labelledby')
    }

    const hasFocusManagement = modalError.querySelector('[autofocus], [data-focus]')
    if (!hasFocusManagement) {
      warnings.push('Error modal may not manage focus properly')
    }

    return true
  }

  /**
   * Check recovery mechanisms
   */
  private async checkRecoveryMechanisms(
    errors: string[], 
    warnings: string[]
  ): Promise<{ available: boolean; userCanRecover: boolean }> {
    let recoveryAvailable = false
    let userCanRecover = false

    switch (this.config.recoveryMechanism) {
      case 'retry':
        const retryResult = await this.checkRetryMechanism(errors, warnings)
        recoveryAvailable = retryResult.available
        userCanRecover = retryResult.functional
        break
      case 'refresh':
        const refreshResult = await this.checkRefreshMechanism(errors, warnings)
        recoveryAvailable = refreshResult.available
        userCanRecover = refreshResult.functional
        break
      case 'fallback':
        const fallbackResult = await this.checkFallbackMechanism(errors, warnings)
        recoveryAvailable = fallbackResult.available
        userCanRecover = fallbackResult.functional
        break
      case 'redirect':
        const redirectResult = await this.checkRedirectMechanism(errors, warnings)
        recoveryAvailable = redirectResult.available
        userCanRecover = redirectResult.functional
        break
      case 'none':
        recoveryAvailable = true // No recovery expected
        userCanRecover = false
        break
    }

    return { available: recoveryAvailable, userCanRecover }
  }

  private async checkRetryMechanism(
    errors: string[], 
    warnings: string[]
  ): Promise<{ available: boolean; functional: boolean }> {
    const retryButton = screen.queryByText(/retry|try again/i) ||
                       screen.queryByRole('button', { name: /retry|try again/i }) ||
                       this.renderResult.container.querySelector('[data-testid*="retry"]')

    if (!retryButton) {
      return { available: false, functional: false }
    }

    // Test retry functionality
    try {
      await this.user.click(retryButton)
      
      // Check if retry action was triggered (loading state, etc.)
      const loadingIndicator = screen.queryByText(/retrying|loading/i) ||
                              this.renderResult.container.querySelector('[data-testid*="loading"]')
      
      if (loadingIndicator) {
        return { available: true, functional: true }
      } else {
        warnings.push('Retry button found but does not show loading state when clicked')
        return { available: true, functional: false }
      }
    } catch (error) {
      errors.push(`Retry mechanism failed: ${error}`)
      return { available: true, functional: false }
    }
  }

  private async checkRefreshMechanism(
    errors: string[], 
    warnings: string[]
  ): Promise<{ available: boolean; functional: boolean }> {
    const refreshButton = screen.queryByText(/refresh|reload/i) ||
                         screen.queryByRole('button', { name: /refresh|reload/i }) ||
                         this.renderResult.container.querySelector('[data-testid*="refresh"]')

    if (!refreshButton) {
      return { available: false, functional: false }
    }

    // Note: Testing actual page refresh in unit tests is complex
    // We check if the button exists and is clickable
    const isClickable = !refreshButton.hasAttribute('disabled')
    
    if (!isClickable) {
      warnings.push('Refresh button found but is disabled')
    }

    return { available: true, functional: isClickable }
  }

  private async checkFallbackMechanism(
    errors: string[], 
    warnings: string[]
  ): Promise<{ available: boolean; functional: boolean }> {
    const fallbackContent = this.renderResult.container.querySelector('[data-testid*="fallback"], .fallback-content') ||
                           screen.queryByText(/fallback|alternative/i)

    if (!fallbackContent) {
      return { available: false, functional: false }
    }

    // Check if fallback provides useful information or functionality
    const hasUsefulContent = fallbackContent.textContent?.length && fallbackContent.textContent.length > 10
    const hasInteractiveElements = fallbackContent.querySelectorAll('button, a, input').length > 0

    if (!hasUsefulContent && !hasInteractiveElements) {
      warnings.push('Fallback content found but appears empty or non-functional')
      return { available: true, functional: false }
    }

    return { available: true, functional: true }
  }

  private async checkRedirectMechanism(
    errors: string[], 
    warnings: string[]
  ): Promise<{ available: boolean; functional: boolean }> {
    const redirectLink = this.renderResult.container.querySelector('a[href], [data-testid*="redirect"]') ||
                        screen.queryByText(/go back|home|dashboard/i)

    if (!redirectLink) {
      return { available: false, functional: false }
    }

    // Check if redirect link has valid href or click handler
    const hasHref = redirectLink.getAttribute('href')
    const hasClickHandler = redirectLink.getAttribute('onclick') || redirectLink.hasAttribute('data-testid')

    if (!hasHref && !hasClickHandler) {
      warnings.push('Redirect element found but lacks href or click handler')
      return { available: true, functional: false }
    }

    return { available: true, functional: true }
  }

  /**
   * Check error reporting
   */
  private async checkErrorReporting(
    errors: string[], 
    warnings: string[]
  ): Promise<{ reported: boolean }> {
    if (!this.config.shouldReportError) {
      return { reported: true } // Not expected to report
    }

    // Check if errors were logged
    const errorLogged = this.errorLog.length > 0

    // Check for error reporting indicators
    const reportingIndicator = this.renderResult.container.querySelector('[data-error-reported]') ||
                              screen.queryByText(/error.*(reported|logged|tracked)/i)

    const reported = errorLogged || !!reportingIndicator

    if (!reported) {
      warnings.push('Error reporting expected but not detected')
    }

    return { reported }
  }

  /**
   * Check error accessibility
   */
  private async checkErrorAccessibility(
    errors: string[], 
    warnings: string[]
  ): Promise<{ compliant: boolean }> {
    const accessibilityReport = this.renderResult.getAccessibilityReport()
    
    // Check for ARIA live regions for error announcements
    const liveRegion = this.renderResult.container.querySelector('[aria-live], [role="alert"]')
    if (!liveRegion) {
      warnings.push('No ARIA live region found for error announcements')
    }

    // Check error focus management
    const errorElement = this.renderResult.container.querySelector('[data-testid*="error"], .error')
    if (errorElement) {
      const isFocusable = errorElement.hasAttribute('tabindex') || 
                         errorElement.matches('button, input, select, textarea, a[href]')
      
      if (!isFocusable && this.config.expectedErrorDisplay === 'modal') {
        warnings.push('Error display should be focusable for screen readers')
      }
    }

    // Check color contrast (simplified check)
    const errorElements = this.renderResult.container.querySelectorAll('.error, [data-error="true"]')
    errorElements.forEach(element => {
      const styles = window.getComputedStyle(element)
      const backgroundColor = styles.backgroundColor
      const color = styles.color
      
      // Basic check - red text on white background should have good contrast
      if (color.includes('rgb(255, 0, 0)') && backgroundColor.includes('rgb(255, 255, 255)')) {
        warnings.push('Error text color may not have sufficient contrast')
      }
    })

    const compliant = accessibilityReport.score > 80 && warnings.length === 0

    return { compliant }
  }

  /**
   * Setup error capture
   */
  private setupErrorCapture(): void {
    this.errorLog = []
    console.error = (...args: any[]) => {
      this.errorLog.push({
        message: args[0]?.toString() || 'Unknown error',
        name: args[0]?.name || 'Error',
        stack: args[0]?.stack,
        timestamp: Date.now(),
        args,
      })
      // Still call original console.error for debugging
      this.originalConsoleError(...args)
    }
  }

  private restoreErrorCapture(): void {
    console.error = this.originalConsoleError
  }
}

/**
 * Quick error testing utilities
 */
export const testErrorHandling = async (
  renderResult: EnhancedRenderResult,
  config: ErrorTestConfig
): Promise<ErrorTestResult> => {
  const tester = new ErrorTester(renderResult, config)
  return await tester.testErrorHandling()
}

export const testErrorBoundary = async (
  renderResult: EnhancedRenderResult,
  triggerConfig: Pick<ErrorTestConfig, 'triggerMethod' | 'triggerSelector' | 'triggerTestId'>
): Promise<ErrorBoundaryTest> => {
  const config: ErrorTestConfig = {
    errorType: 'render',
    expectedErrorDisplay: 'boundary',
    recoveryMechanism: 'retry',
    ...triggerConfig,
  }

  const result = await testErrorHandling(renderResult, config)

  return {
    catchesErrors: result.errorCaught,
    displaysErrorUI: result.errorDisplayed,
    providesRetryMechanism: result.recoveryAvailable,
    logsErrors: result.errorReported,
    reportsErrors: result.errorReported,
    isolatesError: result.errorDetails.errorBoundaryTriggered,
  }
}

/**
 * Test specific error scenarios
 */
export const testNetworkError = async (
  renderResult: EnhancedRenderResult,
  triggerConfig: Pick<ErrorTestConfig, 'triggerSelector' | 'triggerTestId'>
): Promise<ErrorTestResult> => {
  return testErrorHandling(renderResult, {
    errorType: 'network',
    triggerMethod: 'interaction',
    expectedErrorDisplay: 'toast',
    recoveryMechanism: 'retry',
    ...triggerConfig,
  })
}

export const testValidationError = async (
  renderResult: EnhancedRenderResult,
  fieldSelector: string
): Promise<ErrorTestResult> => {
  return testErrorHandling(renderResult, {
    errorType: 'validation',
    triggerMethod: 'interaction',
    triggerSelector: fieldSelector,
    expectedErrorDisplay: 'inline',
    recoveryMechanism: 'none',
  })
}

export const testPermissionError = async (
  renderResult: EnhancedRenderResult,
  triggerConfig: Pick<ErrorTestConfig, 'triggerSelector' | 'triggerTestId'>
): Promise<ErrorTestResult> => {
  return testErrorHandling(renderResult, {
    errorType: 'permission',
    triggerMethod: 'interaction',
    expectedErrorDisplay: 'modal',
    recoveryMechanism: 'redirect',
    ...triggerConfig,
  })
}

/**
 * Common error configurations
 */
export const commonErrorConfigs = {
  renderError: (triggerTestId: string): ErrorTestConfig => ({
    errorType: 'render',
    triggerMethod: 'immediate',
    triggerTestId,
    expectedErrorDisplay: 'boundary',
    recoveryMechanism: 'retry',
  }),

  asyncError: (triggerTestId: string): ErrorTestConfig => ({
    errorType: 'async',
    triggerMethod: 'async',
    triggerTestId,
    expectedErrorDisplay: 'toast',
    recoveryMechanism: 'retry',
  }),

  formValidationError: (fieldTestId: string): ErrorTestConfig => ({
    errorType: 'validation',
    triggerMethod: 'interaction',
    triggerTestId: fieldTestId,
    expectedErrorDisplay: 'inline',
    recoveryMechanism: 'none',
  }),

  networkTimeoutError: (triggerTestId: string): ErrorTestConfig => ({
    errorType: 'timeout',
    triggerMethod: 'async',
    triggerTestId,
    expectedErrorDisplay: 'toast',
    recoveryMechanism: 'retry',
  }),

  unauthorizedError: (triggerTestId: string): ErrorTestConfig => ({
    errorType: 'permission',
    triggerMethod: 'interaction',
    triggerTestId,
    expectedErrorDisplay: 'modal',
    recoveryMechanism: 'redirect',
  }),
}