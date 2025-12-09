import React, { useState, useEffect } from 'react';
import { Avatar, Box, Menu, MenuItem } from '@mui/material';
import { Link, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useAuth } from '../contexts/AuthContext';
import CreateTemplateButton from './CreateTemplateButton';


const Layout = ({ children }: { children: React.ReactNode }) => {
  const { t } = useTranslation();
  const { isAuthenticated, isLoading, logout, user } = useAuth();
  const navigate = useNavigate();
  const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false);
  const [anchorEl, setAnchorEl] = useState(null);

  const handleMenuOpen = (event) => setAnchorEl(event.currentTarget);
  const handleMenuClose = () => setAnchorEl(null);

  const handleLogout = () => {
    logout();
    navigate('/login');
    setIsMobileMenuOpen(false);
  };

  // Handle click outside to close menu
  useEffect(() => {
    const handleClickOutside = (event) => {
      if (anchorEl && !anchorEl.contains(event.target)) {
        handleMenuClose();
      }
    };

    if (anchorEl) {
      document.addEventListener('mousedown', handleClickOutside);
    }

    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [anchorEl]);

  const navLinkClasses = "text-slate-300 hover:text-white transition-colors duration-300 text-sm font-medium";

  return (
    <div className="relative min-h-screen w-full bg-[#0D071F] font-sans text-white overflow-hidden">
        <div className="absolute top-[-10rem] left-[-20rem] w-[40rem] h-[40rem] bg-purple-500/20 rounded-full blur-3xl animate-pulse"></div>
        <div className="absolute bottom-[-5rem] right-[-20rem] w-[40rem] h-[40rem] bg-blue-500/10 rounded-full blur-3xl animate-pulse" style={{ animationDelay: '3s'}}></div>
      
        <header className="fixed top-0 left-0 right-0 z-50 bg-slate-900/50 backdrop-blur-lg border-b border-white/10">
            <nav className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
            <div className="flex items-center justify-between h-16">
                <Link to="/" className="text-xl font-bold text-white">
                    <img src='/logo.png' alt='Letmesign' width={180} />
                </Link>

                {/* Desktop Nav */}
                <div className="hidden md:flex items-center space-x-6">
                {isAuthenticated ? (
                    <>
                        {user?.free_usage_count !== undefined && (
                            <div className="group relative flex items-center" title={`Used ${user.free_usage_count}/10 free emails`}>
                            <div className="flex items-center space-x-2 bg-white/5 border border-white/10 px-3 py-1.5 rounded-lg">
                                <svg xmlns="http://www.w3.org/2000/svg" className="h-5 w-5 text-purple-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
                                <path strokeLinecap="round" strokeLinejoin="round" d="M3 8l7.89 5.26a2 2 0 002.22 0L21 8M5 19h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
                                </svg>
                                <span className="font-semibold text-sm text-white">{user.free_usage_count}/10</span>
                            </div>
                            </div>
                        )}
                        {user?.subscription_status === "free" && (
                            <CreateTemplateButton
                                text={t('common.upgrade')}
                                onClick={() => navigate('/pricing')}
                            />
                        )}
                      
                        {/* <Link to="/pricing" className="bg-gradient-to-r from-blue-600 to-indigo-500 text-white font-semibold px-4 py-2 rounded-lg text-sm hover:opacity-90 transition-opacity">UPRGADE</Link> */}
                        <Avatar onClick={handleMenuOpen} sx={{ cursor: 'pointer', bgcolor: 'purple.500' }}>{user?.name?.charAt(0).toUpperCase()}</Avatar>
                    </>
                ) : (
                    <>
                    <Link to="/login" className={navLinkClasses}>{t('auth.login')}</Link>
                    <Link to="/register" className="bg-gradient-to-r from-purple-600 to-blue-500 text-white font-semibold px-4 py-2 rounded-lg text-sm hover:opacity-90 transition-opacity">
                        {t('auth.register')}
                    </Link>
                    </>
                )}
                </div>
                
                {/* Mobile Menu Button */}
                <div className="md:hidden">
                <button onClick={() => setIsMobileMenuOpen(!isMobileMenuOpen)} aria-label="Toggle menu">
                    {isMobileMenuOpen ? (
                        <svg xmlns="http://www.w3.org/2000/svg" className="h-6 w-6 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                            <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
                        </svg>
                    ) : (
                        <svg xmlns="http://www.w3.org/2000/svg" className="h-6 w-6 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                            <path strokeLinecap="round" strokeLinejoin="round" d="M4 6h16M4 12h16m-7 6h7" />
                        </svg>
                    )}
                </button>
                </div>
            </div>
            </nav>

            <Menu
              anchorEl={anchorEl}
              open={Boolean(anchorEl)}
              onClose={handleMenuClose}
              anchorOrigin={{
                vertical: 'bottom',
                horizontal: 'center',
              }}
              transformOrigin={{
                vertical: 'top',
                horizontal: 'center',
              }}
              sx={{
                 '& .MuiPaper-root': { 
                    backgroundColor: '#374151', 
                    color: 'white' ,
                    minWidth : 250
                } }}
            >
                <MenuItem onClick={() => navigate('/settings')}>{t('navigation.settings')}</MenuItem>
                <MenuItem onClick={handleLogout}>{t('auth.logout')}</MenuItem>
            </Menu>



            {/* Mobile Menu */}
            {isMobileMenuOpen && (
            <div className="md:hidden bg-slate-900/80 backdrop-blur-lg border-t border-white/10">
                <div className="px-2 pt-2 pb-3 space-y-1 sm:px-3">
                {isAuthenticated ? (
                    <>
                    <div className="px-3 py-2">
                        <span className="text-white">{t('common.welcome')}, {user?.name}</span>
                    </div>
                    <Link to="/" onClick={() => setIsMobileMenuOpen(false)} className="block px-3 py-2 rounded-md text-base font-medium text-slate-300 hover:text-white hover:bg-white/10">{t('navigation.dashboard')}</Link>
                    <Link to="/pricing" onClick={() => setIsMobileMenuOpen(false)} className="block px-3 py-2 rounded-md text-base font-medium bg-blue-600 text-white hover:bg-blue-700">{t('navigation.pricing')}</Link>
                    <button onClick={handleLogout} className="w-full text-left block px-3 py-2 rounded-md text-base font-medium text-slate-300 hover:text-white hover:bg-white/10">
                        {t('auth.logout')}
                    </button>
                    </>
                ) : (
                    <>
                    <Link to="/login" onClick={() => setIsMobileMenuOpen(false)} className="block px-3 py-2 rounded-md text-base font-medium text-slate-300 hover:text-white hover:bg-white/10">{t('auth.login')}</Link>
                    <Link to="/register" onClick={() => setIsMobileMenuOpen(false)} className="block px-3 py-2 rounded-md text-base font-medium text-slate-300 hover:text-white hover:bg-white/10">{t('auth.register')}</Link>
                    </>
                )}
                </div>
            </div>
            )}
        </header>
            <Box sx={{
                 maxWidth: { xs: '100%', lg: 1500 },
                  mx: 'auto', 
                  position: 'relative', 
                  zIndex: 1, 
                  px: { xs: 2, sm: 3, md: 4 } 
                  }}>
              <main className="pt-16 h-full">
            {children}
        </main>
        </Box>
 
    </div>
  );
};

export default Layout;
