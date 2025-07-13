import { vi } from 'vitest';
import { type LabelData } from '../components/Labels/Label';

/**
 * Test utilities for label-related tests
 */

// Counter for generating unique IDs
let labelIdCounter = 1;

export const createMockLabel = (overrides: Partial<LabelData> = {}): LabelData => ({
  id: `test-label-${labelIdCounter++}`,
  name: 'Test Label',
  description: 'A test label',
  color: '#0969da',
  background_color: undefined,
  icon: 'star',
  is_system: false,
  created_at: '2024-01-01T00:00:00Z',
  updated_at: '2024-01-01T00:00:00Z',
  document_count: 0,
  source_count: 0,
  ...overrides,
});

export const createMockSystemLabel = (overrides: Partial<LabelData> = {}): LabelData => ({
  id: `system-label-${labelIdCounter++}`,
  name: 'Important',
  description: 'A test label',
  color: '#d73a49',
  background_color: undefined,
  icon: 'star',
  is_system: true,
  created_at: '2024-01-01T00:00:00Z',
  updated_at: '2024-01-01T00:00:00Z',
  document_count: 10,
  source_count: 2,
  ...overrides,
});

export const createMockLabels = (count: number = 3): LabelData[] => {
  const labels: LabelData[] = [];
  
  for (let i = 0; i < count; i++) {
    labels.push(createMockLabel({
      id: `test-label-${i + 1}`,
      name: `Test Label ${i + 1}`,
      color: `#${Math.floor(Math.random() * 16777215).toString(16).padStart(6, '0')}`,
      document_count: Math.floor(Math.random() * 10),
    }));
  }
  
  return labels;
};

export const mockLabelApiResponses = {
  getLabels: (labels: LabelData[] = createMockLabels()) => ({
    data: labels,
  }),
  
  createLabel: (label: Partial<LabelData> = {}) => ({
    data: createMockLabel(label),
  }),
  
  updateLabel: (label: Partial<LabelData> = {}) => ({
    data: createMockLabel(label),
  }),
  
  deleteLabel: () => ({}),
  
  getDocumentLabels: (labels: LabelData[] = []) => ({
    data: labels,
  }),
  
  updateDocumentLabels: (labels: LabelData[] = []) => ({
    data: labels,
  }),
};

/**
 * Helper to wait for async operations in tests
 */
export const waitForAsync = () => new Promise(resolve => setTimeout(resolve, 0));

/**
 * Common test scenarios for label validation
 */
export const labelValidationScenarios = {
  validColors: [
    '#000000',
    '#ffffff',
    '#ff0000',
    '#00ff00',
    '#0000ff',
    '#0969da',
    '#d73a49',
    '#28a745',
  ],
  
  invalidColors: [
    'red',
    '#ff',
    '#gggggg',
    'rgb(255, 0, 0)',
    '#12345',
    '#1234567',
    '',
  ],
  
  validNames: [
    'Test',
    'Test Label',
    'Work-Related',
    'Personal_Project',
    'Label with Numbers 123',
    'A'.repeat(50), // Max length
  ],
  
  invalidNames: [
    '',
    ' ',
    '\t',
    '\n',
    'A'.repeat(51), // Too long
  ],
  
  validIcons: [
    'star',
    'work',
    'folder',
    'archive',
    'person',
    'receipt',
    'scale',
    'medical',
    'dollar',
    'briefcase',
  ],
  
  validDescriptions: [
    undefined,
    '',
    'Short description',
    'A longer description that provides more context about the label',
    'Description with special characters: @#$%^&*()',
  ],
};

/**
 * Mock API client for testing
 */
export const createMockApiClient = () => {
  const mockGet = vi.fn();
  const mockPost = vi.fn();
  const mockPut = vi.fn();
  const mockDelete = vi.fn();
  
  const api = {
    get: mockGet,
    post: mockPost,
    put: mockPut,
    delete: mockDelete,
    defaults: {
      headers: {
        common: {},
      },
    },
  };
  
  // Set up default successful responses
  mockGet.mockResolvedValue(mockLabelApiResponses.getLabels());
  mockPost.mockResolvedValue(mockLabelApiResponses.createLabel());
  mockPut.mockResolvedValue(mockLabelApiResponses.updateLabel());
  mockDelete.mockResolvedValue(mockLabelApiResponses.deleteLabel());
  
  return {
    api,
    mockGet,
    mockPost,
    mockPut,
    mockDelete,
  };
};

