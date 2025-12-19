
import { Box, Typography, Button, Alert, CircularProgress, Dialog, DialogTitle, DialogContent, DialogActions, TextField } from '@mui/material';
import { useEffect, useState } from 'react';
import CreateTemplateButton from '@/components/CreateTemplateButton';
import { Eye, RotateCcw } from 'lucide-react';
import { useUserSettings } from '@/hooks/useUserSettings';
import upstashService from '@/ConfigApi/upstashService';
import { maskApiKey } from '@/utils';
import { useAuth } from '@/contexts/AuthContext';
const ApiKey = () => {
  const {settings} =useUserSettings();
  const { user } = useAuth();
  const [showApiKey, setShowApiKey] = useState(false);
  const [rotating, setRotating] = useState(false);
  const [apiKey, setApiKey] = useState('');
  const [loading, setLoading] = useState(true);
  const [show2FADialog, setShow2FADialog] = useState(false);
  const [twoFactorCode, setTwoFactorCode] = useState('');
  const [verifying2FA, setVerifying2FA] = useState(false);
  const [twoFAError, setTwoFAError] = useState('');
  const [showSetup2FADialog, setShowSetup2FADialog] = useState(false);
  const [qrCodeUrl, setQrCodeUrl] = useState('');
  const [setupCode, setSetupCode] = useState('');
  const [settingUp2FA, setSettingUp2FA] = useState(false);
  const [setupError, setSetupError] = useState('');
  useEffect (() => {
    fetchApiKey();
  }, []);
  const fetchApiKey = async () => {
    try {
      const response = await upstashService.getApiKey();
      setApiKey(response.data.api_key);
    } catch (error) {
      console.error('Failed to fetch API key:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleShowApiKey = () => {
     if(!user?.two_factor_enabled) {
       fetch2FASetup();
       setShowSetup2FADialog(true);
    }
     else if (settings?.force_2fa_with_authenticator_app && !showApiKey) {
      setShow2FADialog(true);
    } else {
      setShowApiKey(!showApiKey);
    }
  };

  const handleVerify2FA = async () => {
    if (!twoFactorCode.trim()) {
      return;
    }

    setVerifying2FA(true);
    setTwoFAError('');
    try {
      const response = await upstashService.verify2FAAction({
        code: twoFactorCode.trim(),
      });

      if (response.success) {
        setShow2FADialog(false);
        setTwoFactorCode('');
        setShowApiKey(true); 
      } else {
        setTwoFAError(response.error || 'Invalid 2FA code. Please try again.');
      }
    } catch (error) {
      console.error('2FA verification error:', error);
      setTwoFAError('Failed to verify 2FA code. Please try again.');
    } finally {
      setVerifying2FA(false);
    }
  };

  const handleClose2FADialog = () => {
    setShow2FADialog(false);
    setTwoFactorCode('');
    setTwoFAError('');
  };

  const fetch2FASetup = async () => {
    try {
      const response = await upstashService.setup2FA();
      if (response.success) {
        setQrCodeUrl(response.data.qr_code_url);
      } else {
        setSetupError('Failed to setup 2FA. Please try again.');
      }
    } catch (error) {
      console.error('2FA setup error:', error);
      setSetupError('Failed to setup 2FA. Please try again.');
    }
  };

  const handleVerifySetup2FA = async () => {
    if (!setupCode.trim()) {
      return;
    }
    setSettingUp2FA(true);
    setSetupError('');
    try {
      const response = await upstashService.verify2FA({
        code: setupCode.trim(),
      });

      if (response.success) {
        setShowSetup2FADialog(false);
        setSetupCode('');
        setQrCodeUrl('');
        // Refetch settings to update two_factor_enabled
        const settingsResponse = await upstashService.getUserSettings();
        if (settingsResponse.success) {
          // Assuming the hook updates, but since it's local, we might need to refetch
          window.location.reload(); 
        }
      } else {
        setSetupError(response.error || 'Invalid 2FA code. Please try again.');
      }
    } catch (error) {
      console.error('2FA setup verification error:', error);
      setSetupError('Failed to verify 2FA code. Please try again.');
    } finally {
      setSettingUp2FA(false);
    }
  };

  const handleCloseSetup2FADialog = () => {
    setShowSetup2FADialog(false);
    setSetupCode('');
    setQrCodeUrl('');
    setSetupError('');
  };

  const displayApiKey = showApiKey ? apiKey : maskApiKey(apiKey);

  const handleRotateApiKey = async () => {
    setRotating(true);
    try {
      await upstashService.rotateApiKey();
      await fetchApiKey();
    } catch (error) {
      console.error('Failed to rotate API key:', error);
    } finally {
      setRotating(false);
    }
  };

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
                    border: '1px solid rgba(68, 60, 60, 0.81)',
                    borderRadius: 1,
                    p: 1,
                    fontSize: '1rem',
                    backgroundColor: 'transparent',
                    minHeight: '2rem',
                    display: 'flex',
                    alignItems: 'center',
                }}
                >
                    {loading ? <CircularProgress size={20} /> : displayApiKey}
                </Box>
            </Box>
            <CreateTemplateButton
                onClick={handleShowApiKey}
                icon={<Eye />}
                text={showApiKey ? 'Hide' : 'Show'}
                disabled={loading}
            />
            <CreateTemplateButton
                onClick={handleRotateApiKey}
                disabled={rotating || loading}
                icon={rotating ? <CircularProgress size={16} /> : <RotateCcw />}
                text="Rotate"
            />
      </Box>

      {/* 2FA Verification Dialog */}
      <Dialog
        open={show2FADialog}
        onClose={handleClose2FADialog}
      >
        <DialogTitle sx={{ color: 'white', textAlign: 'center' }}>
          Two-Factor Authentication Required
        </DialogTitle>
        <DialogContent>
          <Typography variant="body2" sx={{ mb: 2, opacity: 0.8, textAlign: 'center' }}>
            Enter the 6-digit code from your authenticator app to view your API key.
          </Typography>
          <TextField
           fullWidth
            variant="outlined"
            placeholder="000000"
            value={twoFactorCode}
            onChange={(e) => {
              const value = e.target.value.replace(/\D/g, '').slice(0, 6);
              setTwoFactorCode(value);
            }}
            inputProps={{ maxLength: 6 }}
            sx={{
                mb: 2,
                '& .MuiOutlinedInput-root': {
                  backgroundColor: 'rgba(255, 255, 255, 0.1)',
                  '& fieldset': {
                    borderColor: 'rgba(255, 255, 255, 0.3)',
                  },
                  '&:hover fieldset': {
                    borderColor: 'rgba(255, 255, 255, 0.5)',
                  },
                  '&.Mui-focused fieldset': {
                    borderColor: 'white',
                  },
                },
                '& .MuiInputBase-input': {
                  color: 'white',
                  textAlign: 'center',
                  fontSize: '1.5rem',
                  letterSpacing: '0.5rem',
                },
              }}
            disabled={verifying2FA}
            error={!!twoFAError}
            helperText={twoFAError}
          />
        </DialogContent>
        <DialogActions sx={{ justifyContent: 'center', pb: 3 }}>
          <Button
            onClick={handleClose2FADialog}
            disabled={verifying2FA}
            sx={{
              color: 'white',
              border: '1px solid rgba(255, 255, 255, 0.3)',
              '&:hover': {
                backgroundColor: 'rgba(255, 255, 255, 0.1)',
                borderColor: 'rgba(255, 255, 255, 0.5)',
              },
            }}
          >
            Cancel
          </Button>
          <Button
            onClick={handleVerify2FA}
            disabled={verifying2FA || twoFactorCode.length !== 6}
            variant="contained"
            sx={{
              backgroundColor: 'white',
              color: 'black',
              '&:hover': {
                backgroundColor: 'rgba(255, 255, 255, 0.9)',
              },
              '&:disabled': {
                backgroundColor: 'rgba(255, 255, 255, 0.3)',
                color: 'rgba(255, 255, 255, 0.5)',
              },
            }}
          >
            {verifying2FA ? <CircularProgress size={20} /> : 'Verify'}
          </Button>
        </DialogActions>
      </Dialog>

      {/* 2FA Setup Dialog */}
      <Dialog
        open={showSetup2FADialog}
        onClose={handleCloseSetup2FADialog}
      >
        <DialogTitle sx={{ color: 'white', textAlign: 'center' }}>
          Set Up Two-Factor Authentication
        </DialogTitle>
        <DialogContent>
          <Typography variant="body2" sx={{ mb: 2, opacity: 0.8, textAlign: 'center' }}>
            Scan the QR code below with your authenticator app and enter the 6-digit code to enable 2FA.
          </Typography>
          {qrCodeUrl && (
            <Box sx={{ display: 'flex', justifyContent: 'center', mb: 2 }}>
              <img
                        src={`data:image/png;base64,${qrCodeUrl}`}
                        alt="2FA QR Code"
                        style={{
                          maxWidth: '200px',
                          maxHeight: '200px',
                          border: '2px solid white',
                          borderRadius: '8px',
                        }}
                      />
            </Box>
          )}
          <TextField
            fullWidth
            variant="outlined"
            placeholder="000000"
            value={setupCode}
            onChange={(e) => {
              const value = e.target.value.replace(/\D/g, '').slice(0, 6);
              setSetupCode(value);
            }}
            inputProps={{ maxLength: 6 }}
            sx={{
              mb: 2,
              '& .MuiOutlinedInput-root': {
                backgroundColor: 'rgba(255, 255, 255, 0.1)',
                '& fieldset': {
                  borderColor: 'rgba(255, 255, 255, 0.3)',
                },
                '&:hover fieldset': {
                  borderColor: 'rgba(255, 255, 255, 0.5)',
                },
                '&.Mui-focused fieldset': {
                  borderColor: 'white',
                },
              },
              '& .MuiInputBase-input': {
                color: 'white',
                textAlign: 'center',
                fontSize: '1.5rem',
                letterSpacing: '0.5rem',
              },
            }}
            disabled={settingUp2FA}
            error={!!setupError}
            helperText={setupError}
          />
        </DialogContent>
        <DialogActions sx={{ justifyContent: 'center', pb: 3 }}>
          <Button
            onClick={handleCloseSetup2FADialog}
            disabled={settingUp2FA}
            sx={{
              color: 'white',
              border: '1px solid rgba(255, 255, 255, 0.3)',
              '&:hover': {
                backgroundColor: 'rgba(255, 255, 255, 0.1)',
                borderColor: 'rgba(255, 255, 255, 0.5)',
              },
            }}
          >
            Cancel
          </Button>
          <Button
            onClick={handleVerifySetup2FA}
            disabled={settingUp2FA || setupCode.length !== 6}
            variant="contained"
            sx={{
              backgroundColor: 'white',
              color: 'black',
              '&:hover': {
                backgroundColor: 'rgba(255, 255, 255, 0.9)',
              },
              '&:disabled': {
                backgroundColor: 'rgba(255, 255, 255, 0.3)',
                color: 'rgba(255, 255, 255, 0.5)',
              },
            }}
          >
            {settingUp2FA ? <CircularProgress size={20} /> : 'Enable 2FA'}
          </Button>
        </DialogActions>
      </Dialog>
    </>
  );
};

export default ApiKey;