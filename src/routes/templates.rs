use axum::{
    extract::{Path, State, Query, OriginalUri},
    http::{StatusCode, header},
    response::{Json, Response, IntoResponse},
    routing::{get, post, put, delete},
    Router,
    body::Body,
    Extension,
    middleware,
};
use std::collections::HashMap;
use axum_extra::extract::Multipart;
use std::sync::Arc;
use tokio::sync::Mutex;
use serde_json;
use serde::Deserialize;
use utoipa::ToSchema;
use base64::{Engine as _, engine::general_purpose};
use aws_config;

fn get_content_type_from_filename(filename: &str) -> &'static str {
    let filename_lower = filename.to_lowercase();
    if filename_lower.ends_with(".pdf") {
        "application/pdf"
    } else if filename_lower.ends_with(".docx") {
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    } else if filename_lower.ends_with(".doc") {
        "application/msword"
    } else if filename_lower.ends_with(".txt") {
        "text/plain"
    } else if filename_lower.ends_with(".html") || filename_lower.ends_with(".htm") {
        "text/html"
    } else if filename_lower.ends_with(".jpg") || filename_lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if filename_lower.ends_with(".png") {
        "image/png"
    } else if filename_lower.ends_with(".gif") {
        "image/gif"
    } else if filename_lower.ends_with(".webp") {
        "image/webp"
    } else if filename_lower.ends_with(".bmp") {
        "image/bmp"
    } else if filename_lower.ends_with(".tiff") || filename_lower.ends_with(".tif") {
        "image/tiff"
    } else if filename_lower.ends_with(".json") {
        "application/json"
    } else if filename_lower.ends_with(".csv") {
        "text/csv"
    } else if filename_lower.ends_with(".xml") {
        "application/xml"
    } else if filename_lower.ends_with(".xlsx") {
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
    } else if filename_lower.ends_with(".xls") {
        "application/vnd.ms-excel"
    } else {
        "application/octet-stream"
    }
}

use crate::common::responses::ApiResponse;
use crate::models::template::{
    Template, UpdateTemplateRequest, CloneTemplateRequest,
    CreateTemplateFromHtmlRequest, MergeTemplatesRequest,
    TemplateField,
    CreateTemplateFieldRequest, UpdateTemplateFieldRequest,
    FileUploadResponse, CreateTemplateFromFileRequest, CreateTemplateRequest,
    TemplateFolder, CreateFolderRequest, UpdateFolderRequest,
    CreateTemplateFromGoogleDriveRequest
};
use crate::database::connection::DbPool;
use crate::database::models::{CreateTemplate, CreateTemplateField, CreateTemplateFolder};
use crate::database::queries::{TemplateQueries, TemplateFolderQueries, TemplateFieldQueries};
use crate::services::storage::StorageService;
use crate::common::jwt::auth_middleware;

use crate::routes::web::AppState;

#[utoipa::path(
    get,
    path = "/api/templates/{id}/full-info",
    params(
        ("id" = i64, Path, description = "Template ID")
    ),
    responses(
        (status = 200, description = "Template full information retrieved successfully", body = ApiResponse<serde_json::Value>),
        (status = 404, description = "Template not found", body = ApiResponse<serde_json::Value>),
        (status = 500, description = "Internal server error", body = ApiResponse<serde_json::Value>)
    ),
    security(("bearer_auth" = [])),
    tag = "templates"
)]
pub async fn get_template_full_info(
    State(state): State<AppState>,
    Path(template_id): Path<i64>,
    Extension(user_id): Extension<i64>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let pool = &state.lock().await.db_pool;

    // Verify user has permission to access this template
    match TemplateQueries::get_template_by_id(pool, template_id).await {
        Ok(Some(db_template)) => {
            // Get user role to check permissions
            // match crate::database::queries::UserQueries::get_user_by_id(pool, user_id).await {
            //     Ok(Some(user)) => {
            //         // Allow access if user is the owner OR if user has Editor/Admin/Member role
            //         let has_access = db_template.user_id == user_id || 
            //                        matches!(user.role, crate::models::role::Role::Editor | crate::models::role::Role::Admin | crate::models::role::Role::Member);
                    
            //         if !has_access {
            //             return ApiResponse::forbidden("Access denied".to_string());
            //         }
            //     }
            //     _ => return ApiResponse::forbidden("User not found".to_string()),
            // }

            // Convert template to API model with fields loaded
            let template = match convert_db_template_to_template_with_fields(db_template.clone(), pool).await {
                Ok(template) => template,
                Err(e) => return ApiResponse::internal_error(format!("Failed to load template fields: {}", e)),
            };

            // Get submitters for this template, filtered by user_id from JWT
            match crate::database::queries::SubmitterQueries::get_submitters_by_template(pool, template_id).await {
                Ok(db_submitters) => {
                    // Filter submitters to only show those created by the current user
                    let filtered_submitters: Vec<_> = db_submitters.into_iter()
                        .filter(|db_sub| db_sub.user_id == user_id)
                        .collect();

                    let submitters = filtered_submitters.into_iter().map(|db_sub| {
                        let reminder_config = db_sub.reminder_config.as_ref()
                            .and_then(|v| serde_json::from_value(v.clone()).ok());
                            
                        crate::models::submitter::Submitter {
                            id: Some(db_sub.id),
                            template_id: Some(db_sub.template_id),
                            user_id: Some(db_sub.user_id),
                            name: db_sub.name,
                            email: db_sub.email,
                            status: db_sub.status,
                            signed_at: db_sub.signed_at,
                            token: db_sub.token,
                            bulk_signatures: db_sub.bulk_signatures,
                            reminder_config,
                            last_reminder_sent_at: db_sub.last_reminder_sent_at,
                            reminder_count: db_sub.reminder_count,
                            created_at: db_sub.created_at,
                            updated_at: db_sub.updated_at,
                            template_name: None,
                            decline_reason: db_sub.decline_reason,
                            can_download: None,
                            global_settings: None,
                        }
                    }).collect::<Vec<_>>();

                    // Group submitters by creation time proximity (within 1 minute)
                    let mut time_groups: HashMap<String, Vec<crate::models::submitter::Submitter>> = HashMap::new();

                    for submitter in submitters {
                        // Group by minute timestamp (floor to nearest minute)
                        let timestamp = submitter.created_at.timestamp();
                        let minute_key = (timestamp / 60).to_string(); // Group by minute
                        time_groups.entry(minute_key).or_insert_with(Vec::new).push(submitter);
                    }

                    // Build signatures array
                    let mut signatures = Vec::new();

                    // Add signature groups
                    for (_key, parties) in time_groups {
                        let sig_type = if parties.len() > 1 { "bulk" } else { "single" };

                        let overall_status = if parties.iter().all(|s| s.status == "declined") {
                            "declined"
                        } else if parties.iter().all(|s| s.status == "completed" || s.status == "signed") {
                            "completed"
                        } else if parties.iter().all(|s| s.status == "completed" || s.status == "signed" || s.status == "declined") {
                            "mixed"
                        } else if parties.iter().any(|s| s.status == "completed" || s.status == "signed" || s.status == "declined") {
                            "partial"
                        } else {
                            "pending"
                        };

                        let signed_count = parties.iter().filter(|s| s.status == "completed" || s.status == "signed").count();
                        let declined_count = parties.iter().filter(|s| s.status == "declined").count();

                        signatures.push(serde_json::json!({
                            "type": sig_type,
                            "parties": parties,
                            "overall_status": overall_status,
                            "total_parties": parties.len(),
                            "signed_parties": signed_count,
                            "declined_parties": declined_count
                        }));
                    }

                    let data = serde_json::json!({
                        "template": template,
                        "signatures": signatures
                    });

                    ApiResponse::success(data, "Template full information retrieved successfully".to_string())
                }
                Err(e) => ApiResponse::internal_error(format!("Failed to get submitters: {}", e)),
            }
        }
        Ok(None) => ApiResponse::not_found("Template not found".to_string()),
        Err(e) => ApiResponse::internal_error(format!("Failed to retrieve template: {}", e)),
    }
}




/// Helper function to render signatures on PDF
fn render_signatures_on_pdf(
    pdf_bytes: &[u8],
    signatures: &[(String, String, f64, f64, f64, f64, i32)]
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use lopdf::{Document, Object, Stream, Dictionary, ObjectId};
    use lopdf::content::{Content, Operation};
    
    // Load the PDF document
    let mut doc = Document::load_mem(pdf_bytes)?;
    
    // Get page IDs - note: lopdf 0.32 uses (u32, u16) for ObjectId
    let pages: Vec<(u32, u16)> = doc.get_pages()
        .into_iter()
        .map(|(page_num, obj_id)| obj_id)
        .collect();
    
    for (_field_name, signature_value, x, y, width, height, page_num) in signatures {
        // Get the page ID for this signature
        if let Some(&(obj_num, gen_num)) = pages.get(*page_num as usize) {
            let page_id: ObjectId = (obj_num, gen_num);
            
            // Get page height to convert coordinates
            // PDF uses bottom-left origin, but frontend saves top-left origin
            let page_height = if let Ok(page_obj) = doc.get_object(page_id) {
                if let Ok(page_dict) = page_obj.as_dict() {
                    if let Ok(mediabox) = page_dict.get(b"MediaBox") {
                        if let Ok(mediabox_array) = mediabox.as_array() {
                            if mediabox_array.len() >= 4 {
                                // MediaBox format: [x1, y1, x2, y2]
                                // Try different types as lopdf Object can be Integer or Real
                                if let Ok(height_i64) = mediabox_array[3].as_i64() {
                                    height_i64 as f64
                                } else if let Ok(height_f32) = mediabox_array[3].as_f32() {
                                    height_f32 as f64
                                } else {
                                    792.0 // Default Letter height in points
                                }
                            } else {
                                792.0
                            }
                        } else {
                            792.0
                        }
                    } else {
                        792.0 // Default if MediaBox not found
                    }
                } else {
                    792.0
                }
            } else {
                792.0
            };
            
            // Convert Y coordinate from top-left to bottom-left
            // Frontend: y = distance from top
            // PDF: y = distance from bottom
            // We need to subtract the field height so text appears at the top of the field box
            let pdf_y = page_height - *y - *height;
            
            // Calculate font size based on field height (make it ~50% of height for better readability)
            let font_size = (*height * 0.5).max(6.0).min(16.0) as i64;
            
            println!("DEBUG: Field '{}' - web({}, {}) size({}, {}) -> PDF({}, {}) font={} page={}", 
                     _field_name, x, y, width, height, x, pdf_y, font_size, page_num);
            
            // Create text content stream
            let text_operations = vec![
                Operation::new("BT", vec![]), // Begin text
                Operation::new("Tf", vec![
                    Object::Name(b"Arial".to_vec()),
                    Object::Integer(font_size),
                ]), // Set font with calculated size
                Operation::new("Td", vec![
                    Object::Real(*x as f32),
                    Object::Real(pdf_y as f32),  // Use converted Y coordinate
                ]), // Set position
                Operation::new("Tj", vec![
                    Object::string_literal(signature_value.clone()),
                ]), // Show text
                Operation::new("ET", vec![]), // End text
            ];
            
            let content = Content { operations: text_operations };
            let content_data = content.encode()?;
            
            // Create a new content stream
            let stream = Stream::new(Dictionary::new(), content_data);
            let stream_id = doc.add_object(stream);
            
            // Get the page object and add stream to it
            if let Ok(page_obj) = doc.get_object_mut(page_id) {
                if let Ok(page_dict) = page_obj.as_dict_mut() {
                    // Add to page's content array
                    if let Ok(contents_obj) = page_dict.get_mut(b"Contents") {
                        match contents_obj {
                            Object::Reference(ref_id) => {
                                // Single content stream - convert to array
                                let old_ref = *ref_id;
                                *contents_obj = Object::Array(vec![
                                    Object::Reference(old_ref),
                                    Object::Reference(stream_id),
                                ]);
                            }
                            Object::Array(ref mut arr) => {
                                // Multiple content streams - append
                                arr.push(Object::Reference(stream_id));
                            }
                            _ => {}
                        }
                    } else {
                        // No contents yet - create new
                        page_dict.set("Contents", Object::Reference(stream_id));
                    }
                }
            }
        }
    }
    
    // Save modified PDF to bytes
    let mut buffer = Vec::new();
    doc.save_to(&mut buffer)?;
    
    Ok(buffer)
}

pub fn create_template_router() -> Router<AppState> {
    // Public routes (no authentication required)
    let public_routes = Router::new()
        .route("/files/upload/public", post(upload_file_public))
        .route("/files/delete/public", delete(delete_file_public))
        .route("/files/preview/*key", get(preview_file))
        .route("/files/previews/*key", get(download_file_public)) // Public download for preview images
        .route("/files/*key", get(download_file)); // Public download for all files

    // Authenticated routes
    let auth_routes = Router::new()
        // Template folders routes
        .route("/folders", get(get_folders))
        .route("/folders", post(create_folder))
        .route("/folders/:id", get(get_folder))
        .route("/folders/:id", put(update_folder))
        .route("/folders/:id", delete(delete_folder))
        .route("/folders/:id/templates", get(get_folder_templates))
        .route("/templates/:template_id/move/:folder_id", put(move_template_to_folder))
        // Template routes
        .route("/templates", get(get_templates))
        .route("/templates", post(create_template))
        .route("/templates/:id", get(get_template))
        .route("/templates/:id/full-info", get(get_template_full_info))
        .route("/templates/:id", put(update_template))
        .route("/templates/:id", delete(delete_template))
        .route("/templates/:id/clone", post(clone_template))
        .route("/templates/html", post(create_template_from_html))
        .route("/templates/pdf", post(create_template_from_pdf))
        .route("/templates/docx", post(create_template_from_docx))
        .route("/templates/from-file", post(create_template_from_file))
        .route("/templates/google_drive_documents", post(create_template_from_google_drive))
        .route("/templates/merge", post(merge_templates))
        // Template Fields routes
        .route("/templates/:template_id/fields", get(get_template_fields))
        .route("/templates/:template_id/fields", post(create_template_field))
        .route("/templates/:template_id/fields/upload", post(upload_template_field_file))
        .route("/templates/:template_id/fields/:field_id", put(update_template_field))
        .route("/templates/:template_id/fields/:field_id", delete(delete_template_field))
        // File upload must come before wildcard route
        .route("/files/upload", post(upload_file))
        .layer(middleware::from_fn(auth_middleware));

    // Merge public and authenticated routes
    public_routes.merge(auth_routes)
}

// ===== TEMPLATE FOLDER ENDPOINTS =====

