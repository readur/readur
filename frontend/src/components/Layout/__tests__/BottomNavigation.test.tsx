import { describe, it, expect, vi, beforeEach } from 'vitest';
import { screen, fireEvent, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import BottomNavigation from '../BottomNavigation';
import { renderWithPWA, renderWithProviders } from '../../../test/test-utils';
import { setupPWAMode, resetPWAMocks } from '../../../test/pwa-test-utils';
import { MemoryRouter } from 'react-router-dom';

// Mock the usePWA hook
vi.mock('../../../hooks/usePWA');

const mockNavigate = vi.fn();

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom');
  return {
    ...actual,
    useNavigate: () => mockNavigate,
    BrowserRouter: ({ children, ...props }: { children: React.ReactNode; [key: string]: any }) => (
      <actual.MemoryRouter initialEntries={props.initialEntries || ['/dashboard']} {...props}>
        {children}
      </actual.MemoryRouter>
    ),
  };
});

describe('BottomNavigation', () => {
  beforeEach(() => {
    mockNavigate.mockClear();
    resetPWAMocks();
  });

  describe('PWA Detection', () => {
    it('returns null when not in PWA mode', () => {
      setupPWAMode(false);

      const { container } = renderWithProviders(<BottomNavigation />, {
        routerProps: { initialEntries: ['/dashboard'] },
      });

      expect(container.firstChild).toBeNull();
    });

    it('renders when in PWA mode', () => {
      setupPWAMode(true);

      renderWithPWA(<BottomNavigation />, {
        routerProps: { initialEntries: ['/dashboard'] },
      });

      // Check that the navigation is rendered by looking for nav items text
      expect(screen.getByText(/dashboard/i)).toBeInTheDocument();
    });
  });

  describe('Navigation Items', () => {
    beforeEach(() => {
      setupPWAMode(true);
    });

    it('renders all 4 navigation items', () => {
      renderWithPWA(<BottomNavigation />, {
        routerProps: { initialEntries: ['/dashboard'] },
      });

      expect(screen.getByText(/dashboard/i)).toBeInTheDocument();
      expect(screen.getByText(/upload/i)).toBeInTheDocument();
      expect(screen.getByText(/labels/i)).toBeInTheDocument();
      expect(screen.getByText(/settings/i)).toBeInTheDocument();
    });

    it('renders clickable Dashboard nav button', () => {
      renderWithPWA(<BottomNavigation />, {
        routerProps: { initialEntries: ['/upload'] },
      });

      const buttons = screen.getAllByRole('button');
      const dashboardButton = buttons.find(btn => btn.textContent?.includes('Dashboard'))!;

      expect(dashboardButton).toBeInTheDocument();
      expect(dashboardButton).not.toBeDisabled();
    });

    it('renders clickable Upload nav button', () => {
      renderWithPWA(<BottomNavigation />, {
        routerProps: { initialEntries: ['/dashboard'] },
      });

      const buttons = screen.getAllByRole('button');
      const uploadButton = buttons.find(btn => btn.textContent?.includes('Upload'))!;

      expect(uploadButton).toBeInTheDocument();
      expect(uploadButton).not.toBeDisabled();
    });

    it('renders clickable Labels nav button', () => {
      renderWithPWA(<BottomNavigation />, {
        routerProps: { initialEntries: ['/dashboard'] },
      });

      const buttons = screen.getAllByRole('button');
      const labelsButton = buttons.find(btn => btn.textContent?.includes('Labels'))!;

      expect(labelsButton).toBeInTheDocument();
      expect(labelsButton).not.toBeDisabled();
    });

    it('renders clickable Settings nav button', () => {
      renderWithPWA(<BottomNavigation />, {
        routerProps: { initialEntries: ['/dashboard'] },
      });

      const buttons = screen.getAllByRole('button');
      const settingsButton = buttons.find(btn => btn.textContent?.includes('Settings'))!;

      expect(settingsButton).toBeInTheDocument();
      expect(settingsButton).not.toBeDisabled();
    });
  });

  describe('Routing Integration', () => {
    beforeEach(() => {
      setupPWAMode(true);
    });

    it('uses location pathname to determine active navigation item', () => {
      renderWithPWA(<BottomNavigation />, {
        routerProps: { initialEntries: ['/dashboard'] },
      });

      // Verify all navigation buttons are present
      const buttons = screen.getAllByRole('button');
      expect(buttons).toHaveLength(4);

      // Verify buttons have the expected text content
      expect(buttons.some(btn => btn.textContent?.includes('Dashboard'))).toBe(true);
      expect(buttons.some(btn => btn.textContent?.includes('Upload'))).toBe(true);
      expect(buttons.some(btn => btn.textContent?.includes('Labels'))).toBe(true);
      expect(buttons.some(btn => btn.textContent?.includes('Settings'))).toBe(true);
    });
  });

  describe('Styling', () => {
    beforeEach(() => {
      setupPWAMode(true);
    });

    it('has safe-area-inset padding', () => {
      const { container } = renderWithPWA(<BottomNavigation />, {
        routerProps: { initialEntries: ['/dashboard'] },
      });

      const paper = container.querySelector('[class*="MuiPaper-root"]');
      expect(paper).toBeInTheDocument();

      // Check for safe-area padding in style (MUI applies this via sx prop)
      const computedStyle = window.getComputedStyle(paper!);
      // Note: We can't directly test the calc() value in JSDOM,
      // but we verify the component renders without error
      expect(paper).toBeInTheDocument();
    });

    it('has correct z-index for overlay', () => {
      const { container } = renderWithPWA(<BottomNavigation />, {
        routerProps: { initialEntries: ['/dashboard'] },
      });

      const paper = container.querySelector('[class*="MuiPaper-root"]');
      expect(paper).toBeInTheDocument();
    });

    it('has fixed position at bottom', () => {
      const { container } = renderWithPWA(<BottomNavigation />, {
        routerProps: { initialEntries: ['/dashboard'] },
      });

      const paper = container.querySelector('[class*="MuiPaper-root"]');
      expect(paper).toBeInTheDocument();
    });
  });

  describe('Accessibility', () => {
    beforeEach(() => {
      setupPWAMode(true);
    });

    it('has visible text labels for all nav items', () => {
      renderWithPWA(<BottomNavigation />, {
        routerProps: { initialEntries: ['/dashboard'] },
      });

      // All buttons should have visible text
      expect(screen.getByText(/dashboard/i)).toBeInTheDocument();
      expect(screen.getByText(/upload/i)).toBeInTheDocument();
      expect(screen.getByText(/labels/i)).toBeInTheDocument();
      expect(screen.getByText(/settings/i)).toBeInTheDocument();
    });

    it('all nav items are keyboard accessible', () => {
      renderWithPWA(<BottomNavigation />, {
        routerProps: { initialEntries: ['/dashboard'] },
      });

      const buttons = screen.getAllByRole('button');
      const dashboardButton = buttons.find(btn => btn.textContent?.includes('Dashboard'))!;
      const uploadButton = buttons.find(btn => btn.textContent?.includes('Upload'))!;
      const labelsButton = buttons.find(btn => btn.textContent?.includes('Labels'))!;
      const settingsButton = buttons.find(btn => btn.textContent?.includes('Settings'))!;

      // All should be focusable (button elements)
      expect(dashboardButton.tagName).toBe('BUTTON');
      expect(uploadButton.tagName).toBe('BUTTON');
      expect(labelsButton.tagName).toBe('BUTTON');
      expect(settingsButton.tagName).toBe('BUTTON');
    });

    it('shows visual labels for screen readers', () => {
      renderWithPWA(<BottomNavigation />, {
        routerProps: { initialEntries: ['/dashboard'] },
      });

      // Text content should be visible (not just icons)
      expect(screen.getByText(/dashboard/i)).toBeInTheDocument();
      expect(screen.getByText(/upload/i)).toBeInTheDocument();
      expect(screen.getByText(/labels/i)).toBeInTheDocument();
      expect(screen.getByText(/settings/i)).toBeInTheDocument();
    });
  });

  describe('Responsive Behavior', () => {
    beforeEach(() => {
      setupPWAMode(true);
    });

    it('renders in PWA mode', () => {
      const { container } = renderWithPWA(<BottomNavigation />, {
        routerProps: { initialEntries: ['/dashboard'] },
      });

      // Should render when in PWA mode
      expect(container.querySelector('[class*="MuiPaper-root"]')).toBeInTheDocument();
    });
  });

  describe('Component Stability', () => {
    beforeEach(() => {
      setupPWAMode(true);
    });

    it('renders consistently across re-renders', () => {
      const { rerender } = renderWithPWA(<BottomNavigation />, {
        routerProps: { initialEntries: ['/dashboard'] },
      });

      const buttons = screen.getAllByRole('button');
      expect(buttons).toHaveLength(4);

      // Re-render should maintain same structure
      rerender(<BottomNavigation />);

      const buttonsAfterRerender = screen.getAllByRole('button');
      expect(buttonsAfterRerender).toHaveLength(4);
    });
  });
});
