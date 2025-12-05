import { useState, useEffect } from 'react';
import {
  Dialog, DialogContent, DialogActions, Button, IconButton,
  Typography, LinearProgress, TextField, Checkbox, Radio,
  RadioGroup, FormControlLabel, Select, MenuItem,
  FormControl, InputLabel, Box, Card,
  CardMedia, Link
} from '@mui/material';
import CloseIcon from '@mui/icons-material/Close';
import { Trash } from 'lucide-react';
import toast from 'react-hot-toast';
import SignaturePad from './SignaturePad';
import CreateTemplateButton from '../../components/CreateTemplateButton';

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
    suggested?: string;
    allow_custom?: boolean;
  };
  options?: any;
  partner?: string;
  created_at: string;
  updated_at: string;
}

interface FormModalProps {
  open: boolean;
  onClose: () => void;
  currentFieldIndex: number;
  fields: TemplateField[];
  texts: Record<number, string>;
  onTextChange: (fieldId: number, value: string, isMultiple?: boolean, checked?: boolean) => void;
  onNext: () => void;
  onPrevious: () => void;
  onComplete: () => void;
  completing: boolean;
  fileUploading: boolean;
  progress: number;
  uploadFile: (file: File) => Promise<string | null>;
  deleteFile: (fileUrl: string) => Promise<boolean>;
  selectedReason: string;
  setSelectedReason: (reason: string) => void;
  customReason: string;
  setCustomReason: (reason: string) => void;
  submitterInfo: {
    id: number;
    email: string;
    template_name?: string;
    status: string;
    signed_at?: string;
    global_settings?: any;
  } | null;
  user: any;
  clearedFields: Set<number>;
  setPendingUploads: React.Dispatch<React.SetStateAction<Record<number, File>>>;
}

const LinearProgressWithLabel = (props: any) => {
  return (
    <Box sx={{ display: 'flex', alignItems: 'center' }}>
      <Box sx={{ width: '100%', mr: 1 }}>
        <LinearProgress variant="determinate" {...props} />
      </Box>
      <Box sx={{ minWidth: 35 }}>
        <Typography variant="body2" color="text.secondary">{`${Math.round(
          props.value,
        )}%`}</Typography>
      </Box>
    </Box>
  );
};

