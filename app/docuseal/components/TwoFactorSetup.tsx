import React, { useState, useEffect } from 'react';
import {Box,Typography,TextField,Button,Alert,CircularProgress,Grid,Card,CardContent} from '@mui/material';
import { motion } from 'framer-motion';
import toast from 'react-hot-toast';
import upstashService from '../ConfigApi/upstashService';
import { TwoFactorSetup as TwoFactorSetupType } from '../types';

interface TwoFactorSetupProps {
  onSuccess: () => void;
}

const TwoFactorSetup: React.FC<TwoFactorSetupProps> = ({ onSuccess }) => {
  const [loading, setLoading] = useState(false);
  const [setupData, setSetupData] = useState<TwoFactorSetupType | null>(null);
  const [verificationCode, setVerificationCode] = useState('');

  useEffect(() => {
    fetchSetupData();
  }, []);

  const fetchSetupData = async () => {
    setLoading(true);
    try {
      const response = await upstashService.setup2FA();
      if (response.success) {
        setSetupData(response.data);
      } 
    } catch (err) {
      toast.error(err?.error || 'An error occurred while setting up 2FA');
    } finally {
      setLoading(false);
    }
  };

  const handleVerify = async () => {
    if (!setupData || !verificationCode.trim()) {
      toast.error('Please enter the verification code');
      return;
    }
    setLoading(true);
    try {
      const response = await upstashService.verify2FA({
        secret: setupData.secret,
        code: verificationCode.trim(),
      });

      if (response.success) {
        toast.success('2FA setup completed successfully!');
        onSuccess();
      }
    } catch (err) {
      toast.error(err?.error || 'An error occurred during verification');
    } finally {
      setLoading(false);
    }
  };

  const handleSkip = () => {
    // For now, just call onSuccess - in a real app you might want to handle this differently
    onSuccess();
  };

  if (loading && !setupData) {
    return (
      <Box
        sx={{
          display: 'flex',
          justifyContent: 'center',
          alignItems: 'center',
          minHeight: '400px',
        }}
      >
        <CircularProgress size={60} />
      </Box>
    );
  }

  return (
    <Box
      sx={{
        maxWidth: 800,
        mx: 'auto',
        p: 3,
      }}
    >
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.6 }}
      >
        

          {setupData && (
            <Grid container spacing={4}>
              {/* QR Code Section */}
              <Grid item xs={12} md={6}>
                <Card sx={{ backgroundColor: 'rgba(255, 255, 255, 0.1)', color: 'white' }}>
                  <CardContent>
                    <Typography variant="body2" sx={{ mb: 2, opacity: 0.8 }}>
                      Open your authenticator app (Google Authenticator, Authy, etc.) and scan this QR code:
                    </Typography>
                    <Box
                      sx={{
                        display: 'flex',
                        justifyContent: 'center',
                        mb: 2,
                      }}
                    >
                      <img
                        src={`data:image/png;base64,${setupData.qr_code_url}`}
                        alt="2FA QR Code"
                        style={{
                          maxWidth: '200px',
                          maxHeight: '200px',
                          border: '2px solid white',
                          borderRadius: '8px',
                        }}
                      />
                    </Box>
                     <TextField
                      fullWidth
                      variant="outlined"
                      placeholder="000000"
                      value={verificationCode}
                      onChange={(e) => setVerificationCode(e.target.value.replace(/\D/g, '').slice(0, 6))}
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
                      inputProps={{
                        maxLength: 6,
                        style: { textAlign: 'center' },
                      }}
                    />
                     <Button
                      fullWidth
                      variant="contained"
                      onClick={handleVerify}
                      disabled={loading || verificationCode.length !== 6}
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
                      {loading ? <CircularProgress size={24} /> : 'Verify & Enable 2FA'}
                    </Button>
                    {/* <Typography variant="body2" sx={{ fontSize: '0.75rem', opacity: 0.7 }}>
                      If you can't scan the QR code, manually enter this secret:
                    </Typography>
                    <Typography
                      variant="body2"
                      sx={{
                        fontFamily: 'monospace',
                        fontSize: '0.75rem',
                        backgroundColor: 'rgba(255, 255, 255, 0.2)',
                        p: 1,
                        borderRadius: 1,
                        mt: 1,
                        wordBreak: 'break-all',
                      }}
                    >
                      {setupData.secret}
                    </Typography> */}
                  </CardContent>
                </Card>
              </Grid>
            </Grid>
          )}
      </motion.div>
    </Box>
  );
};

export default TwoFactorSetup;