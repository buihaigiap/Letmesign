/**
 * Utility functions for the application
 */

/**
 * Masks an API key for display purposes, showing only the first 8 characters
 * and replacing the rest with asterisks
 * @param apiKey - The full API key to mask
 * @returns The masked API key string
 */
export const maskApiKey = (apiKey: string): string => {
  if (!apiKey || apiKey.length <= 8) {
    return apiKey;
  }

  const visibleChars = 8;
  const maskedChars = apiKey.length - visibleChars;
  const visiblePart = apiKey.substring(0, visibleChars);
  const maskedPart = '*'.repeat(maskedChars);

  return visiblePart + maskedPart;
};