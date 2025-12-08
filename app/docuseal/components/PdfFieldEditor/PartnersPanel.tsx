import React, { useState, useEffect } from 'react';
import { UserRoundPlus  ,  X } from 'lucide-react';
import { TextField, InputAdornment, IconButton } from '@mui/material';

interface PartnersPanelProps {
  partners: string[];
  setPartners: React.Dispatch<React.SetStateAction<string[]>>;
  currentPartner: string;
  setCurrentPartner: (partner: string) => void;
  fields: any[];
  setFields: React.Dispatch<React.SetStateAction<any[]>>;
  getPartnerColorClass: (partner: string) => string;
  checkRole?:any;
  hasAccess?:any;

}

const PartnersPanel: React.FC<PartnersPanelProps> = ({
  partners,
  setPartners,
  currentPartner,
  setCurrentPartner,
  fields,
  setFields , getPartnerColorClass,
  checkRole , hasAccess
}) => {
  const [showList, setShowList] = useState(false);
  const [hoveredIndex, setHoveredIndex] = useState<number | null>(null);

  useEffect(() => {
    const cleaned = partners.filter(p => p);
    if (cleaned.length !== partners.length) {
      setPartners(cleaned);
    }
    if (!currentPartner && cleaned.length > 0) {
      setCurrentPartner(cleaned[0]);
    }
  }, []);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (showList && !(event.target as Element).closest('.partners-dropdown')) {
        setShowList(false);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [showList]);

  return (
    <div className="relative overflow-visible partners-dropdown">
      <style>
        {`
          .scrollbar-hide {
            -ms-overflow-style: none;  /* IE and Edge */
            scrollbar-width: none;  /* Firefox */
          }
          .scrollbar-hide::-webkit-scrollbar {
            display: none;  /* Chrome, Safari and Opera */
          }
        `}
      </style>
      <div>
        <h3 className="text-xl font-semibold mb-4 text-white">Current Partner</h3>
        <TextField
          value={currentPartner || ''}
          onChange={e => {
            const cleanedPartners = partners.filter(p => p);
            const index = cleanedPartners.findIndex(p => p === currentPartner);
            if (index !== -1) {
              const oldName = cleanedPartners[index];
              const newName = e.target.value;
              // Update fields first
              setFields(fields => fields.map(f => f.partner === oldName ? { ...f, partner: newName } : f));
              // Then update partners
              const newPartners = [...cleanedPartners];
              newPartners[index] = e.target.value;
              setPartners(newPartners);
            }
            setCurrentPartner(e.target.value);
          }}
          variant="outlined"
          size="small"
          fullWidth
          InputProps={{
            startAdornment: currentPartner ? (
              <InputAdornment position="start">
                <div className={`w-3 h-3 rounded-full ${getPartnerColorClass(currentPartner).split(' ')[0]}`} />
              </InputAdornment>
            ) : null,
            endAdornment: (
              <InputAdornment position="end">
                <IconButton
                  size="small"
                  onClick={() => setShowList(!showList)}
                  sx={{ color: 'gray' }}
                >
                  {showList ? '▲' : '▼'}
                </IconButton>
              </InputAdornment>
            ),
          }}
          sx={{
            '& .MuiOutlinedInput-root': {
              backgroundColor: 'transparent',
              color: 'white',
              '& fieldset': {
                borderColor: 'gray',
              },
              '&:hover fieldset': {
                borderColor: 'lightgray',
              },
              '&.Mui-focused fieldset': {
                borderColor: 'white',
              },
            },
            '& .MuiInputLabel-root': {
              color: 'white',
            },
          }}
        />
        {showList && (
          <div className="absolute top-full left-0 w-full bg-white rounded mt-1 z-50 max-h-60 overflow-y-auto scrollbar-hide">
            <div className="space-y-2 p-2">
              {partners.filter(p => p).map((partner, index) => (
                <div
                  key={partner}
                  className="flex items-center space-x-2 relative cursor-pointer  p-1 rounded"
                  onMouseEnter={() => setHoveredIndex(index)}
                  onMouseLeave={() => setHoveredIndex(null)}
                  onClick={() => setCurrentPartner(partner)}
                >
                  <span className="flex-1  text-sm flex items-center">
                    <div className={`w-3 h-3 rounded-full mr-2 ${getPartnerColorClass(partner).split(' ')[0]}`} />
                    {partner}
                  </span>
                  {hoveredIndex === index && (
                    <button
                      onClick={(e) => { 
                        e.stopPropagation(); 
                        const newPartners = partners.filter((p, i) => p && i !== index);
                        setPartners(newPartners);
                        
                        // Also remove all fields associated with this partner
                        setFields(prevFields => {
                          const filteredFields = prevFields.filter(f => {
                            const fieldPartner = (f.partner || '').trim();
                            const targetPartner = (partner || '').trim();
                            const shouldKeep = fieldPartner !== targetPartner;
                            console.log(`Field "${f.name}" partner "${fieldPartner}" vs target "${targetPartner}" - keep: ${shouldKeep}`);
                            return shouldKeep;
                          });
                          console.log('Fields after deletion:', filteredFields.length, 'fields remaining');
                          return filteredFields;
                        });
                        
                        if (currentPartner === partner && newPartners.length > 0) {
                          console.log('Updating currentPartner from', currentPartner, 'to', newPartners[0]);
                          setCurrentPartner(newPartners[0]);
                        } else if (currentPartner === partner && newPartners.length === 0) {
                          console.log('No partners left, currentPartner will be empty');
                          setCurrentPartner('');
                        }
                      }}
                    >
                          <X size={20}/>
                    </button>
                  )}
                </div>
              ))}
              {checkRole && !hasAccess && (
                  <div
                    onClick={() => {
                    const ordinals = ['First', 'Second', 'Third', 'Fourth', 'Fifth', 'Sixth', 'Seventh', 'Eighth', 'Ninth', 'Tenth'];
                    const nextOrdinal = ordinals[partners.length] || `Partner ${partners.length + 1}`;
                    setPartners([...partners.filter(p => p), `${nextOrdinal} Party`]);
                }}
                  className="w-full flex items-center gap-2 text-sm rounded cursor-pointer "
              >
                   <UserRoundPlus /> Add Partner
              </div>
              )}
            
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default PartnersPanel;