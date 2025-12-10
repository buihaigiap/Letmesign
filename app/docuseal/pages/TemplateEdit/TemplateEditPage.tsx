import { useState, useEffect, useCallback } from 'react';
import { useParams } from 'react-router-dom';
import upstashService from '../../ConfigApi/upstashService';
import CreateTemplateButton from '../../components/CreateTemplateButton';
import PdfFullView from './PdfFullView';
import CompletionScreen from './CompletionScreen';
import FormModal from './FormModal';
import {
  Dialog, DialogContent, DialogActions, Button,
  Typography, TextField
} from '@mui/material';
import toast from 'react-hot-toast';
import { useAuth } from '../../contexts/AuthContext';
import CompletionDrawer from './CompletionDrawer';
import { useFileUpload } from '../../hooks/useFileUpload';
interface TemplateField {
  id: number;
  template_id: number;
  name: string;
  field_type: string;
  required: boolean;
  display_order: number;
  position: {
    x: number;
    y: number;
    width: number;
    height: number;
    page: number;
  };
  options?: any;
  partner?: string;
  created_at: string;
  updated_at: string;
}

interface TemplateInfo {
  id: number;
  name: string;
  slug: string;
  user_id: number;
  document: {
    filename: string;
    content_type: string;
    size: number;
    url: string;
  };
}



