import { useState, useEffect } from 'react';
import { Box, Typography } from '@mui/material';
import {  FolderOpen as FolderOpenIcon } from '@mui/icons-material';
import { motion } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import GoogleDrivePicker from '../../components/GoogleDrivePicker';
import axios from 'axios';
import toast from 'react-hot-toast';
import CreateTemplateButton from '../../components/CreateTemplateButton';
const EmptyState = () => {
  const { t } = useTranslation();
  const [showGoogleDrivePicker, setShowGoogleDrivePicker] = useState(false);

  // Check if we just returned from Google OAuth
  useEffect(() => {
    const urlParams = new URLSearchParams(window.location.search);
    if (urlParams.get('google_drive_connected') === '1') {
      // Remove the query parameter
      window.history.replaceState({}, '', window.location.pathname);
      // Auto-open the picker
      setShowGoogleDrivePicker(true);
      toast.success('Google Drive connected successfully!');
    }
  }, []);

  const handleGoogleDriveSelect = async (files: any[]) => {
    if (files.length > 0) {
      const file = files[0];
      try {
        console.log('Creating template from Google Drive file:', file);
        // Create template from Google Drive file
        const response = await axios.post('/api/templates/google_drive_documents', {
          google_drive_file_ids: [file.id],
          name: file.name.replace('.pdf', '')
        }, {
          headers: {
            Authorization: `Bearer ${localStorage.getItem('token')}`
          }
        });

        if (response.data.success) {
          toast.success('Template created successfully!');
          window.location.reload(); // Refresh to show new template
        } else {
          const errorMsg = response.data.message || 'Failed to create template';
          console.error('Template creation failed:', errorMsg);
          toast.error(errorMsg);
        }
      } catch (error: any) {
        console.error('Error creating template from Google Drive:', error);
        const errorMsg = error.response?.data?.message || error.message || 'Failed to create template from Google Drive';
        console.error('Error details:', error.response?.data);
        toast.error(errorMsg);
      }
    }
  };

  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.9 }}
      animate={{ opacity: 1, scale: 1 }}
      transition={{ delay: 0.2 }}
    >
      <Box sx={{
        textAlign: 'center',
      }}>
        <Box>
          <FolderOpenIcon sx={{ fontSize: { xs: 40, sm: 60 }, color: 'white' }} />
        </Box>

        <Typography
          variant="h3"
          component="h3"
          fontWeight="800"
        >
          {t('dashboard.emptyState.title')}
        </Typography>

        <Typography variant="h5" sx={{ color: '#94a3b8', mb: 2, maxWidth: 600, mx: 'auto', lineHeight: 1.6, fontSize: { xs: '1rem', sm: '1.25rem' } }}>
          {t('dashboard.emptyState.subtitle')}
        </Typography>
        <Box sx={{ display: 'flex', justifyContent: 'center', gap: 2 }}>
          <CreateTemplateButton
            text={t('dashboard.emptyState.googleDriveButton')}
            onClick={() => setShowGoogleDrivePicker(true)}
            icon={<FolderOpenIcon />}
          />
        </Box>
      </Box>

      <GoogleDrivePicker
        open={showGoogleDrivePicker}
        onClose={() => setShowGoogleDrivePicker(false)}
        onFileSelect={handleGoogleDriveSelect}
      />
    </motion.div>
  );
};

export default EmptyState;