/**
 * Performance Testing Patterns - Comprehensive performance testing utilities
 * Provides reusable patterns for testing component performance and optimization
 */

import { act, renderHook } from '@testing-library/react'
import { profiler } from 'react'
import type { EnhancedRenderResult } from '../render'

export interface PerformanceTestConfig {
  name: string
  iterations?: number
  warmupIterations?: number
  measurementTypes: ('render' | 'update' | 'interaction' | 'memory' | 'bundle')[]
  thresholds?: PerformanceThresholds
  enableProfiler?: boolean
  profileInteractions?: boolean
}

export interface PerformanceThresholds {
  maxRenderTime?: number
  maxUpdateTime?: number
  maxInteractionTime?: number
  maxMemoryUsage?: number
  maxBundleSize?: number
  minFPS?: number
}

export interface PerformanceTestResult {
  success: boolean
  metrics: PerformanceMetrics
  thresholdViolations: ThresholdViolation[]
  recommendations: string[]
  rawData: {
    renderTimes: number[]
    updateTimes: number[]
    interactionTimes: number[]
    memorySnapshots: MemorySnapshot[]
    profilerData?: ProfilerData[]
  }
}

export interface PerformanceMetrics {
  renderTime: {
    average: number
    min: number
    max: number
    p95: number
    p99: number
  }
  updateTime: {
    average: number
    min: number
    max: number
    p95: number
    p99: number
  }
  interactionTime: {
    average: number
    min: number
    max: number
    p95: number
    p99: number
  }
  memoryUsage: {
    initial: number
    peak: number
    final: number
    leaked: number
  }
  fps?: number
  bundleSize?: {
    compressed: number
    uncompressed: number
  }
}

export interface ThresholdViolation {
  metric: string
  threshold: number
  actual: number
  severity: 'warning' | 'error'
}

export interface MemorySnapshot {
  timestamp: number
  heapUsed: number
  heapTotal: number
  external: number
}

export interface ProfilerData {
  id: string
  phase: 'mount' | 'update'
  actualDuration: number
  baseDuration: number
  startTime: number
  commitTime: number
  interactions: any[]
}

/**
 * Comprehensive performance testing class
 */
export class PerformanceTester {
  private config: PerformanceTestConfig
  private renderResult: EnhancedRenderResult
  private profilerData: ProfilerData[] = []
  private memorySnapshots: MemorySnapshot[] = []

  constructor(renderResult: EnhancedRenderResult, config: PerformanceTestConfig) {
    this.renderResult = renderResult
    this.config = {
      iterations: 100,
      warmupIterations: 10,
      measurementTypes: ['render', 'update', 'interaction'],
      enableProfiler: true,
      profileInteractions: true,
      thresholds: {
        maxRenderTime: 100,
        maxUpdateTime: 50,
        maxInteractionTime: 16, // 60fps target
        maxMemoryUsage: 50 * 1024 * 1024, // 50MB
      },
      ...config,
    }
  }

  /**
   * Run comprehensive performance test
   */
  async runPerformanceTest(): Promise<PerformanceTestResult> {
    const rawData = {
      renderTimes: [] as number[],
      updateTimes: [] as number[],
      interactionTimes: [] as number[],
      memorySnapshots: [] as MemorySnapshot[],
      profilerData: [] as ProfilerData[],
    }

    // Take initial memory snapshot
    this.takeMemorySnapshot()

    // Warmup phase
    if (this.config.warmupIterations && this.config.warmupIterations > 0) {
      await this.runWarmup()
    }

    // Measurement phase
    for (const measurementType of this.config.measurementTypes) {
      switch (measurementType) {
        case 'render':
          rawData.renderTimes = await this.measureRenderPerformance()
          break
        case 'update':
          rawData.updateTimes = await this.measureUpdatePerformance()
          break
        case 'interaction':
          rawData.interactionTimes = await this.measureInteractionPerformance()
          break
        case 'memory':
          this.takeMemorySnapshot()
          break
        case 'bundle':
          // Bundle size would be measured during build process
          break
      }
    }

    rawData.memorySnapshots = this.memorySnapshots
    rawData.profilerData = this.profilerData

    // Calculate metrics
    const metrics = this.calculateMetrics(rawData)

    // Check thresholds
    const thresholdViolations = this.checkThresholds(metrics)

    // Generate recommendations
    const recommendations = this.generateRecommendations(metrics, thresholdViolations)

    return {
      success: thresholdViolations.filter(v => v.severity === 'error').length === 0,
      metrics,
      thresholdViolations,
      recommendations,
      rawData,
    }
  }

