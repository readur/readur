import { TEST_CREDENTIALS } from '../fixtures/auth';

export const TEST_USERS = {
  valid: TEST_CREDENTIALS.admin,
  invalid: {
    username: 'invaliduser',
    password: 'wrongpassword'
  }
};

export const TEST_FILES = {
  // Real test images with known OCR content
  test1: '../tests/test_images/test1.png',     // "Test 1\nThis is some text from text 1"
  test2: '../tests/test_images/test2.jpg',     // "Test 2\nThis is some text from text 2"
  test3: '../tests/test_images/test3.jpeg',    // "Test 3\nThis is some text from text 3"
  test4: '../tests/test_images/test4.png',     // "Test 4\nThis is some text from text 4"
  test5: '../tests/test_images/test5.jpg',     // "Test 5\nThis is some text from text 5"
  test6: '../tests/test_images/test6.jpeg',    // "Test 6\nThis is some text from text 6"
  test7: '../tests/test_images/test7.png',     // "Test 7\nThis is some text from text 7"
  test8: '../tests/test_images/test8.jpeg',    // "Test 8\nThis is some text from text 8"
  test9: '../tests/test_images/test9.png',     // "Test 9\nThis is some text from text 9"
  
  // Multilingual test PDFs
  spanishTest: 'test_data/multilingual/spanish_test.pdf',
  englishTest: 'test_data/multilingual/english_test.pdf',
  mixedLanguageTest: 'test_data/multilingual/mixed_language_test.pdf',
  spanishComplex: 'test_data/multilingual/spanish_complex.pdf',
  englishComplex: 'test_data/multilingual/english_complex.pdf',
  
  // Backwards compatibility
  image: '../tests/test_images/test1.png',
  multiline: '../tests/test_images/test2.jpg',
  text: 'test_data/sample.txt'
};

export const SEARCH_QUERIES = {
  simple: 'Test 1',  // Will match test1.png OCR content
  content: 'some text from text',  // Will match multiple test images
  specific: 'Test 3',  // Will match test3.jpeg specifically
  advanced: {
    title: 'Test',
    content: 'some text',
    dateFrom: '2024-01-01',
    dateTo: '2024-12-31'
  },
  empty: '',
  noResults: 'xyzabc123nonexistent'
};

// Expected OCR content for test images
export const EXPECTED_OCR_CONTENT = {
  test1: 'Test 1\nThis is some text from text 1',
  test2: 'Test 2\nThis is some text from text 2',
  test3: 'Test 3\nThis is some text from text 3',
  test4: 'Test 4\nThis is some text from text 4',
  test5: 'Test 5\nThis is some text from text 5',
  test6: 'Test 6\nThis is some text from text 6',
  test7: 'Test 7\nThis is some text from text 7',
  test8: 'Test 8\nThis is some text from text 8',
  test9: 'Test 9\nThis is some text from text 9'
};

export const API_ENDPOINTS = {
  login: '/api/auth/login',
  upload: '/api/documents/upload',
  search: '/api/search',
  documents: '/api/documents',
  settings: '/api/settings'
};

export const TIMEOUTS = {
  short: 5000,
  medium: 10000,
  long: 30000,
  upload: 60000,
  ocr: 120000
};