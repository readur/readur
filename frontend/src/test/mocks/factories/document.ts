/**
 * Document factory for generating realistic test data
 * Uses Faker.js to create consistent, varied mock documents
 */

import { faker } from '@faker-js/faker'
import { MockDocument, FactoryOptions } from '../api/types'

// Set a default seed for reproducible tests
faker.seed(12345)

const MIME_TYPES = [
  'application/pdf',
  'image/jpeg',
  'image/png',
  'text/plain',
  'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
  'application/vnd.ms-excel',
  'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
  'image/gif',
  'image/tiff',
  'text/html',
  'application/json',
  'text/csv',
] as const

const FILE_EXTENSIONS = {
  'application/pdf': 'pdf',
  'image/jpeg': 'jpg',
  'image/png': 'png',
  'text/plain': 'txt',
  'application/vnd.openxmlformats-officedocument.wordprocessingml.document': 'docx',
  'application/vnd.ms-excel': 'xls',
  'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet': 'xlsx',
  'image/gif': 'gif',
  'image/tiff': 'tiff',
  'text/html': 'html',
  'application/json': 'json',
  'text/csv': 'csv',
} as const

const OCR_STATUSES = ['completed', 'failed', 'pending', 'processing'] as const

const SOURCE_TYPES = ['local_folder', 'webdav', 's3', 'upload'] as const

/**
 * Create a single mock document
 */
export const createMockDocument = (overrides: Partial<MockDocument> = {}): MockDocument => {
  const id = faker.string.uuid()
  const mimeType = faker.helpers.arrayElement(MIME_TYPES)
  const extension = FILE_EXTENSIONS[mimeType]
  const fileName = `${faker.system.fileName({ extensionCount: 0 })}.${extension}`
  const createdAt = faker.date.past()
  const hasOcrText = mimeType.startsWith('image/') || mimeType === 'application/pdf'
  const ocrStatus = hasOcrText ? faker.helpers.arrayElement(OCR_STATUSES) : undefined
  
  return {
    id,
    filename: fileName,
    original_filename: fileName,
    file_path: `/documents/${id}/${fileName}`,
    file_size: faker.number.int({ min: 1024, max: 50 * 1024 * 1024 }), // 1KB to 50MB
    mime_type: mimeType,
    tags: faker.helpers.arrayElements([
      'important',
      'work',
      'personal',
      'invoice',
      'receipt',
      'contract',
      'report',
      'presentation',
      'draft',
      'final',
    ], { min: 0, max: 3 }),
    created_at: createdAt.toISOString(),
    updated_at: faker.date.between({ from: createdAt, to: new Date() }).toISOString(),
    user_id: faker.string.uuid(),
    username: faker.internet.userName(),
    file_hash: faker.string.alphanumeric(64),
    original_created_at: faker.date.past({ years: 2 }).toISOString(),
    original_modified_at: faker.date.past({ years: 1 }).toISOString(),
    source_path: faker.system.filePath(),
    source_type: faker.helpers.arrayElement(SOURCE_TYPES),
    source_id: faker.string.uuid(),
    file_permissions: faker.number.int({ min: 600, max: 777 }),
    file_owner: faker.internet.userName(),
    file_group: faker.helpers.arrayElement(['users', 'staff', 'admin', 'documents']),
    source_metadata: {
      sync_batch_id: faker.string.uuid(),
      discovered_at: faker.date.recent().toISOString(),
      checksum: faker.string.alphanumeric(32),
    },
    has_ocr_text: hasOcrText,
    ocr_confidence: hasOcrText && ocrStatus === 'completed' 
      ? faker.number.float({ min: 0.1, max: 1.0, fractionDigits: 3 })
      : undefined,
    ocr_word_count: hasOcrText && ocrStatus === 'completed'
      ? faker.number.int({ min: 10, max: 5000 })
      : undefined,
    ocr_processing_time_ms: hasOcrText && ocrStatus === 'completed'
      ? faker.number.int({ min: 100, max: 30000 })
      : undefined,
    ocr_status: ocrStatus,
    _mockId: `mock-doc-${faker.string.alphanumeric(8)}`,
    ...overrides,
  }
}

/**
 * Create multiple mock documents
 */
export const createMockDocuments = (
  count: number = 10,
  options: FactoryOptions = {}
): MockDocument[] => {
  if (options.seed) {
    faker.seed(options.seed)
  }

  return Array.from({ length: count }, () => 
    createMockDocument(options.overrides)
  )
}

