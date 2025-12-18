
import { Box, Typography, Button, Alert, CircularProgress } from '@mui/material';
import { useAuth } from '@/contexts/AuthContext';
import { useEffect, useState } from 'react';
import CreateTemplateButton from '@/components/CreateTemplateButton';
import { Eye } from 'lucide-react';
import { useUserSettings } from '@/hooks/useUserSettings';
const ApiKey = () => {
  const { user, refreshUser } = useAuth();
  const {settings} =useUserSettings();
  const [showApiKey, setShowApiKey] = useState(false);
  useEffect (() => {
    refreshUser()
  }, []);

  const maskApiKey = (key) => {
    if (!key || key.length <= 6) return key;
    return key.slice(0, 3) + '*'.repeat(key.length - 6) + key.slice(-3);
  };

  const displayApiKey = showApiKey ? user?.api_key : maskApiKey(user?.api_key);

  return (
    <>
        <Typography variant="h4" sx={{ mb: 3 }}>
            API Key Settings
        </Typography>
        <Box sx={{
                display: 'flex',
                alignItems: 'flex-end', 
                gap: 3,
                width: '100%',
              }}
      >
            <Box
                width='100%'
            >
                <Typography variant="h6" >
                    Current API Key:
                </Typography>
                <Box
                sx={{
                    bgcolor: 'grey.100',
                    border: '1px solid rgba(248, 239, 239, 0.81)',
                    borderRadius: 1,
                    p: 1,
                    fontSize: '1rem',
                    backgroundColor: 'transparent',
                }}
                >
                    {displayApiKey}
                </Box>
            </Box>
            <CreateTemplateButton
                onClick={() => setShowApiKey(!showApiKey)}
                icon={<Eye />}
                text={showApiKey ? 'Hide' : 'Show'}
            />
      </Box>
    </>
  );
};

export default ApiKey;