
import { createContext, useState, useContext, useEffect, ReactNode, useCallback } from 'react';
import { User } from '../types';
import upstashService from '../ConfigApi/upstashService';

interface AuthContextType {
  isAuthenticated: boolean;
  isLoading: boolean;
  user: User | null;
  token: string | null;
  login: (token: string, user: User) => void;
  logout: () => void;
  refreshUser: () => Promise<void>;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

export const AuthProvider = ({ children }: { children: ReactNode }) => {
  const [user, setUser] = useState<User | null>(null);
  const [token, setToken] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  useEffect(() => {
    const storedToken = localStorage.getItem('token');
    const storedUser = localStorage.getItem('user');
    
    if (storedToken) {
      // Validate token with API
      upstashService.getMe()
        .then(data => {
          if (data.success && data.data) {
            // Token is valid, set user data from API response
            setToken(storedToken);
            setUser(data.data);
            // Update localStorage with fresh user data
            localStorage.setItem('user', JSON.stringify(data.data));
          } else {
            // Token is invalid, clear stored data
            localStorage.removeItem('token');
            localStorage.removeItem('user');
            setToken(null);
            setUser(null);
          }
        })
        .catch(() => {
          // API call failed, clear stored data
          localStorage.removeItem('token');
          localStorage.removeItem('user');
          setToken(null);
          setUser(null);
        })
        .finally(() => {
          setIsLoading(false);
        });
    } else {
      // No stored token, finish loading
      if (storedUser) {
        localStorage.removeItem('user');
      }
      setIsLoading(false);
    }
  }, []);

  const login = (newToken: string, newUser: User) => {
    localStorage.setItem('token', newToken);
    localStorage.setItem('user', JSON.stringify(newUser));
    setToken(newToken);
    setUser(newUser);
    setIsLoading(false);
  };

  const logout = useCallback(async () => {
    try {
      // Call logout API to clean up server-side data (OAuth tokens, etc.)
      await upstashService.logout();
    } catch (error) {
      console.error('Logout API call failed:', error);
      // Continue with local cleanup even if API call fails
    }
    
    // Clear local storage and state
    localStorage.removeItem('token');
    localStorage.removeItem('user');
    setToken(null);
    setUser(null);
    setIsLoading(false);
  }, []);

  const refreshUser = useCallback(async () => {
    try {
      const data = await upstashService.getMe();
      if (data.success && data.data) {
        setUser(data.data);
        localStorage.setItem('user', JSON.stringify(data.data));
      }
    } catch (error) {
      console.error('Failed to refresh user data:', error);
    }
  }, []);

  return (
    <AuthContext.Provider value={{ isAuthenticated: !!token, isLoading, user, token, login, logout, refreshUser }}>
      {children}
    </AuthContext.Provider>
  );
};

export const useAuth = () => {
  const context = useContext(AuthContext);
  if (context === undefined) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
};

