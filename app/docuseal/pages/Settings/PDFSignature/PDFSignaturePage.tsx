import { useState, useEffect } from 'react';
import { 
  Box, 
  Typography, 
  Paper, 
  Button, 
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Switch,
  FormControl,
  Select,
  MenuItem,
  IconButton,
  Alert,
  CircularProgress
} from '@mui/material';
import { 
  CloudUpload, 
  Add, 
  Delete,
  VerifiedUser 
} from '@mui/icons-material';
import { useDropzone } from 'react-dropzone';
import toast from 'react-hot-toast';

interface Certificate {
  id: number;
  name: string;
  certificate_type: string;
  issuer?: string;
  subject?: string;
  serial_number?: string;
  valid_from?: string;
  valid_to?: string;
  status: 'active' | 'expired' | 'revoked';
  fingerprint?: string;
  created_at: string;
}

interface PDFSignatureSettings {
  flattenForm: boolean;
  filenameFormat: string;
}

const PDFSignaturePage = () => {
  const [certificates, setCertificates] = useState<Certificate[]>([]);
  const [settings, setSettings] = useState<PDFSignatureSettings>({
    flattenForm: false,
    filenameFormat: 'document-name-signed'
  });
  const [verifyLoading, setVerifyLoading] = useState(false);
  const [uploadLoading, setUploadLoading] = useState(false);
  const [verificationResult, setVerificationResult] = useState<{
    valid: boolean;
    message: string;
    details?: any;
    fileName?: string;
  } | null>(null);
  const [uploadedFileName, setUploadedFileName] = useState<string | null>(null);

  // Load certificates and settings on mount
  useEffect(() => {
    const loadData = async () => {
      try {
        const token = localStorage.getItem('token');
        // Load certificates
        const certsResponse = await fetch('/api/pdf-signature/certificates', {
          headers: {
            'Authorization': `Bearer ${token}`
          }
        });
        if (certsResponse.ok) {
          const certsResult = await certsResponse.json();
          if (certsResult.data) {
            setCertificates(certsResult.data);
          }
        }

        // Load settings
        const settingsResponse = await fetch('/api/pdf-signature/settings', {
          headers: {
            'Authorization': `Bearer ${token}`
          }
        });
        if (settingsResponse.ok) {
          const settingsResult = await settingsResponse.json();
          if (settingsResult.data) {
            setSettings({
              flattenForm: settingsResult.data.flatten_form || false,
              filenameFormat: settingsResult.data.filename_format || 'document-name-signed'
            });
          }
        }
      } catch (error) {
        console.error('Failed to load data:', error);
      }
    };

    loadData();
  }, []);

  // Dropzone for PDF verification
  const { getRootProps: getVerifyRootProps, getInputProps: getVerifyInputProps, isDragActive: isVerifyDragActive } = useDropzone({
    accept: {
      'application/pdf': ['.pdf']
    },
    maxFiles: 1,
    onDrop: async (acceptedFiles) => {
      if (acceptedFiles.length > 0) {
        await handleVerifyPDF(acceptedFiles[0]);
      }
    }
  });

  const handleVerifyPDF = async (file: File) => {
    setVerifyLoading(true);
    setVerificationResult(null);
    setUploadedFileName(file.name); // Set uploaded file name immediately

    try {
      const formData = new FormData();
      formData.append('pdf', file);

      const token = localStorage.getItem('token');
      const response = await fetch('/api/pdf-signature/verify', {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${token}`
        },
        body: formData
      });

      if (!response.ok) {
        const text = await response.text();
        let errorMsg = 'Verification failed';
        try {
          const errorData = JSON.parse(text);
          errorMsg = errorData.error || errorData.message || errorMsg;
        } catch {
          errorMsg = text || errorMsg;
        }
        throw new Error(errorMsg);
      }

      const result = await response.json();
      
      // Extract data from ApiResponse wrapper
      const verificationData = result.data || result;
      
      setVerificationResult({
        valid: verificationData.valid,
        message: verificationData.message || result.message || (verificationData.valid 
          ? 'PDF signature is valid ✓' 
          : 'PDF signature is invalid or not signed'),
        details: verificationData.details,
        fileName: file.name
      });
      toast.success(verificationData.valid ? 'Signature verified!' : 'No valid signature found');
    } catch (error: any) {
      console.error('Verification error:', error);
      toast.error(error.message || 'Failed to verify PDF');
    } finally {
      setVerifyLoading(false);
    }
  };

//   const handleUploadCertificate = async (files: File[]) => {
//     if (files.length === 0) return;

//     setUploadLoading(true);
//     try {
//       const formData = new FormData();
//       formData.append('certificate', files[0]);

//       const token = localStorage.getItem('token');
//       const response = await fetch('/api/pdf-signature/certificates', {
//         method: 'POST',
//         headers: {
//           'Authorization': `Bearer ${token}`
//         },
//         body: formData
//       });

//       if (!response.ok) {
//         const text = await response.text();
//         let errorMsg = 'Upload failed';
//         try {
//           const errorData = JSON.parse(text);
//           errorMsg = errorData.error || errorData.message || errorMsg;
//         } catch {
//           errorMsg = text || errorMsg;
//         }
//         throw new Error(errorMsg);
//       }

//       const result = await response.json();
//       if (result.data) {
//         setCertificates(prev => [...prev, result.data]);
//         toast.success(result.message || 'Certificate uploaded successfully');
//       }
//     } catch (error: any) {
//       console.error('Upload error:', error);
//       toast.error(error.message || 'Failed to upload certificate');
//     } finally {
//       setUploadLoading(false);
//     }
//   };

//   const handleDeleteCertificate = async (id: number) => {
//     if (!confirm('Are you sure you want to delete this certificate?')) return;

//     try {
//       const token = localStorage.getItem('token');
//       const response = await fetch(`/api/pdf-signature/certificates/${id}`, {
//         method: 'DELETE',
//         headers: {
//           'Authorization': `Bearer ${token}`
//         }
//       });

//       if (!response.ok) {
//         const text = await response.text();
//         let errorMsg = 'Delete failed';
//         try {
//           const errorData = JSON.parse(text);
//           errorMsg = errorData.error || errorData.message || errorMsg;
//         } catch {
//           errorMsg = text || errorMsg;
//         }
//         throw new Error(errorMsg);
//       }

//       setCertificates(prev => prev.filter(cert => cert.id !== id));
//       toast.success('Certificate deleted');
//     } catch (error: any) {
//       console.error('Delete error:', error);
//       toast.error(error.message || 'Failed to delete certificate');
//     }
//   };

//   const handleSettingsChange = async (key: keyof PDFSignatureSettings, value: any) => {
//     const newSettings = { ...settings, [key]: value };
//     setSettings(newSettings);

//     try {
//       const token = localStorage.getItem('token');
//       const response = await fetch('/api/pdf-signature/settings', {
//         method: 'PUT',
//         headers: {
//           'Content-Type': 'application/json',
//           'Authorization': `Bearer ${token}`
//         },
//         body: JSON.stringify(newSettings)
//       });

//       if (!response.ok) {
//         const text = await response.text();
//         let errorMsg = 'Failed to save settings';
//         try {
//           const errorData = JSON.parse(text);
//           errorMsg = errorData.error || errorData.message || errorMsg;
//         } catch {
//           errorMsg = text || errorMsg;
//         }
//         throw new Error(errorMsg);
//       }

//       const result = await response.json();
//       toast.success(result.message || 'Settings saved');
//     } catch (error: any) {
//       console.error('Settings error:', error);
//       toast.error(error.message || 'Failed to save settings');
//     }
//   };

  return (
    <Box sx={{ p: 3 }}>
      <Typography variant="h4" sx={{ mb: 4, fontWeight: 600 }}>
        PDF Signature
      </Typography>

      {/* PDF Verification Section */}
      <Paper sx={{ 
        p: 3, 
        mb: 4, 
        bgcolor: 'rgba(13, 7, 31, 0.5)', 
        borderRadius: 2,
        border: '1px solid rgba(255, 255, 255, 0.1)'
      }}>
        <Typography variant="subtitle1" sx={{ mb: 2, fontWeight: 500 }}>
          Upload signed PDF file to validate its signature:
        </Typography>

        <Box
          {...getVerifyRootProps()}
          sx={{
            border: '2px dashed',
            borderColor: isVerifyDragActive ? 'primary.main' : 'rgba(255, 255, 255, 0.3)',
            borderRadius: 2,
            p: 4,
            textAlign: 'center',
            cursor: 'pointer',
            bgcolor: isVerifyDragActive ? 'rgba(79, 70, 229, 0.1)' : 'transparent',
            transition: 'all 0.3s',
            '&:hover': {
              borderColor: 'primary.main',
              bgcolor: 'rgba(79, 70, 229, 0.05)'
            }
          }}
        >
          <input {...getVerifyInputProps()} />
          <CloudUpload sx={{ fontSize: 48, color: 'primary.main', mb: 2 }} />
          <Typography variant="h6" sx={{ mb: 1 }}>
            Verify Signed PDF
          </Typography>
          <Typography variant="body2" color="text.secondary">
            {uploadedFileName ? (
              <Box>
                <Typography variant="body2" sx={{ fontWeight: 500 }}>
                  ✓ {uploadedFileName}
                </Typography>
              </Box>
            ) : (
               <Typography variant="body2" sx={{ color: 'text.secondary', mt: 0.5 }}>
                  Click to upload another file or drag and drop
                </Typography>
            )}
          </Typography>
        </Box>

        {verifyLoading && (
          <Box sx={{ display: 'flex', justifyContent: 'center', mt: 3 }}>
            <CircularProgress />
          </Box>
        )}

        {verificationResult && (
          <Alert 
            severity={verificationResult.valid ? 'success' : 'error'} 
            sx={{ mt: 3 }}
            icon={verificationResult.valid ? <VerifiedUser /> : undefined}
          >
            <Typography variant="body1" sx={{ fontWeight: 500, mb: 1 }}>
              {verificationResult.message}
            </Typography>
            {verificationResult.details && (
              <Box sx={{ mt: 2 }}>
                {/* Signer Information */}
                {verificationResult.details.common_name && (
                  <Box sx={{ mb: 1.5 }}>
                    <Typography variant="caption" sx={{ color: 'text.secondary', display: 'block' }}>
                      Signer:
                    </Typography>
                    <Typography variant="body2" sx={{ fontWeight: 600 }}>
                      {verificationResult.details.common_name}
                    </Typography>
                  </Box>
                )}

                {/* Signing Time */}
                {verificationResult.details.signing_time && (
                  <Box sx={{ mb: 1.5 }}>
                    <Typography variant="caption" sx={{ color: 'text.secondary', display: 'block' }}>
                      Signing Time:
                    </Typography>
                    <Typography variant="body2">
                      {new Date(verificationResult.details.signing_time).toLocaleString('vi-VN')}
                    </Typography>
                  </Box>
                )}

                {/* Reason */}
                {verificationResult.details.reason && (
                  <Box sx={{ mb: 1.5 }}>
                    <Typography variant="caption" sx={{ color: 'text.secondary', display: 'block' }}>
                      Reason:
                    </Typography>
                    <Typography variant="body2">
                      {verificationResult.details.reason}
                    </Typography>
                  </Box>
                )}

                {/* Signature Type */}
                {verificationResult.details.signature_type && (
                  <Box sx={{ mb: 1.5 }}>
                    <Typography variant="caption" sx={{ color: 'text.secondary', display: 'block' }}>
                      Signature Type:
                    </Typography>
                    <Typography variant="body2">
                      {verificationResult.details.signature_type}
                    </Typography>
                  </Box>
                )}

                {/* Signature Technical Details */}
                {(verificationResult.details.signature_filter || verificationResult.details.signature_subfilter || verificationResult.details.signature_format) && (
                  <Box sx={{ mb: 1.5 }}>
                    <Typography variant="caption" sx={{ color: 'text.secondary', display: 'block' }}>
                      Technical Details:
                    </Typography>
                    <Typography variant="body2" component="div">
                      {verificationResult.details.signature_filter && (
                        <div>• Filter: {verificationResult.details.signature_filter}</div>
                      )}
                      {verificationResult.details.signature_subfilter && (
                        <div>• SubFilter: {verificationResult.details.signature_subfilter}</div>
                      )}
                      {verificationResult.details.signature_format && (
                        <div>• Format: {verificationResult.details.signature_format}</div>
                      )}
                    </Typography>
                  </Box>
                )}

                {/* Certificate Info (collapsed by default) */}
                {verificationResult.details.certificate_info && (
                  <Box sx={{ mt: 2 }}>
                    <details>
                      <summary style={{ cursor: 'pointer', fontSize: '0.875rem'}}>
                        Certificate Details
                      </summary>
                      <Box sx={{ mt: 1, pl: 2 }}>
                        {verificationResult.details.certificate_info.common_name && (
                          <Typography variant="caption" component="div" sx={{ mb: 0.5 }}>
                            <strong>Email/Common Name:</strong> {verificationResult.details.certificate_info.common_name}
                          </Typography>
                        )}
                        {verificationResult.details.certificate_info.issuer && (
                          <Typography variant="caption" component="div" sx={{ mb: 0.5 }}>
                            <strong>Issuer:</strong> {verificationResult.details.certificate_info.issuer}
                          </Typography>
                        )}
                        {verificationResult.details.certificate_info.subject && (
                          <Typography variant="caption" component="div" sx={{ mb: 0.5 }}>
                            <strong>Subject:</strong> {verificationResult.details.certificate_info.subject}
                          </Typography>
                        )}
                        {verificationResult.details.certificate_info.valid_from && (
                          <Typography variant="caption" component="div" sx={{ mb: 0.5 }}>
                            <strong>Valid From:</strong> {new Date(verificationResult.details.certificate_info.valid_from).toLocaleString('vi-VN')}
                          </Typography>
                        )}
                        {verificationResult.details.certificate_info.valid_to && (
                          <Typography variant="caption" component="div">
                            <strong>Valid To:</strong> {new Date(verificationResult.details.certificate_info.valid_to).toLocaleString('vi-VN')}
                          </Typography>
                        )}
                      </Box>
                    </details>
                  </Box>
                )}
              </Box>
            )}
          </Alert>
        )}
      </Paper>

      {/* Signing Certificates Section */}
      {/* <Paper sx={{ 
        p: 3, 
        mb: 4,
        bgcolor: 'rgba(13, 7, 31, 0.5)', 
        borderRadius: 2,
        border: '1px solid rgba(255, 255, 255, 0.1)'
      }}>
        <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 3 }}>
          <Typography variant="h6" sx={{ fontWeight: 600 }}>
            Signing Certificates
          </Typography>
          <Button
            variant="contained"
            startIcon={uploadLoading ? <CircularProgress size={20} /> : <Add />}
            component="label"
            disabled={uploadLoading}
          >
            UPLOAD CERT
            <input
              type="file"
              hidden
              accept=".p12,.pfx,.pem,.crt"
              onChange={(e) => e.target.files && handleUploadCertificate(Array.from(e.target.files))}
            />
          </Button>
        </Box>

        <Alert severity="info" sx={{ mb: 3 }}>
          <Typography variant="body2" sx={{ fontWeight: 600 }}>
            Unlock with DocuSeal Pro
          </Typography>
          <Typography variant="body2">
            Use your own certificates to sign and verify PDF files.
          </Typography>
          <Button 
            variant="text" 
            size="small" 
            sx={{ mt: 1, p: 0, textTransform: 'none' }}
          >
            Learn More
          </Button>
        </Alert>

        <TableContainer>
          <Table>
            <TableHead>
              <TableRow>
                <TableCell sx={{ color: 'text.secondary', fontWeight: 600 }}>NAME</TableCell>
                <TableCell sx={{ color: 'text.secondary', fontWeight: 600 }}>VALID TO</TableCell>
                <TableCell sx={{ color: 'text.secondary', fontWeight: 600 }}>STATUS</TableCell>
                <TableCell sx={{ color: 'text.secondary', fontWeight: 600 }} align="right">ACTIONS</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {certificates.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={4} align="center" sx={{ py: 4, color: 'text.secondary' }}>
                    No certificates uploaded yet
                  </TableCell>
                </TableRow>
              ) : (
                certificates.map((cert) => (
                  <TableRow key={cert.id}>
                    <TableCell>{cert.name}</TableCell>
                    <TableCell>
                      {cert.valid_to ? new Date(cert.valid_to).toLocaleDateString() : 'N/A'}
                    </TableCell>
                    <TableCell>
                      <Box
                        component="span"
                        sx={{
                          px: 1.5,
                          py: 0.5,
                          borderRadius: 1,
                          fontSize: '0.75rem',
                          fontWeight: 600,
                          bgcolor: 
                            cert.status === 'active' ? 'success.dark' :
                            cert.status === 'expired' ? 'error.dark' : 'warning.dark',
                          color: 'white'
                        }}
                      >
                        {cert.status.toUpperCase()}
                      </Box>
                    </TableCell>
                    <TableCell align="right">
                      <IconButton 
                        size="small" 
                        color="error"
                        onClick={() => handleDeleteCertificate(cert.id)}
                      >
                        <Delete />
                      </IconButton>
                    </TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        </TableContainer>
      </Paper> */}

      {/* Preferences Section */}
      {/* <Paper sx={{ 
        p: 3,
        bgcolor: 'rgba(13, 7, 31, 0.5)', 
        borderRadius: 2,
        border: '1px solid rgba(255, 255, 255, 0.1)'
      }}>
        <Typography variant="h6" sx={{ mb: 3, fontWeight: 600 }}>
          Preferences
        </Typography>

        <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 3 }}>
          <Box>
            <Typography variant="body1" sx={{ fontWeight: 500 }}>
              Remove PDF form fillable fields from the signed PDF (flatten form)
            </Typography>
          </Box>
          <Switch
            checked={settings.flattenForm}
            onChange={(e) => handleSettingsChange('flattenForm', e.target.checked)}
            color="primary"
          />
        </Box>

        <Box>
          <Typography variant="body1" sx={{ fontWeight: 500, mb: 2 }}>
            Document download filename format
          </Typography>
          <FormControl fullWidth>
            <Select
              value={settings.filenameFormat}
              onChange={(e) => handleSettingsChange('filenameFormat', e.target.value)}
              sx={{
                bgcolor: 'rgba(255, 255, 255, 0.05)',
                '& .MuiOutlinedInput-notchedOutline': {
                  borderColor: 'rgba(255, 255, 255, 0.2)'
                }
              }}
            >
              <MenuItem value="document-name-signed">Document Name - Signed.pdf</MenuItem>
              <MenuItem value="document-name">Document Name.pdf</MenuItem>
              <MenuItem value="document-name-date">Document Name - {new Date().toLocaleDateString()}.pdf</MenuItem>
            </Select>
          </FormControl>
        </Box>
      </Paper> */}
    </Box>
  );
};

export default PDFSignaturePage;
