/**
 * Performance testing fixtures and utilities
 * Provides data and configurations for testing application performance
 */

import { createMockDocuments, createMockSources, createMockUsers } from '../factories'
import { MockConfig } from '../api/types'

/**
 * Performance test configurations
 */
export const PERFORMANCE_CONFIGS = {
  FAST_RESPONSES: {
    delay: 10,
    shouldFail: false,
  } as MockConfig,
  
  REALISTIC_RESPONSES: {
    delay: 150,
    shouldFail: false,
  } as MockConfig,
  
  SLOW_RESPONSES: {
    delay: 1000,
    shouldFail: false,
  } as MockConfig,
  
  VERY_SLOW_RESPONSES: {
    delay: 3000,
    shouldFail: false,
  } as MockConfig,
  
  INTERMITTENT_FAILURES: {
    delay: 200,
    shouldFail: Math.random() > 0.8, // 20% failure rate
    errorCode: 500,
  } as MockConfig,
}

/**
 * Large datasets for performance testing
 */
export const LARGE_DATASETS = {
  SMALL_LOAD: {
    documents: createMockDocuments(100),
    users: createMockUsers(5),
    sources: createMockSources(3),
    description: 'Small dataset for baseline performance testing',
  },
  
  MEDIUM_LOAD: {
    documents: createMockDocuments(500),
    users: createMockUsers(20),
    sources: createMockSources(10),
    description: 'Medium dataset for typical usage scenarios',
  },
  
  LARGE_LOAD: {
    documents: createMockDocuments(2000),
    users: createMockUsers(50),
    sources: createMockSources(25),
    description: 'Large dataset for stress testing',
  },
  
  EXTRA_LARGE_LOAD: {
    documents: createMockDocuments(5000),
    users: createMockUsers(100),
    sources: createMockSources(50),
    description: 'Extra large dataset for maximum load testing',
  },
}

/**
 * Performance benchmarks and expectations
 */
export const PERFORMANCE_BENCHMARKS = {
  DOCUMENT_LIST_LOAD: {
    fast: 200, // ms
    acceptable: 500,
    slow: 1000,
  },
  
  SEARCH_RESPONSE: {
    fast: 100,
    acceptable: 300,
    slow: 800,
  },
  
  DOCUMENT_UPLOAD: {
    fast: 500,
    acceptable: 2000,
    slow: 5000,
  },
  
  SOURCE_SYNC_START: {
    fast: 100,
    acceptable: 300,
    slow: 1000,
  },
  
  PAGE_NAVIGATION: {
    fast: 50,
    acceptable: 150,
    slow: 300,
  },
}

/**
 * Memory usage test configurations
 */
export const MEMORY_TEST_CONFIGS = {
  MINIMAL_MEMORY: {
    maxDocuments: 50,
    maxConcurrentRequests: 2,
    description: 'Minimal memory usage configuration',
  },
  
  NORMAL_MEMORY: {
    maxDocuments: 200,
    maxConcurrentRequests: 5,
    description: 'Normal memory usage configuration',
  },
  
  HIGH_MEMORY: {
    maxDocuments: 1000,
    maxConcurrentRequests: 10,
    description: 'High memory usage configuration',
  },
}

/**
 * Network condition simulations
 */
export const NETWORK_CONDITIONS = {
  FAST_CONNECTION: {
    delay: 10,
    jitter: 5,
    packetLoss: 0,
    description: 'Fast, stable internet connection',
  },
  
  TYPICAL_CONNECTION: {
    delay: 100,
    jitter: 20,
    packetLoss: 0.01,
    description: 'Typical home/office internet connection',
  },
  
  SLOW_CONNECTION: {
    delay: 500,
    jitter: 100,
    packetLoss: 0.05,
    description: 'Slow internet connection',
  },
  
  MOBILE_CONNECTION: {
    delay: 300,
    jitter: 150,
    packetLoss: 0.1,
    description: 'Mobile data connection',
  },
  
  POOR_CONNECTION: {
    delay: 1000,
    jitter: 500,
    packetLoss: 0.15,
    description: 'Poor quality connection',
  },
}

/**
 * Concurrency test scenarios
 */
export const CONCURRENCY_SCENARIOS = {
  SINGLE_USER: {
    concurrentUsers: 1,
    requestsPerUser: 10,
    description: 'Single user making multiple requests',
  },
  
  LIGHT_LOAD: {
    concurrentUsers: 5,
    requestsPerUser: 5,
    description: 'Light concurrent load',
  },
  
  MODERATE_LOAD: {
    concurrentUsers: 15,
    requestsPerUser: 10,
    description: 'Moderate concurrent load',
  },
  
  HEAVY_LOAD: {
    concurrentUsers: 50,
    requestsPerUser: 20,
    description: 'Heavy concurrent load',
  },
  
  STRESS_TEST: {
    concurrentUsers: 100,
    requestsPerUser: 50,
    description: 'Stress test with maximum load',
  },
}