#[utoipa::path(
    get,
    path = "/api/folders",
    responses(
        (status = 200, description = "List all template folders with hierarchy", body = ApiResponse<Vec<TemplateFolder>>),
        (status = 500, description = "Internal server error", body = ApiResponse<Vec<TemplateFolder>>)
    ),
    security(("bearer_auth" = [])),
    tag = "folders"
)]
pub async fn get_folders(
    State(state): State<AppState>,
    Extension(user_id): Extension<i64>,
) -> (StatusCode, Json<ApiResponse<Vec<TemplateFolder>>>) {
    let pool = &state.lock().await.db_pool;

    match TemplateFolderQueries::get_team_folders(pool, user_id).await {
        Ok(db_folders) => {
            let mut folders = Vec::new();
            
            // Build hierarchy with proper recursion
            fn build_folder_tree(
                folder_id: i64,
                all_folders: &Vec<crate::database::models::DbTemplateFolder>
            ) -> TemplateFolder {
                let db_folder = all_folders.iter().find(|f| f.id == folder_id).unwrap();
                let mut folder = TemplateFolder {
                    id: db_folder.id,
                    name: db_folder.name.clone(),
                    user_id: db_folder.user_id,
                    parent_folder_id: db_folder.parent_folder_id,
                    created_at: db_folder.created_at,
                    updated_at: db_folder.updated_at,
                    children: Some(Vec::new()),
                    templates: None,
                };

                // Find and build all children
                let children: Vec<TemplateFolder> = all_folders.iter()
                    .filter(|f| f.parent_folder_id == Some(folder_id))
                    .map(|child_db| build_folder_tree(child_db.id, all_folders))
                    .collect();

                if let Some(ref mut children_vec) = folder.children {
                    *children_vec = children;
                }

                folder
            }

            // Build root folders with their full tree
            for db_folder in &db_folders {
                if db_folder.parent_folder_id.is_none() {
                    let root_folder = build_folder_tree(db_folder.id, &db_folders);
                    folders.push(root_folder);
                }
            }

            ApiResponse::success(folders, "Folders retrieved successfully".to_string())
        }
        Err(e) => ApiResponse::internal_error(format!("Failed to retrieve folders: {}", e)),
    }
}

#[utoipa::path(
    post,
    path = "/api/folders",
    request_body = CreateFolderRequest,
    responses(
        (status = 201, description = "Folder created successfully", body = ApiResponse<TemplateFolder>),
        (status = 500, description = "Internal server error", body = ApiResponse<TemplateFolder>)
    ),
    security(("bearer_auth" = [])),
    tag = "folders"
)]
pub async fn create_folder(
    State(state): State<AppState>,
    Extension(user_id): Extension<i64>,
    Json(payload): Json<CreateFolderRequest>,
) -> (StatusCode, Json<ApiResponse<TemplateFolder>>) {
    let pool = &state.lock().await.db_pool;

    // Helper function to calculate folder depth
    fn calculate_depth(folders: &[crate::database::models::DbTemplateFolder], folder_id: i64) -> i32 {
        let mut depth = 1;
        let mut current_id = folder_id;
        loop {
            if let Some(folder) = folders.iter().find(|f| f.id == current_id) {
                if let Some(parent_id) = folder.parent_folder_id {
                    depth += 1;
                    current_id = parent_id;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        depth
    }

    // Check folder creation constraints
    if let Some(parent_id) = payload.parent_folder_id {
        match TemplateFolderQueries::get_folders_by_user(pool, user_id).await {
            Ok(all_folders) => {
                // Check if parent exists
                if !all_folders.iter().any(|f| f.id == parent_id) {
                    return ApiResponse::not_found("Parent folder not found".to_string());
                }

                // Calculate parent depth
                let parent_depth = calculate_depth(&all_folders, parent_id);
                if parent_depth >= 2 {
                    return ApiResponse::bad_request("Cannot create folder: maximum 2 levels allowed".to_string());
                }
                
                // Rule 2: If parent already has children, update existing child instead
                let has_children = all_folders.iter().any(|f| f.parent_folder_id == Some(parent_id));
                if has_children {
                    if let Some(existing_child) = all_folders.iter().find(|f| f.parent_folder_id == Some(parent_id)) {
                        // Determine the name to use for updating
                        let template_name_holder;
                        let update_name = if let Some(name) = &payload.name {
                            Some(name.as_str())
                        } else if let Some(template_id) = payload.template_id {
                            // Get template name when name is not provided
                            match TemplateQueries::get_template_by_id(pool, template_id).await {
                                Ok(Some(template)) if template.user_id == user_id => {
                                    template_name_holder = template.name;
                                    Some(template_name_holder.as_str())
                                }
                                Ok(Some(_)) => return ApiResponse::forbidden("Access denied to template".to_string()),
                                Ok(None) => return ApiResponse::not_found("Template not found".to_string()),
                                Err(e) => return ApiResponse::internal_error(format!("Failed to get template: {}", e)),
                            }
                        } else {
                            return ApiResponse::bad_request("Either name or template_id must be provided".to_string());
                        };

                        match TemplateFolderQueries::update_folder(
                            pool, 
                            existing_child.id, 
                            update_name,
                            Some(existing_child.parent_folder_id)
                        ).await {
                            Ok(Some(updated_folder)) => {
                                let folder = TemplateFolder {
                                    id: updated_folder.id,
                                    name: updated_folder.name,
                                    user_id: updated_folder.user_id,
                                    parent_folder_id: updated_folder.parent_folder_id,
                                    created_at: updated_folder.created_at,
                                    updated_at: updated_folder.updated_at,
                                    children: None,
                                    templates: None,
                                };
                                return ApiResponse::success(folder, "Folder name updated (only 1 child per parent allowed)".to_string());
                            }
                            Ok(None) => return ApiResponse::not_found("Child folder not found".to_string()),
                            Err(e) => return ApiResponse::internal_error(format!("Failed to update folder: {}", e)),
                        }
                    }
                }
            }
            Err(e) => return ApiResponse::internal_error(format!("Failed to check folder hierarchy: {}", e)),
        }
    }

    // Create new folder (either root or first child)
    // Determine folder name
    let folder_name = if let Some(name) = payload.name {
        name
    } else if let Some(template_id) = payload.template_id {
        // Get template name when name is not provided
        match TemplateQueries::get_template_by_id(pool, template_id).await {
            Ok(Some(template)) => {
                // Check user permission to access template
                match crate::database::queries::UserQueries::get_user_by_id(pool, user_id).await {
                    Ok(Some(user)) => {
                        let has_access = template.user_id == user_id || 
                                       matches!(user.role, crate::models::role::Role::Editor | crate::models::role::Role::Admin | crate::models::role::Role::Member);
                        
                        if !has_access {
                            return ApiResponse::forbidden("Access denied to template".to_string());
                        }
                    }
                    _ => return ApiResponse::forbidden("User not found".to_string()),
                }
                template.name
            }
            Ok(None) => return ApiResponse::not_found("Template not found".to_string()),
            Err(e) => return ApiResponse::internal_error(format!("Failed to get template: {}", e)),
        }
    } else {
        return ApiResponse::bad_request("Either name or template_id must be provided".to_string());
    };

    // Get user's account_id
    let account_id = match crate::database::queries::UserQueries::get_user_by_id(pool, user_id).await {
        Ok(Some(user)) => user.account_id,
        Ok(None) => return ApiResponse::not_found("User not found".to_string()),
        Err(e) => return ApiResponse::internal_error(format!("Failed to get user: {}", e)),
    };

    let create_folder = CreateTemplateFolder {
        name: folder_name,
        user_id,
        account_id,
        parent_folder_id: payload.parent_folder_id,
    };

    match TemplateFolderQueries::create_folder(pool, create_folder).await {
        Ok(db_folder) => {
            let folder_id = db_folder.id;

            // Move template to the new folder if template_id is provided
            if let Some(template_id) = payload.template_id {
                // Template access already verified above, move it
                match TemplateQueries::get_template_by_id(pool, template_id).await {
                    Ok(Some(template)) => {
                        // Move template to the new folder
                        if let Err(e) = TemplateFolderQueries::move_template_to_folder(pool, template_id, Some(folder_id), template.user_id).await {
                            // Log error but don't fail the folder creation
                            eprintln!("Failed to move template {} to folder {}: {}", template_id, folder_id, e);
                        }
                    }
                    _ => {
                        // Template access was already checked, this shouldn't happen
                        eprintln!("Template {} not found during folder creation", template_id);
                    }
                }
            }

            let folder = TemplateFolder {
                id: folder_id,
                name: db_folder.name,
                user_id: db_folder.user_id,
                parent_folder_id: db_folder.parent_folder_id,
                created_at: db_folder.created_at,
                updated_at: db_folder.updated_at,
                children: None,
                templates: None,
            };
            ApiResponse::created(folder, "Folder created successfully".to_string())
        }
        Err(e) => ApiResponse::internal_error(format!("Failed to create folder: {}", e)),
    }
}

#[utoipa::path(
    get,
    path = "/api/folders/{id}",
    params(
        ("id" = i64, Path, description = "Folder ID")
    ),
    responses(
        (status = 200, description = "Folder found with templates", body = ApiResponse<TemplateFolder>),
        (status = 404, description = "Folder not found", body = ApiResponse<TemplateFolder>),
        (status = 500, description = "Internal server error", body = ApiResponse<TemplateFolder>)
    ),
    security(("bearer_auth" = [])),
    tag = "folders"
)]
pub async fn get_folder(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Extension(user_id): Extension<i64>,
) -> (StatusCode, Json<ApiResponse<TemplateFolder>>) {
    let pool = &state.lock().await.db_pool;

    match TemplateFolderQueries::get_folder_by_id(pool, id).await {
        Ok(Some(db_folder)) => {
            // Get user role to check permissions
            match crate::database::queries::UserQueries::get_user_by_id(pool, user_id).await {
                Ok(Some(user)) => {
                    // Allow access if user is the owner OR if user has Editor/Admin/Member role
                    let has_access = db_folder.user_id == user_id || 
                                   matches!(user.role, crate::models::role::Role::Editor | crate::models::role::Role::Admin | crate::models::role::Role::Member);
                    
                    if !has_access {
                        return ApiResponse::not_found("Folder not found".to_string());
                    }
                }
                _ => return ApiResponse::forbidden("User not found".to_string()),
            }

            // Get templates in this folder
            match TemplateFolderQueries::get_templates_in_folder(pool, db_folder.user_id, Some(id)).await {
                Ok(db_templates) => {
                    let templates = db_templates.into_iter()
                        .map(|db_template| convert_db_template_to_template_without_fields(db_template))
                        .collect();

                    let folder = TemplateFolder {
                        id: db_folder.id,
                        name: db_folder.name,
                        user_id: db_folder.user_id,
                        parent_folder_id: db_folder.parent_folder_id,
                        created_at: db_folder.created_at,
                        updated_at: db_folder.updated_at,
                        children: None,
                        templates: Some(templates),
                    };
                    ApiResponse::success(folder, "Folder retrieved successfully".to_string())
                }
                Err(e) => ApiResponse::internal_error(format!("Failed to get folder templates: {}", e)),
            }
        }
        Ok(None) => ApiResponse::not_found("Folder not found".to_string()),
        Err(e) => ApiResponse::internal_error(format!("Failed to retrieve folder: {}", e)),
    }
}

#[utoipa::path(
    put,
    path = "/api/folders/{id}",
    params(
        ("id" = i64, Path, description = "Folder ID")
    ),
    request_body = UpdateFolderRequest,
    responses(
        (status = 200, description = "Folder updated successfully", body = ApiResponse<TemplateFolder>),
        (status = 404, description = "Folder not found", body = ApiResponse<TemplateFolder>),
        (status = 500, description = "Internal server error", body = ApiResponse<TemplateFolder>)
    ),
    security(("bearer_auth" = [])),
    tag = "folders"
)]
pub async fn update_folder(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Extension(user_id): Extension<i64>,
    Json(payload): Json<UpdateFolderRequest>,
) -> (StatusCode, Json<ApiResponse<TemplateFolder>>) {
    let pool = &state.lock().await.db_pool;

    // First verify user has permission to access this folder
    match TemplateFolderQueries::get_folder_by_id(pool, id).await {
        Ok(Some(db_folder)) => {
            // Get user role to check permissions
            match crate::database::queries::UserQueries::get_user_by_id(pool, user_id).await {
                Ok(Some(user)) => {
                    // Allow access if user is the owner OR if user has Editor/Admin role (Members have read-only access to others' folders)
                    let has_access = db_folder.user_id == user_id || 
                                   matches!(user.role, crate::models::role::Role::Editor | crate::models::role::Role::Admin);
                    
                    if !has_access {
                        return ApiResponse::forbidden("Access denied".to_string());
                    }
                }
                _ => return ApiResponse::forbidden("User not found".to_string()),
            }

            match TemplateFolderQueries::update_folder(
                pool, 
                id, 
                payload.name.as_deref(),
                None
            ).await {
                Ok(Some(db_folder)) => {
                    let folder = TemplateFolder {
                        id: db_folder.id,
                        name: db_folder.name,
                        user_id: db_folder.user_id,
                        parent_folder_id: db_folder.parent_folder_id,
                        created_at: db_folder.created_at,
                        updated_at: db_folder.updated_at,
                        children: None,
                        templates: None,
                    };
                    ApiResponse::success(folder, "Folder updated successfully".to_string())
                }
                Ok(None) => ApiResponse::not_found("Folder not found".to_string()),
                Err(e) => ApiResponse::internal_error(format!("Failed to update folder: {}", e)),
            }
        }
        Ok(None) => ApiResponse::not_found("Folder not found".to_string()),
        Err(e) => ApiResponse::internal_error(format!("Failed to retrieve folder: {}", e)),
    }
}

#[utoipa::path(
    delete,
    path = "/api/folders/{id}",
    params(
        ("id" = i64, Path, description = "Folder ID")
    ),
    responses(
        (status = 200, description = "Folder deleted successfully", body = ApiResponse<serde_json::Value>),
        (status = 404, description = "Folder not found", body = ApiResponse<serde_json::Value>),
        (status = 500, description = "Internal server error", body = ApiResponse<serde_json::Value>)
    ),
    security(("bearer_auth" = [])),
    tag = "folders"
)]
pub async fn delete_folder(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Extension(user_id): Extension<i64>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let pool = &state.lock().await.db_pool;

    // First verify user has permission to access this folder
    match TemplateFolderQueries::get_folder_by_id(pool, id).await {
        Ok(Some(db_folder)) => {
            // Get user role to check permissions
            match crate::database::queries::UserQueries::get_user_by_id(pool, user_id).await {
                Ok(Some(user)) => {
                    // Allow access if user is the owner OR if user has Editor/Admin role (Members have read-only access to others' folders)
                    let has_access = db_folder.user_id == user_id || 
                                   matches!(user.role, crate::models::role::Role::Editor | crate::models::role::Role::Admin);
                    
                    if !has_access {
                        return ApiResponse::forbidden("Access denied".to_string());
                    }
                }
                _ => return ApiResponse::forbidden("User not found".to_string()),
            }

            match TemplateFolderQueries::delete_folder(pool, id, db_folder.user_id).await {
                Ok(true) => ApiResponse::success(serde_json::json!({"deleted": true}), "Folder deleted successfully".to_string()),
                Ok(false) => ApiResponse::not_found("Folder not found".to_string()),
                Err(e) => ApiResponse::internal_error(format!("Failed to delete folder: {}", e)),
            }
        }
        Ok(None) => ApiResponse::not_found("Folder not found".to_string()),
        Err(e) => ApiResponse::internal_error(format!("Failed to retrieve folder: {}", e)),
    }
}

