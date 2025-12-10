import  { useState, useEffect, useCallback } from 'react';
import { useParams } from 'react-router-dom';
import upstashService from '../ConfigApi/upstashService';
import SignaturePad from './TemplateEdit/SignaturePad';
import CreateTemplateButton from '../components/CreateTemplateButton';
import PdfFullView from './TemplateEdit/PdfFullView';
import { Dialog, DialogContent, DialogActions, Button, IconButton, Typography, LinearProgress, TextField, Checkbox, Radio, RadioGroup, FormControlLabel, Select, MenuItem, FormControl, InputLabel, Box, Card, CardMedia, Link } from '@mui/material';
import CloseIcon from '@mui/icons-material/Close';
import toast from 'react-hot-toast';

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
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [texts, setTexts] = useState<Record<number, string>>({});
  const [currentFieldIndex, setCurrentFieldIndex] = useState(0);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [page, setPage] = useState(1);
  const [fileUploading, setFileUploading] = useState(false);
  const [submitterInfo, setSubmitterInfo] = useState<{ id: number; email: string } | null>(null);
  console.log('fields' , fields)
  const uploadFile = async (file: File): Promise<string | null> => {
    try {
      setFileUploading(true);
      const formData = new FormData();
      formData.append('file', file);

      const data = await upstashService.uploadPublicFile(formData);
      if (data && data.data && data.data.url) {
        return data.data.url;
      } else {
        console.error('File upload failed:', data);
        return null;
      }
    } catch (error) {
      console.error('File upload error:', error);
      return null;
    } finally {
      setFileUploading(false);
    }
  };

  const fetchTemplateFields = useCallback(async () => {
    try {
      const data = await upstashService.getSubmissionFields(token);
      if (data.success) {
        setTemplateInfo(data.data.template_info);
        
        // Extract submitter information if available
        if (data.data.information) {
          setSubmitterInfo({
            id: data.data.information.id,
            email: data.data.information.email
          });
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
        console.log('Processed fields for signing:', processedFields);
      } else {
        setError(data.message || 'Failed to fetch template fields.');
      }
    } catch (err) {
      console.error('Fetch error:', err);
      setError(`Failed to load template. Please check your connection and try again. Details: ${err}`);
    } finally {
      setLoading(false);
    }
  }, [token]);

  useEffect(() => {
    fetchTemplateFields();
  }, [fetchTemplateFields]);

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
    }
  };

  const handleNext = () => {
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
    // Validate required fields
    const missingFields = fields.filter(field => {
      if (!field.required) return false;
      const value = texts[field.id];
      if (!value) return true;
      // For signature fields (images), check if it's a valid data URL
      if (field.field_type === 'signature' && value.startsWith('data:image/')) {
        return false;
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
      return;
    }


    try {
      const signatures = fields.map(field => ({
        field_id: field.id,
        signature_value: texts[field.id] || ''
      }));

      const data = await upstashService.bulkSign(token, {
        signatures,
        ip_address: '', // TODO: get IP
        user_agent: navigator.userAgent
      });
      if (data.success) {
        toast.success('Completed! Thank you for signing.');
        // Redirect or show success message
      } else {
        toast.error(`Error: ${data.message}`);
      }
    } catch (err) {
      console.error('Submit error:', err);
      toast.error('Unable to submit signature. Please try again.');
    }
  };

  const currentField = fields[currentFieldIndex];
  const isLastField = currentFieldIndex === fields.length - 1;
  if (loading) return <div className="flex items-center justify-center min-h-screen">Loading...</div>;
  if (error) return <div className="text-red-500 text-center p-4">{error}</div>;
  return (
    <div className="container mx-auto p-4">
      <h1 className="text-2xl font-bold mb-4">{templateInfo?.name}</h1>
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
      />

      {/* Form Modal */}
      <Dialog
        open={isModalOpen}
        onClose={() => setIsModalOpen(false)}
        maxWidth="sm"
        fullWidth
      >
        <DialogContent sx={{ position: 'relative' }}>
          <IconButton
            onClick={() => setIsModalOpen(false)}
            sx={{
              position: 'absolute',
              top: 8,
              right: 8,
              color: 'grey.500',
            }}
          >
            <CloseIcon />
          </IconButton>
          <div className="mb-4">
            <Typography variant="body2" sx={{ mb: 1 }}>
              Field {currentFieldIndex + 1} / {fields.length}
            </Typography>
            <LinearProgress
              variant="determinate"
              value={((currentFieldIndex + 1) / fields.length) * 100}
              sx={{
                height: 8,
                borderRadius: 4,
                backgroundColor: 'grey.300',
                '& .MuiLinearProgress-bar': {
                  backgroundColor: 'primary.main',
                  borderRadius: 4,
                },
              }}
            />
          </div>

          {currentField && (
            <div className="mb-6">
              <Typography variant="subtitle1" sx={{ mb: 1, fontWeight: 'medium' }}>
                {currentField.name} {currentField.required && <span style={{ color: 'red' }}>*</span>}
              </Typography>
              {currentField.field_type === 'date' ? (
                <TextField
                  type="date"
                  value={texts[currentField.id] || ''}
                  onChange={(e) => handleTextChange(currentField.id, e.target.value)}
                  fullWidth
                  required={currentField.required}
                  autoFocus
                  InputLabelProps={{ shrink: true }}
                />
              ) : currentField.field_type === 'checkbox' ? (
                <FormControlLabel
                  control={
                    <Checkbox
                      checked={texts[currentField.id] === 'true'}
                      onChange={(e) => handleTextChange(currentField.id, e.target.checked ? 'true' : 'false')}
                      required={currentField.required}
                      autoFocus
                    />
                  }
                  label={currentField.name}
                />
              ) : currentField.field_type === 'signature' ? (
                <SignaturePad
                  onSave={(dataUrl) => handleTextChange(currentField.id, dataUrl)}
                  onClear={() => handleTextChange(currentField.id, '')}
                  initialData={texts[currentField.id]}
                />
              ) : currentField.field_type === 'image' ? (
                <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
                  <input
                    type="file"
                    accept="image/*"
                    onChange={async (e) => {
                      const file = e.target.files?.[0];
                      if (file) {
                        const maxSize = 10 * 1024 * 1024; // 10MB
                        if (file.size > maxSize) {
                          toast.error(`File too large. Maximum allowed size is ${maxSize / (1024 * 1024)}MB. Current file: ${(file.size / (1024 * 1024)).toFixed(2)}MB.`);
                          return;
                        }
                        const imageUrl = await uploadFile(file);
                        if (imageUrl) {
                          handleTextChange(currentField.id, imageUrl);
                        } else {
                          toast.error('Unable to upload image. Please try again.');
                        }
                      }
                    }}
                    style={{ display: 'none' }}
                    id={`image-upload-${currentField.id}`}
                    disabled={fileUploading}
                    required={currentField.required}
                  />
                  <label htmlFor={`image-upload-${currentField.id}`}>
                    <Button variant="outlined" component="span" fullWidth disabled={fileUploading}>
                      Select image
                    </Button>
                  </label>
                  <Typography variant="caption" color="text.secondary">
                    Kích thước tối đa: 10MB
                  </Typography>
                  {fileUploading && (
                    <Typography variant="body2" color="primary">
                      Uploading image...
                    </Typography>
                  )}
                  {texts[currentField.id] && (
                    <Box sx={{ mt: 1 }}>
                      <Card sx={{ maxWidth: 200 }}>
                        <CardMedia
                          component="img"
                          height="140"
                          image={texts[currentField.id]}
                          alt="Uploaded preview"
                        />
                      </Card>
                      <Button
                        size="small"
                        color="error"
                        onClick={() => handleTextChange(currentField.id, '')}
                        sx={{ mt: 1 }}
                      >
                        Delete image
                      </Button>
                    </Box>
                  )}
                </Box>
              ) : currentField.field_type === 'file' ? (
                <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
                  <input
                    type="file"
                    onChange={async (e) => {
                      const file = e.target.files?.[0];
                      if (file) {
                        const maxSize = 10 * 1024 * 1024; // 10MB
                        if (file.size > maxSize) {
                          toast.error(`File too large. Maximum allowed size is ${maxSize / (1024 * 1024)}MB. Current file: ${(file.size / (1024 * 1024)).toFixed(2)}MB.`);
                          return;
                        }
                        const fileUrl = await uploadFile(file);
                        if (fileUrl) {
                          handleTextChange(currentField.id, fileUrl);
                        } else {
                          toast.error('Unable to upload file. Please try again.');
                        }
                      }
                    }}
                    style={{ display: 'none' }}
                    id={`file-upload-${currentField.id}`}
                    disabled={fileUploading}
                    required={currentField.required}
                  />
                  <label htmlFor={`file-upload-${currentField.id}`}>
                    <Button variant="outlined" component="span" fullWidth disabled={fileUploading}>
                      Select file
                    </Button>
                  </label>
                  <Typography variant="caption" color="text.secondary">
                    Kích thước tối đa: 10MB
                  </Typography>
                  {fileUploading && (
                    <Typography variant="body2" color="primary">
                      Uploading file...
                    </Typography>
                  )}
                  {texts[currentField.id] && (
                    <Box sx={{ mt: 1 }}>
                      <Link href={texts[currentField.id]} download underline="hover">
                        {decodeURIComponent(texts[currentField.id].split('/').pop() || 'File')}
                      </Link>
                      <Button
                        size="small"
                        color="error"
                        onClick={() => handleTextChange(currentField.id, '')}
                        sx={{ ml: 1 }}
                      >
                        Delete file
                      </Button>
                    </Box>
                  )}
                </Box>
              ) : currentField.field_type === 'number' ? (
                <TextField
                  type="number"
                  value={texts[currentField.id] || ''}
                  onChange={(e) => handleTextChange(currentField.id, e.target.value)}
                  fullWidth
                  placeholder={`Enter ${currentField.name}`}
                  required={currentField.required}
                  autoFocus
                />
              ) : currentField.field_type === 'radio' ? (
                <RadioGroup
                  value={texts[currentField.id] || ''}
                  onChange={(e) => handleTextChange(currentField.id, e.target.value)}
                >
                  {currentField.options?.map((option: string, index: number) => (
                    <FormControlLabel
                      key={index}
                      value={option}
                      control={<Radio required={currentField.required} />}
                      label={option}
                    />
                  ))}
                </RadioGroup>
              ) : currentField.field_type === 'select' ? (
                <FormControl fullWidth required={currentField.required}>
                  <InputLabel>Select an option</InputLabel>
                  <Select
                    value={texts[currentField.id] || ''}
                    onChange={(e) => handleTextChange(currentField.id, e.target.value)}
                    autoFocus
                  >
                    {currentField.options?.map((option: string, index: number) => (
                      <MenuItem key={index} value={option}>
                        {option}
                      </MenuItem>
                    ))}
                  </Select>
                </FormControl>
              ) : currentField.field_type === 'cells' ? (
                <TextField
                  value={texts[currentField.id] || ''}
                  onChange={(e) => handleTextChange(currentField.id, e.target.value)}
                  fullWidth
                  placeholder={`Enter up to ${currentField.options?.columns || 1} characters`}
                  required={currentField.required}
                  autoFocus
                  inputProps={{
                    maxLength: currentField.options?.columns || 1,
                  }}
                />
              ) : (
                <TextField
                  value={texts[currentField.id] || ''}
                  onChange={(e) => handleTextChange(currentField.id, e.target.value)}
                  fullWidth
                  placeholder={`Enter ${currentField.name}`}
                  required={currentField.required}
                  autoFocus
                />
              )}
            </div>
          )}
        </DialogContent>
        <DialogActions>
            <Button
              onClick={handlePrevious}
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
                Previous
            </Button>
          {!isLastField ? (
            <Button
              onClick={handleNext}
              variant="contained"
            >
              Next
            </Button>
          ) : (
            <CreateTemplateButton onClick={() => handleComplete()} text="Complete" />
          )}
        </DialogActions>
      </Dialog>
    </div>
  );
};

export default TemplateEditPage;
