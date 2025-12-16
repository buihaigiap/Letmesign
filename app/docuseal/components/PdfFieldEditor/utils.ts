import { EditorField } from './types';

export const measureTextWidth = (text: string, fontSize: string = '12px', fontFamily: string = 'ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, "Noto Sans", sans-serif, "Apple Color Emoji", "Segoe UI Emoji", "Segoe UI Symbol", "Noto Color Emoji"') => {
  const canvas = document.createElement('canvas');
  const context = canvas.getContext('2d');
  if (!context) return 0;
  context.font = `${fontSize} ${fontFamily}`;
  return context.measureText(text).width;
};

export const updateInputWidth = (tempId: string, text: string, setInputWidths: React.Dispatch<React.SetStateAction<Record<string, number>>>) => {
  const width = measureTextWidth(text, '12px') + 16; // Add some padding
  setInputWidths(prev => ({ ...prev, [tempId]: Math.max(width, 24) })); // Minimum 24px
};

export const getFieldClass = (partner: string | undefined, isSelected: boolean, partnerColorClasses: string[]) => {
  const base = 'absolute hover:bg-opacity-60 transition-all duration-150 , text-black';
  const selectedClass = 'border ';

  let colorClass = 'bg-white bg-opacity-60 border-gray-400'; // Default transparent white
  if (partner) {
    const partners = ['First Party', 'Second Party', 'Third Party', 'Fourth Party', 'Fifth Party']; // This should be passed as param
    const partnerIndex = partners.indexOf(partner);
    if (partnerIndex >= 0) {
      colorClass = partnerColorClasses[partnerIndex % partnerColorClasses.length];
    }
  }

  return `${base} ${colorClass} ${isSelected ? selectedClass : selectedClass}`;
};

export const checkOverlap = (tempId: string, newPos: any, fields: EditorField[]) => {
  return fields.some(f => f.tempId !== tempId &&
    newPos.x < f.position!.x + f.position!.width &&
    newPos.x + newPos.width > f.position!.x &&
    newPos.y < f.position!.y + f.position!.height &&
    newPos.y + newPos.height > f.position!.y
  );
};


 export  const formatDate = (dateString?: string, format?: string) => {
    const date = new Date(dateString);
    const day = date.getDate();
    const month = date.getMonth() + 1;
    const year = date.getFullYear();
    
    const monthNames = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
    const monthNamesFull = ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December'];
    
    const pad = (n: number) => n.toString().padStart(2, '0');
    
    switch (format) {
      case 'MM/DD/YYYY':
        return `${pad(month)}/${pad(day)}/${year}`;
      case 'DD/MM/YYYY':
        return `${pad(day)}/${pad(month)}/${year}`;
      case 'YYYY-MM-DD':
        return `${year}-${pad(month)}-${pad(day)}`;
      case 'DD-MM-YYYY':
        return `${pad(day)}-${pad(month)}-${year}`;
      case 'DD.MM.YYYY':
        return `${pad(day)}.${pad(month)}.${year}`;
      case 'MMM D, YYYY':
        return `${monthNames[month - 1]} ${day}, ${year}`;
      case 'MMMM D, YYYY':
        return `${monthNamesFull[month - 1]} ${day}, ${year}`;
      case 'D MMM YYYY':
        return `${day} ${monthNames[month - 1]} ${year}`;
      case 'D MMMM YYYY':
        return `${day} ${monthNamesFull[month - 1]} ${year}`;
      default:
        return dateString;
    }
  };
