export interface User {
  id: number;
  name: string;
  email: string;
  role: string;
  is_active: boolean;
  activation_token?: string;
  subscription_status: string;
  subscription_expires_at?: string;
  free_usage_count?: number;
  signature?: string;
  initials?: string;
  two_factor_enabled: boolean;
  two_factor_secret?: string;
  created_at: string;
  oauth_tokens?: any[];
}

export interface ApiResponse<T> {
  success: boolean;
  status_code: number;
  message: string;
  data: T;
  error?: string;
}

export interface AuthData {
  token: string;
  user: User;
}

export interface Template {
  user_name: string;
  id: number;
  name: string;
  file_url: string;
  documents?: { 
    url: string;
    filename?: string;
    content_type?: string;
    size?: number;
  }[];
  created_at: string;
  user_id: number;
  slug: string;
  updated_at: string;
  fields?: TemplateField[];
}

export interface Position {
  x: number;
  y: number;
  width: number;
  height: number;
  page: number;
  default_value?: string;
}

export type FieldType = 'text' | 'signature' | 'initials' | 'date' | 'checkbox' | 'number' | 'radio' | 'multiple' | 'select' | 'cells' | 'image' | 'file';

export interface TwoFactorSetup {
  secret: string;
  qr_code_url: string;
}

export interface TwoFactorVerifyRequest {
  secret: string;
  code: string;
}

export interface GlobalSettings {
  id: number;
  company_name?: string;
  timezone?: string;
  locale?: string;
  force_2fa_with_authenticator_app: boolean;
  add_signature_id_to_the_documents: boolean;
  require_signing_reason: boolean;
  allow_typed_text_signatures: boolean;
  allow_to_resubmit_completed_forms: boolean;
  allow_to_decline_documents: boolean;
  remember_and_pre_fill_signatures: boolean;
  require_authentication_for_file_download_links: boolean;
  combine_completed_documents_and_audit_log: boolean;
  expirable_file_download_links: boolean;
  created_at: string;
  updated_at: string;
}

export interface TemplateField {
  id: number;
  name: string;
  field_type: FieldType | string;
  required: boolean;
  position: Position;
  display_order?: number;
  options?: any;
  partner?: string;
}

export interface NewTemplateField {
  name: string;
  field_type: FieldType | string;
  required: boolean;
  position: Position;
  display_order?: number;
  options?: string[];
  partner?: string;
}

export interface Signature {
  field_id: number;
  field_name: string;
  signature_value: string;
}

export interface Submitter {
  id: number;
  name: string;
  email: string;
  status: 'pending' | 'signed' | 'completed';
  token: string;
  template_id: number;
  user_id: number;
  signed_at: string | null;
  created_at: string;
  updated_at: string;
  bulk_signatures?: Signature[];
  template?: Template;
}

export interface TemplateFullInfo {
  template: Template;
  submitters: Submitter[];
  total_submitters: number;
  signatures?: any[];
}

export interface NewSubmitter {
    name: string;
    email: string;
}

export interface SubmissionSignaturesResponse {
  template_info: {
    id: number;
    name: string;
    slug: string;
    user_id: number;
    document: {
      filename: string;
      content_type: string;
      size: number;
      url: string;
    };
  };
  bulk_signatures: {
    field_id: number;
    field_info: TemplateField;
    field_name: string;
    signature_value: string;
    reason?: string;
  }[];
}