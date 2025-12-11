import React from 'react';
import {
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Button,
  TextField,
} from '@mui/material';
import CreateTemplateButton from '../../CreateTemplateButton';
interface DescriptionDialogProps {
  open: boolean;
  onClose: () => void;
  displayTitle: string;
  onDisplayTitleChange: (value: string) => void;
  description: string;
  onDescriptionChange: (value: string) => void;
  onSave: () => void;
}

const DescriptionDialog: React.FC<DescriptionDialogProps> = ({
  open,
  onClose,
  displayTitle,
  onDisplayTitleChange,
  description,
  onDescriptionChange,
  onSave,
}) => {
  return (
    <Dialog open={open} onClose={onClose} maxWidth="sm" fullWidth>
      <DialogTitle>Edit Description</DialogTitle>
      <DialogContent>
        <TextField
          label="Display Title"
          value={displayTitle}
          onChange={(e) => onDisplayTitleChange(e.target.value)}
          fullWidth
          margin="normal"
          autoFocus
        />
        <TextField
          label="Description"
          value={description}
          onChange={(e) => onDescriptionChange(e.target.value)}
          fullWidth
          multiline
          rows={4}
          margin="normal"
        />
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose} sx={{color :"white"}}>Cancel</Button>
        <CreateTemplateButton
            onClick={onSave}
            text="Save"
        />
      </DialogActions>
    </Dialog>
  );
};

export default DescriptionDialog;