  /**
   * Measure render performance
   */
  private async measureRenderPerformance(): Promise<number[]> {
    const renderTimes: number[] = []

    for (let i = 0; i < this.config.iterations!; i++) {
      const startTime = performance.now()
      
      // Force re-render by updating a prop or state
      act(() => {
        // This would trigger a re-render
        this.renderResult.rerender(this.renderResult.container.firstChild as React.ReactElement)
      })

      const endTime = performance.now()
      renderTimes.push(endTime - startTime)

      // Small delay between iterations
      await new Promise(resolve => setTimeout(resolve, 10))
    }

    return renderTimes
  }

  /**
   * Measure update performance
   */
  private async measureUpdatePerformance(): Promise<number[]> {
    const updateTimes: number[] = []

    // Find interactive elements to trigger updates
    const buttons = this.renderResult.container.querySelectorAll('button')
    const inputs = this.renderResult.container.querySelectorAll('input')
    const interactiveElements = [...buttons, ...inputs]

    if (interactiveElements.length === 0) {
      return [] // No interactive elements to test
    }

    for (let i = 0; i < this.config.iterations!; i++) {
      const element = interactiveElements[i % interactiveElements.length]
      const startTime = performance.now()

      if (element.tagName === 'BUTTON') {
        act(() => {
          element.dispatchEvent(new MouseEvent('click', { bubbles: true }))
        })
      } else if (element.tagName === 'INPUT') {
        act(() => {
          (element as HTMLInputElement).value = `test-${i}`
          element.dispatchEvent(new Event('input', { bubbles: true }))
        })
      }

      const endTime = performance.now()
      updateTimes.push(endTime - startTime)

      await new Promise(resolve => setTimeout(resolve, 10))
    }

    return updateTimes
  }

  /**
   * Measure interaction performance
   */
  private async measureInteractionPerformance(): Promise<number[]> {
    const interactionTimes: number[] = []
    const buttons = this.renderResult.container.querySelectorAll('button')

    if (buttons.length === 0) {
      return []
    }

    for (let i = 0; i < Math.min(this.config.iterations!, 50); i++) {
      const button = buttons[i % buttons.length]
      
      // Measure from click to paint
      const startTime = performance.now()

      await act(async () => {
        button.dispatchEvent(new MouseEvent('click', { bubbles: true }))
        // Wait for next frame
        await new Promise(resolve => requestAnimationFrame(resolve))
      })

      const endTime = performance.now()
      interactionTimes.push(endTime - startTime)

      await new Promise(resolve => setTimeout(resolve, 50))
    }

    return interactionTimes
  }

  /**
   * Run warmup iterations
   */
  private async runWarmup(): Promise<void> {
    for (let i = 0; i < this.config.warmupIterations!; i++) {
      // Simple operations to warm up
      act(() => {
        this.renderResult.rerender(this.renderResult.container.firstChild as React.ReactElement)
      })
      await new Promise(resolve => setTimeout(resolve, 10))
    }
  }

  /**
   * Take memory snapshot
   */
  private takeMemorySnapshot(): void {
    if ('memory' in performance) {
      const memory = (performance as any).memory
      this.memorySnapshots.push({
        timestamp: performance.now(),
        heapUsed: memory.usedJSHeapSize,
        heapTotal: memory.totalJSHeapSize,
        external: memory.externalHeapSize || 0,
      })
    }
  }

  /**
   * Calculate performance metrics from raw data
   */
  private calculateMetrics(rawData: any): PerformanceMetrics {
    const calculateStats = (times: number[]) => {
      if (times.length === 0) {
        return { average: 0, min: 0, max: 0, p95: 0, p99: 0 }
      }

      const sorted = [...times].sort((a, b) => a - b)
      return {
        average: times.reduce((sum, time) => sum + time, 0) / times.length,
        min: sorted[0],
        max: sorted[sorted.length - 1],
        p95: sorted[Math.floor(sorted.length * 0.95)],
        p99: sorted[Math.floor(sorted.length * 0.99)],
      }
    }

    const memoryUsage = this.calculateMemoryUsage()

    return {
      renderTime: calculateStats(rawData.renderTimes),
      updateTime: calculateStats(rawData.updateTimes),
      interactionTime: calculateStats(rawData.interactionTimes),
      memoryUsage,
      fps: this.calculateFPS(rawData.interactionTimes),
    }
  }