const TemplateEditPage = () => {
  const { token } = useParams<{ token: string }>();
  const [templateInfo, setTemplateInfo] = useState<TemplateInfo | null>(null);
  const [fields, setFields] = useState<TemplateField[]>([]);
  const [texts, setTexts] = useState<Record<number, string>>({});
  const [currentFieldIndex, setCurrentFieldIndex] = useState(0);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [page, setPage] = useState(1);
  const [submitterInfo, setSubmitterInfo] = useState<{
    id: number;
    email: string;
    template_name?: string;
    status: string;
    signed_at?: string;
    global_settings?: any;
  } | null>(null);
  const [pendingUploads, setPendingUploads] = useState<Record<number, File>>({});
  const [selectedReason, setSelectedReason] = useState<string>('');
  const [customReason, setCustomReason] = useState<string>('');
  const [reasons, setReasons] = useState<Record<number, string>>({});
  const [declineModalOpen, setDeclineModalOpen] = useState(false);
  const [declineReason, setDeclineReason] = useState<string>('');
  const [clearedFields, setClearedFields] = useState<Set<number>>(new Set());
  const { user } = useAuth();
  const [completionDrawerOpen, setCompletionDrawerOpen] = useState(false);
  const [completing, setCompleting] = useState(false);

  // Use custom hook for file upload/delete
  const { uploading: fileUploading, progress, uploadFile, deleteFile } = useFileUpload();

  const fetchTemplateFields = useCallback(async () => {
    try {
      const data = await upstashService.getSubmissionFields(token);
      if (data.success) {
        setTemplateInfo(data.data.template_info);

        // Extract submitter information if available
        if (data.data.information) {
          // Fetch full submitter info to get status
          const submitterData = await upstashService.getSubmitterInfo(token);
          console.log('submitterData', submitterData);
          if (submitterData.success) {
            setSubmitterInfo({
              id: data.data.information.id,
              email: data.data.information.email,
              template_name: submitterData.data.template_name,
              status: submitterData.data.status,
              signed_at: submitterData.data.signed_at, 
              global_settings: submitterData.data?.global_settings
            });
          } else {
            setSubmitterInfo({
              id: data.data.information.id,
              email: data.data.information.email,
              status: 'pending',
              global_settings: submitterData.data?.global_settings
            });
          }
        }

        // Convert position from pixels to decimal (0-1) if needed
        const processedFields = data.data.template_fields.map((field: TemplateField) => {
          if (field.position && typeof field.position.x === 'number') {
            // Use default page dimensions since we don't have actual page dimensions here
            const pageWidth = 600; // Default A4 width in pixels
            const pageHeight = 800; // Default A4 height in pixels

            // Check if position is in pixels (values > 1) or already in decimal (0-1)
            if (field.position.x > 1 || field.position.y > 1 || field.position.width > 1 || field.position.height > 1) {
              // Position is in pixels, convert to decimal (0-1)
              return {
                ...field,
                position: {
                  ...field.position,
                  x: field.position.x / pageWidth,
                  y: field.position.y / pageHeight,
                  width: field.position.width / pageWidth,
                  height: field.position.height / pageHeight
                }
              };
            }
            // Already in decimal format
            return field;
          }
          return field;
        });

        setFields(processedFields);
      }
    } catch (err) {
      console.error('Fetch error:', err);
    }
  }, [token]);

  useEffect(() => {
    fetchTemplateFields();
  }, [fetchTemplateFields]);

  // Initialize texts with default signatures/initials when fields and user are available
  useEffect(() => {
    if (fields.length > 0 && user && submitterInfo?.global_settings?.remember_and_pre_fill_signatures) {
      const initialTexts: Record<number, string> = {};
      fields.forEach(field => {
        if ((field.field_type === 'signature' || field.field_type === 'initials') && !texts[field.id]) {
          const defaultValue = field.field_type === 'signature' ? user.signature : user.initials;
          if (defaultValue) {
            initialTexts[field.id] = defaultValue;
          }
        }
      });
      if (Object.keys(initialTexts).length > 0) {
        setTexts(prev => ({ ...prev, ...initialTexts }));
      }
    }
  }, [fields, user, submitterInfo?.global_settings?.remember_and_pre_fill_signatures]);

  // Update reasons state when selected reason changes
  useEffect(() => {
    if (submitterInfo?.global_settings?.require_signing_reason) {
      const reason = selectedReason === 'Other' ? customReason : selectedReason;
      const newReasons: Record<number, string> = {};
      fields.forEach(field => {
        if (field.field_type === 'signature' || field.field_type === 'initials') {
          newReasons[field.id] = reason;
        }
      });
      setReasons(newReasons);
    }
  }, [selectedReason, customReason, fields, submitterInfo?.global_settings?.require_signing_reason]);

  const onFieldClick = (field: TemplateField) => {
    const globalIndex = fields.findIndex(f => f.id === field.id);
    setCurrentFieldIndex(globalIndex);
    setPage(field.position.page);
    setIsModalOpen(true);
  };

  const handleTextChange = (fieldId: number, value: string, isMultiple: boolean = false, checked?: boolean) => {
    if (isMultiple && checked !== undefined) {
      // Handle multiple selection
      const currentSelections = texts[fieldId] ? texts[fieldId].split(',') : [];
      let newSelections;
      if (checked) {
        newSelections = [...currentSelections, value];
      } else {
        newSelections = currentSelections.filter(item => item !== value);
      }
      setTexts(prev => ({ ...prev, [fieldId]: newSelections.join(',') }));
    } else {
      setTexts(prev => ({ ...prev, [fieldId]: value }));
      // If setting to empty string, mark as cleared
      if (value === '') {
        setClearedFields(prev => new Set([...prev, fieldId]));
      } else {
        // If setting a value, remove from cleared fields
        setClearedFields(prev => {
          const newSet = new Set(prev);
          newSet.delete(fieldId);
          return newSet;
        });
      }
    }
  };

  const handleNext = () => {
    // Validate current field before moving to next
    const currentValue = texts[currentField?.id];
    if (currentField?.required && !currentValue) {
      toast.error(`Please fill in the required field: ${currentField.name}`);
      return;
    }

    if (currentFieldIndex < fields.length - 1) {
      const nextIndex = currentFieldIndex + 1;
      setCurrentFieldIndex(nextIndex);
      const nextField = fields[nextIndex];
      if (nextField.position.page !== page) {
        setPage(nextField.position.page);
      }
    }
  };

  const handlePrevious = () => {
    if (currentFieldIndex > 0) {
      const prevIndex = currentFieldIndex - 1;
      setCurrentFieldIndex(prevIndex);
      const prevField = fields[prevIndex];
      if (prevField.position.page !== page) {
        setPage(prevField.position.page);
      }
    }
  };

  const handleComplete = async () => {
    setCompleting(true);
    // Upload any pending files first
    const finalTexts = { ...texts };
    for (const [fieldId, file] of Object.entries(pendingUploads)) {
      try {
        const fileUrl = await uploadFile(file);
        if (fileUrl) {
          finalTexts[parseInt(fieldId)] = fileUrl;
          // Cleanup blob URL after successful upload
          const blobUrl = texts[parseInt(fieldId)];
          if (blobUrl && blobUrl.startsWith('blob:')) {
            URL.revokeObjectURL(blobUrl);
          }
        } else {
          console.error(`Upload failed for field ${fieldId}`);
          toast.error(`Failed to upload file for field ${fieldId}`);
          setCompleting(false);
          return;
        }
      } catch (error) {
        console.error(`Upload error for field ${fieldId}:`, error);
        toast.error(`Upload error for field ${fieldId}`);
        setCompleting(false);
        return;
      }
    }

    // Validate required fields
    const missingFields = fields.filter(field => {
      if (!field.required) return false;
      const value = finalTexts[field.id];
      if (!value) return true;
      // For signature fields, check if it's not empty
      if (field.field_type === 'signature') {
        return !value || value.trim() === '';
      }
      // For radio fields, check if an option is selected
      if (field.field_type === 'radio') {
        return !value.trim();
      }
      // For multiple fields, check if at least one option is selected
      if (field.field_type === 'multiple') {
        return !value || value.split(',').filter(item => item.trim()).length === 0;
      }
      // For checkbox fields, 'false' is a valid value
      if (field.field_type === 'checkbox') {
        return false;
      }
      // For other fields, check if trimmed value exists
      return !value.trim();
    });
    if (missingFields.length > 0) {
      toast.error(`Please fill in the required fields: ${missingFields.map(f => f.name).join(', ')}`);
      setCompleting(false);
      return;
    }


    try {
      const reason = selectedReason === 'Other' ? customReason : selectedReason;
      const signatures = fields.map(field => ({
        field_id: field.id,
        signature_value: finalTexts[field.id] || '',
        reason: submitterInfo?.global_settings?.require_signing_reason ? reason : undefined
      }));

      // Generate or retrieve session ID
      let sessionId = sessionStorage.getItem('letmesign_session_id');
      if (!sessionId) {
        sessionId = `session_${Date.now()}_${Math.random().toString(36).substring(2, 15)}`;
        sessionStorage.setItem('letmesign_session_id', sessionId);
      }

      // Get user's timezone
      const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone;

      const data = await upstashService.bulkSign(token, {
        signatures,
        user_agent: navigator.userAgent,
        timezone: timezone
      });
      if (data.success) {
        toast.success(data?.message);
        setPendingUploads({});
        // Open completion drawer
        setDeclineModalOpen(false);
        setCompletionDrawerOpen(true);
        // Close decline modal if open
        setIsModalOpen(false);
      } else {
        toast.error(`Error: ${data.message}`);
      }
    } catch (err) {
      console.error('Submit error:', err);
      toast.error('Unable to submit signature. Please try again.');
    } finally {
      setCompleting(false);
    }
  };

  const handleDecline = () => {
    setDeclineModalOpen(true);
  };

  const handleDeclineConfirm = async () => {
    if (!declineReason.trim()) {
      toast.error('Please provide a reason for declining');
      return;
    }

    try {
      // Upload any pending files first
      const finalTexts = { ...texts };
      for (const [fieldId, file] of Object.entries(pendingUploads)) {
        try {
          const fileUrl = await uploadFile(file);
          if (fileUrl) {
            finalTexts[parseInt(fieldId)] = fileUrl;
            // Cleanup blob URL after successful upload
            const blobUrl = texts[parseInt(fieldId)];
            if (blobUrl && blobUrl.startsWith('blob:')) {
              URL.revokeObjectURL(blobUrl);
            }
          } else {
            console.error(`Upload failed for field ${fieldId}`);
            toast.error(`Failed to upload file for field ${fieldId}`);
            return;
          }
        } catch (error) {
          console.error(`Upload error for field ${fieldId}:`, error);
          toast.error(`Upload error for field ${fieldId}`);
          return;
        }
      }

      const reason = selectedReason === 'Other' ? customReason : selectedReason;
      const signatures = fields.map(field => ({
        field_id: field.id,
        signature_value: finalTexts[field.id] || '', // Preserve existing values
        reason: submitterInfo?.global_settings?.require_signing_reason && (field.field_type === 'signature' || field.field_type === 'initials') ? reason : undefined
      }));

      // Generate or retrieve session ID
      let sessionId = sessionStorage.getItem('letmesign_session_id');
      if (!sessionId) {
        sessionId = `session_${Date.now()}_${Math.random().toString(36).substring(2, 15)}`;
        sessionStorage.setItem('letmesign_session_id', sessionId);
      }

      // Get user's timezone
      const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone;

      const data = await upstashService.bulkSign(token, {
        signatures,
        action: 'decline',
        decline_reason: declineReason.trim(),
        user_agent: navigator.userAgent,
        timezone: timezone
      });

      if (data.success) {
        toast.success('Document declined successfully');
        // navigate(`/templates/${templateInfo?.id}`);
        setCompletionDrawerOpen(true);
        setIsModalOpen(false);
        // Clear pending uploads after successful submission
        setPendingUploads({});
      } else {
        toast.error(`Error: ${data.message}`);
      }
    } catch (err) {
      console.error('Decline error:', err);
      toast.error('Unable to decline document. Please try again.');
    } finally {
      setDeclineModalOpen(false);
      setDeclineReason('');
    }
  };

  const currentField = fields[currentFieldIndex];
  const isLastField = currentFieldIndex === fields.length - 1;

  // Check if submission is already completed
  const isCompleted = submitterInfo?.status === 'signed' || submitterInfo?.status === 'completed';
  // If completed, show completion screen
  if (isCompleted) {
    const signedDate = submitterInfo?.signed_at
      ? new Date(submitterInfo.signed_at).toLocaleDateString('en-US', {
        year: 'numeric',
        month: 'long',
        day: 'numeric'
      })
      : 'Recently';

    return (
      <CompletionScreen
        signedDate={signedDate}
        templateName={submitterInfo?.template_name}
        token={token}
        allowResubmit={submitterInfo?.global_settings?.allow_to_resubmit_completed_forms || false}
      />
    );
  }

  // Safety check - if no current field, return loading or error
  if (!currentField) {
    return <div className="text-red-500 text-center p-4">No fields available</div>;
  }

  return (
    <div >
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">{templateInfo?.name}</h1>
        {submitterInfo?.global_settings?.allow_to_decline_documents && (
          <CreateTemplateButton
            text="Decline"
            onClick={handleDecline}
          />
        )}
      </div>


      {/* PDF Full View */}
      <PdfFullView
        templateInfo={templateInfo}
        fields={fields}
        page={page}
        onPageChange={setPage}
        onFieldClick={onFieldClick}
        texts={texts}
        token={token}
        submitterId={submitterInfo?.id}
        submitterEmail={submitterInfo?.email}
        reasons={reasons}
        clearedFields={clearedFields}
        globalSettings={submitterInfo?.global_settings}
      />

      {/* Form Modal */}
      <FormModal
        open={isModalOpen}
        onClose={() => setIsModalOpen(false)}
        currentFieldIndex={currentFieldIndex}
        fields={fields}
        texts={texts}
        onTextChange={handleTextChange}
        onNext={handleNext}
        onPrevious={handlePrevious}
        onComplete={handleComplete}
        completing={completing}
        fileUploading={fileUploading}
        progress={progress}
        uploadFile={uploadFile}
        deleteFile={deleteFile}
        selectedReason={selectedReason}
        setSelectedReason={setSelectedReason}
        customReason={customReason}
        setCustomReason={setCustomReason}
        submitterInfo={submitterInfo}
        user={user}
        clearedFields={clearedFields}
        setPendingUploads={setPendingUploads}
      />

      {/* Decline Modal */}
      <Dialog
        open={declineModalOpen}
        onClose={() => setDeclineModalOpen(false)}
        maxWidth="sm"
        fullWidth
      >
        <DialogContent>
          <Typography variant="h6" sx={{ mb: 2 }}>
            Decline Document
          </Typography>
          <Typography variant="body2" sx={{ mb: 3 }}>
            Please provide a reason for declining to sign this document.
          </Typography>
          <TextField
            label="Decline Reason"
            value={declineReason}
            onChange={(e) => setDeclineReason(e.target.value)}
            fullWidth
            multiline
            rows={4}
            placeholder="Enter your reason for declining..."
            required
            autoFocus
          />
        </DialogContent>
        <DialogActions>
          <Button
            onClick={() => setDeclineModalOpen(false)}
            variant="outlined"
            color="inherit"
          >
            Cancel
          </Button>
          <Button
            onClick={handleDeclineConfirm}
            variant="contained"
            color="error"
            disabled={!declineReason.trim()}
          >
            Decline Document
          </Button>
        </DialogActions>
      </Dialog>


      {/* Completion Drawer */}
      <CompletionDrawer
        open={completionDrawerOpen}
        pdfUrl={templateInfo?.document.url || ''}
        signatures={fields.map(field => ({
          field_id: field.id,
          signature_value: texts[field.id] || '',
          field_info: field
        }))}
        templateName={templateInfo?.name || 'document'}
        submitterInfo={submitterInfo}
        token={token}
      />

    </div>
  );
};

export default TemplateEditPage;
