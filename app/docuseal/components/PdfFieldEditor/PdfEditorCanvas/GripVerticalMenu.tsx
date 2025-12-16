import React from 'react';
import ReactDOM from 'react-dom';
import { CircleAlert, GripVertical , Copy , Move3d } from 'lucide-react';
import {
  Box,
  Select,
  MenuItem,
  FormControl,
  InputLabel,
  TextField,
  Switch,
  FormControlLabel,
  Typography,
} from '@mui/material';
import DescriptionDialog from './DescriptionDialog';
import ConditionDialog from './ConditionDialog';
import upstashService from '../../../ConfigApi/upstashService';
import toast from 'react-hot-toast';

interface GripVerticalMenuProps {
  tempId: string;
  fieldId: number;
  fieldType: string;
  defaultValue?: string;
  onDefaultValueChange: (tempId: string, value: string) => void;
  validation?: {
    type: string;
    minLength?: string;
    maxLength?: string;
    regex?: string;
    errorMessage?: string;
  };
  onValidationChange: (tempId: string, validation: any) => void;
  readOnly?: boolean;
  onReadOnlyChange: (tempId: string, readOnly: boolean) => void;
  onDescriptionChange: (tempId: string, desc: { displayTitle: string, description: string }) => void;
  onConditionChange: (tempId: string, condition: { dependentField: string, condition: string }) => void;
  overlayRef: React.RefObject<HTMLDivElement>;
  token: string;
  templateId: number;
  currentOptions?: any;
  copyToAllPages: (tempId: string, numPages: number) => void;
  numPages: number;
  allFields: Array<{ tempId: string; label: string }>;
}

