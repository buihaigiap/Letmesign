import React from 'react';
import ReactDOM from 'react-dom';
import { GripVertical } from 'lucide-react';
import {
  Box,
  Select,
  MenuItem,
  FormControl,
  InputLabel,
  TextField,
} from '@mui/material';

interface GripVerticalMenuProps {
  tempId: string;
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
  overlayRef: React.RefObject<HTMLDivElement>;
}

const GripVerticalMenu: React.FC<GripVerticalMenuProps> = ({
  tempId,
  defaultValue = '',
  onDefaultValueChange,
  validation = { type: 'none' },
  onValidationChange,
  overlayRef,
}) => {
  const [showMenu, setShowMenu] = React.useState(false);
  const [localDefaultValue, setLocalDefaultValue] = React.useState(defaultValue);
  const [validationType, setValidationType] = React.useState(validation.type || 'none');
  const [minLength, setMinLength] = React.useState(validation.minLength || '');
  const [maxLength, setMaxLength] = React.useState(validation.maxLength || '');
  const [regex, setRegex] = React.useState(validation.regex || '');
  const [errorMessage, setErrorMessage] = React.useState(validation.errorMessage || '');
  const gripRef = React.useRef<HTMLDivElement>(null);

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

  const handleClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    setShowMenu(!showMenu);
  };

  const handleDefaultValueChange = (value: string) => {
    setLocalDefaultValue(value);
    onDefaultValueChange(tempId, value);
  };

  const handleDefaultValueBlur = () => {
    setShowMenu(false);
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

          {/* Default value */}
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

          {/* Validation select */}
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

        </Box>
      , overlayRef.current)}
    </div>
  );
};

export default GripVerticalMenu;
