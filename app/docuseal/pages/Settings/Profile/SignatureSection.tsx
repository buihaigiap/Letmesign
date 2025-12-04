import React, { useState, useEffect } from 'react';
import Typography from '@mui/material/Typography';
import Dialog from '@mui/material/Dialog';
import DialogTitle from '@mui/material/DialogTitle';
import DialogContent from '@mui/material/DialogContent';
import DialogActions from '@mui/material/DialogActions';
import Button from '@mui/material/Button';
import IconButton from '@mui/material/IconButton';
import CloseIcon from '@mui/icons-material/Close';
import { Trash2 } from 'lucide-react';
import Box from '@mui/material/Box';
import CardContent from '@mui/material/CardContent';
import CardActions from '@mui/material/CardActions';
import CreateTemplateButton from '@/components/CreateTemplateButton';
import SignatureRenderer from '@/components/SignatureRenderer';
import SignaturePad from '../../TemplateEdit/SignaturePad';
import upstashService from '@/ConfigApi/upstashService';
import { toast } from 'react-hot-toast';
import { useAuth } from '@/contexts/AuthContext';

interface SignatureSectionProps {
  title: string;
  fieldType?: 'signature' | 'initials';
  initialValue?: string; // Initial value from user profile
  onUpdate?: (value: string) => void; // Callback when updated
  userName: string; // User's name to include in API calls
}

