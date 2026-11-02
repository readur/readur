import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import type { InternalAxiosRequestConfig } from 'axios'
import api, { documentService, searchService, serializeFilterParams } from '../api'

// Drive the real service methods through the real axios instance, but swap the
// network adapter so we can read back the exact request URL they produce.
let requests: string[] = []
const originalAdapter = api.defaults.adapter

beforeEach(() => {
  requests = []
  api.defaults.adapter = async (config: InternalAxiosRequestConfig) => {
    requests.push(api.getUri(config))
    return { data: {}, status: 200, statusText: 'OK', headers: {}, config }
  }
})

afterEach(() => {
  api.defaults.adapter = originalAdapter
})

const LABEL_A = '11111111-1111-1111-1111-111111111111'
const LABEL_B = '22222222-2222-2222-2222-222222222222'

describe('documentService.listFiltered', () => {
  it('sends no query string when called without params', async () => {
    await documentService.listFiltered()
    expect(requests).toEqual(['/api/documents'])
  })

  it('serializes sort, pagination and every filter with comma-joined lists', async () => {
    await documentService.listFiltered({
      limit: 25,
      offset: 50,
      sort_by: 'file_size',
      sort_order: 'asc',
      ocr_status: 'failed',
      mime_types: ['application/pdf', 'image/png'],
      label_ids: [LABEL_A, LABEL_B],
      source_ids: ['33333333-3333-3333-3333-333333333333'],
      source_types: ['webdav', 'direct_upload'],
      tags: ['invoice', 'work'],
      created_from: '2026-01-01T00:00:00Z',
      created_to: '2026-02-01T00:00:00Z',
    })

    expect(requests).toEqual([
      '/api/documents?limit=25&offset=50&sort_by=file_size&sort_order=asc&ocr_status=failed' +
        '&mime_types=application%2Fpdf,image%2Fpng' +
        `&label_ids=${LABEL_A},${LABEL_B}` +
        '&source_ids=33333333-3333-3333-3333-333333333333' +
        '&source_types=webdav,direct_upload' +
        '&tags=invoice,work' +
        '&created_from=2026-01-01T00:00:00Z&created_to=2026-02-01T00:00:00Z',
    ])
  })

  it('accepts pre-joined strings, ISO-formats Dates and drops empty values', async () => {
    await documentService.listFiltered({
      sort_by: 'created_at',
      label_ids: `${LABEL_A},${LABEL_B}`,
      mime_types: [],
      source_ids: undefined,
      created_from: new Date('2026-03-04T05:06:07.000Z'),
    })

    expect(requests).toEqual([
      `/api/documents?sort_by=created_at&label_ids=${LABEL_A},${LABEL_B}&created_from=2026-03-04T05:06:07.000Z`,
    ])
  })
})

describe('searchService.enhancedSearch', () => {
  it('applies the snippet and search-mode defaults', async () => {
    await searchService.enhancedSearch({ query: 'invoice' })
    expect(requests).toEqual([
      '/api/search/enhanced?query=invoice&include_snippets=true&snippet_length=200&search_mode=simple',
    ])
  })

  it('serializes sort and filters, and lets explicit options override defaults', async () => {
    await searchService.enhancedSearch({
      query: 'tax return',
      limit: 10,
      offset: 20,
      include_snippets: false,
      search_mode: 'phrase',
      sort_by: 'ocr_confidence',
      sort_order: 'desc',
      label_ids: [LABEL_A, LABEL_B],
      source_types: ['s3', 'local_folder'],
      created_from: '2026-01-01T00:00:00Z',
    })

    expect(requests).toEqual([
      '/api/search/enhanced?query=tax+return&limit=10&offset=20&include_snippets=false' +
        '&search_mode=phrase&sort_by=ocr_confidence&sort_order=desc' +
        `&label_ids=${LABEL_A},${LABEL_B}&source_types=s3,local_folder` +
        '&created_from=2026-01-01T00:00:00Z&snippet_length=200',
    ])
  })

  it('supports a filter-only search with no query', async () => {
    await searchService.enhancedSearch({ mime_types: ['application/pdf'] })
    expect(requests).toEqual([
      '/api/search/enhanced?mime_types=application%2Fpdf&include_snippets=true&snippet_length=200&search_mode=simple',
    ])
  })
})

describe('serializeFilterParams', () => {
  it('leaves scalar params untouched', () => {
    expect(serializeFilterParams({ limit: 5, sort_order: 'desc' })).toEqual({
      limit: 5,
      sort_order: 'desc',
    })
  })
})
