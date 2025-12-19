import { Button, Typography } from '@mui/material';
import toast from 'react-hot-toast';
import upstashService from '../../ConfigApi/upstashService';
import CreateTemplateButtonProps from '../../components/CreateTemplateButton';
import { downloadSignedPDF, fetchAuditLog, generateMockAuditLog } from '../../services/pdfDownloadService';
import { useBasicSettings } from '../../hooks/useBasicSettings';
import { useEffect, useState } from 'react';

interface CompletionScreenProps {
  signedDate: string;
  templateName?: string;
  token: string;
  allowResubmit: boolean;
}

const CompletionScreen: React.FC<CompletionScreenProps> = ({
  signedDate,
  templateName,
  token,
  allowResubmit
}) => {
  const { globalSettings } = useBasicSettings();
  const [canDownload, setCanDownload] = useState<boolean>(true);
  const [isDownloading, setIsDownloading] = useState<boolean>(false);
  const [isSendingEmail, setIsSendingEmail] = useState<boolean>(false);
  useEffect(() => {
    // Fetch submitter info to get can_download status
    const fetchSubmitterInfo = async () => {
      try {
        const result = await upstashService.getSubmitterInfo(token);
        if (result.success && result.data) {
          // If can_download is undefined, default to true (backward compatibility)
          setCanDownload(result.data.can_download !== false);
        }
      } catch (error) {
        console.error('Error fetching submitter info:', error);
        // On error, default to true to avoid breaking existing functionality
        setCanDownload(true);
      }
    };

    fetchSubmitterInfo();
  }, [token]);
  const handleSendEmail = async () => {
    try {
      setIsSendingEmail(true);
      await upstashService.sendCopyEmail(token);
      toast.success('Email sent successfully');
    } catch (error) {
      toast.error('Failed to send email');
    }finally {
      setIsSendingEmail(false);
    }
  };

  const handleDownload = async () => {
    try {
      setIsDownloading(true);
      // Fetch submitter info, signatures and fields data
      const [submitterResult, signaturesResult, fieldsResult] = await Promise.all([
        upstashService.getSubmitterInfo(token),
        upstashService.getSubmissionSignatures(token),
        upstashService.getSubmissionFields(token)
      ]);

      if (!submitterResult.success || !signaturesResult.success || !fieldsResult.success) {
        throw new Error('Failed to fetch submission data');
      }

      const data = {
        submitter: submitterResult.data,
        ...signaturesResult.data
      };

      let submitterInfo = null;
      if (fieldsResult.data.information) {
        submitterInfo = {
          id: fieldsResult.data.information.id,
          email: fieldsResult.data.information.email
        };
      }

      // Fetch real audit log from backend, fallback to mock if failed
      let auditLog = await fetchAuditLog(token);
      if (!auditLog || auditLog.length === 0) {
        auditLog = generateMockAuditLog(
          submitterInfo?.email || 'Unknown User',
          templateName || 'signed_document'
        );
      }

      // Call downloadSignedPDF with audit log
      await downloadSignedPDF(
        data.template_info.document.url,
        data.bulk_signatures,
        templateName || 'signed_document',
        submitterInfo,
        globalSettings,
        auditLog
      );

      toast.success('Download started');
    } catch (error: any) {
      console.error('Download error:', error);
      if (error.response?.status === 401 || error.status === 401) {
        // Authentication required, redirect to login with return URL
        const currentUrl = window.location.href;
        console.log('CompletionScreen 401 - current URL:', currentUrl);
        const loginUrl = `/login?redirect=${encodeURIComponent(currentUrl)}`;
        console.log('CompletionScreen 401 - redirecting to:', loginUrl);
        window.location.href = window.location.origin + loginUrl;
        return;
      }
      toast.error('Failed to download');
    } finally {
      setIsDownloading(false);
    }
  };

  const handleResubmit = async () => {
    try {
      await upstashService.resubmitSubmission(token);
      toast.success('Form reset successfully. You can now resubmit.');
      window.location.reload();
    } catch (error) {
      toast.error('Failed to reset form');
    }
  };

  return (
    <div className="flex items-center justify-center  ">
      <div className="max-w-md w-full  rounded-lg shadow-lg p-8">
        <div className=" mb-6">
          <div className="h-[200px]">
            <img src='/logo.png' alt="Logo" />
          </div>
          <Typography variant="body2" color="textSecondary" sx={{ mb: 1 }}>
            Template Name: {templateName}
          </Typography>
          <Typography variant="body2" color="textSecondary">
            Signed on {signedDate}
          </Typography>
        </div>

        <div className="space-y-3">
          <Button
            loading ={isSendingEmail}
            disabled={isSendingEmail}
            variant="contained"
            fullWidth
            sx={{
              textTransform: 'none',
              backgroundColor: '#4f46e5',
              '&:hover': { backgroundColor: '#4338ca' }
            }}
            onClick={handleSendEmail}
          >
            SEND COPY TO EMAIL
          </Button>

          {canDownload && (
            <Button
              disabled={isDownloading}
              loading={isDownloading}
              variant="outlined"
              fullWidth
              sx={{
                textTransform: 'none',
                borderColor: '#4f46e5',
                color: 'white',
              }}
              onClick={handleDownload}
            >
              DOWNLOAD DOCUMENTS
            </Button>
          )}

          {!canDownload && (
            <Typography
              variant="body2"
              sx={{
                color: '#9ca3af',
                textAlign: 'center',
                fontStyle: 'italic',
                py: 1
              }}
            >
              Download link has expired (40 minutes after signing)
            </Typography>
          )}

          {allowResubmit && (
            <CreateTemplateButtonProps
              text="RESUBMIT FORM"
              onClick={handleResubmit}
              width="100%"
            />
          )}
        </div>
      </div>
    </div>
  );
};

export default CompletionScreen;
