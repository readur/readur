import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import LabelsSection from '../labels/LabelsSection';
import { createMockLabel } from '../../../test/label-test-utils';
import { apiMock, httpError, ok, renderSettings } from './settingsTestUtils';

vi.mock('../../../services/api', async (importOriginal) => (await import('./apiMock')).mockApiModule(importOriginal));

const mockLabels = [
  createMockLabel({ id: 'label-1', name: 'Important', description: 'High priority items', color: '#d73a49', icon: 'star', is_system: true, document_count: 10, source_count: 2 }),
  createMockLabel({ id: 'label-2', name: 'Work', description: 'Work-related documents', color: '#0969da', icon: 'work', is_system: true, document_count: 5, source_count: 1 }),
  createMockLabel({ id: 'label-3', name: 'Personal Project', description: 'My personal project files', color: '#28a745', icon: 'folder', is_system: false, document_count: 3, source_count: 0 }),
  createMockLabel({ id: 'label-4', name: 'Archive', description: 'Archived items', color: '#6e7781', icon: 'archive', is_system: true, document_count: 0, source_count: 0 }),
];

const render = () => renderSettings(<LabelsSection />, { path: '/settings/labels' });
const card = (name: string) => screen.getByText(name, { selector: '*' }).closest('li') as HTMLElement;

beforeEach(() => {
  apiMock.get.mockResolvedValue(ok(mockLabels));
  apiMock.post.mockResolvedValue(ok(mockLabels[0], 201));
  apiMock.put.mockResolvedValue(ok(mockLabels[0]));
  apiMock.delete.mockResolvedValue(ok(null, 204));
});

describe('LabelsSection: initial rendering', () => {
  it('renders the create button', async () => {
    render();
    expect(await screen.findByRole('button', { name: 'Create Label' })).toBeInTheDocument();
  });

  it('shows a loading state initially', () => {
    apiMock.get.mockImplementation(() => new Promise(() => {}));
    render();
    expect(screen.getByText('Loading labels...')).toHaveAttribute('role', 'status');
  });

  it('fetches labels with counts on mount', async () => {
    render();
    await waitFor(() => expect(apiMock.get).toHaveBeenCalledWith('/labels?include_counts=true'));
  });

  it('displays labels after loading', async () => {
    render();
    expect(await screen.findByText('Important')).toBeInTheDocument();
    expect(screen.getByText('Work')).toBeInTheDocument();
    expect(screen.getByText('Personal Project')).toBeInTheDocument();
  });
});

describe('LabelsSection: error handling', () => {
  it('shows the error message when the API fails', async () => {
    apiMock.get.mockRejectedValue(new Error('API Error'));
    render();
    expect(await screen.findByText('API Error')).toBeInTheDocument();
  });

  it('lets the user dismiss the error', async () => {
    apiMock.get.mockRejectedValue(new Error('API Error'));
    const user = userEvent.setup();
    render();
    const alert = await screen.findByRole('alert');
    await user.click(within(alert).getByRole('button', { name: 'Close' }));
    expect(screen.queryByText('API Error')).not.toBeInTheDocument();
  });

  it('handles 401 errors', async () => {
    apiMock.get.mockRejectedValue({ response: { status: 401 }, message: 'Unauthorized', isAxiosError: true });
    render();
    expect(await screen.findByText('Unauthorized')).toBeInTheDocument();
  });

  it('handles 403 errors', async () => {
    apiMock.get.mockRejectedValue({ response: { status: 403 }, message: 'Forbidden', isAxiosError: true });
    render();
    expect(await screen.findByText('Forbidden')).toBeInTheDocument();
  });

  it('handles 500 errors with a server message', async () => {
    apiMock.get.mockRejectedValue({ response: { status: 500 }, message: 'Internal Server Error', isAxiosError: true });
    render();
    expect(await screen.findByText('Server error. Please try again later.')).toBeInTheDocument();
  });

  it('handles non-array response data', async () => {
    apiMock.get.mockResolvedValue(ok({ error: 'Something went wrong' }));
    render();
    expect(await screen.findByText('Received invalid data format from server')).toBeInTheDocument();
    expect(screen.getByText('No labels found')).toBeInTheDocument();
  });

  it('handles an unexpected status with a valid array', async () => {
    apiMock.get.mockResolvedValue(ok(mockLabels, 202));
    render();
    expect(await screen.findByText('Server returned unexpected response (202)')).toBeInTheDocument();
  });

  it('keeps the list an array when data is null', async () => {
    apiMock.get.mockResolvedValue(ok(null));
    render();
    expect(await screen.findByText('Received invalid data format from server')).toBeInTheDocument();
    expect(screen.getByText('No labels found')).toBeInTheDocument();
  });

  it('handles string response data', async () => {
    apiMock.get.mockResolvedValue(ok('Server maintenance in progress'));
    render();
    expect(await screen.findByText('Received invalid data format from server')).toBeInTheDocument();
  });
});

