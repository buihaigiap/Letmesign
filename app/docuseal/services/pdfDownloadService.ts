import { PDFDocument, rgb, StandardFonts } from 'pdf-lib';
import { hashId } from '../constants/reminderDurations';
import { getPDFPreferences, applyFilenameFormat } from './filenameFormatter';

// Helper function to sanitize text for PDF operations by replacing Unicode characters
// that cannot be encoded in WinAnsi with ASCII equivalents
const sanitizeTextForPDF = (text: string): string => {
  if (!text) return text;

  return text
    // Vietnamese characters
    .replace(/ơ/g, 'o')
    .replace(/ư/g, 'u')
    .replace(/ă/g, 'a')
    .replace(/â/g, 'a')
    .replace(/ê/g, 'e')
    .replace(/ô/g, 'o')
    .replace(/ư/g, 'u')
    .replace(/đ/g, 'd')
    .replace(/ĩ/g, 'i')
    .replace(/ũ/g, 'u')
    .replace(/ễ/g, 'e')
    .replace(/ẫ/g, 'a')
    .replace(/ỗ/g, 'o')
    .replace(/ừ/g, 'u')
    .replace(/Ở/g, 'O')
    .replace(/ờ/g, 'o')
    .replace(/ở/g, 'o')
    .replace(/ỡ/g, 'o')
    .replace(/ợ/g, 'o')
    .replace(/Ư/g, 'U')
    .replace(/Ứ/g, 'U')
    .replace(/ứ/g, 'u')
    .replace(/ừ/g, 'u')
    .replace(/ử/g, 'u')
    .replace(/ữ/g, 'u')
    .replace(/ự/g, 'u')
    .replace(/Ă/g, 'A')
    .replace(/Ắ/g, 'A')
    .replace(/ắ/g, 'a')
    .replace(/ằ/g, 'a')
    .replace(/ẳ/g, 'a')
    .replace(/ẵ/g, 'a')
    .replace(/ặ/g, 'a')
    .replace(/Â/g, 'A')
    .replace(/Ấ/g, 'A')
    .replace(/ấ/g, 'a')
    .replace(/ầ/g, 'a')
    .replace(/ẩ/g, 'a')
    .replace(/ẫ/g, 'a')
    .replace(/ậ/g, 'a')
    .replace(/Ê/g, 'E')
    .replace(/Ế/g, 'E')
    .replace(/ế/g, 'e')
    .replace(/ề/g, 'e')
    .replace(/ể/g, 'e')
    .replace(/ễ/g, 'e')
    .replace(/ệ/g, 'e')
    .replace(/Ô/g, 'O')
    .replace(/Ố/g, 'O')
    .replace(/ố/g, 'o')
    .replace(/ồ/g, 'o')
    .replace(/ổ/g, 'o')
    .replace(/ỗ/g, 'o')
    .replace(/ộ/g, 'o')
    .replace(/Ơ/g, 'O')
    .replace(/Ớ/g, 'O')
    .replace(/ớ/g, 'o')
    .replace(/ờ/g, 'o')
    .replace(/ở/g, 'o')
    .replace(/ỡ/g, 'o')
    .replace(/ợ/g, 'o')
    .replace(/Đ/g, 'D')
    .replace(/Ĩ/g, 'I')
    .replace(/Ũ/g, 'U')
    .replace(/Ễ/g, 'E')
    .replace(/Ẫ/g, 'A')
    .replace(/Ỗ/g, 'O')
    .replace(/Ừ/g, 'U')
    // Additional basic accented characters
    .replace(/à/g, 'a')
    .replace(/á/g, 'a')
    .replace(/ã/g, 'a')
    .replace(/è/g, 'e')
    .replace(/é/g, 'e')
    .replace(/ì/g, 'i')
    .replace(/í/g, 'i')
    .replace(/ò/g, 'o')
    .replace(/ó/g, 'o')
    .replace(/õ/g, 'o')
    .replace(/ù/g, 'u')
    .replace(/ú/g, 'u')
    .replace(/ü/g, 'u')
    .replace(/À/g, 'A')
    .replace(/Á/g, 'A')
    .replace(/Ã/g, 'A')
    .replace(/È/g, 'E')
    .replace(/É/g, 'E')
    .replace(/Ì/g, 'I')
    .replace(/Í/g, 'I')
    .replace(/Ò/g, 'O')
    .replace(/Ó/g, 'O')
    .replace(/Õ/g, 'O')
    .replace(/Ù/g, 'U')
    .replace(/Ú/g, 'U')
    .replace(/Ü/g, 'U')
    // Other common Unicode characters that might cause issues
    .replace(/[^\x00-\x7F]/g, '?'); // Replace any remaining non-ASCII characters with ?
};

// Interface for audit log entry
interface AuditLogEntry {
  timestamp: string;
  action: string;
  user: string;
  details?: string;
  ip?: string;
  user_agent?: string;
  session_id?: string;
  timezone?: string;
}

// Fetch real audit log from backend
export const fetchAuditLog = async (
  submitterToken: string
): Promise<AuditLogEntry[]> => {
  try {
    const API_BASE_URL = (import.meta as any).env?.VITE_API_BASE_URL || '';
    const response = await fetch(`${API_BASE_URL}/api/submitters/${submitterToken}/audit-log`, {
      headers: {
        'Authorization': localStorage.getItem('token') ? `Bearer ${localStorage.getItem('token')}` : ''
      }
    });

    if (!response.ok) {
      console.warn('Failed to fetch audit log, using fallback');
      return [];
    }

    const data = await response.json();
    // Filter out envelope_info object, keep only audit events
    const entries = (data.data || []).filter((entry: any) => entry.type !== 'envelope_info');
    return entries;
  } catch (error) {
    console.error('Error fetching audit log:', error);
    return [];
  }
};