#[utoipa::path(
    get,
    path = "/api/folders/{id}/templates",
    params(
        ("id" = i64, Path, description = "Folder ID")
    ),
    responses(
        (status = 200, description = "Templates in folder retrieved successfully", body = ApiResponse<Vec<Template>>),
        (status = 404, description = "Folder not found", body = ApiResponse<Vec<Template>>),
        (status = 500, description = "Internal server error", body = ApiResponse<Vec<Template>>)
    ),
    security(("bearer_auth" = [])),
    tag = "folders"
)]
pub async fn get_folder_templates(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Extension(user_id): Extension<i64>,
) -> (StatusCode, Json<ApiResponse<Vec<Template>>>) {
    let pool = &state.lock().await.db_pool;

    // Verify folder exists and user has permission
    match TemplateFolderQueries::get_folder_by_id(pool, id).await {
        Ok(Some(db_folder)) => {
            // Get user role to check permissions
            match crate::database::queries::UserQueries::get_user_by_id(pool, user_id).await {
                Ok(Some(user)) => {
                    // Allow access if user is the owner OR if user has Editor/Admin/Member role
                    let has_access = db_folder.user_id == user_id || 
                                   matches!(user.role, crate::models::role::Role::Editor | crate::models::role::Role::Admin | crate::models::role::Role::Member);
                    
                    if !has_access {
                        return ApiResponse::not_found("Folder not found".to_string());
                    }
                }
                _ => return ApiResponse::forbidden("User not found".to_string()),
            }

            // Get templates in this folder
            match TemplateFolderQueries::get_team_templates_in_folder(pool, user_id, id).await {
                Ok(db_templates) => {
                    let mut templates = Vec::new();
                    for db_template in db_templates {
                        let mut template = convert_db_template_to_template_without_fields(db_template.clone());
                        
                        // Get username for the template owner
                        match crate::database::queries::UserQueries::get_user_by_id(pool, db_template.user_id).await {
                            Ok(Some(owner)) => {
                                // Use name if available, fallback to email
                                let display_name = if !owner.name.is_empty() {
                                    owner.name
                                } else {
                                    owner.email
                                };
                                template.user_name = Some(display_name);
                            }
                            Ok(None) => {
                                eprintln!("⚠️ User {} not found for template {}", db_template.user_id, db_template.id);
                            }
                            Err(e) => {
                                eprintln!("❌ Error getting user {} for template {}: {}", db_template.user_id, db_template.id, e);
                            }
                        }
                        
                        templates.push(template);
                    }
                    ApiResponse::success(templates, "Templates retrieved successfully".to_string())
                }
                Err(e) => ApiResponse::internal_error(format!("Failed to get templates: {}", e)),
            }
        }
        Ok(None) => ApiResponse::not_found("Folder not found".to_string()),
        Err(e) => ApiResponse::internal_error(format!("Failed to verify folder: {}", e)),
    }
}

#[utoipa::path(
    put,
    path = "/api/templates/{template_id}/move/{folder_id}",
    params(
        ("template_id" = i64, Path, description = "Template ID"),
        ("folder_id" = i64, Path, description = "Destination Folder ID (use 0 for root)")
    ),
    responses(
        (status = 200, description = "Template moved successfully", body = ApiResponse<serde_json::Value>),
        (status = 404, description = "Template or folder not found", body = ApiResponse<serde_json::Value>),
        (status = 500, description = "Internal server error", body = ApiResponse<serde_json::Value>)
    ),
    security(("bearer_auth" = [])),
    tag = "folders"
)]
pub async fn move_template_to_folder(
    State(state): State<AppState>,
    Path((template_id, folder_id)): Path<(i64, i64)>,
    Extension(user_id): Extension<i64>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let pool = &state.lock().await.db_pool;

    let target_folder_id = if folder_id == 0 { None } else { Some(folder_id) };

    // Get user role to check permissions
    let user = match crate::database::queries::UserQueries::get_user_by_id(pool, user_id).await {
        Ok(Some(user)) => user,
        _ => return ApiResponse::forbidden("User not found".to_string()),
    };

    // Verify template access
    match TemplateQueries::get_template_by_id(pool, template_id).await {
        Ok(Some(template)) => {
            // Allow access if user is the owner OR if user has Editor/Admin role (Members have read-only access to others' templates)
            let has_template_access = template.user_id == user_id || 
                                   matches!(user.role, crate::models::role::Role::Editor | crate::models::role::Role::Admin);
            
            if !has_template_access {
                return ApiResponse::forbidden("Access denied: You do not have permission to move this template".to_string());
            }

            // If moving to a folder (not root), verify folder access
            if let Some(fid) = target_folder_id {
                match TemplateFolderQueries::get_folder_by_id(pool, fid).await {
                    Ok(Some(db_folder)) => {
                        // Check folder access
                        let has_folder_access = db_folder.user_id == user_id || 
                                              matches!(user.role, crate::models::role::Role::Editor | crate::models::role::Role::Admin | crate::models::role::Role::Member);
                        
                        if !has_folder_access {
                            return ApiResponse::forbidden("Access denied: You do not have permission to access this folder".to_string());
                        }
                    }
                    Ok(None) => return ApiResponse::not_found("Destination folder not found".to_string()),
                    Err(e) => return ApiResponse::internal_error(format!("Failed to verify folder: {}", e)),
                }
            }

            match TemplateFolderQueries::move_template_to_folder(pool, template_id, target_folder_id, template.user_id).await {
                Ok(true) => ApiResponse::success(serde_json::json!({"moved": true}), "Template moved successfully".to_string()),
                Ok(false) => ApiResponse::not_found("Template not found".to_string()),
                Err(e) => ApiResponse::internal_error(format!("Failed to move template: {}", e)),
            }
        }
        Ok(None) => ApiResponse::not_found("Template not found".to_string()),
        Err(e) => ApiResponse::internal_error(format!("Failed to verify template: {}", e)),
    }
}

// ===== TEMPLATE ENDPOINTS =====

#[utoipa::path(
    get,
    path = "/api/templates",
    responses(
        (status = 200, description = "List all templates", body = ApiResponse<Vec<Template>>),
        (status = 500, description = "Internal server error", body = ApiResponse<Vec<Template>>)
    ),
    security(("bearer_auth" = [])),
    tag = "templates"
)]
pub async fn get_templates(
    State(state): State<AppState>,
    Extension(user_id): Extension<i64>,
) -> (StatusCode, Json<ApiResponse<Vec<Template>>>) {
    let pool = &state.lock().await.db_pool;

    match TemplateQueries::get_team_templates(pool, user_id).await {
        Ok(db_templates) => {
            let mut templates = Vec::new();
            for db_template in db_templates {
                // Get user name for this template's owner
                let user_name = match crate::database::queries::UserQueries::get_user_by_id(pool, db_template.user_id).await {
                    Ok(Some(user)) => {
                        // Use name if available, fallback to email
                        let display_name = if !user.name.is_empty() {
                            user.name.clone()
                        } else {
                            user.email.clone()
                        };
                        Some(display_name)
                    }
                    Ok(None) => {
                        eprintln!("⚠️ User {} not found for template {}", db_template.user_id, db_template.id);
                        None
                    }
                    Err(e) => {
                        eprintln!("❌ Error getting user {} for template {}: {}", db_template.user_id, db_template.id, e);
                        None
                    }
                };
                
                let mut template = convert_db_template_to_template_without_fields(db_template);
                template.user_name = user_name;
                templates.push(template);
            }
            ApiResponse::success(templates, "Templates retrieved successfully".to_string())
        }
        Err(e) => ApiResponse::internal_error(format!("Failed to retrieve templates: {}", e)),
    }
}

#[utoipa::path(
    get,
    path = "/api/templates/{id}",
    params(
        ("id" = i64, Path, description = "Template ID")
    ),
    responses(
        (status = 200, description = "Template found", body = ApiResponse<Template>),
        (status = 404, description = "Template not found", body = ApiResponse<Template>),
        (status = 500, description = "Internal server error", body = ApiResponse<Template>)
    ),
    security(("bearer_auth" = [])),
    tag = "templates"
)]
pub async fn get_template(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Extension(user_id): Extension<i64>,
) -> (StatusCode, Json<ApiResponse<Template>>) {
    let pool = &state.lock().await.db_pool;

    match TemplateQueries::get_template_by_id(pool, id).await {
        Ok(Some(db_template)) => {
            // Get user role to check permissions
            match crate::database::queries::UserQueries::get_user_by_id(pool, user_id).await {
                Ok(Some(user)) => {
                    // Allow access if user is the owner OR if user has Editor/Admin/Member role
                    let has_access = db_template.user_id == user_id || 
                                   matches!(user.role, crate::models::role::Role::Editor | crate::models::role::Role::Admin | crate::models::role::Role::Member);
                    
                    if !has_access {
                        return ApiResponse::not_found("Template not found".to_string());
                    }
                }
                _ => return ApiResponse::forbidden("User not found".to_string()),
            }
            match convert_db_template_to_template_with_fields(db_template, pool).await {
                Ok(mut template) => {
                    // Get user name for this template's owner
                    let user_name = match crate::database::queries::UserQueries::get_user_by_id(pool, template.user_id).await {
                        Ok(Some(user)) => {
                            // Use name if available, fallback to email
                            let display_name = if !user.name.is_empty() {
                                user.name.clone()
                            } else {
                                user.email.clone()
                            };
                            Some(display_name)
                        }
                        Ok(None) => {
                            eprintln!("⚠️ User {} not found for template {}", template.user_id, template.id);
                            None
                        }
                        Err(e) => {
                            eprintln!("❌ Error getting user {} for template {}: {}", template.user_id, template.id, e);
                            None
                        }
                    };
                    template.user_name = user_name;
                    ApiResponse::success(template, "Template retrieved successfully".to_string())
                }
                Err(e) => ApiResponse::internal_error(format!("Failed to load template fields: {}", e)),
            }
        }
        Ok(None) => ApiResponse::not_found("Template not found".to_string()),
        Err(e) => ApiResponse::internal_error(format!("Failed to retrieve template: {}", e)),
    }
}

#[utoipa::path(
    put,
    path = "/api/templates/{id}",
    params(
        ("id" = i64, Path, description = "Template ID")
    ),
    request_body = UpdateTemplateRequest,
    responses(
        (status = 200, description = "Template updated successfully", body = ApiResponse<Template>),
        (status = 404, description = "Template not found", body = ApiResponse<Template>),
        (status = 500, description = "Internal server error", body = ApiResponse<Template>)
    ),
    security(("bearer_auth" = [])),
    tag = "templates"
)]
pub async fn update_template(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Extension(user_id): Extension<i64>,
    Json(payload): Json<UpdateTemplateRequest>,
) -> (StatusCode, Json<ApiResponse<Template>>) {
    let pool = &state.lock().await.db_pool;

    // First verify user has permission to access this template
    match TemplateQueries::get_template_by_id(pool, id).await {
        Ok(Some(db_template)) => {
            // Get user role to check permissions
            match crate::database::queries::UserQueries::get_user_by_id(pool, user_id).await {
                Ok(Some(user)) => {
                    // Allow access if user is the owner OR if user has Editor/Admin role (Members have read-only access to others' templates)
                    let has_access = db_template.user_id == user_id || 
                                   matches!(user.role, crate::models::role::Role::Editor | crate::models::role::Role::Admin);
                    
                    if !has_access {
                        return ApiResponse::forbidden("Access denied".to_string());
                    }
                }
                _ => return ApiResponse::forbidden("User not found".to_string()),
            }

            // Update template (fields are managed separately via template_fields endpoints)
            match TemplateQueries::update_template(pool, id, payload.name.as_deref()).await {
                Ok(Some(db_template)) => {
                    match convert_db_template_to_template_with_fields(db_template, pool).await {
                        Ok(template) => ApiResponse::success(template, "Template updated successfully".to_string()),
                        Err(e) => ApiResponse::internal_error(format!("Failed to load template fields: {}", e)),
                    }
                }
                Ok(None) => ApiResponse::not_found("Template not found".to_string()),
                Err(e) => ApiResponse::internal_error(format!("Failed to update template: {}", e)),
            }
        }
        Ok(None) => ApiResponse::not_found("Template not found".to_string()),
        Err(e) => ApiResponse::internal_error(format!("Failed to retrieve template: {}", e)),
    }
}

#[utoipa::path(
    delete,
    path = "/api/templates/{id}",
    params(
        ("id" = i64, Path, description = "Template ID")
    ),
    responses(
        (status = 200, description = "Template deleted successfully", body = ApiResponse<String>),
        (status = 404, description = "Template not found", body = ApiResponse<String>),
        (status = 500, description = "Internal server error", body = ApiResponse<String>)
    ),
    security(("bearer_auth" = [])),
    tag = "templates"
)]
pub async fn delete_template(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Extension(user_id): Extension<i64>,
) -> (StatusCode, Json<ApiResponse<String>>) {
    let pool = &state.lock().await.db_pool;

    // First verify user has permission to access this template
    match TemplateQueries::get_template_by_id(pool, id).await {
        Ok(Some(db_template)) => {
            // Get user role to check permissions
            match crate::database::queries::UserQueries::get_user_by_id(pool, user_id).await {
                Ok(Some(user)) => {
                    // Allow access if user is the owner OR if user has Editor/Admin role (Members have read-only access to others' templates)
                    let has_access = db_template.user_id == user_id || 
                                   matches!(user.role, crate::models::role::Role::Editor | crate::models::role::Role::Admin);
                    
                    if !has_access {
                        return ApiResponse::forbidden("Access denied: You do not have permission to access this folder".to_string());
                    }
                }
                _ => return ApiResponse::forbidden("User not found".to_string()),
            }

            // Initialize storage service to delete files
            let storage = match StorageService::new().await {
                Ok(storage) => storage,
                Err(e) => {
                    eprintln!("Warning: Failed to initialize storage for cleanup: {}", e);
                    return ApiResponse::internal_error(format!("Failed to initialize storage: {}", e));
                }
            };

            // Delete files from S3 if documents exist
            if let Some(documents) = &db_template.documents {
                if let Some(docs_array) = documents.as_array() {
                    for doc in docs_array {
                        if let Some(url) = doc.get("url").and_then(|u| u.as_str()) {
                            eprintln!("🗑️ Deleting template document from S3: {}", url);
                            if let Err(e) = storage.delete_file(url).await {
                                eprintln!("⚠️ Warning: Failed to delete file '{}' from S3: {}", url, e);
                            } else {
                                eprintln!("✅ Successfully deleted file from S3: {}", url);
                            }
                        }
                    }
                }
            }

            // Delete template from database
            match TemplateQueries::delete_template(pool, id).await {
                Ok(true) => ApiResponse::success("Template deleted successfully".to_string(), "Template deleted successfully".to_string()),
                Ok(false) => ApiResponse::not_found("Template not found".to_string()),
                Err(e) => ApiResponse::internal_error(format!("Failed to delete template: {}", e)),
            }
        }
        Ok(None) => ApiResponse::not_found("Template not found".to_string()),
        Err(e) => ApiResponse::internal_error(format!("Failed to retrieve template: {}", e)),
    }
}

#[utoipa::path(
    post,
    path = "/api/templates/{id}/clone",
    params(
        ("id" = i64, Path, description = "Template ID to clone")
    ),
    request_body = CloneTemplateRequest,
    responses(
        (status = 201, description = "Template cloned successfully", body = ApiResponse<Template>),
        (status = 404, description = "Original template not found", body = ApiResponse<Template>),
        (status = 500, description = "Internal server error", body = ApiResponse<Template>)
    ),
    security(("bearer_auth" = [])),
    tag = "templates"
)]
pub async fn clone_template(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Extension(user_id): Extension<i64>,
    Json(payload): Json<CloneTemplateRequest>,
) -> (StatusCode, Json<ApiResponse<Template>>) {
    let pool = &state.lock().await.db_pool;

    // First get the original template to get its name
    match TemplateQueries::get_template_by_id(pool, id).await {
        Ok(Some(original_template)) => {
            // Get user role to check permissions
            match crate::database::queries::UserQueries::get_user_by_id(pool, user_id).await {
                Ok(Some(user)) => {
                    // Allow access if user is the owner OR if user has Editor/Admin/Member role
                    let has_access = original_template.user_id == user_id || 
                                   matches!(user.role, crate::models::role::Role::Editor | crate::models::role::Role::Admin | crate::models::role::Role::Member);
                    
                    if !has_access {
                        return ApiResponse::not_found("Template not found".to_string());
                    }
                }
                _ => return ApiResponse::forbidden("User not found".to_string()),
            }

            // Generate new name: original name + " (Clone)"
            let new_name = format!("{} (Clone)", original_template.name);
            
            // Generate a unique slug for the cloned template
            let slug = format!("{}-clone-{}", new_name.to_lowercase().replace(" ", "-").replace("(", "").replace(")", ""), chrono::Utc::now().timestamp());

            match TemplateQueries::clone_template(pool, id, user_id, &new_name, &slug).await {
                Ok(Some(db_template)) => {
                    // Clone template fields from original template
                    use crate::database::queries::TemplateFieldQueries;
                    let _ = TemplateFieldQueries::clone_template_fields(pool, id, db_template.id).await;

                    match convert_db_template_to_template_with_fields(db_template, pool).await {
                        Ok(template) => ApiResponse::created(template, "Template cloned successfully".to_string()),
                        Err(e) => ApiResponse::internal_error(format!("Failed to load template fields: {}", e)),
                    }
                }
                Ok(None) => ApiResponse::not_found("Original template not found".to_string()),
                Err(e) => ApiResponse::internal_error(format!("Failed to clone template: {}", e)),
            }
        }
        Ok(None) => ApiResponse::not_found("Original template not found".to_string()),
        Err(e) => ApiResponse::internal_error(format!("Failed to get original template: {}", e)),
    }
}

