# 🔌 Swagger UI Guide

Readur includes built-in Swagger UI for interactive API documentation and testing. Access it easily through your user profile menu.

## Accessing Swagger UI

1. **Login to Readur** - Authenticate with your user credentials
2. **Click Your Profile** - Click on your profile avatar in the top-right corner
3. **Select "API Documentation"** - Choose the Swagger UI option from the dropdown menu
4. **Interactive Documentation** - Explore and test all available API endpoints

## API Documentation Features

### Endpoint Explorer

- **Complete API Reference**  
  All REST endpoints with detailed descriptions
  
- **Request/Response Examples**  
  Sample data for every endpoint
  
- **Parameter Details**  
  Required and optional parameters with types
  
- **Authentication Info**  
  JWT token requirements and usage

### Interactive Testing

- **Try It Out**  
  Execute API calls directly from the documentation
  
- **Real Data**  
  Test with your actual Readur data and configuration
  
- **Response Validation**  
  See actual responses and status codes
  
- **Error Handling**  
  View error responses and troubleshooting info

## API Categories

### Authentication

- **Login/Logout**  
  User authentication endpoints
  
- **Token Management**  
  JWT token refresh and validation
  
- **User Registration**  
  New user account creation
  
- **Password Reset**  
  Password recovery workflows

### Document Management

- **Upload Documents**  
  Single and batch file upload endpoints
  
- **Document Retrieval**  
  Get document metadata and content
  
- **Document Search**  
  Full-text search with various modes
  
- **Document Operations**  
  Update, delete, and organize documents

### User Management

- **User CRUD**  
  Create, read, update, delete user accounts
  
- **Role Management**  
  Assign and modify user roles
  
- **Permission Control**  
  Manage access rights and restrictions
  
- **User Preferences**  
  Personal settings and configurations

### Source Management

- **Source Configuration**  
  WebDAV, S3, and local folder setup
  
- **Sync Operations**  
  Manual and automated synchronization
  
- **Source Health**  
  Status monitoring and health checks
  
- **Source Statistics**  
  Usage metrics and performance data

### System Administration

- **Health Monitoring**  
  System status and performance metrics
  
- **Analytics Data**  
  Usage statistics and reporting endpoints
  
- **Configuration**  
  System settings and environment variables
  
- **Maintenance**  
  Backup, cleanup, and administrative tasks

## Authentication in Swagger UI

### Using JWT Tokens
1. **Login via API** - Use `/api/auth/login` endpoint to get a JWT token
2. **Copy Token** - Copy the returned JWT token
3. **Authorize** - Click the "Authorize" button in Swagger UI
4. **Enter Token** - Paste your JWT token in the format: `Bearer your_token_here`
5. **Test Endpoints** - All authenticated endpoints now work with your credentials

### Token Management

- **Token Expiry**  
  Tokens expire after a configured time period
  
- **Refresh Tokens**  
  Use refresh token endpoint to get new access tokens
  
- **Logout**  
  Invalidate tokens using the logout endpoint
  
- **Multiple Sessions**  
  Each browser session needs its own token

## Best Practices

### Development Usage

- **Test First**  
  Use Swagger UI to test API endpoints before implementing
  
- **Validate Responses**  
  Check response formats match your expectations
  
- **Error Scenarios**  
  Test error conditions and edge cases
  
- **Performance Testing**  
  Monitor response times for optimization

### Production Considerations

- **Access Control**  
  Swagger UI respects the same authentication as the main app
  
- **Rate Limiting**  
  API rate limits apply to Swagger UI requests
  
- **Logging**  
  All API calls from Swagger UI are logged normally
  
- **Security**  
  Use HTTPS in production for secure token transmission

## Common Use Cases

### Frontend Development

- **API Integration**  
  Test endpoints before implementing in your frontend
  
- **Data Formats**  
  Understand expected request/response formats
  
- **Error Handling**  
  Learn about error codes and messages
  
- **Feature Testing**  
  Validate new features work as expected

### System Integration

- **Third-party Tools**  
  Test integration with external systems
  
- **Automation Scripts**  
  Develop scripts using API documentation
  
- **Monitoring Systems**  
  Integrate health check endpoints
  
- **Data Migration**  
  Use bulk operations for data import/export

### Troubleshooting

- **Debug Issues**  
  Test API calls to isolate problems
  
- **Validate Permissions**  
  Check if user roles have correct access
  
- **Network Testing**  
  Verify connectivity and response times
  
- **Data Verification**  
  Confirm data integrity and processing status

## Advanced Features

### Custom Headers

- **Request Customization**  
  Add custom headers to API requests
  
- **Content-Type**  
  Specify different content types for uploads
  
- **User-Agent**  
  Set custom user agent strings
  
- **Cache Control**  
  Control caching behavior for responses

### Bulk Operations

- **Batch Uploads**  
  Test multiple file uploads simultaneously
  
- **Bulk Updates**  
  Update multiple documents or users at once
  
- **Mass Operations**  
  Perform administrative tasks in bulk
  
- **Data Export**  
  Export large datasets via API

## Configuration Options

Administrators can configure Swagger UI access:

```env
# Enable/disable Swagger UI
SWAGGER_UI_ENABLED=true

# Customize Swagger UI path
SWAGGER_UI_PATH=/docs

# Authentication requirements
SWAGGER_REQUIRE_AUTH=true

# Rate limiting for API documentation
SWAGGER_RATE_LIMIT=1000
```

## Security Considerations

- **Authentication Required**  
  Swagger UI requires the same login as the main application
  
- **Role-Based Access**  
  API endpoints respect user role permissions
  
- **Audit Logging**  
  All API calls are logged for security monitoring
  
- **Token Security**  
  JWT tokens should be kept secure and not shared