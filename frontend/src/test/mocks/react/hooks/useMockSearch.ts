/**
 * useMockSearch - Search functionality testing
 * Provides comprehensive search testing with realistic scenarios
 */

import { useState, useCallback, useEffect, useRef } from 'react'
import { useMockApiContext } from '../providers/MockApiProvider'
import { useMockDocuments } from './useMockDocuments'
import { createMockSearchResult, createMockSuggestion } from '../../factories/search'
import type { MockDocument, MockSearchResult } from '../../api/types'

export interface SearchQuery {
  query: string
  filters?: SearchFilters
  sort?: SearchSort
  facets?: string[]
  highlight?: boolean
  fuzzy?: boolean
  boolean?: 'and' | 'or'
}

export interface SearchFilters {
  fileTypes?: string[]
  dateRange?: { start: Date; end: Date }
  sizeRange?: { min: number; max: number }
  labels?: string[]
  users?: string[]
  hasOcr?: boolean
  confidenceThreshold?: number
}

export interface SearchSort {
  field: 'relevance' | 'date' | 'size' | 'name'
  direction: 'asc' | 'desc'
}

export interface SearchSuggestion {
  text: string
  type: 'query' | 'filter' | 'document'
  metadata?: any
}

export interface SearchMetrics {
  queryTime: number
  totalResults: number
  facetCounts: Record<string, Record<string, number>>
  suggestions: SearchSuggestion[]
  searchId: string
}

export interface UseMockSearchReturn {
  // Search state
  results: MockSearchResult[]
  totalResults: number
  isSearching: boolean
  currentQuery: SearchQuery | null
  searchHistory: SearchQuery[]
  
  // Search operations
  search: (query: SearchQuery) => Promise<MockSearchResult[]>
  enhancedSearch: (query: string, options?: Partial<SearchQuery>) => Promise<MockSearchResult[]>
  clearResults: () => void
  
  // Query management
  saveQuery: (name: string, query: SearchQuery) => void
  loadSavedQuery: (name: string) => SearchQuery | null
  getSavedQueries: () => Record<string, SearchQuery>
  deleteSavedQuery: (name: string) => void
  
  // Search suggestions
  getSuggestions: (partialQuery: string) => Promise<SearchSuggestion[]>
  getAutoComplete: (input: string) => Promise<string[]>
  
  // Search scenarios
  simulateTypoCorrection: (query: string) => Promise<MockSearchResult[]>
  simulateSlowSearch: (query: SearchQuery, delay?: number) => Promise<MockSearchResult[]>
  simulateSearchFailure: (errorType?: 'timeout' | 'server' | 'index') => Promise<void>
  simulateEmptyResults: (query: SearchQuery) => Promise<MockSearchResult[]>
  
  // Advanced search features
  performSemanticSearch: (query: string) => Promise<MockSearchResult[]>
  performImageSearch: (imageQuery: string) => Promise<MockSearchResult[]>
  performFullTextSearch: (query: string, options?: FullTextOptions) => Promise<MockSearchResult[]>
  
  // Search analytics
  getSearchMetrics: () => SearchAnalytics
  getPopularQueries: () => QueryStats[]
  getSearchTrends: () => TrendData[]
  
  // Testing utilities
  benchmarkSearch: (queries: string[], iterations?: number) => Promise<BenchmarkResults>
  stressTestSearch: (concurrentQueries: number) => Promise<void>
  validateSearchResults: (query: SearchQuery, results: MockSearchResult[]) => ValidationReport
  
  // Configuration
  setSearchConfig: (config: Partial<SearchConfig>) => void
  resetSearch: () => void
}

export interface FullTextOptions {
  highlightMatches?: boolean
  contextSize?: number
  includeSnippets?: boolean
  boostFields?: Record<string, number>
}

export interface SearchAnalytics {
  totalSearches: number
  averageQueryTime: number
  popularTerms: string[]
  failureRate: number
  userSatisfaction: number
}

export interface QueryStats {
  query: string
  count: number
  averageResults: number
  clickThroughRate: number
}

export interface TrendData {
  date: string
  searchVolume: number
  topQueries: string[]
}

export interface BenchmarkResults {
  averageTime: number
  minTime: number
  maxTime: number
  totalQueries: number
  failedQueries: number
  throughput: number
}

export interface ValidationReport {
  isValid: boolean
  relevanceScore: number
  coverageScore: number
  issues: string[]
  suggestions: string[]
}

export interface SearchConfig {
  maxResults: number
  highlightEnabled: boolean
  fuzzyEnabled: boolean
  typoCorrection: boolean
  semanticSearch: boolean
  cacheResults: boolean
  debugMode: boolean
}

