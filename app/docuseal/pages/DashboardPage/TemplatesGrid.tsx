import React, { useState, useEffect } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { Box, Card, CardContent, Typography, Tooltip, Dialog, DialogTitle, DialogContent, DialogActions, TextField, Button,  Autocomplete } from '@mui/material';
import {
  AccessTime as AccessTimeIcon,
  PictureAsPdf as PictureAsPdfIcon,
  Description as DescriptionIcon,
  Edit as EditIcon,
  ContentCopy as ContentCopyIcon,
  Delete as DeleteIcon,
  DragIndicator as DragIndicatorIcon,
  Person as PersonIcon
} from '@mui/icons-material';
import { motion } from 'framer-motion';
import { Template } from '../../types';
import upstashService from '../../ConfigApi/upstashService';
import toast from 'react-hot-toast';
import { canTemplate } from '@/hooks/useRoleAccess';
import Pagination from '../../components/Pagination';
interface TemplatesGridProps {
  templates: any;
  onRefresh: () => void;
  currentFolderId?: number | null;
}
import { useRoleAccess } from '@/hooks/useRoleAccess';
const TemplatesGrid: React.FC<TemplatesGridProps> = ({ templates, onRefresh, currentFolderId }) => {
  const [hoveredCard, setHoveredCard] = useState<string | number | null>(null);
  const navigate = useNavigate();
  const [showMoveModal, setShowMoveModal] = useState(false);
  const [selectedTemplateId, setSelectedTemplateId] = useState<string | number | null>(null);
  const [folders, setFolders] = useState<any[]>([]);
  const [selectedFolderId, setSelectedFolderId] = useState<number | null>(null);
  const [selectedValue, setSelectedValue] = useState<any>(null);
  const [newFolderName, setNewFolderName] = useState('');
  const hasAccess = useRoleAccess(['agent']);

  const [currentPage, setCurrentPage] = useState(1);
  const itemsPerPage = 12;
  const totalPages = Math.ceil(templates.length / itemsPerPage);
  const currentItems = templates.slice((currentPage - 1) * itemsPerPage, currentPage * itemsPerPage);

  useEffect(() => {
    if (currentPage > totalPages) {
      setCurrentPage(totalPages || 1);
    }
  }, [currentPage, totalPages]);
  useEffect(() => {
    if (showMoveModal) {
      const fetchFolders = async () => {
        try {
          const response = await upstashService.getFolders();
          if (response.success) {
            setFolders(response.data || []);
          } else {
            toast.error('Cannot load folder list');
          }
        } catch (error) {
          console.error('Fetch folders error:', error);
          toast.error('Error loading folders');
        }
      };
      fetchFolders();
    }
  }, [showMoveModal]);

  const handleClone = async (templateId: string | number) => {
    try {
      const response = await upstashService.cloneTemplate(templateId);
      if (response.success) {
        onRefresh();
        toast.success('Template cloned successfully!');
      } else {
        toast.error('Error cloning template: ' + (response.message || 'Unknown error'));
      }
    } catch (error) {
      console.error('Clone template error:', error);
      toast.error('Error cloning template');
    }
  };

  const handleDelete = async (templateId: string | number) => {
    // Show confirmation dialog
    const confirmDelete = window.confirm('Are you sure you want to delete this template? This action cannot be undone.');
    if (!confirmDelete) return;
    try {
      const response = await upstashService.deleteTemplate(templateId);
      if (response.success) {
        onRefresh();
        toast.success('Template deleted successfully!');
      }
    } catch (error: any) {
      console.error('Delete template error:', error);
      toast.error(error?.error || error?.message || 'Failed to delete template');
    }
  };

  const handleSaveMove = async () => {
    const trimmedName = newFolderName ? newFolderName.trim() : '';
    if (!trimmedName && !selectedFolderId) {
      toast.error('Please enter a new folder name or select an existing folder.');
      return;
    }

    try {
      let response;
      if (selectedFolderId) {
        // Move to existing folder
        response = await upstashService.moveTemplatePut(selectedTemplateId, selectedFolderId);
      } else {
        // Create new folder and move
        const body = {
          name: trimmedName || null,
          parent_folder_id: selectedFolderId || (trimmedName ? currentFolderId : null),
          template_id: selectedTemplateId
        };
        response = await upstashService.moveTemplate(body);
      }
      if (response.success || response.status === 200) {
        toast.success('Template moved successfully!');
        onRefresh();
        setShowMoveModal(false);
        setNewFolderName('');
        setSelectedFolderId(null);
        setSelectedValue(null);
      } else {
        toast.error('Error moving template: ' + (response.message || 'Unknown error'));
      }
    } catch (error) {
      console.error('Move template error:', error);
      toast.error('Error moving template');
    }
  };

  const handleMove = (templateId: string | number) => {
    setSelectedTemplateId(templateId);
    setShowMoveModal(true);
  };
  const getFileIcon = (filename: string) => {
    const extension = filename.split('.').pop()?.toLowerCase();
    switch (extension) {
      case 'pdf':
        return PictureAsPdfIcon;
      case 'doc':
      case 'docx':
        return DescriptionIcon;
      case 'jpg':
      case 'jpeg':
      case 'png':
      case 'gif':
      case 'bmp':
      case 'svg':
        return DescriptionIcon;
      case 'txt':
        return DescriptionIcon;
      default:
        return DescriptionIcon;
    }
  };

  const containerVariants = {
    hidden: { opacity: 0 },
    visible: {
      opacity: 1,
      transition: {
        staggerChildren: 0.1,
      },
    },
  };

  const itemVariants = {
    hidden: { y: 20, opacity: 0 },
    visible: {
      y: 0,
      opacity: 1,
      transition: {
        type: "spring" as const,
        stiffness: 100,
      },
    },
  };

  return (
    <motion.div variants={containerVariants} initial="hidden" animate="visible">
      <Box
        sx={{
          display: 'grid',
          gridTemplateColumns: {
            xs: '1fr',
            sm: 'repeat(2, 1fr)',
            md: 'repeat(3, 1fr)',
            lg: 'repeat(4, 1fr)'
          },
          gap: { xs: 2, sm: 3, md: 4 },
          px: { xs: 2, sm: 0 },
        }}
      >
        {currentItems.map(template => (
          <Box key={template.id}>
            <motion.div
              variants={itemVariants}
            >
              <Card
                component={Link}
                to={`/templates/${template.id}`}
                sx={{
                  height: '100%',
                  textDecoration: 'none',
                  background: 'linear-gradient(135deg, rgba(15, 23, 42, 0.9) 0%, rgba(30, 41, 59, 0.8) 100%)',
                  backdropFilter: 'blur(10px)',
                  borderRadius: 4,
                  overflow: 'hidden',
                  position: 'relative',
                  transition: 'all 0.3s ease',
                }}
                onMouseEnter={() => setHoveredCard(template.id)}
                onMouseLeave={() => setHoveredCard(null)}
              >
                <CardContent sx={{ p: { xs: 3, sm: 4 } }}>
                  <Box sx={{ display: 'flex', alignItems: 'flex-start', gap: { xs: 2, sm: 3 }, mb: 3 }}>
                    <Box sx={{
                      p: { xs: 1.5, sm: 2 },
                      borderRadius: 3,
                      background: 'linear-gradient(135deg, #4F46E5 0%, #7C3AED 100%)',
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      flexShrink: 0,
                      boxShadow: '0 8px 24px rgba(79, 70, 229, 0.3)',
                      position: 'relative',
                    }}>
                      {React.createElement(getFileIcon(template.documents[0]?.filename || ''), {
                        sx: { color: 'white', fontSize: { xs: 20, sm: 24 } }
                      })}
                    </Box>
                    <Box sx={{ flex: 1, minWidth: 0 }}>
                      <Typography
                        sx={{
                          color: 'white',
                          fontWeight: '700',
                          mb: 1.5,
                          fontSize: { xs: '1rem', sm: '1rem' },
                          overflow: 'hidden',
                          textOverflow: 'ellipsis',
                          display: '-webkit-box',
                          WebkitLineClamp: 2,
                          WebkitBoxOrient: 'vertical',
                          lineHeight: 1.3
                        }}
                      >
                        {template.name}
                      </Typography>
                    </Box>
                  </Box>
                   <Box sx={{ display: 'flex', alignItems: 'center', color: '#94a3b8', mt: 'auto' }}>
                    <PersonIcon sx={{ fontSize: { xs: 16, sm: 18 }, mr: 1.5, opacity: 0.8 }} />
                    <Typography variant="body2" sx={{ fontWeight: 500, fontSize: { xs: '0.8rem', sm: '0.875rem' } }}>
                        {template.user_name}
                    </Typography>
                  </Box>
                  <Box sx={{ display: 'flex', alignItems: 'center', color: '#94a3b8', mt: 'auto' }}>
                    <AccessTimeIcon sx={{ fontSize: { xs: 16, sm: 18 }, mr: 1.5, opacity: 0.8 }} />
                    <Typography variant="body2" sx={{ fontWeight: 500, fontSize: { xs: '0.8rem', sm: '0.875rem' } }}>
                      {new Date(template.created_at).toLocaleDateString('vi-VN')}
                    </Typography>
                  </Box>
                </CardContent>

                {/* Hover Actions Overlay */}
                {hoveredCard === template.id && !hasAccess && (
                  <Box
                    sx={{
                      position: 'absolute',
                      top: 16,
                      right: 16,
                      display: 'flex',
                      flexDirection: 'column',
                      gap: 1,
                      zIndex: 10,
                    }}
                    onClick={(e) => e.preventDefault()} // Prevent card click when clicking actions
                  >
                    <Tooltip title="Move" placement="left">
                        <DragIndicatorIcon
                            onClick={(e) => {
                            e.preventDefault();
                            handleMove(template.id);
                            }}
                            fontSize="small"
                        />
                    </Tooltip>
                      {canTemplate(template) && (
                        <Tooltip title="Edit" placement="left">
                          <EditIcon
                              onClick={(e) => {
                                  e.preventDefault();
                                  navigate(`/templates/${template.id}/editor`);
                              }}
                              fontSize="small" 
                          />
                        </Tooltip>
                      )}         
                    <Tooltip title="Clone" placement="left">
                      <ContentCopyIcon
                        onClick={(e) => {
                            e.preventDefault();
                              handleClone(template.id);
                            }}
                            fontSize="small"
                      />
                    </Tooltip>

                    {canTemplate(template) && (
                      <Tooltip title="Delete" placement="left">
                        <DeleteIcon 
                            onClick={(e) => {
                            e.preventDefault();
                                  handleDelete(template.id);
                            }}
                            fontSize="small"
                         />
                      </Tooltip>
                    )}
                  </Box>
                )}
              </Card>
            </motion.div>
          </Box>
        ))}
      </Box>
        {currentItems.length > 0 &&  (
           <Pagination
              currentPage={currentPage}
              totalPages={totalPages}
              onPageChange={setCurrentPage}
            />
        )}
      

      <Dialog
        open={showMoveModal} 
        onClose={() => {
          setShowMoveModal(false);
          setNewFolderName('');
          setSelectedFolderId(null);
          setSelectedValue(null);
        }} 
        maxWidth="sm" fullWidth
       >
        <DialogTitle>Move Template</DialogTitle>
        <DialogContent>
          <Autocomplete
            value={selectedValue}
            onChange={(event, newValue) => {
              if (typeof newValue === 'string') {
                setNewFolderName(newValue.trim());
                setSelectedFolderId(null);
                setSelectedValue(newValue.trim());
              } else if (newValue) {
                setNewFolderName(null); // Khi chọn folder, name = null
                setSelectedFolderId(newValue.id);
                setSelectedValue(newValue);
              } else {
                setNewFolderName('');
                setSelectedFolderId(null);
                setSelectedValue(null);
              }
            }}
            onInputChange={(event, newInputValue) => {
              setNewFolderName(newInputValue.trim());
              setSelectedValue(newInputValue.trim());
              if (newInputValue.trim()) {
                setSelectedFolderId(null);
              }
            }}
            options={folders.map(f => ({ id: f.id, name: f.name }))}
            getOptionLabel={(option) => typeof option === 'string' ? option : option.name}
            freeSolo
            sx={{ 
              '& .MuiAutocomplete-listbox': { backgroundColor: '#424242' },
              '& .MuiAutocomplete-paper': { backgroundColor: '#424242' }
            }}
            slotProps={{
              listbox: {
                sx: { backgroundColor: '#424242' }
              },
              paper: {
                sx: { backgroundColor: '#424242' }
              }
            }}
            renderInput={(params) => (
              <TextField
                {...params}
                label="Folder Name or Select Existing Folder"
                fullWidth
                variant="outlined"
                sx={{ mt: 2 }}
              />
            )}
          />
        </DialogContent>
        <DialogActions>
          <Button 
            sx={{ color: 'white' }}
            onClick={() => {
            setNewFolderName('');
            setSelectedFolderId(null);
            setSelectedValue(null);
            setShowMoveModal(false);
          }}>Cancel</Button>
          <Button onClick={handleSaveMove} variant="contained">Save</Button>
        </DialogActions>
      </Dialog>
    </motion.div>
  );
};

export default TemplatesGrid;