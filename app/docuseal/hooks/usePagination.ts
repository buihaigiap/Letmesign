import { useState, useEffect } from 'react';


interface UsePaginationReturn<T> {
  currentPage: number;
  setCurrentPage: (page: number) => void;
  totalPages: number;
  currentItems: any;
  resetPage: () => void;
}

export const usePagination = <T,>({
  items,
  itemsPerPage
}:any): UsePaginationReturn<any> => {
  const [currentPage, setCurrentPage] = useState(1);

  const totalPages = Math.ceil(items.length / itemsPerPage);

  const indexOfLastItem = currentPage * itemsPerPage;
  const indexOfFirstItem = indexOfLastItem - itemsPerPage;
  const currentItems = items.slice(indexOfFirstItem, indexOfLastItem);

  const resetPage = () => setCurrentPage(1);

  // Reset to page 1 when items change
  useEffect(() => {
    setCurrentPage(1);
  }, [items]);

  return {
    currentPage,
    setCurrentPage,
    totalPages,
    currentItems,
    resetPage
  };
};