const GripVerticalMenu: React.FC<GripVerticalMenuProps> = ({
  tempId,
  fieldId,
  fieldType,
  defaultValue = '',
  onDefaultValueChange,
  validation = { type: 'none' },
  onValidationChange,
  readOnly = false,
  onReadOnlyChange,
  onDescriptionChange,
  onConditionChange,
  overlayRef,
  token,
  templateId,
  currentOptions = {},
  copyToAllPages,
  numPages,
  allFields,
}) => {
  const [showMenu, setShowMenu] = React.useState(false);
  const [localDefaultValue, setLocalDefaultValue] = React.useState(defaultValue);
  const [validationType, setValidationType] = React.useState(validation.type || 'none');
  const [minLength, setMinLength] = React.useState(validation.minLength || '');
  const [maxLength, setMaxLength] = React.useState(validation.maxLength || '');
  const [regex, setRegex] = React.useState(validation.regex || '');
  const [errorMessage, setErrorMessage] = React.useState(validation.errorMessage || '');
  const [isReadOnly, setIsReadOnly] = React.useState(readOnly);
  const gripRef = React.useRef<HTMLDivElement>(null);
  const menuRef = React.useRef<HTMLDivElement>(null);
  const [selectOpen, setSelectOpen] = React.useState(false);
  const [dialogOpen, setDialogOpen] = React.useState(false);
  const [description, setDescription] = React.useState('');
  const [displayTitle, setDisplayTitle] = React.useState('');
  const [conditionDialogOpen, setConditionDialogOpen] = React.useState(false);
  const [dependentField, setDependentField] = React.useState('');
  const [conditionType, setConditionType] = React.useState('not_empty');

  React.useEffect(() => {
    setLocalDefaultValue(defaultValue);
  }, [defaultValue]);

  React.useEffect(() => {
    setValidationType(validation.type || 'none');
    setMinLength(validation.minLength || '');
    setMaxLength(validation.maxLength || '');
    setRegex(validation.regex || '');
    setErrorMessage(validation.errorMessage || '');
  }, [validation]);

  React.useEffect(() => {
    setIsReadOnly(readOnly);
  }, [readOnly]);

  React.useEffect(() => {
    if (conditionDialogOpen) {
      setDependentField(currentOptions?.condition?.dependentField || '');
      setConditionType(currentOptions?.condition?.condition || 'not_empty');
    }
  }, [conditionDialogOpen]);

  React.useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (selectOpen || dialogOpen || conditionDialogOpen) return;
      if (
        menuRef.current &&
        !menuRef.current.contains(event.target as Node) &&
        gripRef.current &&
        !gripRef.current.contains(event.target as Node)
      ) {
        setShowMenu(false);
      }
    };

    if (showMenu) {
      document.addEventListener('mousedown', handleClickOutside);
    }

    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [showMenu, selectOpen, dialogOpen, conditionDialogOpen]);

  const handleClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    setShowMenu(!showMenu);
  };

  const handleDefaultValueChange = (value: string) => {
    setLocalDefaultValue(value);
    onDefaultValueChange(tempId, value);
  };

  

  const handleMenuClose = () => {
    setShowMenu(false);
    const newValidation = {
      type: validationType,
      ...(validationType === 'length' && { minLength, maxLength }),
      ...(validationType === 'custom' && { regex, errorMessage }),
    };
    onValidationChange(tempId, newValidation);
  };

  const handleDefaultValueKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      handleMenuClose();
    } else if (e.key === 'Escape') {
      setLocalDefaultValue(defaultValue);
      setValidationType(validation.type || 'none');
      setMinLength(validation.minLength || '');
      setMaxLength(validation.maxLength || '');
      setRegex(validation.regex || '');
      setErrorMessage(validation.errorMessage || '');
      setIsReadOnly(readOnly);
      setShowMenu(false);
    }
  };

  const validationOptions = [
    { value: 'none', label: 'None' },
    { value: 'length', label: 'Length' },
    { value: 'ssn', label: 'SSN' },
    { value: 'ein', label: 'EIN' },
    { value: 'email', label: 'Email' },
    { value: 'url', label: 'URL' },
    { value: 'zip', label: 'ZIP' },
    { value: 'custom', label: 'Custom' },
    { value: 'numbers_only', label: 'Numbers only' },
    { value: 'letters_only', label: 'Letters only' },
  ];

  const inputStyle = {
    width: "100%",
    "& .MuiOutlinedInput-input": {
      fontSize: "14px",
      color: "#000",
      padding: "4px",
    },
    "& .MuiInputLabel-root": {
      color: "black",
    },
    "& .MuiOutlinedInput-root": {
      borderRadius: "4px",
      "& fieldset": { borderColor: "black" },
      "&:hover fieldset": { borderColor: "black" },
      "&.Mui-focused fieldset": { borderColor: "black" },
    },
  };

  return (
    <div
      ref={gripRef}
      className="relative grip-menu hover:bg-white hover:bg-opacity-10 w-4 h-4"
      data-grip
    >
      <GripVertical
        size={15}
        className="cursor-pointer"
        onClick={handleClick}
      />

      {showMenu && gripRef.current && overlayRef.current && ReactDOM.createPortal(
        <Box
          ref={menuRef}
          className="MuiMenu-root"
          sx={{
            position: 'absolute',
            zIndex: 9999,
            bgcolor: "white",
            width: 'auto',
            padding: '8px',
            borderRadius: '4px',
            minWidth: 200,
            display: 'flex',
            flexDirection: 'column',
            gap: 1,
            ...(() => {
              const rect = gripRef.current!.getBoundingClientRect();
              const overlayRect = overlayRef.current!.getBoundingClientRect();
              return {
                top: rect.bottom - overlayRect.top,
                left: rect.left - overlayRect.left,
              };
            })(),
          }}
        >

          {/* Default value - hide for image, multiple, signature, initials, and file */}
          {fieldType !== 'image' && fieldType !== 'multiple' && fieldType !== 'signature' && fieldType !== 'initials' && fieldType !== 'file' && (
            <TextField
              variant="outlined"
              size="small"
              placeholder="Default value"
              value={localDefaultValue}
              onChange={(e) => handleDefaultValueChange(e.target.value)}
              onBlur={handleMenuClose}
              onKeyDown={handleDefaultValueKeyDown}
              onClick={(e) => e.stopPropagation()}
              sx={inputStyle}
            />
          )}

          {/* Validation select - hide for image, multiple, signature, initials, and file */}
          {fieldType !== 'image' && fieldType !== 'multiple' && fieldType !== 'signature' && fieldType !== 'initials' && fieldType !== 'file' && (
          <>
          <FormControl fullWidth>
            <InputLabel
              sx={{
                color: 'black',
                '&.Mui-focused': { color: 'black' },
                '&.MuiInputLabel-shrink': { color: 'black' },
              }}
            >
              Validation
            </InputLabel>

            <Select
              value={validationType}
              label="Validation"
              onChange={(e) => {
                const newType = e.target.value;
                setValidationType(newType);
                const newValidation = {
                  type: newType,
                  ...(newType === 'length' && { minLength, maxLength }),
                  ...(newType === 'custom' && { regex, errorMessage }),
                };
                onValidationChange(tempId, newValidation);
              }}
              onClick={(e) => e.stopPropagation()}
              sx={{
                fontSize: '14px',
                height: '20px',
                borderRadius: '4px',
                '& .MuiSelect-select': { color: '#000' },
                '& .Mui-focused .MuiSelect-select': { color: '#000' },
                '& .MuiSelect-select.MuiSelect-select': { color: '#000' },
                '&.Mui-disabled .MuiSelect-select': {
                  color: '#000',
                  WebkitTextFillColor: '#000',
                },
                '& .MuiOutlinedInput-notchedOutline': {
                  borderColor: 'black',
                },
                '&:hover .MuiOutlinedInput-notchedOutline': {
                  borderColor: 'black',
                },
                '&.Mui-focused .MuiOutlinedInput-notchedOutline': {
                  borderColor: 'black',
                },
              }}
            >
              {validationOptions.map(option => (
                <MenuItem key={option.value} value={option.value}>
                  {option.label}
                </MenuItem>
              ))}
            </Select>
          </FormControl>

          {validationType === 'length' && (
            <Box
              sx={{ 
                display: 'flex',
                gap: '2px'
              }}
            >
              <TextField
                type="number"
                placeholder="Min length"
                size="small"
                value={minLength}
                onChange={(e) => {
                  const val = e.target.value;
                  setMinLength(val);
                  const newValidation = {
                    type: validationType,
                    minLength: val,
                    maxLength,
                  };
                  onValidationChange(tempId, newValidation);
                }}
                onClick={(e) => e.stopPropagation()}
                sx={inputStyle}
              />

              <TextField
                type="number"
                placeholder="Max length"
                size="small"
                value={maxLength}
                onChange={(e) => {
                  const val = e.target.value;
                  setMaxLength(val);
                  const newValidation = {
                    type: validationType,
                    minLength,
                    maxLength: val,
                  };
                  onValidationChange(tempId, newValidation);
                }}
                onClick={(e) => e.stopPropagation()}
                sx={inputStyle}
              />
            </Box>
          )}

          {validationType === 'custom' && (
            <Box
              sx={{
                display: "flex",
                flexDirection: "column",
                gap: "6px", 
              }}
            >
              <TextField
                placeholder="Regexp validation"
                size="small"
                value={regex}
                onChange={(e) => {
                  const val = e.target.value;
                  setRegex(val);
                  const newValidation = {
                    type: validationType,
                    regex: val,
                    errorMessage,
                  };
                  onValidationChange(tempId, newValidation);
                }}
                onClick={(e) => e.stopPropagation()}
                sx={inputStyle}
              />

              <TextField
                placeholder="Error message"
                size="small"
                value={errorMessage}
                onChange={(e) => {
                  const val = e.target.value;
                  setErrorMessage(val);
                  const newValidation = {
                    type: validationType,
                    regex,
                    errorMessage: val,
                  };
                  onValidationChange(tempId, newValidation);
                }}
                onClick={(e) => e.stopPropagation()}
                sx={inputStyle}
              />
            </Box>
          )}
          </>
          )}

          {/* Read-Only - hide for image, multiple, signature, initials, and file */}
          {fieldType !== 'image' && fieldType !== 'multiple' &&
           fieldType !== 'signature' && fieldType !== 'initials' && fieldType !== 'file' &&
            (
          <FormControlLabel
            control={
              <Switch
                checked={isReadOnly}
                onChange={(e) => {
                  const checked = e.target.checked;
                  setIsReadOnly(checked);
                  onReadOnlyChange(tempId, checked);
                }}
                onClick={(e) => e.stopPropagation()}
                sx={{
                  '& .MuiSwitch-switchBase.Mui-checked': {
                    color: 'black',
                  },
                  '& .MuiSwitch-switchBase.Mui-checked + .MuiSwitch-track': {
                    backgroundColor: 'black',
                  },
                }}
              />
            }
            label="Read-Only"
            sx={{
              color: 'black',
              '& .MuiFormControlLabel-label': {
                fontSize: '14px',
              },
            }}
          />
          )}

          {/* Description - always show */}
          <Box onClick={(e) => {
            e.stopPropagation();
            setDisplayTitle(currentOptions?.displayTitle || '');
            setDescription(currentOptions?.description || '');
            setDialogOpen(true);
            setShowMenu(false);
          }} sx={{ cursor: 'pointer', display: 'flex', alignItems: 'center' }}>
            <CircleAlert color='black' style={{ marginRight: 8 }} />
            <Typography sx={{ fontSize: '14px', color: 'black' }}>
              Description
            </Typography>
          </Box>

          {/* Copy to All Pages - hide for image, multiple, and file */}
          {fieldType !== 'image' && fieldType !== 'multiple' && fieldType !== 'file' && (
          <Box 
              onClick={(e) => {
                e.stopPropagation();
                if (numPages > 1) {
                  copyToAllPages(tempId, numPages);
                  setShowMenu(false);
                  toast.success('Field copied to all pages');
                }
              }}
              sx={{ 
                cursor: numPages > 1 ? 'pointer' : 'not-allowed', 
                display: 'flex', 
                alignItems: 'center',
                opacity: numPages > 1 ? 1 : 0.5
              }}
          >
              <Copy color='black' style={{ marginRight: 8 }} />
              <Typography sx={{ fontSize: '14px', color: 'black' }}>
                Copy to All Pages
              </Typography>
          </Box>
          )}

          {/* Condition - always show */}
          {allFields.length > 0 && allFields[0].tempId !== tempId && (
            <Box onClick={(e) => {
              e.stopPropagation();
              setConditionDialogOpen(true);
              setShowMenu(false);
            }} sx={{ cursor: 'pointer', display: 'flex', alignItems: 'center' }}>
              <Move3d color='black' style={{ marginRight: 8 }} />
              <Typography sx={{ fontSize: '14px', color: 'black' }}>
                Condition
              </Typography>
            </Box>
          )}
        </Box>
      , overlayRef.current)}
      <DescriptionDialog
        open={dialogOpen}
        onClose={() => setDialogOpen(false)}
        displayTitle={displayTitle}
        onDisplayTitleChange={setDisplayTitle}
        description={description}
        onDescriptionChange={setDescription}
        onSave={() => {
          const newOptions = { ...currentOptions, displayTitle, description };
          onDescriptionChange(tempId, { displayTitle, description });
          setDialogOpen(false);
          upstashService.updateField(templateId, fieldId, { options: newOptions }).catch((error) => {
            console.error('Failed to save description:', error);
            toast.error('Failed to save description');
          });
        }}
      />
      <ConditionDialog
        open={conditionDialogOpen}
        onClose={() => setConditionDialogOpen(false)}
        dependentField={dependentField}
        onDependentFieldChange={setDependentField}
        condition={conditionType}
        onConditionChange={setConditionType}
        allFields={allFields}
        currentTempId={tempId}
        onSave={() => {
          // Convert tempId to field name before saving
          const depField = allFields.find(f => f.tempId === dependentField);
          const fieldIdentifier = depField ? depField.label : dependentField;
          onConditionChange(tempId, { 
            dependentField: fieldIdentifier, 
            condition: conditionType 
          });
          setConditionDialogOpen(false);
        }}
      />
    </div>
  );
};

export default GripVerticalMenu;
