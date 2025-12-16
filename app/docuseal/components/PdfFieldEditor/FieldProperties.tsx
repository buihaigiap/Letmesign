import React, { useState } from 'react';
import { EditorField } from './types';
import { Plus, X, Trash2 } from 'lucide-react';
import upstashService from '../../ConfigApi/upstashService';
import toast from 'react-hot-toast';

interface FieldPropertiesProps {
  fields: EditorField[];
  currentPartner: string;
  selectedFieldTempId: string | null;
  setSelectedFieldTempId: (id: string | null) => void;
  updateField: (tempId: string, updates: Partial<EditorField>) => void;
  getCurrentToolIcon: (fieldType: string) => React.ReactElement;
  deleteField: (tempId: string) => void;
  token: string;
  templateId: number;
}

const FieldProperties: React.FC<FieldPropertiesProps> = ({
  fields,
  currentPartner,
  selectedFieldTempId,
  setSelectedFieldTempId,
  updateField,
  getCurrentToolIcon,
  deleteField,
  token,
  templateId
}) => {
  const [newOptionText, setNewOptionText] = useState('');
  const [hoveredFieldId, setHoveredFieldId] = useState<string | null>(null);

  const selectedField = fields.find(f => f.tempId === selectedFieldTempId);
  const hasOptions = selectedField && ['radio', 'multiple', 'select'].includes(selectedField.field_type);

  const handleAddOption = () => {
    if (!selectedField || !newOptionText.trim()) return;
    const currentOptions = selectedField.options && typeof selectedField.options === 'object' && !Array.isArray(selectedField.options) ? selectedField.options.options : selectedField.options;
    const updatedOptions = [...(Array.isArray(currentOptions) ? currentOptions : []), newOptionText.trim()];
    const optionsObject = Array.isArray(selectedField.options) ? { options: updatedOptions } : { ...selectedField.options, options: updatedOptions };
    updateField(selectedField.tempId, { options: optionsObject });
    setNewOptionText('');
    if (selectedField.id) {
      upstashService.updateField(templateId, selectedField.id, { options: optionsObject }).catch((error) => {
        console.error('Failed to save options:', error);
        toast.error('Failed to save options');
      });
    }
  };

  const handleRemoveOption = (index: number) => {
    if (!selectedField) return;
    const currentOptions = selectedField.options && typeof selectedField.options === 'object' && !Array.isArray(selectedField.options) ? selectedField.options.options : selectedField.options;
    const updatedOptions = (Array.isArray(currentOptions) ? currentOptions : []).filter((_, i) => i !== index);
    const optionsObject = Array.isArray(selectedField.options) ? { options: updatedOptions } : { ...selectedField.options, options: updatedOptions };
    updateField(selectedField.tempId, { options: optionsObject });
    if (selectedField.id) {
      upstashService.updateField(templateId, selectedField.id, { options: optionsObject }).catch((error) => {
        console.error('Failed to save options:', error);
        toast.error('Failed to save options');
      });
    }
  };

  const handleUpdateOption = (index: number, value: string) => {
    if (!selectedField) return;
    const currentOptions = selectedField.options && typeof selectedField.options === 'object' && !Array.isArray(selectedField.options) ? selectedField.options.options : selectedField.options;
    const updatedOptions = [...(Array.isArray(currentOptions) ? currentOptions : [])];
    updatedOptions[index] = value;
    const optionsObject = Array.isArray(selectedField.options) ? { options: updatedOptions } : { ...selectedField.options, options: updatedOptions };
    updateField(selectedField.tempId, { options: optionsObject });
    if (selectedField.id) {
      upstashService.updateField(templateId, selectedField.id, { options: optionsObject }).catch((error) => {
        console.error('Failed to save options:', error);
        toast.error('Failed to save options');
      });
    }
  };

  return (
    <div>
      <div className="space-y-4">
        <div>
          <div className="max-h-40 overflow-y-auto space-y-2">
            <div>
              <div >
                {fields.filter(f => f.partner === currentPartner).map(field => (
                  <div
                    key={field.tempId}
                    className={`p-2 rounded-md text-sm relative group ${
                      selectedFieldTempId === field.tempId
                        ? 'bg-indigo-600 '
                        : 'hover:bg-gray-600 hover:text-white '
                    }`}
                    onMouseEnter={() => setHoveredFieldId(field.tempId)}
                    onMouseLeave={() => setHoveredFieldId(null)}
                  >
                    
                     {selectedFieldTempId === field.tempId ? (
                      <div className="flex items-center text-white gap-2">
                        {getCurrentToolIcon(field.field_type)}
                        <input
                          type="text"
                          value={field.name}
                          onChange={e => updateField(field.tempId, { name: e.target.value })}
                          className="bg-transparent border-none outline-none text-white font-medium flex-1"
                        />
                        {/* Delete icon - always visible when selected */}
                        <button
                          onClick={(e) => {
                            e.stopPropagation();
                            deleteField(field.tempId);
                            setSelectedFieldTempId(null);
                          }}
                          className="p-1 text-red-400 hover:text-red-300 hover:bg-red-900/20 rounded transition-colors"
                          title="Delete field"
                        >
                          <Trash2 className="w-4 h-4" />
                        </button>
                      </div>
                    ) : (
                      <div className="flex items-center gap-2 text-white">
                        <div
                          onClick={() => setSelectedFieldTempId(field.tempId)}
                          className="cursor-pointer font-medium flex items-center gap-2 flex-1"
                        >
                          {getCurrentToolIcon(field.field_type)}
                          {field.name}
                        </div>
                        {/* Delete icon - visible on hover */}
                        {hoveredFieldId === field.tempId && (
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              deleteField(field.tempId);
                            }}
                            className="p-1 text-red-400 hover:text-red-300 hover:bg-red-900/20 rounded transition-colors"
                            title="Delete field"
                          >
                            <Trash2 className="w-4 h-4" />
                          </button>
                        )}
                      </div>
                    )}
                  </div>
                ))}
              </div>
            </div>
            {fields.filter(f => !f.partner).length > 0 && (
              <div>
                <h5 className="text-sm font-medium text-gray-400 mb-1">Unassigned</h5>
                <div className="space-y-1">
                  {fields.filter(f => !f.partner).map(field => (
                    <div
                      key={field.tempId}
                      className={`p-2 rounded-md text-sm relative group ${
                        selectedFieldTempId === field.tempId
                          ? 'bg-indigo-600 text-white'
                          : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
                      }`}
                      onMouseEnter={() => setHoveredFieldId(field.tempId)}
                      onMouseLeave={() => setHoveredFieldId(null)}
                    >
                      {selectedFieldTempId === field.tempId ? (
                        <div className="flex items-center gap-2">
                          {getCurrentToolIcon(field.field_type)}
                          <input
                            type="text"
                            value={field.name}
                            onChange={e => updateField(field.tempId, { name: e.target.value })}
                            className="bg-transparent border-none outline-none text-white font-medium flex-1"
                          />
                          {/* Delete icon - always visible when selected */}
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              deleteField(field.tempId);
                              setSelectedFieldTempId(null);
                            }}
                            className=" text-red-400 hover:text-red-300 hover:bg-red-900/20 rounded transition-colors"
                            title="Delete field"
                          >
                            <Trash2 className="w-4 h-4" />
                          </button>
                        </div>
                      ) : (
                        <div className="flex items-center gap-2">
                          <div
                            onClick={() => setSelectedFieldTempId(field.tempId)}
                            className="cursor-pointer font-medium flex items-center flex-1"
                          >
                            {getCurrentToolIcon(field.field_type)}
                            <span className="ml-2">{field.name}</span>
                          </div>
                          {/* Delete icon - visible on hover */}
                          {hoveredFieldId === field.tempId && (
                            <button
                              onClick={(e) => {
                                e.stopPropagation();
                                deleteField(field.tempId);
                              }}
                              className="p-1 text-red-400 hover:text-red-300 hover:bg-red-900/20 rounded transition-colors"
                              title="Delete field"
                            >
                              <Trash2 className="w-4 h-4" />
                            </button>
                          )}
                        </div>
                      )}
                      <div className="text-xs opacity-75 capitalize">{field.field_type}</div>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        </div>

        {/* Options Editor for radio, multiple, select */}
        {hasOptions && selectedField && (
          <div className="mt-4 border-t border-gray-600 pt-4">
            <h4 className="text-sm font-semibold text-white mb-2">Options</h4>
            <div className="space-y-2">
              {(() => {
                const currentOptions = selectedField.options && typeof selectedField.options === 'object' && !Array.isArray(selectedField.options) ? selectedField.options.options : selectedField.options;
                return (Array.isArray(currentOptions) ? currentOptions : []).map((option, index) => (
                  <div key={index} className="flex items-center gap-2">
                    <input
                      type="text"
                      value={option}
                      onChange={(e) => handleUpdateOption(index, e.target.value)}
                      className="flex-1 px-2 py-1 bg-gray-700 text-white rounded text-sm border border-gray-600 focus:border-indigo-500 outline-none"
                      placeholder={`Option ${index + 1}`}
                    />
                    <button
                      onClick={() => handleRemoveOption(index)}
                      className="p-1 text-red-400 hover:text-red-300 hover:bg-red-900/20 rounded"
                      title="Remove option"
                    >
                      <X className="w-4 h-4" />
                    </button>
                  </div>
                ));
              })()}
              
              <div className="flex items-center gap-2 mt-2">
                <input
                  type="text"
                  value={newOptionText}
                  onChange={(e) => setNewOptionText(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') {
                      e.preventDefault();
                      handleAddOption();
                    }
                  }}
                  className="flex-1 px-2 py-1 bg-gray-700 text-white rounded text-sm border border-gray-600 focus:border-indigo-500 outline-none"
                  placeholder="Add new option..."
                />
                <button
                  onClick={handleAddOption}
                  className="p-1 text-green-400 hover:text-green-300 hover:bg-green-900/20 rounded"
                  title="Add option"
                >
                  <Plus className="w-4 h-4" />
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default FieldProperties;