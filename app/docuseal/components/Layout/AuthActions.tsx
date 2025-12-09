// components/AuthActions.tsx
import { Link } from 'react-router-dom';
import { Mail } from 'lucide-react';
import CreateTemplateButton from '../CreateTemplateButton';

export const AuthActions = ({
  isAuthenticated,
  user,
  t,
  navigate,
  onAvatarClick,
}) => {
  if (!isAuthenticated) {
    return (
      <>
        <Link to="/login" className="text-slate-300 hover:text-white">
          {t('auth.login')}
        </Link>

        <Link
          to="/register"
          className="bg-gradient-to-r from-purple-600 to-blue-500 text-white font-semibold px-4 py-2 rounded-lg text-sm"
        >
          {t('auth.register')}
        </Link>
      </>
    );
  }

  return (
    <>
      {user?.free_usage_count !== undefined && (
        <div className="flex items-center space-x-2 bg-white/5 border border-white/10 px-3 py-1.5 rounded-lg">
          <Mail className="h-5 w-5 text-purple-400" />
          <span className="font-semibold text-sm text-white">
            {user.free_usage_count}/10
          </span>
        </div>
      )}

      {user?.subscription_status === 'free' && (
        <CreateTemplateButton
          text={t('common.upgrade')}
          onClick={() => navigate('/pricing')}
        />
      )}

      {onAvatarClick}
    </>
  );
};
