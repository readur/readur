/**
 * Source factory for generating realistic test data
 */

import { faker } from '@faker-js/faker'
import { MockSource, FactoryOptions } from '../api/types'

faker.seed(12345)

const SOURCE_TYPES = ['local_folder', 'webdav', 's3'] as const
const SYNC_STATUSES = ['idle', 'syncing', 'error'] as const

/**
 * Create a single mock source
 */
export const createMockSource = (overrides: Partial<MockSource> = {}): MockSource => {
  const sourceType = faker.helpers.arrayElement(SOURCE_TYPES)
  const createdAt = faker.date.past({ years: 1 })
  const updatedAt = faker.date.between({ from: createdAt, to: new Date() })
  const lastSyncAt = faker.datatype.boolean(0.8) ? 
    faker.date.between({ from: createdAt, to: new Date() }) : 
    undefined

  const basePath = sourceType === 'local_folder' 
    ? faker.system.directoryPath()
    : sourceType === 'webdav'
    ? '/remote/documents'
    : 'company-documents/uploads'

  return {
    id: faker.string.uuid(),
    name: faker.company.name() + ' Documents',
    source_type: sourceType,
    path: basePath,
    enabled: faker.datatype.boolean(0.85), // 85% chance of being enabled
    user_id: faker.string.uuid(),
    created_at: createdAt.toISOString(),
    updated_at: updatedAt.toISOString(),
    last_sync_at: lastSyncAt?.toISOString(),
    sync_status: faker.helpers.arrayElement(SYNC_STATUSES),
    ...(sourceType === 'webdav' && {
      webdav_config: {
        url: faker.internet.url(),
        username: faker.internet.userName(),
        password: '***', // Masked in responses
        path: '/documents',
      }
    }),
    ...(sourceType === 's3' && {
      s3_config: {
        bucket: faker.lorem.slug(2),
        region: faker.helpers.arrayElement(['us-east-1', 'us-west-2', 'eu-west-1', 'ap-southeast-1']),
        access_key_id: faker.string.alphanumeric(20).toUpperCase(),
        secret_access_key: '***', // Masked in responses
        prefix: faker.lorem.slug(1) + '/',
      }
    }),
    ...overrides,
  }
}

/**
 * Create multiple mock sources
 */
export const createMockSources = (
  count: number = 3,
  options: FactoryOptions = {}
): MockSource[] => {
  if (options.seed) {
    faker.seed(options.seed)
  }

  return Array.from({ length: count }, () => 
    createMockSource(options.overrides)
  )
}

/**
 * Create a mock source with specific characteristics
 */
export const createMockSourceWithScenario = (scenario: string): MockSource => {
  const baseSource = createMockSource()

  switch (scenario) {
    case 'local_folder_active':
      return {
        ...baseSource,
        name: 'Local Documents',
        source_type: 'local_folder',
        path: '/home/user/documents',
        enabled: true,
        sync_status: 'idle',
        last_sync_at: faker.date.recent().toISOString(),
      }

    case 'webdav_syncing':
      return {
        ...baseSource,
        name: 'NextCloud Documents',
        source_type: 'webdav',
        path: '/documents',
        enabled: true,
        sync_status: 'syncing',
        webdav_config: {
          url: 'https://cloud.example.com/remote.php/webdav',
          username: 'testuser',
          password: '***',
          path: '/documents',
        },
      }

    case 's3_error':
      return {
        ...baseSource,
        name: 'AWS S3 Bucket',
        source_type: 's3',
        path: 'company-docs/',
        enabled: true,
        sync_status: 'error',
        s3_config: {
          bucket: 'company-docs',
          region: 'us-east-1',
          access_key_id: 'AKIAIOSFODNN7EXAMPLE',
          secret_access_key: '***',
          prefix: 'documents/',
        },
      }

    case 'disabled_source':
      return {
        ...baseSource,
        enabled: false,
        sync_status: 'idle',
      }

    case 'never_synced':
      return {
        ...baseSource,
        last_sync_at: undefined,
        sync_status: 'idle',
      }

    case 'recently_created':
      const now = new Date()
      return {
        ...baseSource,
        created_at: new Date(now.getTime() - 30 * 60 * 1000).toISOString(), // 30 minutes ago
        updated_at: now.toISOString(),
        last_sync_at: undefined,
      }

    default:
      console.warn(`Unknown source scenario: ${scenario}`)
      return baseSource
  }
}

/**
 * Create sources for specific test scenarios
 */
export const createSourceScenarios = (): Record<string, MockSource[]> => ({
  empty: [],
  single_local: [createMockSourceWithScenario('local_folder_active')],
  single_webdav: [createMockSourceWithScenario('webdav_syncing')],
  single_s3: [createMockSourceWithScenario('s3_error')],
  mixed_types: [
    createMockSourceWithScenario('local_folder_active'),
    createMockSourceWithScenario('webdav_syncing'),
    createMockSourceWithScenario('s3_error'),
  ],
  with_disabled: [
    createMockSourceWithScenario('local_folder_active'),
    createMockSourceWithScenario('disabled_source'),
  ],
  sync_states: [
    createMockSource({ sync_status: 'idle' }),
    createMockSource({ sync_status: 'syncing' }),
    createMockSource({ sync_status: 'error' }),
  ],
  many: createMockSources(10),
})

/**
 * Reset faker seed for consistent test results
 */
export const resetSourceFactorySeed = (seed: number = 12345) => {
  faker.seed(seed)
}