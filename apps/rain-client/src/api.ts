// Typed API Client and Formatters for RAIN Client

let hostUrl = localStorage.getItem('rain_host_url') || 'http://localhost:7890';
let authToken = localStorage.getItem('rain_auth_token') || '';

export function getHostUrl(): string {
  return hostUrl;
}

export function setHostUrl(url: string) {
  hostUrl = url.replace(/\/+$/, '');
  localStorage.setItem('rain_host_url', hostUrl);
}

export function getAuthToken(): string {
  return authToken;
}

export function setAuthToken(token: string) {
  authToken = token;
  localStorage.setItem('rain_auth_token', token);
}

export interface ApiResult<T> {
  data: T;
  latencyMs: number;
}

export async function apiRequest<T>(endpoint: string, options: RequestInit = {}): Promise<ApiResult<T>> {
  const base = hostUrl.replace(/\/+$/, '');
  const url = `${base}${endpoint}`;
  const start = performance.now();

  const headers = new Headers(options.headers || {});
  headers.set('Content-Type', 'application/json');
  if (authToken) {
    headers.set('Authorization', `Bearer ${authToken}`);
  }

  try {
    const res = await fetch(url, { ...options, headers });
    const latencyMs = Math.round(performance.now() - start);
    const json = await res.json();
    if (!json.success) {
      throw new Error(json.error || 'Request failed');
    }
    return { data: json.data as T, latencyMs };
  } catch (err: any) {
    throw err;
  }
}

// Utility Formatters
export function formatBytes(bytes: number): string {
  if (bytes <= 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${units[i]}`;
}

export function formatTime(isoStr: string | null): string {
  if (!isoStr) return '';
  const d = new Date(isoStr);
  if (isNaN(d.getTime())) return '';
  return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: true });
}

export function formatUptime(seconds: number): string {
  if (seconds <= 0) return '0s';
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