#[utoipa::path(
    post,
    path = "/api/templates",
    request_body = CreateTemplateRequest,
    responses(
        (status = 201, description = "Template created successfully", body = ApiResponse<Template>),
        (status = 500, description = "Internal server error", body = ApiResponse<Template>)
    ),
    security(("bearer_auth" = [])),
    tag = "templates"
)]
pub async fn create_template(
    State(state): State<AppState>,
    Extension(user_id): Extension<i64>,
    Json(payload): Json<CreateTemplateRequest>,
) -> (StatusCode, Json<ApiResponse<Template>>) {
    let pool = &state.lock().await.db_pool;

    // Get user's account_id
    let user = match crate::database::queries::UserQueries::get_user_by_id(pool, user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => return ApiResponse::not_found("User not found".to_string()),
        Err(e) => return ApiResponse::internal_error(format!("Database error: {}", e)),
    };

    // Decode base64 document
    let document_data = match general_purpose::STANDARD.decode(&payload.document) {
        Ok(data) => data,
        Err(e) => return ApiResponse::bad_request(format!("Invalid base64 document: {}", e)),
    };

    // Initialize storage service
    let storage = match StorageService::new().await {
        Ok(storage) => storage,
        Err(e) => return ApiResponse::internal_error(format!("Failed to initialize storage: {}", e)),
    };

    // Generate filename and upload document
    let filename = format!("{}.txt", payload.name.to_lowercase().replace(" ", "_"));
    let file_key = match storage.upload_file(document_data, &filename, "text/plain").await {
        Ok(key) => key,
        Err(e) => return ApiResponse::internal_error(format!("Failed to upload document: {}", e)),
    };

    // Generate unique slug
    let slug = format!("template-{}-{}", payload.name.to_lowercase().replace(" ", "-"), chrono::Utc::now().timestamp());

    // Create template in database
    let create_template = CreateTemplate {
        name: payload.name.clone(),
        slug: slug.clone(),
        user_id: user_id,
        account_id: user.account_id,
        folder_id: payload.folder_id,
        documents: Some(serde_json::json!([{
            "filename": filename,
            "content_type": "text/plain",
            "size": 0,
            "url": file_key
        }])),
    };

    match TemplateQueries::create_template(pool, create_template).await {
        Ok(db_template) => {
            let template_id = db_template.id;

            // Create fields if provided
            if let Some(fields) = payload.fields {
                for field_req in fields {
                    let create_field = CreateTemplateField {
                        template_id,
                        name: field_req.name,
                        field_type: field_req.field_type,
                        required: field_req.required,
                        display_order: field_req.display_order.unwrap_or(0),
                        position: field_req.position.map(|p| serde_json::to_value(p).unwrap_or(serde_json::Value::Null)),
                        options: field_req.options,
                        metadata: None,
                        partner: field_req.partner,
                    };

                    if let Err(e) = crate::database::queries::TemplateFieldQueries::create_template_field(pool, create_field).await {
                        // Try to clean up template if field creation fails
                        let _ = TemplateQueries::delete_template(pool, template_id).await;
                        return ApiResponse::internal_error(format!("Failed to create template field: {}", e));
                    }
                }
            }

            match convert_db_template_to_template_with_fields(db_template, pool).await {
                Ok(template) => ApiResponse::created(template, "Template created successfully".to_string()),
                Err(e) => {
                    // Try to clean up template if loading fields fails
                    let _ = TemplateQueries::delete_template(pool, template_id).await;
                    ApiResponse::internal_error(format!("Failed to load template fields: {}", e))
                }
            }
        }
        Err(e) => {
            ApiResponse::internal_error(format!("Failed to create template: {}", e))
        }
    }
}

// Placeholder handlers for creating templates from different sources
// These would need actual implementation for PDF/HTML processing

#[utoipa::path(
    post,
    path = "/api/templates/html",
    request_body = CreateTemplateFromHtmlRequest,
    responses(
        (status = 201, description = "Template created from HTML", body = ApiResponse<Template>),
        (status = 500, description = "Internal server error", body = ApiResponse<Template>)
    ),
    security(("bearer_auth" = [])),
    tag = "templates"
)]
pub async fn create_template_from_html(
    State(state): State<AppState>,
    Extension(user_id): Extension<i64>,
    Json(payload): Json<CreateTemplateFromHtmlRequest>,
) -> (StatusCode, Json<ApiResponse<Template>>) {
    let pool = &state.lock().await.db_pool;

    // Initialize storage service
    let storage = match StorageService::new().await {
        Ok(storage) => {
            eprintln!("Storage initialized successfully");
            storage
        },
        Err(e) => {
            eprintln!("Storage init error: {:?}", e);
            // For now, skip storage and just create template in DB
            eprintln!("Skipping storage upload, creating template in DB only");
            return create_template_without_storage(pool, payload, user_id).await;
        }
    };

    // Convert HTML to bytes
    let html_data = payload.html.as_bytes().to_vec();
    let filename = format!("{}.html", payload.name.to_lowercase().replace(" ", "_"));

    // Upload HTML file to storage
    let file_key = match storage.upload_file(html_data, &filename, "text/html").await {
        Ok(key) => key,
        Err(e) => return ApiResponse::internal_error(format!("Failed to upload HTML file: {}", e)),
    };

    // Generate unique slug
    let slug = format!("html-{}-{}", payload.name.to_lowercase().replace(" ", "-"), chrono::Utc::now().timestamp());

    // Get user's account_id
    let account_id = match crate::database::queries::UserQueries::get_user_by_id(pool, user_id).await {
        Ok(Some(user)) => user.account_id,
        Ok(None) => return ApiResponse::not_found("User not found".to_string()),
        Err(e) => return ApiResponse::internal_error(format!("Failed to get user: {}", e)),
    };

    // Create template in database
    let create_template = CreateTemplate {
        name: payload.name.clone(),
        slug: slug.clone(),
        user_id: user_id,
        account_id,
        folder_id: payload.folder_id,
        // fields: None, // Removed - fields will be added separately
        documents: None, // Skip documents for now
    };

    match TemplateQueries::create_template(pool, create_template).await {
        Ok(db_template) => {
            match convert_db_template_to_template_with_fields(db_template, pool).await {
                Ok(template) => ApiResponse::created(template, "Template created from HTML successfully".to_string()),
                Err(e) => {
                    // Try to delete uploaded file if database operation fails
                    let _ = storage.delete_file(&file_key).await;
                    ApiResponse::internal_error(format!("Failed to load template fields: {}", e))
                }
            }
        }
        Err(e) => {
            // Try to delete uploaded file if database operation fails
            let _ = storage.delete_file(&file_key).await;
            ApiResponse::internal_error(format!("Failed to create template: {}", e))
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/templates/pdf",
    request_body = CreateTemplateFromPdfRequest,
    responses(
        (status = 201, description = "Template created from PDF", body = ApiResponse<Template>),
        (status = 500, description = "Internal server error", body = ApiResponse<Template>)
    ),
    security(("bearer_auth" = [])),
    tag = "templates"
)]
pub async fn create_template_from_pdf(
    State(state): State<AppState>,
    Extension(user_id): Extension<i64>,
    mut multipart: Multipart,
) -> (StatusCode, Json<ApiResponse<Template>>) {
    let pool = &state.lock().await.db_pool;

    // Initialize storage service
    let storage = match StorageService::new().await {
        Ok(storage) => storage,
        Err(e) => return ApiResponse::internal_error(format!("Failed to initialize storage: {}", e)),
    };

    let mut pdf_data = Vec::new();
    let mut filename = String::new();
    let mut template_name = String::new();

    // Parse multipart form data
    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let field_name = field.name().unwrap_or("").to_string();

        match field_name.as_str() {
            "pdf" => {
                filename = field.file_name().unwrap_or("template.pdf").to_string();
                pdf_data = field.bytes().await.unwrap_or_default().to_vec();
            }
            "name" => {
                template_name = String::from_utf8(field.bytes().await.unwrap_or_default().to_vec())
                    .unwrap_or_else(|_| "Untitled Template".to_string());
            }
            _ => {}
        }
    }

    if pdf_data.is_empty() {
        return ApiResponse::bad_request("PDF file is required".to_string());
    }

    if template_name.is_empty() {
        template_name = "PDF Template".to_string();
    }

    // Upload file to storage
    let file_key = match storage.upload_file(pdf_data, &filename, "application/pdf").await {
        Ok(key) => key,
        Err(e) => return ApiResponse::internal_error(format!("Failed to upload file: {}", e)),
    };

    // Generate unique slug
    let slug = format!("pdf-{}-{}", template_name.to_lowercase().replace(" ", "-"), chrono::Utc::now().timestamp());

    // Get user's account_id
    let account_id = match crate::database::queries::UserQueries::get_user_by_id(pool, user_id).await {
        Ok(Some(user)) => user.account_id,
        Ok(None) => return ApiResponse::not_found("User not found".to_string()),
        Err(e) => return ApiResponse::internal_error(format!("Failed to get user: {}", e)),
    };

    // Create template in database
    let create_template = CreateTemplate {
        name: template_name.clone(),
        slug: slug.clone(),
        user_id: user_id,
        account_id,
        folder_id: None, // PDF uploads don't specify folder initially
        // fields: None, // TODO: Extract fields from PDF - REMOVED
        documents: Some(serde_json::json!([{
            "filename": filename,
            "content_type": "application/pdf",
            "size": 0,
            "url": file_key
        }])),
    };

    match TemplateQueries::create_template(pool, create_template).await {
        Ok(db_template) => {
            match convert_db_template_to_template_with_fields(db_template, pool).await {
                Ok(template) => ApiResponse::created(template, "Template created from PDF successfully".to_string()),
                Err(e) => {
                    // Try to delete uploaded file if database operation fails
                    let _ = storage.delete_file(&file_key).await;
                    ApiResponse::internal_error(format!("Failed to load template fields: {}", e))
                }
            }
        }
        Err(e) => {
            // Try to delete uploaded file if database operation fails
            let _ = storage.delete_file(&file_key).await;
            ApiResponse::internal_error(format!("Failed to create template: {}", e))
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/templates/google_drive_documents",
    request_body = CreateTemplateFromGoogleDriveRequest,
    responses(
        (status = 201, description = "Template created from Google Drive successfully", body = ApiResponse<Template>),
        (status = 400, description = "Bad request", body = ApiResponse<Template>),
        (status = 500, description = "Internal server error", body = ApiResponse<Template>)
    ),
    security(("bearer_auth" = [])),
    tag = "templates"
)]
pub async fn create_template_from_google_drive(
    State(state): State<AppState>,
    Extension(user_id): Extension<i64>,
    Json(payload): Json<CreateTemplateFromGoogleDriveRequest>,
) -> (StatusCode, Json<ApiResponse<Template>>) {
    eprintln!("🚀 create_template_from_google_drive called for user_id={}", user_id);
    eprintln!("📁 Google Drive file IDs: {:?}", payload.google_drive_file_ids);
    
    let pool = &state.lock().await.db_pool;

    // Initialize storage service
    eprintln!("💾 Initializing storage...");
    let storage = match StorageService::new().await {
        Ok(storage) => storage,
        Err(e) => return ApiResponse::internal_error(format!("Failed to initialize storage: {}", e)),
    };

    if payload.google_drive_file_ids.is_empty() {
        return ApiResponse::bad_request("No Google Drive files provided".to_string());
    }

    // For now, process only the first file
    let file_id = &payload.google_drive_file_ids[0];

    // Get OAuth token for the user
    eprintln!("🔑 Getting OAuth token for user_id={}...", user_id);
    let oauth_token = match crate::database::queries::OAuthTokenQueries::get_oauth_token(pool, user_id, "google").await {
        Ok(Some(token)) => {
            eprintln!("✅ OAuth token found, expires_at: {:?}", token.expires_at);
            // Check if token is expired
            if let Some(expires_at) = token.expires_at {
                if expires_at < chrono::Utc::now() {
                    eprintln!("❌ Token expired!");
                    return ApiResponse::bad_request("Google Drive access token expired. Please reconnect your Google Drive.".to_string());
                }
            }
            token
        },
        Ok(None) => {
            eprintln!("❌ No OAuth token found!");
            return ApiResponse::bad_request("Google Drive not connected. Please connect your Google Drive first.".to_string());
        },
        Err(e) => {
            eprintln!("❌ Database error: {}", e);
            return ApiResponse::internal_error(format!("Database error: {}", e));
        },
    };

    // Create HTTP client
    let client = reqwest::Client::new();

    // Get file metadata first to check MIME type
    let metadata_url = format!("https://www.googleapis.com/drive/v3/files/{}?fields=name,mimeType", file_id);
    eprintln!("🔍 Getting metadata from: {}", metadata_url);
    let metadata_response = match client
        .get(&metadata_url)
        .header("Authorization", format!("Bearer {}", oauth_token.access_token))
        .send()
        .await
    {
        Ok(resp) => {
            eprintln!("📡 Metadata response status: {}", resp.status());
            resp
        },
        Err(e) => {
            eprintln!("❌ Failed to get metadata: {}", e);
            return ApiResponse::internal_error(format!("Failed to get file metadata: {}", e));
        }
    };

    if !metadata_response.status().is_success() {
        let status = metadata_response.status();
        eprintln!("❌ Metadata request failed with status: {}", status);
        let error_body = metadata_response.text().await.unwrap_or_default();
        eprintln!("❌ Error body: {}", error_body);
        return ApiResponse::bad_request(format!("Failed to get file metadata. Status: {}. You may need to grant additional permissions.", status));
    }

    let metadata: serde_json::Value = match metadata_response.json().await {
        Ok(json) => {
            eprintln!("✅ Metadata received: {}", serde_json::to_string_pretty(&json).unwrap_or_default());
            json
        },
        Err(e) => {
            eprintln!("❌ Failed to parse metadata JSON: {}", e);
            return ApiResponse::internal_error(format!("Failed to parse metadata: {}", e));
        }
    };

    let filename = metadata["name"].as_str().unwrap_or("document").to_string();
    let mime_type = metadata["mimeType"].as_str().unwrap_or("");
    eprintln!("📄 File name: {}, MIME type: {}", filename, mime_type);

    // Check if it's a Google Workspace file that needs export
    let (download_url, export_mime_type) = if mime_type.starts_with("application/vnd.google-apps.") {
        eprintln!("🔄 Google Workspace file detected, using export API");
        let export_type = match mime_type {
            "application/vnd.google-apps.document" => "application/pdf",
            "application/vnd.google-apps.spreadsheet" => "application/pdf",
            "application/vnd.google-apps.presentation" => "application/pdf",
            "application/vnd.google-apps.drawing" => "application/pdf",
            _ => {
                eprintln!("❌ Unsupported Google Workspace type: {}", mime_type);
                return ApiResponse::bad_request(format!("Unsupported Google Workspace file type: {}", mime_type));
            }
        };
        // URL encode the MIME type for the export API
        let encoded_mime = urlencoding::encode(export_type);
        (format!("https://www.googleapis.com/drive/v3/files/{}/export?mimeType={}", file_id, encoded_mime), export_type)
    } else {
        (format!("https://www.googleapis.com/drive/v3/files/{}?alt=media", file_id), mime_type)
    };

    // Download or export the file
    eprintln!("⬇️ Downloading from: {}", download_url);
    let response = match client
        .get(&download_url)
        .header("Authorization", format!("Bearer {}", oauth_token.access_token))
        .send()
        .await
    {
        Ok(resp) => {
            eprintln!("📡 Got response, status: {}", resp.status());
            resp
        },
        Err(e) => {
            eprintln!("❌ Failed to download: {}", e);
            return ApiResponse::internal_error(format!("Failed to download file: {}", e));
        },
    };

    if !response.status().is_success() {
        eprintln!("❌ Download failed with status: {}", response.status());
        return ApiResponse::bad_request("Failed to download file from Google Drive. The file may not exist or you may not have permission.".to_string());
    }

    let file_data = match response.bytes().await {
        Ok(bytes) => bytes.to_vec(),
        Err(e) => return ApiResponse::internal_error(format!("Failed to read file data: {}", e)),
    };
    eprintln!("✅ Downloaded {} bytes from Google Drive", file_data.len());

    // Add appropriate extension if Google Workspace file was exported
    let final_filename = if mime_type.starts_with("application/vnd.google-apps.") && !filename.ends_with(".pdf") {
        format!("{}.pdf", filename)
    } else {
        filename.clone()
    };

    // Determine content type from export type or original MIME
    let content_type = if mime_type.starts_with("application/vnd.google-apps.") {
        export_mime_type
    } else {
        get_content_type_from_filename(&final_filename)
    };
    eprintln!("📋 Final filename: {}, Content type: {}", final_filename, content_type);

    // Upload file to storage
    let file_key = match storage.upload_file(file_data.clone(), &final_filename, content_type).await {
        Ok(key) => {
            eprintln!("✅ File uploaded to storage: {}", key);
            key
        },
        Err(e) => {
            eprintln!("❌ Failed to upload file: {}", e);
            return ApiResponse::internal_error(format!("Failed to upload file: {}", e));
        }
    };

    // Generate unique slug - remove file extension for slug
    let name_without_ext = final_filename.rsplit_once('.').map(|(name, _)| name).unwrap_or(&final_filename);
    let slug = format!("gdrive-{}-{}", name_without_ext.to_lowercase().replace(" ", "-"), chrono::Utc::now().timestamp());

    let template_name = payload.name.unwrap_or_else(|| name_without_ext.to_string());

    // Get user's account_id
    let account_id = match crate::database::queries::UserQueries::get_user_by_id(pool, user_id).await {
        Ok(Some(user)) => user.account_id,
        Ok(None) => return ApiResponse::not_found("User not found".to_string()),
        Err(e) => return ApiResponse::internal_error(format!("Failed to get user: {}", e)),
    };

    // Create template in database
    let create_template = CreateTemplate {
        name: template_name.clone(),
        slug: slug.clone(),
        user_id: user_id,
        account_id,
        folder_id: payload.folder_id,
        documents: Some(serde_json::json!([{
            "filename": final_filename,
            "content_type": content_type,
            "size": file_data.len(),
            "url": file_key
        }])),
    };

    match TemplateQueries::create_template(pool, create_template).await {
        Ok(db_template) => {
            match convert_db_template_to_template_with_fields(db_template, pool).await {
                Ok(template) => ApiResponse::created(template, "Template created from Google Drive successfully".to_string()),
                Err(e) => {
                    // Try to delete uploaded file if database operation fails
                    let _ = storage.delete_file(&file_key).await;
                    ApiResponse::internal_error(format!("Failed to load template fields: {}", e))
                }
            }
        }
        Err(e) => {
            // Try to delete uploaded file if database operation fails
            let _ = storage.delete_file(&file_key).await;
            ApiResponse::internal_error(format!("Failed to create template: {}", e))
        }
}
}

#[utoipa::path(
    post,
    path = "/api/templates/docx",
    request_body = CreateTemplateFromDocxRequest,
    responses(
        (status = 201, description = "Template created from DOCX", body = ApiResponse<Template>),
        (status = 500, description = "Internal server error", body = ApiResponse<Template>)
    ),
    security(("bearer_auth" = [])),
    tag = "templates"
)]
pub async fn create_template_from_docx(
    State(state): State<AppState>,
    Extension(user_id): Extension<i64>,
    mut multipart: Multipart,
) -> (StatusCode, Json<ApiResponse<Template>>) {
    let pool = &state.lock().await.db_pool;

    // Initialize storage service
    let storage = match StorageService::new().await {
        Ok(storage) => storage,
        Err(e) => return ApiResponse::internal_error(format!("Failed to initialize storage: {}", e)),
    };

    let mut docx_data = Vec::new();
    let mut filename = String::new();
    let mut template_name = String::new();

    // Parse multipart form data
    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let field_name = field.name().unwrap_or("").to_string();

        match field_name.as_str() {
            "docx" => {
                filename = field.file_name().unwrap_or("template.docx").to_string();
                docx_data = field.bytes().await.unwrap_or_default().to_vec();
            }
            "name" => {
                template_name = String::from_utf8(field.bytes().await.unwrap_or_default().to_vec())
                    .unwrap_or_else(|_| "Untitled Template".to_string());
            }
            _ => {}
        }
    }

    if docx_data.is_empty() {
        return ApiResponse::bad_request("DOCX file is required".to_string());
    }

    if template_name.is_empty() {
        template_name = "DOCX Template".to_string();
    }

    // Upload file to storage
    let file_key = match storage.upload_file(docx_data, &filename, "application/vnd.openxmlformats-officedocument.wordprocessingml.document").await {
        Ok(key) => key,
        Err(e) => return ApiResponse::internal_error(format!("Failed to upload file: {}", e)),
    };

    // Generate unique slug
    let slug = format!("docx-{}-{}", template_name.to_lowercase().replace(" ", "-"), chrono::Utc::now().timestamp());

    // Get user's account_id
    let account_id = match crate::database::queries::UserQueries::get_user_by_id(pool, user_id).await {
        Ok(Some(user)) => user.account_id,
        Ok(None) => return ApiResponse::not_found("User not found".to_string()),
        Err(e) => return ApiResponse::internal_error(format!("Failed to get user: {}", e)),
    };

    // Create template in database
    let create_template = CreateTemplate {
        name: template_name.clone(),
        slug: slug.clone(),
        user_id: user_id,
        account_id,
        folder_id: None, // DOCX uploads don't specify folder initially
        // fields: None, // TODO: Extract fields from DOCX - REMOVED
        documents: Some(serde_json::json!([{
            "filename": filename,
            "content_type": "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "size": 0, // TODO: Get actual file size
            "url": file_key
        }])),
    };

    match TemplateQueries::create_template(pool, create_template).await {
        Ok(db_template) => {
            match convert_db_template_to_template_with_fields(db_template, pool).await {
                Ok(template) => ApiResponse::created(template, "Template created from DOCX successfully".to_string()),
                Err(e) => {
                    // Try to delete uploaded file if database operation fails
                    let _ = storage.delete_file(&file_key).await;
                    ApiResponse::internal_error(format!("Failed to load template fields: {}", e))
                }
            }
        }
        Err(e) => {
            // Try to delete uploaded file if database operation fails
            let _ = storage.delete_file(&file_key).await;
            ApiResponse::internal_error(format!("Failed to create template: {}", e))
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/templates/merge",
    request_body = MergeTemplatesRequest,
    responses(
        (status = 201, description = "Templates merged successfully", body = ApiResponse<Template>),
        (status = 500, description = "Internal server error", body = ApiResponse<Template>)
    ),
    security(("bearer_auth" = [])),
    tag = "templates"
)]
pub async fn merge_templates(
    State(state): State<AppState>,
    Extension(user_id): Extension<i64>,
    Json(_payload): Json<MergeTemplatesRequest>,
) -> (StatusCode, Json<ApiResponse<Template>>) {
    // TODO: Implement template merging logic
    ApiResponse::internal_error("Template merging not yet implemented".to_string())
}