const FormModal = ({
  open,
  onClose,
  currentFieldIndex,
  fields,
  texts,
  onTextChange,
  onNext,
  onPrevious,
  onComplete,
  completing,
  fileUploading,
  progress,
  uploadFile,
  deleteFile,
  selectedReason,
  setSelectedReason,
  customReason,
  setCustomReason,
  submitterInfo,
  user,
  clearedFields,
  setPendingUploads
}: FormModalProps) => {
  const currentField = fields[currentFieldIndex];
  const isLastField = currentFieldIndex === fields.length - 1;

  const handleDeleteImage = async (fieldId: number) => {
    const fileUrl = texts[fieldId];
    if (fileUrl) {
      const success = await deleteFile(fileUrl);
      if (success) {
        onTextChange(fieldId, '');
      }
    } else {
      onTextChange(fieldId, '');
    }
  };

  if (!currentField) {
    return null;
  }

  return (
    <Dialog
      open={open}
      onClose={onClose}
      maxWidth="sm"
      fullWidth
    >
      <DialogContent sx={{ position: 'relative' }}>
        <IconButton
          onClick={onClose}
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

        <div className="mb-6">
          {currentField.field_type === 'date' ? (
            <TextField
              type="date"
              value={texts[currentField.id] || ''}
              onChange={(e) => onTextChange(currentField.id, e.target.value)}
              fullWidth
              required={currentField.required}
              autoFocus
              InputLabelProps={{ shrink: true }}
              sx={{
                '& .MuiInputBase-input': { color: 'white' },
                '& input::-webkit-calendar-picker-indicator': {
                  filter: 'invert(1)',
                },
              }}
            />
          ) : currentField.field_type === 'checkbox' ? (
            <FormControlLabel
              control={
                <Checkbox
                  checked={texts[currentField.id] === 'true'}
                  onChange={(e) => onTextChange(currentField.id, e.target.checked ? 'true' : 'false')}
                  required={currentField.required}
                  autoFocus
                />
              }
              label={currentField.name}
            />
          ) : currentField.field_type === 'signature' || currentField.field_type === 'initials' ? (
            <SignaturePad
              onSave={(dataUrl) => onTextChange(currentField.id, dataUrl)}
              onClear={() => onTextChange(currentField.id, '')}
              initialData={texts[currentField.id] || (!clearedFields.has(currentField.id) && submitterInfo?.global_settings?.remember_and_pre_fill_signatures && (currentField.field_type === 'signature' ? user?.signature : user?.initials))}
              fieldType={currentField.field_type}
              onFileSelected={(file) => {
                if (file) {
                  const blobUrl = URL.createObjectURL(file);
                  onTextChange(currentField.id, blobUrl);
                  setPendingUploads(prev => ({ ...prev, [currentField.id]: file }));
                } else {
                  setPendingUploads(prev => {
                    const newUploads = { ...prev };
                    delete newUploads[currentField.id];
                    return newUploads;
                  });
                }
              }}
            />
          ) : currentField.field_type === 'image' ? (
            <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
              <input
                type="file"
                accept="image/*"
                onChange={async (e) => {
                  const file = e.target.files?.[0];
                  if (file) {
                    const maxSize = 10 * 1024 * 1024;
                    if (file.size > maxSize) {
                      toast.error(`File too large. Maximum allowed size is ${maxSize / (1024 * 1024)}MB. Current file: ${(file.size / (1024 * 1024)).toFixed(2)}MB.`);
                      return;
                    }
                    const imageUrl = await uploadFile(file);
                    if (imageUrl) {
                      onTextChange(currentField.id, imageUrl);
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
              {!texts[currentField.id] && !fileUploading && (
                <>
                  <label htmlFor={`image-upload-${currentField.id}`}>
                    <Button
                      variant="outlined"
                      component="span"
                      fullWidth
                      sx={{ color: 'white' }}
                    >
                      Select image
                    </Button>
                  </label>
                  <Typography variant="caption" color="text.secondary">
                    Kích thước tối đa: 10MB
                  </Typography>
                </>
              )}
              {fileUploading && (
                <LinearProgressWithLabel value={progress} />
              )}
              {texts[currentField.id] && !fileUploading && (
                <Box 
                  sx={{ 
                    mt: 1, 
                    position: 'relative',
                    maxWidth: 200,
                    mx: 'auto',
                    '&:hover .delete-icon': {
                      opacity: 1
                    }
                  }}
                >
                  <Card>
                    <CardMedia
                      component="img"
                      height="140"
                      image={texts[currentField.id]}
                      alt="Uploaded preview"
                      sx={{ objectFit: 'contain' }}
                    />
                  </Card>
                  <IconButton
                    className="delete-icon"
                    onClick={() => handleDeleteImage(currentField.id)}
                    sx={{
                      position: 'absolute',
                      top: '50%',
                      left: '50%',
                      transform: 'translate(-50%, -50%)',
                      backgroundColor: 'rgba(0, 0, 0, 0.6)',
                      color: 'white',
                      opacity: 0,
                      transition: 'opacity 0.3s ease',
                      '&:hover': {
                        backgroundColor: 'rgba(255, 0, 0, 0.8)',
                      }
                    }}
                  >
                    <Trash size={20} />
                  </IconButton>
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
                    const maxSize = 10 * 1024 * 1024;
                    if (file.size > maxSize) {
                      toast.error(`File too large. Maximum allowed size is ${maxSize / (1024 * 1024)}MB. Current file: ${(file.size / (1024 * 1024)).toFixed(2)}MB.`);
                      return;
                    }
                    const fileUrl = await uploadFile(file);
                    if (fileUrl) {
                      onTextChange(currentField.id, fileUrl);
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
              {!texts[currentField.id] && !fileUploading && (
                <>
                  <label htmlFor={`file-upload-${currentField.id}`}>
                    <Button
                      variant="outlined"
                      component="span"
                      fullWidth
                      sx={{ color: 'white' }}
                    >
                      Select file
                    </Button>
                  </label>
                  <Typography variant="caption" color="text.secondary">
                    Kích thước tối đa: 10MB
                  </Typography>
                </>
              )}
              {fileUploading && (
                <LinearProgressWithLabel value={progress} />
              )}
              {texts[currentField.id] && !fileUploading && (
                <Box sx={{ mt: 1, display: 'flex', alignItems: 'center', gap: 1 }}>
                  <Link href={texts[currentField.id]} download underline="hover" color='white'>
                    {decodeURIComponent(texts[currentField.id].split('/').pop() || 'File')}
                  </Link>
                  <IconButton
                    onClick={() => handleDeleteImage(currentField.id)}
                    sx={{
                      color: 'red',
                      padding: '4px',
                      '&:hover': {
                        backgroundColor: 'rgba(255, 0, 0, 0.1)',
                      }
                    }}
                  >
                    <Trash size={20} />
                  </IconButton>
                </Box>
              )}
            </Box>
          ) : currentField.field_type === 'number' ? (
            <TextField
              type="number"
              value={texts[currentField.id] || ''}
              onChange={(e) => onTextChange(currentField.id, e.target.value)}
              fullWidth
              placeholder={`Enter ${currentField.name}`}
              required={currentField.required}
              autoFocus
            />
          ) : currentField.field_type === 'multiple' ? (
            <div>
              {currentField.options?.map((option: string, index: number) => (
                <FormControlLabel
                  key={index}
                  control={
                    <Checkbox
                      checked={texts[currentField.id]?.split(',').includes(option)}
                      onChange={(e) => onTextChange(currentField.id, option, true, e.target.checked)}
                      required={currentField.required}
                    />
                  }
                  label={option}
                />
              ))}
            </div>
          ) : currentField.field_type === 'radio' ? (
            <RadioGroup
              value={texts[currentField.id] || ''}
              onChange={(e) => onTextChange(currentField.id, e.target.value)}
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
                label="Select an option"
                onChange={(e) => onTextChange(currentField.id, e.target.value)}
                MenuProps={{
                  PaperProps: {
                    sx: {
                      color: 'white'
                    }
                  }
                }}
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
              onChange={(e) => onTextChange(currentField.id, e.target.value)}
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
              onChange={(e) => onTextChange(currentField.id, e.target.value)}
              fullWidth
              placeholder={`Enter ${currentField.name}`}
              required={currentField.required}
              autoFocus
            />
          )}
        </div>

        {submitterInfo?.global_settings?.require_signing_reason && (currentField.field_type === 'signature' || currentField.field_type === 'initials') && (
          <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2, mt: 2 }}>
            <FormControl fullWidth>
              <InputLabel>Signing Reason</InputLabel>
              <Select
                value={selectedReason}
                onChange={(e) => setSelectedReason(e.target.value)}
                label="Signing Reason"
              >
                <MenuItem value="Approved">Approved</MenuItem>
                <MenuItem value="Reviewed">Reviewed</MenuItem>
                <MenuItem value="Authored">Authored</MenuItem>
                <MenuItem value="Other">Other</MenuItem>
              </Select>
            </FormControl>
            {selectedReason === 'Other' && (
              <TextField
                label="Custom Reason"
                value={customReason}
                onChange={(e) => setCustomReason(e.target.value)}
                fullWidth
                variant="outlined"
              />
            )}
          </Box>
        )}
      </DialogContent>
      <DialogActions>
        {currentFieldIndex > 0 && (
          <Button
            disabled={completing}
            onClick={onPrevious}
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
        )}
        {!isLastField ? (
          <CreateTemplateButton
            onClick={onNext}
            text="Next"
          />
        ) : (
          <CreateTemplateButton
            loading={completing}
            onClick={onComplete}
            text="Complete"
          />
        )}
      </DialogActions>
    </Dialog>
  );
};

export default FormModal;