describe('LabelsSection: search and filtering', () => {
  it('renders the search input', async () => {
    render();
    expect(await screen.findByPlaceholderText('Search labels...')).toBeInTheDocument();
  });

  it('filters labels by name', async () => {
    const user = userEvent.setup();
    render();
    await screen.findByText('Important');
    await user.type(screen.getByRole('searchbox', { name: 'Search labels...' }), 'work');
    expect(screen.getByText('Work')).toBeInTheDocument();
    expect(screen.queryByText('Important')).not.toBeInTheDocument();
    expect(screen.queryByText('Personal Project')).not.toBeInTheDocument();
  });

  it('filters labels by description', async () => {
    const user = userEvent.setup();
    render();
    await screen.findByText('Important');
    await user.type(screen.getByRole('searchbox', { name: 'Search labels...' }), 'priority');
    expect(screen.getByText('Important')).toBeInTheDocument();
    expect(screen.queryByText('Work')).not.toBeInTheDocument();
  });

  it('toggles system labels', async () => {
    const user = userEvent.setup();
    render();
    await screen.findByText('Important');
    const toggle = screen.getByRole('switch', { name: 'System Labels' });
    expect(toggle).toBeChecked();
    await user.click(toggle);
    expect(screen.queryByText('Important')).not.toBeInTheDocument();
    expect(screen.getByText('Personal Project')).toBeInTheDocument();
  });
});

describe('LabelsSection: grouping', () => {
  it('shows the system labels section', async () => {
    render();
    expect(await screen.findByRole('heading', { name: 'System Labels' })).toBeInTheDocument();
  });

  it('shows the user labels section', async () => {
    render();
    expect(await screen.findByRole('heading', { name: 'My Labels' })).toBeInTheDocument();
  });

  it('puts each label in the right group', async () => {
    render();
    const system = (await screen.findByRole('heading', { name: 'System Labels' })).closest('section') as HTMLElement;
    const mine = screen.getByRole('heading', { name: 'My Labels' }).closest('section') as HTMLElement;
    expect(within(system).getByText('Important')).toBeInTheDocument();
    expect(within(system).getByText('Archive')).toBeInTheDocument();
    expect(within(mine).getByText('Personal Project')).toBeInTheDocument();
    expect(within(mine).queryByText('Work')).not.toBeInTheDocument();
  });
});

