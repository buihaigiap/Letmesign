import React, { useState } from 'react';
import { Link } from 'react-router-dom';
import { motion } from 'framer-motion';
interface Folder {
  id: number;
  name: string;
  parent_folder_id?: number;
  children?: Folder[];
}
interface FoldersListProps {
  folders: Folder[];
  title?: string;
}
const FoldersList: React.FC<FoldersListProps> = ({ folders, title = "Folders" }) => {
  const [currentPage, setCurrentPage] = useState(1);
  const itemsPerPage = 10;
  const totalPages = Math.ceil(folders.length / itemsPerPage);
  const indexOfLastItem = currentPage * itemsPerPage;
  const indexOfFirstItem = indexOfLastItem - itemsPerPage;
  const currentFolders = folders.slice(indexOfFirstItem, indexOfLastItem);

  if (folders.length === 0) return null;

  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.95, y: 20 }}
      animate={{ opacity: 1, scale: 1, y: 0 }}
      transition={{ delay: 0.1, duration: 0.6, ease: "easeOut" }}
      style={{ marginBottom: '2rem' }}
    >
      <h2 style={{ color: 'white', marginBottom: '1rem' }}>{title}</h2>
      <div style={{
        display: 'grid',
        gridTemplateColumns: 'repeat(auto-fill, minmax(200px, 1fr))',
        gap: '1rem',
      }}>
        {currentFolders.map((folder) => (
          <Link
            key={folder.id}
            to={`/folders/${folder.id}`}
            style={{
              backgroundColor: 'rgba(30, 41, 59, 0.8)',
              borderRadius: '8px',
              textDecoration: 'none',
              color: 'white',
              transition: 'all 0.3s ease',
              border: '1px solid rgba(255, 255, 255, 0.1)',
                padding: '1rem',

            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.backgroundColor = 'rgba(30, 41, 59, 1)';
              e.currentTarget.style.transform = 'translateY(-2px)';
              e.currentTarget.style.boxShadow = '0 4px 12px rgba(0, 0, 0, 0.3)';
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.backgroundColor = 'rgba(30, 41, 59, 0.8)';
              e.currentTarget.style.transform = 'translateY(0)';
              e.currentTarget.style.boxShadow = 'none';
            }}
          >
            <div style={{
              fontSize: '1rem',
              fontWeight: '500'
            }}>
              📁   {folder.name}
            </div>
          </Link>
        ))}
      </div>
      {totalPages > 1 && (
        <div style={{
          display: 'flex',
          justifyContent: 'center',
          alignItems: 'center',
          marginTop: '1rem',
          gap: '1rem'
        }}>
          <button
            onClick={() => setCurrentPage(prev => Math.max(prev - 1, 1))}
            disabled={currentPage === 1}
            style={{
              backgroundColor: currentPage === 1 ? 'rgba(100, 100, 100, 0.5)' : 'rgba(30, 41, 59, 0.8)',
              color: 'white',
              border: '1px solid rgba(255, 255, 255, 0.1)',
              borderRadius: '4px',
              padding: '0.5rem 1rem',
              cursor: currentPage === 1 ? 'not-allowed' : 'pointer',
              transition: 'all 0.3s ease'
            }}
          >
            Previous
          </button>
          <span style={{ color: 'white' }}>
            Page {currentPage} of {totalPages}
          </span>
          <button
            onClick={() => setCurrentPage(prev => Math.min(prev + 1, totalPages))}
            disabled={currentPage === totalPages}
            style={{
              backgroundColor: currentPage === totalPages ? 'rgba(100, 100, 100, 0.5)' : 'rgba(30, 41, 59, 0.8)',
              color: 'white',
              border: '1px solid rgba(255, 255, 255, 0.1)',
              borderRadius: '4px',
              padding: '0.5rem 1rem',
              cursor: currentPage === totalPages ? 'not-allowed' : 'pointer',
              transition: 'all 0.3s ease'
            }}
          >
            Next
          </button>
        </div>
      )}
    </motion.div>
  );
};

export default FoldersList;