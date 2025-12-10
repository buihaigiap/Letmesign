import React from 'react';
import { Rnd } from 'react-rnd';
import FieldRenderer from '../../FieldRenderer';
import { GripVertical } from 'lucide-react';
import { getFieldClass, measureTextWidth } from '../utils';
import { partnerColorClasses } from '../partnerColors';
import { fieldTools } from '../constants';
import GripVerticalMenu from './GripVerticalMenu';

interface PdfEditorCanvasProps {
  overlayRef: React.RefObject<HTMLDivElement>;
  fieldRefs: React.MutableRefObject<Map<string, HTMLDivElement>>;
  fields: any[];
  currentPage: number;
  canvasClientWidth: number;
  canvasClientHeight: number;
  selectedFieldTempId: string | null;
  activeTool: any;
  resizingColumn: { tempId: string; index: number } | null;
  currentHandlePosition: number | null;
  currentRect: React.CSSProperties | null;
  editingFieldTempId: string | null;
  inputWidths: Record<string, number>;
  showPartnerDropdown: string | null;
  showToolDropdown: string | null;
  partners: string[];
  checkRole: boolean;
  hasAccess: boolean;
  getCurrentToolIcon: (fieldType: string, className?: string) => React.ReactElement;
  getPartnerColorClass: (partner: string) => string;
  handleOverlayMouseDown: (e: React.MouseEvent<HTMLDivElement>) => void;
  handleOverlayMouseMove: (e: React.MouseEvent<HTMLDivElement>) => void;
  handleOverlayMouseUp: (e: React.MouseEvent<HTMLDivElement>) => void;
  handleDragStop: (tempId: string, e: any, d: { x: number; y: number }) => void;
  handleResizeStop: (tempId: string, e: any, direction: string, ref: HTMLElement, delta: any, position: { x: number; y: number }) => void;
  setSelectedFieldTempId: (id: string | null) => void;
  setActiveTool: (tool: any) => void;
  setOriginalFieldName: (name: string) => void;
  setEditingFieldTempId: (id: string | null) => void;
  setInputWidths: React.Dispatch<React.SetStateAction<Record<string, number>>>;
  setShowPartnerDropdown: (id: string | null) => void;
  setShowToolDropdown: (id: string | null) => void;
  setResizingColumn: (value: { tempId: string; index: number } | null) => void;
  updateField: (tempId: string, updates: any) => void;
  deleteField: (tempId: string) => void;
  duplicateField: (tempId: string) => void;
  originalFieldName: string;
}

const PdfEditorCanvas: React.FC<PdfEditorCanvasProps> = ({
    overlayRef,fieldRefs,fields,currentPage,canvasClientWidth,canvasClientHeight,selectedFieldTempId,
    activeTool,resizingColumn,currentHandlePosition,currentRect,editingFieldTempId,inputWidths,
    showPartnerDropdown,showToolDropdown,partners,checkRole,hasAccess,getCurrentToolIcon,
    getPartnerColorClass,handleOverlayMouseDown,handleOverlayMouseMove,handleOverlayMouseUp,handleDragStop,
    handleResizeStop,setSelectedFieldTempId,setActiveTool,setOriginalFieldName,
    setEditingFieldTempId,setShowPartnerDropdown,setShowToolDropdown,setResizingColumn,
    updateField,deleteField,duplicateField,originalFieldName,
    setInputWidths
}) => {
  const updateInputWidth = (tempId: string, text: string) => {
    const width = measureTextWidth(text, '12px') + 16;
    setInputWidths(prev => ({ ...prev, [tempId]: Math.max(width, 24) }));
  };

  return (
    <div
      ref={overlayRef}
      className="absolute top-0 left-0 w-full h-full z-10"
      onMouseDown={handleOverlayMouseDown}
      onMouseMove={handleOverlayMouseMove}
      onMouseUp={handleOverlayMouseUp}
      style={{ cursor: activeTool !== 'cursor' ? 'crosshair' : 'default' }}
    >
      {fields.filter(f => f.position?.page === currentPage).map(f => {
        const isSelected = selectedFieldTempId === f.tempId;

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
                        // Nếu focus chuyển sang menu MUI hoặc grip menu, không ẩn UI
                        if (e.relatedTarget && (e.relatedTarget.closest('.MuiMenu-root') || e.relatedTarget.closest('.grip-menu'))) {
                          return;
                        }
                        // Nếu focus chuyển sang SVG element (như GripVertical icon), không ẩn UI
                        if (e.relatedTarget instanceof SVGElement) {
                          return;
                        }
                        // Nếu focus chuyển sang checkbox hoặc label, hoặc các element trong field-label, không ẩn UI
                        if (e.relatedTarget && ((e.relatedTarget as HTMLElement).tagName === 'LABEL' || (e.relatedTarget as HTMLElement).tagName === 'INPUT' || (e.relatedTarget as HTMLElement).closest('.field-label'))) {
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
                <GripVerticalMenu
                  tempId={f.tempId}
                  onDuplicate={duplicateField}
                  onDelete={deleteField}
                  defaultValue={f.position?.default_value || ''}
                  onDefaultValueChange={(tempId, value) => updateField(tempId, { position: { ...f.position, default_value: value } })}
                />
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
  );
};

export default PdfEditorCanvas;
