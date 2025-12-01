import { useState, useEffect } from 'react';
import {Box,CardContent,Typography,FormControl,InputLabel,Select,MenuItem,Button,FormControlLabel,Switch,TextField,
} from '@mui/material';
import {NotificationsActive,Email,Warning,ErrorOutline,
} from '@mui/icons-material';
import upstashService from '../../../ConfigApi/upstashService';
import CreateTemplateButton from '../../../components/CreateTemplateButton';
import toast from 'react-hot-toast';
import { REMINDER_DURATIONS } from '../../../constants/reminderDurations';

export default function ReminderSettingsPage() {
  const [settings, setSettings] = useState<any>({
    first_reminder_hours: null,
    second_reminder_hours: null,
    third_reminder_hours: null,
    receive_notification_on_completion: false,
    completion_notification_email: '',
  });
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    fetchSettings();
  }, []);

  const fetchSettings = async () => {
    try {
      const res = await upstashService.getReminderSettings();
      if (res.success) setSettings(res.data);
    } finally {
      setLoading(false);
    }
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      await upstashService.updateReminderSettings({
        first_reminder_hours: settings.first_reminder_hours,
        second_reminder_hours: settings.second_reminder_hours,
        third_reminder_hours: settings.third_reminder_hours,
        receive_notification_on_completion: settings.receive_notification_on_completion,
        completion_notification_email: settings.completion_notification_email || null,
      });
      toast.success('Reminder settings saved successfully');
    } finally {
      setSaving(false);
    }
  };

  const handleSelectChange =
    (field: any) =>
    (event: any) => {
      setSettings({
        ...settings,
        [field]: event.target.value === '0' ? null : Number(event.target.value),
      });
    };

  const handleTextChange = (field: string) => (event: any) => {
    setSettings({
      ...settings,
      [field]: event.target.value,
    });
  };

  const handleSwitchChange = (field: string) => (event: any) => {
    setSettings({
      ...settings,
      [field]: event.target.checked,
    });
  };

  const reminderConfigs = [
    {
      key: 'first_reminder_hours',
      label: '📬 First Reminder',
      description: 'Send the first reminder email after this amount of time',
      previewText: 'Polite reminder email',
    },
    {
      key: 'second_reminder_hours',
      label: '⚠️ Second Reminder',
      description: 'Send a warning email after this amount of time',
      previewText: 'Warning email',
    },
    {
      key: 'third_reminder_hours',
      label: '🚨 Third Reminder',
      description: 'Send the third urgent email after this amount of time',
      previewText: 'Urgent email',
    },
  ] as const;

  return (
    <Box>
      <CardContent>
        {/* Header */}
        <Box display="flex" alignItems="center" mb={3}>
          <NotificationsActive sx={{ fontSize: 40, mr: 2 }} />
          <Box>
            <Typography variant="h4" fontWeight="bold" gutterBottom>
              Email Reminder Configuration
            </Typography>
            <Typography variant="body2" color="text.secondary">
              Automatically send reminder emails to signers who haven’t completed signing the document
            </Typography>
          </Box>
        </Box>

        {/* Reminder Selects */}
        <Box display="flex" gap={4} flexWrap="wrap" justifyContent="space-between">
          {reminderConfigs.map(({ key, label, description }) => (
            <FormControl key={key} sx={{ minWidth: 220, flex: 1 }}>
              <InputLabel id={`${key}-label`}>{label}</InputLabel>
              <Select
                labelId={`${key}-label`}
                value={settings[key] || 0}
                onChange={handleSelectChange(key)}
                label={label}
                sx={{ bgcolor: 'transparent' }}
              >
                <MenuItem value={0}>
                  <em>Select duration...</em>
                </MenuItem>
                {REMINDER_DURATIONS.map((d) => (
                  <MenuItem key={d.hours} value={d.hours}>
                    {d.label}
                  </MenuItem>
                ))}
              </Select>
              <Typography variant="caption" color="text.secondary" sx={{ mt: 1, ml: 1 }}>
                {description}
              </Typography>
            </FormControl>
          ))}
        </Box>

        {/* Notification on Completion */}
        <Box mt={3}>
          <FormControlLabel
            control={
              <Switch
                checked={settings.receive_notification_on_completion}
                onChange={handleSwitchChange('receive_notification_on_completion')}
                color="primary"
              />
            }
            label="Enable signature progress notifications"
            sx={{ mb: 2 }}
          />
          <TextField
            fullWidth
            label="Completion Notification Email"
            value={settings.completion_notification_email}
            onChange={handleTextChange('completion_notification_email')}
            placeholder="Enter email address for completion notifications"
            type="email"
            // disabled={!settings.receive_notification_on_completion}
            sx={{ mb: 1 }}
          />
          <Typography variant="caption" color="text.secondary">
            Enter an email address to receive notifications each time a signer completes their signature
          </Typography>
        </Box>

        {/* Actions */}
        <Box display="flex" justifyContent="flex-end" gap={2} mt={4} pt={3} borderTop={1} borderColor="divider">
           {/* <Button
                onClick={fetchSettings}
                disabled={saving}
                variant="outlined"
                color="inherit"
                sx={{
                  borderColor: "#475569",
                  color: "#cbd5e1",
                  textTransform: "none",
                  fontWeight: 500,
                  "&:hover": { backgroundColor: "#334155" },
            }}
            >
                Cancel
            </Button> */}
          <CreateTemplateButton
            onClick={handleSave}
            disabled={saving }
            loading={saving}
            text= {saving ? 'Saving...' : 'Save Configuration'}
          />
        </Box>
      </CardContent>
    </Box>
  );
}
