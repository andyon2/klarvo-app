/**
 * Platform detection for conditional UI rendering.
 * On Tauri Android, the WebView user agent contains "Android".
 * This is evaluated once at module load time -- no re-render overhead.
 */
export const isMobile = /Android|iPhone|iPad/i.test(navigator.userAgent);
export const isDesktop = !isMobile;
