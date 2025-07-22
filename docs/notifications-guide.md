# 🔔 Notifications Guide

Readur provides comprehensive real-time notifications to keep you informed about document processing, sync events, and system status.

## Notification Types

### Document Processing
- **OCR Completion** - Text extraction finished for uploaded documents
- **Processing Errors** - Failed OCR or document processing alerts
- **Batch Processing** - Status updates for multiple document uploads
- **Quality Warnings** - Low-quality OCR results requiring attention

### Sync Events
- **Source Sync Complete** - WebDAV, S3, or local folder synchronization finished
- **New Documents Found** - Fresh documents discovered during sync
- **Sync Errors** - Connection issues or permission problems
- **Conflict Resolution** - File conflicts requiring user intervention

### System Status
- **Health Alerts** - System performance warnings or failures
- **Maintenance Windows** - Scheduled maintenance notifications
- **Security Events** - Login attempts, permission changes
- **Storage Warnings** - Disk space or quota limitations

## Notification Delivery

### In-App Notifications
- **Real-time Badge** - Notification counter in the top navigation
- **Notification Panel** - Expandable list of recent alerts
- **Toast Messages** - Immediate pop-up notifications for urgent items
- **Dashboard Widgets** - Status cards showing notification summaries

### Email Notifications
- **Immediate Alerts** - Critical system or processing failures
- **Daily Digest** - Summary of processing activity and status
- **Weekly Reports** - System health and usage statistics
- **Custom Triggers** - User-configured alert conditions

## Notification Settings

### User Preferences
Access notification settings through:
1. Click your profile in the top-right corner
2. Select "Notification Settings"
3. Configure your preferences for each notification type

### Notification Categories
- **Critical** - System failures, security alerts (always enabled)
- **Important** - Processing errors, sync failures
- **Informational** - Completion notifications, status updates
- **Promotional** - Feature updates, tips (can be disabled)

### Delivery Preferences
- **In-App Only** - Notifications appear only within Readur
- **Email + In-App** - Notifications sent to both locations
- **Email Only** - Notifications sent only via email
- **Disabled** - No notifications for this category

## Advanced Configuration

### Admin Settings
Administrators can configure system-wide notification policies:

```env
# Email notification settings
SMTP_HOST=smtp.your-domain.com
SMTP_PORT=587
SMTP_USERNAME=notifications@your-domain.com
SMTP_PASSWORD=your-password

# Notification thresholds
CRITICAL_ERROR_THRESHOLD=5
WARNING_BATCH_SIZE=100
DIGEST_FREQUENCY=daily

# Webhook integrations
SLACK_WEBHOOK_URL=https://hooks.slack.com/...
TEAMS_WEBHOOK_URL=https://your-org.webhook.office.com/...
```

### Webhook Integration
Send notifications to external systems:

#### Slack Integration
```json
{
  "channel": "#readur-alerts",
  "username": "Readur Bot",
  "text": "OCR processing completed for 15 documents",
  "attachments": [
    {
      "color": "good",
      "fields": [
        {"title": "Success Rate", "value": "93%", "short": true},
        {"title": "Processing Time", "value": "2m 34s", "short": true}
      ]
    }
  ]
}
```

#### Teams Integration
```json
{
  "@type": "MessageCard",
  "themeColor": "0076D7",
  "summary": "Readur Notification",
  "sections": [{
    "activityTitle": "Document Processing Complete",
    "activitySubtitle": "15 documents processed successfully",
    "facts": [
      {"name": "Success Rate", "value": "93%"},
      {"name": "Processing Time", "value": "2m 34s"}
    ]
  }]
}
```

## Managing Notifications

### Notification History
- **View All** - Complete history of notifications
- **Filter by Type** - Show only specific notification categories
- **Search** - Find notifications by content or date
- **Archive** - Mark notifications as read or hide them

### Bulk Actions
- **Mark All Read** - Clear all unread notification badges
- **Delete Old** - Remove notifications older than specified date
- **Export** - Download notification history as CSV or JSON

## Troubleshooting

### Missing Notifications
- Check notification settings in your profile
- Verify email address is correct and confirmed
- Check spam/junk folder for email notifications
- Ensure browser notifications are enabled

### Too Many Notifications
- Adjust notification thresholds in settings
- Disable informational categories
- Switch to daily digest mode for non-critical items
- Use filters to focus on important notifications

### Email Delivery Issues
- Verify SMTP configuration (admin only)
- Check email server reputation and SPF records
- Test email delivery with notification test feature
- Review email bounce logs in admin panel