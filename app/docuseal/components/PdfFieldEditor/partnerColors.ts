export const partnerColorClasses = [
  'bg-blue-500 bg-opacity-40 border-blue-400',
  'bg-green-500 bg-opacity-40 border-green-400',
  'bg-purple-500 bg-opacity-40 border-purple-400',
  'bg-orange-500 bg-opacity-40 border-orange-400',
  'bg-pink-500 bg-opacity-40 border-pink-400',
  'bg-teal-500 bg-opacity-40 border-teal-400',
  'bg-indigo-500 bg-opacity-40 border-indigo-400',
  'bg-red-500 bg-opacity-40 border-red-400',
  'bg-cyan-500 bg-opacity-40 border-cyan-400',
  'bg-lime-500 bg-opacity-40 border-lime-400',
  'bg-violet-500 bg-opacity-40 border-violet-400',
  'bg-yellow-500 bg-opacity-40 border-yellow-400'
];

export const getPartnerColorClass = (partner: string, partners: string[]) => {
  const index = partners.indexOf(partner);
  return partnerColorClasses[index % partnerColorClasses.length];
};