/**
 * Create a mock document with specific characteristics
 */
export const createMockDocumentWithScenario = (scenario: string): MockDocument => {
  const baseDoc = createMockDocument()

  switch (scenario) {
    case 'pdf_with_high_confidence_ocr':
      return {
        ...baseDoc,
        mime_type: 'application/pdf',
        filename: 'high-quality-document.pdf',
        has_ocr_text: true,
        ocr_status: 'completed',
        ocr_confidence: 0.95,
        ocr_word_count: 1250,
        ocr_processing_time_ms: 2500,
        _scenario: scenario,
      }

    case 'image_with_failed_ocr':
      return {
        ...baseDoc,
        mime_type: 'image/jpeg',
        filename: 'blurry-scan.jpg',
        has_ocr_text: false,
        ocr_status: 'failed',
        ocr_confidence: undefined,
        ocr_word_count: undefined,
        ocr_processing_time_ms: undefined,
        _scenario: scenario,
      }

    case 'large_pdf_pending_ocr':
      return {
        ...baseDoc,
        mime_type: 'application/pdf',
        filename: 'large-document.pdf',
        file_size: 25 * 1024 * 1024, // 25MB
        has_ocr_text: false,
        ocr_status: 'pending',
        _scenario: scenario,
      }

    case 'text_file_no_ocr':
      return {
        ...baseDoc,
        mime_type: 'text/plain',
        filename: 'notes.txt',
        has_ocr_text: false,
        ocr_status: undefined,
        ocr_confidence: undefined,
        ocr_word_count: undefined,
        _scenario: scenario,
      }

    case 'duplicate_document':
      return {
        ...baseDoc,
        filename: 'duplicate-file.pdf',
        file_hash: 'duplicate-hash-12345',
        _scenario: scenario,
      }

    case 'recently_uploaded':
      const now = new Date()
      return {
        ...baseDoc,
        created_at: new Date(now.getTime() - 5 * 60 * 1000).toISOString(), // 5 minutes ago
        updated_at: now.toISOString(),
        source_type: 'upload',
        _scenario: scenario,
      }

    case 'webdav_synced':
      return {
        ...baseDoc,
        source_type: 'webdav',
        source_path: '/webdav/documents/reports/quarterly-report.pdf',
        source_metadata: {
          ...baseDoc.source_metadata,
          webdav_url: 'https://cloud.example.com/webdav/',
          etag: faker.string.alphanumeric(32),
          last_modified: faker.date.recent().toISOString(),
        },
        _scenario: scenario,
      }

    case 's3_synced':
      return {
        ...baseDoc,
        source_type: 's3',
        source_path: 's3://company-docs/invoices/2024/invoice-001.pdf',
        source_metadata: {
          ...baseDoc.source_metadata,
          s3_bucket: 'company-docs',
          s3_key: 'invoices/2024/invoice-001.pdf',
          s3_etag: faker.string.alphanumeric(32),
          s3_version_id: faker.string.uuid(),
        },
        _scenario: scenario,
      }

    default:
      console.warn(`Unknown document scenario: ${scenario}`)
      return { ...baseDoc, _scenario: scenario }
  }
}

/**
 * Create documents for specific test scenarios
 */
export const createDocumentScenarios = (): Record<string, MockDocument[]> => ({
  empty: [],
  single: [createMockDocument()],
  few: createMockDocuments(3),
  many: createMockDocuments(25),
  mixed_mime_types: [
    createMockDocumentWithScenario('pdf_with_high_confidence_ocr'),
    createMockDocumentWithScenario('image_with_failed_ocr'),
    createMockDocumentWithScenario('text_file_no_ocr'),
  ],
  ocr_scenarios: [
    createMockDocumentWithScenario('pdf_with_high_confidence_ocr'),
    createMockDocumentWithScenario('image_with_failed_ocr'),
    createMockDocumentWithScenario('large_pdf_pending_ocr'),
  ],
  source_scenarios: [
    createMockDocumentWithScenario('recently_uploaded'),
    createMockDocumentWithScenario('webdav_synced'),
    createMockDocumentWithScenario('s3_synced'),
  ],
  duplicates: [
    createMockDocumentWithScenario('duplicate_document'),
    createMockDocumentWithScenario('duplicate_document'),
  ],
})

/**
 * Reset faker seed for consistent test results
 */
export const resetDocumentFactorySeed = (seed: number = 12345) => {
  faker.seed(seed)
}