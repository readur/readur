/**
 * Form Testing Patterns - Comprehensive form testing utilities
 * Provides reusable patterns for testing forms with modern React practices
 */

import { screen, fireEvent, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import type { EnhancedRenderResult } from '../render'

export interface FormTestConfig {
  formTestId?: string
  submitButtonTestId?: string
  resetButtonTestId?: string
  fields: FormFieldConfig[]
  validationMode?: 'onChange' | 'onBlur' | 'onSubmit'
  submitExpectedResult?: 'success' | 'error' | 'loading'
  enableAccessibilityChecks?: boolean
}

export interface FormFieldConfig {
  testId: string
  type: 'text' | 'email' | 'password' | 'number' | 'select' | 'checkbox' | 'radio' | 'textarea' | 'file'
  label?: string
  required?: boolean
  validation?: {
    pattern?: RegExp
    minLength?: number
    maxLength?: number
    min?: number
    max?: number
    custom?: (value: any) => string | null
  }
  initialValue?: any
  testValues?: {
    valid: any[]
    invalid: any[]
  }
}

export interface FormTestResult {
  success: boolean
  errors: string[]
  warnings: string[]
  performanceMetrics: {
    renderTime: number
    validationTime: number
    submissionTime: number
  }
  accessibilityScore?: number
}

/**
 * Comprehensive form testing pattern
 */
export class FormTester {
  private renderResult: EnhancedRenderResult
  private config: FormTestConfig
  private user: ReturnType<typeof userEvent.setup>
  private startTime: number = 0

  constructor(renderResult: EnhancedRenderResult, config: FormTestConfig) {
    this.renderResult = renderResult
    this.config = config
    this.user = userEvent.setup()
  }

  /**
   * Test complete form workflow
   */
  async testFormWorkflow(): Promise<FormTestResult> {
    const errors: string[] = []
    const warnings: string[] = []
    const startTime = performance.now()

    try {
      // 1. Test form rendering
      await this.testFormRendering(errors)

      // 2. Test field interactions
      await this.testFieldInteractions(errors, warnings)

      // 3. Test validation
      await this.testFormValidation(errors)

      // 4. Test submission
      await this.testFormSubmission(errors)

      // 5. Test accessibility
      let accessibilityScore: number | undefined
      if (this.config.enableAccessibilityChecks) {
        accessibilityScore = await this.testAccessibility(errors, warnings)
      }

      const totalTime = performance.now() - startTime

      return {
        success: errors.length === 0,
        errors,
        warnings,
        performanceMetrics: {
          renderTime: totalTime,
          validationTime: 0, // Would be measured during validation tests
          submissionTime: 0, // Would be measured during submission tests
        },
        accessibilityScore,
      }
    } catch (error) {
      errors.push(`Form test failed: ${error}`)
      return {
        success: false,
        errors,
        warnings,
        performanceMetrics: {
          renderTime: performance.now() - startTime,
          validationTime: 0,
          submissionTime: 0,
        },
      }
    }
  }

  /**
   * Test form rendering and structure
   */
  private async testFormRendering(errors: string[]): Promise<void> {
    // Check form element exists
    const formElement = this.config.formTestId 
      ? this.renderResult.queryByTestId(this.config.formTestId)
      : this.renderResult.container.querySelector('form')

    if (!formElement) {
      errors.push('Form element not found')
      return
    }

    // Check all required fields are present
    for (const field of this.config.fields) {
      const fieldElement = this.renderResult.queryByTestId(field.testId)
      if (!fieldElement) {
        errors.push(`Field with testId "${field.testId}" not found`)
      }
    }

    // Check submit button exists
    if (this.config.submitButtonTestId) {
      const submitButton = this.renderResult.queryByTestId(this.config.submitButtonTestId)
      if (!submitButton) {
        errors.push('Submit button not found')
      }
    }
  }

  /**
   * Test individual field interactions
   */
  private async testFieldInteractions(errors: string[], warnings: string[]): Promise<void> {
    for (const field of this.config.fields) {
      try {
        await this.testFieldInteraction(field, errors, warnings)
      } catch (error) {
        errors.push(`Field interaction test failed for ${field.testId}: ${error}`)
      }
    }
  }

  private async testFieldInteraction(
    field: FormFieldConfig, 
    errors: string[], 
    warnings: string[]
  ): Promise<void> {
    const fieldElement = this.renderResult.getByTestId(field.testId)

    switch (field.type) {
      case 'text':
      case 'email':
      case 'password':
      case 'textarea':
        await this.testTextFieldInteraction(fieldElement, field, errors, warnings)
        break
      case 'number':
        await this.testNumberFieldInteraction(fieldElement, field, errors, warnings)
        break
      case 'select':
        await this.testSelectFieldInteraction(fieldElement, field, errors, warnings)
        break
      case 'checkbox':
        await this.testCheckboxInteraction(fieldElement, field, errors, warnings)
        break
      case 'radio':
        await this.testRadioInteraction(fieldElement, field, errors, warnings)
        break
      case 'file':
        await this.testFileFieldInteraction(fieldElement, field, errors, warnings)
        break
    }
  }

  private async testTextFieldInteraction(
    element: HTMLElement, 
    field: FormFieldConfig, 
    errors: string[], 
    warnings: string[]
  ): Promise<void> {
    const input = element as HTMLInputElement

    // Test typing
    await this.user.clear(input)
    await this.user.type(input, 'test value')
    
    if (input.value !== 'test value') {
      errors.push(`Text input ${field.testId} does not update value correctly`)
    }

    // Test clearing
    await this.user.clear(input)
    if (input.value !== '') {
      warnings.push(`Text input ${field.testId} does not clear properly`)
    }

    // Test valid values if provided
    if (field.testValues?.valid) {
      for (const validValue of field.testValues.valid) {
        await this.user.clear(input)
        await this.user.type(input, validValue)
        
        // Check for validation errors (should not appear)
        await waitFor(() => {
          const errorElement = screen.queryByText(new RegExp(`${field.label}.*error`, 'i'))
          if (errorElement) {
            errors.push(`Valid value "${validValue}" shows validation error`)
          }
        })
      }
    }

    // Test invalid values if provided
    if (field.testValues?.invalid) {
      for (const invalidValue of field.testValues.invalid) {
        await this.user.clear(input)
        await this.user.type(input, invalidValue)
        
        // Trigger validation based on mode
        if (this.config.validationMode === 'onBlur') {
          fireEvent.blur(input)
        }
        
        // Check for validation errors (should appear)
        await waitFor(() => {
          const errorElement = screen.queryByText(new RegExp(`${field.label}.*error`, 'i'))
          if (!errorElement) {
            warnings.push(`Invalid value "${invalidValue}" does not show validation error`)
          }
        })
      }
    }
  }

  private async testNumberFieldInteraction(
    element: HTMLElement, 
    field: FormFieldConfig, 
    errors: string[], 
    warnings: string[]
  ): Promise<void> {
    const input = element as HTMLInputElement

    // Test numeric input
    await this.user.clear(input)
    await this.user.type(input, '123')
    
    if (input.value !== '123') {
      errors.push(`Number input ${field.testId} does not accept numeric values`)
    }

    // Test non-numeric input
    await this.user.clear(input)
    await this.user.type(input, 'abc')
    
    // Many number inputs prevent non-numeric input, so this might be empty
    if (input.value === 'abc') {
      warnings.push(`Number input ${field.testId} accepts non-numeric values`)
    }

    // Test min/max validation if configured
    if (field.validation?.min !== undefined) {
      await this.user.clear(input)
      await this.user.type(input, (field.validation.min - 1).toString())
      
      if (this.config.validationMode === 'onBlur') {
        fireEvent.blur(input)
      }
      
      await waitFor(() => {
        const errorElement = screen.queryByText(new RegExp('minimum', 'i'))
        if (!errorElement) {
          warnings.push(`Minimum validation not working for ${field.testId}`)
        }
      })
    }
  }

  private async testSelectFieldInteraction(
    element: HTMLElement, 
    field: FormFieldConfig, 
    errors: string[], 
    warnings: string[]
  ): Promise<void> {
    const select = element as HTMLSelectElement

    // Test selecting first option
    const options = select.querySelectorAll('option')
    if (options.length < 2) {
      warnings.push(`Select ${field.testId} has insufficient options`)
      return
    }

    // Select second option (first is usually placeholder)
    await this.user.selectOptions(select, options[1].value)
    
    if (select.value !== options[1].value) {
      errors.push(`Select ${field.testId} does not update selected value`)
    }
  }

  private async testCheckboxInteraction(
    element: HTMLElement, 
    field: FormFieldConfig, 
    errors: string[], 
    warnings: string[]
  ): Promise<void> {
    const checkbox = element as HTMLInputElement

    // Test checking
    await this.user.click(checkbox)
    if (!checkbox.checked) {
      errors.push(`Checkbox ${field.testId} does not check properly`)
    }

    // Test unchecking
    await this.user.click(checkbox)
    if (checkbox.checked) {
      errors.push(`Checkbox ${field.testId} does not uncheck properly`)
    }
  }

  private async testRadioInteraction(
    element: HTMLElement, 
    field: FormFieldConfig, 
    errors: string[], 
    warnings: string[]
  ): Promise<void> {
    const radio = element as HTMLInputElement

    // Test selecting radio button
    await this.user.click(radio)
    if (!radio.checked) {
      errors.push(`Radio button ${field.testId} does not select properly`)
    }
  }

  private async testFileFieldInteraction(
    element: HTMLElement, 
    field: FormFieldConfig, 
    errors: string[], 
    warnings: string[]
  ): Promise<void> {
    const fileInput = element as HTMLInputElement

    // Create a test file
    const testFile = new File(['test content'], 'test.txt', { type: 'text/plain' })

    // Test file selection
    await this.user.upload(fileInput, testFile)
    
    if (!fileInput.files || fileInput.files.length === 0) {
      errors.push(`File input ${field.testId} does not accept files`)
    } else if (fileInput.files[0].name !== 'test.txt') {
      errors.push(`File input ${field.testId} does not store file correctly`)
    }
  }

  /**
   * Test form validation
   */
  private async testFormValidation(errors: string[]): Promise<void> {
    this.startTime = performance.now()

    // Test required field validation
    for (const field of this.config.fields.filter(f => f.required)) {
      const fieldElement = this.renderResult.getByTestId(field.testId)
      
      // Clear the field
      if (field.type === 'text' || field.type === 'email' || field.type === 'password') {
        await this.user.clear(fieldElement as HTMLInputElement)
      }

      // Trigger validation
      if (this.config.validationMode === 'onBlur') {
        fireEvent.blur(fieldElement)
      } else if (this.config.validationMode === 'onSubmit') {
        // Will be tested in submission
        continue
      }

      // Check for required field error
      await waitFor(() => {
        const errorElement = screen.queryByText(new RegExp('required', 'i'))
        if (!errorElement) {
          errors.push(`Required validation not working for ${field.testId}`)
        }
      })
    }

    // Test pattern validation
    for (const field of this.config.fields.filter(f => f.validation?.pattern)) {
      const fieldElement = this.renderResult.getByTestId(field.testId) as HTMLInputElement
      
      await this.user.clear(fieldElement)
      await this.user.type(fieldElement, 'invalid-pattern-value')
      
      if (this.config.validationMode === 'onBlur') {
        fireEvent.blur(fieldElement)
      }
      
      await waitFor(() => {
        const errorElement = screen.queryByText(new RegExp('invalid', 'i'))
        if (!errorElement) {
          errors.push(`Pattern validation not working for ${field.testId}`)
        }
      })
    }
  }

  /**
   * Test form submission
   */
  private async testFormSubmission(errors: string[]): Promise<void> {
    this.startTime = performance.now()

    // Fill form with valid data
    await this.fillFormWithValidData()

    // Find and click submit button
    const submitButton = this.config.submitButtonTestId 
      ? this.renderResult.getByTestId(this.config.submitButtonTestId)
      : this.renderResult.container.querySelector('button[type="submit"]') ||
        this.renderResult.container.querySelector('input[type="submit"]')

    if (!submitButton) {
      errors.push('Submit button not found')
      return
    }

    // Submit form
    await this.user.click(submitButton)

    // Check expected result
    switch (this.config.submitExpectedResult) {
      case 'success':
        await waitFor(() => {
          const successElement = screen.queryByText(new RegExp('success', 'i'))
          if (!successElement) {
            errors.push('Success message not shown after form submission')
          }
        })
        break
      case 'error':
        await waitFor(() => {
          const errorElement = screen.queryByText(new RegExp('error', 'i'))
          if (!errorElement) {
            errors.push('Error message not shown when expected')
          }
        })
        break
      case 'loading':
        const loadingElement = screen.queryByText(new RegExp('loading|submitting', 'i'))
        if (!loadingElement) {
          errors.push('Loading state not shown during submission')
        }
        break
    }
  }

  /**
   * Fill form with valid test data
   */
  private async fillFormWithValidData(): Promise<void> {
    for (const field of this.config.fields) {
      const fieldElement = this.renderResult.getByTestId(field.testId)
      
      let testValue = field.initialValue

      if (!testValue && field.testValues?.valid?.length) {
        testValue = field.testValues.valid[0]
      }

      if (!testValue) {
        // Generate default test values based on field type
        switch (field.type) {
          case 'text':
            testValue = 'Test Value'
            break
          case 'email':
            testValue = 'test@example.com'
            break
          case 'password':
            testValue = 'TestPassword123!'
            break
          case 'number':
            testValue = '123'
            break
          case 'checkbox':
            if (field.required) {
              await this.user.click(fieldElement)
            }
            continue
          case 'select':
            const select = fieldElement as HTMLSelectElement
            const options = select.querySelectorAll('option')
            if (options.length > 1) {
              await this.user.selectOptions(select, options[1].value)
            }
            continue
          case 'file':
            const file = new File(['test'], 'test.txt', { type: 'text/plain' })
            await this.user.upload(fieldElement, file)
            continue
        }
      }

      if (testValue && (field.type === 'text' || field.type === 'email' || field.type === 'password' || field.type === 'textarea')) {
        await this.user.clear(fieldElement as HTMLInputElement)
        await this.user.type(fieldElement, testValue)
      }
    }
  }

  /**
   * Test form accessibility
   */
  private async testAccessibility(errors: string[], warnings: string[]): Promise<number> {
    const report = this.renderResult.getAccessibilityReport()
    
    errors.push(...report.issues)
    warnings.push(...report.warnings)
    
    return report.score
  }
}

/**
 * Quick form testing utilities
 */
export const testForm = async (
  renderResult: EnhancedRenderResult,
  config: FormTestConfig
): Promise<FormTestResult> => {
  const tester = new FormTester(renderResult, config)
  return await tester.testFormWorkflow()
}

export const testFormField = async (
  renderResult: EnhancedRenderResult,
  fieldConfig: FormFieldConfig
): Promise<{ success: boolean; errors: string[] }> => {
  const tester = new FormTester(renderResult, {
    fields: [fieldConfig],
    validationMode: 'onChange',
  })
  
  const errors: string[] = []
  const warnings: string[] = []
  
  await tester['testFieldInteraction'](fieldConfig, errors, warnings)
  
  return {
    success: errors.length === 0,
    errors,
  }
}

export const testFormValidation = async (
  renderResult: EnhancedRenderResult,
  fields: FormFieldConfig[],
  validationMode: 'onChange' | 'onBlur' | 'onSubmit' = 'onBlur'
): Promise<{ success: boolean; errors: string[] }> => {
  const tester = new FormTester(renderResult, {
    fields,
    validationMode,
  })
  
  const errors: string[] = []
  await tester['testFormValidation'](errors)
  
  return {
    success: errors.length === 0,
    errors,
  }
}

// Common form field configurations
export const commonFormFields = {
  email: (testId: string, required = true): FormFieldConfig => ({
    testId,
    type: 'email',
    label: 'Email',
    required,
    validation: {
      pattern: /^[^\s@]+@[^\s@]+\.[^\s@]+$/,
    },
    testValues: {
      valid: ['test@example.com', 'user@domain.co.uk'],
      invalid: ['invalid-email', 'test@', '@domain.com'],
    },
  }),

  password: (testId: string, required = true): FormFieldConfig => ({
    testId,
    type: 'password',
    label: 'Password',
    required,
    validation: {
      minLength: 8,
      pattern: /^(?=.*[a-z])(?=.*[A-Z])(?=.*\d)(?=.*[@$!%*?&])[A-Za-z\d@$!%*?&]/, // Strong password
    },
    testValues: {
      valid: ['Password123!', 'MySecure@Pass1'],
      invalid: ['weak', '12345678', 'PASSWORD'],
    },
  }),

  text: (testId: string, label: string, required = false): FormFieldConfig => ({
    testId,
    type: 'text',
    label,
    required,
    testValues: {
      valid: ['Valid text', 'Another valid value'],
      invalid: [], // Text fields usually accept any value
    },
  }),

  number: (testId: string, label: string, min?: number, max?: number): FormFieldConfig => ({
    testId,
    type: 'number',
    label,
    validation: { min, max },
    testValues: {
      valid: [
        min !== undefined ? min.toString() : '0',
        max !== undefined ? max.toString() : '100',
      ],
      invalid: [
        min !== undefined ? (min - 1).toString() : '-1',
        max !== undefined ? (max + 1).toString() : '1000',
      ],
    },
  }),
}