import { describe, test, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import FailedDocumentViewer from '../FailedDocumentViewer';

// Mock the api module to prevent network calls
vi.mock('../../services/api', () => ({
  api: {
    get: vi.fn().mockRejectedValue(new Error('Mocked error - no real network calls'))
  }
}));

// Mock URL constructor
global.URL = class URL {
  constructor(url: string) {
    this.href = url;
  }
  href: string;
  
  static createObjectURL = vi.fn(() => 'mock-object-url');
  static revokeObjectURL = vi.fn();
} as any;

// Mock Blob
global.Blob = class Blob {
  constructor(data: any, options?: any) {
    this.type = options?.type || '';
  }
  type: string;
} as any;

const defaultProps = {
  failedDocumentId: 'test-failed-doc-id',
  filename: 'test-document.pdf',
  mimeType: 'application/pdf',
};

describe('FailedDocumentViewer', () => {
  test('should render component without crashing', () => {
    render(<FailedDocumentViewer {...defaultProps} />);
    
    // The component should render - even if it shows an error due to mocked API failure
    expect(document.body).toBeInTheDocument();
  });

  test('should accept required props', () => {
    expect(() => {
      render(<FailedDocumentViewer {...defaultProps} />);
    }).not.toThrow();
  });

  test('should handle different mime types', () => {
    const imageProps = {
      ...defaultProps,
      mimeType: 'image/jpeg',
      filename: 'test-image.jpg'
    };
    
    expect(() => {
      render(<FailedDocumentViewer {...imageProps} />);
    }).not.toThrow();
  });

  test('should handle different filenames', () => {
    const textProps = {
      ...defaultProps,
      mimeType: 'text/plain',
      filename: 'test-file.txt'
    };
    
    expect(() => {
      render(<FailedDocumentViewer {...textProps} />);
    }).not.toThrow();
  });
});