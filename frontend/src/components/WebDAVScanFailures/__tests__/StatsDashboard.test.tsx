import React from 'react';
import { render, screen } from '@testing-library/react';
import { vi, describe, it, expect } from 'vitest';
import { ThemeProvider } from '@mui/material/styles';

import StatsDashboard from '../StatsDashboard';
import { WebDAVScanFailureStats } from '../../../services/api';
import theme from '../../../theme';

const renderWithTheme = (component: React.ReactElement) => {
  return render(
    <ThemeProvider theme={theme}>
      {component}
    </ThemeProvider>
  );
};

const mockStats: WebDAVScanFailureStats = {
  active_failures: 15,
  resolved_failures: 35,
  excluded_directories: 5,
  critical_failures: 3,
  high_failures: 7,
  medium_failures: 4,
  low_failures: 1,
  ready_for_retry: 8,
};

describe('StatsDashboard', () => {
  it('renders all stat cards with correct values', () => {
    renderWithTheme(<StatsDashboard stats={mockStats} />);

    // Check title
    expect(screen.getByText('Scan Failure Statistics')).toBeInTheDocument();

    // Check individual stat cards
    expect(screen.getByText('15')).toBeInTheDocument(); // Active failures
    expect(screen.getByText('3')).toBeInTheDocument(); // Critical failures
    expect(screen.getByText('7')).toBeInTheDocument(); // High failures
    expect(screen.getByText('4')).toBeInTheDocument(); // Medium failures
    expect(screen.getByText('1')).toBeInTheDocument(); // Low failures
    expect(screen.getByText('8')).toBeInTheDocument(); // Ready for retry
    expect(screen.getByText('35')).toBeInTheDocument(); // Resolved failures
    expect(screen.getByText('5')).toBeInTheDocument(); // Excluded directories

    // Check labels
    expect(screen.getByText('Active Failures')).toBeInTheDocument();
    expect(screen.getByText('Critical')).toBeInTheDocument();
    expect(screen.getByText('High Priority')).toBeInTheDocument();
    expect(screen.getByText('Medium Priority')).toBeInTheDocument();
    expect(screen.getByText('Low Priority')).toBeInTheDocument();
    expect(screen.getByText('Ready for Retry')).toBeInTheDocument();
    expect(screen.getByText('Resolved Failures')).toBeInTheDocument();
    expect(screen.getByText('Excluded Directories')).toBeInTheDocument();
  });

  it('calculates success rate correctly', () => {
    renderWithTheme(<StatsDashboard stats={mockStats} />);

    // Total failures = active (15) + resolved (35) = 50
    // Success rate = resolved (35) / total (50) = 70%
    expect(screen.getByText('70.0%')).toBeInTheDocument();
    expect(screen.getByText('35 of 50 failures resolved')).toBeInTheDocument();
  });

  it('displays 100% success rate when no failures exist', () => {
    const noFailuresStats: WebDAVScanFailureStats = {
      active_failures: 0,
      resolved_failures: 0,
      excluded_directories: 0,
      critical_failures: 0,
      high_failures: 0,
      medium_failures: 0,
      low_failures: 0,
      ready_for_retry: 0,
    };

    renderWithTheme(<StatsDashboard stats={noFailuresStats} />);

    expect(screen.getByText('100%')).toBeInTheDocument();
  });

  it('calculates percentages correctly for severity breakdown', () => {
    renderWithTheme(<StatsDashboard stats={mockStats} />);

    // Total failures = 50
    // Critical: 3/50 = 6%
    // High: 7/50 = 14%
    // Medium: 4/50 = 8%
    // Low: 1/50 = 2%
    expect(screen.getByText('6.0% of total')).toBeInTheDocument();
    expect(screen.getByText('14.0% of total')).toBeInTheDocument();
    expect(screen.getByText('8.0% of total')).toBeInTheDocument();
    expect(screen.getByText('2.0% of total')).toBeInTheDocument();
  });

  it('calculates retry percentage correctly', () => {
    renderWithTheme(<StatsDashboard stats={mockStats} />);

    // Ready for retry: 8/15 active failures = 53.3%
    expect(screen.getByText('53.3% of total')).toBeInTheDocument();
  });

  it('renders loading state with skeletons', () => {
    renderWithTheme(<StatsDashboard stats={mockStats} isLoading={true} />);

    // Should show skeleton cards instead of actual data
    const skeletons = document.querySelectorAll('.MuiSkeleton-root');
    expect(skeletons.length).toBeGreaterThan(0);
  });

  it('handles zero active failures for retry percentage', () => {
    const zeroActiveStats: WebDAVScanFailureStats = {
      ...mockStats,
      active_failures: 0,
      ready_for_retry: 0,
    };

    renderWithTheme(<StatsDashboard stats={zeroActiveStats} />);

    // Should not crash and should show 0% for retry percentage
    expect(screen.getByText('0')).toBeInTheDocument(); // Active failures
    expect(screen.getByText('0.0% of total')).toBeInTheDocument(); // Retry percentage when no active failures
  });

  it('displays descriptive text for each stat', () => {
    renderWithTheme(<StatsDashboard stats={mockStats} />);

    // Check descriptions
    expect(screen.getByText('Requiring attention')).toBeInTheDocument();
    expect(screen.getByText('Immediate action needed')).toBeInTheDocument();
    expect(screen.getByText('Important issues')).toBeInTheDocument();
    expect(screen.getByText('Moderate issues')).toBeInTheDocument();
    expect(screen.getByText('Minor issues')).toBeInTheDocument();
    expect(screen.getByText('Can be retried now')).toBeInTheDocument();
    expect(screen.getByText('Successfully resolved')).toBeInTheDocument();
    expect(screen.getByText('Manually excluded')).toBeInTheDocument();
  });

  it('applies correct hover effects to cards', () => {
    renderWithTheme(<StatsDashboard stats={mockStats} />);

    const cards = document.querySelectorAll('.MuiCard-root');
    expect(cards.length).toBeGreaterThan(0);

    // Cards should have transition styles for hover effects
    cards.forEach(card => {
      expect(card).toHaveStyle('transition: all 0.2s ease-in-out');
    });
  });
});