const DEFAULT_SEARCH_CONFIG: SearchConfig = {
  maxResults: 50,
  highlightEnabled: true,
  fuzzyEnabled: true,
  typoCorrection: true,
  semanticSearch: false,
  cacheResults: true,
  debugMode: false,
}

export const useMockSearch = (): UseMockSearchReturn => {
  const [results, setResults] = useState<MockSearchResult[]>([])
  const [totalResults, setTotalResults] = useState(0)
  const [isSearching, setIsSearching] = useState(false)
  const [currentQuery, setCurrentQuery] = useState<SearchQuery | null>(null)
  const [searchHistory, setSearchHistory] = useState<SearchQuery[]>([])
  const [savedQueries, setSavedQueries] = useState<Record<string, SearchQuery>>({})
  const [searchConfig, setSearchConfigState] = useState<SearchConfig>(DEFAULT_SEARCH_CONFIG)
  const [searchAnalytics, setSearchAnalytics] = useState<SearchAnalytics>({
    totalSearches: 0,
    averageQueryTime: 0,
    popularTerms: [],
    failureRate: 0,
    userSatisfaction: 0.85,
  })
  const [resultCache, setResultCache] = useState<Map<string, { results: MockSearchResult[]; timestamp: number }>>(new Map())
  
  const apiContext = useMockApiContext?.()
  const { documents } = useMockDocuments()
  const searchTimeRef = useRef<number>(0)

  // Main search function
  const search = useCallback(async (query: SearchQuery): Promise<MockSearchResult[]> => {
    setIsSearching(true)
    setCurrentQuery(query)
    searchTimeRef.current = performance.now()
    
    // Add to search history
    setSearchHistory(prev => [query, ...prev.slice(0, 19)]) // Keep last 20 searches
    
    try {
      // Check cache first
      const cacheKey = JSON.stringify(query)
      if (searchConfig.cacheResults && resultCache.has(cacheKey)) {
        const cached = resultCache.get(cacheKey)!
        if (Date.now() - cached.timestamp < 300000) { // 5 minute cache
          setResults(cached.results)
          setTotalResults(cached.results.length)
          return cached.results
        }
      }

      // Simulate network delay
      const delay = apiContext?.getCurrentState?.()?.isSetup ? 100 : 50
      await new Promise(resolve => setTimeout(resolve, delay))

      // Perform search logic
      const searchResults = performMockSearch(query, documents, searchConfig)
      
      // Cache results
      if (searchConfig.cacheResults) {
        resultCache.set(cacheKey, { results: searchResults, timestamp: Date.now() })
      }

      setResults(searchResults)
      setTotalResults(searchResults.length)
      
      // Update analytics
      const queryTime = performance.now() - searchTimeRef.current
      setSearchAnalytics(prev => ({
        ...prev,
        totalSearches: prev.totalSearches + 1,
        averageQueryTime: (prev.averageQueryTime * prev.totalSearches + queryTime) / (prev.totalSearches + 1),
      }))

      return searchResults
      
    } catch (error) {
      setSearchAnalytics(prev => ({
        ...prev,
        failureRate: (prev.failureRate * prev.totalSearches + 1) / (prev.totalSearches + 1),
      }))
      throw error
    } finally {
      setIsSearching(false)
    }
  }, [documents, searchConfig, apiContext, resultCache])

  const enhancedSearch = useCallback(async (queryString: string, options: Partial<SearchQuery> = {}): Promise<MockSearchResult[]> => {
    const query: SearchQuery = {
      query: queryString,
      sort: { field: 'relevance', direction: 'desc' },
      highlight: true,
      fuzzy: searchConfig.fuzzyEnabled,
      ...options,
    }
    
    return search(query)
  }, [search, searchConfig])

  const clearResults = useCallback(() => {
    setResults([])
    setTotalResults(0)
    setCurrentQuery(null)
  }, [])

  // Query management
  const saveQuery = useCallback((name: string, query: SearchQuery) => {
    setSavedQueries(prev => ({ ...prev, [name]: query }))
  }, [])

  const loadSavedQuery = useCallback((name: string): SearchQuery | null => {
    return savedQueries[name] || null
  }, [savedQueries])

  const getSavedQueries = useCallback(() => savedQueries, [savedQueries])

  const deleteSavedQuery = useCallback((name: string) => {
    setSavedQueries(prev => {
      const { [name]: deleted, ...rest } = prev
      return rest
    })
  }, [])

  // Search suggestions
  const getSuggestions = useCallback(async (partialQuery: string): Promise<SearchSuggestion[]> => {
    await new Promise(resolve => setTimeout(resolve, 50))
    
    const suggestions: SearchSuggestion[] = []
    
    // Query suggestions from history
    const historySuggestions = searchHistory
      .filter(h => h.query.toLowerCase().includes(partialQuery.toLowerCase()))
      .slice(0, 3)
      .map(h => ({ text: h.query, type: 'query' as const }))
    
    suggestions.push(...historySuggestions)
    
    // Document name suggestions
    const docSuggestions = documents
      .filter(d => d.filename.toLowerCase().includes(partialQuery.toLowerCase()))
      .slice(0, 3)
      .map(d => ({ text: d.filename, type: 'document' as const, metadata: { id: d.id } }))
    
    suggestions.push(...docSuggestions)
    
    // Filter suggestions
    if (partialQuery.includes(':')) {
      suggestions.push(
        { text: 'type:pdf', type: 'filter' },
        { text: 'size:>1MB', type: 'filter' },
        { text: 'date:today', type: 'filter' }
      )
    }
    
    return suggestions
  }, [searchHistory, documents])

  const getAutoComplete = useCallback(async (input: string): Promise<string[]> => {
    await new Promise(resolve => setTimeout(resolve, 30))
    
    const terms = documents
      .flatMap(d => [d.filename, d.ocr_text || ''])
      .join(' ')
      .toLowerCase()
      .split(/\s+/)
      .filter(term => term.length > 2 && term.startsWith(input.toLowerCase()))
      .slice(0, 10)
    
    return [...new Set(terms)]
  }, [documents])

  // Search scenarios
  const simulateTypoCorrection = useCallback(async (query: string): Promise<MockSearchResult[]> => {
    const correctedQuery = correctTypos(query)
    
    if (correctedQuery !== query) {
      console.log(`Did you mean: "${correctedQuery}"?`)
    }
    
    return enhancedSearch(correctedQuery)
  }, [enhancedSearch])

  const simulateSlowSearch = useCallback(async (query: SearchQuery, delay: number = 5000): Promise<MockSearchResult[]> => {
    setIsSearching(true)
    await new Promise(resolve => setTimeout(resolve, delay))
    return search(query)
  }, [search])

  const simulateSearchFailure = useCallback(async (errorType: 'timeout' | 'server' | 'index' = 'server'): Promise<void> => {
    setIsSearching(true)
    await new Promise(resolve => setTimeout(resolve, 1000))
    
    const errorMessages = {
      timeout: 'Search request timed out',
      server: 'Search service temporarily unavailable',
      index: 'Search index is being rebuilt',
    }
    
    setIsSearching(false)
    throw new Error(errorMessages[errorType])
  }, [])

  const simulateEmptyResults = useCallback(async (query: SearchQuery): Promise<MockSearchResult[]> => {
    setIsSearching(true)
    await new Promise(resolve => setTimeout(resolve, 200))
    
    setResults([])
    setTotalResults(0)
    setCurrentQuery(query)
    setIsSearching(false)
    
    return []
  }, [])

  // Advanced search features
  const performSemanticSearch = useCallback(async (query: string): Promise<MockSearchResult[]> => {
    // Simulate semantic understanding
    const semanticTerms = expandSemanticQuery(query)
    const expandedQuery = [query, ...semanticTerms].join(' OR ')
    
    return enhancedSearch(expandedQuery, { fuzzy: true })
  }, [enhancedSearch])

  const performImageSearch = useCallback(async (imageQuery: string): Promise<MockSearchResult[]> => {
    // Simulate image-based search
    const imageResults = documents.filter(doc => 
      doc.mime_type.startsWith('image/') && 
      (doc.ocr_text?.toLowerCase().includes(imageQuery.toLowerCase()) || false)
    )
    
    return imageResults.map(doc => createMockSearchResult({
      document: doc,
      relevance_score: 0.8 + Math.random() * 0.2,
      snippet: `Image containing: ${imageQuery}`,
    }))
  }, [documents])

  const performFullTextSearch = useCallback(async (query: string, options: FullTextOptions = {}): Promise<MockSearchResult[]> => {
    const {
      highlightMatches = true,
      contextSize = 100,
      includeSnippets = true,
      boostFields = { filename: 2.0, ocr_text: 1.0 },
    } = options

    const results = await enhancedSearch(query, { highlight: highlightMatches })
    
    if (includeSnippets) {
      return results.map(result => ({
        ...result,
        snippet: generateSnippet(result.document, query, contextSize),
        relevance_score: boostRelevance(result, query, boostFields),
      }))
    }
    
    return results
  }, [enhancedSearch])

  // Search analytics
  const getSearchMetrics = useCallback((): SearchAnalytics => {
    return searchAnalytics
  }, [searchAnalytics])

  const getPopularQueries = useCallback((): QueryStats[] => {
    const queryMap = new Map<string, number>()
    
    searchHistory.forEach(search => {
      queryMap.set(search.query, (queryMap.get(search.query) || 0) + 1)
    })
    
    return Array.from(queryMap.entries())
      .sort(([,a], [,b]) => b - a)
      .slice(0, 10)
      .map(([query, count]) => ({
        query,
        count,
        averageResults: Math.floor(Math.random() * 20) + 5,
        clickThroughRate: 0.3 + Math.random() * 0.4,
      }))
  }, [searchHistory])

  const getSearchTrends = useCallback((): TrendData[] => {
    return Array.from({ length: 7 }, (_, i) => {
      const date = new Date(Date.now() - i * 24 * 60 * 60 * 1000)
      return {
        date: date.toISOString().split('T')[0],
        searchVolume: Math.floor(Math.random() * 100) + 20,
        topQueries: ['document', 'invoice', 'contract'].slice(0, Math.floor(Math.random() * 3) + 1),
      }
    }).reverse()
  }, [])

  // Testing utilities
  const benchmarkSearch = useCallback(async (queries: string[], iterations: number = 1): Promise<BenchmarkResults> => {
    const times: number[] = []
    let failedQueries = 0
    
    for (let i = 0; i < iterations; i++) {
      for (const query of queries) {
        try {
          const start = performance.now()
          await enhancedSearch(query)
          times.push(performance.now() - start)
        } catch (error) {
          failedQueries++
        }
      }
    }
    
    return {
      averageTime: times.reduce((sum, time) => sum + time, 0) / times.length,
      minTime: Math.min(...times),
      maxTime: Math.max(...times),
      totalQueries: queries.length * iterations,
      failedQueries,
      throughput: queries.length * iterations / (times.reduce((sum, time) => sum + time, 0) / 1000),
    }
  }, [enhancedSearch])

  const stressTestSearch = useCallback(async (concurrentQueries: number): Promise<void> => {
    const queries = Array.from({ length: concurrentQueries }, (_, i) => `stress test query ${i}`)
    
    const promises = queries.map(query => enhancedSearch(query))
    await Promise.allSettled(promises)
  }, [enhancedSearch])

  const validateSearchResults = useCallback((query: SearchQuery, results: MockSearchResult[]): ValidationReport => {
    const issues: string[] = []
    const suggestions: string[] = []
    
    // Check relevance
    const avgRelevance = results.reduce((sum, r) => sum + r.relevance_score, 0) / results.length
    if (avgRelevance < 0.3) {
      issues.push('Low average relevance score')
      suggestions.push('Consider expanding query or using fuzzy search')
    }
    
    // Check result count
    if (results.length === 0) {
      issues.push('No results found')
      suggestions.push('Try broader search terms or remove filters')
    } else if (results.length > 100) {
      issues.push('Too many results returned')
      suggestions.push('Add more specific search terms or filters')
    }
    
    // Check for duplicates
    const uniqueIds = new Set(results.map(r => r.document.id))
    if (uniqueIds.size !== results.length) {
      issues.push('Duplicate results detected')
      suggestions.push('Implement deduplication logic')
    }
    
    return {
      isValid: issues.length === 0,
      relevanceScore: avgRelevance,
      coverageScore: Math.min(1, results.length / 20), // Optimal around 20 results
      issues,
      suggestions,
    }
  }, [])

  const setSearchConfig = useCallback((config: Partial<SearchConfig>) => {
    setSearchConfigState(prev => ({ ...prev, ...config }))
  }, [])

  const resetSearch = useCallback(() => {
    setResults([])
    setTotalResults(0)
    setCurrentQuery(null)
    setSearchHistory([])
    setSavedQueries({})
    setSearchConfigState(DEFAULT_SEARCH_CONFIG)
    setSearchAnalytics({
      totalSearches: 0,
      averageQueryTime: 0,
      popularTerms: [],
      failureRate: 0,
      userSatisfaction: 0.85,
    })
    setResultCache(new Map())
  }, [])

  return {
    results,
    totalResults,
    isSearching,
    currentQuery,
    searchHistory,
    search,
    enhancedSearch,
    clearResults,
    saveQuery,
    loadSavedQuery,
    getSavedQueries,
    deleteSavedQuery,
    getSuggestions,
    getAutoComplete,
    simulateTypoCorrection,
    simulateSlowSearch,
    simulateSearchFailure,
    simulateEmptyResults,
    performSemanticSearch,
    performImageSearch,
    performFullTextSearch,
    getSearchMetrics,
    getPopularQueries,
    getSearchTrends,
    benchmarkSearch,
    stressTestSearch,
    validateSearchResults,
    setSearchConfig,
    resetSearch,
  }
}

