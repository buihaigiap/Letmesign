import {
  Typography,
  Box
} from '@mui/material';
import { useTranslation } from 'react-i18next';
import BasicInformation from './BasicInformation';
import PreferencesSection from './PreferencesSection';
import { useBasicSettings } from '../../hooks/useBasicSettings';

const GeneralSettings = () => {
  const { t } = useTranslation();
  const { globalSettings, loading, refetch } = useBasicSettings();
  console.log('Global Settings:', globalSettings);
  // Extract preferences from global settings
  const preferences = globalSettings ? {
    force2fa: globalSettings.force_2fa_with_authenticator_app || false,
    addSignatureId: globalSettings.add_signature_id_to_the_documents || false,
    requireSigningReason: globalSettings.require_signing_reason || false,
    allowTypedTextSignatures: globalSettings.allow_typed_text_signatures || false,
    allowResubmitCompletedForms: globalSettings.allow_to_resubmit_completed_forms || false,
    allowDeclineDocuments: globalSettings.allow_to_decline_documents || false,
    rememberPrefillSignatures: globalSettings.remember_and_pre_fill_signatures || false,
    requireAuthForDownload: globalSettings.require_authentication_for_file_download_links || false,
    combineCompletedAudit: globalSettings.combine_completed_documents_and_audit_log || false,
    expirableDownloadLinks: globalSettings.expirable_file_download_links || false
  } : {
    force2fa: false,
    addSignatureId: false,
    requireSigningReason: false,
    allowTypedTextSignatures: false,
    allowResubmitCompletedForms: false,
    allowDeclineDocuments: false,
    rememberPrefillSignatures: false,
    requireAuthForDownload: false,
    combineCompletedAudit: false,
    expirableDownloadLinks: false
  };

  return (
    <Box sx={{ p: 3 }}>
      <Typography variant="h4" sx={{ mb: 3 }}>
        {t('settings.general.title')}
      </Typography>

      <BasicInformation
        initialCompanyName={globalSettings?.company_name}
        initialTimezone={globalSettings?.timezone}
        initialLocale={globalSettings?.locale}
      />

      <PreferencesSection
        initialPreferences={preferences}
        onSettingsUpdate={refetch}
      />
    </Box>
  );
};

export default GeneralSettings;
