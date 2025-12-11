import { useRef } from 'react';
import { NewTemplateField } from '../../../types';
import upstashService from '../../../ConfigApi/upstashService';
import { Field } from '../types';
import toast from 'react-hot-toast';

export const useFieldManagement = (
  fields: Field[],
  setFields: React.Dispatch<React.SetStateAction<Field[]>>,
  originalFields: Record<number, any>,
  setOriginalFields: React.Dispatch<React.SetStateAction<Record<number, any>>>,
  deletedIds: Set<number>,
  setDeletedIds: React.Dispatch<React.SetStateAction<Set<number>>>,
  pageWidth: number,
  pageHeight: number,
  partners: string[],
  token: string | null,
  templateId: number
) => {
  const initialFieldsLengthRef = useRef<number>(0);

  const updateField = (tempId: string, updates: Partial<Field>) => {
    setFields(prev => prev.map(f => f.tempId === tempId ? { ...f, ...updates } : f));
  };

  const deleteField = (tempId: string) => {
    console.log('deleteField called with tempId:', tempId);
    const field = fields.find(f => f.tempId === tempId);
    console.log('Found field to delete:', field);
    if (field?.id) {
      setDeletedIds(prev => {
        const newSet = new Set([...prev, field.id]);
        console.log('Added to deletedIds:', field.id);
        return newSet;
      });
    }
    setFields(prev => {
      const newFields = prev.filter(f => f.tempId !== tempId);
      console.log('Fields after deletion:', newFields.length, 'fields remaining');
      return newFields;
    });
  };

  const duplicateField = (tempId: string) => {
    const field = fields.find(f => f.tempId === tempId);
    if (!field) return;

    const newTempId = `field-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
    const baseName = field.name.replace(/_\d+$/, ''); // Remove trailing number if exists
    const existingNames = fields.map(f => f.name);
    let newName = baseName;
    let counter = 1;
    while (existingNames.includes(newName)) {
      newName = `${baseName}_${counter}`;
      counter++;
    }

    const duplicatedField = {
      ...field,
      tempId: newTempId,
      name: newName,
      position: field.position ? {
        ...field.position,
        x: Math.min(0.9, field.position.x + 0.05), // Offset slightly
        y: Math.min(0.9, field.position.y + 0.05),
      } : field.position,
    };

    setFields(prev => [...prev, duplicatedField]);
  };

  const handleSaveClick = async () => {
    if (!token) return;

    // Use pageWidth/Height if available, otherwise use default dimensions
    const effectivePageWidth = pageWidth || 600;
    const effectivePageHeight = pageHeight || 800;
    
    console.log('Saving with dimensions:', { pageWidth, pageHeight, effectivePageWidth, effectivePageHeight });

    // Auto-assign partner to fields without partner (use first partner)
    let workingFields = fields;
    const fieldsWithoutPartner = workingFields.filter(f => !f.partner);
    if (fieldsWithoutPartner.length > 0) {
      console.warn('⚠️ Found', fieldsWithoutPartner.length, 'fields without partner, auto-assigning to first partner:', partners[0]);

      if (partners.length === 0) {
        toast.error('Cannot save: No partners defined. Please add at least one partner first.');
        return;
      }

      // Auto-assign first partner to fields without partner
      workingFields = workingFields.map(f =>
        !f.partner ? { ...f, partner: partners[0] } : f
      );

      // Update state
      setFields(workingFields);
    }

    // Only process fields that are NOT in deletedIds
    const activeFields = workingFields.filter(f => !f.id || !deletedIds.has(f.id));

        // Helper function to convert position from decimal (0-1) to pixels for API
    const convertPositionToPixels = (field: any): NewTemplateField => {
      const { id, tempId, ...rest } = field;
      
      // Ensure position values are within bounds (0-1 decimal)
      const clampedPosition = field.position ? {
        x: Math.max(0, Math.min(1, field.position.x)),
        y: Math.max(0, Math.min(1, field.position.y)),
        width: Math.max(0.01, Math.min(1, field.position.width)),
        height: Math.max(0.01, Math.min(1, field.position.height)),
        page: field.position.page
      } : null;

      if (clampedPosition && field.position) {
        const changed = 
          clampedPosition.x !== field.position.x ||
          clampedPosition.y !== field.position.y ||
          clampedPosition.width !== field.position.width ||
          clampedPosition.height !== field.position.height;
        
        if (changed) {
          console.warn('⚠️ Field position out of bounds, clamped:', field.name, field.position, '->', clampedPosition);
        }
      }

      return {
        ...rest,
        position: clampedPosition ? {
          x: clampedPosition.x * effectivePageWidth,
          y: clampedPosition.y * effectivePageHeight,
          width: clampedPosition.width * effectivePageWidth,
          height: clampedPosition.height * effectivePageHeight,
          page: clampedPosition.page,
          ...(field.options?.defaultValue !== undefined && { default_value: field.options.defaultValue })
        } : field.position
      } as NewTemplateField;
    };

    const toCreate = activeFields.filter(f => !f.id).map(convertPositionToPixels);

    // Helper function to check if field has actually changed
    const hasFieldChanged = (current: any, original: any): boolean => {
      if (!original) return false;

      // Compare each property individually
      if (current.name !== original.name) return true;
      if (current.required !== original.required) return true;
      if (current.partner !== original.partner) return true;

      // Compare position
      if (current.position && original.position) {
        const posChanged =
          Math.abs(current.position.x - original.position.x) > 0.01 ||
          Math.abs(current.position.y - original.position.y) > 0.01 ||
          Math.abs(current.position.width - original.position.width) > 0.01 ||
          Math.abs(current.position.height - original.position.height) > 0.01 ||
          current.position.page !== original.position.page;
        if (posChanged) return true;
      } else if (current.position !== original.position) {
        return true;
      }

      // Compare options (deep comparison)
      if (JSON.stringify(current.options) !== JSON.stringify(original.options)) {
        return true;
      }

      return false;
    };
    const toUpdate = activeFields.filter(f => f.id && originalFields[f.id] && hasFieldChanged(f, originalFields[f.id]));
    const toDelete = Array.from(deletedIds);
    if (toUpdate.length > 0) {
      console.log('  - Fields:', toUpdate.map(f => ({ name: f.name, id: f.id })));
      // Log what changed for each field
      toUpdate.forEach(f => {
        const orig = originalFields[f.id!];
        const changes: string[] = [];
        if (f.name !== orig.name) changes.push(`name: "${orig.name}" → "${f.name}"`);
        if (f.required !== orig.required) changes.push(`required: ${orig.required} → ${f.required}`);
        if (f.partner !== orig.partner) changes.push(`partner: "${orig.partner}" → "${f.partner}"`);
        if (JSON.stringify(f.options) !== JSON.stringify(orig.options)) changes.push('options changed');
        if (f.position && orig.position) {
          const posChanges = [];
          if (Math.abs(f.position.x - orig.position.x) > 0.01) posChanges.push('x');
          if (Math.abs(f.position.y - orig.position.y) > 0.01) posChanges.push('y');
          if (Math.abs(f.position.width - orig.position.width) > 0.01) posChanges.push('width');
          if (Math.abs(f.position.height - orig.position.height) > 0.01) posChanges.push('height');
          if (f.position.page !== orig.position.page) posChanges.push('page');
          if (f.position.default_value !== orig.position.default_value) posChanges.push('default_value');
          if (posChanges.length > 0) changes.push(`position: ${posChanges.join(', ')}`);
        }
        console.log(`    ${f.name} (ID: ${f.id}): ${changes.join(', ')}`);
      });
    }
    if (toDelete.length > 0) console.log('  - IDs:', toDelete);

    try {
      // Create new fields
      let createdFields: any[] = [];
      if (toCreate.length > 0) {
        const createPromises = toCreate.map(field =>
          upstashService.createField(templateId, field)
        );
        createdFields = await Promise.all(createPromises);
      }

      // Update existing fields
      if (toUpdate.length > 0) {
        console.log('Updating', toUpdate.length, 'fields...');
        const updatePromises = toUpdate.map(field => {
          // Convert position from decimal (0-1) to pixels, same as create logic
          const positionInPixels = field.position ? {
            x: field.position.x * effectivePageWidth,
            y: field.position.y * effectivePageHeight,
            width: field.position.width * effectivePageWidth,
            height: field.position.height * effectivePageHeight,
            page: field.position.page,
            ...(field.position.default_value !== undefined && { default_value: field.position.default_value })
          } : field.position;

          return upstashService.updateField(templateId, field.id, {
            name: field.name,
            required: field.required,
            position: positionInPixels,
            options: field.options,
            partner: field.partner
          });
        });
        await Promise.all(updatePromises);
        console.log('✓ Updated', toUpdate.length, 'fields');
      }

      // Delete fields
      if (toDelete.length > 0) {
        console.log('Deleting', toDelete.length, 'fields...');
        const deletePromises = toDelete.map(id => {
          console.log('  - Deleting field ID:', id);
          return upstashService.deleteField(templateId, id);
        });
        await Promise.all(deletePromises);
        console.log('✓ Deleted', toDelete.length, 'fields');
      }

      // Update local state - remove deleted fields and assign IDs to newly created fields
      const sortedToCreate = toCreate.sort((a, b) => a.display_order - b.display_order);
      const updatedFields = workingFields
        .filter(f => !f.id || !deletedIds.has(f.id)) // Remove deleted fields
        .map(f => {
          if (!f.id) {
            const index = sortedToCreate.findIndex(c => c.name === f.name && c.field_type === f.field_type);
            if (index !== -1 && createdFields[index]) {
              const created = createdFields[index];
              console.log('Assigning id to new field:', f.name, 'id:', created.id, 'index:', index);
              return { ...f, id: created.id, tempId: `field-${created.id}` };
            }
          }
          return f;
        });

      setFields(updatedFields);
      setOriginalFields(Object.fromEntries(updatedFields.filter(f => f.id).map(f => [f.id!, f as any])));
      setDeletedIds(new Set());

      // Update refs to prevent re-initialization after save
      initialFieldsLengthRef.current = updatedFields.length;

      return { success: true };
    } catch (err) {
      console.error('Error saving changessssssssssss:', err);
      toast.error(err?.response?.data?.error );
      return { success: false };
    }
  };

  return {
    updateField,
    deleteField,
    duplicateField,
    handleSaveClick,
    initialFieldsLengthRef
  };
};