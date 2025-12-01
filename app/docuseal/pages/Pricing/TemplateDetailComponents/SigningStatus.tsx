import React from 'react';
import { Submitter } from '../../../types';
import upstashService from '../../../ConfigApi/upstashService';
import { Trash2 } from 'lucide-react';
import toast from 'react-hot-toast';
import { useTranslation } from 'react-i18next';
import SubmitterItem from './SubmitterItem';

interface SigningStatusProps {
  templateInfo: any;
  handleViewSubmission: (token: string) => void;
  handleDeleteSubmitter: (id: number) => void;
  fetchTemplateInfo: () => void;
  setShowInviteModal: (show: boolean) => void;
}

const SigningStatus: React.FC<SigningStatusProps> = ({
  templateInfo,
  handleViewSubmission,
  handleDeleteSubmitter,
  fetchTemplateInfo,
  setShowInviteModal,
}) => {
  const { t } = useTranslation();
  return (
    <div className="mt-6">
      {templateInfo.signatures && templateInfo.signatures.length > 0 ? (
        <div className="space-y-6">
          <div className="flex justify-between items-center">
            <h2 className="text-2xl font-semibold">{t('templates.detail.signingStatus')}</h2>
            <button onClick={() => setShowInviteModal(true)} className="px-4 py-2 font-semibold text-white bg-indigo-600 rounded-md hover:bg-indigo-700">
              {t('templates.detail.addRecipients')}
            </button>
          </div>
          <div className="space-y-4">
            {templateInfo.signatures.map((signature: any, signatureIndex: number) => (
              <div key={signatureIndex} className="bg-white/5 border border-white/10 rounded-lg p-4 border">
                <div className="flex items-center justify-between mb-3 text-gray-500">
                  <h3 className="text-lg font-medium text-white">
                    {signature.type === 'bulk' ? t('templates.detail.bulkSignature') : t('templates.detail.singleSignature')}
                    <span className="text-sm ml-2">
                      ({signature.parties.length} {t('templates.detail.parties')})
                    </span>
                  </h3>
                  <span className={`px-3 py-1 text-xs font-bold rounded-full uppercase ${
                    signature.overall_status === 'completed'
                      ? 'bg-green-100 text-green-800'
                      : 'bg-yellow-100 text-yellow-800'
                  }`}>
                    {signature.overall_status}
                  </span>
                </div>
                {signature.type === 'bulk' ? (
                  <div className="flex justify-between items-center rounded-lg shadow-sm">
                    <div className="space-y-2 flex-1">
                      {signature.parties.map((party: any) => (
                        <SubmitterItem
                          key={party.id}
                          party={party}
                          signatureType={signature.type}
                          overallStatus={signature.overall_status}
                          showActions={false}
                        />
                      ))}
                    </div>
                    <div className="flex items-center gap-2">
                      <button
                        onClick={() => handleViewSubmission(signature.parties[0].token)}
                        className="px-3 py-1.5 text-sm font-semibold
                             border-gray-500 , border
                           rounded-full hover:bg-gray-800
                            hover:text-white transition-colors"
                      >
                        {t('templates.detail.view')}
                      </button>
                      <button
                        onClick={async (e) => {
                          e.stopPropagation();
                          if (confirm(t('templates.detail.confirm.deleteBulkSignature', { count: signature.parties.length }))) {
                            try {
                              // Delete all parties in the bulk signature
                              const deletePromises = signature.parties.map(party =>
                                upstashService.deleteSubmitter(party.id)
                              );
                              await Promise.all(deletePromises);
                              toast.success(t('templates.detail.success.bulkSignatureDeleted'));
                              fetchTemplateInfo();
                            } catch (err) {
                              console.error('Bulk delete error:', err);
                              toast.error(t('templates.detail.errors.bulkSignatureDeletionFailed'));
                            }
                          }
                        }}
                        className="p-1.5 text-gray-600 hover:text-red-600 transition-colors"
                      >
                         <Trash2 color='red'/>
                      </button>
                    </div>
                  </div>
                ) : (
                  <div className="space-y-2">
                    {signature.parties.map((party: any) => (
                      <SubmitterItem
                        key={party.id}
                        party={party}
                        signatureType={signature.type}
                        overallStatus={signature.overall_status}
                        onView={handleViewSubmission}
                        onDelete={handleDeleteSubmitter}
                        pdfUrl={templateInfo.template.file_url}
                      />
                    ))}
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>
      ) : (
        <div className="text-center py-12">
          <h2 className="text-2xl font-semibold mb-4">{t('templates.detail.emptyState.title')}</h2>
          <p className="text-gray-400 mb-6">{t('templates.detail.emptyState.description')}</p>
          <button onClick={() => setShowInviteModal(true)} className="px-6 py-3 font-semibold text-white bg-indigo-600 rounded-md hover:bg-indigo-700">
            {t('templates.detail.emptyState.sendToRecipients')}
          </button>
        </div>
      )}
    </div>
  );
};

export default SigningStatus;