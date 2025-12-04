import React, { useState } from 'react';
import { useAuth } from '../../contexts/AuthContext';
import upstashService from '../../ConfigApi/upstashService';
import { motion } from 'framer-motion';
import { User, Mail, Lock, Eye, EyeOff, FileText } from 'lucide-react';
import { useNavigate, useSearchParams } from 'react-router-dom';

const AuthForm: React.FC<{ isRegister?: boolean }> = ({ isRegister }) => {
    const [name, setName] = useState('');
    const [email, setEmail] = useState('');
    const [password, setPassword] = useState('');
    const [error, setError] = useState('');
    const [success, setSuccess] = useState('');
    const [loading, setLoading] = useState(false);
    const [showPassword, setShowPassword] = useState(false);
    const { login } = useAuth();
    const navigate = useNavigate();
    const [searchParams] = useSearchParams();
    const redirectUrl = searchParams.get('redirect');

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        setError('');
        setSuccess('');
        setLoading(true);

        try {
            if (isRegister) {
                // Register API call
                const data = await upstashService.Register({ name, email, password });
                if (data.success) {
                    setSuccess('Registration successful! Redirecting to login page...');
                    setTimeout(() => navigate('/login'), 2000);
                } else {
                    setError(data.message || 'Registration failed');
                }
            } else {
                // Login API call
                const data = await upstashService.Login({ email, password });

                if (data.success) {
                    login(data.data.token, data.data.user);
                    // Store redirect URL if present
                    if (redirectUrl) {
                        localStorage.setItem( 'redirectAfterLogin', redirectUrl);
                    }
                    navigate('/');
                } else {
                    setError(data.message || 'Login failed');
                }
            }
        } catch (err) {
            setError(err?.error || 'An error occurred');
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
                className="w-full max-w-md"
            >
                {/* Form Container */}
                <motion.div
                    initial={{ opacity: 0, scale: 0.95 }}
                    animate={{ opacity: 1, scale: 1 }}
                    transition={{ delay: 0.3, duration: 0.5 }}
                    className="relative bg-slate-900/60 backdrop-blur-xl border border-slate-700/50 rounded-3xl p-8 shadow-2xl overflow-hidden"
                >
                    <div className="relative z-10">
                        <motion.h2
                            initial={{ opacity: 0 }}
                            animate={{ opacity: 1 }}
                            transition={{ delay: 0.4 }}
                            className="text-center text-3xl font-bold text-white mb-8"
                        >
                            {isRegister ? 'Create Account' : 'Login'}
                        </motion.h2>

                        <form onSubmit={handleSubmit} className="space-y-6">
                            {/* Name Input - Only for Register */}
                            {isRegister && (
                                <motion.div
                                    initial={{ opacity: 0, x: -20 }}
                                    animate={{ opacity: 1, x: 0 }}
                                    transition={{ delay: 0.5 }}
                                    className="relative"
                                >
                                    <div className="absolute inset-y-0 left-0 pl-4 flex items-center pointer-events-none">
                                        <User className="w-5 h-5 text-slate-400" />
                                    </div>
                                    <input
                                        type="text"
                                        placeholder="Your Name"
                                        value={name}
                                        onChange={e => setName(e.target.value)}
                                        required
                                        className="w-full bg-slate-800/70 border border-slate-600/50 rounded-xl pl-12 pr-4 py-4 text-white placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-violet-500 focus:border-transparent transition-all duration-300 hover:bg-slate-800/80"
                                    />
                                </motion.div>
                            )}

                            {/* Email Input */}
                            <motion.div
                                initial={{ opacity: 0, x: -20 }}
                                animate={{ opacity: 1, x: 0 }}
                                transition={{ delay:0.6}}
                                className="relative"
                            >
                                <div className="absolute inset-y-0 left-0 pl-4 flex items-center pointer-events-none">
                                    <Mail className="w-5 h-5 text-slate-400" />
                                </div>
                                <input
                                    type="email"
                                    placeholder="Email Address"
                                    value={email}
                                    onChange={e => setEmail(e.target.value)}
                                    required
                                    className="w-full bg-slate-800/70 border border-slate-600/50 rounded-xl pl-12 pr-4 py-4 text-white placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-violet-500 focus:border-transparent transition-all duration-300 hover:bg-slate-800/80"
                                />
                            </motion.div>

                            {/* Password Input */}
                            <motion.div
                                initial={{ opacity: 0, x: -20 }}
                                animate={{ opacity: 1, x: 0 }}
                                transition={{ delay: isRegister ? 0.7 : 0.6 }}
                                className="relative"
                            >
                                <div className="absolute inset-y-0 left-0 pl-4 flex items-center pointer-events-none">
                                    <Lock className="w-5 h-5 text-slate-400" />
                                </div>
                                <input
                                    type={showPassword ? "text" : "password"}
                                    placeholder="Password"
                                    value={password}
                                    onChange={e => setPassword(e.target.value)}
                                    required
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
                                animate={{ opacity: 1, y: 0 }}
                                transition={{ delay: isRegister ? 0.8 : 0.7 }}
                                whileTap={{ scale: 0.98 }}
                                type="submit"
                                disabled={loading}
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
                                                                                        Processing...
                                        </>
                                    ) : (
                                        <>
                                            {isRegister ? 'Register' : 'Login'}
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

                            {/* Toggle Link */}
                            <motion.p
                                initial={{ opacity: 0 }}
                                animate={{ opacity: 1 }}
                                transition={{ delay: isRegister ? 0.9 : 0.8 }}
                                className="text-center text-sm text-slate-400"
                            >
                                {isRegister ? 'Already have an account? ' : 'Don\'t have an account? '}
                                <motion.button
                                    onClick={() => navigate(isRegister ? '/login' : '/register')}
                                    className="font-medium text-violet-400 hover:text-violet-300 transition-colors duration-200 relative"
                                    whileHover={{ scale: 1.05 }}
                                    whileTap={{ scale: 0.95 }}
                                >
                                    {isRegister ? 'Login now' : 'Register now'}
                                    <motion.div
                                        className="absolute -bottom-1 left-0 w-0 h-0.5 bg-violet-400 group-hover:w-full transition-all duration-300"
                                        whileHover={{ width: "100%" }}
                                    />
                                </motion.button>
                            </motion.p>

                            {/* Forgot Password Link - Only for Login */}
                            {!isRegister && (
                                <motion.p
                                    initial={{ opacity: 0 }}
                                    animate={{ opacity: 1 }}
                                    transition={{ delay: 0.9 }}
                                    className="text-center text-sm text-slate-400"
                                >
                                    <motion.button
                                        onClick={() => navigate('/forgot-password')}
                                        className="font-medium text-violet-400 hover:text-violet-300 transition-colors duration-200"
                                        whileHover={{ scale: 1.05 }}
                                        whileTap={{ scale: 0.95 }}
                                    >
                                        Forgot password?
                                    </motion.button>
                                </motion.p>
                            )}
                        </form>
                    </div>
                </motion.div>
            </motion.div>
        </div>
    );
}

export default AuthForm;