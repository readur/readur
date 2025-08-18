/**
 * Label factory for generating realistic test data
 */

import { faker } from '@faker-js/faker'
import { MockLabel, FactoryOptions } from '../api/types'

faker.seed(12345)

const LABEL_COLORS = [
  '#ef4444', '#f97316', '#f59e0b', '#eab308', '#84cc16',
  '#22c55e', '#10b981', '#14b8a6', '#06b6d4', '#0ea5e9',
  '#3b82f6', '#6366f1', '#8b5cf6', '#a855f7', '#d946ef',
  '#ec4899', '#f43f5e', '#64748b', '#6b7280', '#374151',
] as const

const COMMON_LABELS = [
  'Important', 'Work', 'Personal', 'Invoice', 'Receipt', 
  'Contract', 'Report', 'Meeting Notes', 'Draft', 'Final',
  'Archive', 'Tax Documents', 'Insurance', 'Medical',
  'Legal', 'Financial', 'Project', 'Client', 'Vendor',
  'Urgent', 'Review', 'Approved', 'Rejected', 'Pending'
] as const

/**
 * Create a single mock label
 */
export const createMockLabel = (overrides: Partial<MockLabel> = {}): MockLabel => {
  const createdAt = faker.date.past({ years: 1 })
  const updatedAt = faker.date.between({ from: createdAt, to: new Date() })

  return {
    id: faker.string.uuid(),
    name: faker.helpers.arrayElement(COMMON_LABELS),
    color: faker.helpers.arrayElement(LABEL_COLORS),
    user_id: faker.string.uuid(),
    created_at: createdAt.toISOString(),
    updated_at: updatedAt.toISOString(),
    document_count: faker.number.int({ min: 0, max: 150 }),
    ...overrides,
  }
}

/**
 * Create multiple mock labels
 */
export const createMockLabels = (
  count: number = 10,
  options: FactoryOptions = {}
): MockLabel[] => {
  if (options.seed) {
    faker.seed(options.seed)
  }

  return Array.from({ length: count }, () => 
    createMockLabel(options.overrides)
  )
}

/**
 * Create a mock label with specific characteristics
 */
export const createMockLabelWithScenario = (scenario: string): MockLabel => {
  const baseLabel = createMockLabel()

  switch (scenario) {
    case 'important_label':
      return {
        ...baseLabel,
        name: 'Important',
        color: '#ef4444', // Red
        document_count: faker.number.int({ min: 25, max: 100 }),
      }

    case 'work_label':
      return {
        ...baseLabel,
        name: 'Work',
        color: '#3b82f6', // Blue
        document_count: faker.number.int({ min: 50, max: 200 }),
      }

    case 'personal_label':
      return {
        ...baseLabel,
        name: 'Personal',
        color: '#22c55e', // Green
        document_count: faker.number.int({ min: 10, max: 75 }),
      }

    case 'unused_label':
      return {
        ...baseLabel,
        name: 'Unused Category',
        color: '#6b7280', // Gray
        document_count: 0,
      }

    case 'new_label':
      const now = new Date()
      return {
        ...baseLabel,
        name: 'New Label',
        created_at: new Date(now.getTime() - 24 * 60 * 60 * 1000).toISOString(), // 1 day ago
        updated_at: now.toISOString(),
        document_count: faker.number.int({ min: 0, max: 5 }),
      }

    case 'popular_label':
      return {
        ...baseLabel,
        name: 'Invoice',
        color: '#f59e0b', // Amber
        document_count: faker.number.int({ min: 100, max: 500 }),
      }

    case 'archive_label':
      return {
        ...baseLabel,
        name: 'Archive',
        color: '#64748b', // Slate
        document_count: faker.number.int({ min: 200, max: 1000 }),
      }

    default:
      console.warn(`Unknown label scenario: ${scenario}`)
      return baseLabel
  }
}

/**
 * Create labels for specific test scenarios
 */
export const createLabelScenarios = (): Record<string, MockLabel[]> => ({
  empty: [],
  single: [createMockLabelWithScenario('important_label')],
  basic_set: [
    createMockLabelWithScenario('important_label'),
    createMockLabelWithScenario('work_label'),
    createMockLabelWithScenario('personal_label'),
  ],
  with_unused: [
    createMockLabelWithScenario('important_label'),
    createMockLabelWithScenario('unused_label'),
  ],
  popular_labels: [
    createMockLabelWithScenario('popular_label'),
    createMockLabelWithScenario('archive_label'),
  ],
  mixed_usage: [
    createMockLabelWithScenario('popular_label'),
    createMockLabelWithScenario('work_label'),
    createMockLabelWithScenario('unused_label'),
    createMockLabelWithScenario('new_label'),
  ],
  many: createMockLabels(25),
})

/**
 * Create a comprehensive set of default labels
 */
export const createDefaultLabels = (): MockLabel[] => [
  createMockLabelWithScenario('important_label'),
  createMockLabelWithScenario('work_label'),
  createMockLabelWithScenario('personal_label'),
  createMockLabel({ name: 'Invoice', color: '#f59e0b' }),
  createMockLabel({ name: 'Receipt', color: '#84cc16' }),
  createMockLabel({ name: 'Contract', color: '#8b5cf6' }),
  createMockLabel({ name: 'Report', color: '#06b6d4' }),
  createMockLabel({ name: 'Archive', color: '#64748b' }),
]

/**
 * Reset faker seed for consistent test results
 */
export const resetLabelFactorySeed = (seed: number = 12345) => {
  faker.seed(seed)
}