  /**
   * Calculate memory usage metrics
   */
  private calculateMemoryUsage() {
    if (this.memorySnapshots.length === 0) {
      return { initial: 0, peak: 0, final: 0, leaked: 0 }
    }

    const initial = this.memorySnapshots[0].heapUsed
    const final = this.memorySnapshots[this.memorySnapshots.length - 1].heapUsed
    const peak = Math.max(...this.memorySnapshots.map(s => s.heapUsed))
    const leaked = Math.max(0, final - initial)

    return { initial, peak, final, leaked }
  }

  /**
   * Calculate approximate FPS from interaction times
   */
  private calculateFPS(interactionTimes: number[]): number {
    if (interactionTimes.length === 0) return 0

    const avgInteractionTime = interactionTimes.reduce((sum, time) => sum + time, 0) / interactionTimes.length
    return Math.min(60, 1000 / avgInteractionTime)
  }

  /**
   * Check performance thresholds
   */
  private checkThresholds(metrics: PerformanceMetrics): ThresholdViolation[] {
    const violations: ThresholdViolation[] = []
    const thresholds = this.config.thresholds!

    if (thresholds.maxRenderTime && metrics.renderTime.average > thresholds.maxRenderTime) {
      violations.push({
        metric: 'renderTime.average',
        threshold: thresholds.maxRenderTime,
        actual: metrics.renderTime.average,
        severity: metrics.renderTime.average > thresholds.maxRenderTime * 2 ? 'error' : 'warning',
      })
    }

    if (thresholds.maxUpdateTime && metrics.updateTime.average > thresholds.maxUpdateTime) {
      violations.push({
        metric: 'updateTime.average',
        threshold: thresholds.maxUpdateTime,
        actual: metrics.updateTime.average,
        severity: metrics.updateTime.average > thresholds.maxUpdateTime * 2 ? 'error' : 'warning',
      })
    }

    if (thresholds.maxInteractionTime && metrics.interactionTime.average > thresholds.maxInteractionTime) {
      violations.push({
        metric: 'interactionTime.average',
        threshold: thresholds.maxInteractionTime,
        actual: metrics.interactionTime.average,
        severity: 'error', // Interaction time is critical for UX
      })
    }

    if (thresholds.maxMemoryUsage && metrics.memoryUsage.peak > thresholds.maxMemoryUsage) {
      violations.push({
        metric: 'memoryUsage.peak',
        threshold: thresholds.maxMemoryUsage,
        actual: metrics.memoryUsage.peak,
        severity: 'warning',
      })
    }

    if (thresholds.minFPS && metrics.fps && metrics.fps < thresholds.minFPS) {
      violations.push({
        metric: 'fps',
        threshold: thresholds.minFPS,
        actual: metrics.fps,
        severity: 'error',
      })
    }

    return violations
  }

  /**
   * Generate performance recommendations
   */
  private generateRecommendations(
    metrics: PerformanceMetrics,
    violations: ThresholdViolation[]
  ): string[] {
    const recommendations: string[] = []

    // Render performance recommendations
    if (metrics.renderTime.average > 50) {
      recommendations.push('Consider using React.memo for components that render frequently')
      recommendations.push('Check for unnecessary re-renders with React DevTools Profiler')
    }

    if (metrics.renderTime.max > 200) {
      recommendations.push('Some renders are very slow - investigate heavy computations in render')
    }

    // Update performance recommendations
    if (metrics.updateTime.average > 30) {
      recommendations.push('State updates are slow - consider optimizing setState calls')
      recommendations.push('Use useCallback and useMemo to prevent unnecessary recalculations')
    }

    // Interaction performance recommendations
    if (metrics.interactionTime.average > 16) {
      recommendations.push('Interactions are not achieving 60fps - consider debouncing or throttling')
      recommendations.push('Use React.startTransition for non-urgent updates')
    }

    // Memory recommendations
    if (metrics.memoryUsage.leaked > 10 * 1024 * 1024) { // 10MB
      recommendations.push('Potential memory leak detected - check for event listeners and subscriptions')
    }

    if (metrics.memoryUsage.peak > 100 * 1024 * 1024) { // 100MB
      recommendations.push('High memory usage - consider lazy loading and code splitting')
    }

    // General recommendations based on violations
    violations.forEach(violation => {
      if (violation.severity === 'error') {
        recommendations.push(`Critical performance issue: ${violation.metric} exceeds threshold by ${((violation.actual / violation.threshold - 1) * 100).toFixed(1)}%`)
      }
    })

    return recommendations
  }
}

/**
 * Quick performance testing utilities
 */
export const testComponentPerformance = async (
  renderResult: EnhancedRenderResult,
  config: Partial<PerformanceTestConfig> = {}
): Promise<PerformanceTestResult> => {
  const tester = new PerformanceTester(renderResult, {
    name: 'Component Performance Test',
    ...config,
  })
  return await tester.runPerformanceTest()
}

