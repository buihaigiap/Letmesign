import React, { useState, useEffect, useRef, forwardRef, useImperativeHandle, useCallback } from 'react';
import { FieldType } from '../../types';
import upstashService from '../../ConfigApi/upstashService';
import { Rnd } from 'react-rnd';
import PdfDisplay, { PdfDisplayRef } from '../PdfDisplay';
import FieldRenderer from '../FieldRenderer';
import FieldTools from './FieldTools';
import PartnersPanel from './PartnersPanel';
import FieldProperties from './FieldProperties';
import { Type, PenTool, Hash, User, Calendar, CheckSquare, Circle, List, ChevronDown, Table, ImageIcon, File, MousePointer } from 'lucide-react';
import { partnerColorClasses } from './constants';
import { fieldTools } from './constants';
import { measureTextWidth, getFieldClass } from './utils';
import { useFieldManagement } from './hooks/useFieldManagement';
import { usePdfInitialization } from './hooks/usePdfInitialization';
import { canTemplate, useRoleAccess } from '@/hooks/useRoleAccess';
import { useAuth } from '../../contexts/AuthContext';
const DocumentEditor = forwardRef<any>(function DocumentEditor({ template, token }: any, ref) {
  const { user } = useAuth();
  const overlayRef = useRef<HTMLDivElement>(null);
  const pdfDisplayRef = useRef<PdfDisplayRef>(null);
  const fieldRefs = useRef(new Map<string, HTMLDivElement>());
  const [error, setError] = useState('');
  // const [scale] = useState(1.5);
  const [currentPage, setCurrentPage] = useState(1);
  const [pageWidth, setPageWidth] = useState(0);
  const [pageHeight, setPageHeight] = useState(0);
  const [canvasClientWidth, setCanvasClientWidth] = useState(0);
  const [canvasClientHeight, setCanvasClientHeight] = useState(0);
  const [fields, setFields] = useState<any[]>([]);
  const [selectedFieldTempId, setSelectedFieldTempId] = useState<string | null>(null);
  const [activeTool, setActiveTool] = useState<any>('cursor');
  const [lastFieldTool, setLastFieldTool] = useState<FieldType>('text');
  const [isDrawing, setIsDrawing] = useState(false);
  const [startPos, setStartPos] = useState({ x: 0, y: 0 });
  const [currentRect, setCurrentRect] = useState<React.CSSProperties | null>(null);
  const [originalFieldName, setOriginalFieldName] = useState<string>('');
  const [editingFieldTempId, setEditingFieldTempId] = useState<string | null>(null);
  const [inputWidths, setInputWidths] = useState<Record<string, number>>({});
  const [originalFields, setOriginalFields] = useState<Record<number, any>>({});
  const [deletedIds, setDeletedIds] = useState<Set<number>>(new Set());
  const [resizingColumn, setResizingColumn] = useState<{ tempId: string, index: number } | null>(null);
  const [currentHandlePosition, setCurrentHandlePosition] = useState<number | null>(null);
  const [partners, setPartners] = useState<string[]>([]);
  const [currentPartner, setCurrentPartner] = useState<string>('');
  const [showPartnerDropdown, setShowPartnerDropdown] = useState<string | null>(null);
  const [showToolDropdown, setShowToolDropdown] = useState<string | null>(null);
  const [isInitialized, setIsInitialized] = useState(false);
  const [isPdfLoaded, setIsPdfLoaded] = useState(false);
  const [globalSettings, setGlobalSettings] = useState<any>(null);
  const prevPartnersRef = useRef(partners);
  const checkRole = canTemplate(template)
  const hasAccess = useRoleAccess(['agent']);
  useEffect(() => {
    prevPartnersRef.current = partners;
  }, [partners]);
  // Custom hooks
  const { updateField, deleteField, handleSaveClick } = useFieldManagement(
    fields,
    setFields,
    originalFields,
    setOriginalFields,
    deletedIds,
    setDeletedIds,
    pageWidth,
    pageHeight,
    partners,
    token,
    template.id
  );

  const { initialTemplateIdRef: pdfInitRef, initialFieldsLengthRef: pdfFieldsRef } = usePdfInitialization(
    template,
    pageWidth,
    pageHeight,
    isInitialized,
    setIsInitialized,
    setFields,
    setOriginalFields,
    setPartners,
    setCurrentPartner,
    setDeletedIds,
    [],
    deletedIds,
    isPdfLoaded
  );

  // Expose saveFields method via ref
  useImperativeHandle(ref, () => ({
    saveFields: handleSaveClick,
    getPartners: () => partners,
    getFields: () => fields
  }));

  // Fetch global settings for logo display
  useEffect(() => {
    const fetchGlobalSettings = async () => {
      try {
        const response = await upstashService.getUserSettings();
        setGlobalSettings(response.data);
      } catch (error) {
        console.warn('Failed to fetch global settings:', error);
      }
    };

    if (user) {
      fetchGlobalSettings();
    }
  }, [user]);

  const updateInputWidth = (tempId: string, text: string) => {
    const width = measureTextWidth(text, '12px') + 16; // Add some padding
    setInputWidths(prev => ({ ...prev, [tempId]: Math.max(width, 24) })); // Minimum 24px
  };

  const getPartnerColorClass = (partner: string) => {
    const index = partners.indexOf(partner);
    return partnerColorClasses[index % partnerColorClasses.length];
  };

  const getCurrentToolIcon = (fieldType: string, className: string = 'w-4 h-4'): React.ReactElement => {
    switch (fieldType) {
      case 'text': return <Type className={className} />;
      case 'signature': return <PenTool className={className} />;
      case 'number': return <Hash className={className} />;
      case 'initials': return <User className={className} />;
      case 'date': return <Calendar className={className} />;
      case 'checkbox': return <CheckSquare className={className} />;
      case 'radio': return <Circle className={className} />;
      case 'multiple': return <List className={className} />;
      case 'select': return <ChevronDown className={className} />;
      case 'cells': return <Table className={className} />;
      case 'image': return <ImageIcon className={className} />;
      case 'file': return <File className={className} />;
      case 'cursor': return <MousePointer className={className} />;
      default: return <Type className={className} />;
    }
  };

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (showPartnerDropdown && !(e.target as Element).closest('.partner-dropdown')) {
        setShowPartnerDropdown(null);
      }
      if (showToolDropdown && !(e.target as Element).closest('.tool-dropdown')) {
        setShowToolDropdown(null);
      }
      // Hide editing UI when clicking outside field label
      if (editingFieldTempId && !(e.target as Element).closest('.field-label')) {
        setEditingFieldTempId(null);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [showPartnerDropdown, showToolDropdown, editingFieldTempId]);

  const updateCanvasDimensions = () => {
    setCanvasClientWidth(pdfDisplayRef.current?.getCanvasClientWidth() || 0);
    setCanvasClientHeight(pdfDisplayRef.current?.getCanvasClientHeight() || 0);
  };

  useEffect(() => {
    const handleResize = () => updateCanvasDimensions();
    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

  useEffect(() => {
    updateCanvasDimensions();
  }, [currentPage, isPdfLoaded]);

  useEffect(() => {
    if (overlayRef.current) {
      updateCanvasDimensions();
    }
  }, [overlayRef.current]);

  const handleOverlayMouseDown = (e: React.MouseEvent<HTMLDivElement>) => {
    if (resizingColumn) return;
    if ((activeTool === 'cursor' && !e.shiftKey) || e.target !== overlayRef.current) return;
    e.preventDefault();
    setIsDrawing(true);
    setSelectedFieldTempId(null);
    const rect = overlayRef.current!.getBoundingClientRect();
    setStartPos({ x: e.clientX - rect.left, y: e.clientY - rect.top });
  };

  const handleOverlayMouseMove = (e: React.MouseEvent<HTMLDivElement>) => {
    if (resizingColumn) {
      const field = fields.find(f => f.tempId === resizingColumn.tempId);
      if (!field || field.field_type !== 'cells') return;
      const rect = overlayRef.current!.getBoundingClientRect();
      const mouseX = e.clientX - rect.left;
      // Position is already in decimal (0-1), so multiply by canvasClientWidth directly
      const fieldX = field.position!.x * canvasClientWidth;
      const fieldWidth = field.position!.width * canvasClientWidth;

      if (resizingColumn.index === -1) {
        // Creating columns by dragging - limit handle position within field boundaries
        const minCellWidth = 10; // Minimum width per cell in pixels
        const maxColumns = Math.floor(fieldWidth / minCellWidth); // Calculate max columns based on field width

        const newHandlePos = Math.max(0, Math.min(fieldWidth, mouseX - fieldX)); // Clamp within 0 to fieldWidth
        const newRatio = Math.max(0.001, Math.min(1, newHandlePos / fieldWidth));
        setCurrentHandlePosition(newRatio);

        const calculatedColumns = Math.round(1 / newRatio);
        const numColumns = Math.max(1, Math.min(maxColumns, calculatedColumns)); // Limit by maxColumns based on width

        if (numColumns !== field.options.columns) {
          updateField(field.tempId, { options: { columns: numColumns, widths: Array(numColumns).fill(1) } });
        }
      }
      return;
    }
    if (!isDrawing) return;
    const rect = overlayRef.current!.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    setCurrentRect({
      position: 'absolute',
      left: Math.min(x, startPos.x),
      top: Math.min(y, startPos.y),
      width: Math.abs(x - startPos.x),
      height: Math.abs(y - startPos.y),
      border: '2px dashed #a5b4fc',
      backgroundColor: 'rgba(99, 102, 241, 0.2)',
    });
  };

  const handleOverlayMouseUp = (e: React.MouseEvent<HTMLDivElement>) => {
    if (resizingColumn) {
      setResizingColumn(null);
      setCurrentHandlePosition(null);
      return;
    }
    if (!isDrawing || (activeTool === 'cursor' && !e.shiftKey)) return;
    setIsDrawing(false);
    setCurrentRect(null);

    const rect = overlayRef.current!.getBoundingClientRect();
    const endX = e.clientX - rect.left;
    const endY = e.clientY - rect.top;
    const width = Math.abs(endX - startPos.x);
    const height = Math.abs(endY - startPos.y);
    // Allow smaller height, only require minimum width
    if (width < 20 || height < 5) return;

    if (partners.length === 0) {
      alert('Please add at least one partner before creating fields.');
      return;
    }

    const displayWidth = canvasClientWidth || rect.width || 600;
    const displayHeight = canvasClientHeight || rect.height || 800;

    // Tính tỷ lệ thập phân (0-1) thay vì phần trăm (0-100)
    const x = Math.max(0, Math.min(1, Math.min(startPos.x, endX) / displayWidth));
    const y = Math.max(0, Math.min(1, Math.min(startPos.y, endY) / displayHeight));
    const w = Math.max(0.01, Math.min(1 - x, width / displayWidth));
    const h = Math.max(0.01, Math.min(1 - y, height / displayHeight));

    const fieldType: FieldType = activeTool === 'cursor' && e.shiftKey ? lastFieldTool : (activeTool === 'cursor' ? lastFieldTool : activeTool as FieldType);
    const newField: any = {
      tempId: `new-${Date.now()}`,
      name: `${fieldType}_${fields.filter(f => f.field_type === fieldType).length + 1}`,
      field_type: fieldType,
      required: true,
      display_order: fields.length + 1,
      position: {
        x: x,
        y: y,
        width: w,
        height: h,
        page: currentPage
      },
      ...(fieldType === 'radio' && { options: ['Option 1', 'Option 2'] }),
      ...(fieldType === 'multiple' && { options: ['Option 1', 'Option 2'] }),
      ...(fieldType === 'select' && { options: ['Option 1', 'Option 2'] }),
      ...(fieldType === 'cells' && { options: { columns: 3, widths: [1, 1, 1] } }),
      partner: currentPartner
    };
    setFields(prev => [...prev, newField]);
    setSelectedFieldTempId(newField.tempId);
    setActiveTool('cursor');
  };

  const handleDragStop = (tempId: string, e: any, d: { x: number; y: number }) => {
    const field = fields.find(f => f.tempId === tempId);
    if (!field || !field.position) return;

    const displayWidth = canvasClientWidth || 600;
    const displayHeight = canvasClientHeight || 800;

    // Tính tỷ lệ thập phân (0-1)
    const x = Math.max(0, Math.min(1 - field.position.width, d.x / displayWidth));
    const y = Math.max(0, Math.min(1 - field.position.height, d.y / displayHeight));

    let newPos = {
      ...field.position,
      x: x,
      y: y
    };
    updateField(tempId, { position: newPos });
  };

  const handleResizeStop = (tempId: string, e: any, direction: string, ref: HTMLElement, delta: any, position: { x: number; y: number }) => {
    const field = fields.find(f => f.tempId === tempId);
    if (!field || !field.position) return;

    const displayWidth = canvasClientWidth || 600;
    const displayHeight = canvasClientHeight || 800;

    // Tính tỷ lệ thập phân (0-1)
    const x = Math.max(0, position.x / displayWidth);
    const y = Math.max(0, position.y / displayHeight);
    const width = Math.max(0.01, Math.min(1 - x, ref.offsetWidth / displayWidth));
    const height = Math.max(0.01, Math.min(1 - y, ref.offsetHeight / displayHeight));

    let newPos = {
      ...field.position,
      x: x,
      y: y,
      width: width,
      height: height
    };
    updateField(tempId, { position: newPos });
  };

  if (error) return <div className="text-red-400 bg-gray-800 p-4 rounded break-words">{error}</div>;

  return (
    <div className="grid grid-cols-1 lg:grid-cols-3 ">
      <div className="lg:col-span-2">
        <PdfDisplay
          ref={pdfDisplayRef}
          filePath={template.file_url}
          token={token}
          page={currentPage}
          onPageChange={(newPage: number) => setCurrentPage(newPage)}
          onLoad={() => {
            const pageW = pdfDisplayRef.current?.getPageWidth() || 0;
            const pageH = pdfDisplayRef.current?.getPageHeight() || 0;
            const canvasW = pdfDisplayRef.current?.getCanvasClientWidth() || 0;
            const canvasH = pdfDisplayRef.current?.getCanvasClientHeight() || 0;

            setPageWidth(pageW);
            setPageHeight(pageH);
            setCanvasClientWidth(canvasW);
            setCanvasClientHeight(canvasH);
            setIsPdfLoaded(true);
          }}
          globalSettings={globalSettings}
        >
          <div ref={overlayRef} className="absolute top-0 left-0 w-full h-full z-10" onMouseDown={handleOverlayMouseDown} onMouseMove={handleOverlayMouseMove} onMouseUp={handleOverlayMouseUp} style={{ cursor: activeTool !== 'cursor' ? 'crosshair' : 'default' }}>
            {fields.filter(f => f.position?.page === currentPage).map(f => {
              const isSelected = selectedFieldTempId === f.tempId;

              const resizeHandles = ['nw', 'n', 'ne', 'e', 'se', 's', 'sw', 'w'];
              const handleCursors: { [key: string]: string } = {
                'nw': 'nwse-resize', 'n': 'ns-resize', 'ne': 'nesw-resize', 'e': 'ew-resize',
                'se': 'nwse-resize', 's': 'ns-resize', 'sw': 'nesw-resize', 'w': 'ew-resize',
              };

              return (
                <Rnd
                  key={f.tempId}
                  size={{
                    width: f.position!.width * (canvasClientWidth || 600),
                    height: f.position!.height * (canvasClientHeight || 800)
                  }}
                  position={{
                    x: f.position!.x * (canvasClientWidth || 600),
                    y: f.position!.y * (canvasClientHeight || 800)
                  }}
                  onDragStop={(e, d) => handleDragStop(f.tempId, e, d)}
                  onResizeStop={(e, direction, ref, delta, position) => handleResizeStop(f.tempId, e, direction, ref, delta, position)}
                  dragAxis="both"
                  bounds="parent"
                  className={getFieldClass(f.partner, isSelected, partnerColorClasses)}
                  enableResizing={isSelected ? (f.field_type === 'cells' ? {
                    top: true,
                    right: true,
                    bottom: false,
                    left: true,
                    topRight: true,
                    bottomRight: false,
                    bottomLeft: false,
                    topLeft: true
                  } : true) : false}
                  minWidth={20}
                  minHeight={5}
                  style={{ cursor: activeTool === 'cursor' ? 'move' : 'default' }}
                >
                  {/* Field name label above the box */}
                  {isSelected && (
                    <div
                      className="absolute z-11 bg-black bg-opacity-50 p-1 select-none flex items-center field-label"
                      style={{
                        top: -30,
                        left: -4,
                        fontSize: '12px',
                        color: 'white'
                      }}
                      onClick={(e) => {
                        e.stopPropagation();
                        setSelectedFieldTempId(f.tempId);
                        setActiveTool('cursor');
                        setOriginalFieldName(f.name);
                        updateInputWidth(f.tempId, f.name);
                        setEditingFieldTempId(f.tempId);
                      }}
                    >
                      <div
                        className={`w-3 h-3 rounded-full mr-1 cursor-pointer ${(getPartnerColorClass(f.partner) || 'bg-gray-500 text-gray-900').split(' ')[0]}`}
                        onClick={(e) => {
                          e.stopPropagation();
                          setShowPartnerDropdown(showPartnerDropdown === f.tempId ? null : f.tempId);
                        }}
                        title={`Partner: ${f.partner}`}
                      />
                      {/* Field Type Icon Selector */}
                      <div
                        className="mr-1 cursor-pointer  rounded hover:bg-white hover:bg-opacity-10 w-4 h-4 flex items-center justify-center"
                        onClick={(e) => {
                          e.stopPropagation();
                          setShowToolDropdown(showToolDropdown === f.tempId ? null : f.tempId);
                        }}
                        title={`Field Type: ${f.field_type}`}
                      >
                        {getCurrentToolIcon(f.field_type)}
                      </div>
                      {editingFieldTempId === f.tempId ? (
                        <>
                          <input
                            type="text"
                            value={f.name}
                            onChange={e => {
                              updateField(f.tempId, { name: e.target.value });
                              updateInputWidth(f.tempId, e.target.value);
                            }}
                            onKeyDown={e => {
                              if (e.key === 'Enter') {
                                setEditingFieldTempId(null);
                              } else if (e.key === 'Escape') {
                                updateField(f.tempId, { name: originalFieldName });
                                updateInputWidth(f.tempId, originalFieldName);
                                setEditingFieldTempId(null);
                              }
                            }}
                            onClick={e => e.stopPropagation()}
                            onBlur={e => {
                              // Nếu focus chuyển sang checkbox hoặc label, không ẩn UI
                              if (e.relatedTarget && ((e.relatedTarget as HTMLElement).tagName === 'LABEL' || (e.relatedTarget as HTMLElement).tagName === 'INPUT')) {
                                return;
                              }
                              if (!e.target.value.trim()) {
                                updateField(f.tempId, { name: originalFieldName });
                                updateInputWidth(f.tempId, originalFieldName);
                              }
                              setEditingFieldTempId(null);
                            }}
                            style={{ width: `${inputWidths[f.tempId] || 24}px` }}
                            className="bg-transparent border-none outline-none text-white font-medium text-xs"
                            autoFocus
                          />
                          {/* Required Checkbox */}
                          <label className="ml-2 flex items-center cursor-pointer" title="Required">
                            <input
                              type="checkbox"
                              checked={f.required}
                              onChange={e => updateField(f.tempId, { required: e.target.checked })}
                              onClick={e => e.stopPropagation()}
                              className="w-3 h-3 text-indigo-600 bg-transparent border border-white border-opacity-30 rounded focus:ring-indigo-500 mr-1"
                            />
                            <span className="text-xs text-white">Required</span>
                          </label>
                        </>
                      ) : (
                        f.name
                      )}
                      <button
                        onClick={(e) => { e.stopPropagation(); deleteField(f.tempId); }}
                        className="ml-2 w-3 h-3 bg-red-500 hover:bg-red-600 text-white text-xs rounded-full flex items-center justify-center"
                        title="Delete field"
                      >
                        ×
                      </button>
                      {showToolDropdown === f.tempId && (
                        <div className="tool-dropdown absolute top-full left-0 mt-1 bg-white border border-gray-300 rounded shadow-lg z-20" style={{ width: 'auto', minWidth: '150px' }}>
                          {fieldTools.filter(t => t.type !== 'cursor').map(tool => (
                            <div
                              key={tool.type}
                              className="px-2 py-1 hover:bg-gray-100 cursor-pointer text-xs text-black flex items-center"
                              onClick={(e) => {
                                e.stopPropagation();
                                const newType = tool.type as any;
                                const baseName = tool.name;
                                const existingCount = fields.filter(field => field.field_type === newType && field.tempId !== f.tempId).length + 1;
                                const newName = `${baseName}_${existingCount}`;
                                updateField(f.tempId, { field_type: newType, name: newName });
                                updateInputWidth(f.tempId, newName);
                                setShowToolDropdown(null);
                              }}
                            >
                              {tool.iconComponent('w-4 h-4 mr-2')}
                              {tool.name}
                            </div>
                          ))}
                        </div>
                      )}
                      {showPartnerDropdown === f.tempId && checkRole && !hasAccess && (
                        <div className="partner-dropdown absolute top-full left-0 mt-1 bg-white border border-gray-300 rounded shadow-lg z-20" style={{ width: 'auto' }}>
                          {partners.filter(p => p !== f.partner).map(p => (
                            <div
                              key={p}
                              className="px-2 py-1 hover:bg-gray-100 cursor-pointer text-xs text-black flex items-center"
                              onClick={(e) => {
                                e.stopPropagation();
                                updateField(f.tempId, { partner: p });
                                setShowPartnerDropdown(null);
                              }}
                            >
                              <div className={`w-3 h-3 rounded-full mr-2 ${(getPartnerColorClass(p) || 'bg-gray-500 text-gray-900').split(' ')[0]}`} />
                              {p}
                            </div>
                          ))}
                        </div>
                      )}
                    </div>
                  )}
                  <div
                    ref={el => { if (el) fieldRefs.current.set(f.tempId, el); else fieldRefs.current.delete(f.tempId); }}
                    onClick={(e) => {
                      e.stopPropagation();
                      setSelectedFieldTempId(f.tempId);
                      setActiveTool('cursor');
                    }}
                    className="w-full h-full relative"
                  >
                    {f.field_type === 'cells' ? (
                      <>
                        <FieldRenderer
                          field={f}
                        />
                        {/* Bottom bar with single handle for column resizing */}
                        <div className="absolute bottom-0 left-0 right-0 h-4 bg-gray-200 flex items-center justify-center overflow-hidden">
                          <div
                            className="absolute w-3 h-3 bg-blue-500 rounded-full cursor-ew-resize border border-white"
                            style={{
                              left: `${Math.max(0, Math.min(100, ((resizingColumn?.tempId === f.tempId && currentHandlePosition !== null) ? currentHandlePosition : (1 / f.options.columns)) * 100))}%`,
                              top: '50%',
                              transform: 'translate(-50%, -50%)'
                            }}
                            onMouseDown={(e) => { e.stopPropagation(); setResizingColumn({ tempId: f.tempId, index: -1 }); }}
                          ></div>
                        </div>
                      </>
                    ) : (
                      <FieldRenderer
                        field={f}
                      />
                    )}
                  </div>
                </Rnd>
              );
            })}
            {currentRect && <div style={currentRect}></div>}
          </div>
        </PdfDisplay>
      </div>

      <div className="lg:col-span-1 text-black p-4 rounded-lg space-y-6">
        <PartnersPanel
          checkRole={checkRole}
          hasAccess={hasAccess}
          getPartnerColorClass={getPartnerColorClass}
          partners={partners}
          setPartners={setPartners}
          currentPartner={currentPartner}
          setCurrentPartner={setCurrentPartner}
          fields={fields}
          setFields={setFields}
        />

        <FieldProperties
          getCurrentToolIcon={getCurrentToolIcon}
          fields={fields}
          currentPartner={currentPartner}
          selectedFieldTempId={selectedFieldTempId}
          setSelectedFieldTempId={setSelectedFieldTempId}
          updateField={updateField}
          deleteField={deleteField}
        />
        {checkRole && !hasAccess && (
          <div>
            <FieldTools
              activeTool={activeTool}
              setActiveTool={setActiveTool}
              setLastFieldTool={setLastFieldTool}
            />
          </div>
        )}

      </div>
    </div>
  );
});

export default DocumentEditor;
