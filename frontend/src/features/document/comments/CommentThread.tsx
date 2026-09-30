import { useId, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { commentsService, type CommentThread as Thread, type CommentWithAuthor } from '../../../services/api';
import { Button, useToast } from '../../../ui';
import { ExpandLess, ExpandMore } from '../../../ui/icons';
import { CommentForm } from './CommentForm';
import { CommentItem } from './CommentItem';
import styles from './Comments.module.css';

export interface CommentThreadProps {
  thread: Thread;
  documentId: string;
  currentUserId: string;
  currentUserRole: string;
  onChanged: () => void;
}

/** A top-level comment with its replies (one level deep, loaded in pages of 50). */
export function CommentThread({ thread, documentId, currentUserId, currentUserRole, onChanged }: CommentThreadProps) {
  const { t } = useTranslation();
  const toast = useToast();
  const [expanded, setExpanded] = useState(false);
  const [replying, setReplying] = useState(false);
  const [extra, setExtra] = useState<CommentWithAuthor[]>([]);
  const [loadingMore, setLoadingMore] = useState(false);
  const repliesId = useId();

  const replies = [...thread.replies, ...extra.filter((r) => !thread.replies.some((x) => x.id === r.id))];
  const hasMore = thread.reply_count > replies.length;

  const edit = async (commentId: string, content: string) => {
    await commentsService.update(documentId, commentId, { content });
    onChanged();
  };
  const remove = async (commentId: string) => {
    try {
      await commentsService.delete(documentId, commentId);
      onChanged();
    } catch {
      toast.show({ title: t('document.comments.deleteFailed', "Couldn't delete the comment"), tone: 'danger' });
    }
  };
  const reply = async (content: string) => {
    await commentsService.create(documentId, { content, parent_id: thread.id });
    setReplying(false);
    setExpanded(true);
    onChanged();
  };
  const loadMore = async () => {
    setLoadingMore(true);
    try {
      const res = await commentsService.getReplies(documentId, thread.id, 50, replies.length);
      setExtra((prev) => [...prev, ...res.data]);
    } catch {
      toast.show({ title: t('document.comments.loadRepliesFailed', "Couldn't load more replies"), tone: 'danger' });
    } finally {
      setLoadingMore(false);
    }
  };

  const shared = { currentUserId, currentUserRole, onEdit: edit, onDelete: remove };

  return (
    <li className={styles.thread}>
      <CommentItem comment={thread} onReply={() => setReplying((v) => !v)} {...shared} />

      {replying ? (
        <div className={styles.nested}>
          <CommentForm
            label={t('document.comments.replyLabel', { name: thread.username, defaultValue: 'Reply to {{name}}' })}
            submitLabel={t('document.comments.reply', 'Reply')}
            autoFocus
            onCancel={() => setReplying(false)}
            onSubmit={reply}
          />
        </div>
      ) : null}

      {thread.reply_count > 0 ? (
        <div className={styles.nested}>
          <Button
            size="sm"
            variant="ghost"
            aria-expanded={expanded}
            aria-controls={repliesId}
            icon={expanded ? <ExpandLess fontSize="inherit" /> : <ExpandMore fontSize="inherit" />}
            onPress={() => setExpanded((v) => !v)}
          >
            {expanded
              ? t('document.comments.hideReplies', { count: thread.reply_count, defaultValue: 'Hide {{count}} replies' })
              : t('document.comments.showReplies', { count: thread.reply_count, defaultValue: 'Show {{count}} replies' })}
          </Button>
          {expanded ? (
            <ul id={repliesId} className={styles.replies}>
              {replies.map((r) => (
                <li key={r.id}>
                  <CommentItem comment={r} {...shared} />
                </li>
              ))}
              {hasMore ? (
                <li>
                  <Button size="sm" variant="ghost" isPending={loadingMore} onPress={loadMore}>
                    {t('document.comments.loadMore', 'Load more replies')}
                  </Button>
                </li>
              ) : null}
            </ul>
          ) : null}
        </div>
      ) : null}
    </li>
  );
}