/**
 * Performance test utilities
 */
export class PerformanceTestUtils {
  private static measurements: Map<string, number[]> = new Map()
  
  /**
   * Start timing a performance measurement
   */
  static startMeasurement(name: string): () => number {
    const startTime = performance.now()
    
    return () => {
      const duration = performance.now() - startTime
      this.recordMeasurement(name, duration)
      return duration
    }
  }
  
  /**
   * Record a performance measurement
   */
  static recordMeasurement(name: string, duration: number) {
    if (!this.measurements.has(name)) {
      this.measurements.set(name, [])
    }
    this.measurements.get(name)!.push(duration)
  }
  
  /**
   * Get performance statistics for a measurement
   */
  static getStats(name: string) {
    const measurements = this.measurements.get(name) || []
    if (measurements.length === 0) {
      return null
    }
    
    const sorted = [...measurements].sort((a, b) => a - b)
    const sum = measurements.reduce((a, b) => a + b, 0)
    
    return {
      count: measurements.length,
      min: sorted[0],
      max: sorted[sorted.length - 1],
      average: sum / measurements.length,
      median: sorted[Math.floor(sorted.length / 2)],
      p95: sorted[Math.floor(sorted.length * 0.95)],
      p99: sorted[Math.floor(sorted.length * 0.99)],
    }
  }
  
  /**
   * Clear all measurements
   */
  static clearMeasurements() {
    this.measurements.clear()
  }
  
  /**
   * Simulate network latency
   */
  static async simulateNetworkLatency(condition: keyof typeof NETWORK_CONDITIONS) {
    const config = NETWORK_CONDITIONS[condition]
    const jitter = Math.random() * config.jitter
    const delay = config.delay + jitter
    
    // Simulate packet loss
    if (Math.random() < config.packetLoss) {
      throw new Error('Simulated packet loss')
    }
    
    await new Promise(resolve => setTimeout(resolve, delay))
  }
  
  /**
   * Generate performance test report
   */
  static generateReport(): string {
    const report = ['Performance Test Report', '='.repeat(25), '']
    
    for (const [name, measurements] of this.measurements.entries()) {
      const stats = this.getStats(name)
      if (stats) {
        report.push(`${name}:`)
        report.push(`  Count: ${stats.count}`)
        report.push(`  Average: ${stats.average.toFixed(2)}ms`)
        report.push(`  Median: ${stats.median.toFixed(2)}ms`)
        report.push(`  Min: ${stats.min.toFixed(2)}ms`)
        report.push(`  Max: ${stats.max.toFixed(2)}ms`)
        report.push(`  95th percentile: ${stats.p95.toFixed(2)}ms`)
        report.push(`  99th percentile: ${stats.p99.toFixed(2)}ms`)
        report.push('')
      }
    }
    
    return report.join('\n')
  }
}

/**
 * Performance test assertions
 */
export const PERFORMANCE_ASSERTIONS = {
  /**
   * Assert that an operation completes within expected time
   */
  assertPerformance: (
    operation: string,
    duration: number,
    benchmark: keyof typeof PERFORMANCE_BENCHMARKS
  ) => {
    const thresholds = PERFORMANCE_BENCHMARKS[benchmark]
    
    if (duration <= thresholds.fast) {
      console.log(`✅ ${operation} completed fast: ${duration.toFixed(2)}ms`)
    } else if (duration <= thresholds.acceptable) {
      console.log(`⚠️ ${operation} completed acceptably: ${duration.toFixed(2)}ms`)
    } else if (duration <= thresholds.slow) {
      console.warn(`🐌 ${operation} completed slowly: ${duration.toFixed(2)}ms`)
    } else {
      console.error(`❌ ${operation} too slow: ${duration.toFixed(2)}ms`)
      throw new Error(`Performance assertion failed: ${operation} took ${duration.toFixed(2)}ms, expected < ${thresholds.slow}ms`)
    }
  },
  
  /**
   * Assert that average performance is within bounds
   */
  assertAveragePerformance: (
    operation: string,
    benchmark: keyof typeof PERFORMANCE_BENCHMARKS
  ) => {
    const stats = PerformanceTestUtils.getStats(operation)
    if (!stats) {
      throw new Error(`No performance data available for ${operation}`)
    }
    
    const thresholds = PERFORMANCE_BENCHMARKS[benchmark]
    if (stats.average > thresholds.acceptable) {
      throw new Error(
        `Average performance too slow: ${stats.average.toFixed(2)}ms, expected < ${thresholds.acceptable}ms`
      )
    }
  },
}