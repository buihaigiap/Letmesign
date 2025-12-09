import React, { useState, useEffect } from 'react';
import GoogleDrivePicker from '../components/GoogleDrivePicker';
import { useGoogleDriveAutoOpen } from '../hooks/useGoogleDriveAutoOpen';

interface GoogleDriveManagerProps {
  onFileSelect: (files: any[]) => void;
  triggerButton?: React.ReactNode;
  children?: React.ReactNode;
}

const GoogleDriveManager: React.FC<GoogleDriveManagerProps> = ({
  onFileSelect,
  triggerButton,
  children
}) => {
  const [showGoogleDrivePicker, setShowGoogleDrivePicker] = useState(false);
  const { shouldOpenGoogleDrive, resetAutoOpen } = useGoogleDriveAutoOpen();

  // Auto-open Google Drive picker when returning from OAuth
  useEffect(() => {
    if (shouldOpenGoogleDrive) {
      setShowGoogleDrivePicker(true);
      resetAutoOpen();
    }
  }, [shouldOpenGoogleDrive, resetAutoOpen]);

  const handleClose = () => {
    setShowGoogleDrivePicker(false);
  };

  const handleOpen = () => {
    setShowGoogleDrivePicker(true);
  };

  return (
    <>
      {triggerButton && (
        <div onClick={handleOpen}>
          {triggerButton}
        </div>
      )}
      {children && (
        <div onClick={handleOpen}>
          {children}
        </div>
      )}
      <GoogleDrivePicker
        open={showGoogleDrivePicker}
        onClose={handleClose}
        onFileSelect={onFileSelect}
      />
    </>
  );
};

export default GoogleDriveManager;