/**
 * Test rendering performance specifically
 */
export const testRenderPerformance = async (
  renderResult: EnhancedRenderResult,
  iterations: number = 100
): Promise<{ averageTime: number; violations: ThresholdViolation[] }> => {
  const result = await testComponentPerformance(renderResult, {
    name: 'Render Performance',
    iterations,
    measurementTypes: ['render'],
    thresholds: { maxRenderTime: 50 },
  })

  return {
    averageTime: result.metrics.renderTime.average,
    violations: result.thresholdViolations,
  }
}

/**
 * Test memory usage
 */
export const testMemoryUsage = async (
  renderResult: EnhancedRenderResult,
  operations: (() => void)[]
): Promise<{ memoryUsage: PerformanceMetrics['memoryUsage']; hasLeaks: boolean }> => {
  const tester = new PerformanceTester(renderResult, {
    name: 'Memory Test',
    measurementTypes: ['memory'],
  })

  // Initial snapshot
  tester['takeMemorySnapshot']()

  // Run operations
  for (const operation of operations) {
    operation()
    await new Promise(resolve => setTimeout(resolve, 100))
    tester['takeMemorySnapshot']()
  }

  // Force garbage collection if available
  if ('gc' in window) {
    (window as any).gc()
    await new Promise(resolve => setTimeout(resolve, 100))
    tester['takeMemorySnapshot']()
  }

  const memoryUsage = tester['calculateMemoryUsage']()
  const hasLeaks = memoryUsage.leaked > 5 * 1024 * 1024 // 5MB threshold

  return { memoryUsage, hasLeaks }
}

/**
 * Performance hook testing
 */
export const testHookPerformance = <T>(
  hook: () => T,
  operations: Array<(result: { current: T }) => void>,
  iterations: number = 100
): { averageTime: number; maxTime: number } => {
  const times: number[] = []

  for (let i = 0; i < iterations; i++) {
    const startTime = performance.now()
    
    const { result } = renderHook(hook)
    
    operations.forEach(operation => {
      act(() => {
        operation(result)
      })
    })

    const endTime = performance.now()
    times.push(endTime - startTime)
  }

  return {
    averageTime: times.reduce((sum, time) => sum + time, 0) / times.length,
    maxTime: Math.max(...times),
  }
}

/**
 * Bundle size analysis (for build-time testing)
 */
export const analyzeBundleSize = async (
  componentPath: string
): Promise<{ size: number; gzippedSize: number; recommendations: string[] }> => {
  // This would integrate with webpack-bundle-analyzer or similar
  // For now, return mock data
  const recommendations: string[] = []

  // Mock analysis
  const estimatedSize = 50000 // 50KB
  const estimatedGzipped = 15000 // 15KB

  if (estimatedSize > 100000) {
    recommendations.push('Component bundle is large - consider code splitting')
  }

  if (estimatedGzipped > 30000) {
    recommendations.push('Gzipped size is high - check for duplicate dependencies')
  }

  return {
    size: estimatedSize,
    gzippedSize: estimatedGzipped,
    recommendations,
  }
}

/**
 * Common performance test configurations
 */
export const commonPerformanceConfigs = {
  quickTest: (): PerformanceTestConfig => ({
    name: 'Quick Performance Test',
    iterations: 50,
    warmupIterations: 5,
    measurementTypes: ['render', 'interaction'],
    thresholds: {
      maxRenderTime: 100,
      maxInteractionTime: 16,
    },
  }),

  comprehensiveTest: (): PerformanceTestConfig => ({
    name: 'Comprehensive Performance Test',
    iterations: 200,
    warmupIterations: 20,
    measurementTypes: ['render', 'update', 'interaction', 'memory'],
    thresholds: {
      maxRenderTime: 50,
      maxUpdateTime: 30,
      maxInteractionTime: 16,
      maxMemoryUsage: 50 * 1024 * 1024,
      minFPS: 30,
    },
  }),

  memoryTest: (): PerformanceTestConfig => ({
    name: 'Memory Usage Test',
    iterations: 100,
    measurementTypes: ['memory'],
    thresholds: {
      maxMemoryUsage: 25 * 1024 * 1024,
    },
  }),

  interactionTest: (): PerformanceTestConfig => ({
    name: 'Interaction Performance Test',
    iterations: 100,
    measurementTypes: ['interaction'],
    thresholds: {
      maxInteractionTime: 16,
      minFPS: 60,
    },
  }),
}