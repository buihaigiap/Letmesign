import axiosClient from "./axiosClient";
import axios from "axios";

const JWT_LOCAL_STORAGE_KEY = 'token';

const upstashService = {
    // Auth APIs
    Login: async (data: any): Promise<any> => {
        const url = '/api/auth/login';
        return await axiosClient.post(url, data)
    },
    Register: async (data: any): Promise<any> => {
        const url = '/api/auth/register';
        return await axiosClient.post(url, data)
    },
    getMe: async (): Promise<any> => {
        const url = '/api/me';
        return await axiosClient.get(url)
    },
    changePassword: async (data: any): Promise<any> => {
        const url = '/api/auth/change-password';
        return await axiosClient.put(url, data)
    },
    forgotPassword: async (data: any): Promise<any> => {
        const url = '/api/auth/forgot-password';
        return await axiosClient.post(url, data)
    },
    resetPassword: async (data: any): Promise<any> => {
        const url = '/api/auth/reset-password';
        return await axiosClient.post(url, data)
    },

    updateProfile: async (data: any): Promise<any> => {
        const url = '/api/auth/profile';
        return await axiosClient.put(url, data)
    },
    logout: async (): Promise<any> => {
        const url = '/api/auth/logout';
        return await axiosClient.post(url)
    },
    getGoogleDriveStatus: async (): Promise<any> => {
        const url = '/api/auth/google-drive/status';
        return await axiosClient.get(url)
    },
    // Template APIs
    getTemplates: async (params?: { page?: number; limit?: number; search?: string }): Promise<any> => {
        const url = '/api/templates';
        const queryParams = new URLSearchParams();
        if (params?.page) queryParams.append('page', params.page.toString());
        if (params?.limit) queryParams.append('limit', params.limit.toString());
        if (params?.search) queryParams.append('search', params.search.toString());
        const fullUrl = queryParams.toString() ? `${url}?${queryParams.toString()}` : url;
        return await axiosClient.get(fullUrl)
    },
    createTemplateFromFile: async (data: any): Promise<any> => {
        const url = '/api/templates/from-file';
        return await axiosClient.post(url, data)
    },
    getTemplateFullInfo: async (id: number): Promise<any> => {
        const url = `/api/templates/${id}/full-info`;
        return await axiosClient.get(url)
    },
    cloneTemplate: async (id: any): Promise<any> => {
        const url = `/api/templates/${id}/clone`;
        return await axiosClient.post(url, {})
    },
    deleteTemplate: async (id: any): Promise<any> => {
        const url = `/api/templates/${id}`;
        return await axiosClient.delete(url)
    },

    // File APIs
    uploadFile: async (formData: FormData): Promise<any> => {
        const url = '/api/files/upload';
        return await axiosClient.post(url, formData, {
            headers: { 'Content-Type': 'multipart/form-data' }
        })
    },
    uploadPublicFile: async (formData: FormData, onUploadProgress?: (progressEvent: any) => void): Promise<any> => {
        const url = '/api/files/upload/public';
        // Use axios directly to bypass authorization interceptor for public endpoint
        return await axios.post(`${axiosClient.defaults.baseURL}${url}`, formData, {
            headers: { 'Content-Type': 'multipart/form-data' },
            onUploadProgress
        })
    },
    deletePublicFile: async (fileUrl: string): Promise<any> => {
        const url = '/api/files/delete/public';
        // Use axios directly to bypass authorization interceptor for public endpoint
        return await axios.delete(`${axiosClient.defaults.baseURL}${url}`, {
            data: { file_url: fileUrl }
        })
    },
    previewFile: async (url: string) => {
        // Use axios directly to bypass response interceptor that strips headers
        const fullUrl = `${axiosClient.defaults.baseURL}/api/files/preview/${url}`;
        const config = {
            headers: {
                'Authorization': localStorage.getItem(JWT_LOCAL_STORAGE_KEY) ? `Bearer ${localStorage.getItem(JWT_LOCAL_STORAGE_KEY)}` : undefined
            },
            responseType: 'json' as const
        };
        return await axios.get(fullUrl, config);
    },
    // downLoadFile: async (token: string) => {
    //     const apiUrl = `${axiosClient.defaults.baseURL}/public/download/${token}`;
    //     const config = {
    //         headers: {
    //             'Authorization': localStorage.getItem(JWT_LOCAL_STORAGE_KEY) ? `Bearer ${localStorage.getItem(JWT_LOCAL_STORAGE_KEY)}` : undefined
    //         },
    //         responseType: 'blob' as const
    //     };
    //     return await axios.get(apiUrl, config);
    // },

    // Submission APIs
    createSubmission: async (data: any): Promise<any> => {
        const url = '/api/submissions';
        return await axiosClient.post(url, data)
    },
    deleteSubmitter: async (submitterId: number): Promise<any> => {
        const url = `/api/submitters/${submitterId}`;
        return await axiosClient.delete(url)
    },

    // Submission/Signatures APIs
    getSubmissionSignatures: async (token: string): Promise<any> => {
        const url = `/public/submissions/${token}/signatures`;
        return await axiosClient.get(url)
    },
    getSubmitterInfo: async (token: string): Promise<any> => {
        const url = `/public/submissions/${token}`;
        return await axiosClient.get(url)
    },
    getSubmissionFields: async (token: string): Promise<any> => {
        const url = `/public/submissions/${token}/fields`;
        return await axiosClient.get(url)
    },
    bulkSign: async (token: string, data: any): Promise<any> => {
        const url = `/public/signatures/bulk/${token}`;
        return await axiosClient.post(url, data)
    },
    resubmitSubmission: async (token: string): Promise<any> => {
        const url = `/public/submissions/${token}/resubmit`;
        return await axiosClient.put(url)
    },
    sendCopyEmail: async (token: string): Promise<any> => {
        const url = `/public/submissions/${token}/send-copy`;
        return await axiosClient.post(url)
    },

    // Field APIs
    createField: async (templateId: number, data: any): Promise<any> => {
        const url = `/api/templates/${templateId}/fields`;
        return await axiosClient.post(url, data)
    },
    updateField: async (templateId: number, fieldId: number, data: any): Promise<any> => {
        const url = `/api/templates/${templateId}/fields/${fieldId}`;
        return await axiosClient.put(url, data)
    },
    deleteField: async (templateId: number, fieldId: number): Promise<any> => {
        const url = `/api/templates/${templateId}/fields/${fieldId}`;
        return await axiosClient.delete(url)
    },

    // Folder APIs
    getFolders: async (): Promise<any> => {
        const url = '/api/folders';
        return await axiosClient.get(url)
    },
    getFolderTemplates: async (folderId: number, page?: number, limit?: number): Promise<any> => {
        const params = new URLSearchParams();
        if (page) params.append('page', page.toString());
        if (limit) params.append('limit', limit.toString());
        const url = `/api/folders/${folderId}/templates${params.toString() ? '?' + params.toString() : ''}`;
        return await axiosClient.get(url)
    },
    moveTemplate: async (data: any): Promise<any> => {
        const url = '/api/folders';
        return await axiosClient.post(url, data)
    },
    updateFolder: async (folderId: number, data: any): Promise<any> => {
        const url = `/api/folders/${folderId}`;
        return await axiosClient.put(url, data)
    },
    moveTemplatePut: async (template_id: any, parent_folder_id: any): Promise<any> => {
        const url = `/api/templates/${template_id}/move/${parent_folder_id}`;
        return await axiosClient.put(url)
    },
    deleteFolder: async (folderId: number): Promise<any> => {
        const url = `/api/folders/${folderId}`;
        return await axiosClient.delete(url)
    },
    // Team APIs can be added here
    addTeam : async (data: any): Promise<any> => {
        const url = '/api/auth/users';
        return await axiosClient.post(url, data)
    },
    updateTeam : async (id: number, data: any): Promise<any> => {
        const url = `/api/admin/members/${id}`;
        return await axiosClient.put(url, data)
    },
    deleteTeam : async (id: number): Promise<any> => {
        const url = `/api/admin/members/${id}`;
        return await axiosClient.delete(url)
    },
    activateAccount : async (data: any): Promise<any> => {
        const url = '/api/auth/activate';
        return await axiosClient.post(url, data)
    },
    getUserAccounts : async (page?: number, limit?: number): Promise<any> => {
        const params = new URLSearchParams();
        if (page) params.append('page', page.toString());
        if (limit) params.append('limit', limit.toString());
        const url = `/api/admin/members${params.toString() ? '?' + params.toString() : ''}`;
        return await axiosClient.get(url)
    },

    // Reminder Settings APIs
    getReminderSettings: async (): Promise<any> => {
        const url = '/api/reminder-settings';
        return await axiosClient.get(url)
    },
    updateReminderSettings: async (data: any): Promise<any> => {
        const url = '/api/reminder-settings';
        return await axiosClient.put(url, data)
    },

    // Basic Settings APIs
    getBasicSettings: async (): Promise<any> => {
        const url = '/api/settings/basic-info';
        return await axiosClient.get(url)
    },
    updateBasicSettings: async (data: any): Promise<any> => {
        const url = '/api/settings/basic-info';
        return await axiosClient.put(url, data)
    },

    // User Settings APIs (per-user preferences)
    getUserSettings: async (): Promise<any> => {
        const url = '/api/settings/user';
        return await axiosClient.get(url)
    },
    updateUserSettings: async (data: any): Promise<any> => {
        const url = '/api/settings/user';
        console.log('Making PUT request to:', url, 'with data:', data);
        const result = await axiosClient.put(url, data);
        console.log('PUT request result:', result);
        return result;
    },

    // 2FA APIs
    setup2FA: async (email?: string): Promise<any> => {
        const url = `/api/auth/2fa/setup${email ? `?email=${encodeURIComponent(email)}` : ''}`;
        return await axiosClient.get(url)
    },
    verify2FA: async (data: any): Promise<any> => {
        const url = '/api/auth/2fa/verify';
        return await axiosClient.post(url, data)
    },

    // Email Templates APIs
    getEmailTemplates: async (): Promise<any> => {
        const url = '/api/email-templates';
        return await axiosClient.get(url)
    },
    getEmailTemplate: async (id: number): Promise<any> => {
        const url = `/api/email-templates/${id}`;
        return await axiosClient.get(url)
    },
    updateEmailTemplate: async (id: number, data: any): Promise<any> => {
        const url = `/api/email-templates/${id}`;
        return await axiosClient.put(url, data)
    },

    uploadLogo: async (file: File): Promise<any> => {
        const formData = new FormData();
        formData.append('logo', file);
        const url = '/api/settings/upload-logo';
        return await axiosClient.post(url, formData, {
            headers: {
                'Content-Type': 'multipart/form-data',
            },
        });
    },

}
export default upstashService