// Helper function to create template without storage (for testing)
async fn create_template_without_storage(
    pool: &sqlx::PgPool,
    payload: CreateTemplateFromHtmlRequest,
    user_id: i64,
) -> (StatusCode, Json<ApiResponse<Template>>) {
    // Generate unique slug
    let slug = format!("html-{}-{}", payload.name.to_lowercase().replace(" ", "-"), chrono::Utc::now().timestamp());

    // Get user's account_id
    let account_id = match crate::database::queries::UserQueries::get_user_by_id(pool, user_id).await {
        Ok(Some(user)) => user.account_id,
        Ok(None) => return ApiResponse::not_found("User not found".to_string()),
        Err(e) => return ApiResponse::internal_error(format!("Failed to get user: {}", e)),
    };

    // Create template in database
    let create_template = CreateTemplate {
        name: payload.name.clone(),
        slug: slug.clone(),
        user_id: user_id,
        account_id,
        folder_id: payload.folder_id,
        // fields: None, // Removed - fields will be added separately
        documents: None, // Skip documents for now
    };

    match TemplateQueries::create_template(pool, create_template).await {
        Ok(db_template) => {
            match convert_db_template_to_template_with_fields(db_template, pool).await {
                Ok(template) => ApiResponse::created(template, "Template created from HTML successfully (without storage)".to_string()),
                Err(e) => ApiResponse::internal_error(format!("Failed to load template fields: {}", e)),
            }
        }
        Err(e) => {
            ApiResponse::internal_error(format!("Failed to create template: {}", e))
        }
    }
}

// Helper function to convert database template to API template (sync version for simple cases)
pub fn convert_db_template_to_template(db_template: crate::database::models::DbTemplate) -> Template {
    Template {
        id: db_template.id,
        name: db_template.name,
        slug: db_template.slug,
        user_id: db_template.user_id,
        user_name: None, // Will be set by caller if needed
        folder_id: db_template.folder_id,
        template_fields: None, // Will be loaded separately if needed
        submitters: None, // No longer stored in templates
        documents: db_template.documents.and_then(|v| serde_json::from_value(v).ok()),
        created_at: db_template.created_at,
        updated_at: db_template.updated_at,
    }
}

// Helper function to convert database template to API template with fields loaded (async)
pub fn convert_db_template_to_template_without_fields(
    db_template: crate::database::models::DbTemplate,
) -> Template {
    Template {
        id: db_template.id,
        name: db_template.name,
        slug: db_template.slug,
        user_id: db_template.user_id,
        user_name: None, // Will be set by caller if needed
        folder_id: db_template.folder_id,
        template_fields: None,
        submitters: None,
        documents: db_template.documents.and_then(|v| serde_json::from_value(v).ok()),
        created_at: db_template.created_at,
        updated_at: db_template.updated_at,
    }
}

pub async fn convert_db_template_to_template_with_fields(
    db_template: crate::database::models::DbTemplate,
    pool: &sqlx::PgPool
) -> Result<Template, sqlx::Error> {
    use crate::database::queries::TemplateFieldQueries;

    let template_fields = TemplateFieldQueries::get_template_fields(pool, db_template.id).await?
        .into_iter()
        .map(|db_field| TemplateField {
            id: db_field.id,
            template_id: db_field.template_id,
            name: db_field.name,
            field_type: db_field.field_type,
            required: db_field.required,
            display_order: db_field.display_order,
            position: db_field.position.and_then(|v| serde_json::from_value(v).ok()),
            options: db_field.options,
            partner: db_field.partner,
            created_at: db_field.created_at,
            updated_at: db_field.updated_at,
        })
        .collect::<Vec<_>>();

    Ok(Template {
        id: db_template.id,
        name: db_template.name,
        slug: db_template.slug,
        user_id: db_template.user_id,
        user_name: None, // Will be set by caller if needed
        folder_id: db_template.folder_id,
        template_fields: Some(template_fields),
        submitters: None, // No longer stored in templates
        documents: db_template.documents.and_then(|v| serde_json::from_value(v).ok()),
        created_at: db_template.created_at,
        updated_at: db_template.updated_at,
    })
}

#[utoipa::path(
    get,
    path = "/api/files/{key}",
    params(
        ("key" = String, Path, description = "File path in storage (e.g., 'templates/1759746273_test.pdf')")
    ),
    responses(
        (status = 200, description = "File downloaded successfully"),
        (status = 404, description = "File not found")
    ),
    tag = "files"
)]
pub async fn download_file(
    Path(key): Path<String>,
) -> Response<Body> {
    // Initialize storage service
    let storage = match StorageService::new().await {
        Ok(storage) => storage,
        Err(_) => {
            // Return default PDF on storage error
            const DEFAULT_PDF: &[u8] = b"%PDF-1.4\n1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n>>\nendobj\n2 0 obj\n<<\n/Type /Pages\n/Kids [3 0 R]\n/Count 1\n>>\nendobj\n3 0 obj\n<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [0 0 612 792]\n/Contents 4 0 R\n>>\nendobj\n4 0 obj\n<<\n/Length 0\n>>\nstream\n\nendstream\nendobj\nxref\n0 5\n0000000000 65535 f \n0000000009 00000 n \n0000000058 00000 n \n0000000115 00000 n \n0000000170 00000 n \ntrailer\n<<\n/Size 5\n/Root 1 0 R\n>>\nstartxref\n226\n%%EOF";
            let response = Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/pdf")
                .header(header::CONTENT_DISPOSITION, format!("inline; filename=\"{}\"", key))
                .header("Access-Control-Allow-Origin", "*")
                .header("Access-Control-Expose-Headers", "*")
                .header("Content-Length", DEFAULT_PDF.len().to_string())
                .body(Body::from(DEFAULT_PDF.to_vec()))
                .unwrap();
            return response;
        }
    };

    // Download file from storage
    let file_data = match storage.download_file(&key).await {
        Ok(data) => data,
        Err(_) => {
            println!("File not found in storage: {}", key);
            // Return 404 Not Found response
            let response = Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header(header::CONTENT_TYPE, "text/plain")
                .header("Access-Control-Allow-Origin", "*")
                .header("Access-Control-Expose-Headers", "*")
                .body(Body::from("File not found"))
                .unwrap();
            return response;
        }
    };
    // Determine content type based on file extension
    let content_type = get_content_type_from_filename(&key);

    // Create response with file data
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_DISPOSITION, format!("inline; filename=\"{}\"", key))
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Expose-Headers", "*")
        .header("Content-Length", file_data.len().to_string())
        .body(Body::from(file_data))
        .unwrap();

    response
}

