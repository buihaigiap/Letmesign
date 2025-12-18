import React, { createContext, useContext, useState } from 'react';

interface AuthContextType {
    isAuthenticated: boolean;
    user: any| null;
    login: (token: string, userData: any) => void;
    logout: () => void;
}

const AuthContext = createContext<AuthContextType | null>(null);

export const useAuth = () => {
    const context = useContext(AuthContext);
    if (!context) {
        throw new Error("useAuth must be used within an AuthProvider");
    }
    return context;
};

export const AuthProvider = ({ children }: { children: React.ReactNode }) => {
    const [isAuthenticated, setIsAuthenticated] = useState(false);
    const [user, setUser] = useState<{name: string} | null>(null);

    const login = (token: string, userData: any) => {
        setIsAuthenticated(true);
        setUser(userData);
         window.location.hash = '/'; // Redirect after login
    };

    const logout = () => {
        setIsAuthenticated(false);
        setUser(null);
        window.location.hash = '/login'; // Redirect after logout
    };
    
    const value = { isAuthenticated, user, login, logout };

    return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}
