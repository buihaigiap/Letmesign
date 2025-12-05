import React, { useState, useEffect, useRef, forwardRef, useImperativeHandle } from "react";
import upstashService from "../ConfigApi/upstashService";
import { API_BASE_URL } from "@/config";
interface PdfDisplayProps {
  documentUrl?: string;
  filePath?: string;
  token?: string | null;
  scale?: number;
  page?: number;
  onPageChange?: (page: number) => void;
  onLoad?: () => void;
  onError?: (error: string) => void;
  children?: React.ReactNode; // For overlays
  globalSettings?: any; // Add globalSettings prop
}
export interface PdfDisplayRef {
  getDocState: () => DocumentState | null;
  getCanvasRef: () => HTMLCanvasElement | null;
  getOverlayRef: () => HTMLDivElement | null;
  getPageWidth: () => number;
  getPageHeight: () => number;
  getCanvasClientWidth: () => number;
  getCanvasClientHeight: () => number;
}
type DocumentState =
  | { type: "images"; content: string[]; currentPage: number; numPages: number }
  | { type: "image"; content: string }
  | { type: "unsupported"; content: string };

const PdfDisplay = forwardRef<PdfDisplayRef, PdfDisplayProps>(({
  documentUrl,
  filePath,
  token,
  page,
  onPageChange,
  onLoad,
  onError,
  children,
  globalSettings
}, ref) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const overlayRef = useRef<HTMLDivElement>(null);
  const imgRef = useRef<HTMLImageElement>(null);
  const [docState, setDocState] = useState<DocumentState | null>(null);
  const [error, setError] = useState("");
  const [thumbnails, setThumbnails] = useState<string[]>([]);
  const [pageWidth, setPageWidth] = useState(0);
  const [pageHeight, setPageHeight] = useState(0);
  const [loadingThumbnails, setLoadingThumbnails] = useState<Set<number>>(new Set());
  const [loadedThumbnails, setLoadedThumbnails] = useState<Set<number>>(new Set());
  const [loadingMainImage, setLoadingMainImage] = useState(false);

  // Hide scrollbar for thumbnail sidebar
  useEffect(() => {
    const style = document.createElement('style');
    style.textContent = `
      .thumbnail-sidebar::-webkit-scrollbar {
        display: none;
      }
    `;
    document.head.appendChild(style);
    return () => {
      if (document.head.contains(style)) {
        document.head.removeChild(style);
      }
    };
  }, []);
  useEffect(() => {
    if (docState?.type === 'images' && page && page !== docState.currentPage) {
      setDocState(prev => prev && prev.type === 'images' ? { ...prev, currentPage: page } : prev);
    }
  }, [page, docState]);

  useEffect(() => {
    let isCancelled = false;

    const loadDocument = async () => {
      try {
        setError("");
        setDocState(null);

        if (!documentUrl && !filePath) {
          throw new Error("Document URL or file path is required");
        }

        let response: any;

        if (token && !(filePath && (filePath.startsWith('http://') || filePath.startsWith('https://')))) {
          // Internal - use upstashService
          response = await upstashService.previewFile(filePath || documentUrl);
        } else if (documentUrl) {
          // External URL - use fetch
          const headers: HeadersInit = {
            'Accept': 'application/pdf,image/*,text/*,*/*'
          };

          response = await fetch(documentUrl, {
            headers,
            mode: 'cors',
          });
        } else if (filePath && (filePath.startsWith('http://') || filePath.startsWith('https://'))) {
          // External file path - use fetch
          const headers: HeadersInit = {
            'Accept': 'application/pdf,image/*,text/*,*/*'
          };

          response = await fetch(filePath, {
            headers,
            mode: 'cors',
          });
        } else if (filePath) {
          // Internal file path without token - try upstashService
          response = await upstashService.previewFile(filePath);
        } else {
          throw new Error("Invalid document source");
        }

        // Check for HTTP errors
        if (response.status && response.status >= 400) {
          const errorText = response.data ? new TextDecoder().decode(response.data).substring(0, 200) : 'Unknown error';
          throw new Error(`HTTP ${response.status}: ${response.statusText || 'Error'}\n${errorText}`);
        }

        // Create a unified response object for processing
        const unifiedResponse = {
          ok: response.status ? response.status >= 200 && response.status < 300 : true,
          status: response.status || 200,
          statusText: response.statusText || 'OK',
          headers: {
            get: (name: string) => {
              if (response.headers.get) {
                return response.headers.get(name);
              } else {
                // Axios response headers
                return response.headers[name.toLowerCase()] || response.headers[name] || null;
              }
            }
          },
          text: async () => {
            if (response.text) {
              return response.text();
            } else {
              // Axios response with JSON data
              return JSON.stringify(response.data);
            }
          }
        };

        // Handle content type detection
        let contentType = "";

        contentType = unifiedResponse.headers.get("content-type") || "";

        console.log("Content-Type:", contentType);
        console.log("Response data:", response.data);

        if (contentType.includes("application/json") || contentType === "application/pdf") {
          await loadImages(response);
        } else if (contentType.startsWith("image/")) {
          await loadImage(filePath || documentUrl);
        } else {
          await loadUnsupported(unifiedResponse, contentType);
        }

      } catch (err: any) {
        if (isCancelled) return;
        console.error("Document load error:", err);
        setError(err.message || "Failed to load document");
        onError?.(err.message || "Failed to load document");
      } 
    };

    const loadImages = async (response: any) => {
      const data = response.data;
      
      // Handle single image response (new format)
      if (data && data.type === "image" && data.url) {
        const fullUrl = data.url.startsWith('http') ? data.url : `${API_BASE_URL}${data.url}`;
        setDocState({ type: "image", content: fullUrl });
        // Don't call onLoad here - wait for image to actually load
      }
      // Handle multi-page PDF response
      else if (data && data.pages && Array.isArray(data.pages)) {
        const fullUrls = data.pages.map((url: string) => url.startsWith('http') ? url : `${API_BASE_URL}${url}`);
        setDocState({ type: "images", content: fullUrls, currentPage: 1, numPages: data.total_pages });
        setThumbnails(fullUrls);
        // Don't call onLoad here - wait for image to actually load
      } else {
        throw new Error("Invalid image preview response");
      }
    };

    const loadImage = async (imageUrl: string) => {
      const fullUrl = imageUrl.startsWith('http') ? imageUrl : `${API_BASE_URL}${imageUrl}`;
      setDocState({ type: "image", content: fullUrl });
      // Don't call onLoad here - wait for image to actually load
    };

    const loadUnsupported = async (response: any, contentType: string) => {
      const textPreview = await response.text();
      if (isCancelled) return;
      setDocState({
        type: 'unsupported',
        content: `Unsupported content type: ${contentType}. Preview: ${textPreview.substring(0,200)}`
      });
      onLoad?.();
    };

    if (documentUrl || filePath) {
      loadDocument();
    }

    return () => {
      isCancelled = true;
    };
  }, [documentUrl, filePath, token]);

  useImperativeHandle(ref, () => ({
    getDocState: () => docState,
    getCanvasRef: () => canvasRef.current,
    getOverlayRef: () => overlayRef.current,
    getPageWidth: () => pageWidth,
    getPageHeight: () => pageHeight,
    getCanvasClientWidth: () => {
      if (!imgRef.current || !overlayRef.current) return 0;
      const img = imgRef.current;
      const container = overlayRef.current;
      const containerRatio = container.clientWidth / container.clientHeight;
      const imageRatio = img.naturalWidth / img.naturalHeight;
      
      if (containerRatio > imageRatio) {
        // Container is wider than image ratio, image will be height-constrained
        return container.clientHeight * imageRatio;
      } else {
        // Container is taller than image ratio, image will be width-constrained
        return container.clientWidth;
      }
    },
    getCanvasClientHeight: () => {
      if (!imgRef.current || !overlayRef.current) return 0;
      const img = imgRef.current;
      const container = overlayRef.current;
      const containerRatio = container.clientWidth / container.clientHeight;
      const imageRatio = img.naturalWidth / img.naturalHeight;
      
      if (containerRatio > imageRatio) {
        // Container is wider than image ratio, image will be height-constrained
        return container.clientHeight;
      } else {
        // Container is taller than image ratio, image will be width-constrained
        return container.clientWidth / imageRatio;
      }
    },
  }));

  const renderContent = () => {
    
    // Show loading when no docState yet
    if (!docState && !error) {
      return (
        <div className="flex items-center justify-center min-h-[400px] bg-gray-900 rounded-lg">
          <div className="text-center">
            <div className="animate-spin rounded-full h-16 w-16 border-b-4 border-blue-500 mx-auto mb-4"></div>
            <p className="text-white text-sm">Loading document...</p>
          </div>
        </div>
      );
    }
    
    switch(docState?.type) {
      case 'images':
        const currentImageUrl = docState.content[docState.currentPage - 1];
        return (
          <div className="w-full   ">
            <div className="relative inline-block">
              {/* Loading overlay for main image */}
              {loadingMainImage && (
                <div className="absolute inset-0 flex items-center justify-center bg-gray-900 bg-opacity-75 rounded-lg z-20">
                  <div className="text-center">
                    <div className="animate-spin rounded-full h-16 w-16 border-b-4 border-blue-500 mx-auto mb-4"></div>
                    <p className="text-white text-sm">Loading page {docState.currentPage}...</p>
                  </div>
                </div>
              )}
              
              <img
                ref={imgRef}
                src={currentImageUrl}
                alt={`Page ${docState.currentPage}`}
                className="w-full h-auto object-contain shadow-lg"
                onLoadStart={() => setLoadingMainImage(true)}
                onLoad={(e) => {
                  const img = e.target as HTMLImageElement;
                  setPageWidth(img.naturalWidth || img.clientWidth);
                  setPageHeight(img.naturalHeight || img.clientHeight );
                  setLoadingMainImage(false);
                  onLoad?.(); // Call onLoad AFTER image dimensions are available
                }}
                onError={() => setLoadingMainImage(false)}
              />
              <div ref={overlayRef} className="absolute top-0 left-0 w-full h-full z-10">
                {/* Company Logo and Name Overlay */}
                {globalSettings && (globalSettings.logo_url || globalSettings.company_name) && (
                  <div className="absolute top-4 left-4 right-4 z-20 flex items-center gap-4  p-3 ">
                    {globalSettings.logo_url && (
                      <img
                        src={globalSettings.logo_url.startsWith('http') ? globalSettings.logo_url : `${API_BASE_URL}${globalSettings.logo_url}`}
                        alt="Company Logo"
                        className="h-12 w-auto object-contain"
                        onError={(e) => {
                          // Hide logo if it fails to load
                          (e.target as HTMLImageElement).style.display = 'none';
                        }}
                      />
                    )}
                    {globalSettings.company_name && (
                      <div className="text-lg font-bold text-gray-800">
                        {globalSettings.company_name}
                      </div>
                    )}
                  </div>
                )}
                {children}
              </div>
            </div>
          </div>
        );
      case 'image':
        return (
          <div className="w-full">
            <div className="relative inline-block">
              {/* Loading overlay for single image */}
              {loadingMainImage && (
                <div className="absolute inset-0 flex items-center justify-center bg-gray-900 bg-opacity-75 rounded-lg z-20 min-h-[400px]">
                  <div className="text-center">
                    <div className="animate-spin rounded-full h-16 w-16 border-b-4 border-blue-500 mx-auto mb-4"></div>
                    <p className="text-white text-sm">Loading document...</p>
                  </div>
                </div>
              )}
              
              <img
                ref={imgRef}
                src={docState.content}
                alt="Document"
                className="w-full h-auto object-contain shadow-lg"
                onLoadStart={() => setLoadingMainImage(true)}
                onLoad={(e) => {
                  const img = e.target as HTMLImageElement;
                  setPageWidth(img.naturalWidth || img.clientWidth || 600);
                  setPageHeight(img.naturalHeight || img.clientHeight || 800);
                  setLoadingMainImage(false);
                  onLoad?.(); // Call onLoad AFTER image dimensions are available
                }}
                onError={() => setLoadingMainImage(false)}
              />
              <div ref={overlayRef} className="absolute top-0 left-0 w-full h-full z-10">
                {/* Company Logo and Name Overlay */}
                {globalSettings && (globalSettings.logo_url || globalSettings.company_name) && (
                  <div className="absolute top-4 left-4 right-4 z-20 flex items-center gap-4 bg-white bg-opacity-90 p-3 rounded-lg shadow-md">
                    {globalSettings.logo_url && (
                      <img
                        src={globalSettings.logo_url.startsWith('http') ? globalSettings.logo_url : `${API_BASE_URL}${globalSettings.logo_url}`}
                        alt="Company Logo"
                        className="h-12 w-auto object-contain"
                        onError={(e) => {
                          // Hide logo if it fails to load
                          (e.target as HTMLImageElement).style.display = 'none';
                        }}
                      />
                    )}
                    {globalSettings.company_name && (
                      <div className="text-lg font-bold text-gray-800">
                        {globalSettings.company_name}
                      </div>
                    )}
                  </div>
                )}
                {children}
              </div>
            </div>
          </div>
        );
      case 'unsupported':
        return (
          <div className="w-full border border-yellow-500 bg-yellow-50 p-4 rounded">
            <p className="text-yellow-800 font-semibold mb-2">⚠️ Unsupported Content</p>
            <p className="text-yellow-700 text-sm font-mono">{docState.content}</p>
          </div>
        );
      default:
        return null;
    }
  };

  if (error) {
    return (
      <div className="flex flex-col justify-center items-center h-full min-h-[400px] bg-gray-800 rounded-lg p-4 text-center">
        <div className="text-red-400 mb-4">
          <svg className="w-12 h-12 mx-auto mb-2" fill="currentColor" viewBox="0 0 20 20">
            <path fillRule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z" clipRule="evenodd" />
          </svg>
        </div>
        <p className="text-red-400 font-semibold mb-2">Error Loading Document</p>
        <p className="text-red-500 text-sm bg-gray-900 p-3 rounded max-w-md break-words">{error}</p>
      </div>
    );
  }

  return (
    <div className="flex h-full min-h-[80vh]">
      {/* Thumbnail Sidebar */}
      {docState?.type === 'images' && thumbnails.length > 0 && (
        <div className="hidden md:block w-40 overflow-y-auto max-h-[80vh] thumbnail-sidebar" style={{ scrollbarWidth: 'none', msOverflowStyle: 'none' }}>
          <div className="px-2">
            <h3 className="text-md font-semibold mb-2">Pages</h3>
            <div className="space-y-2">
              {thumbnails.map((thumbnail, index) => (
                <div
                  key={index}
                  onClick={() => onPageChange && onPageChange(index + 1)}
                  className={`cursor-pointer border-2 rounded transition-colors relative ${
                    (page || docState.currentPage) === index + 1
                      ? 'border-blue-500 bg-blue-500 bg-opacity-20'
                      : 'border-gray-600 hover:border-gray-500'
                  }`}
                >
                  {/* Loading overlay for thumbnail */}
                  {loadingThumbnails.has(index) && (
                    <div className="absolute inset-0 flex items-center justify-center bg-gray-800 bg-opacity-75 rounded z-10">
                      <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
                    </div>
                  )}
                  
                  <img
                    src={thumbnail}
                    alt={`Page ${index + 1}`}
                    className="w-full h-auto"
                    onLoadStart={() => {
                      setLoadingThumbnails(prev => new Set(prev).add(index));
                    }}
                    onLoad={() => {
                      setLoadedThumbnails(prev => new Set(prev).add(index));
                      setLoadingThumbnails(prev => {
                        const newSet = new Set(prev);
                        newSet.delete(index);
                        return newSet;
                      });
                    }}
                    onError={() => {
                      setLoadingThumbnails(prev => {
                        const newSet = new Set(prev);
                        newSet.delete(index);
                        return newSet;
                      });
                    }}
                  />
                  <div className="text-center text-xs text-gray-400 py-1">
                    {index + 1}
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}

      {/* Main Content */}
      <div className="flex-1 rounded-lg justify-center items-start min-h-[80vh] relative shadow-inner">
          {renderContent()}
      </div>
    </div>
  );
});

export default PdfDisplay;