#[utoipa::path(
    get,
    path = "/api/files/previews/{key}",
    params(
        ("key" = String, Path, description = "File path in storage (e.g., 'templates/previews/1759746273_test_page_1.jpg')")
    ),
    responses(
        (status = 200, description = "File downloaded successfully"),
        (status = 404, description = "File not found")
    ),
    tag = "files"
)]
pub async fn download_file_public(
    Path(key): Path<String>,
) -> Response<Body> {
    // Initialize storage service
    let storage = match StorageService::new().await {
        Ok(storage) => storage,
        Err(_) => {
            // Return default image on storage error
            const DEFAULT_IMAGE: &[u8] = b"iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg=="; // 1x1 transparent PNG
            let response = Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "image/png")
                .header(header::CONTENT_DISPOSITION, format!("inline; filename=\"{}\"", key))
                .header("Access-Control-Allow-Origin", "*")
                .header("Access-Control-Expose-Headers", "*")
                .header("Content-Length", DEFAULT_IMAGE.len().to_string())
                .body(Body::from(DEFAULT_IMAGE.to_vec()))
                .unwrap();
            return response;
        }
    };

    // Download file from storage
    let file_data = match storage.download_file(&key).await {
        Ok(data) => data,
        Err(_) => {
            println!("File not found in storage: {}", key);
            
            // Check if this is a preview request that needs to be generated
            // Pattern: templates/previews/FILENAME_page_N.jpg
            if key.contains("/previews/") && key.contains("_page_") {
                // Extract page number and original PDF path
                if let Some(page_start) = key.rfind("_page_") {
                    let after_page = &key[page_start + 6..];
                    if let Some(dot_pos) = after_page.find('.') {
                        if let Ok(page_number) = after_page[..dot_pos].parse::<i32>() {
                            // Extract format
                            let format = &after_page[dot_pos + 1..];
                            
                            // Reconstruct original PDF path
                            // From: templates/previews/FILENAME_page_2.jpg
                            // To: templates/FILENAME.pdf
                            let base_key = &key[..page_start];
                            let pdf_key = base_key.replace("/previews/", "/") + ".pdf";
                            
                            println!("Generating preview on-demand: {} from {}", key, pdf_key);
                            
                            // Download original PDF
                            if let Ok(pdf_data) = storage.download_file(&pdf_key).await {
                                // Render the requested page
                                if let Ok(image_data) = render_pdf_page_to_image(&pdf_data, page_number, format) {
                                    // Save to storage for future requests
                                    let content_type = if format == "png" { "image/png" } else { "image/jpeg" };
                                    let _ = storage.upload_file_with_key(image_data.clone(), &key, content_type).await;
                                    
                                    // Return the generated image
                                    let response = Response::builder()
                                        .status(StatusCode::OK)
                                        .header(header::CONTENT_TYPE, content_type)
                                        .header(header::CONTENT_DISPOSITION, format!("inline; filename=\"page_{}.{}\"", page_number, format))
                                        .header("Access-Control-Allow-Origin", "*")
                                        .header("Access-Control-Expose-Headers", "*")
                                        .header("Content-Length", image_data.len().to_string())
                                        .header("Cache-Control", "public, max-age=86400")
                                        .body(Body::from(image_data))
                                        .unwrap();
                                    return response;
                                } else {
                                    println!("Failed to render PDF page {} from {}", page_number, pdf_key);
                                }
                            } else {
                                println!("Original PDF not found: {}", pdf_key);
                            }
                        }
                    }
                }
            }
            
            // Return 404 Not Found response - NO CACHE for errors
            let response = Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header(header::CONTENT_TYPE, "text/plain")
                .header("Access-Control-Allow-Origin", "*")
                .header("Access-Control-Expose-Headers", "*")
                .header("Cache-Control", "no-store, no-cache, must-revalidate, max-age=0")
                .header("Pragma", "no-cache")
                .header("Expires", "0")
                .body(Body::from("File not found"))
                .unwrap();
            return response;
        }
    };
    // Determine content type based on file extension
    let content_type = get_content_type_from_filename(&key);

    // Create response with file data
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_DISPOSITION, format!("inline; filename=\"{}\"", key))
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Expose-Headers", "*")
        .header("Content-Length", file_data.len().to_string())
        .body(Body::from(file_data))
        .unwrap();

    response
}

#[derive(serde::Deserialize)]
pub struct PreviewQuery {
    page: Option<i32>,
    format: Option<String>,
}

#[axum::debug_handler]
#[utoipa::path(
    get,
    path = "/api/files/preview/{key}",
    params(
        ("key" = String, Path, description = "File key in storage (e.g., 'templates/1234567890_document.pdf' or 'templates/1234567890_document_page_2.png')"),
        ("page" = Option<i32>, Query, description = "Page number - if not provided, returns JSON with all page URLs"),
        ("format" = Option<String>, Query, description = "Image format: jpg or png (default: jpg)")
    ),
    responses(
        (status = 200, description = "Preview image or JSON with all pages"),
        (status = 404, description = "File not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "files"
)]
pub async fn preview_file(
    State(_state): State<AppState>,
    Path(key): Path<String>,
    Query(query): Query<PreviewQuery>,
) -> impl IntoResponse {
    // Wildcard paths include leading slash, so remove it
    let key = key.trim_start_matches('/');
    
    // Parse page number and format from URL
    let mut page_number: Option<i32> = None;
    let mut image_format = "jpg";
    let mut file_key = key.to_string();
    
    // Check if key contains page number in filename (e.g., "document_page_2.png")
    if key.contains("_page_") {
        // Extract page number and format from filename
        if let Some(page_start) = key.rfind("_page_") {
            let after_page = &key[page_start + 6..]; // Skip "_page_"
            
            // Try to extract page number
            if let Some(dot_pos) = after_page.find('.') {
                if let Ok(page) = after_page[..dot_pos].parse::<i32>() {
                    page_number = Some(page);
                }
                
                // Extract format from extension
                let extension = &after_page[dot_pos + 1..];
                if extension == "png" || extension == "jpg" || extension == "jpeg" {
                    image_format = extension;
                }
                
                // Remove the _page_X.ext suffix to get the original file key
                // Also remove /previews/ from the path if present
                let base_key = &key[..page_start];
                file_key = if base_key.contains("/previews/") {
                    base_key.replace("/previews/", "/") + ".pdf"
                } else {
                    base_key.to_string() + ".pdf"
                };
            }
        }
    }
    
    // Override with query parameters if provided
    if let Some(page) = query.page {
        page_number = Some(page);
    }
    if let Some(format_str) = &query.format {
        if format_str == "png" || format_str == "jpg" || format_str == "jpeg" {
            image_format = format_str;
        }
    }
    
    // Initialize storage service
    let storage = match StorageService::new().await {
        Ok(storage) => storage,
        Err(e) => {
            eprintln!("Failed to initialize storage: {:?}", e);
            let response = Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header(header::CONTENT_TYPE, "text/plain")
                .header("Access-Control-Allow-Origin", "*")
                .body(Body::from("Storage initialization failed"))
                .unwrap();
            return response;
        }
    };
    
    // If no page number specified, check if file is PDF or image
    if page_number.is_none() {
        // Check file extension to determine if it's a PDF or image
        let is_pdf = file_key.to_lowercase().ends_with(".pdf");
        
        if !is_pdf {
            // For non-PDF files (images), return URL immediately without downloading
            let file_url = format!("/api/files/{}", file_key);
            let json_response = serde_json::json!({
                "url": file_url,
                "type": "image"
            });
            
            let response = Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .header("Access-Control-Allow-Origin", "*")
                .header("Access-Control-Expose-Headers", "*")
                .header("Cache-Control", "public, max-age=3600")
                .body(Body::from(json_response.to_string()))
                .unwrap();
            return response;
        }
        
        // For PDF files, download to get page count
        let pdf_data = match storage.download_file(&file_key).await {
            Ok(data) => data,
            Err(e) => {
                eprintln!("Failed to download PDF: {:?}", e);
                let response = Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header(header::CONTENT_TYPE, "application/json")
                    .header("Access-Control-Allow-Origin", "*")
                    .body(Body::from(serde_json::json!({"error": "PDF file not found"}).to_string()))
                    .unwrap();
                return response;
            }
        };
        
        // Get total page count
        let total_pages = match get_pdf_page_count(&pdf_data) {
            Ok(count) => count,
            Err(e) => {
                eprintln!("Failed to read PDF: {:?}, file_key: {}, pdf_data len: {}, falling back to 1 page", e, file_key, pdf_data.len());
                1 // Fallback to 1 page if PDF parsing fails
            }
        };
        
        // OPTIMIZATION: Generate URLs for lazy loading - DON'T render all pages immediately
        // Only generate first page preview, let frontend request others on-demand
        let mut page_urls = Vec::new();
        
        // Extract base filename (without .pdf extension)
        // file_key could be "templates/xyz.pdf" or "templates/previews/xyz.pdf"
        let base_path = file_key.trim_end_matches(".pdf");
        
        // Determine the correct preview path
        let (preview_base, base_filename) = if base_path.starts_with("templates/previews/") {
            // Already in previews folder, use as-is
            (base_path.to_string(), base_path.trim_start_matches("templates/previews/").to_string())
        } else if base_path.starts_with("templates/") {
            // In templates folder, move to previews
            let filename = base_path.trim_start_matches("templates/");
            (format!("templates/previews/{}", filename), filename.to_string())
        } else {
            // No templates prefix, add it
            (format!("templates/previews/{}", base_path), base_path.to_string())
        };
        
        // Generate first page only to show something immediately
        let first_page_preview_key = format!("{}_page_1.{}", preview_base, image_format);
        
        // Check if first page preview exists, if not generate it
        if storage.download_file(&first_page_preview_key).await.is_err() {
            if let Ok(image_data) = render_pdf_page_to_image(&pdf_data, 1, image_format) {
                let content_type = match image_format {
                    "png" => "image/png",
                    _ => "image/jpeg",
                };
                let _ = storage.upload_file_with_key(image_data, &first_page_preview_key, content_type).await;
            }
        }
        
        // Return URLs for all pages (they will be lazy-loaded by frontend)
        for page_num in 1..=total_pages {
            let preview_key = format!("{}_page_{}.{}", preview_base, page_num, image_format);
            page_urls.push(format!("/api/files/previews/{}", preview_key));
        }
        
        // Return JSON response with all page URLs for lazy loading
        let json_response = serde_json::json!({
            "total_pages": total_pages,
            "format": image_format,
            "pages": page_urls
        });
        
        let response = Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .header("Access-Control-Allow-Origin", "*")
            .header("Access-Control-Expose-Headers", "*")
            .header("Cache-Control", "public, max-age=300") // Cache for 5 minutes
            .body(Body::from(json_response.to_string()))
            .unwrap();
        return response;
    }
    
    // Single page request - return image
    let page_number = page_number.unwrap();
    
    // Initialize storage service
    let storage = match StorageService::new().await {
        Ok(storage) => storage,
        Err(e) => {
            eprintln!("Failed to initialize storage: {:?}", e);
            let response = Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header(header::CONTENT_TYPE, "text/plain")
                .header("Access-Control-Allow-Origin", "*")
                .body(Body::from("Storage initialization failed"))
                .unwrap();
            return response;
        }
    };
    
    // Step 2: Check if preview image already exists
    // Determine correct preview path (avoid duplication)
    let base_path = file_key.trim_end_matches(".pdf");
    let preview_key = if base_path.starts_with("templates/previews/") {
        // Already in previews folder
        format!("{}_page_{}.{}", base_path, page_number, image_format)
    } else if base_path.starts_with("templates/") {
        // In templates folder, move to previews
        let filename = base_path.trim_start_matches("templates/");
        format!("templates/previews/{}_page_{}.{}", filename, page_number, image_format)
    } else {
        // No templates prefix
        format!("templates/previews/{}_page_{}.{}", base_path, page_number, image_format)
    };
    
    // Try to download existing preview
    if let Ok(preview_data) = storage.download_file(&preview_key).await {
        println!("Found existing preview: {}", preview_key);
        let content_type = match image_format {
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            _ => "image/jpeg",
        };
        
        let response = Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, content_type)
            .header(header::CONTENT_DISPOSITION, format!("inline; filename=\"page_{}.{}\"", page_number, image_format))
            .header("Access-Control-Allow-Origin", "*")
            .header("Access-Control-Expose-Headers", "*")
            .header("Content-Length", preview_data.len().to_string())
            .header("Cache-Control", "public, max-age=86400") // Cache for 24 hours
            .header("ETag", format!("\"{}\"", preview_key)) // Add ETag for caching
            .body(Body::from(preview_data))
            .unwrap();
        return response;
    }
    
    // Step 3: Preview doesn't exist, create it by rendering PDF page to image
    println!("Preview not found, generating: {}", preview_key);
    
    // Download the original PDF
    let pdf_data = match storage.download_file(&file_key).await {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to download PDF: {:?}", e);
            let response = Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header(header::CONTENT_TYPE, "text/plain")
                .header("Access-Control-Allow-Origin", "*")
                .header("Cache-Control", "no-store, no-cache, must-revalidate, max-age=0")
                .header("Pragma", "no-cache")
                .body(Body::from("PDF file not found"))
                .unwrap();
            return response;
        }
    };
    
    // Render PDF page to image using pdf2image or similar
    let image_data = match render_pdf_page_to_image(&pdf_data, page_number, image_format) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to render PDF page: {:?}", e);
            let response = Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header(header::CONTENT_TYPE, "text/plain")
                .header("Access-Control-Allow-Origin", "*")
                .header("Cache-Control", "no-store, no-cache, must-revalidate, max-age=0")
                .header("Pragma", "no-cache")
                .body(Body::from(format!("Failed to render PDF page: {}", e)))
                .unwrap();
            return response;
        }
    };
    
    // Step 4: Upload the generated preview image to storage with exact key
    let content_type = match image_format {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        _ => "image/jpeg",
    };
    
    match storage.upload_file_with_key(image_data.clone(), &preview_key, content_type).await {
        Ok(_) => println!("Preview saved: {}", preview_key),
        Err(e) => eprintln!("Failed to save preview: {:?}", e),
    }
    
    // Step 5: Return the preview image
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_DISPOSITION, format!("inline; filename=\"page_{}.{}\"", page_number, image_format))
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Expose-Headers", "*")
        .header("Content-Length", image_data.len().to_string())
        .header("Cache-Control", "public, max-age=86400") // Cache for 24 hours
        .header("ETag", format!("\"{}\"", preview_key)) // Add ETag for caching
        .body(Body::from(image_data))
        .unwrap();

    response
}

/// Helper function to render a PDF page to an image
fn render_pdf_page_to_image(
    pdf_bytes: &[u8],
    page_number: i32,
    format: &str,
) -> Result<Vec<u8>, String> {
    use image::{ImageFormat};
    use std::process::Command;
    use std::io::Write;

    // Use pdftoppm command line tool instead of pdfium for better compatibility
    let mut child = Command::new("pdftoppm")
        .arg("-png")           // Output PNG format
        .arg("-f").arg(page_number.to_string())  // First page to convert
        .arg("-l").arg(page_number.to_string())  // Last page to convert  
        .arg("-scale-to").arg("800")  // Scale to width 800px
        .arg("-singlefile")    // Don't add page number suffix
        .arg("-")              // Read from stdin
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn pdftoppm: {}", e))?;

    // Write PDF data to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(pdf_bytes).map_err(|e| format!("Failed to write to pdftoppm: {}", e))?;
    }

    // Read output
    let result = child.wait_with_output()
        .map_err(|e| format!("Failed to wait for pdftoppm: {}", e))?;

    if !result.status.success() {
        return Err(format!("pdftoppm failed: {}", String::from_utf8_lossy(&result.stderr)));
    }

    // Convert PNG to requested format if needed
    if format == "png" {
        return Ok(result.stdout);
    }

    // Load PNG and convert to JPG
    let img = image::load_from_memory_with_format(&result.stdout, ImageFormat::Png)
        .map_err(|e| format!("Failed to load PNG: {}", e))?;

    let mut output_bytes = Vec::new();
    
    match format {
        "jpg" | "jpeg" => {
            let rgb_img = img.to_rgb8();
            rgb_img.write_to(&mut std::io::Cursor::new(&mut output_bytes), ImageFormat::Jpeg)
                .map_err(|e| format!("Failed to encode JPEG: {}", e))?;
        }
        _ => {
            return Err(format!("Unsupported format: {}", format));
        }
    }

    Ok(output_bytes)
}

