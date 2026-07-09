/** Mirror StringUtils::formatFileSize (1 decimal, B..TB). */
export function formatFileSize(bytes: number): string {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let size = bytes;
  let unit = 0;
  while (size >= 1024 && unit < 4) { size /= 1024; unit++; }
  return `${size.toFixed(1)} ${units[unit]}`;
}
