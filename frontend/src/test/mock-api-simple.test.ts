import { describe, it, expect } from 'vitest'
import { createMockDocument } from './mocks/factories/document'
import { createMockUser } from './mocks/factories/user'

describe('Mock API Basic Functionality', () => {
  it('should create mock documents', () => {
    const doc = createMockDocument()
    expect(doc).toBeDefined()
    expect(doc.id).toBeDefined()
    expect(doc.filename).toBeDefined()
    expect(doc.mime_type).toBeDefined()
  })

  it('should create mock users', () => {
    const user = createMockUser()
    expect(user).toBeDefined()
    expect(user.id).toBeDefined()
    expect(user.username).toBeDefined()
    expect(user.email).toBeDefined()
  })

  it('should support document overrides', () => {
    const doc = createMockDocument({
      filename: 'test.pdf',
      mime_type: 'application/pdf',
      file_size: 1024
    })
    expect(doc.filename).toBe('test.pdf')
    expect(doc.mime_type).toBe('application/pdf')
    expect(doc.file_size).toBe(1024)
  })

  it('should support user overrides', () => {
    const user = createMockUser({
      username: 'testuser',
      email: 'test@example.com',
      role: 'admin'
    })
    expect(user.username).toBe('testuser')
    expect(user.email).toBe('test@example.com')
    expect(user.role).toBe('admin')
  })
})