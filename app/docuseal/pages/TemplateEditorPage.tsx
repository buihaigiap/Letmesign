import React, { useState, useEffect, useCallback, useRef } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useAuth } from '../contexts/AuthContext';
import { Template } from '../types';
import DocumentEditor from '../components/PdfFieldEditor/index';
import upstashService from '../ConfigApi/upstashService';
import { Box, Button, CircularProgress, Typography, Alert } from '@mui/material';
import { ArrowBack as ArrowBackIcon, Save as SaveIcon } from '@mui/icons-material';
import toast from 'react-hot-toast';
import { canTemplate , useRoleAccess } from '../hooks/useRoleAccess';
import CreateTemplateButton from '../components/CreateTemplateButton';
const TemplateEditorPage = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { token } = useAuth();
  const [template, setTemplate] = useState<Template | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [saving, setSaving] = useState(false);
  const editorRef = useRef<any>(null);
  const hasAccess = useRoleAccess(['agent']);

  const fetchTemplate = useCallback(async () => {
    if (!id || !token) return;
    setLoading(true);
    try {
      const data = await upstashService.getTemplateFullInfo(parseInt(id));
      if (data.success) {
        const templateData = data.data;
        const template: Template = {
          id: templateData.template.id,
          name: templateData.template.name,
          file_url: templateData.template.documents?.[0]?.url || '',
          documents: templateData.template.documents,
          created_at: templateData.template.created_at,
          user_id: templateData.template.user_id,
          slug: templateData.template.slug,
          updated_at: templateData.template.updated_at,
          fields: templateData.template.template_fields.map((f: any) => ({
            id: f.id,
            name: f.name,
            field_type: f.field_type,
            required: f.required,
            position: f.position,
            display_order: f.display_order,
            options: f.options,
            partner: f.partner
          })),
          user_name: ''
        };

        setTemplate(template);
      } else {
        setError(data.message || 'Failed to fetch template details.');
      }
    } catch (err) {
      console.error('Template fetch error:', err);
      setError('An unexpected error occurred while fetching template.');
    } finally {
      setLoading(false);
    }
  }, [id, token]);

  useEffect(() => {
    fetchTemplate();
  }, [fetchTemplate]);

  const handleSave = async () => {
    if (!editorRef.current) return;

    // Validation: Check if each partner has at least one field
    const partners = editorRef.current.getPartners ? editorRef.current.getPartners() : [];
    const fields = editorRef.current.getFields ? editorRef.current.getFields() : [];

    for (const partner of partners) {
      const hasField = fields.some((field: any) => field.partner === partner);
      if (!hasField) {
        toast.error(`Please add fields for the ${partner}. Or, remove the ${partner} if not needed.`);
        return;
      }
    }

    setSaving(true);
    try {
      const result = await editorRef.current.saveFields();
      if (result.success) {
        toast.success('Template saved successfully!');
        navigate(-1);
      } 
    } catch (error) {
      console.error('Save error:', error);
    } finally {
      setSaving(false);
    }
  };

  const handleBack = () => {
    navigate(-1);
  };

  if (loading) {
    return (
      <Box sx={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        minHeight: '100vh',
        gap: 2
      }}>
        <CircularProgress size={60} />
        <Typography variant="h6">Loading template editor...</Typography>
      </Box>
    );
  }

  if (error) {
    return (
      <Box sx={{
        maxWidth: 800,
        mx: 'auto',
        mt: 4,
        p: 3
      }}>
        <Alert severity="error" sx={{ mb: 2 }}>
          {error}
        </Alert>
        <Button
          startIcon={<ArrowBackIcon />}
          onClick={handleBack}
          variant="outlined"
        >
          Back to Template
        </Button>
      </Box>
    );
  }

  if (!template) {
    return (
      <Box sx={{
        maxWidth: 800,
        mx: 'auto',
        mt: 4,
        p: 3
      }}>
        <Alert severity="warning">
          Template not found
        </Alert>
        <Button
          startIcon={<ArrowBackIcon />}
          onClick={handleBack}
          variant="outlined"
          sx={{ mt: 2 }}
        >
          Back to Dashboard
        </Button>
      </Box>
    );
  }

  return (
    <Box sx={{
      color: 'white'
    }}>
      {/* Header */}
      <Box sx={{
        px: { xs: 2, sm: 3, md: 4 },
        py: 2
      }}>
        <Box sx={{
          // maxWidth: 1400,
          mx: 'auto',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          flexWrap: 'wrap',
          gap: 2
        }}>
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 2 }}>
            <CreateTemplateButton
              onClick={handleBack}
              text="Back"
              icon={<ArrowBackIcon />}
              width="100px"
            />
            {/* <Button
              startIcon={<ArrowBackIcon />}
              onClick={handleBack}
              variant="outlined"
            >
              Back
            </Button> */}
            <Typography variant="h5" component="h1" fontWeight="600">
              Editing: {template.name}
            </Typography>
          </Box>
          {canTemplate(template) && !hasAccess && (
             <Button
                startIcon={<SaveIcon />}
                onClick={handleSave}
                variant="contained"
                disabled={saving}
                sx={{
                  background: 'linear-gradient(135deg, #4F46E5 0%, #7C3AED 100%)',
                  '&:hover': {
                    background: 'linear-gradient(135deg, #4338CA 0%, #6D28D9 100%)'
                  },
                  minWidth: 120
            }}
          >
            {saving ? 'Saving...' : 'Save Changes'}
          </Button>
          )}
        </Box>
      </Box>

      {/* Editor Content */}
      <Box sx={{
        // py: 4
      }}>
        {React.createElement(DocumentEditor as any, {
          ref: editorRef,
          template: template,
          token: token || ''
        })}
      </Box>
    </Box>
  );
};

export default TemplateEditorPage;