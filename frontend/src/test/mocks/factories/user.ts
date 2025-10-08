/**
 * User factory for generating realistic test data
 */

import { faker } from '@faker-js/faker'
import { MockUser, FactoryOptions } from '../api/types'

faker.seed(12345)

const USER_ROLES = ['user', 'admin'] as const

/**
 * Create a single mock user
 */
export const createMockUser = (overrides: Partial<MockUser> = {}): MockUser => {
  const firstName = faker.person.firstName()
  const lastName = faker.person.lastName()
  const username = faker.internet.userName({ firstName, lastName })
  const email = faker.internet.email({ firstName, lastName })
  const createdAt = faker.date.past({ years: 2 })

  return {
    id: faker.string.uuid(),
    username,
    email,
    role: faker.helpers.arrayElement(USER_ROLES),
    created_at: createdAt.toISOString(),
    is_active: faker.datatype.boolean(0.9), // 90% chance of being active
    oidc_sub: faker.datatype.boolean(0.3) ? faker.string.uuid() : undefined, // 30% chance of OIDC
    ...overrides,
  }
}

/**
 * Create multiple mock users
 */
export const createMockUsers = (
  count: number = 5,
  options: FactoryOptions = {}
): MockUser[] => {
  if (options.seed) {
    faker.seed(options.seed)
  }

  return Array.from({ length: count }, () => 
    createMockUser(options.overrides)
  )
}

/**
 * Create a mock user with specific characteristics
 */
export const createMockUserWithScenario = (scenario: string): MockUser => {
  const baseUser = createMockUser()

  switch (scenario) {
    case 'admin_user':
      return {
        ...baseUser,
        username: 'admin',
        email: 'admin@readur.local',
        role: 'admin',
        is_active: true,
      }

    case 'regular_user':
      return {
        ...baseUser,
        username: 'testuser',
        email: 'user@readur.local',
        role: 'user',
        is_active: true,
      }

    case 'inactive_user':
      return {
        ...baseUser,
        is_active: false,
      }

    case 'oidc_user':
      return {
        ...baseUser,
        oidc_sub: `oidc|${faker.string.alphanumeric(24)}`,
        email: faker.internet.email(),
      }

    case 'new_user':
      const now = new Date()
      return {
        ...baseUser,
        created_at: new Date(now.getTime() - 24 * 60 * 60 * 1000).toISOString(), // 1 day ago
      }

    default:
      console.warn(`Unknown user scenario: ${scenario}`)
      return baseUser
  }
}

/**
 * Create users for specific test scenarios
 */
export const createUserScenarios = (): Record<string, MockUser[]> => ({
  empty: [],
  single_admin: [createMockUserWithScenario('admin_user')],
  single_user: [createMockUserWithScenario('regular_user')],
  mixed_roles: [
    createMockUserWithScenario('admin_user'),
    createMockUserWithScenario('regular_user'),
    createMockUser({ role: 'user' }),
    createMockUser({ role: 'admin' }),
  ],
  with_inactive: [
    createMockUserWithScenario('regular_user'),
    createMockUserWithScenario('inactive_user'),
  ],
  oidc_users: [
    createMockUserWithScenario('oidc_user'),
    createMockUserWithScenario('oidc_user'),
  ],
  many: createMockUsers(20),
})

/**
 * Get a default authenticated user for testing
 */
export const getDefaultTestUser = (): MockUser => 
  createMockUserWithScenario('regular_user')

/**
 * Get a default admin user for testing
 */
export const getDefaultAdminUser = (): MockUser => 
  createMockUserWithScenario('admin_user')

/**
 * Reset faker seed for consistent test results
 */
export const resetUserFactorySeed = (seed: number = 12345) => {
  faker.seed(seed)
}