// Helper function to generate mock audit log for testing (fallback)
// TODO: Remove this after backend API is ready
export const generateMockAuditLog = (
  submitterEmail: string,
  templateName: string
): AuditLogEntry[] => {
  return [
    {
      timestamp: new Date(Date.now() - 3600000).toLocaleString('vi-VN', { timeZone: 'Asia/Ho_Chi_Minh' }),
      action: 'Document Created',
      user: submitterEmail || 'Unknown User',
      details: `Template "${templateName}" was uploaded and fields were configured`
    },
    {
      timestamp: new Date(Date.now() - 1800000).toLocaleString('vi-VN', { timeZone: 'Asia/Ho_Chi_Minh' }),
      action: 'Document Sent',
      user: 'System',
      details: `Document sent to ${submitterEmail} for signature`
    },
    {
      timestamp: new Date().toLocaleString('vi-VN', { timeZone: 'Asia/Ho_Chi_Minh' }),
      action: 'Document Signed',
      user: submitterEmail,
      details: `All required fields completed and document submitted successfully`
    }
  ];
};

// Helper function to render vector signature to canvas and convert to image
export const renderSignatureToImage = (signatureData: string, width: number, height: number, options?: {
  submitterId?: number;
  submitterEmail?: string;
  reason?: string;
  additionalText?: string;
  globalSettings?: any;
}): Promise<string> => {
  return new Promise((resolve, reject) => {
    try {

      // Use provided dimensions (already clamped by caller)
      const canvasWidth = Math.round(width);
      const canvasHeight = Math.round(height);

      // Safety check: ensure dimensions are reasonable
      if (canvasWidth > 2000 || canvasHeight > 2000 || canvasWidth < 50 || canvasHeight < 50) {
        console.warn('Canvas dimensions out of range, using defaults:', canvasWidth, canvasHeight);
        reject(new Error(`Invalid canvas dimensions: ${canvasWidth}x${canvasHeight}`));
        return;
      }

      const canvas = document.createElement('canvas');
      canvas.width = canvasWidth;
      canvas.height = canvasHeight;
      const ctx = canvas.getContext('2d');

      if (!ctx) {
        reject(new Error('Cannot get canvas context'));
        return;
      }

      // Parse signature data
      const pointGroups = JSON.parse(signatureData);


      if (!pointGroups || pointGroups.length === 0) {
        reject(new Error('Empty signature data'));
        return;
      }

      // Clear canvas WITHOUT background (transparent)
      ctx.clearRect(0, 0, canvasWidth, canvasHeight);

      // Find bounds of signature to scale it properly
      let minX = Infinity, minY = Infinity;
      let maxX = -Infinity, maxY = -Infinity;

      pointGroups.forEach((group: any[]) => {
        group.forEach((point: any) => {
          minX = Math.min(minX, point.x);
          minY = Math.min(minY, point.y);
          maxX = Math.max(maxX, point.x);
          maxY = Math.max(maxY, point.y);
        });
      });


      const signatureWidth = maxX - minX;
      const signatureHeight = maxY - minY;

      if (signatureWidth <= 0 || signatureHeight <= 0) {
        reject(new Error('Invalid signature dimensions'));
        return;
      }


      // Calculate text height dynamically (giống SignatureRenderer)
      let textHeight = 0;
      if (options?.globalSettings?.add_signature_id_to_the_documents || (options?.globalSettings?.require_signing_reason && options?.reason)) {
        // Estimate text height: 12px per line + 6px padding
        let lineCount = 0;
        if (options?.globalSettings?.add_signature_id_to_the_documents) {
          lineCount += (options?.submitterId ? 1 : 0) + (options?.submitterEmail ? 1 : 0) + 1; // date
        }
        if (options?.globalSettings?.require_signing_reason && options?.reason) {
          lineCount += 1;
        }
        textHeight = lineCount > 0 ? (lineCount - 1) * 8 + 8 + 2 : 0; // More precise: (lines-1)*lineHeight + fontSize + padding
      }


      // Calculate scale to fit signature in canvas with minimal padding, giống web viewer
      const padding = 5;
      const scaleX = (canvasWidth - padding * 2) / signatureWidth;
      const scaleY = ((canvasHeight - textHeight) - padding * 2) / signatureHeight;
      const scale = Math.min(scaleX, scaleY); // Use minimum scale to preserve aspect ratio


      // Calculate offset to center signature
      const offsetX = (canvasWidth - signatureWidth * scale) / 2 - minX * scale;
      const offsetY = ((canvasHeight - textHeight) - signatureHeight * scale) / 2 - minY * scale;


      // Draw signature with natural line width similar to web viewer
      ctx.strokeStyle = '#000000';
      ctx.lineWidth = 2.5; // Match web viewer thickness
      ctx.lineCap = 'round';
      ctx.lineJoin = 'round';
      ctx.globalAlpha = 1.0; // Ensure full opacity
      ctx.miterLimit = 10; // Prevent sharp corners

      pointGroups.forEach((group: any[]) => {
        if (group.length === 0) return;

        ctx.beginPath();
        group.forEach((point: any, index: number) => {
          const x = point.x * scale + offsetX;
          const y = point.y * scale + offsetY;

          if (index === 0) {
            ctx.moveTo(x, y);
          } else {
            ctx.lineTo(x, y);
          }
        });
        ctx.stroke();
      });

      // Re-enable image smoothing for text
      ctx.imageSmoothingEnabled = true;

      // Render additional text below the signature if enabled (giống SignatureRenderer)
      const { submitterId, submitterEmail, reason, additionalText, globalSettings } = options || {};

      let textToShow: string[] = [];
      if (globalSettings?.add_signature_id_to_the_documents) {
        if (submitterId) textToShow.push(`ID: ${hashId(submitterId + 1)}`);
        if (submitterEmail) textToShow.push(submitterEmail);
        textToShow.push(new Date().toLocaleString('vi-VN', {
          year: 'numeric', month: '2-digit', day: '2-digit',
          hour: '2-digit', minute: '2-digit', second: '2-digit',
          timeZone: 'Asia/Ho_Chi_Minh'
        }));
      } else if (additionalText) {
        textToShow = [additionalText];
      }

      // Always show reason if require_signing_reason is enabled and reason exists
      if (globalSettings?.require_signing_reason && reason) {
        if (globalSettings?.add_signature_id_to_the_documents) {
          // Show both reason and ID/email/date
          textToShow = [`Reason: ${reason}`, `ID: ${hashId(submitterId + 1)}`, submitterEmail, new Date().toLocaleString('vi-VN', {
            year: 'numeric', month: '2-digit', day: '2-digit',
            hour: '2-digit', minute: '2-digit', second: '2-digit',
            timeZone: 'Asia/Ho_Chi_Minh'
          })].filter(Boolean);
        } else {
          // Show only reason
          textToShow = [`Reason: ${reason}`];
        }
      }

      if (textToShow.length > 0) {
        ctx.fillStyle = '#000000';
        ctx.font = `bold normal 8px 'Helvetica, Arial, sans-serif'`;
        ctx.textAlign = 'left';
        ctx.textBaseline = 'bottom';

        // Calculate line height
        const lineHeight = 8;
        let y = canvasHeight - 2;

        // Draw lines from bottom to top
        for (let i = textToShow.length - 1; i >= 0; i--) {
          ctx.fillText(textToShow[i], 5, y);
          y -= lineHeight;
        }
      }


      // Convert canvas to data URL
      const imageDataUrl = canvas.toDataURL('image/png');

      // Verify the data URL is valid
      if (!imageDataUrl || imageDataUrl.length < 100 || !imageDataUrl.startsWith('data:image/png')) {
        reject(new Error('Failed to create valid PNG data URL'));
        return;
      }

      resolve(imageDataUrl);
    } catch (error) {
      console.error('❌ Error in renderSignatureToImage:', error);
      reject(error);
    }
  });
};

