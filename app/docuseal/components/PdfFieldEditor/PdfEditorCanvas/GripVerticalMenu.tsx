import React from 'react';
import { GripVertical } from 'lucide-react';
import { Input   , Box, Select, MenuItem, FormControl, InputLabel} from '@mui/material';

interface GripVerticalMenuProps {
  tempId: string;
  onDuplicate: (tempId: string) => void;
  onDelete: (tempId: string) => void;
  defaultValue?: string;
  onDefaultValueChange: (tempId: string, value: string) => void;
  // Add more actions as needed
}

const GripVerticalMenu: React.FC<GripVerticalMenuProps> = ({
  tempId,
  onDuplicate,
  onDelete,
  defaultValue = '',
  onDefaultValueChange,
}) => {
  const [showMenu, setShowMenu] = React.useState(false);
  const [localDefaultValue, setLocalDefaultValue] = React.useState(defaultValue);
  const [validationType, setValidationType] = React.useState('none');

  React.useEffect(() => {
    setLocalDefaultValue(defaultValue);
  }, [defaultValue]);

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

  const handleDefaultValueKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      setShowMenu(false);
    } else if (e.key === 'Escape') {
      setLocalDefaultValue(defaultValue);
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

  return (
    <div className="relative grip-menu hover:bg-white hover:bg-opacity-10 w-4 h-4" data-grip>
      <GripVertical
        size={15}
        className="cursor-pointer"
        onClick={handleClick}
      />
      {showMenu && (
        <Box
          className="MuiMenu-root"
          sx={{
            position: 'absolute',
            top: '100%',
            left: 0,
            mt: 1,
            zIndex: 20,
            bgcolor :"white",
            width: 'auto',
            padding: '8px',
            borderRadius: '4px',
            minWidth: 200,
            display: 'flex',
            flexDirection: 'column',
            gap: 1,
          }}
        >
            <input
              type="text"
              value={localDefaultValue}
              onChange={(e) => handleDefaultValueChange(e.target.value)}
              onBlur={handleDefaultValueBlur}
              onKeyDown={handleDefaultValueKeyDown}
              onClick={(e) => e.stopPropagation()}
              style={{
                width: '100%',
                fontSize: '14px',
                color: '#000',
                border: '1px solid black',
                borderRadius: '4px',
              }}
              placeholder="Default value"
            />
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
                onChange={(e) => setValidationType(e.target.value)}
                onClick={(e) => e.stopPropagation()}
                sx={{
                  fontSize: '14px',
                  height: '20px',
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
                 <MenuItem
                    key={option.value}
                    value={option.value}
                   
                  >
                    {option.label}
                  </MenuItem>
                ))}
              </Select>
            </FormControl>

        </Box>
      )}
    </div>
  );
};

export default GripVerticalMenu;

