import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import { setupMockApi, createMockDocument, mockApi } from './mocks'

describe('Mock API Framework Verification', () => {
  let cleanup: () => void

  beforeAll(async () => {
    const setup = await setupMockApi({
      scenario: 'ACTIVE_SYSTEM',
      networkCondition: 'fast',
    })
    cleanup = setup.cleanup
  })

  afterAll(() => {
    if (cleanup) cleanup()
  })

  it('should have mock API quick setup methods', () => {
    expect(mockApi.quick).toBeDefined()
    expect(mockApi.withAuth).toBeDefined()
    expect(mockApi.empty).toBeDefined()
    expect(mockApi.slow).toBeDefined()
    expect(mockApi.offline).toBeDefined()
  })

  it('should create mock documents', () => {
    const doc = createMockDocument()
    expect(doc).toHaveProperty('id')
    expect(doc).toHaveProperty('filename')
    expect(doc).toHaveProperty('mime_type')
    expect(doc).toHaveProperty('created_at')
  })

  it('should support different scenarios', async () => {
    // Test switching scenarios
    await mockApi.empty()
    await mockApi.withAuth()
    await mockApi.slow()
    
    // All should work without errors
    expect(true).toBe(true)
  })
})