import { useState, useEffect } from 'react';
import upstashService from '../ConfigApi/upstashService';

export const useUserSettings = () => {
  const [settings, setSettings] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);

  const fetchSettings = async () => {
    try {
      setLoading(true);
      setError(null);
      const response = await upstashService.getUserSettings();
      if (response.success) {
        setSettings(response.data);
      } else {
        setError(response.message || 'Failed to fetch user settings');
      }
    } catch (err) {
      setError(err.message || 'An error occurred while fetching user settings');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchSettings();
  }, []);

  return { settings, loading, error, refetch: fetchSettings };
};