/// Get PDF metadata (page count, dimensions, etc.)
/// Helper function to get PDF page count
fn get_pdf_page_count(pdf_bytes: &[u8]) -> Result<i32, String> {
    use lopdf::Document;
    eprintln!("PDF first 100 bytes: {:?}", &pdf_bytes[..std::cmp::min(100, pdf_bytes.len())]);
    eprintln!("PDF last 100 bytes: {:?}", &pdf_bytes[pdf_bytes.len().saturating_sub(100)..]);
    let doc = Document::load_mem(pdf_bytes).map_err(|e| {
        eprintln!("lopdf load error: {:?}, pdf_bytes len: {}", e, pdf_bytes.len());
        e.to_string()
    })?;
    Ok(doc.get_pages().len() as i32)
}

// ===== TEMPLATE FIELDS ENDPOINTS =====

#[utoipa::path(
    get,
    path = "/api/templates/{template_id}/fields",
    params(
        ("template_id" = i64, Path, description = "Template ID")
    ),
    responses(
        (status = 200, description = "Template fields retrieved successfully", body = ApiResponse<Vec<TemplateField>>),
        (status = 404, description = "Template not found", body = ApiResponse<Vec<TemplateField>>),
        (status = 500, description = "Internal server error", body = ApiResponse<Vec<TemplateField>>)
    ),
    security(("bearer_auth" = [])),
    tag = "template_fields"
)]
pub async fn get_template_fields(
    State(state): State<AppState>,
    Path(template_id): Path<i64>,
    Extension(user_id): Extension<i64>,
) -> (StatusCode, Json<ApiResponse<Vec<TemplateField>>>) {
    let pool = &state.lock().await.db_pool;

    // Verify user has permission to access this template
    match TemplateQueries::get_template_by_id(pool, template_id).await {
        Ok(Some(db_template)) => {
            // Get user role to check permissions
            match crate::database::queries::UserQueries::get_user_by_id(pool, user_id).await {
                Ok(Some(user)) => {
                    // Allow access if user is the owner OR if user has Editor/Admin/Member role (Members can read fields from team templates)
                    let has_access = db_template.user_id == user_id || 
                                   matches!(user.role, crate::models::role::Role::Editor | crate::models::role::Role::Admin | crate::models::role::Role::Member);
                    
                    if !has_access {
                        return ApiResponse::not_found("Template not found".to_string());
                    }
                }
                _ => return ApiResponse::not_found("User not found".to_string()),
            }
        }
        Ok(None) => return ApiResponse::not_found("Template not found".to_string()),
        Err(e) => return ApiResponse::internal_error(format!("Failed to verify template: {}", e)),
    }

    match crate::database::queries::TemplateFieldQueries::get_template_fields(pool, template_id).await {
        Ok(fields) => {
            let template_fields: Vec<TemplateField> = fields.into_iter()
                .map(|db_field| TemplateField {
                    id: db_field.id,
                    template_id: db_field.template_id,
                    name: db_field.name,
                    field_type: db_field.field_type,
                    required: db_field.required,
                    display_order: db_field.display_order,
                    position: db_field.position.and_then(|v| serde_json::from_value(v).ok()),
                    options: db_field.options,
                    partner: db_field.partner,
                    created_at: db_field.created_at,
                    updated_at: db_field.updated_at,
                })
                .collect();

            ApiResponse::success(template_fields, "Template fields retrieved successfully".to_string())
        }
        Err(e) => ApiResponse::internal_error(format!("Failed to retrieve template fields: {}", e)),
    }
}

#[utoipa::path(
    post,
    path = "/api/templates/{template_id}/fields",
    params(
        ("template_id" = i64, Path, description = "Template ID")
    ),
    request_body = CreateTemplateFieldRequest,
    responses(
        (status = 201, description = "Template field(s) created successfully", body = ApiResponse<Vec<TemplateField>>),
        (status = 404, description = "Template not found", body = ApiResponse<Vec<TemplateField>>),
        (status = 500, description = "Internal server error", body = ApiResponse<Vec<TemplateField>>)
    ),
    security(("bearer_auth" = [])),
    tag = "template_fields"
)]
pub async fn create_template_field(
    State(state): State<AppState>,
    Path(template_id): Path<i64>,
    Extension(user_id): Extension<i64>,
    Json(payload): Json<serde_json::Value>,
) -> (StatusCode, Json<ApiResponse<Vec<TemplateField>>>) {
    let pool = &state.lock().await.db_pool;

    // Verify user has permission to access this template
    match TemplateQueries::get_template_by_id(pool, template_id).await {
        Ok(Some(db_template)) => {
            // Get user role to check permissions
            match crate::database::queries::UserQueries::get_user_by_id(pool, user_id).await {
                Ok(Some(user)) => {
                    // Allow access if user is the owner OR if user has Editor/Admin role (Members have read-only access to others' templates)
                    let has_access = db_template.user_id == user_id || 
                                   matches!(user.role, crate::models::role::Role::Editor | crate::models::role::Role::Admin);
                    
                    if !has_access {
                        return ApiResponse::forbidden("Access denied: You do not have permission to modify this template".to_string());
                    }
                }
                _ => return ApiResponse::not_found("User not found".to_string()),
            }
        }
        Ok(None) => return ApiResponse::not_found("Template not found".to_string()),
        Err(e) => return ApiResponse::internal_error(format!("Failed to verify template: {}", e)),
    }

    // Check if it's bulk request (has "fields" array) or single field
    let field_requests: Vec<CreateTemplateFieldRequest> = if let Some(fields) = payload.get("fields") {
        if let Some(fields_array) = fields.as_array() {
            fields_array.iter()
                .filter_map(|f| serde_json::from_value(f.clone()).ok())
                .collect()
        } else {
            return ApiResponse::bad_request("Invalid fields format".to_string());
        }
    } else {
        // Single field request
        match serde_json::from_value::<CreateTemplateFieldRequest>(payload) {
            Ok(field) => vec![field],
            Err(_) => return ApiResponse::bad_request("Invalid field format".to_string()),
        }
    };

    if field_requests.is_empty() {
        return ApiResponse::bad_request("No fields provided".to_string());
    }

    let mut created_fields = Vec::new();

    for field_req in field_requests {
        let create_field = CreateTemplateField {
            template_id,
            name: field_req.name,
            field_type: field_req.field_type,
            required: field_req.required,
            display_order: field_req.display_order.unwrap_or(0),
            position: field_req.position.map(|p| serde_json::to_value(p).unwrap_or(serde_json::Value::Null)),
            options: field_req.options,
            metadata: None,
            partner: field_req.partner,
        };

        match crate::database::queries::TemplateFieldQueries::create_template_field(pool, create_field).await {
            Ok(db_field) => {
                let template_field = TemplateField {
                    id: db_field.id,
                    template_id: db_field.template_id,
                    name: db_field.name,
                    field_type: db_field.field_type,
                    required: db_field.required,
                    display_order: db_field.display_order,
                    position: db_field.position.and_then(|v| serde_json::from_value(v).ok()),
                    options: db_field.options,
                    partner: db_field.partner,
                    created_at: db_field.created_at,
                    updated_at: db_field.updated_at,
                };
                created_fields.push(template_field);
            }
            Err(e) => return ApiResponse::internal_error(format!("Failed to create template field: {}", e)),
        }
    }

    ApiResponse::created(created_fields, "Template fields created successfully".to_string())
}

#[utoipa::path(
    post,
    path = "/api/templates/{template_id}/fields/upload",
    params(
        ("template_id" = i64, Path, description = "Template ID")
    ),
    request_body = String,
    responses(
        (status = 201, description = "Field created successfully", body = TemplateField),
        (status = 400, description = "Bad request"),
        (status = 404, description = "Template not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "template_fields"
)]
pub async fn upload_template_field_file(
    State(state): State<AppState>,
    Path(template_id): Path<i64>,
    Extension(user_id): Extension<i64>,
    mut multipart: Multipart,
) -> (StatusCode, Json<ApiResponse<TemplateField>>) {
    let pool = &state.lock().await.db_pool;

    // Initialize S3 client
    let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new("us-east-1"))
        .endpoint_url(std::env::var("STORAGE_ENDPOINT").unwrap_or_else(|_| "http://localhost:9000".to_string()))
        .load()
        .await;
    let s3_client = aws_sdk_s3::Client::new(&config);

    // Verify template belongs to user
    match TemplateQueries::get_template_by_id(pool, template_id).await {
        Ok(Some(db_template)) => {
            if db_template.user_id != user_id {
                return ApiResponse::forbidden("Access denied: You can only upload files to your own templates".to_string());
            }
        }
        Ok(None) => return ApiResponse::not_found("Template not found".to_string()),
        Err(e) => return ApiResponse::internal_error(format!("Failed to verify template: {}", e)),
    }

    let mut name = None;
    let mut field_type = None;
    let mut required = None;
    let mut display_order = None;
    let mut position = None;
    let mut options = None;
    let mut file_data = None;
    let mut filename = None;

    while let Some(field) = multipart.next_field().await.unwrap() {
        let field_name = field.name().unwrap().to_string();
        let file_name = field.file_name().map(|s| s.to_string());
        let field_data = field.bytes().await.unwrap();

        match field_name.as_str() {
            "name" => name = Some(String::from_utf8(field_data.to_vec()).unwrap()),
            "field_type" => field_type = Some(String::from_utf8(field_data.to_vec()).unwrap()),
            "required" => required = Some(String::from_utf8(field_data.to_vec()).unwrap() == "true"),
            "display_order" => display_order = Some(String::from_utf8(field_data.to_vec()).unwrap().parse::<i32>().unwrap_or(0)),
            "position" => position = Some(String::from_utf8(field_data.to_vec()).unwrap()),
            "options" => options = Some(String::from_utf8(field_data.to_vec()).unwrap()),
            "file" => {
                file_data = Some(field_data.to_vec());
                if let Some(f) = file_name {
                    filename = Some(f);
                }
            }
            _ => {}
        }
    }

    let name = if let Some(n) = name { n } else { return ApiResponse::bad_request("name is required".to_string()); };
    let field_type = if let Some(ft) = field_type { ft } else { return ApiResponse::bad_request("field_type is required".to_string()); };
    let required = required.unwrap_or(false);
    let display_order = display_order.unwrap_or(0);

    // Only allow image and file types for upload
    if field_type != "image" && field_type != "file" {
        return ApiResponse::bad_request("Only image and file field types are supported for upload".to_string());
    }

    let file_data = if let Some(fd) = file_data { fd } else { return ApiResponse::bad_request("file is required".to_string()); };
    let filename = if let Some(f) = filename { f } else { return ApiResponse::bad_request("filename is required".to_string()); };

    // Upload to S3
    let timestamp = chrono::Utc::now().timestamp();
    let s3_key = format!("template_fields/{}_{}", timestamp, filename);

    let content_type = get_content_type_from_filename(&filename);

    match s3_client
        .put_object()
        .bucket("docuseal") // Replace with your bucket name
        .key(&s3_key)
        .body(file_data.into())
        .content_type(content_type)
        .send()
        .await
    {
        Ok(_) => {
            let s3_url = format!("https://docuseal.s3.amazonaws.com/{}", s3_key); // Replace with your S3 URL

            // Create options with URL
            let mut options_value = serde_json::Value::Object(serde_json::Map::new());
            if let Some(opts) = options {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&opts) {
                    options_value = parsed;
                }
            }
            options_value["url"] = serde_json::Value::String(s3_url.clone());

            let create_field = CreateTemplateField {
                template_id,
                name,
                field_type,
                required,
                display_order,
                position: position.map(|p| serde_json::from_str(&p).unwrap_or(serde_json::Value::Null)),
                options: Some(options_value),
                metadata: None,
                partner: None, // No partner specified in file upload
            };

            match crate::database::queries::TemplateFieldQueries::create_template_field(pool, create_field).await {
                Ok(db_field) => {
                    let template_field = TemplateField {
                        id: db_field.id,
                        template_id: db_field.template_id,
                        name: db_field.name,
                        field_type: db_field.field_type,
                        required: db_field.required,
                        display_order: db_field.display_order,
                        position: db_field.position.and_then(|v| serde_json::from_value(v).ok()),
                        options: db_field.options,
                        partner: db_field.partner,
                        created_at: db_field.created_at,
                        updated_at: db_field.updated_at,
                    };
                    ApiResponse::created(template_field, "Template field created successfully".to_string())
                }
                Err(e) => ApiResponse::internal_error(format!("Failed to create template field: {}", e)),
            }
        }
        Err(e) => ApiResponse::internal_error(format!("Failed to upload file to S3: {}", e)),
    }
}

#[utoipa::path(
    put,
    path = "/api/templates/{template_id}/fields/{field_id}",
    params(
        ("template_id" = i64, Path, description = "Template ID"),
        ("field_id" = i64, Path, description = "Field ID")
    ),
    request_body = UpdateTemplateFieldRequest,
    responses(
        (status = 200, description = "Template field updated successfully", body = ApiResponse<TemplateField>),
        (status = 404, description = "Template field not found", body = ApiResponse<TemplateField>),
        (status = 500, description = "Internal server error", body = ApiResponse<TemplateField>)
    ),
    security(("bearer_auth" = [])),
    tag = "template_fields"
)]
pub async fn update_template_field(
    State(state): State<AppState>,
    Path((template_id, field_id)): Path<(i64, i64)>,
    Extension(user_id): Extension<i64>,
    Json(payload): Json<UpdateTemplateFieldRequest>,
) -> (StatusCode, Json<ApiResponse<TemplateField>>) {
    let pool = &state.lock().await.db_pool;

    // Verify user has permission to access this template
    match TemplateQueries::get_template_by_id(pool, template_id).await {
        Ok(Some(db_template)) => {
            // Get user role to check permissions
            match crate::database::queries::UserQueries::get_user_by_id(pool, user_id).await {
                Ok(Some(user)) => {
                    // Allow access if user is the owner OR if user has Editor/Admin role (Members have read-only access to others' templates)
                    let has_access = db_template.user_id == user_id || 
                                   matches!(user.role, crate::models::role::Role::Editor | crate::models::role::Role::Admin);
                    
                    if !has_access {
                        return ApiResponse::forbidden("Access denied: You do not have permission to modify this template".to_string());
                    }
                }
                _ => return ApiResponse::not_found("User not found".to_string()),
            }
        }
        Ok(None) => return ApiResponse::not_found("Template not found".to_string()),
        Err(e) => return ApiResponse::internal_error(format!("Failed to verify template: {}", e)),
    }

    let update_field = CreateTemplateField {
        template_id,
        name: payload.name.unwrap_or_else(|| "temp".to_string()),
        field_type: payload.field_type.unwrap_or_else(|| "text".to_string()),
        required: payload.required.unwrap_or(false),
        display_order: payload.display_order.unwrap_or(0),
        position: payload.position.map(|p| serde_json::to_value(p).unwrap_or(serde_json::Value::Null)),
        options: payload.options,
        metadata: None,
        partner: payload.partner,
    };

    match crate::database::queries::TemplateFieldQueries::update_template_field(pool, field_id, update_field).await {
        Ok(Some(db_field)) => {
            let template_field = TemplateField {
                id: db_field.id,
                template_id: db_field.template_id,
                name: db_field.name,
                field_type: db_field.field_type,
                required: db_field.required,
                display_order: db_field.display_order,
                position: db_field.position.and_then(|v| serde_json::from_value(v).ok()),
                options: db_field.options,
                partner: db_field.partner,
                created_at: db_field.created_at,
                updated_at: db_field.updated_at,
            };

            ApiResponse::success(template_field, "Template field updated successfully".to_string())
        }
        Ok(None) => ApiResponse::not_found("Template field not found".to_string()),
        Err(e) => ApiResponse::internal_error(format!("Failed to update template field: {}", e)),
    }
}

