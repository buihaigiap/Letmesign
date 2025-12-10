import React, { useState, useEffect, useRef, forwardRef, useImperativeHandle } from 'react';
import { FieldType } from '../../types';
import upstashService from '../../ConfigApi/upstashService';
import PdfDisplay, { PdfDisplayRef } from '../PdfDisplay';
import FieldTools from './FieldTools';
import PartnersPanel from './PartnersPanel';
import FieldProperties from './FieldProperties';
import PdfEditorCanvas from './PdfEditorCanvas/PdfEditorCanvas';
import { getPartnerColorClass } from './partnerColors';
import { getCurrentToolIcon } from './constants';
import { useFieldManagement } from './hooks/useFieldManagement';
import { usePdfInitialization } from './hooks/usePdfInitialization';
import { useCanvasInteractions } from './hooks/useCanvasInteractions';
import { canTemplate, useRoleAccess } from '@/hooks/useRoleAccess';
import { useAuth } from '../../contexts/AuthContext';
const DocumentEditor = forwardRef<any>(function DocumentEditor({ template, token }: any, ref) {
  const { user } = useAuth();
  // Refs
  const overlayRef = useRef<HTMLDivElement>(null);
  const pdfDisplayRef = useRef<PdfDisplayRef>(null);
  const fieldRefs = useRef(new Map<string, HTMLDivElement>());
  const prevPartnersRef = useRef<string[]>([]);
  // PDF State
  const [error, setError] = useState('');
  const [currentPage, setCurrentPage] = useState(1);
  const [pageWidth, setPageWidth] = useState(0);
  const [pageHeight, setPageHeight] = useState(0);
  const [canvasClientWidth, setCanvasClientWidth] = useState(0);
  const [canvasClientHeight, setCanvasClientHeight] = useState(0);
  const [isPdfLoaded, setIsPdfLoaded] = useState(false);
  const [isInitialized, setIsInitialized] = useState(false);
  const [globalSettings, setGlobalSettings] = useState<any>(null);
  // Fields State
  const [fields, setFields] = useState<any[]>([]);
  const [originalFields, setOriginalFields] = useState<Record<number, any>>({});
  const [deletedIds, setDeletedIds] = useState<Set<number>>(new Set());
  const [selectedFieldTempId, setSelectedFieldTempId] = useState<string | null>(null);
  // Tools State
  const [activeTool, setActiveTool] = useState<any>('cursor');
  const [lastFieldTool, setLastFieldTool] = useState<FieldType>('text');
  
  // Editing State
  const [originalFieldName, setOriginalFieldName] = useState<string>('');
  const [editingFieldTempId, setEditingFieldTempId] = useState<string | null>(null);
  const [inputWidths, setInputWidths] = useState<Record<string, number>>({});
  // Resizing State
  const [resizingColumn, setResizingColumn] = useState<{ tempId: string, index: number } | null>(null);
  const [currentHandlePosition, setCurrentHandlePosition] = useState<number | null>(null);
  // Partners State
  const [partners, setPartners] = useState<string[]>([]);
  const [currentPartner, setCurrentPartner] = useState<string>('');
  // Dropdown State
  const [showPartnerDropdown, setShowPartnerDropdown] = useState<string | null>(null);
  const [showToolDropdown, setShowToolDropdown] = useState<string | null>(null);
  const checkRole = canTemplate(template);
  const hasAccess = useRoleAccess(['agent']);

  useEffect(() => {
    prevPartnersRef.current = partners;
  }, [partners]);
  // Custom hooks
  const { updateField, deleteField, duplicateField, handleSaveClick } = useFieldManagement(
    fields,setFields,originalFields,setOriginalFields,deletedIds,
    setDeletedIds,pageWidth,pageHeight,partners,token,template.id
  );

  const { initialTemplateIdRef: pdfInitRef, initialFieldsLengthRef: pdfFieldsRef } = usePdfInitialization(
    template,pageWidth,pageHeight,isInitialized,setIsInitialized,setFields,
    setOriginalFields,setPartners,setCurrentPartner,setDeletedIds,
    [],deletedIds,isPdfLoaded
  );

  const {isDrawing,currentRect,handleOverlayMouseDown,handleOverlayMouseMove,handleOverlayMouseUp,handleDragStop,handleResizeStop,
  } = useCanvasInteractions({
    overlayRef,fields,setFields,currentPage,canvasClientWidth,canvasClientHeight,
    activeTool,lastFieldTool,resizingColumn,setResizingColumn,setCurrentHandlePosition,
    setSelectedFieldTempId,setActiveTool,updateField,currentPartner,partners,
  });

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

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      // Nếu click vào SVG element (như GripVertical icon), không ẩn UI
      if (e.target instanceof SVGElement) {
        return;
      }
      if (showPartnerDropdown && !(e.target as Element).closest('.partner-dropdown')) {
        setShowPartnerDropdown(null);
      }
      if (showToolDropdown && !(e.target as Element).closest('.tool-dropdown')) {
        setShowToolDropdown(null);
      }
      // Hide editing UI when clicking outside field label - removed to allow GripVertical menu
      // if (editingFieldTempId && !(e.target as Element).closest('.field-label')) {
      //   setEditingFieldTempId(null);
      // }
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
          <PdfEditorCanvas
            overlayRef={overlayRef}
            fieldRefs={fieldRefs}
            fields={fields}
            currentPage={currentPage}
            canvasClientWidth={canvasClientWidth}
            canvasClientHeight={canvasClientHeight}
            selectedFieldTempId={selectedFieldTempId}
            activeTool={activeTool}
            resizingColumn={resizingColumn}
            currentHandlePosition={currentHandlePosition}
            currentRect={currentRect}
            editingFieldTempId={editingFieldTempId}
            inputWidths={inputWidths}
            showPartnerDropdown={showPartnerDropdown}
            showToolDropdown={showToolDropdown}
            partners={partners}
            checkRole={checkRole}
            hasAccess={hasAccess}
            getCurrentToolIcon={getCurrentToolIcon}
            getPartnerColorClass={(partner) => getPartnerColorClass(partner, partners)}
            handleOverlayMouseDown={handleOverlayMouseDown}
            handleOverlayMouseMove={handleOverlayMouseMove}
            handleOverlayMouseUp={handleOverlayMouseUp}
            handleDragStop={handleDragStop}
            handleResizeStop={handleResizeStop}
            setSelectedFieldTempId={setSelectedFieldTempId}
            setActiveTool={setActiveTool}
            setOriginalFieldName={setOriginalFieldName}
            setEditingFieldTempId={setEditingFieldTempId}
            setShowPartnerDropdown={setShowPartnerDropdown}
            setShowToolDropdown={setShowToolDropdown}
            setResizingColumn={setResizingColumn}
            setInputWidths={setInputWidths}
            updateField={updateField}
            deleteField={deleteField}
            duplicateField={duplicateField}
            originalFieldName={originalFieldName}
          />
        </PdfDisplay>
      </div>

      <div className="lg:col-span-1 text-black p-4 rounded-lg space-y-6">
        <PartnersPanel
          checkRole={checkRole}
          hasAccess={hasAccess}
          getPartnerColorClass={(partner) => getPartnerColorClass(partner, partners)}
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
