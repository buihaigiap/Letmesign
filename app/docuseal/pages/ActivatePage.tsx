import React, { useState, useEffect, FormEvent } from 'react';
import { User, Loader, Eye, EyeOff } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import upstashService from '../ConfigApi/upstashService';
import toast from 'react-hot-toast';

const ActivatePage: React.FC = () => {
  const urlParams = new URLSearchParams(window.location.search);
  const navigate = useNavigate();
  const [loading, setLoading] = useState(false);
  const [password, setPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);

  const name = urlParams.get('name') || 'Guest';
  const email = urlParams.get('email') || 'no-email@example.com';
  const token = urlParams.get('token') || '';

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    if (!password) {
      toast.error('Please enter a password.');
      return;
    }
    if (password.length < 8) {
      toast.error('Password must be at least 8 characters long.');
      return;
    }

    setLoading(true);
    try {
      const data = await upstashService.activateAccount({
        email,
        name,
        password,
        token
      });
      if (data.success) {
        toast.success('Account activated successfully! You can now log in.');
        setPassword('');
      } else {
        toast.error(data.error || data.message || 'Activation failed. The link may have expired. Please try again.');
      }
    } catch (error) {
      toast.error('Activation failed. The link may have expired. Please try again.');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className=" min-h-screen w-full flex items-center justify-center ">
      <div className="relative z-10 w-full max-w-md p-8 space-y-8 bg-slate-800/50 backdrop-blur-sm border border-slate-700 rounded-2xl shadow-2xl">
        <div className="flex flex-col items-center text-center">
          <div className="p-3 bg-slate-700/50 rounded-full border border-slate-600 mb-4">
             <User className="w-12 h-12 text-purple-400" />
          </div>
          <h1 className="text-3xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-purple-400 to-indigo-400">
            Activate Your Account
          </h1>
          <p className="mt-2 text-slate-400">Welcome, {name}! Set your password to get started.</p>
        </div>
        
        <form onSubmit={handleSubmit} className="space-y-6">
          <div className="space-y-2">
            <label htmlFor="name" className="text-sm font-medium text-slate-300">Name</label>
            <input
              id="name"
              type="text"
              value={name}
              readOnly
              className="w-full px-4 py-2 bg-slate-900/80 border border-slate-700 rounded-lg text-slate-300 cursor-not-allowed focus:outline-none"
            />
          </div>

          <div className="space-y-2">
            <label htmlFor="email" className="text-sm font-medium text-slate-300">Email</label>
            <input
              id="email"
              type="email"
              value={email}
              readOnly
              className="w-full px-4 py-2 bg-slate-900/80 border border-slate-700 rounded-lg text-slate-300 cursor-not-allowed focus:outline-none"
            />
          </div>

          <div className="space-y-2">
            <label htmlFor="password" className="text-sm font-medium text-slate-300">Password</label>
            <div className="relative">
              <input
                id="password"
                type={showPassword ? "text" : "password"}
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder="Enter your new password"
                required
                className="w-full px-4 py-2 pr-12 bg-slate-900/80 border border-slate-700 rounded-lg text-white placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-purple-500 focus:border-purple-500 transition-all duration-300"
              />
              <button
                type="button"
                onClick={() => setShowPassword(!showPassword)}
                className="absolute right-3 top-1/2 transform -translate-y-1/2 text-slate-400 hover:text-slate-300 transition-colors duration-200"
              >
                {showPassword ? <EyeOff className="w-5 h-5" /> : <Eye className="w-5 h-5" />}
              </button>
            </div>
          </div>

          <button
            type="submit"
            disabled={loading}
            className="w-full flex justify-center items-center gap-2 py-3 px-4 font-bold text-white bg-gradient-to-r from-indigo-600 to-purple-600 rounded-lg hover:from-indigo-700 hover:to-purple-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-offset-slate-900 focus:ring-purple-500 transition-all duration-300 disabled:opacity-50 disabled:cursor-not-allowed transform hover:scale-105 disabled:transform-none"
          >
            {loading && <Loader className="animate-spin w-5 h-5" />}
            {loading ? 'Activating...' : 'Activate Account'}
          </button>
        </form>

        <div className="text-center">
            <button
                onClick={() => navigate('/login')}
                className="w-full text-sm font-medium text-slate-400 bg-transparent border border-slate-700 rounded-lg py-2 px-4 hover:bg-slate-800 hover:text-white transition-colors duration-300"
            >
                Back to Login
            </button>
        </div>
      </div>
    </div>
  );
};
export default ActivatePage;