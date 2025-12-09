// Layout.tsx
import React, { useState } from 'react';
import { Box } from '@mui/material';
import { Link, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useAuth } from '../../contexts/AuthContext';
import UserMenu from './UserMenu';
import { AuthActions } from './AuthActions';

const Layout = ({ children }:any) => {
  const { t } = useTranslation();
  const { isAuthenticated, logout, user } = useAuth();
  const navigate = useNavigate();

  const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false);
  const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null);

  const handleLogout = () => {
    logout();
    navigate('/login');
    setIsMobileMenuOpen(false);
    setAnchorEl(null);
  };

  return (
    <div className="relative min-h-screen bg-[#0D071F] text-white">
      <header className="fixed top-0 w-full z-50 bg-slate-900/50 backdrop-blur border-b border-white/10">
        <nav className="max-w-7xl mx-auto px-4">
          <div className="flex h-16 items-center justify-between">
            <Link to="/">
              <img src="/logo.png" alt="Letmesign" width={180} />
            </Link>

            {/* Desktop */}
            <div className="hidden md:flex gap-4 items-center">
              <AuthActions
                isAuthenticated={isAuthenticated}
                user={user}
                t={t}
                navigate={navigate}
                onAvatarClick={
                  isAuthenticated && (
                    <UserMenu
                      anchorEl={anchorEl}
                      open={Boolean(anchorEl)}
                      onOpen={(e) => setAnchorEl(e.currentTarget)}
                      onClose={() => setAnchorEl(null)}
                      userName={user?.name}
                      t={t}
                      navigate={navigate}
                      onLogout={handleLogout}
                    />
                  )
                }
              />
            </div>

            {/* Mobile toggle */}
            <button
              className="md:hidden"
              onClick={() => setIsMobileMenuOpen(!isMobileMenuOpen)}
            >
              ☰
            </button>
          </div>
        </nav>

        {/* Mobile Menu */}
        {isMobileMenuOpen && (
          <div className="md:hidden border-t border-white/10">
            <div className="p-4 flex flex-col gap-3">
              {!isAuthenticated ? (
                <>
                  <Link to="/login">{t('auth.login')}</Link>
                  <Link to="/register">{t('auth.register')}</Link>
                </>
              ) : (
                <>
                  <Link to="/" onClick={() => setIsMobileMenuOpen(false)}>
                    Dashboard
                  </Link>
                  <Link to="/pricing">{t('navigation.pricing')}</Link>

                  <button onClick={handleLogout}>{t('auth.logout')}</button>
                </>
              )}
            </div>
          </div>
        )}
      </header>

      <Box sx={{ maxWidth: 1500, mx: 'auto', pt: 8, px: 3 }}>
        <main>{children}</main>
      </Box>
    </div>
  );
};

export default Layout;
