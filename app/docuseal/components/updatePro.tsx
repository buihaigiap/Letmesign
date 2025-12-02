import { OctagonAlert } from 'lucide-react';
const UpdatePro = () => {
    return (
        <>
            <div className='flex justify-center items-center gap-2'>
                <div>
                    <OctagonAlert />
                </div>
                <div>
                    <h3>Unlock with Letmesign Pro</h3>
                    <p>Display your company name and logo when signing documents.</p>
                    <a href="/pricing" className="underline">Learn More</a>
                </div>
            </div>
        </>
    );
}
export default UpdatePro;