#[utoipa::path(
    delete,
    path = "/api/templates/{template_id}/fields/{field_id}",
    params(
        ("template_id" = i64, Path, description = "Template ID"),
        ("field_id" = i64, Path, description = "Field ID")
    ),
    responses(
        (status = 200, description = "Template field deleted successfully", body = ApiResponse<serde_json::Value>),
        (status = 404, description = "Template field not found", body = ApiResponse<serde_json::Value>),
        (status = 500, description = "Internal server error", body = ApiResponse<serde_json::Value>)
    ),
    security(("bearer_auth" = [])),
    tag = "template_fields"
)]
pub async fn delete_template_field(
    State(state): State<AppState>,
    Path((template_id, field_id)): Path<(i64, i64)>,
    Extension(user_id): Extension<i64>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let pool = &state.lock().await.db_pool;

    // Verify user has permission to access this template
    match TemplateQueries::get_template_by_id(pool, template_id).await {
        Ok(Some(db_template)) => {
            // Get user role to check permissions
            match crate::database::queries::UserQueries::get_user_by_id(pool, user_id).await {
                Ok(Some(user)) => {
                    // Allow access if user is the owner OR if user has Editor/Admin role (Members have read-only access to others' templates)
                    let has_access = db_template.user_id == user_id || 
                                   matches!(user.role, crate::models::role::Role::Editor | crate::models::role::Role::Admin);
                    
                    if !has_access {
                        return ApiResponse::forbidden("Access denied: You do not have permission to modify this template".to_string());
                    }
                }
                _ => return ApiResponse::not_found("User not found".to_string()),
            }
        }
        Ok(None) => return ApiResponse::not_found("Template not found".to_string()),
        Err(e) => return ApiResponse::internal_error(format!("Failed to verify template: {}", e)),
    }

    match crate::database::queries::TemplateFieldQueries::delete_template_field(pool, field_id).await {
        Ok(true) => ApiResponse::success(serde_json::json!({"deleted": true}), "Template field deleted successfully".to_string()),
        Ok(false) => ApiResponse::not_found("Template field not found".to_string()),
        Err(e) => ApiResponse::internal_error(format!("Failed to delete template field: {}", e)),
    }
}

// ===== PUBLIC FILE UPLOAD ENDPOINT (for signing) =====

#[utoipa::path(
    post,
    path = "/api/files/upload/public",
    request_body = FileUploadRequest,
    responses(
        (status = 201, description = "File uploaded successfully", body = ApiResponse<FileUploadResponse>),
        (status = 400, description = "Bad request - No file provided or invalid file type. Supported types: Images (JPG, PNG, GIF, WEBP, BMP, TIFF), Documents (PDF, DOCX, DOC, TXT, HTML, XLSX, XLS), Data (JSON, CSV, XML)", body = ApiResponse<FileUploadResponse>),
        (status = 500, description = "Internal server error", body = ApiResponse<FileUploadResponse>)
    ),
    tag = "files"
)]
pub async fn upload_file_public(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> (StatusCode, Json<ApiResponse<FileUploadResponse>>) {
    let _pool = &state.lock().await.db_pool;

    // Initialize storage service
    let storage = match StorageService::new().await {
        Ok(storage) => storage,
        Err(e) => return ApiResponse::internal_error(format!("Failed to initialize storage: {}", e)),
    };

    let mut file_data = Vec::new();
    let mut filename = String::new();
    let mut content_type = String::new();

    // Parse multipart form data
    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let field_name = field.name().unwrap_or("").to_string();

        match field_name.as_str() {
            "file" => {
                filename = field.file_name().unwrap_or("uploaded_file").to_string();
                
                // Determine content type from filename
                content_type = get_content_type_from_filename(&filename).to_string();
                
                file_data = field.bytes().await.unwrap_or_default().to_vec();
            }
            _ => {}
        }
    }

    if file_data.is_empty() {
        return ApiResponse::bad_request("File is required".to_string());
    }

    // Validate file type - allow multiple file types including images, documents, and PDFs
    let allowed_types = [
        // Images
        "image/jpeg",
        "image/png", 
        "image/gif",
        "image/webp",
        "image/bmp",
        "image/tiff",
        // Documents
        "application/pdf",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document", // DOCX
        "application/msword", // DOC
        "text/plain", // TXT
        "text/html", // HTML
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", // XLSX
        "application/vnd.ms-excel", // XLS
        // Other common types
        "application/json",
        "text/csv",
        "application/xml",
        "text/xml"
    ];

    if !allowed_types.contains(&content_type.as_str()) {
        return ApiResponse::bad_request(format!("File type not allowed. Supported types: Images (JPG, PNG, GIF, WEBP, BMP, TIFF), Documents (PDF, DOCX, DOC, TXT, HTML, XLSX, XLS), Data (JSON, CSV, XML). Detected type: {}", content_type));
    }

    // Upload file to storage
    let file_key = match storage.upload_file(file_data.clone(), &filename, &content_type).await {
        Ok(key) => key,
        Err(e) => return ApiResponse::internal_error(format!("Failed to upload file: {}", e)),
    };

    // Determine file type category
    let file_type = if content_type.starts_with("image/") {
        "image".to_string()
    } else if content_type == "application/pdf" {
        "pdf".to_string()
    } else if content_type.starts_with("application/vnd.openxmlformats-officedocument.wordprocessingml") || content_type == "application/msword" {
        "document".to_string()
    } else if content_type.starts_with("application/vnd.openxmlformats-officedocument.spreadsheetml") || content_type == "application/vnd.ms-excel" {
        "spreadsheet".to_string()
    } else if content_type == "text/plain" {
        "text".to_string()
    } else if content_type == "text/html" {
        "html".to_string()
    } else if content_type == "application/json" {
        "json".to_string()
    } else if content_type == "text/csv" {
        "csv".to_string()
    } else if content_type.contains("xml") {
        "xml".to_string()
    } else {
        "file".to_string()
    };

    // Generate file URL (direct S3 URL)
    let file_url = storage.get_public_url(&file_key);

    // Create response
    let upload_response = FileUploadResponse {
        id: file_key.clone(),
        filename: filename.clone(),
        file_type,
        file_size: file_data.len() as i64,
        url: file_url,
        content_type,
        uploaded_at: chrono::Utc::now(),
    };

    ApiResponse::created(upload_response, "File uploaded successfully".to_string())
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct DeleteFileRequest {
    pub file_url: String,
}

#[utoipa::path(
    delete,
    path = "/api/files/delete/public",
    request_body = DeleteFileRequest,
    responses(
        (status = 200, description = "File deleted successfully", body = ApiResponse<String>),
        (status = 400, description = "Bad request - Invalid file URL", body = ApiResponse<String>),
        (status = 500, description = "Internal server error", body = ApiResponse<String>)
    ),
    tag = "files"
)]
pub async fn delete_file_public(
    State(_state): State<AppState>,
    Json(payload): Json<DeleteFileRequest>,
) -> (StatusCode, Json<ApiResponse<String>>) {
    // Extract file key from URL
    // URL format: /api/files/templates/1234567890_filename.ext
    let file_key = if payload.file_url.starts_with("/api/files/") {
        payload.file_url.trim_start_matches("/api/files/")
    } else if payload.file_url.contains("/api/files/") {
        // Handle full URL
        payload.file_url.split("/api/files/").nth(1).unwrap_or("")
    } else {
        // Assume it's already a key
        &payload.file_url
    };

    if file_key.is_empty() {
        return ApiResponse::bad_request("Invalid file URL".to_string());
    }

    // Initialize storage service
    let storage = match StorageService::new().await {
        Ok(storage) => storage,
        Err(e) => return ApiResponse::internal_error(format!("Failed to initialize storage: {}", e)),
    };

    // Delete file from storage
    match storage.delete_file(file_key).await {
        Ok(_) => ApiResponse::success("File deleted successfully".to_string(), "File deleted successfully".to_string()),
        Err(e) => {
            eprintln!("Failed to delete file {}: {:?}", file_key, e);
            ApiResponse::internal_error(format!("Failed to delete file: {}", e))
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/files/upload",
    request_body = FileUploadRequest,
    responses(
        (status = 201, description = "File uploaded successfully", body = ApiResponse<FileUploadResponse>),
        (status = 400, description = "Bad request - No file provided or invalid file type", body = ApiResponse<FileUploadResponse>),
        (status = 500, description = "Internal server error", body = ApiResponse<FileUploadResponse>)
    ),
    security(("bearer_auth" = [])),
    tag = "files"
)]
pub async fn upload_file(
    State(state): State<AppState>,
    Extension(user_id): Extension<i64>,
    mut multipart: Multipart,
) -> (StatusCode, Json<ApiResponse<FileUploadResponse>>) {
    let _pool = &state.lock().await.db_pool;

    // Initialize storage service
    let storage = match StorageService::new().await {
        Ok(storage) => storage,
        Err(e) => return ApiResponse::internal_error(format!("Failed to initialize storage: {}", e)),
    };

    let mut file_data = Vec::new();
    let mut filename = String::new();
    let mut content_type = String::new();

    println!("🔍 [FILE UPLOAD] Starting to parse multipart data...");

    // Parse multipart form data
    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let field_name = field.name().unwrap_or("").to_string();
        let field_filename = field.file_name().map(|s| s.to_string());
        let field_content_type = field.content_type().map(|s| s.to_string());
        
        println!("📦 [FILE UPLOAD] Found field: {}", field_name);
        println!("   - filename: {:?}", field_filename);
        println!("   - content-type: {:?}", field_content_type);

        match field_name.as_str() {
            "file" => {
                filename = field_filename.unwrap_or_else(|| "uploaded_file".to_string());
                println!("📄 [FILE UPLOAD] Using filename: {}", filename);
                
                // Determine content type from filename
                content_type = get_content_type_from_filename(&filename).to_string();
                println!("📋 [FILE UPLOAD] Determined Content-Type: {}", content_type);
                
                // Read bytes from field
                file_data = field.bytes().await.unwrap_or_default().to_vec();
                println!("💾 [FILE UPLOAD] File size after read: {} bytes", file_data.len());
            }
            _ => {
                println!("⚠️ [FILE UPLOAD] Ignoring unknown field: {}", field_name);
            }
        }
    }

    println!("✅ [FILE UPLOAD] Parsing complete. File data size: {}", file_data.len());

    if file_data.is_empty() {
        println!("❌ [FILE UPLOAD] ERROR: No file data received!");
        return ApiResponse::bad_request("File is required".to_string());
    }

    // Validate file type - only allow PDF, DOCX, and images
    let allowed_types = [
        "application/pdf",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "application/msword",
        "image/jpeg",
        "image/png",
        "image/gif",
        "image/webp",
        "image/bmp",
        "image/tiff"
    ];

    if !allowed_types.contains(&content_type.as_str()) {
        return ApiResponse::bad_request(format!("File type not allowed. Supported types: PDF, DOCX, DOC, JPG, PNG, GIF, WEBP, BMP, TIFF. Detected type: {}", content_type));
    }

    // Upload file to storage
    let file_key = match storage.upload_file(file_data.clone(), &filename, &content_type).await {
        Ok(key) => key,
        Err(e) => return ApiResponse::internal_error(format!("Failed to upload file: {}", e)),
    };

    // Determine file type category
    let file_type = if content_type == "application/pdf" {
        "pdf".to_string()
    } else if content_type.starts_with("application/vnd.openxmlformats-officedocument.wordprocessingml") || content_type == "application/msword" {
        "document".to_string()
    } else if content_type.starts_with("image/") {
        "image".to_string()
    } else {
        "unknown".to_string()
    };

    // Generate file URL (direct S3 URL)
    let file_url = storage.get_public_url(&file_key);

    // Create response
    let upload_response = FileUploadResponse {
        id: file_key.clone(),
        filename: filename.clone(),
        file_type,
        file_size: file_data.len() as i64,
        url: file_url,
        content_type,
        uploaded_at: chrono::Utc::now(),
    };

    ApiResponse::created(upload_response, "File uploaded successfully".to_string())
}

// ===== CREATE TEMPLATE FROM UPLOADED FILE =====

#[utoipa::path(
    post,
    path = "/api/templates/from-file",
    request_body = CreateTemplateFromFileRequest,
    responses(
        (status = 201, description = "Template created from uploaded file", body = ApiResponse<Template>),
        (status = 400, description = "Bad request - Invalid file ID", body = ApiResponse<Template>),
        (status = 404, description = "File not found", body = ApiResponse<Template>),
        (status = 500, description = "Internal server error", body = ApiResponse<Template>)
    ),
    security(("bearer_auth" = [])),
    tag = "templates"
)]
pub async fn create_template_from_file(
    State(state): State<AppState>,
    Extension(user_id): Extension<i64>,
    Json(payload): Json<CreateTemplateFromFileRequest>,
) -> (StatusCode, Json<ApiResponse<Template>>) {
    let pool = &state.lock().await.db_pool;

    // Initialize storage service to verify file exists
    let storage = match StorageService::new().await {
        Ok(storage) => storage,
        Err(e) => return ApiResponse::internal_error(format!("Failed to initialize storage: {}", e)),
    };

    // Check if file exists in storage
    let file_exists = match storage.file_exists(&payload.file_id).await {
        Ok(exists) => exists,
        Err(e) => return ApiResponse::internal_error(format!("Failed to check file existence: {}", e)),
    };

    if !file_exists {
        return ApiResponse::not_found("File not found in storage".to_string());
    }

    // Determine content type from file extension
    let content_type = get_content_type_from_filename(&payload.file_id);
    
    // Generate unique slug
    let slug = format!("file-{}-{}", payload.name.to_lowercase().replace(" ", "-"), chrono::Utc::now().timestamp());
    
    // Get user's account_id
    let account_id = match crate::database::queries::UserQueries::get_user_by_id(pool, user_id).await {
        Ok(Some(user)) => user.account_id,
        Ok(None) => return ApiResponse::not_found("User not found".to_string()),
        Err(e) => return ApiResponse::internal_error(format!("Failed to get user: {}", e)),
    };
    
    // Create template in database
    let create_template = CreateTemplate {
        name: payload.name.clone(),
        slug: slug.clone(),
        user_id: user_id,
        account_id,
        folder_id: payload.folder_id,
        documents: Some(serde_json::json!([{
            "filename": payload.file_id.split('/').last().unwrap_or(&payload.file_id),
            "content_type": content_type,
            "size": 0, // TODO: Get actual file size
            "url": payload.file_id.clone()
        }])),
    };    match TemplateQueries::create_template(pool, create_template).await {
        Ok(db_template) => {
            match convert_db_template_to_template_with_fields(db_template, pool).await {
                Ok(template) => ApiResponse::created(template, "Template created from file successfully".to_string()),
                Err(e) => ApiResponse::internal_error(format!("Failed to load template fields: {}", e))
            }
        }
        Err(e) => ApiResponse::internal_error(format!("Failed to create template: {}", e))
    }
}