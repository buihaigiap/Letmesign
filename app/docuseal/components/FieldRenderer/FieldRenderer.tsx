import React from 'react';
import SignatureRenderer from '../SignatureRenderer';
import { fieldTools } from '../PdfFieldEditor/constants';
import { useBasicSettings } from '../../hooks/useBasicSettings';
interface FieldPosition {
  x: number;
  y: number;
  width: number;
  height: number;
  page: number;
  default_value?: string;
}

interface Field {
  id?: number;
  tempId?: string;
  name: string;
  field_type: string;
  position: FieldPosition;
  options?: any;
  partner?: string;
}

interface FieldRendererProps {
  field: Field;
  value?: string;
  className?: string;
  style?: React.CSSProperties;
  onClick?: () => void;
  title?: string;
  children?: React.ReactNode;
  defaultSignature?: string; // Signature từ user profile
  defaultInitials?: string;  // Initials từ user profile
  submitterId?: number;
  submitterEmail?: string;
  reason?: string;
  globalSettings ?: any;
}
const FieldRenderer: React.FC<FieldRendererProps> = ({
  field,
  value,
  className,
  style,
  onClick,
  title,
  children,
  defaultSignature,
  defaultInitials,
  submitterId,
  submitterEmail,
  reason,globalSettings
}) => {
  const renderFieldContent = () => {
    // Nếu có children (như editing UI), ưu tiên render children
    if (children) {
      return children;
    }

    // Safety check for field
    if (!field) {
      return null;
    }

    // Xác định giá trị hiển thị: ưu tiên value, sau đó dùng default_value từ field, sau đó dùng default từ user profile
    const displayValue = value ||
      field.position?.default_value ||
      (field.field_type === 'signature' ? defaultSignature :
        field.field_type === 'initials' ? defaultInitials :
          undefined);
    // Nếu có displayValue, render theo field type
    if (displayValue) {
      switch (field.field_type) {
        case 'signature':
        case 'initials':
          return (
            <SignatureRenderer
              fieldType={field.field_type}
              data={displayValue}
              width={field.position.width * 600}
              height={field.position.height * 800}
              submitterId={submitterId}
              submitterEmail={submitterEmail}
              reason={reason}
              globalSettings ={globalSettings}
            />
          );

        case 'image':
          return (
            <div className="w-full h-full">
              <img
                src={displayValue}
                alt="Uploaded"
                className="w-full h-full object-contain"
              />
            </div>
          );

        case 'file':
          return (
            <div className="w-full h-full flex items-center overflow-hidden">
              <span className="text-xs truncate" title={decodeURIComponent(displayValue.split('/').pop() || 'File')}>
                {decodeURIComponent(displayValue.split('/').pop() || 'File')}
              </span>
            </div>
          );

        case 'checkbox':
          return (
            <div className="w-full h-full">
              <div className={`w-full h-full ${displayValue === 'true' ? 'bg-indigo-600' : ''}`}>
                {displayValue === 'true' && (
                  <svg className="w-full h-full text-white p-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={3} d="M5 13l4 4L19 7" />
                  </svg>
                )}
              </div>
            </div>
          );

        case 'radio':
        case 'select':
          return (
            <div className="w-full h-full flex items-center justify-center">
              <span className="truncate text-sm">{displayValue}</span>
            </div>
          );

        case 'multiple':
          return (
            <div className="w-full h-full flex items-center justify-center px-2 overflow-hidden">
              <span className="text-xs truncate">{displayValue}</span>
            </div>
          );

        case 'cells':
          return (
            <div
              className="w-full h-full grid overflow-hidden"
              style={{
                gridTemplateColumns: field.options?.widths?.map((w: number) => `${w}fr`).join(' ') || '1fr 1fr 1fr'
              }}
            >
              {Array.from({ length: field.options?.columns || 3 }, (_, i) => {
                const char = displayValue?.[i] || '';
                return (
                  <div key={i} className="border border-gray-400 flex items-center justify-end text-base font-bold px-1">
                    {char}
                  </div>
                );
              })}
            </div>
          );

        default:
          return displayValue;
      }
    }

    // Nếu không có value, hiển thị preview/placeholder
    if (field.field_type === 'cells') {
      return (
        <div
          className="w-full h-full grid overflow-hidden"
          style={{
            gridTemplateColumns: field.options?.widths?.map((w: number) => `${w}fr`).join(' ') || '1fr 1fr 1fr'
          }}
        >
          {Array.from({ length: field.options?.columns || 3 }, (_, i) => (
            <div key={i} className="border border-gray-400 flex items-center justify-center text-xs bg-white bg-opacity-50">
              {i + 1}
            </div>
          ))}
        </div>
      );
    }

    // Icon placeholder cho các field type khác
    return (
      <div className='w-full h-full flex items-center justify-center text-black'>
        {fieldTools.find(ft => ft.type === field.field_type)?.iconComponent('w-6 h-6')}
      </div>
    );
  };

  return (
    <div
      className={`w-full h-full flex items-center ${className}`}
      style={style}
      onClick={onClick}
      title={title}
    >
      {renderFieldContent()}
    </div>
  );
};

export default FieldRenderer;
