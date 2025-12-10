import React from 'react';
import { FieldType } from '../../types';
import { FieldTool } from './types';
import { MousePointer, Type, PenTool, Hash, User, Calendar, CheckSquare, Circle, List, ChevronDown, Table, ImageIcon, File } from 'lucide-react';

export const fieldTools: { name: string; type: FieldTool; iconComponent: (className: string) => React.ReactElement }[] = [
  { name: 'Cursor', type: 'cursor', iconComponent: (className) => <MousePointer className={className} /> },
  { name: 'Text', type: 'text', iconComponent: (className) => <Type className={className} /> },
  { name: 'Signature', type: 'signature', iconComponent: (className) => <PenTool className={className} /> },
  { name: 'Number', type: 'number', iconComponent: (className) => <Hash className={className} /> },
  { name: 'Initials', type: 'initials', iconComponent: (className) => <User className={className} /> },
  { name: 'Date', type: 'date', iconComponent: (className) => <Calendar className={className} /> },
  { name: 'Checkbox', type: 'checkbox', iconComponent: (className) => <CheckSquare className={className} /> },
  { name: 'Radio', type: 'radio', iconComponent: (className) => <Circle className={className} /> },
  { name: 'Multiple', type: 'multiple', iconComponent: (className) => <List className={className} /> },
  { name: 'Select', type: 'select' as FieldType, iconComponent: (className) => <ChevronDown className={className} /> },
  { name: 'Cells', type: 'cells', iconComponent: (className) => <Table className={className} /> },
  { name: 'Image', type: 'image', iconComponent: (className) => <ImageIcon className={className} /> },
  { name: 'File', type: 'file', iconComponent: (className) => <File className={className} /> },
];

export const getCurrentToolIcon = (fieldType: string, className: string = 'w-4 h-4'): React.ReactElement => {
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

