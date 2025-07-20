import { describe, test, expect, vi } from 'vitest';
import { 
  ErrorHelper, 
  ErrorCodes, 
  type ApiErrorResponse, 
  type AxiosErrorWithCode,
  type ErrorCode 
} from '../errors';

describe('ErrorHelper', () => {
  describe('getErrorInfo', () => {
    test('should handle structured API error response', () => {
      const structuredError = {
        response: {
          data: {
            error: 'User not found',
            code: 'USER_NOT_FOUND',
            status: 404
          },
          status: 404
        }
      };

      const result = ErrorHelper.getErrorInfo(structuredError);

      expect(result).toEqual({
        message: 'User not found',
        code: 'USER_NOT_FOUND',
        status: 404
      });
    });

    test('should handle legacy error format with message', () => {
      const legacyError = {
        response: {
          data: {
            message: 'Legacy error message'
          },
          status: 500
        }
      };

      const result = ErrorHelper.getErrorInfo(legacyError);

      expect(result).toEqual({
        message: 'Legacy error message',
        status: 500
      });
    });

    test('should handle axios errors without structured data', () => {
      const axiosError = {
        message: 'Network Error',
        response: {
          status: 500
        }
      };

      const result = ErrorHelper.getErrorInfo(axiosError);

      expect(result).toEqual({
        message: 'Network Error',
        status: 500
      });
    });

    test('should handle Error objects', () => {
      const error = new Error('Something went wrong');

      const result = ErrorHelper.getErrorInfo(error);

      expect(result).toEqual({
        message: 'Something went wrong'
      });
    });

    test('should handle string errors', () => {
      const result = ErrorHelper.getErrorInfo('Simple error string');

      expect(result).toEqual({
        message: 'An unknown error occurred'
      });
    });

    test('should handle null/undefined errors', () => {
      expect(ErrorHelper.getErrorInfo(null)).toEqual({
        message: 'An unknown error occurred'
      });

      expect(ErrorHelper.getErrorInfo(undefined)).toEqual({
        message: 'An unknown error occurred'
      });
    });

    test('should handle empty response data', () => {
      const emptyError = {
        response: {
          data: {},
          status: 400
        },
        message: 'Bad Request'
      };

      const result = ErrorHelper.getErrorInfo(emptyError);

      expect(result).toEqual({
        message: 'Bad Request',
        status: 400
      });
    });
  });

  describe('isErrorCode', () => {
    test('should return true for matching structured error code', () => {
      const error = {
        response: {
          data: {
            error: 'User not found',
            code: 'USER_NOT_FOUND',
            status: 404
          }
        }
      };

      expect(ErrorHelper.isErrorCode(error, ErrorCodes.USER_NOT_FOUND)).toBe(true);
    });

    test('should return false for non-matching structured error code', () => {
      const error = {
        response: {
          data: {
            error: 'User not found',
            code: 'USER_NOT_FOUND',
            status: 404
          }
        }
      };

      expect(ErrorHelper.isErrorCode(error, ErrorCodes.USER_DUPLICATE_USERNAME)).toBe(false);
    });

    test('should return false for legacy error without code', () => {
      const error = {
        response: {
          data: {
            message: 'Legacy error'
          }
        }
      };

      expect(ErrorHelper.isErrorCode(error, ErrorCodes.USER_NOT_FOUND)).toBe(false);
    });

    test('should return false for errors without response', () => {
      const error = new Error('Network Error');

      expect(ErrorHelper.isErrorCode(error, ErrorCodes.USER_NOT_FOUND)).toBe(false);
    });

    test('should handle null/undefined errors', () => {
      expect(ErrorHelper.isErrorCode(null, ErrorCodes.USER_NOT_FOUND)).toBe(false);
      expect(ErrorHelper.isErrorCode(undefined, ErrorCodes.USER_NOT_FOUND)).toBe(false);
    });
  });

  describe('getUserMessage', () => {
    test('should return error message for structured error', () => {
      const error = {
        response: {
          data: {
            error: 'Custom error message',
            code: 'USER_NOT_FOUND',
            status: 404
          }
        }
      };

      expect(ErrorHelper.getUserMessage(error)).toBe('Custom error message');
    });

    test('should return fallback message when provided', () => {
      const error = null;
      expect(ErrorHelper.getUserMessage(error, 'Custom fallback')).toBe('Custom fallback');
    });

    test('should return default fallback when no message', () => {
      const error = null;
      expect(ErrorHelper.getUserMessage(error)).toBe('An error occurred');
    });
  });

  describe('getSuggestedAction', () => {
    test('should return specific action for user duplicate username', () => {
      const error = {
        response: {
          data: {
            error: 'Username already exists',
            code: 'USER_DUPLICATE_USERNAME',
            status: 409
          }
        }
      };

      expect(ErrorHelper.getSuggestedAction(error)).toBe('Please choose a different username');
    });

    test('should return specific action for invalid credentials', () => {
      const error = {
        response: {
          data: {
            error: 'Invalid login',
            code: 'USER_INVALID_CREDENTIALS',
            status: 401
          }
        }
      };

      expect(ErrorHelper.getSuggestedAction(error)).toBe('Please check your username and password');
    });

    test('should return null for unknown error codes', () => {
      const error = {
        response: {
          data: {
            error: 'Unknown error',
            code: 'UNKNOWN_ERROR_CODE',
            status: 500
          }
        }
      };

      expect(ErrorHelper.getSuggestedAction(error)).toBe(null);
    });

    test('should return null for errors without codes', () => {
      const error = new Error('Generic error');
      expect(ErrorHelper.getSuggestedAction(error)).toBe(null);
    });
  });

  describe('shouldShowRetry', () => {
    test('should return true for retryable error codes', () => {
      const error = {
        response: {
          data: {
            error: 'Connection failed',
            code: 'SOURCE_CONNECTION_FAILED',
            status: 503
          }
        }
      };

      expect(ErrorHelper.shouldShowRetry(error)).toBe(true);
    });

    test('should return true for 5xx server errors', () => {
      const error = {
        response: {
          data: {
            message: 'Internal server error'
          },
          status: 500
        }
      };

      expect(ErrorHelper.shouldShowRetry(error)).toBe(true);
    });

    test('should return false for client errors', () => {
      const error = {
        response: {
          data: {
            error: 'Bad request',
            code: 'USER_INVALID_CREDENTIALS',
            status: 400
          }
        }
      };

      expect(ErrorHelper.shouldShowRetry(error)).toBe(false);
    });
  });

  describe('getErrorCategory', () => {
    test('should categorize user auth errors correctly', () => {
      const authCodes = [
        'USER_INVALID_CREDENTIALS',
        'USER_TOKEN_EXPIRED', 
        'USER_SESSION_EXPIRED'
      ];

      authCodes.forEach(code => {
        const error = {
          response: {
            data: { error: 'Test', code, status: 401 }
          }
        };
        expect(ErrorHelper.getErrorCategory(error)).toBe('auth');
      });
    });

    test('should categorize user validation errors correctly', () => {
      const validationCodes = [
        'USER_INVALID_PASSWORD',
        'USER_INVALID_EMAIL',
        'USER_INVALID_USERNAME'
      ];

      validationCodes.forEach(code => {
        const error = {
          response: {
            data: { error: 'Test', code, status: 400 }
          }
        };
        expect(ErrorHelper.getErrorCategory(error)).toBe('validation');
      });
    });

    test('should categorize network errors correctly', () => {
      const error = {
        response: {
          data: {
            error: 'Connection failed',
            code: 'SOURCE_CONNECTION_FAILED',
            status: 503
          }
        }
      };

      expect(ErrorHelper.getErrorCategory(error)).toBe('network');
    });

    test('should categorize by HTTP status for errors without specific codes', () => {
      const statusTests = [
        { status: 400, expectedCategory: 'validation' },
        { status: 401, expectedCategory: 'auth' },
        { status: 403, expectedCategory: 'auth' },
        { status: 422, expectedCategory: 'validation' },
        { status: 500, expectedCategory: 'server' },
        { status: 502, expectedCategory: 'server' },
        { status: 503, expectedCategory: 'server' }
      ];

      statusTests.forEach(({ status, expectedCategory }) => {
        const error = {
          response: {
            data: { message: 'Test error' },
            status
          }
        };
        expect(ErrorHelper.getErrorCategory(error)).toBe(expectedCategory);
      });
    });

    test('should return unknown for unclassified errors', () => {
      const error = new Error('Generic error');
      expect(ErrorHelper.getErrorCategory(error)).toBe('unknown');
    });
  });

  describe('getErrorIcon', () => {
    test('should return appropriate icons for error categories', () => {
      const iconTests = [
        { category: 'auth', expectedIcon: '🔒' },
        { category: 'validation', expectedIcon: '⚠️' },
        { category: 'network', expectedIcon: '🌐' },
        { category: 'server', expectedIcon: '🔧' },
        { category: 'unknown', expectedIcon: '❌' }
      ];

      iconTests.forEach(({ category, expectedIcon }) => {
        // Create an error that will categorize to the desired category
        let error;
        switch (category) {
          case 'auth':
            error = { response: { data: { code: 'USER_INVALID_CREDENTIALS' }, status: 401 } };
            break;
          case 'validation':
            error = { response: { data: { code: 'USER_INVALID_PASSWORD' }, status: 400 } };
            break;
          case 'network':
            error = { response: { data: { code: 'SOURCE_CONNECTION_FAILED' }, status: 503 } };
            break;
          case 'server':
            error = { response: { data: { message: 'Server error' }, status: 500 } };
            break;
          default:
            error = new Error('Unknown error');
        }

        expect(ErrorHelper.getErrorIcon(error)).toBe(expectedIcon);
      });
    });
  });

  describe('formatErrorForDisplay', () => {
    test('should format error with actions included', () => {
      const error = {
        response: {
          data: {
            error: 'Username already exists',
            code: 'USER_DUPLICATE_USERNAME',
            status: 409
          }
        }
      };

      const result = ErrorHelper.formatErrorForDisplay(error, true);

      expect(result.message).toBe('Username already exists');
      expect(result.code).toBe('USER_DUPLICATE_USERNAME');
      expect(result.status).toBe(409);
      expect(result.suggestedAction).toBe('Please choose a different username');
      expect(result.category).toBe('validation');
      expect(result.icon).toBe('⚠️');
      expect(result.severity).toBe('warning');
      expect(typeof result.shouldShowRetry).toBe('boolean');
    });

    test('should format error without actions', () => {
      const error = {
        response: {
          data: {
            error: 'Username already exists',
            code: 'USER_DUPLICATE_USERNAME',
            status: 409
          }
        }
      };

      const result = ErrorHelper.formatErrorForDisplay(error, false);

      expect(result.message).toBe('Username already exists');
      expect(result.suggestedAction).toBe(null);
      expect(result.shouldShowRetry).toBe(false);
    });

    test('should set correct severity based on category', () => {
      const severityTests = [
        { code: 'USER_INVALID_PASSWORD', expectedSeverity: 'warning' }, // validation
        { code: 'USER_INVALID_CREDENTIALS', expectedSeverity: 'info' }, // auth
        { code: 'SOURCE_CONNECTION_FAILED', expectedSeverity: 'error' } // network
      ];

      severityTests.forEach(({ code, expectedSeverity }) => {
        const error = {
          response: {
            data: { error: 'Test', code, status: 400 }
          }
        };
        
        const result = ErrorHelper.formatErrorForDisplay(error, true);
        expect(result.severity).toBe(expectedSeverity);
      });
    });
  });

  describe('handleSpecificError', () => {
    test('should handle session expired errors with login callback', () => {
      const error = {
        response: {
          data: {
            error: 'Session expired',
            code: 'USER_SESSION_EXPIRED',
            status: 401
          }
        }
      };

      const onLogin = vi.fn();
      const result = ErrorHelper.handleSpecificError(error, undefined, onLogin);

      expect(result).toBe(true);
      expect(onLogin).toHaveBeenCalled();
    });

    test('should handle retryable errors with retry callback', async () => {
      const error = {
        response: {
          data: {
            error: 'Connection failed',
            code: 'SOURCE_CONNECTION_FAILED',
            status: 503
          }
        }
      };

      const onRetry = vi.fn();
      const result = ErrorHelper.handleSpecificError(error, onRetry);

      expect(result).toBe(true);
      
      // Wait for the timeout to trigger
      await new Promise(resolve => setTimeout(resolve, 2100));
      expect(onRetry).toHaveBeenCalled();
    });

    test('should not handle non-specific errors', () => {
      const error = {
        response: {
          data: {
            error: 'Generic error',
            code: 'SOME_OTHER_ERROR',
            status: 400
          }
        }
      };

      const result = ErrorHelper.handleSpecificError(error);
      expect(result).toBe(false);
    });

    test('should not handle session expired without login callback', () => {
      const error = {
        response: {
          data: {
            error: 'Session expired',
            code: 'USER_SESSION_EXPIRED',
            status: 401
          }
        }
      };

      const result = ErrorHelper.handleSpecificError(error);
      expect(result).toBe(false);
    });
  });

  describe('ErrorCodes constants', () => {
    test('should have all required user error codes', () => {
      expect(ErrorCodes.USER_NOT_FOUND).toBe('USER_NOT_FOUND');
      expect(ErrorCodes.USER_DUPLICATE_USERNAME).toBe('USER_DUPLICATE_USERNAME');
      expect(ErrorCodes.USER_DUPLICATE_EMAIL).toBe('USER_DUPLICATE_EMAIL');
      expect(ErrorCodes.USER_INVALID_CREDENTIALS).toBe('USER_INVALID_CREDENTIALS');
      expect(ErrorCodes.USER_SESSION_EXPIRED).toBe('USER_SESSION_EXPIRED');
      expect(ErrorCodes.USER_TOKEN_EXPIRED).toBe('USER_TOKEN_EXPIRED');
      expect(ErrorCodes.USER_PERMISSION_DENIED).toBe('USER_PERMISSION_DENIED');
      expect(ErrorCodes.USER_ACCOUNT_DISABLED).toBe('USER_ACCOUNT_DISABLED');
    });

    test('should have all required source error codes', () => {
      expect(ErrorCodes.SOURCE_NOT_FOUND).toBe('SOURCE_NOT_FOUND');
      expect(ErrorCodes.SOURCE_CONNECTION_FAILED).toBe('SOURCE_CONNECTION_FAILED');
      expect(ErrorCodes.SOURCE_AUTH_FAILED).toBe('SOURCE_AUTH_FAILED');
      expect(ErrorCodes.SOURCE_CONFIG_INVALID).toBe('SOURCE_CONFIG_INVALID');
      expect(ErrorCodes.SOURCE_SYNC_IN_PROGRESS).toBe('SOURCE_SYNC_IN_PROGRESS');
    });

    test('should have all required label error codes', () => {
      expect(ErrorCodes.LABEL_NOT_FOUND).toBe('LABEL_NOT_FOUND');
      expect(ErrorCodes.LABEL_DUPLICATE_NAME).toBe('LABEL_DUPLICATE_NAME');
      expect(ErrorCodes.LABEL_INVALID_NAME).toBe('LABEL_INVALID_NAME');
      expect(ErrorCodes.LABEL_INVALID_COLOR).toBe('LABEL_INVALID_COLOR');
      expect(ErrorCodes.LABEL_IN_USE).toBe('LABEL_IN_USE');
      expect(ErrorCodes.LABEL_SYSTEM_MODIFICATION).toBe('LABEL_SYSTEM_MODIFICATION');
      expect(ErrorCodes.LABEL_MAX_LABELS_REACHED).toBe('LABEL_MAX_LABELS_REACHED');
    });
  });

  describe('Edge cases and robustness', () => {
    test('should handle malformed error objects', () => {
      const malformedErrors = [
        { response: null },
        { response: { data: null } },
        { response: { data: { error: null, code: null } } },
        { response: { status: 'not-a-number' } },
        { code: 123 }, // numeric code instead of string
        { message: { nested: 'object' } } // object instead of string
      ];

      malformedErrors.forEach(error => {
        expect(() => ErrorHelper.getErrorInfo(error)).not.toThrow();
        expect(() => ErrorHelper.formatErrorForDisplay(error, true)).not.toThrow();
        expect(() => ErrorHelper.isErrorCode(error, ErrorCodes.USER_NOT_FOUND)).not.toThrow();
        expect(() => ErrorHelper.getErrorCategory(error)).not.toThrow();
        expect(() => ErrorHelper.getErrorIcon(error)).not.toThrow();
      });
    });

    test('should handle very long error messages', () => {
      const longMessage = 'x'.repeat(10000);
      const error = {
        response: {
          data: {
            error: longMessage,
            code: 'USER_NOT_FOUND',
            status: 404
          }
        }
      };

      const result = ErrorHelper.getErrorInfo(error);
      expect(result.message).toBe(longMessage);
      expect(result.code).toBe('USER_NOT_FOUND');
    });

    test('should handle non-string error codes gracefully', () => {
      const error = {
        response: {
          data: {
            error: 'Test error',
            code: 12345, // numeric code
            status: 400
          }
        }
      };

      // Should not crash and should handle it as if no code was provided
      expect(() => ErrorHelper.getErrorInfo(error)).not.toThrow();
      expect(() => ErrorHelper.isErrorCode(error, ErrorCodes.USER_NOT_FOUND)).not.toThrow();
    });
  });

  describe('Type safety', () => {
    test('should handle ApiErrorResponse interface correctly', () => {
      const apiError: ApiErrorResponse = {
        error: 'Test error message',
        code: 'USER_NOT_FOUND',
        status: 404
      };

      const error = {
        response: {
          data: apiError,
          status: 404
        }
      };

      const result = ErrorHelper.getErrorInfo(error);
      expect(result.message).toBe('Test error message');
      expect(result.code).toBe('USER_NOT_FOUND');
      expect(result.status).toBe(404);
    });

    test('should handle AxiosErrorWithCode interface correctly', () => {
      const axiosError: AxiosErrorWithCode = {
        response: {
          data: {
            error: 'Axios error',
            code: 'SOURCE_CONNECTION_FAILED',
            status: 503
          },
          status: 503,
          statusText: 'Service Unavailable',
          headers: {}
        },
        message: 'Request failed',
        name: 'AxiosError'
      };

      const result = ErrorHelper.getErrorInfo(axiosError);
      expect(result.message).toBe('Axios error');
      expect(result.code).toBe('SOURCE_CONNECTION_FAILED');
      expect(result.status).toBe(503);
    });
  });
});