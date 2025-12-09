import React, { useState } from 'react';
import PricingComponent from './PricingComponent';
import SubscriptionModal from './SubscriptionModal';
import { plans } from './type';
import { useNavigate } from 'react-router-dom';
const PricingPage: React.FC = () => {
  const [modalState, setModalState] = useState<any>({ isOpen: false });
  const navigate = useNavigate();
  const [error, setError] = useState('');

  const handleSubscribe = (plan: any, price?: number, period?: 'monthly' | 'yearly') => {
    if (plan.id === 'free') {
      alert(`Enjoy the ${plan.name} plan! Your journey starts now.`);
      navigate('/');
      return;
    }
    
    // It must be the Pro plan
    if (plan.id === 'pro' && price !== undefined && period) {
      setModalState({ isOpen: true, plan, price, period });
      setError('');
    }
  };

  const handleCloseModal = () => {
    setModalState({ isOpen: false });
  };


  const getModalPlan = (): any | null => {
      if (!modalState.plan || modalState.price === undefined || !modalState.period) return null;
      return {
          id: modalState.plan.id,
          name: modalState.plan.name,
          price: modalState.price,
          period: modalState.period,
          description: '',
          features: [],
          buttonText: '',
          buttonVariant: 'primary' as const,
      };
  };
  const modalPlan = getModalPlan();

  return (
    <div>
      <div className="absolute top-[-10rem] left-[-20rem] w-[40rem] h-[40rem] bg-purple-500/20 rounded-full blur-3xl animate-pulse"></div>
      
      <div className="relative z-10 flex flex-col items-center ">
        <header className="text-center mb-5">
          <h1 className="text-4xl md:text-5xl font-bold tracking-tight bg-clip-text text-transparent bg-gradient-to-br from-white to-slate-400">
            Find the perfect plan
          </h1>
          <p className="mt-4 text-lg text-slate-300 max-w-2xl mx-auto">
            Start for free, then upgrade for more power. No hidden fees, cancel anytime.
          </p>
        </header>

        <main className="w-full max-w-7xl mx-auto">
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-8 items-stretch">
                {plans.map(plan => (
                    <PricingComponent key={plan.id} plan={plan} onSubscribe={handleSubscribe} />
                ))}
            </div>
        </main>
        
        <footer className="text-center mt-12 text-slate-400">
            <p>Need a custom solution? <a href="mailto:sales@example.com" className="font-medium text-purple-400 hover:text-purple-300">Contact us</a>.</p>
        </footer>
      </div>

      {modalPlan && (
        <SubscriptionModal
          isOpen={modalState.isOpen}
          onClose={handleCloseModal}
          plan={modalPlan}
          error={error}
        />
      )}
    </div>
  );
};

export default PricingPage;