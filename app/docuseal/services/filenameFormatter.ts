// Helper service to generate download filename based on user settings

interface FilenameFormatOptions {
  documentName: string;
  submissionStatus: string;
  submitterEmails: string[];
  completedAt?: string;
}

/**
 * Apply filename format template with placeholders
 * 
 * Placeholders:
 * - {document.name} -> original document name
 * - {submission.status} -> "Signed" or "Completed"
 * - {submission.submitters} -> submitter email(s)
 * - {submission.completed_at} -> completion date
 */
export function applyFilenameFormat(
  format: string,
  options: FilenameFormatOptions
): string {
  let result = format;
  
  // Remove .pdf extension from document name if exists
  const docName = options.documentName.replace(/\.pdf$/i, '');
  
  // Replace {document.name}
  result = result.replace(/{document\.name}/g, docName);
  
  // Replace {submission.status}
  const statusDisplay = options.submissionStatus === 'completed' || options.submissionStatus === 'signed' 
    ? 'Signed' 
    : 'Completed';
  result = result.replace(/{submission\.status}/g, statusDisplay);
  
  // Replace {submission.submitters}
  const submittersStr = options.submitterEmails.length > 0 
    ? options.submitterEmails[0] // Use first submitter for simplicity
    : 'unknown';
  result = result.replace(/{submission\.submitters}/g, submittersStr);
  
  // Replace {submission.completed_at}
  if (options.completedAt) {
    const date = new Date(options.completedAt);
    const formatted = date.toLocaleDateString('en-US', { 
      month: 'short', 
      day: 'numeric', 
      year: 'numeric' 
    });
    result = result.replace(/{submission\.completed_at}/g, formatted);
  } else {
    result = result.replace(/{submission\.completed_at}/g, '');
  }
  
  // Clean up any remaining placeholders or extra spaces/dashes
  result = result
    .replace(/ - -/g, ' -')
    .replace(/- -/g, '-')
    .replace(/ - $/g, '')
    .replace(/ -$/g, '')
    .replace(/-$/g, '')
    .trim();
  
  // Add .pdf extension if not already present
  if (!result.endsWith('.pdf')) {
    result += '.pdf';
  }
  
  return result;
}

/**
 * Fetch user's PDF preferences settings
 */
export async function getPDFPreferences(): Promise<{ filenameFormat: string } | null> {
  try {
    const token = localStorage.getItem('token');
    if (!token) return null;
    
    const response = await fetch('/api/pdf-signature/settings', {
      headers: {
        'Authorization': `Bearer ${token}`
      }
    });
    
    if (!response.ok) return null;
    
    const result = await response.json();
    if (result.success && result.data) {
      return {
        filenameFormat: result.data.filename_format || '{document.name}'
      };
    }
    
    return null;
  } catch (error) {
    console.error('Failed to fetch PDF preferences:', error);
    return null;
  }
}

/**
 * Generate download filename based on user settings
 */
export async function generateDownloadFilename(
  documentName: string,
  submitterEmail: string,
  submissionStatus: string = 'completed',
  completedAt?: string,
  withAuditLog: boolean = false
): Promise<string> {
  // Try to fetch user preferences
  const preferences = await getPDFPreferences();
  
  if (preferences && preferences.filenameFormat) {
    const filename = applyFilenameFormat(preferences.filenameFormat, {
      documentName,
      submissionStatus,
      submitterEmails: [submitterEmail],
      completedAt
    });
    
    // If with audit log, insert "_with_audit" before .pdf
    if (withAuditLog) {
      return filename.replace(/\.pdf$/i, '_with_audit.pdf');
    }
    
    return filename;
  }
  
  // Fallback to default format if settings not available
  const baseName = documentName.replace(/\.pdf$/i, '');
  const suffix = withAuditLog ? '_with_audit' : '';
  return `signed_${baseName}${suffix}.pdf`;
}