// Helper functions
function performMockSearch(query: SearchQuery, documents: MockDocument[], config: SearchConfig): MockSearchResult[] {
  const { query: queryString, filters, sort } = query
  
  // Filter documents based on query and filters
  let filteredDocs = documents.filter(doc => {
    // Basic text matching
    const textMatch = queryString.toLowerCase().split(' ').some(term =>
      doc.filename.toLowerCase().includes(term) ||
      doc.ocr_text?.toLowerCase().includes(term) ||
      false
    )
    
    if (!textMatch) return false
    
    // Apply filters
    if (filters?.fileTypes && !filters.fileTypes.some(type => doc.mime_type.includes(type))) {
      return false
    }
    
    if (filters?.hasOcr !== undefined) {
      const hasOcr = doc.ocr_status === 'completed' && !!doc.ocr_text
      if (filters.hasOcr !== hasOcr) {
        return false
      }
    }
    
    return true
  })
  
  // Sort results
  if (sort?.field === 'relevance') {
    filteredDocs.sort((a, b) => calculateRelevance(b, queryString) - calculateRelevance(a, queryString))
  } else if (sort?.field === 'date') {
    filteredDocs.sort((a, b) => {
      const aTime = new Date(a.created_at).getTime()
      const bTime = new Date(b.created_at).getTime()
      return sort.direction === 'desc' ? bTime - aTime : aTime - bTime
    })
  }
  
  // Convert to search results
  return filteredDocs.slice(0, config.maxResults).map(doc => 
    createMockSearchResult({
      document: doc,
      relevance_score: calculateRelevance(doc, queryString),
      snippet: generateSnippet(doc, queryString),
    })
  )
}

