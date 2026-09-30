import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { CommentWithAuthor } from '../../../services/api';
import { isAdminRole } from '../../../auth/roles';
import type { UserRole } from '../../../types/generated';
import { IconButton } from '../../../ui';
import { Delete, Edit, Reply } from '../../../ui/icons';
import { CommentForm } from './CommentForm';
import { relativeTime } from './relativeTime';
import styles from './Comments.module.css';

export interface CommentItemProps {
  comment: CommentWithAuthor;
  currentUserId: string;
  currentUserRole: UserRole;
  onReply?: () => void;
  onEdit: (commentId: string, content: string) => Promise<void>;
  onDelete: (commentId: string) => Promise<void>;
}

/** One comment: author, time, text, and reply/edit/delete for whoever may do so. */
export function CommentItem({ comment, currentUserId, currentUserRole, onReply, onEdit, onDelete }: CommentItemProps) {
  const { t, i18n } = useTranslation();
  const [editing, setEditing] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const isAuthor = currentUserId === comment.user_id;
  const canDelete = isAuthor || isAdminRole(currentUserRole);

  const remove = async () => {
    setDeleting(true);
    try {
      await onDelete(comment.id);
    } finally {
      setDeleting(false);
    }
  };

  return (
    <article className={styles.item} aria-label={t('document.comments.by', { name: comment.username, defaultValue: 'Comment by {{name}}' })}>
      <header className={styles.meta}>
        <span className={styles.author}>{comment.username}</span>
        <time className={styles.time} dateTime={comment.created_at}>
          {relativeTime(comment.created_at, i18n.language)}
        </time>
        {comment.is_edited ? <span className={styles.edited}>{t('document.comments.edited', 'edited')}</span> : null}
      </header>

      {editing ? (
        <CommentForm
          label={t('document.comments.editLabel', 'Edit comment')}
          initialValue={comment.content}
          submitLabel={t('document.comments.save', 'Save')}
          autoFocus
          onCancel={() => setEditing(false)}
          onSubmit={async (content) => {
            await onEdit(comment.id, content);
            setEditing(false);
          }}
        />
      ) : (
        <p className={styles.content}>{comment.content}</p>
      )}

      {!editing ? (
        <div className={styles.actions}>
          {onReply ? (
            <IconButton size="sm" label={t('document.comments.reply', 'Reply')} icon={<Reply fontSize="inherit" />} onPress={onReply} />
          ) : null}
          {isAuthor ? (
            <IconButton
              size="sm"
              label={t('document.comments.edit', 'Edit comment')}
              icon={<Edit fontSize="inherit" />}
              onPress={() => setEditing(true)}
            />
          ) : null}
          {canDelete ? (
            <IconButton
              size="sm"
              label={t('document.comments.delete', 'Delete comment')}
              icon={<Delete fontSize="inherit" />}
              isPending={deleting}
              onPress={remove}
            />
          ) : null}
        </div>
      ) : null}
    </article>
  );
}
