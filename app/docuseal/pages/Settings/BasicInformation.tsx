import { useState, useEffect } from 'react';
import {
  Typography,
  Box,
  TextField,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
  Button
} from '@mui/material';
import upstashService from '../../ConfigApi/upstashService';
import toast from 'react-hot-toast';
import { useTranslation } from 'react-i18next';

const TIMEZONES = [
  "International Date Line West","Midway Island","American Samoa","Hawaii","Alaska","Pacific Time (US & Canada)","Tijuana","Mountain Time (US & Canada)","Arizona","Chihuahua","Mazatlan","Central Time (US & Canada)","Saskatchewan","Guadalajara","Mexico City","Monterrey","Central America","Eastern Time (US & Canada)","Indiana (East)","Bogota","Lima","Quito","Atlantic Time (Canada)","Caracas","La Paz","Santiago","Asuncion","Newfoundland","Brasilia","Buenos Aires","Montevideo","Georgetown","Puerto Rico","Greenland","Mid-Atlantic","Azores","Cape Verde Is.","Dublin","Edinburgh","Lisbon",
  "London","Casablanca","Monrovia","UTC","Belgrade","Bratislava","Budapest","Ljubljana","Prague","Sarajevo","Skopje","Warsaw","Zagreb","Brussels","Copenhagen","Madrid","Paris","Amsterdam","Berlin","Bern","Zurich","Rome","Stockholm","Vienna","West Central Africa","Bucharest","Cairo","Helsinki","Kyiv","Riga","Sofia","Tallinn","Vilnius","Athens","Istanbul","Minsk","Jerusalem","Harare","Pretoria","Kaliningrad","Moscow","St. Petersburg",
  "Volgograd","Samara","Kuwait","Riyadh","Nairobi","Baghdad","Tehran","Abu Dhabi","Muscat","Baku","Tbilisi","Yerevan","Kabul","Ekaterinburg","Islamabad","Karachi","Tashkent","Chennai","Kolkata","Mumbai","New Delhi","Kathmandu","Dhaka","Sri Jayawardenepura","Almaty","Astana","Novosibirsk","Rangoon","Bangkok","Hanoi","Jakarta","Krasnoyarsk","Beijing","Chongqing","Hong Kong","Urumqi","Kuala Lumpur","Singapore","Taipei","Perth","Irkutsk","Ulaanbaatar","Seoul","Osaka","Sapporo","Tokyo","Yakutsk","Darwin","Adelaide","Canberra","Melbourne","Sydney","Brisbane","Hobart","Vladivostok","Guam","Port Moresby","Magadan","Srednekolymsk","Solomon Is.","New Caledonia","Fiji","Kamchatka","Marshall Is.","Auckland","Wellington","Nuku'alofa","Tokelau Is.","Chatham Is.","Samoa"
];

const LOCALES = [
  { value: 'en-US', label: 'English (United States)' },
  { value: 'en-GB', label: 'English (United Kingdom)' },
  { value: 'fr-FR', label: 'Français' },
  { value: 'es-ES', label: 'Español' },
  { value: 'pt-PT', label: 'Português' },
  { value: 'de-DE', label: 'Deutsch' },
  { value: 'it-IT', label: 'Italiano' },
  { value: 'nl-NL', label: 'Nederlands' }
];

interface BasicInformationProps {
  initialCompanyName: string;
  initialTimezone: string;
  initialLocale: string;
}

export default function BasicInformation({
  initialCompanyName,
  initialTimezone,
  initialLocale
}: BasicInformationProps) {
  const { t, i18n } = useTranslation();
  const [companyName, setCompanyName] = useState(initialCompanyName);
  const [timezone, setTimezone] = useState(initialTimezone);
  const [locale, setLocale] = useState(initialLocale);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    setCompanyName(initialCompanyName);
  }, [initialCompanyName]);

  useEffect(() => {
    setTimezone(initialTimezone);
  }, [initialTimezone]);

  useEffect(() => {
    setLocale(initialLocale);
  }, [initialLocale]);

  console.log('companyName' , companyName)
  const handleUpdate = async () => {
    // Validation
    if (!companyName.trim()) {
      toast.error('Please enter company name');
      return;
    }
    if (!timezone) {
      toast.error('Please select timezone');
      return;
    }
    if (!locale) {
      toast.error('Please select language');
      return;
    }

    setLoading(true);
    try {
      await upstashService.updateUserSettings({
        company_name: companyName,
        timezone: timezone,
        locale: locale
      });

      // Change language immediately if locale changed
      if (locale !== i18n.language) {
        i18n.changeLanguage(locale);
      }

      toast.success('Basic information updated successfully');
    } catch (error) {
      console.error('Failed to update basic information:', error);
      toast.error('Failed to update basic information');
    } finally {
      setLoading(false);
    }
  };

  const hasChanges = companyName !== initialCompanyName ||
                    timezone !== initialTimezone ||
                    locale !== initialLocale;

  return (
    <div className="bg-white/5 border border-white/10 rounded-lg p-4 mb-4">
      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 2 }}>
        <Typography variant="h6">
          {t('settings.general.basicInfo')}
        </Typography>
        <Button
          onClick={handleUpdate}
          disabled={loading}
          sx={{ minWidth: 100, color : 'white'  , backgroundColor : '#3f51b5'}}
        >
          {loading ? 'Updating...' : 'Update'}
        </Button>
      </Box>

      <TextField
        fullWidth
        label={t('settings.general.companyName')}
        value={companyName}
        onChange={(e) => setCompanyName(e.target.value)}
        sx={{ mb: 2 }}
      />

      <Box sx={{ display: 'flex', gap: 2, mb: 2 }}>
        <FormControl fullWidth>
          <InputLabel>{t('settings.general.timeZone')}</InputLabel>
          <Select
            value={timezone}
            label={t('settings.general.timeZone')}
            onChange={(e) => setTimezone(e.target.value)}
          >
            {TIMEZONES.map((tz) => (
              <MenuItem key={tz} value={tz}>{tz}</MenuItem>
            ))}
          </Select>
        </FormControl>

        <FormControl fullWidth>
          <InputLabel>{t('settings.general.language')}</InputLabel>
          <Select
            value={locale}
            label={t('settings.general.language')}
            onChange={(e) => setLocale(e.target.value)}
          >
            {LOCALES.map((loc) => (
              <MenuItem key={loc.value} value={loc.value}>
                {loc.label}
              </MenuItem>
            ))}
          </Select>
        </FormControl>
      </Box>
    </div>
  );
}