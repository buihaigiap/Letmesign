import { useState, useEffect } from 'react';
import toast from 'react-hot-toast';
import { useTranslation } from 'react-i18next';

export const useGoogleDriveAutoOpen = () => {
  const [shouldOpenGoogleDrive, setShouldOpenGoogleDrive] = useState(false);
  const { t } = useTranslation();

  useEffect(() => {
    const urlParams = new URLSearchParams(window.location.search);
    if (urlParams.get('google_drive_connected') === '1') {
      // Remove the query parameter
      window.history.replaceState({}, '', window.location.pathname);
      // Auto-open the picker
      setShouldOpenGoogleDrive(true);
      toast.success(t('googleDrive.connected', 'Google Drive connected successfully!'));
    }
  }, [t]);

  const resetAutoOpen = () => setShouldOpenGoogleDrive(false);

  return { shouldOpenGoogleDrive, resetAutoOpen };
};