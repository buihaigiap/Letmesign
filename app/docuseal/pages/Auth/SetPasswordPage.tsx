import React, { useState, useEffect } from 'react';
import axios from 'axios';
import { motion } from 'framer-motion';
import { Lock, Eye, EyeOff, CheckCircle } from 'lucide-react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import toast from 'react-hot-toast';

const SetPasswordPage: React.FC = () => {
    const [searchParams] = useSearchParams();
    const [password, setPassword] = useState('');
    const [confirmPassword, setConfirmPassword] = useState('');
    const [error, setError] = useState('');
    const [success, setSuccess] = useState('');
    const [loading, setLoading] = useState(false);
    const [showPassword, setShowPassword] = useState(false);
    const [showConfirmPassword, setShowConfirmPassword] = useState(false);
    const navigate = useNavigate();
    const token = searchParams.get('token');

    useEffect(() => {
        if (!token) {
            setError('Invalid or missing activation token');
        }
    }, [token]);

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        setError('');
        setSuccess('');

        if (!token) {
            setError('Invalid or missing activation token');
            return;
        }

        if (password !== confirmPassword) {
            setError('Passwords do not match');
            return;
        }

        if (password.length < 8) {
            setError('Password must be at least 8 characters long');
            return;
        }

        setLoading(true);

        try {
            const response = await axios.post('/api/auth/set-password', {
                token,
                password,
            });

            if (response.data.message) {
                setSuccess(response.data.message);
                toast.success('Password set successfully! Redirecting to login...');
                setTimeout(() => navigate('/login'), 2000);
            } else if (response.data.error) {
                setError(response.data.error);
                toast.error(response.data.error);
            }
        } catch (err: any) {
            const errorMsg = err.response?.data?.error || 'Failed to set password. Please try again.';
            setError(errorMsg);
            toast.error(errorMsg);
        } finally {
            setLoading(false);
        }
    };

    return (
        <div className="min-h-screen flex items-center justify-center ">
            <motion.div
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.6 }}
                className="w-full max-w-md px-4"
            >
                {/* Form Container */}
                <motion.div
                    initial={{ opacity: 0, scale: 0.95 }}
                    animate={{ opacity: 1, scale: 1 }}
                    transition={{ delay: 0.3, duration: 0.5 }}
                    className="relative bg-slate-900/60 backdrop-blur-xl border border-slate-700/50 rounded-3xl p-8 shadow-2xl overflow-hidden"
                >
                    {/* Background Gradient Orbs */}
                    <div className="absolute -top-20 -right-20 w-64 h-64 bg-violet-500/20 rounded-full blur-3xl" />
                    <div className="absolute -bottom-20 -left-20 w-64 h-64 bg-blue-500/20 rounded-full blur-3xl" />

                    <div className="relative z-10">
                        <motion.div
                            initial={{ opacity: 0 }}
                            animate={{ opacity: 1 }}
                            transition={{ delay: 0.4 }}
                            className="text-center mb-8"
                        >
                            <h2 className="text-3xl font-bold text-white mb-2">Set Your Password</h2>
                            <p className="text-slate-400">Welcome to your team! Please set a secure password to get started.</p>
                        </motion.div>

                        <form onSubmit={handleSubmit} className="space-y-6">
                            {/* Password Input */}
                            <motion.div
                                initial={{ opacity: 0, x: -20 }}
                                animate={{ opacity: 1, x: 0 }}
                                transition={{ delay: 0.5 }}
                                className="relative"
                            >
                                <div className="absolute inset-y-0 left-0 pl-4 flex items-center pointer-events-none">
                                    <Lock className="w-5 h-5 text-slate-400" />
                                </div>
                                <input
                                    type={showPassword ? "text" : "password"}
                                    placeholder="New Password"
                                    value={password}
                                    onChange={e => setPassword(e.target.value)}
                                    required
                                    minLength={8}
                                    className="w-full bg-slate-800/70 border border-slate-600/50 rounded-xl pl-12 pr-12 py-4 text-white placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-violet-500 focus:border-transparent transition-all duration-300 hover:bg-slate-800/80"
                                />
                                <button
                                    type="button"
                                    onClick={() => setShowPassword(!showPassword)}
                                    className="absolute inset-y-0 right-0 pr-4 flex items-center text-slate-400 hover:text-slate-300 transition-colors"
                                >
                                    {showPassword ? <EyeOff className="w-5 h-5" /> : <Eye className="w-5 h-5" />}
                                </button>
                            </motion.div>

                            {/* Confirm Password Input */}
                            <motion.div
                                initial={{ opacity: 0, x: -20 }}
                                animate={{ opacity: 1, x: 0 }}
                                transition={{ delay: 0.6 }}
                                className="relative"
                            >
                                <div className="absolute inset-y-0 left-0 pl-4 flex items-center pointer-events-none">
                                    <Lock className="w-5 h-5 text-slate-400" />
                                </div>
                                <input
                                    type={showConfirmPassword ? "text" : "password"}
                                    placeholder="Confirm Password"
                                    value={confirmPassword}
                                    onChange={e => setConfirmPassword(e.target.value)}
                                    required
                                    minLength={8}
                                    className="w-full bg-slate-800/70 border border-slate-600/50 rounded-xl pl-12 pr-12 py-4 text-white placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-violet-500 focus:border-transparent transition-all duration-300 hover:bg-slate-800/80"
                                />
                                <button
                                    type="button"
                                    onClick={() => setShowConfirmPassword(!showConfirmPassword)}
                                    className="absolute inset-y-0 right-0 pr-4 flex items-center text-slate-400 hover:text-slate-300 transition-colors"
                                >
                                    {showConfirmPassword ? <EyeOff className="w-5 h-5" /> : <Eye className="w-5 h-5" />}
                                </button>
                            </motion.div>

                            {/* Password Requirements */}
                            <motion.div
                                initial={{ opacity: 0 }}
                                animate={{ opacity: 1 }}
                                transition={{ delay: 0.7 }}
                                className="p-3 bg-slate-800/50 rounded-lg border border-slate-700/50"
                            >
                                <p className="text-xs text-slate-400 mb-2">Password must contain:</p>
                                <ul className="text-xs text-slate-400 space-y-1">
                                    <li className="flex items-center gap-2">
                                        <CheckCircle className={`w-3 h-3 ${password.length >= 8 ? 'text-green-400' : 'text-slate-600'}`} />
                                        At least 8 characters
                                    </li>
                                </ul>
                            </motion.div>

                            {/* Error Message */}
                            {error && (
                                <motion.div
                                    initial={{ opacity: 0, scale: 0.95 }}
                                    animate={{ opacity: 1, scale: 1 }}
                                    className="p-4 bg-red-500/10 border border-red-500/30 rounded-xl text-red-300 text-sm text-center backdrop-blur-sm"
                                >
                                    {error}
                                </motion.div>
                            )}

                            {/* Success Message */}
                            {success && (
                                <motion.div
                                    initial={{ opacity: 0, scale: 0.95 }}
                                    animate={{ opacity: 1, scale: 1 }}
                                    className="p-4 bg-green-500/10 border border-green-500/30 rounded-xl text-green-300 text-sm text-center backdrop-blur-sm"
                                >
                                    {success}
                                </motion.div>
                            )}

                            {/* Submit Button */}
                            <motion.button
                                initial={{ opacity: 0, y: 10 }}
                                animate={{ opacity: 1, y: 0 }}
                                transition={{ delay: 0.8 }}
                                whileTap={{ scale: 0.98 }}
                                type="submit"
                                disabled={loading || !token}
                                className="w-full py-4 px-6 rounded-xl text-lg font-semibold text-white bg-gradient-to-r from-violet-600 via-purple-600 to-blue-600 hover:from-violet-500 hover:via-purple-500 hover:to-blue-500 shadow-lg hover:shadow-xl hover:shadow-violet-500/25 transform transition-all duration-300 disabled:opacity-50 disabled:cursor-not-allowed disabled:transform-none relative overflow-hidden group"
                            >
                                <span className="relative z-10 flex items-center justify-center gap-2">
                                    {loading ? (
                                        <>
                                            <motion.div
                                                animate={{ rotate: 360 }}
                                                transition={{ duration: 1, repeat: Infinity, ease: "linear" }}
                                                className="w-5 h-5 border-2 border-white/30 border-t-white rounded-full"
                                            />
                                            Setting Password...
                                        </>
                                    ) : (
                                        <>
                                            Set Password & Activate Account
                                            <motion.div
                                                initial={{ x: -10, opacity: 0 }}
                                                animate={{ x: 0, opacity: 1 }}
                                                transition={{ delay: 0.2 }}
                                            >
                                                →
                                            </motion.div>
                                        </>
                                    )}
                                </span>
                                <div className="absolute inset-0 bg-gradient-to-r from-violet-400 to-blue-400 opacity-0 group-hover:opacity-20 transition-opacity duration-300" />
                            </motion.button>
                        </form>

                        {/* Back to Login */}
                        <motion.div
                            initial={{ opacity: 0 }}
                            animate={{ opacity: 1 }}
                            transition={{ delay: 0.9 }}
                            className="mt-6 text-center"
                        >
                            <button
                                onClick={() => navigate('/login')}
                                className="text-slate-400 hover:text-white transition-colors text-sm"
                            >
                                Back to Login
                            </button>
                        </motion.div>
                    </div>
                </motion.div>
            </motion.div>
        </div>
    );
};

export default SetPasswordPage;
