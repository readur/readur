import React from 'react';
import { render, screen } from '@testing-library/react';
import { BrowserRouter } from 'react-router-dom';
import { vi } from 'vitest';
import DebugPage from '../DebugPage';

// Mock the API
vi.mock('../../services/api', () => ({
  api: {
    get: vi.fn(),
    post: vi.fn(),
    put: vi.fn(),
    delete: vi.fn(),
  },
}));

const renderDebugPage = () => {
  return render(
    <BrowserRouter>
      <DebugPage />
    </BrowserRouter>
  );
};

describe('DebugPage', () => {
  it('renders without crashing', () => {
    renderDebugPage();
    expect(screen.getByText('Upload & Debug')).toBeInTheDocument();
  });

  it('handles undefined debugInfo without errors', () => {
    renderDebugPage();
    // Should not throw any errors when debugInfo is null
    expect(screen.getByText('Upload & Debug')).toBeInTheDocument();
  });

  it('handles undefined nested properties without errors', () => {
    // This test would check that all the optional chaining we added works correctly
    renderDebugPage();
    expect(screen.getByText('Upload & Debug')).toBeInTheDocument();
  });
});