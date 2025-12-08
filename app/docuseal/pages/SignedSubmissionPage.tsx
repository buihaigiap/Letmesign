import  { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import PdfViewer from '../components/PdfViewer';
import SignatureRenderer from '../components/SignatureRenderer';
import upstashService from '../ConfigApi/upstashService';
import { FilePenLine, Mail, Download } from 'lucide-react';

const SignedSubmissionPage = () => {
  const { token } = useParams<{ token: string }>();
  const navigate = useNavigate();
  const [data, setData] = useState<any>(null);
  const [error, setError] = useState('');
  const [submitterInfo, setSubmitterInfo] = useState<{ id: number; email: string } | null>(null);
  useEffect(() => {
    const fetchData = async () => {
      try {
        // Fetch submitter info, signatures and fields data in parallel
        const [submitterResult, signaturesResult, fieldsResult] = await Promise.all([
          upstashService.getSubmitterInfo(token),
          upstashService.getSubmissionSignatures(token),
          upstashService.getSubmissionFields(token)
        ]);

        if (submitterResult.success) {
          setData(prevData => ({ ...prevData, submitter: submitterResult.data }));
        }

        if (signaturesResult.success) {
          setData(prevData => ({ ...prevData, ...signaturesResult.data }));
        } else {
          setError(signaturesResult.message || 'Failed to fetch signatures data');
        }

        if (fieldsResult.success && fieldsResult.data.information) {
          setSubmitterInfo({
            id: fieldsResult.data.information.id,
            email: fieldsResult.data.information.email
          });
        }
      } catch (err) {
        console.error('Fetch error:', err);
        setError('An error occurred while fetching data');
      } 
    };

    if (token) {
      fetchData();
    }
  }, [token]);

  if (error) {
    return (
      <div className="min-h-screen bg-gray-900 text-white flex items-center justify-center">
        <div className="text-center">
          <p className="text-red-500 mb-4">{error}</p>
          <button onClick={() => navigate(-1)} className="px-4 py-2 bg-indigo-600 rounded-md hover:bg-indigo-700">
            Go Back
          </button>
        </div>
      </div>
    );
  }

  if (!data) return null;

  return (
    <div className="min-h-screen  text-white">
      {/* Header */}
      <div className="mx-auto mb-6 px-4">
        <div className="flex items-center justify-between">
          <button 
            onClick={() => navigate(-1)} 
            className="px-4 py-2 bg-gray-700 rounded-md hover:bg-gray-600 transition-colors flex items-center gap-2"
          >
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 19l-7-7m0 0l7-7m-7 7h18" />
            </svg>
            Back
          </button>
          <h1 className="text-xl font-semibold">{data.template_info.name}</h1>
        </div>
      </div>

      {/* Main Content */}
      <div className="flex flex-col lg:flex-row gap-3 px-4">
        {/* PDF Viewer */}
        <div className="flex justify-center">
          <PdfViewer
            filePath={data.template_info.document.url}
            fields={data?.bulk_signatures?.map(sig => ({ ...sig.field_info, signature_value: sig.signature_value, reason: sig.reason }))}
            submitterId={submitterInfo?.id}
            submitterEmail={submitterInfo?.email}
            globalSettings={data?.submitter?.global_settings}
            // scale={1.5}
          />
        </div>
        {/* Signature Information Panel */}
        <div className="w-full lg:w-80 bg-gray-800 rounded-lg p-4">
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <h2 className="text-lg font-semibold mb-4">Thông tin chữ ký</h2>
          </div> 
              <div className="space-y-2">
                <p className="text-sm font-medium">{data.submitter?.name}</p>
                {submitterInfo && (
                  <div className="flex items-center gap-2">
                    <Mail className="w-4 h-4 text-gray-400" />
                    <p className="text-sm text-gray-300">{data.submitter?.email}</p>
                  </div>
                )}
                <div className="flex items-center gap-2">
                  <FilePenLine className="w-4 h-4 text-gray-400" />
                  <p className="text-sm text-gray-300">{data.submitter?.created_at}</p>
                </div>
                {data.submitter?.status === 'declined' && data.submitter?.decline_reason && (
                    <p className="text-sm">
                      <strong>Reason:</strong> {data.submitter.decline_reason}
                    </p>
                )}
              </div>

            {/* Signature Details */}
            {data.bulk_signatures?.map((sig, index) => (
              <div key={index} className="bg-gray-700 rounded-md p-3">
                <h3 className="text-md font-medium mb-2">{sig?.field_name}</h3>
                {sig.reason && (
                  <div className="mb-2">
                    <p className="text-sm text-gray-300"><strong>Reason:</strong> {sig.reason}</p>
                  </div>
                )}
                {sig.signature_value && (
                  <div className="mb-2">
                    {sig.signature_value.startsWith('data:image/') || sig.signature_value.startsWith('blob:') || sig.signature_value.startsWith('http') ? (
                      <img 
                        src={sig.signature_value} 
                        alt={`Signature ${index + 1}`} 
                        className="max-w-full h-auto border border-gray-600 rounded"
                      />
                    ) : sig.signature_value.startsWith('[') || sig.signature_value.startsWith('{') || sig.field_info?.field_type === 'signature' ? (
                      <div className="border border-gray-600 rounded p-2">
                        <SignatureRenderer 
                          data={sig.signature_value} 
                          width={200} 
                          height={100}
                          submitterId={submitterInfo?.id}
                          submitterEmail={submitterInfo?.email}
                          reason={sig.reason}
                          globalSettings={data?.submitter?.global_settings}
                        />
                      </div>
                    ) : (
                      <p className="text-sm text-gray-300">{sig.signature_value}</p>
                    )}
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
};

export default SignedSubmissionPage;