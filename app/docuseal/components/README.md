# Shared Components

## Pagination Component

A reusable pagination component with consistent styling.

### Usage

```tsx
import Pagination from '../components/Pagination';

const MyComponent = () => {
  const [currentPage, setCurrentPage] = useState(1);
  const totalPages = 10;

  return (
    <Pagination
      currentPage={currentPage}
      totalPages={totalPages}
      onPageChange={setCurrentPage}
    />
  );
};
```

### Props

- `currentPage`: Current active page number
- `totalPages`: Total number of pages
- `onPageChange`: Callback function when page changes
- `className`: Optional CSS class name

## usePagination Hook

A custom hook for handling pagination logic.

### Usage

```tsx
import { usePagination } from '../hooks/usePagination';

const MyComponent = ({ items }) => {
  const itemsPerPage = 12;
  const { currentPage, setCurrentPage, totalPages, currentItems } = usePagination({
    items,
    itemsPerPage
  });

  return (
    <div>
      {currentItems.map(item => <div key={item.id}>{item.name}</div>)}
      <Pagination
        currentPage={currentPage}
        totalPages={totalPages}
        onPageChange={setCurrentPage}
      />
    </div>
  );
};
```

### Return Values

- `currentPage`: Current active page
- `setCurrentPage`: Function to change current page
- `totalPages`: Total number of pages
- `currentItems`: Array of items for current page
- `resetPage`: Function to reset to page 1