describe('LabelsSection: cards', () => {
  it('shows name, description and usage counts', async () => {
    render();
    await screen.findByText('Important');
    const important = card('Important');
    expect(within(important).getByText('High priority items')).toBeInTheDocument();
    expect(within(important).getByText('Documents: 10')).toBeInTheDocument();
    expect(within(important).getByText('Sources: 2')).toBeInTheDocument();
  });

  it('shows edit and delete buttons for user labels', async () => {
    render();
    await screen.findByText('Personal Project');
    expect(screen.getByRole('button', { name: 'Edit label Personal Project' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Delete label Personal Project' })).toBeInTheDocument();
  });

  it('marks system labels and gives them no edit or delete buttons', async () => {
    render();
    await screen.findByText('Important');
    const important = card('Important');
    expect(within(important).getByText('System')).toBeInTheDocument();
    expect(within(important).queryByRole('button', { name: /Edit label|Delete label/ })).not.toBeInTheDocument();
  });
});

describe('LabelsSection: create', () => {
  it('opens the create dialog', async () => {
    const user = userEvent.setup();
    render();
    await user.click(await screen.findByRole('button', { name: 'Create Label' }));
    expect(await screen.findByRole('dialog', { name: 'Create New Label' })).toBeInTheDocument();
  });

  it('posts the new label', async () => {
    const user = userEvent.setup();
    render();
    await user.click(await screen.findByRole('button', { name: 'Create Label' }));
    const dialog = await screen.findByRole('dialog', { name: 'Create New Label' });
    await user.type(within(dialog).getByRole('textbox', { name: /Label Name/ }), 'New Label');
    await user.click(within(dialog).getByRole('button', { name: 'Create' }));
    await waitFor(() => expect(apiMock.post).toHaveBeenCalledWith('/labels', expect.objectContaining({ name: 'New Label' })));
  });

  it('shows a duplicate-name error in the dialog', async () => {
    apiMock.post.mockRejectedValue(httpError(409, { error: 'dup', code: 'LABEL_DUPLICATE_NAME' }));
    const user = userEvent.setup();
    render();
    await user.click(await screen.findByRole('button', { name: 'Create Label' }));
    const dialog = await screen.findByRole('dialog', { name: 'Create New Label' });
    await user.type(within(dialog).getByRole('textbox', { name: /Label Name/ }), 'Work');
    await user.click(within(dialog).getByRole('button', { name: 'Create' }));
    expect(await within(dialog).findByText(/dup|already exists/)).toBeInTheDocument();
  });
});

describe('LabelsSection: edit', () => {
  it('opens the edit dialog', async () => {
    const user = userEvent.setup();
    render();
    await user.click(await screen.findByRole('button', { name: 'Edit label Personal Project' }));
    expect(await screen.findByRole('dialog', { name: 'Edit Label' })).toBeInTheDocument();
  });

  it('puts the updated label', async () => {
    const user = userEvent.setup();
    render();
    await user.click(await screen.findByRole('button', { name: 'Edit label Personal Project' }));
    const dialog = await screen.findByRole('dialog', { name: 'Edit Label' });
    const name = within(dialog).getByRole('textbox', { name: /Label Name/ });
    await user.clear(name);
    await user.type(name, 'Updated Label');
    await user.click(within(dialog).getByRole('button', { name: 'Update' }));
    await waitFor(() =>
      expect(apiMock.put).toHaveBeenCalledWith('/labels/label-3', expect.objectContaining({ name: 'Updated Label' })),
    );
  });
});

describe('LabelsSection: delete', () => {
  it('asks for confirmation', async () => {
    const user = userEvent.setup();
    render();
    await user.click(await screen.findByRole('button', { name: 'Delete label Personal Project' }));
    const dialog = await screen.findByRole('alertdialog', { name: 'Delete Label' });
    expect(within(dialog).getByText(/are you sure you want to delete the label/i)).toBeInTheDocument();
    expect(apiMock.delete).not.toHaveBeenCalled();
  });

  it('warns when the label is in use', async () => {
    const user = userEvent.setup();
    render();
    await user.click(await screen.findByRole('button', { name: 'Delete label Personal Project' }));
    const dialog = await screen.findByRole('alertdialog', { name: 'Delete Label' });
    expect(within(dialog).getByText(/This label is currently used by 3 document\(s\)/)).toBeInTheDocument();
  });

  it('deletes on confirm', async () => {
    const user = userEvent.setup();
    render();
    await user.click(await screen.findByRole('button', { name: 'Delete label Personal Project' }));
    await user.click(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'Delete' }));
    await waitFor(() => expect(apiMock.delete).toHaveBeenCalledWith('/labels/label-3'));
    await waitFor(() => expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument());
  });

  it('cancels without deleting', async () => {
    const user = userEvent.setup();
    render();
    await user.click(await screen.findByRole('button', { name: 'Delete label Personal Project' }));
    await user.click(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'Cancel' }));
    await waitFor(() => expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument());
    expect(apiMock.delete).not.toHaveBeenCalled();
  });

  it('keeps the dialog open and explains when the label is in use', async () => {
    apiMock.delete.mockRejectedValue(httpError(409, { error: 'in use', code: 'LABEL_IN_USE' }));
    const user = userEvent.setup();
    render();
    await user.click(await screen.findByRole('button', { name: 'Delete label Personal Project' }));
    const dialog = await screen.findByRole('alertdialog');
    await user.click(within(dialog).getByRole('button', { name: 'Delete' }));
    expect(await within(dialog).findByText(/currently assigned to documents/)).toBeInTheDocument();
  });
});

