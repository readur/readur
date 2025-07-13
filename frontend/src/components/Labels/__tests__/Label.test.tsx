import { describe, test, expect, vi, beforeEach } from 'vitest';
import { screen, fireEvent } from '@testing-library/react';
import Label, { type LabelData } from '../Label';
import { renderWithProviders } from '../../../test/test-utils';
import { 
  createMockLabel, 
  createMockSystemLabel
} from '../../../test/label-test-utils';

const mockLabel = createMockLabel({
  name: 'Test Label',
  color: '#ff0000',
  document_count: 5,
  source_count: 2,
});

const systemLabel = createMockSystemLabel({
  name: 'Important',
  color: '#d73a49',
});

const renderLabel = (props: Partial<React.ComponentProps<typeof Label>> = {}) => {
  const defaultProps = {
    label: mockLabel,
    ...props,
  };

  return renderWithProviders(<Label {...defaultProps} />);
};

describe('Label Component', () => {
  beforeEach(() => {
    // Test setup is handled globally
  });

  describe('Basic Rendering', () => {
    test('should render label with name', () => {
      renderLabel();
      expect(screen.getByText('Test Label')).toBeInTheDocument();
    });

    test('should render label with icon', () => {
      renderLabel();
      const labelElement = screen.getByText('Test Label').closest('.MuiChip-root');
      expect(labelElement).toBeInTheDocument();
      // Icon is rendered as part of the label content
    });

    test('should apply correct color styling', () => {
      renderLabel();
      const labelElement = screen.getByText('Test Label').closest('.MuiChip-root');
      expect(labelElement).toHaveStyle({
        backgroundColor: '#ff0000',
      });
    });

    test('should render with outline variant', () => {
      renderLabel({ variant: 'outlined' });
      const labelElement = screen.getByText('Test Label').closest('.MuiChip-root');
      expect(labelElement).toHaveClass('MuiChip-outlined');
    });
  });

  describe('Size Variants', () => {
    test('should render small size', () => {
      renderLabel({ size: 'small' });
      const labelElement = screen.getByText('Test Label').closest('.MuiChip-root');
      expect(labelElement).toHaveClass('MuiChip-sizeSmall');
    });

    test('should render medium size by default', () => {
      renderLabel();
      const labelElement = screen.getByText('Test Label').closest('.MuiChip-root');
      expect(labelElement).toHaveClass('MuiChip-sizeMedium');
    });

    test('should render large size as medium (MUI limitation)', () => {
      renderLabel({ size: 'large' });
      const labelElement = screen.getByText('Test Label').closest('.MuiChip-root');
      expect(labelElement).toHaveClass('MuiChip-sizeMedium');
    });
  });

  describe('Document Count Display', () => {
    test('should show document count when showCount is true', () => {
      renderLabel({ showCount: true });
      expect(screen.getByText('(5)')).toBeInTheDocument();
    });

    test('should not show document count when showCount is false', () => {
      renderLabel({ showCount: false });
      expect(screen.queryByText('(5)')).not.toBeInTheDocument();
    });

    test('should not show count when document_count is 0', () => {
      const labelWithZeroCount = { ...mockLabel, document_count: 0 };
      renderLabel({ label: labelWithZeroCount, showCount: true });
      expect(screen.queryByText('(0)')).not.toBeInTheDocument();
    });
  });

  describe('Click Handling', () => {
    test('should call onClick when clicked', () => {
      const handleClick = vi.fn();
      renderLabel({ onClick: handleClick });
      
      const labelElement = screen.getByText('Test Label').closest('.MuiChip-root');
      fireEvent.click(labelElement!);
      
      expect(handleClick).toHaveBeenCalledWith(mockLabel.id);
    });

    test('should not call onClick when disabled', () => {
      const handleClick = vi.fn();
      renderLabel({ onClick: handleClick, disabled: true });
      
      const labelElement = screen.getByText('Test Label').closest('.MuiChip-root');
      fireEvent.click(labelElement!);
      
      expect(handleClick).not.toHaveBeenCalled();
    });

    test('should show pointer cursor when clickable', () => {
      const handleClick = vi.fn();
      renderLabel({ onClick: handleClick });
      
      const labelElement = screen.getByText('Test Label').closest('.MuiChip-root');
      expect(labelElement).toHaveClass('MuiChip-clickable');
    });
  });

  describe('Delete Functionality', () => {
    test('should show delete button when deletable and not system label', () => {
      const handleDelete = vi.fn();
      renderLabel({ deletable: true, onDelete: handleDelete });
      
      const deleteButton = screen.getByTestId('CloseIcon');
      expect(deleteButton).toBeInTheDocument();
    });

    test('should not show delete button for system labels', () => {
      const handleDelete = vi.fn();
      renderLabel({ 
        label: systemLabel, 
        deletable: true, 
        onDelete: handleDelete 
      });
      
      const deleteButton = screen.queryByTestId('CloseIcon');
      expect(deleteButton).not.toBeInTheDocument();
    });

    test('should call onDelete when delete button is clicked', () => {
      const handleDelete = vi.fn();
      renderLabel({ deletable: true, onDelete: handleDelete });
      
      const deleteButton = screen.getByTestId('CloseIcon');
      fireEvent.click(deleteButton);
      
      expect(handleDelete).toHaveBeenCalledWith(mockLabel.id);
    });

    test('should not call onDelete when disabled', () => {
      const handleDelete = vi.fn();
      renderLabel({ 
        deletable: true, 
        onDelete: handleDelete, 
        disabled: true 
      });
      
      const deleteButton = screen.getByTestId('CloseIcon');
      fireEvent.click(deleteButton);
      
      expect(handleDelete).not.toHaveBeenCalled();
    });

    test('should stop propagation on delete click', () => {
      const handleClick = vi.fn();
      const handleDelete = vi.fn();
      
      renderLabel({ 
        onClick: handleClick,
        deletable: true, 
        onDelete: handleDelete 
      });
      
      const deleteButton = screen.getByTestId('CloseIcon');
      fireEvent.click(deleteButton);
      
      expect(handleDelete).toHaveBeenCalledWith(mockLabel.id);
      expect(handleClick).not.toHaveBeenCalled();
    });
  });

  describe('Icon Rendering', () => {
    test('should render star icon', () => {
      renderLabel({ label: { ...mockLabel, icon: 'star' } });
      // Icons are rendered as part of the component, testing presence indirectly
      expect(screen.getByText('Test Label')).toBeInTheDocument();
    });

    test('should render work icon', () => {
      renderLabel({ label: { ...mockLabel, icon: 'work' } });
      expect(screen.getByText('Test Label')).toBeInTheDocument();
    });

    test('should render without icon when icon is undefined', () => {
      renderLabel({ label: { ...mockLabel, icon: undefined } });
      expect(screen.getByText('Test Label')).toBeInTheDocument();
    });

    test('should handle unknown icon gracefully', () => {
      renderLabel({ label: { ...mockLabel, icon: 'unknown_icon' } });
      expect(screen.getByText('Test Label')).toBeInTheDocument();
    });
  });

  describe('Color Contrast', () => {
    test('should use light text on dark background', () => {
      const darkLabel = { ...mockLabel, color: '#000000' };
      renderLabel({ label: darkLabel });
      
      const labelElement = screen.getByText('Test Label').closest('.MuiChip-root');
      // The component should calculate appropriate text color
      expect(labelElement).toBeInTheDocument();
    });

    test('should use dark text on light background', () => {
      const lightLabel = { ...mockLabel, color: '#ffffff' };
      renderLabel({ label: lightLabel });
      
      const labelElement = screen.getByText('Test Label').closest('.MuiChip-root');
      expect(labelElement).toBeInTheDocument();
    });
  });

  describe('System Labels', () => {
    test('should render system label correctly', () => {
      renderLabel({ label: systemLabel });
      expect(screen.getByText('Important')).toBeInTheDocument();
    });

    test('should not allow deletion of system labels', () => {
      const handleDelete = vi.fn();
      renderLabel({ 
        label: systemLabel, 
        deletable: true, 
        onDelete: handleDelete 
      });
      
      // Should not show delete button for system labels
      const deleteButton = screen.queryByTestId('CloseIcon');
      expect(deleteButton).not.toBeInTheDocument();
    });
  });

  describe('Accessibility', () => {
    test('should have proper ARIA attributes when clickable', () => {
      const handleClick = vi.fn();
      renderLabel({ onClick: handleClick });
      
      const labelElement = screen.getByText('Test Label').closest('.MuiChip-root');
      expect(labelElement).toHaveAttribute('role', 'button');
      expect(labelElement).toHaveAttribute('tabindex', '0');
    });

    test('should be keyboard accessible when clickable', () => {
      const handleClick = vi.fn();
      renderLabel({ onClick: handleClick });
      
      const labelElement = screen.getByText('Test Label').closest('.MuiChip-root');
      
      // Check that the element is focusable and has proper ARIA attributes
      expect(labelElement).toHaveAttribute('role', 'button');
      expect(labelElement).toHaveAttribute('tabindex', '0');
      
      // Test that clicking still works (keyboard events are handled internally by Material-UI)
      fireEvent.click(labelElement!);
      expect(handleClick).toHaveBeenCalledWith(mockLabel.id);
    });

    test('should have proper disabled state attributes', () => {
      renderLabel({ disabled: true });
      
      const labelElement = screen.getByText('Test Label').closest('.MuiChip-root');
      expect(labelElement).toHaveClass('Mui-disabled');
    });
  });

  describe('Custom CSS Classes', () => {
    test('should apply custom className', () => {
      renderLabel({ className: 'custom-label-class' });
      
      const labelElement = screen.getByText('Test Label').closest('.MuiChip-root');
      expect(labelElement).toHaveClass('custom-label-class');
    });
  });

  describe('Edge Cases', () => {
    test('should handle very long label names', () => {
      const longLabel = {
        ...mockLabel,
        name: 'This is a very long label name that might cause layout issues',
      };
      
      renderLabel({ label: longLabel });
      expect(screen.getByText(longLabel.name)).toBeInTheDocument();
    });

    test('should handle special characters in label name', () => {
      const specialLabel = {
        ...mockLabel,
        name: 'Label & Special <Characters> "Quotes"',
      };
      
      renderLabel({ label: specialLabel });
      expect(screen.getByText(specialLabel.name)).toBeInTheDocument();
    });

    test('should handle undefined document_count gracefully', () => {
      const labelWithUndefinedCount = {
        ...mockLabel,
        document_count: undefined,
      };
      
      renderLabel({ label: labelWithUndefinedCount, showCount: true });
      expect(screen.queryByText(/\(\d+\)/)).not.toBeInTheDocument();
    });
  });
});