import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import type { InternalAxiosRequestConfig } from 'axios'
import api, { documentService, ocrService } from '../api'

// Real service methods through the real axios instance, with the network adapter swapped
// so the exact method, URL, headers and body can be read back.
let sent: InternalAxiosRequestConfig[] = []
const originalAdapter = api.defaults.adapter

beforeEach(() => {
  sent = []
  api.defaults.adapter = async (config: InternalAxiosRequestConfig) => {
    sent.push(config)
    return { data: {}, status: 200, statusText: 'OK', headers: {}, config }
  }
})

afterEach(() => {
  api.defaults.adapter = originalAdapter
})

const contentType = (config: InternalAxiosRequestConfig) => String(config.headers.getContentType?.() ?? config.headers['Content-Type'])

describe('documentService.retryOcr', () => {
  it('POSTs an empty JSON body so the server accepts the request', async () => {
    await documentService.retryOcr('doc-123')

    expect(sent).toHaveLength(1)
    const [config] = sent
    expect(config.method).toBe('post')
    expect(api.getUri(config)).toBe('/api/documents/doc-123/ocr/retry')
    expect(contentType(config)).toMatch(/^application\/json/)
    expect(JSON.parse(config.data as string)).toEqual({})
  })
})

describe('ocrService.retryWithLanguage', () => {
  it('sends the chosen languages as JSON', async () => {
    await ocrService.retryWithLanguage('doc-9', undefined, ['eng', 'spa'])

    const [config] = sent
    expect(api.getUri(config)).toBe('/api/documents/doc-9/ocr/retry')
    expect(contentType(config)).toMatch(/^application\/json/)
    expect(JSON.parse(config.data as string)).toEqual({ languages: ['eng', 'spa'] })
  })
})
