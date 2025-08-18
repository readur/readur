import React, { createContext, useContext, useState, useCallback, useEffect, useRef } from 'react';
import { Notification, NotificationType, NotificationBatch } from '../types/notification';
import { v4 as uuidv4 } from 'uuid';

interface NotificationContextType {
  notifications: Notification[];
  unreadCount: number;
  addNotification: (notification: Omit<Notification, 'id' | 'timestamp' | 'read'>) => void;
  markAsRead: (id: string) => void;
  markAllAsRead: () => void;
  clearNotification: (id: string) => void;
  clearAll: () => void;
  addBatchNotification: (
    type: NotificationType,
    operation: 'upload' | 'ocr' | 'watch',
    files: Array<{ name: string; success: boolean }>
  ) => void;
}

const NotificationContext = createContext<NotificationContextType | undefined>(undefined);

const BATCH_WINDOW_MS = 2000; // 2 seconds to batch notifications
const MAX_NOTIFICATIONS = 50;

export const NotificationProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [notifications, setNotifications] = useState<Notification[]>([]);
  const batchesRef = useRef<Map<string, NotificationBatch>>(new Map());
  const batchTimersRef = useRef<Map<string, NodeJS.Timeout>>(new Map());

  const unreadCount = notifications.filter(n => !n.read).length;

  const addNotification = useCallback((notification: Omit<Notification, 'id' | 'timestamp' | 'read'>) => {
    const newNotification: Notification = {
      ...notification,
      id: uuidv4(),
      timestamp: new Date(),
      read: false,
    };

    setNotifications(prev => {
      const updated = [newNotification, ...prev];
      // Keep only the most recent notifications
      return updated.slice(0, MAX_NOTIFICATIONS);
    });
  }, []);

  const addBatchNotification = useCallback((
    type: NotificationType,
    operation: 'upload' | 'ocr' | 'watch',
    files: Array<{ name: string; success: boolean }>
  ) => {
    const batchKey = `${operation}-${type}`;
    const existingBatch = batchesRef.current.get(batchKey);

    if (existingBatch) {
      // Update existing batch
      existingBatch.count += files.length;
      existingBatch.successCount += files.filter(f => f.success).length;
      existingBatch.failureCount += files.filter(f => !f.success).length;
    } else {
      // Create new batch
      const batch: NotificationBatch = {
        batchId: uuidv4(),
        type,
        operation,
        count: files.length,
        successCount: files.filter(f => f.success).length,
        failureCount: files.filter(f => !f.success).length,
        startTime: new Date(),
      };
      batchesRef.current.set(batchKey, batch);
    }

    // Clear existing timer
    const existingTimer = batchTimersRef.current.get(batchKey);
    if (existingTimer) {
      clearTimeout(existingTimer);
    }

    // Set new timer to finalize batch
    const timer = setTimeout(() => {
      const batch = batchesRef.current.get(batchKey);
      if (batch) {
        batch.endTime = new Date();
        
        // Create notification based on batch
        let title = '';
        let message = '';

        if (batch.count === 1) {
          // Single file - show specific notification
          const fileName = files[0]?.name || 'file';
          if (operation === 'upload') {
            title = batch.successCount > 0 ? 'File Uploaded' : 'Upload Failed';
            message = batch.successCount > 0 
              ? `${fileName} uploaded successfully`
              : `Failed to upload ${fileName}`;
          } else if (operation === 'ocr') {
            title = batch.successCount > 0 ? 'OCR Complete' : 'OCR Failed';
            message = batch.successCount > 0
              ? `Text extracted from ${fileName}`
              : `Failed to extract text from ${fileName}`;
          } else if (operation === 'watch') {
            title = 'File Detected';
            message = `${fileName} added from watch folder`;
          }
        } else {
          // Multiple files - show batch notification
          if (operation === 'upload') {
            title = 'Batch Upload Complete';
            if (batch.failureCount === 0) {
              message = `${batch.successCount} files uploaded successfully`;
            } else if (batch.successCount === 0) {
              message = `Failed to upload ${batch.failureCount} files`;
            } else {
              message = `${batch.successCount} files uploaded, ${batch.failureCount} failed`;
            }
          } else if (operation === 'ocr') {
            title = 'Batch OCR Complete';
            if (batch.failureCount === 0) {
              message = `Text extracted from ${batch.successCount} documents`;
            } else if (batch.successCount === 0) {
              message = `Failed to process ${batch.failureCount} documents`;
            } else {
              message = `${batch.successCount} documents processed, ${batch.failureCount} failed`;
            }
          } else if (operation === 'watch') {
            title = 'Files Detected';
            message = `${batch.count} files added from watch folder`;
          }
        }

        addNotification({
          type: batch.failureCount > 0 && batch.successCount === 0 ? 'error' : 
                batch.failureCount > 0 ? 'warning' : 'success',
          title,
          message,
          metadata: {
            batchId: batch.batchId,
            fileCount: batch.count,
          },
        });

        // Clean up
        batchesRef.current.delete(batchKey);
        batchTimersRef.current.delete(batchKey);
      }
    }, BATCH_WINDOW_MS);

    batchTimersRef.current.set(batchKey, timer);
  }, [addNotification]);

  const markAsRead = useCallback((id: string) => {
    setNotifications(prev =>
      prev.map(n => n.id === id ? { ...n, read: true } : n)
    );
  }, []);

  const markAllAsRead = useCallback(() => {
    setNotifications(prev => prev.map(n => ({ ...n, read: true })));
  }, []);

  const clearNotification = useCallback((id: string) => {
    setNotifications(prev => prev.filter(n => n.id !== id));
  }, []);

  const clearAll = useCallback(() => {
    setNotifications([]);
  }, []);

  // Cleanup timers on unmount
  useEffect(() => {
    return () => {
      batchTimersRef.current.forEach(timer => clearTimeout(timer));
    };
  }, []);

  return (
    <NotificationContext.Provider
      value={{
        notifications,
        unreadCount,
        addNotification,
        markAsRead,
        markAllAsRead,
        clearNotification,
        clearAll,
        addBatchNotification,
      }}
    >
      {children}
    </NotificationContext.Provider>
  );
};

export const useNotifications = () => {
  const context = useContext(NotificationContext);
  if (!context) {
    throw new Error('useNotifications must be used within NotificationProvider');
  }
  return context;
};

// Simplified hook for basic notification usage
export const useNotification = () => {
  const { addNotification } = useNotifications();
  
  const showNotification = useCallback((notification: { 
    type: NotificationType; 
    message: string; 
    title?: string;
  }) => {
    addNotification({
      type: notification.type,
      title: notification.title || '',
      message: notification.message,
    });
  }, [addNotification]);

  return { showNotification };
};