// Main PDF download function
export const downloadSignedPDF = async (
  pdfUrl: string,
  signatures: any[],
  templateName: string,
  submitterInfo?: { id: number; email: string } | null,
  globalSettings?: any,
  auditLog?: AuditLogEntry[],
  submissionStatus?: string,
  completedAt?: string
) => {
  // Fetch PDF file từ server với binary response
  const API_BASE_URL = (import.meta as any).env?.VITE_API_BASE_URL || '';
  const fullUrl = `${API_BASE_URL}/api/files/${pdfUrl}`;
  const response = await fetch(fullUrl, {
    headers: {
      'Authorization': localStorage.getItem('token') ? `Bearer ${localStorage.getItem('token')}` : ''
    }
  });

  if (!response.ok) {
    throw new Error(`Failed to fetch PDF: ${response.statusText}`);
  }

  const pdfBytes = await response.arrayBuffer();

  // Load PDF với pdf-lib
  const pdfDoc = await PDFDocument.load(pdfBytes);
  const pages = pdfDoc.getPages();

  // Embed font that supports Vietnamese
  let font;
  try {
    const fontResponse = await fetch('https://fonts.gstatic.com/s/roboto/v30/KFOmCnqEu92Fr1Mu72xK.ttf');
    if (fontResponse.ok) {
      const fontBytes = await fontResponse.arrayBuffer();
      font = await pdfDoc.embedFont(fontBytes);
    } else {
      throw new Error('Font fetch failed');
    }
  } catch (error) {
    console.warn('Failed to embed custom font, falling back to Helvetica:', error);
    font = await pdfDoc.embedFont(StandardFonts.Helvetica);
  }

  // Lặp qua tất cả chữ ký và render lên PDF
  for (const signature of signatures) {
    const field = signature.field_info;
    const signatureValue = signature.signature_value;


    if (!signatureValue || !field.position) continue;

    const pageIndex = field.position.page - 1; // Convert 1-based to 0-based
    if (pageIndex < 0 || pageIndex >= pages.length) continue;

    const page = pages[pageIndex];
    const { width: pageWidth, height: pageHeight } = page.getSize();

    // Normalize position giống như web viewer (sử dụng default PDF dimensions 600x800)
    const normalizePosition = (position: any) => {
      if (!position || typeof position.x !== 'number') return position;

      // Check if position is in pixels (values > 1) or already in decimal (0-1)
      if (position.x > 1 || position.y > 1 || position.width > 1 || position.height > 1) {
        // Position is in pixels, convert to relative (0-1) using DEFAULT PDF dimensions như web viewer
        const DEFAULT_PDF_WIDTH = 600;
        const DEFAULT_PDF_HEIGHT = 800;
        return {
          ...position,
          x: position.x / DEFAULT_PDF_WIDTH,
          y: position.y / DEFAULT_PDF_HEIGHT,
          width: position.width / DEFAULT_PDF_WIDTH,
          height: position.height / DEFAULT_PDF_HEIGHT
        };
      }
      // Already in relative format
      return position;
    };

    const normalizedPos = normalizePosition(field.position);

    // DÙNG CÔNG THỨC GIỐNG FRONTEND (PdfViewer.tsx)
    // Position trong database là pixel values, normalize về relative (0-1) dùng default 600x800 như web viewer
    const x = Math.max(0, Math.min(1, normalizedPos.x)) * pageWidth;
    const y = Math.max(0, Math.min(1, normalizedPos.y)) * pageHeight;
    const fieldWidth = Math.max(0, Math.min(1, normalizedPos.width)) * pageWidth;
    const fieldHeight = Math.max(0, Math.min(1, normalizedPos.height)) * pageHeight;
    // PDF coordinates: bottom-left origin, nhưng ta cần convert từ top-left
    const pdfX = Math.max(0, Math.min(pageWidth - fieldWidth, x));
    const pdfY = Math.max(0, pageHeight - y - fieldHeight);


    // Render based on field type
    if (field.field_type === 'text' || field.field_type === 'date' || field.field_type === 'number') {
      // Render text as image to preserve Unicode characters
      const fontSize = Math.min(fieldHeight * 0.6, 12);
      const canvas = document.createElement('canvas');
      const ctx = canvas.getContext('2d');
      if (ctx) {
        // Use higher resolution for sharp text
        const scale = 4;
        canvas.width = fieldWidth * scale;
        canvas.height = fieldHeight * scale;
        ctx.scale(scale, scale);
        ctx.font = `bold normal ${fontSize}px 'Helvetica, Arial, sans-serif'`;
        ctx.fillStyle = 'black';
        ctx.textBaseline = 'middle';
        ctx.fillText(signatureValue, 0, fieldHeight / 2);
        const imageDataUrl = canvas.toDataURL('image/png');
        const imageBytes = await fetch(imageDataUrl).then(r => r.arrayBuffer());
        const image = await pdfDoc.embedPng(imageBytes);
        page.drawImage(image, {
          x: pdfX,
          y: pdfY,
          width: fieldWidth,
          height: fieldHeight,
        });
      } else {
        // Fallback to text if canvas not available
        page.drawText(signatureValue, {
          x: pdfX,
          y: pdfY + fieldHeight * 0.3,
          size: fontSize,
          font: font,
          color: rgb(0, 0, 0),
        });
      }
    } else if (field.field_type === 'signature' || field.field_type === 'initials') {
      // Xử lý chữ ký (có thể là image hoặc drawn signature)
      if (signatureValue.startsWith('data:image/') || signatureValue.startsWith('/api/')) {
        // Chữ ký dạng image - embed vào PDF
        try {
          let imageUrl = signatureValue;
          // If it's an API URL, construct the full URL
          if (signatureValue.startsWith('/api/')) {
            const API_BASE_URL = (import.meta as any).env?.VITE_API_BASE_URL || '';
            imageUrl = `${API_BASE_URL}${signatureValue}`;
          }

          const imageBytes = await fetch(imageUrl, {
            headers: {
              'Authorization': localStorage.getItem('token') ? `Bearer ${localStorage.getItem('token')}` : ''
            }
          }).then(res => res.arrayBuffer());

          let image;
          if (signatureValue.includes('png') || signatureValue.includes('.png')) {
            image = await pdfDoc.embedPng(imageBytes);
          } else {
            image = await pdfDoc.embedJpg(imageBytes);
          }

          // Scale image to fit field
          const imgDims = image.scale(1);
          const scale = Math.min(fieldWidth / imgDims.width, fieldHeight / imgDims.height);

          page.drawImage(image, {
            x: pdfX,
            y: pdfY,
            width: imgDims.width * scale,
            height: imgDims.height * scale,
          });
        } catch (err) {
          console.error('Error embedding signature image:', err);
        }
      } else if (signatureValue.startsWith('[') || signatureValue.startsWith('{')) {
        // Vector signature data - render to canvas then embed as image
        // Use EXACT field dimensions như web viewer, không clamp
        const canvasWidth = fieldWidth;
        const canvasHeight = fieldHeight;

        try {
          // Render signature to canvas and get image data
          const signatureImageUrl = await renderSignatureToImage(
            signatureValue,
            canvasWidth,
            canvasHeight,
            {
              submitterId: submitterInfo?.id,
              submitterEmail: submitterInfo?.email,
              reason: signature.reason,
              globalSettings
            }
          );


          // Embed the rendered signature image at the exact field dimensions
          const imageBytes = await fetch(signatureImageUrl).then(res => res.arrayBuffer());
          const image = await pdfDoc.embedPng(imageBytes);
5

          page.drawImage(image, {
            x: pdfX,
            y: pdfY,
            width: fieldWidth,
            height: fieldHeight,
          });

        } catch (err) {
          console.error('Error rendering vector signature:', err);
          // Fallback to text placeholder
          const fontSize = Math.min(fieldHeight * 0.6, 12);
          page.drawText('[Signature]', {
            x: pdfX,
            y: pdfY + fieldHeight * 0.3,
            size: fontSize,
            font: font,
            color: rgb(0, 0, 0),
          });
        }
      } else {
        // Plain text signature - render as image
        const fontSize = Math.min(fieldHeight * 0.6, 12);
        const canvas = document.createElement('canvas');
        const ctx = canvas.getContext('2d');
        if (ctx) {
          // Use higher resolution for sharp text
          const scale = 4;
          canvas.width = fieldWidth * scale;
          canvas.height = fieldHeight * scale;
          ctx.scale(scale, scale);
          ctx.font = `bold normal ${fontSize}px 'Helvetica, Arial, sans-serif'`;
          ctx.fillStyle = 'black';
          ctx.textBaseline = 'middle';
          ctx.fillText(signatureValue, 0, fieldHeight / 2);
          const imageDataUrl = canvas.toDataURL('image/png');
          const imageBytes = await fetch(imageDataUrl).then(r => r.arrayBuffer());
          const image = await pdfDoc.embedPng(imageBytes);
          page.drawImage(image, {
            x: pdfX,
            y: pdfY,
            width: fieldWidth,
            height: fieldHeight,
          });
        } else {
          page.drawText(signatureValue, {
            x: pdfX,
            y: pdfY + fieldHeight * 0.3,
            size: fontSize,
            font: font,
            color: rgb(0, 0, 0),
          });
        }
      }
    } else if (field.field_type === 'checkbox') {
      // Render checkbox
      if (signatureValue === 'true') {
        // Draw checkmark
        const checkSize = Math.min(fieldWidth, fieldHeight) * 0.8;
        page.drawText('✓', {
          x: pdfX + (fieldWidth - checkSize) / 2,
          y: pdfY + (fieldHeight - checkSize) / 2,
          size: checkSize,
          font: font,
          color: rgb(0, 0, 0),
        });
      }
    } else if (field.field_type === 'image') {
      // Handle uploaded images
      if (signatureValue.startsWith('http') || signatureValue.startsWith('blob:') || signatureValue.startsWith('data:image/')) {
        try {
          const imageBytes = await fetch(signatureValue).then(res => res.arrayBuffer());
          let image;
          if (signatureValue.includes('png')) {
            image = await pdfDoc.embedPng(imageBytes);
          } else {
            image = await pdfDoc.embedJpg(imageBytes);
          }

          const imgDims = image.scale(1);
          const scale = Math.min(fieldWidth / imgDims.width, fieldHeight / imgDims.height);

          page.drawImage(image, {
            x: pdfX,
            y: pdfY,
            width: imgDims.width * scale,
            height: imgDims.height * scale,
          });
        } catch (err) {
          console.error('Error embedding image:', err);
        }
      }
    }
  }

  // Add audit log pages if provided
  if (auditLog && auditLog.length > 0) {
    await generateAuditLogPages(pdfDoc, auditLog, globalSettings);
  }

  // Save và download PDF
  const pdfBytesModified = await pdfDoc.save();
  
  // Get filename format from user preferences
  const preferences = await getPDFPreferences();
  const filenameFormat = preferences?.filenameFormat || '{document.name}';
  
  // Generate filename based on user's format
  const filename = applyFilenameFormat(filenameFormat, {
    documentName: templateName,
    submissionStatus: submissionStatus || 'signed',
    submitterEmails: submitterInfo?.email ? [submitterInfo.email] : [],
    completedAt: completedAt || new Date().toISOString()
  });
  
  // Send to backend to add digital signature structure
  try {
    const token = localStorage.getItem('token');
    const formData = new FormData();
    const pdfBlob = new Blob([pdfBytesModified as any], { type: 'application/pdf' });
    formData.append('pdf', pdfBlob, filename);
    formData.append('signer_email', submitterInfo?.email || 'unknown@letmesign.com');
    formData.append('signer_name', submitterInfo?.email || submitterInfo?.id ? `User ${submitterInfo.id}` : 'Unknown Signer');
    formData.append('reason', `Document signed via Letmesign on ${new Date().toLocaleDateString('vi-VN')}`);
    
    const response = await fetch('/api/pdf-signature/sign-visual-pdf', {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${token}`
      },
      body: formData
    });
    
    if (response.ok) {
      const result = await response.json();
      if (result.success && result.data?.pdf_base64) {
        // Use digitally signed PDF
        const signedPdfBytes = Uint8Array.from(atob(result.data.pdf_base64), c => c.charCodeAt(0));
        const signedBlob = new Blob([signedPdfBytes], { type: 'application/pdf' });
        const link = document.createElement('a');
        link.href = URL.createObjectURL(signedBlob);
        link.download = filename;
        document.body.appendChild(link);
        link.click();
        document.body.removeChild(link);
        URL.revokeObjectURL(link.href);
        
        return; // Success, exit function
      }
    }
    
    // If signing failed, log warning and continue with visual-only PDF
    console.warn('Digital signing failed or not available, downloading visual-only PDF');
  } catch (signError) {
    console.error('Digital signing error:', signError);
  }
  
  // Fallback: Download visual-only PDF if signing failed
  const blob = new Blob([pdfBytesModified as any], { type: 'application/pdf' });
  const link = document.createElement('a');
  link.href = URL.createObjectURL(blob);
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  URL.revokeObjectURL(link.href);
  
  console.warn('⚠️ Downloaded visual-only PDF (digital signature not added)');
};

// Generate audit log pages to append to PDF
export const generateAuditLogPages = async (
  pdfDoc: PDFDocument,
  auditLog: AuditLogEntry[],
  globalSettings?: any
): Promise<void> => {
  // Embed fonts that support Vietnamese
  let font, boldFont;
  try {
    const fontResponse = await fetch('https://fonts.gstatic.com/s/roboto/v30/KFOmCnqEu92Fr1Mu72xK.ttf');
    const boldFontResponse = await fetch('https://fonts.gstatic.com/s/roboto/v30/KFOlCnqEu92Fr1MmWUlFBBc4.ttf');
    if (fontResponse.ok && boldFontResponse.ok) {
      const fontBytes = await fontResponse.arrayBuffer();
      const boldFontBytes = await boldFontResponse.arrayBuffer();
      font = await pdfDoc.embedFont(fontBytes);
      boldFont = await pdfDoc.embedFont(boldFontBytes);
    } else {
      throw new Error('Font fetch failed');
    }
  } catch (error) {
    console.warn('Failed to embed custom fonts, falling back to Helvetica:', error);
    font = await pdfDoc.embedFont(StandardFonts.Helvetica);
    boldFont = await pdfDoc.embedFont(StandardFonts.HelveticaBold);
  }
  
  const pageWidth = 595; // A4 width in points
  const pageHeight = 842; // A4 height in points
  const margin = 50;
  const lineHeight = 15;
  const maxWidth = pageWidth - 2 * margin;
  
  let page = pdfDoc.addPage([pageWidth, pageHeight]);
  let yPosition = pageHeight - margin;
  
  // Try to embed logo (optional)
  let logo = null;
  try {
    const logoUrl = globalSettings?.logo_url || '/logo.png'; // Use user logo or default
    const logoResponse = await fetch(logoUrl);
    if (logoResponse.ok) {
      const logoBytes = await logoResponse.arrayBuffer();
      logo = await pdfDoc.embedPng(logoBytes);
    }
  } catch (err) {
    console.warn('Logo not found, continuing without logo:', err);
  }
  
  // Draw logo and title on the same line
  if (logo) {
    const logoHeight = 130;
    const logoWidth = logo.width * (logoHeight / logo.height);
    
    // Draw logo on the left
    page.drawImage(logo, {
      x: margin,
      y: yPosition - logoHeight,
      width: logoWidth,
      height: logoHeight,
    });
    
    // Draw company name if available
    let titleY = yPosition - logoHeight / 2 - 9;
    if (globalSettings?.company_name) {
      const companyText = globalSettings.company_name;
      // Render company name as image
      const canvas = document.createElement('canvas');
      const ctx = canvas.getContext('2d');
      if (ctx) {
        // Use higher resolution for sharp text
        const scale = 4;
        ctx.font = `${16 * scale}px sans-serif`;
        const textWidth = ctx.measureText(companyText).width / scale;
        canvas.width = textWidth * scale;
        canvas.height = 20 * scale;
        ctx.scale(scale, scale);
        ctx.font = '16px sans-serif';
        ctx.fillStyle = 'black';
        ctx.textBaseline = 'top';
        ctx.fillText(companyText, 0, 0);
        const imageDataUrl = canvas.toDataURL('image/png');
        const imageBytes = await fetch(imageDataUrl).then(r => r.arrayBuffer());
        const image = await pdfDoc.embedPng(imageBytes);
        page.drawImage(image, {
          x: margin + logoWidth + 20,
          y: titleY - 16,
          width: textWidth,
          height: 20,
        });
      } else {
        const companyWidth = boldFont.widthOfTextAtSize(companyText, 16);
        page.drawText(companyText, {
          x: margin + logoWidth + 20,
          y: titleY,
          size: 16,
          font: boldFont,
          color: rgb(0, 0, 0),
        });
      }
      titleY -= 25; // Move audit log title down
    }
    
    // Draw title aligned to the right edge
    const titleText = 'Audit Log';
    const titleWidth = boldFont.widthOfTextAtSize(titleText, 18);
    const titleX = pageWidth - margin - titleWidth; // Right-aligned
    page.drawText(titleText, {
      x: titleX,
      y: titleY, // Center vertically with logo or below company name
      size: 18,
      font: boldFont,
      color: rgb(0, 0, 0),
    });
    
    yPosition -= logoHeight + 20;
  } else {
    // No logo, draw company name and title
    let currentY = yPosition;
    if (globalSettings?.company_name) {
      const companyText = globalSettings.company_name;
      // Render company name as image
      const canvas = document.createElement('canvas');
      const ctx = canvas.getContext('2d');
      if (ctx) {
        // Use higher resolution for sharp text
        const scale = 4;
        ctx.font = `${16 * scale}px sans-serif`;
        const textWidth = ctx.measureText(companyText).width / scale;
        canvas.width = textWidth * scale;
        canvas.height = 20 * scale;
        ctx.scale(scale, scale);
        ctx.font = '16px sans-serif';
        ctx.fillStyle = 'black';
        ctx.textBaseline = 'top';
        ctx.fillText(companyText, 0, 0);
        const imageDataUrl = canvas.toDataURL('image/png');
        const imageBytes = await fetch(imageDataUrl).then(r => r.arrayBuffer());
        const image = await pdfDoc.embedPng(imageBytes);
        page.drawImage(image, {
          x: margin,
          y: currentY - 16,
          width: textWidth,
          height: 20,
        });
      } else {
        page.drawText(companyText, {
          x: margin,
          y: currentY,
          size: 16,
          font: boldFont,
          color: rgb(0, 0, 0),
        });
      }
      currentY -= 25;
    }
    
    // Draw title right-aligned
    const titleText = 'Audit Log';
    const titleWidth = boldFont.widthOfTextAtSize(titleText, 18);
    page.drawText(titleText, {
      x: pageWidth - margin - titleWidth,
      y: currentY,
      size: 18,
      font: boldFont,
      color: rgb(0, 0, 0),
    });
    
    yPosition -= 30;
  }
  
  // Draw separator line
  page.drawLine({
    start: { x: margin, y: yPosition },
    end: { x: pageWidth - margin, y: yPosition },
    thickness: 1,
    color: rgb(0, 0, 0),
  });
  
  yPosition -= 20;
  
  // Draw audit log entries
  for (const entry of auditLog) {
    // Skip invalid entries
    if (!entry || !entry.timestamp || !entry.action || !entry.user) {
      console.warn('Skipping invalid audit log entry:', entry);
      continue;
    }

    // Check if we need a new page
    if (yPosition < margin + 120) {
      page = pdfDoc.addPage([pageWidth, pageHeight]);
      yPosition = pageHeight - margin;
    }
    
    // Draw timestamp
    page.drawText(sanitizeTextForPDF(entry.timestamp || 'N/A'), {
      x: margin,
      y: yPosition,
      size: 10,
      font: boldFont,
      color: rgb(0, 0, 0),
    });
    yPosition -= lineHeight;
    
    // Draw action
    page.drawText(`Action: ${sanitizeTextForPDF(entry.action || 'Unknown')}`, {
      x: margin + 10,
      y: yPosition,
      size: 9,
      font: font,
      color: rgb(0, 0, 0),
    });
    yPosition -= lineHeight;
    
    // Draw user
    page.drawText(`User: ${sanitizeTextForPDF(entry.user || 'Unknown')}`, {
      x: margin + 10,
      y: yPosition,
      size: 9,
      font: font,
      color: rgb(0.2, 0.2, 0.2),
    });
    yPosition -= lineHeight;
    
    // Draw details if available
    if (entry.details) {
      const detailsText = `Details: ${sanitizeTextForPDF(entry.details)}`;
      const words = detailsText.split(' ');
      let line = '';
      
      for (const word of words) {
        const testLine = line + word + ' ';
        const testWidth = font.widthOfTextAtSize(testLine, 9);
        
        if (testWidth > maxWidth - 20 && line.length > 0) {
          page.drawText(line.trim(), {
            x: margin + 10,
            y: yPosition,
            size: 9,
            font: font,
            color: rgb(0.3, 0.3, 0.3),
          });
          yPosition -= lineHeight;
          line = word + ' ';
          
          // Check if we need a new page
          if (yPosition < margin + 40) {
            page = pdfDoc.addPage([pageWidth, pageHeight]);
            yPosition = pageHeight - margin;
          }
        } else {
          line = testLine;
        }
      }
      
      if (line.trim().length > 0) {
        page.drawText(line.trim(), {
          x: margin + 10,
          y: yPosition,
          size: 9,
          font: font,
          color: rgb(0.3, 0.3, 0.3),
        });
        yPosition -= lineHeight;
      }
    }

    // Draw additional metadata if available
    const metadata: string[] = [];
    if (entry.ip) metadata.push(`IP: ${sanitizeTextForPDF(entry.ip)}`);
    if (entry.session_id) metadata.push(`Session: ${sanitizeTextForPDF(entry.session_id)}`);
    if (entry.timezone) metadata.push(`Timezone: ${sanitizeTextForPDF(entry.timezone)}`);
    if (entry.user_agent) {
      // Truncate user agent if too long
      const ua = entry.user_agent.length > 50 ? entry.user_agent.substring(0, 47) + '...' : entry.user_agent;
      metadata.push(`User Agent: ${sanitizeTextForPDF(ua)}`);
    }

    if (metadata.length > 0) {
      // Draw each metadata line separately to avoid overflow
      for (const meta of metadata) {
        if (yPosition < margin + 20) {
          page = pdfDoc.addPage([pageWidth, pageHeight]);
          yPosition = pageHeight - margin;
        }
        page.drawText(meta, {
          x: margin + 10,
          y: yPosition,
          size: 7,
          font: font,
          color: rgb(0.5, 0.5, 0.5),
        });
        yPosition -= 12;
      }
    }
    
    // Add spacing between entries
    yPosition -= 10;
    
    // Draw separator line
    if (yPosition > margin + 20) {
      page.drawLine({
        start: { x: margin, y: yPosition },
        end: { x: pageWidth - margin, y: yPosition },
        thickness: 0.5,
        color: rgb(0.7, 0.7, 0.7),
      });
      yPosition -= 15;
    }
  }
};

// Download signed PDF with audit log combined
export const downloadSignedPDFWithAuditLog = async (
  pdfUrl: string,
  signatures: any[],
  templateName: string,
  submitterInfo?: { id: number; email: string } | null,
  globalSettings?: any,
  auditLog?: AuditLogEntry[],
  submissionStatus?: string,
  completedAt?: string
) => {
  // Fetch PDF file từ server với binary response
  const API_BASE_URL = (import.meta as any).env?.VITE_API_BASE_URL || '';
  const fullUrl = `${API_BASE_URL}/api/files/${pdfUrl}`;
  const response = await fetch(fullUrl, {
    headers: {
      'Authorization': localStorage.getItem('token') ? `Bearer ${localStorage.getItem('token')}` : ''
    }
  });

  if (!response.ok) {
    throw new Error(`Failed to fetch PDF: ${response.statusText}`);
  }

  const pdfBytes = await response.arrayBuffer();

  // Load PDF với pdf-lib
  const pdfDoc = await PDFDocument.load(pdfBytes);
  const pages = pdfDoc.getPages();

  // Embed font that supports Vietnamese
  let font;
  try {
    const fontResponse = await fetch('https://fonts.gstatic.com/s/roboto/v30/KFOmCnqEu92Fr1Mu72xK.ttf');
    if (fontResponse.ok) {
      const fontBytes = await fontResponse.arrayBuffer();
      font = await pdfDoc.embedFont(fontBytes);
    } else {
      throw new Error('Font fetch failed');
    }
  } catch (error) {
    console.warn('Failed to embed custom font, falling back to Helvetica:', error);
    font = await pdfDoc.embedFont(StandardFonts.Helvetica);
  }

  // Lặp qua tất cả chữ ký và render lên PDF (same logic as downloadSignedPDF)
  for (const signature of signatures) {
    const field = signature.field_info;
    const signatureValue = signature.signature_value;
    if (!signatureValue || !field.position) continue;

    const pageIndex = field.position.page - 1;
    if (pageIndex < 0 || pageIndex >= pages.length) continue;

    const page = pages[pageIndex];
    const { width: pageWidth, height: pageHeight } = page.getSize();

    const normalizePosition = (position: any) => {
      if (!position || typeof position.x !== 'number') return position;

      if (position.x > 1 || position.y > 1 || position.width > 1 || position.height > 1) {
        const DEFAULT_PDF_WIDTH = 600;
        const DEFAULT_PDF_HEIGHT = 800;
        return {
          ...position,
          x: position.x / DEFAULT_PDF_WIDTH,
          y: position.y / DEFAULT_PDF_HEIGHT,
          width: position.width / DEFAULT_PDF_WIDTH,
          height: position.height / DEFAULT_PDF_HEIGHT
        };
      }
      return position;
    };

    const normalizedPos = normalizePosition(field.position);
    const x = Math.max(0, Math.min(1, normalizedPos.x)) * pageWidth;
    const y = Math.max(0, Math.min(1, normalizedPos.y)) * pageHeight;
    const fieldWidth = Math.max(0, Math.min(1, normalizedPos.width)) * pageWidth;
    const fieldHeight = Math.max(0, Math.min(1, normalizedPos.height)) * pageHeight;

    const pdfX = Math.max(0, Math.min(pageWidth - fieldWidth, x));
    const pdfY = Math.max(0, pageHeight - y - fieldHeight);

    // Render based on field type (same as downloadSignedPDF)
    if (field.field_type === 'text' || field.field_type === 'date' || field.field_type === 'number') {
      const fontSize = Math.min(fieldHeight * 0.6, 12);
      const canvas = document.createElement('canvas');
      const ctx = canvas.getContext('2d');
      if (ctx) {
        // Use higher resolution for sharp text
        const scale = 4;
        canvas.width = fieldWidth * scale;
        canvas.height = fieldHeight * scale;
        ctx.scale(scale, scale);
        ctx.font = `bold normal ${fontSize}px 'Helvetica, Arial, sans-serif'`;
        ctx.fillStyle = 'black';
        ctx.textBaseline = 'middle';
        ctx.fillText(signatureValue, 0, fieldHeight / 2);
        const imageDataUrl = canvas.toDataURL('image/png');
        const imageBytes = await fetch(imageDataUrl).then(r => r.arrayBuffer());
        const image = await pdfDoc.embedPng(imageBytes);
        page.drawImage(image, {
          x: pdfX,
          y: pdfY,
          width: fieldWidth,
          height: fieldHeight,
        });
      } else {
        page.drawText(signatureValue, {
          x: pdfX,
          y: pdfY + fieldHeight * 0.3,
          size: fontSize,
          font: font,
          color: rgb(0, 0, 0),
        });
      }
    } else if (field.field_type === 'signature' || field.field_type === 'initials') {
      if (signatureValue.startsWith('data:image/') || signatureValue.startsWith('/api/')) {
        try {
          let imageUrl = signatureValue;
          // If it's an API URL, construct the full URL
          if (signatureValue.startsWith('/api/')) {
            const API_BASE_URL = (import.meta as any).env?.VITE_API_BASE_URL || '';
            imageUrl = `${API_BASE_URL}${signatureValue}`;
          }

          const imageBytes = await fetch(imageUrl, {
            headers: {
              'Authorization': localStorage.getItem('token') ? `Bearer ${localStorage.getItem('token')}` : ''
            }
          }).then(res => res.arrayBuffer());

          let image;
          if (signatureValue.includes('png') || signatureValue.includes('.png')) {
            image = await pdfDoc.embedPng(imageBytes);
          } else {
            image = await pdfDoc.embedJpg(imageBytes);
          }

          const imgDims = image.scale(1);
          const scale = Math.min(fieldWidth / imgDims.width, fieldHeight / imgDims.height);

          page.drawImage(image, {
            x: pdfX,
            y: pdfY,
            width: imgDims.width * scale,
            height: imgDims.height * scale,
          });
        } catch (err) {
          console.error('Error embedding signature image:', err);
        }
      } else if (signatureValue.startsWith('[') || signatureValue.startsWith('{')) {
        const canvasWidth = fieldWidth;
        const canvasHeight = fieldHeight;

        try {
          const signatureImageUrl = await renderSignatureToImage(
            signatureValue,
            canvasWidth,
            canvasHeight,
            {
              submitterId: submitterInfo?.id,
              submitterEmail: submitterInfo?.email,
              reason: signature.reason,
              globalSettings
            }
          );

          const imageBytes = await fetch(signatureImageUrl).then(res => res.arrayBuffer());
          const image = await pdfDoc.embedPng(imageBytes);

          page.drawImage(image, {
            x: pdfX,
            y: pdfY,
            width: fieldWidth,
            height: fieldHeight,
          });
        } catch (err) {
          console.error('Error rendering vector signature:', err);
          const fontSize = Math.min(fieldHeight * 0.6, 12);
          page.drawText('[Signature]', {
            x: pdfX,
            y: pdfY + fieldHeight * 0.3,
            size: fontSize,
            font: font,
            color: rgb(0, 0, 0),
          });
        }
      } else {
        const fontSize = Math.min(fieldHeight * 0.6, 12);
        const canvas = document.createElement('canvas');
        const ctx = canvas.getContext('2d');
        if (ctx) {
          // Use higher resolution for sharp text
          const scale = 4;
          canvas.width = fieldWidth * scale;
          canvas.height = fieldHeight * scale;
          ctx.scale(scale, scale);
          ctx.font = `bold normal ${fontSize}px 'Helvetica, Arial, sans-serif'`;
          ctx.fillStyle = 'black';
          ctx.textBaseline = 'middle';
          ctx.fillText(signatureValue, 0, fieldHeight / 2);
          const imageDataUrl = canvas.toDataURL('image/png');
          const imageBytes = await fetch(imageDataUrl).then(r => r.arrayBuffer());
          const image = await pdfDoc.embedPng(imageBytes);
          page.drawImage(image, {
            x: pdfX,
            y: pdfY,
            width: fieldWidth,
            height: fieldHeight,
          });
        } else {
          page.drawText(signatureValue, {
            x: pdfX,
            y: pdfY + fieldHeight * 0.3,
            size: fontSize,
            font: font,
            color: rgb(0, 0, 0),
          });
        }
      }
    } else if (field.field_type === 'checkbox') {
      if (signatureValue === 'true') {
        const checkSize = Math.min(fieldWidth, fieldHeight) * 0.8;
        page.drawText('✓', {
          x: pdfX + (fieldWidth - checkSize) / 2,
          y: pdfY + (fieldHeight - checkSize) / 2,
          size: checkSize,
          font: font,
          color: rgb(0, 0, 0),
        });
      }
    } else if (field.field_type === 'image') {
      if (signatureValue.startsWith('http') || signatureValue.startsWith('blob:') || signatureValue.startsWith('data:image/')) {
        try {
          const imageBytes = await fetch(signatureValue).then(res => res.arrayBuffer());
          let image;
          if (signatureValue.includes('png')) {
            image = await pdfDoc.embedPng(imageBytes);
          } else {
            image = await pdfDoc.embedJpg(imageBytes);
          }

          const imgDims = image.scale(1);
          const scale = Math.min(fieldWidth / imgDims.width, fieldHeight / imgDims.height);

          page.drawImage(image, {
            x: pdfX,
            y: pdfY,
            width: imgDims.width * scale,
            height: imgDims.height * scale,
          });
        } catch (err) {
          console.error('Error embedding image:', err);
        }
      }
    }
  }

  // Add audit log pages if provided
  if (auditLog && auditLog.length > 0) {
    await generateAuditLogPages(pdfDoc, auditLog, globalSettings);
  }

  // Get filename format from user preferences
  const preferences = await getPDFPreferences();
  const filenameFormat = preferences?.filenameFormat || '{document.name}';
  
  // Generate filename based on user's format
  const filename = applyFilenameFormat(filenameFormat, {
    documentName: templateName,
    submissionStatus: submissionStatus || 'signed',
    submitterEmails: submitterInfo?.email ? [submitterInfo.email] : [],
    completedAt: completedAt || new Date().toISOString()
  });

  // Save và download PDF
  const pdfBytesModified = await pdfDoc.save();
  const blob = new Blob([pdfBytesModified as any], { type: 'application/pdf' });
  const link = document.createElement('a');
  link.href = URL.createObjectURL(blob);
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  URL.revokeObjectURL(link.href);
};