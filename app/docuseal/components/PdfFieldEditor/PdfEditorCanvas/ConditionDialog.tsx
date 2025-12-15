import React from 'react';
import {
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Button,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
} from '@mui/material';

interface ConditionDialogProps {
  open: boolean;
  onClose: () => void;
  dependentField: string;
  onDependentFieldChange: (value: string) => void;
  condition: string;
  onConditionChange: (value: string) => void;
  allFields: Array<{ tempId: string; label: string }>;
  currentTempId: string;
  onSave: () => void;
}

const ConditionDialog: React.FC<ConditionDialogProps> = ({
  open,
  onClose,
  dependentField,
  onDependentFieldChange,
  condition,
  onConditionChange,
  allFields = [],
  currentTempId,
  onSave,
}) => {
  const availableFields = allFields.filter(field => field.tempId !== currentTempId);
  return (
    <Dialog open={open} onClose={onClose} maxWidth="sm" fullWidth>
      <DialogTitle>Set Condition</DialogTitle>
      <DialogContent>
        <FormControl fullWidth margin="normal">
          <InputLabel>Dependent Field</InputLabel>
          <Select
            value={dependentField}
            label="Dependent Field"
            onChange={(e) => onDependentFieldChange(e.target.value)}
          >
            {availableFields.map((field) => (
              <MenuItem key={field.tempId} value={field.tempId}>
                {field.label}
              </MenuItem>
            ))}
          </Select>
        </FormControl>
        <FormControl fullWidth margin="normal">
          <InputLabel>Condition</InputLabel>
          <Select
            value={condition}
            label="Condition"
            onChange={(e) => onConditionChange(e.target.value)}
          >
            <MenuItem value="not_empty">Not Empty</MenuItem>
            <MenuItem value="empty">Empty</MenuItem>
          </Select>
        </FormControl>
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose}>Cancel</Button>
        <Button onClick={onSave} variant="contained">
          Save
        </Button>
      </DialogActions>
    </Dialog>
  );
};

export default ConditionDialog;