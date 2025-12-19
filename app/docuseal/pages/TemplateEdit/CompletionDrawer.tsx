import React from 'react';
import {Drawer,Box,Typography,Button , Link} from '@mui/material';
import { Download as DownloadIcon, Email as EmailIcon } from '@mui/icons-material';
import CreateTemplateButton from '@/components/CreateTemplateButton';
import { downloadSignedPDF } from '../../services/pdfDownloadService';
import toast from 'react-hot-toast';
import { BadgeCheck } from 'lucide-react';

import upstashService from '../../ConfigApi/upstashService';
import confetti from 'canvas-confetti';

interface CompletionDrawerProps {
  open: boolean;
  pdfUrl: string;
  signatures: any[];
  templateName: string;
  submitterInfo?: any;
  token?: string;
}

const CompletionDrawer: React.FC<CompletionDrawerProps> = ({
  open,
  pdfUrl,
  signatures,
  templateName,
  submitterInfo,
  token
}) => {
  const [isDownloading, setIsDownloading] = React.useState(false);
  const [isSendingEmail, setIsSendingEmail] = React.useState(false);
  React.useEffect(() => {
    if (submitterInfo?.global_settings?.enable_confetti && open) {
      confetti({
        particleCount: 100,
        spread: 70,
        origin: { y: 0.6 }
      });
    }
  }, [open]);

  const handleDownload = async () => {
    try {
      setIsDownloading(true);
      console.log('Signatures for download:', signatures.map(sig => ({
        field_type: sig.field_info?.field_type,
        signature_value: sig.signature_value
      })));
      await downloadSignedPDF(
        pdfUrl,
        signatures,
        templateName,
        submitterInfo,
        submitterInfo?.global_settings
      );
      toast.success('Document downloaded successfully');
    } catch (error) {
      console.error('Download error:', error);
      toast.error('Failed to download document');
    } finally {
      setIsDownloading(false);
    }
  };

  const handleSendEmail = async () => {
    if (!token) {
      toast.error('Token is missing');
      return;
    }
    try {
      setIsSendingEmail(true);
      await upstashService.sendCopyEmail(token);
      toast.success('Email sent successfully');
    } catch (error) {
      console.error('Email send error:', error);
      toast.error('Failed to send email');
    } finally {
      setIsSendingEmail(false);
    }
  };

  return (
    <Drawer
      anchor="bottom"
      open={open}
      ModalProps={{
        disableEscapeKeyDown: true,
      }}
      sx={{
        '& .MuiDrawer-paper': {
          width: '100%',
          maxWidth: 800,
          position: 'absolute',
          left: '35%',
          transform: 'translate(-50%, -50%)',
          py: 3,
          px: 10,
          display: 'flex',
          flexDirection: 'column'
        }
      }}
    >
      {/* Header */}
      <Box sx={{ display: 'flex', justifyContent: 'center', mb: 2 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          <BadgeCheck color="green" />
          <Typography variant="h6" component="h2" sx={{ fontWeight: 600 }}>
            {submitterInfo?.global_settings?.completion_title || 'Signed successfully'}
          </Typography>
        </Box>
      </Box>

      {/* Body */}
      {submitterInfo?.global_settings?.completion_body !== "" && (
        <Box>
          <Typography variant="body1" sx={{ color: 'white', lineHeight: 1.6 }}>
            {submitterInfo?.global_settings?.completion_body}
          </Typography>
        </Box>
      )}

      {/* Buttons */}
      <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>

        {/* Download Button */}
        <Button
          variant="contained"
          startIcon={!isDownloading ? <DownloadIcon /> : null}
          onClick={handleDownload}
          fullWidth
          disabled={isDownloading}
          loading={isDownloading}
        >
          Download Document
        </Button>

        {/* Send Email Button */}
        <Button
          sx={{ color: 'white' }}
          variant="outlined"
          startIcon={!isSendingEmail ? <EmailIcon /> : null}
          onClick={handleSendEmail}
          fullWidth
          disabled={isSendingEmail}
          loading={isSendingEmail}
        >
          Send Copy Via Email
        </Button>

        {/* Redirect Button */}
        {submitterInfo?.global_settings?.redirect_title !== "" && (
          <CreateTemplateButton
            text={submitterInfo?.global_settings?.redirect_title}
            onClick={() => {
              window.open(submitterInfo?.global_settings?.redirect_url, "_blank");
            }}
          />
        )}

     <Typography variant="body1" sx={{ color: "white", lineHeight: 1.6 }}>
        Powered by{" "}
        <Link
          href="/"
          target="_blank"
          color="inherit"
        >
          LetMesign
        </Link>{" "}
        - open source documents software
      </Typography>

      </Box>
    </Drawer>
  );
};

export default CompletionDrawer;