/**
 * Color contrast testing utility
 */
export const testColorContrast = (backgroundColor: string, textColor: string): number => {
  // Simplified WCAG contrast ratio calculation for testing
  const getLuminance = (color: string): number => {
    const hex = color.replace('#', '');
    const r = parseInt(hex.substr(0, 2), 16) / 255;
    const g = parseInt(hex.substr(2, 2), 16) / 255;
    const b = parseInt(hex.substr(4, 2), 16) / 255;
    
    const sRGB = [r, g, b].map(channel => {
      return channel <= 0.03928 
        ? channel / 12.92 
        : Math.pow((channel + 0.055) / 1.055, 2.4);
    });
    
    return 0.2126 * sRGB[0] + 0.7152 * sRGB[1] + 0.0722 * sRGB[2];
  };
  
  const bgLuminance = getLuminance(backgroundColor);
  const textLuminance = getLuminance(textColor);
  
  const lighter = Math.max(bgLuminance, textLuminance);
  const darker = Math.min(bgLuminance, textLuminance);
  
  return (lighter + 0.05) / (darker + 0.05);
};

/**
 * Test data builders for complex scenarios
 */
export const testDataBuilders = {
  /**
   * Creates a set of labels that represent typical usage patterns
   */
  createTypicalLabelSet: (): LabelData[] => [
    createMockSystemLabel({ name: 'Important', description: 'High priority items', color: '#d73a49', icon: 'star' }),
    createMockSystemLabel({ name: 'Work', description: 'Work-related documents', color: '#0969da', icon: 'work' }),
    createMockSystemLabel({ name: 'Personal', description: 'Personal documents', color: '#28a745', icon: 'person' }),
    createMockLabel({ name: 'Project Alpha', description: 'My personal project files', color: '#8250df', icon: 'folder' }),
    createMockLabel({ name: 'Invoices', description: 'Financial documents', color: '#fb8500', icon: 'receipt' }),
    createMockLabel({ name: 'Archive', description: 'Archived items', color: '#6e7781', icon: 'archive', document_count: 0 }),
  ],
  
  /**
   * Creates labels for testing edge cases
   */
  createEdgeCaseLabels: (): LabelData[] => [
    createMockLabel({ 
      name: 'Very Long Label Name That Might Cause Layout Issues',
      description: 'This is a very long description that might cause text wrapping and layout issues in various UI components and should be handled gracefully',
      color: '#000000',
    }),
    createMockLabel({ 
      name: 'Special Chars & Symbols <>"\'',
      description: 'Label with special characters: @#$%^&*()[]{}|\\:";\'<>?,./',
      color: '#ffffff',
    }),
    createMockLabel({ 
      name: 'Unicode Test 🏷️ 📋 ⭐',
      description: 'Testing unicode characters and emojis',
      color: '#ff69b4',
    }),
  ],
  
  /**
   * Creates a large dataset for performance testing
   */
  createLargeDataset: (size: number = 100): LabelData[] => {
    const labels: LabelData[] = [];
    const colors = ['#ff0000', '#00ff00', '#0000ff', '#ffff00', '#ff00ff', '#00ffff'];
    const icons = ['star', 'work', 'folder', 'archive', 'person', 'receipt'];
    
    for (let i = 0; i < size; i++) {
      labels.push(createMockLabel({
        id: `label-${i}`,
        name: `Label ${i.toString().padStart(3, '0')}`,
        description: `Description for label ${i}`,
        color: colors[i % colors.length],
        icon: icons[i % icons.length],
        is_system: i < 10, // First 10 are system labels
        document_count: Math.floor(Math.random() * 50),
        source_count: Math.floor(Math.random() * 5),
      }));
    }
    
    return labels;
  },
};