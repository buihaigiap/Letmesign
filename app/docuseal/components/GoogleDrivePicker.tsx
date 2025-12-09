import React, { useState, useEffect, useRef } from 'react';
import { Box, Modal, Button, Typography, CircularProgress } from '@mui/material';
import { Close as CloseIcon, CloudUpload as CloudUploadIcon } from '@mui/icons-material';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';

interface GoogleDrivePickerProps {
  open: boolean;
  onClose: () => void;
  onFileSelect: (files: any[]) => void;
  forceReauth?: boolean;
}

const GoogleDrivePicker: React.FC<GoogleDrivePickerProps> = ({
  open,
  onClose,
  onFileSelect,
  forceReauth = false
}) => {
  const { t } = useTranslation();
  const [isLoading, setIsLoading] = useState(true);
  const [showOAuth, setShowOAuth] = useState(false);
  const iframeRef = useRef<HTMLIFrameElement>(null);

  // Check for Google Drive connection on mount and when modal opens
  useEffect(() => {
    const checkGoogleDriveConnection = () => {
      const urlParams = new URLSearchParams(window.location.search);
      if (urlParams.get('google_drive_connected') === '1') {
        // Remove the query parameter from URL
        const newUrl = window.location.pathname + window.location.hash;
        window.history.replaceState({}, '', newUrl);
        
        // Show success message (you might want to use toast here)
        console.log('Google Drive connected successfully!');
        
        // The modal should already be open via the open prop
        // But we can ensure it's visible
      }
    };

    if (open) {
      checkGoogleDriveConnection();
    }
  }, [open]);

  useEffect(() => {
    const handleMessage = (event: MessageEvent) => {
      if (event.data.type === 'google-drive-files-picked') {
        const files = event.data.files || [];
        onFileSelect(files);
        onClose();
      } else if (event.data.type === 'google-drive-picker-loaded') {
        setIsLoading(false);
      } else if (event.data.type === 'google-drive-picker-request-oauth') {
        // Redirect to OAuth instead of showing OAuth button
        handleOAuth();
      }
    };

    if (open) {
      window.addEventListener('message', handleMessage);
      setIsLoading(true);
      setShowOAuth(false);
    }

    return () => {
      window.removeEventListener('message', handleMessage);
    };
  }, [open, onClose, onFileSelect]);

  const handleOAuth = () => {
    // Get current JWT token from localStorage
    const token = localStorage.getItem('token');
    
    // Redirect to Google OAuth
    const state = JSON.stringify({
      redir: window.location.pathname,
      token: token // Include JWT token in state
    });

    window.location.href = `/auth/google_oauth2?state=${encodeURIComponent(state)}`;
  };

  return (
    <Modal
      open={open}
      onClose={onClose}
      aria-labelledby="google-drive-modal"
      sx={{
        display: 'flex',
        alignItems: 'flex-start',
        justifyContent: 'center',
        pt: 5,
        px: 2
      }}
    >
      <Box
        sx={{
          bgcolor: 'background.paper',
          borderRadius: 2,
          boxShadow: 24,
          width: '100%',
          maxWidth: 650,
          maxHeight: '80vh',
          display: 'flex',
          flexDirection: 'column'
        }}
      >
        {/* Header */}
        <Box
          sx={{
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
            p: 2,
            borderBottom: 1,
            borderColor: 'divider'
          }}
        >
          <Typography variant="h6" component="h2">
            {t('googleDrive.title')}
          </Typography>
          <Button onClick={onClose} size="small">
            <CloseIcon  sx={{color : 'white'}}/>
          </Button>
        </Box>

        {/* Content */}
        <Box sx={{ flex: 1, position: 'relative', minHeight: 400 }}>
          {showOAuth ? (
            <Box
              sx={{
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                justifyContent: 'center',
                height: '100%',
                p: 3
              }}
            >
              <CloudUploadIcon sx={{ fontSize: 48, color: 'primary.main', mb: 2 }} />
              <Typography variant="h6" gutterBottom>
                {t('googleDrive.connectTitle')}
              </Typography>
              <Typography variant="body2" color="text.secondary" textAlign="center" mb={3}>
                {t('googleDrive.connectDescription')}
              </Typography>
              <Button
                variant="contained"
                onClick={handleOAuth}
                startIcon={<CloudUploadIcon />}
              >
                {t('googleDrive.connectButton')}
              </Button>
            </Box>
          ) : (
            <>
              {isLoading && (
                <Box
                  sx={{
                    position: 'absolute',
                    top: 0,
                    left: 0,
                    right: 0,
                    bottom: 0,
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    bgcolor: 'rgba(255, 255, 255, 0.8)',
                    zIndex: 1
                  }}
                >
                  <CircularProgress />
                </Box>
              )}
              <iframe
                ref={iframeRef}
                src={`/template_google_drive${forceReauth ? '?force_reauth=1' : ''}`}
                style={{
                  width: '100%',
                  height: '100%',
                  minHeight: 400,
                  border: 'none',
                  borderRadius: '0 0 8px 8px'
                }}
                title="Google Drive Picker"
              />
              {/* Overlay to cover Google Picker's close button */}
              <Box
                sx={{
                  position: 'absolute',
                  top: 0,
                  right: 0,
                  width: 60,
                  height: 60,
                  bgcolor: 'white',
                  zIndex: 9999,
                  pointerEvents: 'none'
                }}
              />
            </>
          )}
        </Box>
      </Box>
    </Modal>
  );
};

export default GoogleDrivePicker;