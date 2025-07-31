import { describe, test, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { ProgressStatistics } from '../ProgressStatistics';
import { SyncProgressInfo } from '../../../services/api';

const createMockProgressInfo = (overrides: Partial<SyncProgressInfo> = {}): SyncProgressInfo => ({
  source_id: 'test-source-123',
  phase: 'processing_files',
  phase_description: 'Processing files',
  elapsed_time_secs: 120,
  directories_found: 10,
  directories_processed: 7,
  files_found: 50,
  files_processed: 30,
  bytes_processed: 1024000,
  processing_rate_files_per_sec: 2.5,
  files_progress_percent: 60.0,
  estimated_time_remaining_secs: 80,
  current_directory: '/Documents/Projects',
  current_file: 'important-document.pdf',
  errors: 0,
  warnings: 0,
  is_active: true,
  ...overrides,
});

describe('ProgressStatistics', () => {
  test('should display file progress', () => {
    const progressInfo = createMockProgressInfo();
    render(<ProgressStatistics progressInfo={progressInfo} phaseColor="#1976d2" />);

    expect(screen.getByText('30 / 50 files (60.0%)')).toBeInTheDocument();
    expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '60');
  });

  test('should not show progress bar when no files found', () => {
    const progressInfo = createMockProgressInfo({ files_found: 0 });
    render(<ProgressStatistics progressInfo={progressInfo} phaseColor="#1976d2" />);

    expect(screen.queryByRole('progressbar')).not.toBeInTheDocument();
  });

  test('should display directory statistics', () => {
    const progressInfo = createMockProgressInfo();
    render(<ProgressStatistics progressInfo={progressInfo} phaseColor="#1976d2" />);

    expect(screen.getByText('7 / 10')).toBeInTheDocument();
  });

  test('should format bytes correctly', () => {
    const testCases = [
      { bytes: 0, expected: '0 B' },
      { bytes: 1024, expected: '1 KB' },
      { bytes: 1048576, expected: '1 MB' },
      { bytes: 1073741824, expected: '1 GB' },
    ];

    testCases.forEach(({ bytes, expected }) => {
      const { rerender } = render(
        <ProgressStatistics 
          progressInfo={createMockProgressInfo({ bytes_processed: bytes })} 
          phaseColor="#1976d2" 
        />
      );
      expect(screen.getByText(expected)).toBeInTheDocument();
      rerender(<div />); // Clear for next test
    });
  });

  test('should display processing rate', () => {
    const progressInfo = createMockProgressInfo({ processing_rate_files_per_sec: 3.7 });
    render(<ProgressStatistics progressInfo={progressInfo} phaseColor="#1976d2" />);

    expect(screen.getByText('3.7 files/sec')).toBeInTheDocument();
  });

  test('should format duration correctly', () => {
    const testCases = [
      { seconds: 45, expected: '45s' },
      { seconds: 90, expected: '1m 30s' },
      { seconds: 3661, expected: '1h 1m' },
    ];

    testCases.forEach(({ seconds, expected }) => {
      const { rerender } = render(
        <ProgressStatistics 
          progressInfo={createMockProgressInfo({ elapsed_time_secs: seconds })} 
          phaseColor="#1976d2" 
        />
      );
      expect(screen.getByText(expected)).toBeInTheDocument();
      rerender(<div />); // Clear for next test
    });
  });

  test('should display estimated time remaining', () => {
    const progressInfo = createMockProgressInfo({ estimated_time_remaining_secs: 150 });
    render(<ProgressStatistics progressInfo={progressInfo} phaseColor="#1976d2" />);

    expect(screen.getByText(/Estimated time remaining: 2m 30s/)).toBeInTheDocument();
  });

  test('should not show estimated time when unavailable', () => {
    const progressInfo = createMockProgressInfo({ estimated_time_remaining_secs: undefined });
    render(<ProgressStatistics progressInfo={progressInfo} phaseColor="#1976d2" />);

    expect(screen.queryByText(/Estimated time remaining/)).not.toBeInTheDocument();
  });

  test('should display current directory and file', () => {
    const progressInfo = createMockProgressInfo();
    render(<ProgressStatistics progressInfo={progressInfo} phaseColor="#1976d2" />);

    expect(screen.getByText('/Documents/Projects')).toBeInTheDocument();
    expect(screen.getByText('important-document.pdf')).toBeInTheDocument();
  });

  test('should not show current file when unavailable', () => {
    const progressInfo = createMockProgressInfo({ current_file: undefined });
    render(<ProgressStatistics progressInfo={progressInfo} phaseColor="#1976d2" />);

    expect(screen.getByText('/Documents/Projects')).toBeInTheDocument();
    expect(screen.queryByText('Current File')).not.toBeInTheDocument();
  });

  test('should display errors and warnings', () => {
    const progressInfo = createMockProgressInfo({ errors: 2, warnings: 5 });
    render(<ProgressStatistics progressInfo={progressInfo} phaseColor="#1976d2" />);

    expect(screen.getByText('2 errors')).toBeInTheDocument();
    expect(screen.getByText('5 warnings')).toBeInTheDocument();
  });

  test('should handle singular error/warning labels', () => {
    const progressInfo = createMockProgressInfo({ errors: 1, warnings: 1 });
    render(<ProgressStatistics progressInfo={progressInfo} phaseColor="#1976d2" />);

    expect(screen.getByText('1 error')).toBeInTheDocument();
    expect(screen.getByText('1 warning')).toBeInTheDocument();
  });

  test('should not show errors/warnings when zero', () => {
    const progressInfo = createMockProgressInfo({ errors: 0, warnings: 0 });
    render(<ProgressStatistics progressInfo={progressInfo} phaseColor="#1976d2" />);

    expect(screen.queryByText(/error/)).not.toBeInTheDocument();
    expect(screen.queryByText(/warning/)).not.toBeInTheDocument();
  });
});