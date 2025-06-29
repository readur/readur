import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import axios from 'axios';
import { ocrService } from '../api';

// Mock axios
vi.mock('axios');
const mockedAxios = vi.mocked(axios);

describe('OCR API Service', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('getAvailableLanguages', () => {
    it('should fetch available languages successfully', async () => {
      const mockResponse = {
        data: {
          languages: [
            { code: 'eng', name: 'English' },
            { code: 'spa', name: 'Spanish' },
            { code: 'fra', name: 'French' },
          ],
          current_user_language: 'eng',
        },
        status: 200,
      };

      mockedAxios.get.mockResolvedValueOnce(mockResponse);

      const result = await ocrService.getAvailableLanguages();

      expect(mockedAxios.get).toHaveBeenCalledWith('/ocr/languages');
      expect(result).toEqual(mockResponse);
    });

    it('should handle network errors', async () => {
      const networkError = new Error('Network Error');
      mockedAxios.get.mockRejectedValueOnce(networkError);

      await expect(ocrService.getAvailableLanguages()).rejects.toThrow('Network Error');
      expect(mockedAxios.get).toHaveBeenCalledWith('/ocr/languages');
    });

    it('should handle empty language list', async () => {
      const mockResponse = {
        data: {
          languages: [],
          current_user_language: null,
        },
        status: 200,
      };

      mockedAxios.get.mockResolvedValueOnce(mockResponse);

      const result = await ocrService.getAvailableLanguages();

      expect(result.data.languages).toEqual([]);
      expect(result.data.current_user_language).toBeNull();
    });
  });

  describe('getHealthStatus', () => {
    it('should fetch OCR health status successfully', async () => {
      const mockResponse = {
        data: {
          status: 'healthy',
          tesseract_version: '5.3.0',
          available_languages: ['eng', 'spa', 'fra'],
        },
        status: 200,
      };

      mockedAxios.get.mockResolvedValueOnce(mockResponse);

      const result = await ocrService.getHealthStatus();

      expect(mockedAxios.get).toHaveBeenCalledWith('/ocr/health');
      expect(result).toEqual(mockResponse);
    });

    it('should handle unhealthy OCR service', async () => {
      const mockResponse = {
        data: {
          status: 'unhealthy',
          error: 'Tesseract not found',
        },
        status: 503,
      };

      mockedAxios.get.mockResolvedValueOnce(mockResponse);

      const result = await ocrService.getHealthStatus();

      expect(result.data.status).toBe('unhealthy');
      expect(result.data.error).toBe('Tesseract not found');
    });
  });

  describe('retryWithLanguage', () => {
    const documentId = 'doc-123';

    it('should retry OCR without language parameter', async () => {
      const mockResponse = {
        data: {
          success: true,
          message: 'OCR retry queued successfully',
          queue_id: 'queue-456',
          estimated_wait_minutes: 5,
        },
        status: 200,
      };

      mockedAxios.post.mockResolvedValueOnce(mockResponse);

      const result = await ocrService.retryWithLanguage(documentId);

      expect(mockedAxios.post).toHaveBeenCalledWith(
        `/documents/${documentId}/retry-ocr`,
        {}
      );
      expect(result).toEqual(mockResponse);
    });

    it('should retry OCR with language parameter', async () => {
      const language = 'spa';
      const mockResponse = {
        data: {
          success: true,
          message: 'OCR retry queued successfully',
          queue_id: 'queue-456',
          estimated_wait_minutes: 3,
        },
        status: 200,
      };

      mockedAxios.post.mockResolvedValueOnce(mockResponse);

      const result = await ocrService.retryWithLanguage(documentId, language);

      expect(mockedAxios.post).toHaveBeenCalledWith(
        `/documents/${documentId}/retry-ocr`,
        { language: 'spa' }
      );
      expect(result).toEqual(mockResponse);
    });

    it('should handle retry failure', async () => {
      const errorResponse = {
        response: {
          data: {
            success: false,
            message: 'Document not found',
          },
          status: 404,
        },
      };

      mockedAxios.post.mockRejectedValueOnce(errorResponse);

      await expect(ocrService.retryWithLanguage(documentId)).rejects.toEqual(errorResponse);
    });

    it('should handle queue full error', async () => {
      const errorResponse = {
        response: {
          data: {
            success: false,
            message: 'OCR queue is currently full. Please try again later.',
          },
          status: 429,
        },
      };

      mockedAxios.post.mockRejectedValueOnce(errorResponse);

      await expect(ocrService.retryWithLanguage(documentId, 'eng')).rejects.toEqual(errorResponse);
    });

    it('should handle invalid language error', async () => {
      const errorResponse = {
        response: {
          data: {
            success: false,
            message: 'Language "invalid" is not supported',
          },
          status: 400,
        },
      };

      mockedAxios.post.mockRejectedValueOnce(errorResponse);

      await expect(ocrService.retryWithLanguage(documentId, 'invalid')).rejects.toEqual(errorResponse);
    });

    it('should handle network timeout', async () => {
      const timeoutError = new Error('timeout of 10000ms exceeded');
      timeoutError.name = 'TimeoutError';
      
      mockedAxios.post.mockRejectedValueOnce(timeoutError);

      await expect(ocrService.retryWithLanguage(documentId)).rejects.toThrow('timeout of 10000ms exceeded');
    });

    it('should handle empty string language as undefined', async () => {
      const mockResponse = {
        data: {
          success: true,
          message: 'OCR retry queued successfully',
        },
        status: 200,
      };

      mockedAxios.post.mockResolvedValueOnce(mockResponse);

      await ocrService.retryWithLanguage(documentId, '');

      expect(mockedAxios.post).toHaveBeenCalledWith(
        `/documents/${documentId}/retry-ocr`,
        {}
      );
    });

    it('should preserve language whitespace and special characters', async () => {
      const language = 'chi_sim'; // Chinese Simplified
      const mockResponse = {
        data: {
          success: true,
          message: 'OCR retry queued successfully',
        },
        status: 200,
      };

      mockedAxios.post.mockResolvedValueOnce(mockResponse);

      await ocrService.retryWithLanguage(documentId, language);

      expect(mockedAxios.post).toHaveBeenCalledWith(
        `/documents/${documentId}/retry-ocr`,
        { language: 'chi_sim' }
      );
    });
  });

  describe('Error Handling', () => {
    it('should handle 401 unauthorized errors', async () => {
      const unauthorizedError = {
        response: {
          status: 401,
          data: {
            message: 'Unauthorized',
          },
        },
      };

      mockedAxios.get.mockRejectedValueOnce(unauthorizedError);

      await expect(ocrService.getAvailableLanguages()).rejects.toEqual(unauthorizedError);
    });

    it('should handle 403 forbidden errors', async () => {
      const forbiddenError = {
        response: {
          status: 403,
          data: {
            message: 'Insufficient permissions',
          },
        },
      };

      mockedAxios.get.mockRejectedValueOnce(forbiddenError);

      await expect(ocrService.getHealthStatus()).rejects.toEqual(forbiddenError);
    });

    it('should handle 500 internal server errors', async () => {
      const serverError = {
        response: {
          status: 500,
          data: {
            message: 'Internal server error',
          },
        },
      };

      mockedAxios.post.mockRejectedValueOnce(serverError);

      await expect(ocrService.retryWithLanguage('doc-123')).rejects.toEqual(serverError);
    });

    it('should handle malformed response data', async () => {
      const malformedResponse = {
        data: null,
        status: 200,
      };

      mockedAxios.get.mockResolvedValueOnce(malformedResponse);

      const result = await ocrService.getAvailableLanguages();
      expect(result.data).toBeNull();
    });
  });

  describe('Request Configuration', () => {
    it('should use correct base URL', async () => {
      const mockResponse = { data: {}, status: 200 };
      mockedAxios.get.mockResolvedValueOnce(mockResponse);

      await ocrService.getAvailableLanguages();

      expect(mockedAxios.get).toHaveBeenCalledWith('/ocr/languages');
    });

    it('should handle concurrent requests', async () => {
      const mockResponse = { data: {}, status: 200 };
      mockedAxios.get.mockResolvedValue(mockResponse);
      mockedAxios.post.mockResolvedValue(mockResponse);

      const requests = [
        ocrService.getAvailableLanguages(),
        ocrService.getHealthStatus(),
        ocrService.retryWithLanguage('doc-1', 'eng'),
        ocrService.retryWithLanguage('doc-2', 'spa'),
      ];

      await Promise.all(requests);

      expect(mockedAxios.get).toHaveBeenCalledTimes(2);
      expect(mockedAxios.post).toHaveBeenCalledTimes(2);
    });
  });
});