import { BrowserRouter, Routes, Route, Navigate, Outlet } from 'react-router-dom';
import { AuthProvider, useAuth } from './contexts/AuthContext';
import LoginPage from './pages/Auth/LoginPage';
import RegisterPage from './pages/Auth/RegisterPage';
import ForgotPasswordForm from './pages/Auth/ForgotPasswordForm';
import SetPasswordPage from './pages/Auth/SetPasswordPage';
import DashboardPage from './pages/DashboardPage/DashboardPage';
import TemplateDetailPage from './pages/Pricing/TemplateDetailPage';
import TemplateEditorPage from './pages/TemplateEditorPage';
import TemplateEditPage from './pages/TemplateEdit/TemplateEditPage';
import SignPage from './pages/TemplateEdit/SignPage';
import SignedSubmissionPage from './pages/SignedSubmissionPage';
import Layout from './components/Layout';
import PricingPage from './pages/Pricing/PricingPage';
import FolderPage from './pages/DashboardPage/FolderPage';
import SettingsPage from './pages/Settings/SettingsPage';
import { Toaster } from "react-hot-toast";
import ActivatePage from './pages/Settings/Activate/ActivatePage';
import './src/i18n';
import { useEffect } from 'react';
import { useNavigate } from 'react-router-dom';

// Global component to handle Google Drive OAuth redirect
const GoogleDriveOAuthHandler: React.FC = () => {
  const navigate = useNavigate();

  useEffect(() => {
    const urlParams = new URLSearchParams(window.location.search);
    if (urlParams.get('google_drive_connected') === '1') {
      // Just ensure we're on the dashboard, don't remove the parameter
      // The EmptyState component will handle the parameter and auto-open the modal
      if (window.location.pathname !== '/') {
        navigate('/?google_drive_connected=1', { replace: true });
      }
    }
  }, [navigate]);

  return null;
};

// Fix: Replaced the old PrivateRoute component with a new layout route component.
// This uses `<Outlet />` to render child routes if the user is authenticated, resolving the errors.
const PrivateRoutes = () => {
  const { isAuthenticated, isLoading } = useAuth();
  
  // Show loading while validating authentication
  if (isLoading) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto"></div>
          <p className="mt-4 text-gray-600">Loading...</p>
        </div>
      </div>
    );
  }
  
  return isAuthenticated ? <Outlet /> : <Navigate to="/login" />;
};

function App() {
  return (
    <AuthProvider>
      <BrowserRouter>
        <Layout>
          <GoogleDriveOAuthHandler />
          <Routes>
            <Route path="/login" element={<LoginPage />} />
            <Route path="/register" element={<RegisterPage />} />
            <Route path="/forgot-password" element={<ForgotPasswordForm />} />
            <Route path="/set-password" element={<SetPasswordPage />} />
            <Route path="/pricing" element={<PricingPage />} />
            <Route path="/sign/:token" element={<SignPage />} />
            <Route path="/signed-submission/:token" element={<SignedSubmissionPage />} />
            <Route path="/activate" element={<ActivatePage />} />
            <Route path="/templates/:token/edit" element={<TemplateEditPage />} />
            
            <Route element={<PrivateRoutes />}>
              <Route path="/" element={<DashboardPage />} />
              <Route path="/folders/:folderId" element={<FolderPage />} />
              <Route path="/templates/:id" element={<TemplateDetailPage />} />
              <Route path="/templates/:id/editor" element={<TemplateEditorPage />} />
              <Route path="/settings/*" element={<SettingsPage />} />
            </Route>

            <Route path="*" element={<Navigate to="/" />} />
          </Routes>
        </Layout>
            <Toaster />
      </BrowserRouter>
    </AuthProvider>
  );
}

export default App;
