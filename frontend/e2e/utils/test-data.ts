export const TEST_USERS = {
  valid: {
    username: 'admin',
    password: 'readur2024'
  },
  invalid: {
    username: 'invaliduser',
    password: 'wrongpassword'
  }
};

export const TEST_FILES = {
  pdf: 'test_data/sample.pdf',
  image: 'test_data/hello_ocr.png',
  text: 'test_data/sample.txt',
  multiline: 'test_data/multiline.png',
  numbers: 'test_data/numbers.png'
};

export const SEARCH_QUERIES = {
  simple: 'test document',
  advanced: {
    title: 'important',
    content: 'contract',
    dateFrom: '2024-01-01',
    dateTo: '2024-12-31'
  },
  empty: '',
  noResults: 'xyzabc123nonexistent'
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