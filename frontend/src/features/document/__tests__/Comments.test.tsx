import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiModule from '../../../services/api';
import type { CommentThread } from '../../../services/api';
import { act } from '@testing-library/react';
import { COMMENTS_POLL_MS, CommentsPanel, useComments } from '../comments/CommentsPanel';
import type { ApiMock } from './mockApi';
import { Providers, testUser } from './testUtils';

vi.mock('../../../services/api', async () => (await import('./mockApi')).createApiMock());

const m = apiModule as unknown as ApiMock;

const thread = (overrides: Partial<CommentThread> = {}): CommentThread => ({
  id: 'c-1',
  document_id: 'doc-1',
  user_id: 'user-1',
  parent_id: null,
  content: 'Please check the total.',
  is_edited: false,
  created_at: new Date(Date.now() - 5 * 60_000).toISOString(),
  updated_at: new Date().toISOString(),
  username: 'ada',
  user_role: 'user',
  reply_count: 0,
  replies: [],
  ...overrides,
});

function Panel({ poll = true }: { poll?: boolean }) {
  const comments = useComments('doc-1', { poll });
  return (
    <>
      <output aria-label="comment count">{comments.count}</output>
      <CommentsPanel documentId="doc-1" comments={comments} />
    </>
  );
}

function renderPanel(user: typeof testUser | null = testUser, poll = true) {
  return render(
    <Providers user={user}>
      <Panel poll={poll} />
    </Providers>,
  );
}

const list = () => screen.findByRole('list', { name: 'Comments' });

beforeEach(() => {
  m.commentsService.list.mockResolvedValue({ data: [thread()] });
});

