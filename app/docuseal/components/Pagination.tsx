import React from 'react';
import { Pagination as MuiPagination, Box } from '@mui/material';

interface PaginationProps {
  currentPage: number;
  totalPages: number;
  onPageChange: (page: number) => void;
  className?: string;
}

const Pagination: React.FC<PaginationProps> = ({
  currentPage,
  totalPages,
  onPageChange,
  className = ''
}) => {
  if (totalPages <= 1) return null;

  return (
    <Box
      sx={{
        display: 'flex',
        justifyContent: 'center',
        marginTop: '1rem',
        marginBottom: '1rem'
      }}
      className={className}
    >
      <MuiPagination
        count={totalPages}
        page={currentPage}
        onChange={(event, page) => onPageChange(page)}
        sx={{
          '& .MuiPaginationItem-root': {
            color: 'white',
          },
          '& .Mui-selected': {
            fontSize: '1.2rem',
            fontWeight: 'bold',
          },
          '& .MuiPaginationItem-root:hover': {
            backgroundColor: 'rgba(30, 41, 59, 0.6)',
          },
        }}
      />
    </Box>
  );
};

export default Pagination;