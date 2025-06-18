import { useContext } from 'react';
import { api } from '../services/api';
import { AuthContext } from '../contexts/AuthContext';

export const useApi = () => {
  const { user } = useContext(AuthContext);
  
  // Ensure the API instance has the current auth token
  const token = localStorage.getItem('token');
  if (token && user) {
    api.defaults.headers.common['Authorization'] = `Bearer ${token}`;
  }
  
  return api;
};