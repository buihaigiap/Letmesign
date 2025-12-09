// components/UserMenu.tsx
import { Avatar, Menu, MenuItem } from '@mui/material';
import { CircleUser, LogOut, Sparkle } from 'lucide-react';

interface UserMenuProps {
  anchorEl: HTMLElement | null;
  open: boolean;
  onOpen: (e: React.MouseEvent<HTMLElement>) => void;
  onClose: () => void;
  onLogout: () => void;
  navigate: (path: string) => void;
  userName?: string;
  t: any;
}

const UserMenu = ({
  anchorEl,
  open,
  onOpen,
  onClose,
  onLogout,
  navigate,
  userName,
  t,
}: UserMenuProps) => {
  const menuItems = [
    {
      label: 'Profile',
      icon: <CircleUser className="w-4 h-4 mr-2" />,
      onClick: () => navigate('/settings'),
    },
    {
      label: 'Verify PDF',
      icon: <Sparkle className="w-4 h-4 mr-2" />,
      onClick: () => navigate('/settings/pdf-signature'),
    },
    {
      label: t('auth.logout'),
      icon: <LogOut className="w-4 h-4 mr-2" />,
      onClick: onLogout,
    },
  ];

  return (
    <>
      <Avatar
        sx={{ cursor: 'pointer', bgcolor: 'purple.500' }}
        onClick={onOpen}
      >
        {userName?.charAt(0).toUpperCase()}
      </Avatar>

      <Menu
        anchorEl={anchorEl}
        open={open}
        onClose={onClose}
        anchorOrigin={{ vertical: 'bottom', horizontal: 'center' }}
        transformOrigin={{ vertical: 'top', horizontal: 'center' }}
        sx={{
          '& .MuiPaper-root': {
            backgroundColor: '#374151',
            color: 'white',
            minWidth: 200,
          },
        }}
      >
        {menuItems.map((item, idx) => (
          <MenuItem
            key={idx}
            onClick={() => {
              item.onClick();
              onClose();
            }}
          >
            {item.icon}
            {item.label}
          </MenuItem>
        ))}
      </Menu>
    </>
  );
};

export default UserMenu;
