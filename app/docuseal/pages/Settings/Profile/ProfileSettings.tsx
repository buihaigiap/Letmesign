import { useState, useEffect } from 'react';
import { Typography, Box, TextField, Button, IconButton } from '@mui/material';
import Visibility from '@mui/icons-material/Visibility';
import VisibilityOff from '@mui/icons-material/VisibilityOff';
import upstashService from '../../../ConfigApi/upstashService';
import toast from 'react-hot-toast';
import CreateTemplateButton from '@/components/CreateTemplateButton';
import SignatureSection from './SignatureSection';
import { useAuth } from '@/contexts/AuthContext';

const ProfileSettings = () => {
  const { user } = useAuth();
  const [name, setName] = useState('');
  const [email, setEmail] = useState('');
  const [signature, setSignature] = useState('');
  const [initials, setInitials] = useState('');
  const [currentPassword, setCurrentPassword] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [showCurrentPassword, setShowCurrentPassword] = useState(false);
  const [showNewPassword, setShowNewPassword] = useState(false);
  const [showConfirmPassword, setShowConfirmPassword] = useState(false);

  useEffect(() => {
    if (user) {
      setName(user.name || '');
      setEmail(user.email || '');
      setSignature(user.signature || '');
      setInitials(user.initials || '');
    }
  }, [user]);

  const handleUpdateProfile = async (e) => {
    e.preventDefault();
    try {
      await upstashService.updateProfile({ name, email });
      toast.success('Profile updated successfully');
    } catch (err) {
      toast.error('Failed to update profile');
    }
  };

  const handleChangePassword = async (e) => {
    e.preventDefault();
    if (!currentPassword || !newPassword || !confirmPassword) {
      toast.error('Please fill in all fields');
      return;
    }
    if (newPassword !== confirmPassword) {
      toast.error('New passwords do not match');
      return;
    }
    try {
      await upstashService.changePassword({ current_password: currentPassword, new_password: newPassword });
      toast.success('Password changed successfully');
      setCurrentPassword('');
      setNewPassword('');
      setConfirmPassword('');
    } catch (err) {
      toast.error('Failed to change password');
    }
  };

  return (
    <Box sx={{ p: 3 }}>
      <Typography variant="h4" sx={{ mb: 3 }}>
        Profile Settings
      </Typography>
      <Box sx={{ display: 'flex', flexDirection: { xs: 'column', md: 'row' }, gap: 3 }}>
        <div className="bg-white/5 border border-white/10 rounded-lg p-4 flex-1">
          <Typography variant="h6" sx={{ mb: 2 }}>
            Update Profile
          </Typography>
          <Box component="form" onSubmit={handleUpdateProfile}>
            <TextField
              fullWidth
              label="Name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              sx={{ mb: 2 }}
            />
            <TextField
              fullWidth
              label="Email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              sx={{ mb: 2 }}
            />
            <CreateTemplateButton 
                text ='Update Profile'
            />
          </Box>
        </div>
        <div className="bg-white/5 border border-white/10 rounded-lg p-4 flex-1">
          <Typography variant="h6" sx={{ mb: 2 }}>
            Change Password
          </Typography>
          <Box component="form" onSubmit={handleChangePassword}>
            <TextField
              fullWidth
              type={showCurrentPassword ? 'text' : 'password'}
              label="Current Password"
              value={currentPassword}
              onChange={(e) => setCurrentPassword(e.target.value)}
              sx={{ mb: 2 }}
              InputProps={{
                endAdornment: (
                  <IconButton
                    onClick={() => setShowCurrentPassword(!showCurrentPassword)}
                    edge="end"
                  >
                    {showCurrentPassword ? <VisibilityOff /> : <Visibility />}
                  </IconButton>
                ),
              }}
            />
            <TextField
              fullWidth
              type={showNewPassword ? 'text' : 'password'}
              label="New Password"
              value={newPassword}
              onChange={(e) => setNewPassword(e.target.value)}
              sx={{ mb: 2 }}
              InputProps={{
                endAdornment: (
                  <IconButton
                    onClick={() => setShowNewPassword(!showNewPassword)}
                    edge="end"
                  >
                    {showNewPassword ? <VisibilityOff /> : <Visibility />}
                  </IconButton>
                ),
              }}
            />
            <TextField
              fullWidth
              type={showConfirmPassword ? 'text' : 'password'}
              label="Confirm New Password"
              value={confirmPassword}
              onChange={(e) => setConfirmPassword(e.target.value)}
              sx={{ mb: 2 }}
              InputProps={{
                endAdornment: (
                  <IconButton
                    onClick={() => setShowConfirmPassword(!showConfirmPassword)}
                    edge="end"
                  >
                    {showConfirmPassword ? <VisibilityOff /> : <Visibility />}
                  </IconButton>
                ),
              }}
            />
            <CreateTemplateButton 
               text =' Change Password'
            />
          </Box>
        </div>
      </Box>
      <SignatureSection 
        title="Signature" 
        fieldType="signature" 
        initialValue={signature}
        onUpdate={(value) => setSignature(value)}
        userName={name}
      />
      <SignatureSection 
        title="Initials" 
        fieldType="initials" 
        initialValue={initials}
        onUpdate={(value) => setInitials(value)}
        userName={name}
      />
    </Box>
  );
};

export default ProfileSettings;