describe('LabelsSection: empty states', () => {
  it('shows the empty state when there are no labels', async () => {
    apiMock.get.mockResolvedValue(ok([]));
    render();
    expect(await screen.findByText('No labels found')).toBeInTheDocument();
    expect(screen.getByText("You haven't created any labels yet")).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Create Your First Label' })).toBeInTheDocument();
  });

  it('shows the no-match state for a search', async () => {
    const user = userEvent.setup();
    render();
    await screen.findByText('Important');
    await user.type(screen.getByRole('searchbox', { name: 'Search labels...' }), 'nonexistent');
    expect(screen.getByText('No labels found')).toBeInTheDocument();
    expect(screen.getByText('No labels match "nonexistent"')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Create Your First Label' })).not.toBeInTheDocument();
  });

  it('opens the create dialog from the empty state', async () => {
    apiMock.get.mockResolvedValue(ok([]));
    const user = userEvent.setup();
    render();
    await user.click(await screen.findByRole('button', { name: 'Create Your First Label' }));
    expect(await screen.findByRole('dialog', { name: 'Create New Label' })).toBeInTheDocument();
  });
});

describe('LabelsSection: refresh', () => {
  it('refetches after creating', async () => {
    const user = userEvent.setup();
    render();
    await waitFor(() => expect(apiMock.get).toHaveBeenCalledTimes(1));
    await user.click(await screen.findByRole('button', { name: 'Create Label' }));
    const dialog = await screen.findByRole('dialog', { name: 'Create New Label' });
    await user.type(within(dialog).getByRole('textbox', { name: /Label Name/ }), 'New Label');
    await user.click(within(dialog).getByRole('button', { name: 'Create' }));
    await waitFor(() => expect(apiMock.get).toHaveBeenCalledTimes(2));
  });

  it('refetches after deleting', async () => {
    const user = userEvent.setup();
    render();
    await user.click(await screen.findByRole('button', { name: 'Delete label Personal Project' }));
    await user.click(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'Delete' }));
    await waitFor(() => expect(apiMock.get).toHaveBeenCalledTimes(2));
  });
});

describe('LabelsSection: operation errors', () => {
  it('shows the error when deleting fails', async () => {
    apiMock.delete.mockRejectedValue(new Error('Delete failed'));
    const user = userEvent.setup();
    render();
    await user.click(await screen.findByRole('button', { name: 'Delete label Personal Project' }));
    await user.click(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'Delete' }));
    expect(await screen.findByText('Delete failed')).toBeInTheDocument();
  });

  it('closes the dialog and refreshes when the label was already deleted', async () => {
    apiMock.delete.mockRejectedValue(httpError(404, { error: 'gone', code: 'LABEL_NOT_FOUND' }));
    const user = userEvent.setup();
    render();
    await user.click(await screen.findByRole('button', { name: 'Delete label Personal Project' }));
    await user.click(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'Delete' }));
    await waitFor(() => expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument());
    expect(await screen.findByText('Label not found. It may have already been deleted.')).toBeInTheDocument();
    expect(apiMock.get).toHaveBeenCalledTimes(2);
  });
});
