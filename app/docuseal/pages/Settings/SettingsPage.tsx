import { Routes, Route, Navigate, Link, useLocation } from 'react-router-dom';
import { Box, List, ListItem, ListItemButton, ListItemText, Typography, Paper } from '@mui/material';
import { useTranslation } from 'react-i18next';
import ProfileSettings from './Profile/ProfileSettings';
import UsersSettings from './Activate/UsersSettings';
import TeamSettings from './Activate/TeamSettings';
import ReminderSettingsPage from './ReminderSettings/ReminderSettingsPage';
import GeneralSettings from './GeneralSettings';
import PersonalizationPage from './Personalization';
import PDFSignaturePage from './PDFSignature/PDFSignaturePage';

const SettingsPage = () => {
  const { t } = useTranslation();
  const location = useLocation();

  const menuItems = [
    { text: t('navigation.profile'), path: '/settings/profile' },
    { text: t('navigation.users'), path: '/settings/users' },
    { text: 'Team Accounts', path: '/settings/team' },
    { text: t('navigation.reminders'), path: '/settings/reminders' },
    { text: 'Personalization', path: '/settings/personalization' },
    { text: 'E-Sign Settings', path: '/settings/pdf-signature' },
    { text: t('navigation.general'), path: '/settings/general' },
  ];

  return (
    <Box sx={{ display: 'flex', minHeight: 'calc(100vh - 4rem)', color: 'white', gap: 2  , p: 2}}>
      <Box sx={{ width: 250}}>
        <Typography variant="h6" sx={{ mb: 2 }}>
          {t('settings.title')}
        </Typography>
        <List sx={{ '& .MuiListItem-root': { mb: 1 } }}>
          {menuItems.map((item) => (
            <ListItem key={item.path} disablePadding>
              <ListItemButton component={Link} to={item.path} sx={{ color: 'white', bgcolor: location.pathname === item.path ? 'rgba(79, 70, 229, 0.5)' : 'transparent', '&:hover': { bgcolor: location.pathname === item.path ? 'rgba(79, 70, 229, 0.5)' : 'rgba(79, 70, 229, 0.3)' }, borderRadius: 1 }}>
                <ListItemText primary={item.text} />
              </ListItemButton>
            </ListItem>
          ))}
        </List>
      </Box>
      <Box sx={{ flex: 1 }}>
        <Paper 
          sx={{ p: 3, bgcolor: 'rgba(13, 7, 31, 0.9)', color: 'white', borderRadius: 2, backdropFilter: 'blur(10px)', border: '1px solid rgba(255, 255, 255, 0.1)' }}>
          <Routes>
        
            <Route path="profile" element={<ProfileSettings />} />
            <Route path="users" element={<UsersSettings />} />
            <Route path="team" element={<TeamSettings />} />
            <Route path="reminders" element={<ReminderSettingsPage />} />
            <Route path="personalization" element={<PersonalizationPage />} />
            <Route path="pdf-signature" element={<PDFSignaturePage />} />
            <Route path="general" element={<GeneralSettings />} />
            <Route index element={<Navigate to="profile" replace />} />
          </Routes>
        </Paper>
      </Box>
    </Box>
  );
};

export default SettingsPage;