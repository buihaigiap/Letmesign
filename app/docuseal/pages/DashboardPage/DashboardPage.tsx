import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '../../contexts/AuthContext';
import { Template } from '../../types';
import upstashService from '../../ConfigApi/upstashService';
import { Box, CircularProgress } from '@mui/material';
import { motion } from 'framer-motion';
import toast from 'react-hot-toast';
import DashboardHeader from './DashboardHeader';
import DashboardError from './DashboardError';
import TemplatesGrid from './TemplatesGrid';
import EmptyState from './EmptyState';
import FoldersList from '../../components/FoldersList';
import NewTemplateModal from '../../components/NewTemplateModal';
import TwoFactorSetup from '../../components/TwoFactorSetup';

const DashboardPage = () => {
  const [templates, setTemplates] = useState<Template[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [folders, setFolders] = useState<any[]>([]);
  const [showNewTemplateModal, setShowNewTemplateModal] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [requires2FA, setRequires2FA] = useState(false);
  const navigate = useNavigate();
  const { token, user, refreshUser } = useAuth();
  // Check if we just returned from Google OAuth
  useEffect(() => {
    const urlParams = new URLSearchParams(window.location.search);
    if (urlParams.get('google_drive_connected') === '1') {
      // Remove the query parameter
      window.history.replaceState({}, '', window.location.pathname);
      toast.success('Google Drive connected successfully!');
    }

    // Check for redirect after login
    const redirectUrl = localStorage.getItem('redirectAfterLogin');
    if (redirectUrl && token) {
      console.log('Found redirect URL, navigating to:', redirectUrl);
      localStorage.removeItem('redirectAfterLogin');
      window.location.href = redirectUrl;
    }
  }, [token, navigate]);  const fetchTemplates = async () => {
    if (!token) {
        setError("Authentication token not found.");
        setLoading(false);
        return;
    }
    try {
      setLoading(true);
      setError('');
      const data = await upstashService.getTemplates();
      if (data.success) {
        setTemplates(data.data);
      } else {
        setError(data.message || 'Failed to fetch templates.');
      }

      // Fetch folders
      const foldersData = await upstashService.getFolders();
      if (foldersData.success) {
        setFolders(foldersData.data || []);
      }
    } catch (err) {
      setError('An unexpected error occurred while fetching templates.');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchTemplates();
  }, [token]);

  // Check 2FA requirements
  useEffect(() => {
    const check2FARequirements = async () => {
      if (!user) {
        console.log('check2FARequirements: no user, skipping');
        return;
      }


      try {
        const settingsResponse = await upstashService.getBasicSettings();
        if (settingsResponse.success) {
          const force2FA = settingsResponse.data.force_2fa_with_authenticator_app;
          const has2FA = user.two_factor_enabled;
          const newRequires2FA = force2FA && !has2FA;

          console.log('check2FARequirements: force2FA:', force2FA, 'has2FA:', has2FA, 'newRequires2FA:', newRequires2FA);
          setRequires2FA(newRequires2FA);
        } else {
          console.log('check2FARequirements: failed to get settings');
        }
      } catch (err) {
        console.error('Failed to fetch global settings:', err);
      }
    };

    check2FARequirements();
  }, [user]);

  const handle2FASuccess = async () => {

    // Refresh user data to get updated 2FA status
    await refreshUser();


    // Force re-check of 2FA requirements by triggering useEffect
    // We can do this by temporarily setting requires2FA to trigger re-evaluation
    setRequires2FA(false);

    // Refresh templates data after 2FA setup
    await fetchTemplates();
  };  const filteredFolders = folders.filter(folder =>
    folder.name.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const filteredTemplates = templates.filter(template =>
    template.name.toLowerCase().includes(searchQuery.toLowerCase())
  );
  if (requires2FA) {
    return <TwoFactorSetup onSuccess={handle2FASuccess} />;
  }


  return (
    <Box sx={{
      marginTop: { xs: 4, md: 6 },
    }}>
      <Box >
        {/* Header Section */}
        <DashboardHeader 
          onCreateNew={() => setShowNewTemplateModal(true)} 
          searchQuery={searchQuery}
          onSearchChange={setSearchQuery}
        />

        {/* Content Container */}
        <motion.div
          initial={{ opacity: 0, scale: 0.95, y: 20 }}
          animate={{ opacity: 1, scale: 1, y: 0 }}
          transition={{ delay: 0.3, duration: 0.6, ease: "easeOut" }}
        >
          <FoldersList folders={filteredFolders} />

        {loading ? (
            <Box sx={{ display: 'flex', justifyContent: 'center', alignItems: 'center', minHeight: '400px' }}>
              <CircularProgress size={60} />
            </Box>
          ) : error ? (
            <DashboardError error={error} />
          ) : (
            <>
              {filteredTemplates.length > 0 && (
                <TemplatesGrid templates={filteredTemplates} onRefresh={fetchTemplates} />
              )}
              <EmptyState />
            </>
          )}
        </motion.div>
      </Box>
      <NewTemplateModal
        open={showNewTemplateModal}
        onClose={() => setShowNewTemplateModal(false)}
        folderId={null}
        onSuccess={fetchTemplates}
      />
    </Box>
  );
};

export default DashboardPage;