function calculateRelevance(doc: MockDocument, query: string): number {
  const queryTerms = query.toLowerCase().split(' ')
  let score = 0
  
  // Filename matches
  queryTerms.forEach(term => {
    if (doc.filename.toLowerCase().includes(term)) {
      score += 0.3
    }
  })
  
  // OCR text matches
  if (doc.ocr_text) {
    queryTerms.forEach(term => {
      const termCount = (doc.ocr_text?.toLowerCase().match(new RegExp(term, 'g')) || []).length
      score += termCount * 0.1
    })
  }
  
  return Math.min(1, score)
}

function generateSnippet(doc: MockDocument, query: string, contextSize: number = 100): string {
  const text = doc.ocr_text || doc.filename
  const queryTerms = query.toLowerCase().split(' ')
  
  for (const term of queryTerms) {
    const index = text.toLowerCase().indexOf(term)
    if (index !== -1) {
      const start = Math.max(0, index - contextSize / 2)
      const end = Math.min(text.length, index + contextSize / 2)
      const snippet = text.substring(start, end)
      return `...${snippet}...`
    }
  }
  
  return text.substring(0, contextSize) + '...'
}

function correctTypos(query: string): string {
  const corrections: Record<string, string> = {
    'documnet': 'document',
    'seach': 'search',
    'invioce': 'invoice',
    'contarct': 'contract',
  }
  
  let corrected = query
  Object.entries(corrections).forEach(([typo, correct]) => {
    corrected = corrected.replace(new RegExp(typo, 'gi'), correct)
  })
  
  return corrected
}

function expandSemanticQuery(query: string): string[] {
  const semanticMap: Record<string, string[]> = {
    'document': ['file', 'paper', 'record'],
    'invoice': ['bill', 'receipt', 'statement'],
    'contract': ['agreement', 'deal', 'terms'],
    'report': ['analysis', 'summary', 'findings'],
  }
  
  const expansions: string[] = []
  query.toLowerCase().split(' ').forEach(term => {
    if (semanticMap[term]) {
      expansions.push(...semanticMap[term])
    }
  })
  
  return expansions
}

function boostRelevance(result: MockSearchResult, query: string, boostFields: Record<string, number>): number {
  let boostedScore = result.relevance_score
  
  Object.entries(boostFields).forEach(([field, boost]) => {
    if (field === 'filename' && result.document.filename.toLowerCase().includes(query.toLowerCase())) {
      boostedScore *= boost
    } else if (field === 'ocr_text' && result.document.ocr_text?.toLowerCase().includes(query.toLowerCase())) {
      boostedScore *= boost
    }
  })
  
  return Math.min(1, boostedScore)
}