const SignatureSection: React.FC<SignatureSectionProps> = ({ 
  title, 
  fieldType = 'signature',
  initialValue = '',
  onUpdate,
  userName
}) => {
  const { refreshUser } = useAuth();
  const [open, setOpen] = useState(false);
  const [savedData, setSavedData] = useState<string>(initialValue);
  const [tempData, setTempData] = useState<string>('');
  const [uploadedFile, setUploadedFile] = useState<File | null>(null);
  const [loading, setLoading] = useState(false);

  // Update savedData when initialValue changes
  useEffect(() => {
    setSavedData(initialValue);
  }, [initialValue]);

  // Clear uploadedFile when tempData changes to non-blob/non-http URL
  // This handles when user switches from upload mode to draw/type mode
  useEffect(() => {
    if (tempData && !tempData.startsWith('blob:') && !tempData.startsWith('http')) {
      // User switched to draw or type mode
      setUploadedFile(null);
    }
  }, [tempData]);

  const handleOpen = () => {
    setTempData(savedData);
    setUploadedFile(null); // Clear any previous uploaded file
    setOpen(true);
  };

  const handleClose = () => {
    setOpen(false);
    setTempData('');
    setUploadedFile(null); // Clear uploaded file when closing
  };

  const handleSave = (dataUrl: string) => {
    setTempData(dataUrl);
    
    // Clear uploaded file if user is not in upload mode
    // (i.e., if they switched to draw or type mode)
    if (!dataUrl.startsWith('blob:') && !dataUrl.startsWith('http')) {
      // User is drawing (data:image) or typing (plain text)
      setUploadedFile(null);
    }
  };

  const handleFileSelected = (file: File | null) => {
    setUploadedFile(file);
    // When file is selected, we still need tempData for preview (blob URL)
    // but uploadedFile takes priority in handleAdd
  };

  const handleAdd = async () => {
    // Check if we have data to save (either tempData or uploadedFile)
    if (!tempData && !uploadedFile) {
      toast.error('Please draw, type, or upload a signature first');
      return;
    }

    setLoading(true);
    try {
      let signatureUrl = tempData;
      
      // If user uploaded a file, use it directly
      if (uploadedFile) {
        const formData = new FormData();
        formData.append('file', uploadedFile);
        
        const uploadResponse = await upstashService.uploadFile(formData);
        if (uploadResponse?.data?.url) {
          signatureUrl = uploadResponse.data.url;
        } else {
          throw new Error('Failed to upload file');
        }
      }
      // Check if tempData is a base64 image (drawn signature)
      else if (tempData.startsWith('data:image')) {
        // Convert base64 to blob
        const response = await fetch(tempData);
        const blob = await response.blob();
        
        // Create FormData and upload file
        const formData = new FormData();
        const filename = `${fieldType}_${Date.now()}.png`;
        formData.append('file', blob, filename);
        
        // Upload file to get URL
        const uploadResponse = await upstashService.uploadFile(formData);
        if (uploadResponse?.data?.url) {
          signatureUrl = uploadResponse.data.url;
        } else {
          throw new Error('Failed to upload file');
        }
      }
      // If it's typed text or already a URL, use as is
      
      // Prepare update data based on field type
      const updateData: any = {
        name: userName,
      };
      
      if (fieldType === 'signature') {
        updateData.signature = signatureUrl;
      } else {
        updateData.initials = signatureUrl;
      }

      // Call API to update profile
      const profileResponse = await upstashService.updateProfile(updateData);
      
      if (profileResponse?.data) {
        setSavedData(signatureUrl);
        handleClose();
        toast.success(`${title} saved successfully!`);
        
        // Refresh user data in AuthContext
        await refreshUser();
        
        // Call callback if provided
        if (onUpdate) {
          onUpdate(signatureUrl);
        }
      }
    } catch (error: any) {
      console.error('Error saving:', error);
      toast.error(error?.error || error?.message || `Failed to save ${title.toLowerCase()}`);
    } finally {
      setLoading(false);
    }
  };

  const handleDelete = async () => {
    setLoading(true);
    try {
      // Prepare update data to clear the field
      const updateData: any = {
        name: userName,
      };
      
      if (fieldType === 'signature') {
        updateData.signature = '';
      } else {
        updateData.initials = '';
      }

      // Call API to update profile
      await upstashService.updateProfile(updateData);
      
      setSavedData('');
      toast.success(`${title} deleted successfully!`);
      
      // Refresh user data in AuthContext
      await refreshUser();
      
      // Call callback if provided
      if (onUpdate) {
        onUpdate('');
      }
    } catch (error: any) {
      console.error('Error deleting:', error);
      toast.error(error?.error || error?.message || `Failed to delete ${title.toLowerCase()}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <>
      <Typography variant="h4" sx={{ mb: 3, mt: fieldType === 'initials' ? 6 : 0 }}>
        {title}
      </Typography>

      {/* Display saved data */}
      {savedData && (
        <Box sx={{ display: 'flex', justifyContent: 'center', mb: 3 }}>
          <Box sx={{ width: { xs: '100%', sm: '50%', md: '33.333%' } }}>
            <CardContent>
              <SignatureRenderer
                color="white"
                data={savedData}
                width={500}
                height={500}
                fieldType={fieldType}
              />
            </CardContent>
            <CardActions sx={{ justifyContent: 'flex-end', pt: 0 }}>
              <Button
                size="small"
                color="error"
                startIcon={<Trash2 className="w-4 h-4" />}
                onClick={handleDelete}
              >
                Delete
              </Button>
            </CardActions>
          </Box>
        </Box>
      )}

      <CreateTemplateButton
        width="100%"
        text={savedData ? `Update ${title}` : `Add ${title}`}
        onClick={handleOpen}
      />

      {/* Dialog */}
      <Dialog open={open} onClose={handleClose} maxWidth="md" fullWidth>
        <DialogTitle>
          Create {title}
          <IconButton
            aria-label="close"
            onClick={handleClose}
            sx={{ position: 'absolute', right: 8, top: 8, color: (theme) => theme.palette.grey[500] }}
          >
            <CloseIcon />
          </IconButton>
        </DialogTitle>
        <DialogContent dividers>
          <SignaturePad 
            noType={true}
            onSave={handleSave} 
            onClear={() => {
              setTempData('');
              setUploadedFile(null);
            }} 
            initialData={tempData}
            onFileSelected={handleFileSelected}
          />
        </DialogContent>
        <DialogActions>
          <Button 
              variant="outlined"
              sx={{
                borderColor: "#475569",
                color: "#cbd5e1",
                textTransform: "none",
                fontWeight: 500,
                "&:hover": { backgroundColor: "#334155" },
             }}
            onClick={handleClose}
          >
            Cancel
          </Button>

          <CreateTemplateButton
            text={`Add ${title}`}
            onClick={handleAdd}
            loading={loading}
            disabled={!tempData && !uploadedFile}
          />
        </DialogActions>
      </Dialog>
    </>
  );
};

export default SignatureSection;
