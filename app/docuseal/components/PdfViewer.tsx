import { useState, useRef, useEffect } from "react";
import PdfDisplay, { PdfDisplayRef } from "./PdfDisplay";
import SignatureRenderer from "./SignatureRenderer";

interface DocumentViewerProps {
  documentUrl?: string;
  filePath?: string;
  token?: string | null;
  fields?: any[];
  texts?: Record<number, string>;
  onFieldClick?: (field: any) => void;
  page?: number;
  onPageChange?: (page: number) => void;
  scale?: number;
  showDebug?: boolean;
  submitterId?: number;
  submitterEmail?: string;
  globalSettings?: any;
}

const DocumentViewer: React.FC<DocumentViewerProps> = ({
  documentUrl,
  filePath,
  token,
  fields = [],
  texts = {},
  onFieldClick,
  page,
  onPageChange,
  scale: initialScale = 1.5,
  submitterId,
  submitterEmail,
  globalSettings
}) => {
  const [currentPage, setCurrentPage] = useState(page || 1);
  const [scale, setScale] = useState(initialScale);
  const pdfRef = useRef<PdfDisplayRef>(null);
  const handlePageChange = (newPage: number) => {
    setCurrentPage(newPage); 
    if (onPageChange) onPageChange(newPage);
  };
  
  const updateScale = () => {
    if (pdfRef.current) {
      const displayedHeight = pdfRef.current.getCanvasClientHeight();
      const pdfHeight = 792; // Letter height in points
      if (pdfHeight > 0) {
        setScale(displayedHeight / pdfHeight);
      }
    }
  };

  // Update scale when window resizes
  useEffect(() => {
    const handleResize = () => {
      updateScale();
    };

    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

  // Normalize field position to decimal (0-1) if it's in pixels
  const normalizePosition = (position: any) => {
    if (!position || typeof position.x !== 'number') return position;
    
    const pageWidth = 600; // Default A4 width in pixels
    const pageHeight = 800; // Default A4 height in pixels
    
    // Check if position is in pixels (values > 1) or already in decimal (0-1)
    if (position.x > 1 || position.y > 1 || position.width > 1 || position.height > 1) {
      // Position is in pixels, convert to decimal (0-1)
      return {
        ...position,
        x: position.x / pageWidth,
        y: position.y / pageHeight,
        width: position.width / pageWidth,
        height: position.height / pageHeight
      };
    }
    // Already in decimal format
    return position;
  };

  return (
    <div className="flex flex-col items-center">
      <PdfDisplay
        documentUrl={documentUrl}
        filePath={filePath}
        token={token}
        // scale={initialScale}
        page={currentPage}
        onPageChange={handlePageChange}
        onLoad={updateScale}
        ref={pdfRef}
        globalSettings={globalSettings}
      >
        {fields.filter(f => f?.position?.page === currentPage)?.map((f, index) => {
          // Normalize position to decimal (0-1)
          const normalizedPos = normalizePosition(f.position);
          // Position data is in relative coordinates (0-1), scale converts to display pixels
          const isNarrow = normalizedPos.height > 0 && (normalizedPos.width / normalizedPos.height) > 6;
          
          // Gộp logic: ưu tiên signature_value, fallback sang texts[f.id]
          const displayValue = (f as any).signature_value || texts[f.id];
          const fieldType = (f as any).field_type;
          
          return (
            <div
              key={f.id}
              // className={`absolute ${(f as any).signature_value ? '' : 'border-2 border-blue-500 bg-blue-500 bg-opacity-20 hover:bg-opacity-40 cursor-pointer'}`}
              style={{
                position: 'absolute',
                left: `${normalizedPos.x * 100}%`,
                top: `${normalizedPos.y * 100}%`,
                width: `${normalizedPos.width * 100}%`,
                height: `${normalizedPos.height * 100}%`,
                fontSize: '16px',
                color: 'black',
                fontWeight: 'bold'
              }}
            onClick={() => !(f as any).signature_value && onFieldClick && onFieldClick(f)}
          >
            <div className={`w-full h-full flex ${fieldType === "initials" ? " items-start" : "items-center "} text-md text-black font-semibold`}>
              {displayValue ? (
                fieldType === 'file' ? (
                  <a 
                    href={displayValue} 
                    download 
                    className="text-black underline cursor-pointer text-xs"
                    onClick={(e) => e.stopPropagation()}
                  >
                    {decodeURIComponent(displayValue.split('/').pop() || 'File')}
                  </a>
                ) : fieldType === 'cells' ? (
                  <div className="w-full h-full grid overflow-hidden" style={{ gridTemplateColumns: (f as any).options?.widths?.map((w: number) => `${w}fr`).join(' ') || '1fr 1fr 1fr' }}>
                    {Array.from({ length: (f as any).options?.columns || 3 }, (_, i) => {
                      const char = displayValue?.[i] || '';
                      return (
                        <div key={i} className="border border-gray-400 flex items-center justify-end text-base font-bold px-1">
                          {char}
                        </div>
                      );
                    })}
                  </div>
                ) : fieldType === 'multiple' ? (
                  <div className="w-full h-full flex items-center text-sm font-semibold">
                    {displayValue.split(',').join(', ')}
                  </div>
                ) : fieldType === 'image' || displayValue.startsWith('data:image/') || displayValue.startsWith('blob:') || displayValue.startsWith('http') ? (
                  <img 
                    src={displayValue} 
                    alt={fieldType === 'image' ? "Uploaded image" : "Signature"} 
                    className="object-contain mx-auto w-full h-full"
                  />
                ) : displayValue.startsWith('[') || displayValue.startsWith('{') || fieldType === 'signature' ? (
                  <SignatureRenderer 
                    data={displayValue} 
                    width={normalizedPos.width * 600} 
                    height={normalizedPos.height * 800}
                    fieldType={fieldType}
                    submitterId={submitterId}
                    submitterEmail={submitterEmail}
                    reason={(f as any).reason}
                    globalSettings={globalSettings}
                  />
                ) : fieldType === 'checkbox' ? (
                  displayValue === 'true' ? (
                    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" className="w-full h-full"><path d="M19 3H5C3.9 3 3 3.9 3 5V19C3 20.1 3.9 21 5 21H19C20.1 21 21 20.1 21 19V5C21 3.9 20.1 3 19 3ZM10 17L5 12L6.41 10.59L10 14.17L17.59 6.58L19 8L10 17Z" fill="currentColor"/></svg>
                  ) : (
                    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" className="w-full h-full"><rect x="3" y="3" width="18" height="18" rx="2" stroke="currentColor" strokeWidth="2"/></svg>
                  )
                ) : (
                  <span 
                    className="text-sm"
                    style={fieldType === "initials" ? 
                      { 
                        display: 'block',
                        position: 'absolute',
                        height: '100%', 
                        fontFamily: 'Helvetica, Arial, sans-serif', 
                        fontStyle: 'normal', 
                        fontWeight: 'normal', 
                        lineHeight: `${normalizedPos.height * 800}px` 
                      } : { whiteSpace: 'pre', fontFamily: 'Helvetica, Arial, sans-serif' }
                    }
                  >
                    {displayValue}
                  </span>
                )
              ) : fieldType === 'radio' ? (
                <div className="w-full h-full flex items-center text-sm font-semibold">
                  {texts[f.id] || `Select ${(f as any).name}`}
                </div>
              ) : (
                <span className="text-sm">{f.name}</span>
              )}
            </div>
          </div>
        );
        })}
      </PdfDisplay>
      {/* <Box>

      </Box> */}
    </div>
  );
};

export default DocumentViewer;