describe('comments', () => {
  it('lists comments with author, relative time and text', async () => {
    renderPanel();
    const comment = await screen.findByRole('article', { name: 'Comment by ada' });
    expect(within(comment).getByText('Please check the total.')).toBeInTheDocument();
    expect(within(comment).getByText('5 min. ago')).toHaveAttribute('dateTime');
    expect(m.commentsService.list).toHaveBeenCalledWith('doc-1');
  });

  it('shows an empty state when there are none', async () => {
    m.commentsService.list.mockResolvedValue({ data: [] });
    renderPanel();
    expect(await screen.findByRole('heading', { name: 'No comments yet' })).toBeInTheDocument();
  });

  it('shows an error when comments cannot load', async () => {
    m.commentsService.list.mockRejectedValue(new Error('down'));
    renderPanel();
    expect(await screen.findByRole('alert')).toHaveTextContent("Couldn't load comments.");
  });

  it('adds a comment, showing it right away', async () => {
    const user = userEvent.setup();
    m.commentsService.list.mockResolvedValueOnce({ data: [] });
    let resolve: (v: unknown) => void = () => {};
    m.commentsService.create.mockReturnValue(new Promise((r) => (resolve = r)));
    renderPanel();
    await screen.findByRole('heading', { name: 'No comments yet' });
    const box = screen.getByRole('textbox', { name: 'New comment' });
    const post = screen.getByRole('button', { name: 'Post' });
    expect(post).toBeDisabled();
    await user.type(box, '  Looks right  ');
    await user.click(post);
    expect(m.commentsService.create).toHaveBeenCalledWith('doc-1', { content: 'Looks right' });
    const optimistic = await screen.findByRole('article', { name: 'Comment by ada' });
    expect(within(optimistic).getByText('Looks right')).toBeInTheDocument();
    m.commentsService.list.mockResolvedValue({ data: [thread({ id: 'c-9', content: 'Looks right' })] });
    resolve({ data: {} });
    await waitFor(() => expect(m.commentsService.list).toHaveBeenCalledTimes(2));
    await waitFor(() => expect(box).toHaveValue(''));
  });

  it('rolls back and explains when posting fails', async () => {
    const user = userEvent.setup();
    m.commentsService.list.mockResolvedValue({ data: [] });
    m.commentsService.create.mockRejectedValue(new Error('nope'));
    renderPanel();
    await screen.findByRole('heading', { name: 'No comments yet' });
    await user.type(screen.getByRole('textbox', { name: 'New comment' }), 'Hello');
    await user.click(screen.getByRole('button', { name: 'Post' }));
    expect(await screen.findByText("Couldn't save the comment. Try again.")).toBeInTheDocument();
    expect(screen.queryByRole('article')).not.toBeInTheDocument();
    expect(screen.getByRole('textbox', { name: 'New comment' })).toHaveValue('Hello');
  });

  it('deletes own comments', async () => {
    const user = userEvent.setup();
    m.commentsService.delete.mockResolvedValue({});
    renderPanel();
    await list();
    await user.click(screen.getByRole('button', { name: 'Delete comment' }));
    expect(m.commentsService.delete).toHaveBeenCalledWith('doc-1', 'c-1');
    await waitFor(() => expect(m.commentsService.list).toHaveBeenCalledTimes(2));
  });

  it("does not offer edit or delete on someone else's comment", async () => {
    m.commentsService.list.mockResolvedValue({ data: [thread({ user_id: 'user-2', username: 'bob' })] });
    renderPanel();
    await screen.findByRole('article', { name: 'Comment by bob' });
    expect(screen.queryByRole('button', { name: 'Edit comment' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Delete comment' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Reply' })).toBeInTheDocument();
  });

  it('lets an admin delete anyone’s comment', async () => {
    m.commentsService.list.mockResolvedValue({ data: [thread({ user_id: 'user-2', username: 'bob' })] });
    renderPanel({ ...testUser, role: 'admin' as const });
    await screen.findByRole('article', { name: 'Comment by bob' });
    expect(screen.getByRole('button', { name: 'Delete comment' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Edit comment' })).not.toBeInTheDocument();
  });

  it('edits a comment in place', async () => {
    const user = userEvent.setup();
    m.commentsService.update.mockResolvedValue({ data: {} });
    renderPanel();
    await list();
    await user.click(screen.getByRole('button', { name: 'Edit comment' }));
    const box = screen.getByRole('textbox', { name: 'Edit comment' });
    expect(box).toHaveValue('Please check the total.');
    await user.clear(box);
    await user.type(box, 'Total is fine');
    await user.click(screen.getByRole('button', { name: 'Save' }));
    expect(m.commentsService.update).toHaveBeenCalledWith('doc-1', 'c-1', { content: 'Total is fine' });
    await waitFor(() => expect(screen.queryByRole('textbox', { name: 'Edit comment' })).not.toBeInTheDocument());
  });

  it('replies to a thread and shows replies on demand', async () => {
    const user = userEvent.setup();
    const reply = { ...thread({ id: 'r-1', parent_id: 'c-1', content: 'Checked.', username: 'bob', user_id: 'user-2' }) };
    m.commentsService.list.mockResolvedValue({ data: [thread({ reply_count: 1, replies: [reply] })] });
    m.commentsService.create.mockResolvedValue({ data: {} });
    renderPanel();
    await list();
    expect(screen.queryByText('Checked.')).not.toBeInTheDocument();
    const toggle = screen.getByRole('button', { name: /^Show 1 repl/ });
    await user.click(toggle);
    expect(screen.getByText('Checked.')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /^Hide 1 repl/ })).toHaveAttribute('aria-expanded', 'true');

    await user.click(screen.getAllByRole('button', { name: 'Reply' })[0]);
    await user.type(screen.getByRole('textbox', { name: 'Reply to ada' }), 'Thanks');
    await user.click(within(screen.getByRole('textbox', { name: 'Reply to ada' }).closest('form') as HTMLElement).getByRole('button', { name: 'Reply' }));
    expect(m.commentsService.create).toHaveBeenCalledWith('doc-1', { content: 'Thanks', parent_id: 'c-1' });
  });

  it('loads more replies than the first page', async () => {
    const user = userEvent.setup();
    const first = thread({ id: 'r-1', parent_id: 'c-1', content: 'First reply' });
    m.commentsService.list.mockResolvedValue({ data: [thread({ reply_count: 2, replies: [first] })] });
    m.commentsService.getReplies.mockResolvedValue({ data: [thread({ id: 'r-2', parent_id: 'c-1', content: 'Second reply' })] });
    renderPanel();
    await list();
    await user.click(screen.getByRole('button', { name: 'Show 2 replies' }));
    await user.click(screen.getByRole('button', { name: 'Load more replies' }));
    expect(m.commentsService.getReplies).toHaveBeenCalledWith('doc-1', 'c-1', 50, 1);
    expect(await screen.findByText('Second reply')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Load more replies' })).not.toBeInTheDocument();
  });

  it('hides the new-comment box when signed out', async () => {
    renderPanel(null);
    await list();
    expect(screen.queryByRole('textbox', { name: 'New comment' })).not.toBeInTheDocument();
  });
});

describe('useComments', () => {
  it('counts threads and their replies', async () => {
    m.commentsService.list.mockResolvedValue({ data: [thread(), thread({ id: 'c-2', reply_count: 2 })] });
    renderPanel();
    await waitFor(() => expect(screen.getByRole('status', { name: 'comment count' })).toHaveTextContent('4'));
  });

  it('loads once and polls only while asked to', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    try {
      renderPanel(testUser, false);
      await waitFor(() => expect(m.commentsService.list).toHaveBeenCalledTimes(1));
      await act(async () => {
        vi.advanceTimersByTime(COMMENTS_POLL_MS * 2);
      });
      expect(m.commentsService.list).toHaveBeenCalledTimes(1);
    } finally {
      vi.useRealTimers();
    }
  });
});
