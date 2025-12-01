import React, { useState } from 'react';
import upstashService from '../../ConfigApi/upstashService';
import { motion } from 'framer-motion';
import { Mail, Lock, ArrowLeft, Eye, EyeOff } from 'lucide-react';
import { useNavigate } from 'react-router-dom';

const ForgotPasswordForm: React.FC = () => {
    const [step, setStep] = useState<'email' | 'reset'>('email');
    const [email, setEmail] = useState('');
    const [resetCode, setResetCode] = useState('');
    const [newPassword, setNewPassword] = useState('');
    const [error, setError] = useState('');
    const [success, setSuccess] = useState('');
    const [loading, setLoading] = useState(false);
    const [showPassword, setShowPassword] = useState(false);
    const navigate = useNavigate();

    const handleEmailSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        setError('');
        setSuccess('');
        setLoading(true);

        if (!email.trim()) {
            setError('Please enter your email address');
            setLoading(false);
            return;
        }

        console.log('Email being sent:', email);

        try {
            const data = await upstashService.forgotPassword({ email });
            console.log('API Response:', data);

            if (data.success) {
                setSuccess('Reset code has been sent to your email!');
                setTimeout(() => {
                    setStep('reset');
                    setSuccess('');
                }, 2000);
            }   
        } catch (err) {
            console.error('API Error:', err);
            setError(err?.error);
        } finally {
            setLoading(false);
        }
    };

    const handleResetSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        setError('');
        setSuccess('');
        setLoading(true);

        try {
            const data = await upstashService.resetPassword({ email, new_password: newPassword, reset_code: resetCode });
           
            if (data.success) {
                setSuccess('Password has been reset successfully! Redirecting to login page...');
                setTimeout(() => navigate('/login'), 2000);
            } else {
                setError(data.message || 'Password reset failed');
            }
        } catch (err) {
            setError('An error occurred. Please try again.');
        } finally {
            setLoading(false);
        }
    };

    return (
        <div className="min-h-screen flex items-center justify-center">
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
                            {step === 'email' ? 'Forgot Password' : 'Reset Password'}
                        </motion.h2>

                        {step === 'email' ? (
                            <form onSubmit={handleEmailSubmit} className="space-y-6">
                                {/* Email Input */}
                                <motion.div
                                    initial={{ opacity: 0, x: -20 }}
                                    animate={{ opacity: 1, x: 0 }}
                                    transition={{ delay: 0.5 }}
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
                                    transition={{ delay: 0.6 }}
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
                                                Sending...
                                            </>
                                        ) : (
                                            <>
                                                Send Reset Code
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
                        ) : (
                            <form onSubmit={handleResetSubmit} className="space-y-6">
                                {/* Reset Code Input */}
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
                                        type="text"
                                        placeholder="Reset Code"
                                        value={resetCode}
                                        onChange={e => setResetCode(e.target.value)}
                                        required
                                        className="w-full bg-slate-800/70 border border-slate-600/50 rounded-xl pl-12 pr-4 py-4 text-white placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-violet-500 focus:border-transparent transition-all duration-300 hover:bg-slate-800/80"
                                    />
                                </motion.div>

                                {/* New Password Input */}
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
                                        type={showPassword ? "text" : "password"}
                                        placeholder="New Password"
                                        value={newPassword}
                                        onChange={e => setNewPassword(e.target.value)}
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
                                    transition={{ delay: 0.7 }}
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
                                                Resetting...
                                            </>
                                        ) : (
                                            <>
                                                Reset Password
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
                        )}

                        {/* Back Button */}
                        <motion.button
                            initial={{ opacity: 0 }}
                            animate={{ opacity: 1 }}
                            transition={{ delay: 0.8 }}
                            onClick={() => step === 'reset' ? setStep('email') : navigate('/login')}
                            className="w-full mt-6 py-3 px-4 rounded-xl text-sm font-medium text-slate-400 hover:text-violet-400 transition-colors duration-200 flex items-center justify-center gap-2 group"
                            whileHover={{ scale: 1.02 }}
                            whileTap={{ scale: 0.98 }}
                        >
                            <ArrowLeft className="w-4 h-4 group-hover:-translate-x-1 transition-transform" />
                            {step === 'reset' ? 'Back to email input' : 'Back to login'}
                        </motion.button>
                    </div>
                </motion.div>
            </motion.div>
        </div>
    );
};

export default ForgotPasswordForm;