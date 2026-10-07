import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useAuth } from '../../../contexts/AuthContext';
import { commentsService, type CommentThread as Thread } from '../../../services/api';
import { EmptyState, Skeleton } from '../../../ui';
import { CommentForm } from './CommentForm';
import { CommentThread } from './CommentThread';
import styles from './Comments.module.css';

/** Comments refresh in the background so other people's replies show up. */
export const COMMENTS_POLL_MS = 30_000;

export interface CommentsState {
  threads: Thread[];
  loading: boolean;
  error: boolean;
  /** Threads plus their replies. */
  count: number;
  reload: () => Promise<void>;
  /** Posts a comment, showing it right away; throws (after rolling back) when posting fails. */
  create: (content: string) => Promise<void>;
}

/**
 * A document's comments. Loads once; with `poll`, refreshes in the background so other people's
 * replies show up (only while someone is looking at them).
 */
export function useComments(documentId: string, { poll }: { poll: boolean }): CommentsState {
  const { user } = useAuth();
  const [threads, setThreads] = useState<Thread[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(false);

  const reload = useCallback(async () => {
    try {
      const res = await commentsService.list(documentId);
      setThreads(Array.isArray(res.data) ? res.data : []);
      setError(false);
    } catch {
      setError(true);
    } finally {
      setLoading(false);
    }
  }, [documentId]);

  useEffect(() => {
    setLoading(true);
    void reload();
  }, [reload]);

  useEffect(() => {
    if (!poll) return undefined;
    const timer = setInterval(() => void reload(), COMMENTS_POLL_MS);
    return () => clearInterval(timer);
  }, [poll, reload]);

  const create = async (content: string) => {
    if (!user) return;
    const now = new Date().toISOString();
    const optimistic: Thread = {
      id: `pending-${Date.now()}`,
      document_id: documentId,
      user_id: user.id,
      parent_id: null,
      content,
      is_edited: false,
      created_at: now,
      updated_at: now,
      username: user.username,
      user_role: user.role,
      reply_count: 0,
      replies: [],
    };
    setThreads((prev) => [optimistic, ...prev]);
    try {
      await commentsService.create(documentId, { content });
    } catch (err) {
      setThreads((prev) => prev.filter((c) => c.id !== optimistic.id));
      throw err;
    }
    await reload();
  };

  const count = threads.reduce((sum, thread) => sum + 1 + (thread.reply_count ?? 0), 0);
  return { threads, loading, error, count, reload, create };
}

/** Comment list for one document with a new-comment box on top; `comments` comes from useComments. */
export function CommentsPanel({ documentId, comments }: { documentId: string; comments: CommentsState }) {
  const { t } = useTranslation();
  const { user } = useAuth();
  const { threads, loading, error, reload: fetchComments, create } = comments;

  return (
    <div className={styles.panel}>
      {user ? (
        <CommentForm
          label={t('document.comments.newLabel', 'New comment')}
          placeholder={t('document.comments.placeholder', 'Write a comment…')}
          onSubmit={create}
        />
      ) : null}

      {error ? (
        <p className={styles.error} role="alert">
          {t('document.comments.loadFailed', "Couldn't load comments.")}
        </p>
      ) : null}

      {loading ? (
        <Skeleton lines={3} label={t('document.comments.loading', 'Loading comments')} />
      ) : threads.length === 0 && !error ? (
        <EmptyState
          headingAs="h3"
          title={t('document.comments.empty', 'No comments yet')}
          description={t('document.comments.emptyHint', 'Start the conversation about this document.')}
        />
      ) : (
        <ul className={styles.threads} aria-label={t('document.comments.list', 'Comments')}>
          {threads.map((thread) => (
            <CommentThread
              key={thread.id}
              thread={thread}
              documentId={documentId}
              currentUserId={user?.id ?? ''}
              currentUserRole={user?.role ?? 'user'}
              onChanged={fetchComments}
            />
          ))}
        </ul>
      )}
    </div>
  );
}
