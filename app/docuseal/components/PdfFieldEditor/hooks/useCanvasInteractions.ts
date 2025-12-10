import { useState, useCallback } from 'react';
import { FieldType } from '../../../types';

interface UseCanvasInteractionsProps {
  overlayRef: React.RefObject<HTMLDivElement>;
  fields: any[];
  setFields: (fields: any[] | ((prev: any[]) => any[])) => void;
  currentPage: number;
  canvasClientWidth: number;
  canvasClientHeight: number;
  activeTool: any;
  lastFieldTool: FieldType;
  resizingColumn: { tempId: string; index: number } | null;
  setResizingColumn: (value: { tempId: string; index: number } | null) => void;
  setCurrentHandlePosition: (pos: number | null) => void;
  setSelectedFieldTempId: (id: string | null) => void;
  setActiveTool: (tool: any) => void;
  updateField: (tempId: string, updates: any) => void;
  currentPartner: string;
  partners: string[];
}

export const useCanvasInteractions = ({
  overlayRef,
  fields,
  setFields,
  currentPage,
  canvasClientWidth,
  canvasClientHeight,
  activeTool,
  lastFieldTool,
  resizingColumn,
  setResizingColumn,
  setCurrentHandlePosition,
  setSelectedFieldTempId,
  setActiveTool,
  updateField,
  currentPartner,
  partners,
}: UseCanvasInteractionsProps) => {
  const [isDrawing, setIsDrawing] = useState(false);
  const [startPos, setStartPos] = useState({ x: 0, y: 0 });
  const [currentRect, setCurrentRect] = useState<React.CSSProperties | null>(null);

  const handleOverlayMouseDown = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    if (resizingColumn) return;
    if ((activeTool === 'cursor' && !e.shiftKey) || e.target !== overlayRef.current) return;
    e.preventDefault();
    setIsDrawing(true);
    setSelectedFieldTempId(null);
    const rect = overlayRef.current!.getBoundingClientRect();
    setStartPos({ x: e.clientX - rect.left, y: e.clientY - rect.top });
  }, [resizingColumn, activeTool, overlayRef, setSelectedFieldTempId]);

  const handleOverlayMouseMove = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    if (resizingColumn) {
      const field = fields.find(f => f.tempId === resizingColumn.tempId);
      if (!field || field.field_type !== 'cells') return;
      const rect = overlayRef.current!.getBoundingClientRect();
      const mouseX = e.clientX - rect.left;
      const fieldX = field.position!.x * canvasClientWidth;
      const fieldWidth = field.position!.width * canvasClientWidth;

      if (resizingColumn.index === -1) {
        const minCellWidth = 10;
        const maxColumns = Math.floor(fieldWidth / minCellWidth);

        const newHandlePos = Math.max(0, Math.min(fieldWidth, mouseX - fieldX));
        const newRatio = Math.max(0.001, Math.min(1, newHandlePos / fieldWidth));
        setCurrentHandlePosition(newRatio);

        const calculatedColumns = Math.round(1 / newRatio);
        const numColumns = Math.max(1, Math.min(maxColumns, calculatedColumns));

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
  }, [resizingColumn, fields, overlayRef, canvasClientWidth, isDrawing, startPos, setCurrentHandlePosition, updateField]);

  const handleOverlayMouseUp = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
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
    
    if (width < 20 || height < 5) return;

    if (partners.length === 0) {
      alert('Please add at least one partner before creating fields.');
      return;
    }

    const displayWidth = canvasClientWidth || rect.width || 600;
    const displayHeight = canvasClientHeight || rect.height || 800;

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
      position: { x, y, width: w, height: h, page: currentPage },
      ...(fieldType === 'radio' && { options: ['Option 1', 'Option 2'] }),
      ...(fieldType === 'multiple' && { options: ['Option 1', 'Option 2'] }),
      ...(fieldType === 'select' && { options: ['Option 1', 'Option 2'] }),
      ...(fieldType === 'cells' && { options: { columns: 3, widths: [1, 1, 1] } }),
      partner: currentPartner
    };
    setFields(prev => [...prev, newField]);
    setSelectedFieldTempId(newField.tempId);
    setActiveTool('cursor');
  }, [resizingColumn, isDrawing, activeTool, overlayRef, startPos, partners, canvasClientWidth, canvasClientHeight, lastFieldTool, fields, currentPage, currentPartner, setResizingColumn, setCurrentHandlePosition, setFields, setSelectedFieldTempId, setActiveTool]);

  const handleDragStop = useCallback((tempId: string, e: any, d: { x: number; y: number }) => {
    const field = fields.find(f => f.tempId === tempId);
    if (!field || !field.position) return;

    const displayWidth = canvasClientWidth || 600;
    const displayHeight = canvasClientHeight || 800;

    const x = Math.max(0, Math.min(1 - field.position.width, d.x / displayWidth));
    const y = Math.max(0, Math.min(1 - field.position.height, d.y / displayHeight));

    updateField(tempId, {
      position: {
        ...field.position,
        x,
        y
      }
    });
  }, [fields, canvasClientWidth, canvasClientHeight, updateField]);

  const handleResizeStop = useCallback((tempId: string, e: any, direction: string, ref: HTMLElement, delta: any, position: { x: number; y: number }) => {
    const field = fields.find(f => f.tempId === tempId);
    if (!field || !field.position) return;

    const displayWidth = canvasClientWidth || 600;
    const displayHeight = canvasClientHeight || 800;

    const x = Math.max(0, position.x / displayWidth);
    const y = Math.max(0, position.y / displayHeight);
    const width = Math.max(0.01, Math.min(1 - x, ref.offsetWidth / displayWidth));
    const height = Math.max(0.01, Math.min(1 - y, ref.offsetHeight / displayHeight));

    updateField(tempId, {
      position: {
        ...field.position,
        x,
        y,
        width,
        height
      }
    });
  }, [fields, canvasClientWidth, canvasClientHeight, updateField]);

  return {
    isDrawing,
    currentRect,
    handleOverlayMouseDown,
    handleOverlayMouseMove,
    handleOverlayMouseUp,
    handleDragStop,
    handleResizeStop,
  };
};
