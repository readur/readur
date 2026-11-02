import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import type { InternalAxiosRequestConfig } from 'axios'
import api, { searchService, sourceService, sourcesService } from '../api'
import type { MonthCount, SourceArrivals } from '../api'

// Drive the real service methods through the real axios instance with a
// swapped network adapter, recording each request URL and replying with `reply`.
let requests: string[] = []
let reply: unknown = []
const originalAdapter = api.defaults.adapter

beforeEach(() => {
  requests = []
  reply = []
  api.defaults.adapter = async (config: InternalAxiosRequestConfig) => {
    requests.push(api.getUri(config))
    return { data: reply, status: 200, statusText: 'OK', headers: {}, config }
  }
})

afterEach(() => {
  api.defaults.adapter = originalAdapter
})

describe('sourceService.getArrivals', () => {
  it('is the same service as sourcesService', () => {
    expect(sourceService).toBe(sourcesService)
  })

  it('omits days so the backend default (14) applies', async () => {
    await sourceService.getArrivals()
    expect(requests).toEqual(['/api/sources/arrivals'])
  })

  it('sends the requested day count', async () => {
    await sourceService.getArrivals(30)
    expect(requests).toEqual(['/api/sources/arrivals?days=30'])
  })

  it('returns the typed lanes', async () => {
    const lanes: SourceArrivals[] = [
      {
        key: 'upload',
        source_id: null,
        kind: 'upload',
        name: 'Uploads',
        days: [
          { date: '2026-09-29', count: 0 },
          { date: '2026-09-30', count: 3 },
        ],
        today: 3,
        last_arrival_at: '2026-09-30T08:04:00Z',
        enabled: true,
        status: null,
      },
    ]
    reply = lanes
    const response = await sourceService.getArrivals(2)
    expect(response.data).toEqual(lanes)
  })
})

describe('searchService.getTimeline', () => {
  it('sends the query with the simple search mode by default', async () => {
    await searchService.getTimeline({ query: 'shoulder injury' })
    expect(requests).toEqual(['/api/search/timeline?query=shoulder+injury&search_mode=simple'])
  })

  it('serializes filters like enhancedSearch and keeps an explicit mode', async () => {
    await searchService.getTimeline({
      query: 'mri',
      search_mode: 'phrase',
      label_ids: ['11111111-1111-1111-1111-111111111111', '22222222-2222-2222-2222-222222222222'],
      source_types: ['webdav'],
      mime_types: [],
      created_from: new Date('2026-01-01T00:00:00Z'),
      created_to: '2026-02-01T00:00:00Z',
    })
    expect(requests).toEqual([
      '/api/search/timeline?query=mri&search_mode=phrase' +
        '&label_ids=11111111-1111-1111-1111-111111111111,22222222-2222-2222-2222-222222222222' +
        '&source_types=webdav' +
        '&created_from=2026-01-01T00:00:00.000Z&created_to=2026-02-01T00:00:00Z',
    ])
  })

  it('sends the same filter params as enhancedSearch for the same input', async () => {
    const params = { query: 'knee', ocr_status: 'completed', tags: ['medical'] }
    await searchService.getTimeline(params)
    await searchService.enhancedSearch(params)
    const [timeline, enhanced] = requests.map((uri) => new URL(uri, 'http://x').searchParams)
    for (const key of ['query', 'ocr_status', 'tags', 'search_mode']) {
      expect(timeline.get(key)).toBe(enhanced.get(key))
    }
    expect(timeline.has('include_snippets')).toBe(false)
  })

  it('returns the typed months', async () => {
    const months: MonthCount[] = [
      { month: '2026-01', count: 2 },
      { month: '2026-03', count: 5 },
    ]
    reply = months
    const response = await searchService.getTimeline({ query: 'mri' })
    expect(response.data).toEqual(months)
  })
})
