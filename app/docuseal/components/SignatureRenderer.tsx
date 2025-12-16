import { useBasicSettings } from '@/hooks/useBasicSettings';
import React, { useMemo } from 'react';
import { hashId } from '../constants/reminderDurations';
import { API_BASE_URL } from '../config';

interface SignatureRendererProps {
  data: string; // JSON string of point groups or typed text
  width?: number;
  height?: number;
  fieldType?: string;
  color?: string; // Color for signature/text
  additionalText?: string; // Additional text to display below the signature
  submitterId?: number;
  submitterEmail?: string;
  reason?: string; // Signing reason to display
  globalSettings?: any;
}

const SignatureRenderer: React.FC<SignatureRendererProps> = ({
  data,
  width = 200,
  height = 100,
  fieldType,
  color = '#000000',
  additionalText,
  submitterId,
  submitterEmail,
  reason, globalSettings
}) => {
  // 1. Parse Data Type
  const { type, content, vectorBounds } = useMemo(() => {
    if (!data) return { type: 'EMPTY', content: null };
    const trimmedData = data.trim();
    if (trimmedData.startsWith('http') || trimmedData.startsWith('/') || trimmedData.startsWith('blob:') || trimmedData.startsWith('data:')) {
      return { type: 'IMAGE', content: trimmedData };
    }

    try {
      const json = JSON.parse(data);
      if (Array.isArray(json) && json.length > 0) {
        // Calculate bounds for Vector
        let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
        json.forEach((group: any[]) => {
          group.forEach((p: any) => {
            minX = Math.min(minX, p.x);
            minY = Math.min(minY, p.y);
            maxX = Math.max(maxX, p.x);
            maxY = Math.max(maxY, p.y);
          });
        });
        return {
          type: 'VECTOR',
          content: json,
          vectorBounds: { minX, minY, width: maxX - minX, height: maxY - minY }
        };
      }
    } catch (e) {
      // Ignore
    }

    // Fallback: If it's a very long string with no spaces, it might be a base64 image without prefix
    if (trimmedData.length > 100 && !trimmedData.includes(' ')) {
      // Try to prepend data:image/png;base64, if it looks like base64
      return { type: 'IMAGE', content: `data:image/png;base64,${trimmedData}` };
    }

    return { type: 'TEXT', content: data };
  }, [data]);

  // 2. Prepare Metadata Text
  const metadataLines = useMemo(() => {
    // Don't show metadata for initials
    if (fieldType === 'initials') {
      return [];
    }
    
    const timeZoneMap: Record<string, string> = {
      "Midway Island": "Pacific/Midway",
      "Hawaii": "Pacific/Honolulu",
      "Alaska": "America/Anchorage",
      "Pacific": "America/Los_Angeles",
      "Mountain": "America/Denver",
      "Central": "America/Chicago",
      "Eastern": "America/New_York",
      "Atlantic": "America/Halifax",
      "Newfoundland": "America/St_Johns",
      "London": "Europe/London",
      "Berlin": "Europe/Berlin",
      "Paris": "Europe/Paris",
      "Rome": "Europe/Rome",
      "Moscow": "Europe/Moscow",
      "Tokyo": "Asia/Tokyo",
      "Shanghai": "Asia/Shanghai",
      "Hong Kong": "Asia/Hong_Kong",
      "Singapore": "Asia/Singapore",
      "Sydney": "Australia/Sydney",
      "UTC": "UTC"
    };

    let timeZone = 'Asia/Ho_Chi_Minh';
    const configuredTimeZone = globalSettings?.timezone;
    if (configuredTimeZone) {
      const mappedTimeZone = timeZoneMap[configuredTimeZone] || configuredTimeZone;
      try {
        new Intl.DateTimeFormat('en', { timeZone: mappedTimeZone });
        timeZone = mappedTimeZone;
      } catch {
        // Invalid time zone
      }
    }
    const locale = globalSettings?.locale || 'vi-VN';
    const dateOptions: Intl.DateTimeFormatOptions = {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
      timeZone
    };

    let lines: string[] = [];

    if (globalSettings?.add_signature_id_to_the_documents) {
      if (globalSettings?.require_signing_reason && reason) {
        lines.push(`Reason: ${reason}`);
      }
      if (submitterId) lines.push(`ID: ${hashId(submitterId + 1)}`);
      if (submitterEmail) lines.push(submitterEmail);
      lines.push(new Date().toLocaleString(locale, dateOptions));
    } else {
      if (globalSettings?.require_signing_reason && reason) {
        lines.push(`Reason: ${reason}`);
      } else if (additionalText) {
        lines.push(additionalText);
      }
    }

    return lines;
  }, [globalSettings, additionalText, submitterId, submitterEmail, reason]);

  // Render Helpers
  const renderContent = () => {
    if (type === 'IMAGE') {
      const src = (content as string).startsWith('http') || (content as string).startsWith('data:') ? content as string : API_BASE_URL + (content as string);
      return (
        <img
          src={src}
          alt="Signature"
          style={{
            maxWidth: '100%',
            maxHeight: '100%',
            objectFit: 'contain',
            display: 'block'
          }}
        />
      );
    }

    if (type === 'VECTOR') {
      const { minX, minY, width: vWidth, height: vHeight } = vectorBounds!;
      // Add some padding to viewbox
      const padding = 2;
      const vbX = minX - padding;
      const vbY = minY - padding;
      const vbW = vWidth + padding * 2;
      const vbH = vHeight + padding * 2;

      return (
        <svg
          viewBox={`${vbX} ${vbY} ${vbW} ${vbH}`}
          style={{ width: '100%', height: '100%', overflow: 'visible' }}
          preserveAspectRatio="xMidYMid meet"
        >
          {(content as any[]).map((group, i) => (
            <path
              key={i}
              d={`M ${group.map((p: any) => `${p.x} ${p.y}`).join(' L ')}`}
              fill="none"
              stroke={color}
              strokeWidth="2.5"
              strokeLinecap="round"
              strokeLinejoin="round"
              vectorEffect="non-scaling-stroke"
            />
          ))}
        </svg>
      );
    }

    if (type === 'TEXT') {
      const isInitials = fieldType === 'initials';
      const fontStyle = isInitials ? 'italic' : 'normal';
      const fontFamily = isInitials ? 'Helvetica, sans-serif' : 'sans-serif';
      
      const textLength = (content as string).length;
      const baseFontSize = 100;
      let fontSize = `${baseFontSize}px`;
      
      const lineHeightFactor = isInitials ? 1.2 : 1.0;
      const textHeight = baseFontSize * lineHeightFactor; 
      const scaleY = height / textHeight;
      
      const charWidthFactor = isInitials ? 0.6 : 0.6;
      const textWidth = textLength * baseFontSize * charWidthFactor;
      const scaleX = width / textWidth;
      
      const scale = Math.min(scaleX, scaleY);
      const transform = `scale(${scale})`;

      return (
        <div style={{
          fontFamily,
          fontStyle,
          fontSize,
          color,
          textAlign: 'center',
          whiteSpace: 'nowrap',
          lineHeight: isInitials ? '1.2' : 'normal', // Changed to match lineHeightFactor
          fontWeight: isInitials ? 'bold' : 'normal',
          overflow: 'visible',
          width: '100%',
          height: '100%',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          transform,
          transformOrigin: 'center center'
        }}>
          {content as string}
        </div>
      );
    }

    return null;
  };

  return (
    <div style={{
      width: '100%',
      height: '100%',
      display: 'flex',
      flexDirection: 'column',
      overflow: 'hidden',
      position: 'relative'
    }}>
      <div style={{
        flex: 1,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        overflow: 'hidden',
        minHeight: 0 // Important for flex child overflow
      }}>
        {renderContent()}
      </div>

      {metadataLines.length > 0 && (
        <div style={{
          marginTop: '2px',
          fontSize: '9px',
          lineHeight: '10px',
          color,
          textAlign: 'left',
          paddingLeft: '5px',
          flexShrink: 0
        }}>
          {metadataLines.map((line, i) => (
            <div key={i}>{line}</div>
          ))}
        </div>
      )}
    </div>
  );
};

export default SignatureRenderer;