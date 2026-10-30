// Typed API Client and Formatters for RAIN Client

export function normalizeHostUrl(input: string): string {
  let url: URL;
  try { url = new URL(input.trim()); } catch { throw new Error('Enter a complete hub address, such as https://hub.example.'); }
  if (!['http:', 'https:'].includes(url.protocol) || !url.hostname) throw new Error('Use an HTTPS hub address.');
  if (url.username || url.password || url.search || url.hash || !['', '/'].includes(url.pathname)) throw new Error('Use only the hub address, without credentials, a path, or a query.');
  if (url.protocol === 'http:' && !['localhost', '127.0.0.1', '[::1]'].includes(url.hostname)) throw new Error('Remote hubs require HTTPS. HTTP is allowed only on this machine.');
  return url.origin;
}

function getDefaultHostUrl(): string {
  const origin = window.location.origin;
  const webPage = /^https?:/.test(origin);
  // A remotely served workspace always talks to its own authenticated gateway.
  if (webPage && !['localhost', '127.0.0.1', '[::1]'].includes(window.location.hostname)) return origin;
  const saved = localStorage.getItem('rain_host_url');
  if (saved) { try { return normalizeHostUrl(saved); } catch { localStorage.removeItem('rain_host_url'); } }
  return webPage ? origin : 'http://localhost:7890';
}

let hostUrl = getDefaultHostUrl();
// Never reuse a device token under a different hub identity.
let authToken = localStorage.getItem('rain_auth_host') === hostUrl ? localStorage.getItem('rain_auth_token') || '' : '';

export function getHostUrl(): string { return hostUrl; }
export function setHostUrl(input: string) {
  const next = normalizeHostUrl(input);
  if (next !== hostUrl) setAuthToken('');
  hostUrl = next;
  localStorage.setItem('rain_host_url', hostUrl);
}

export function getAuthToken(): string {
  return authToken;
}

export function setAuthToken(token: string) {
  authToken = token;
  localStorage.setItem('rain_auth_token', token);
  localStorage.setItem('rain_auth_host', hostUrl);
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
    headers.set('X-Rain-Device-Token', authToken);
  }

  try {
    const res = await fetch(url, { ...options, headers, signal: options.signal || AbortSignal.timeout(25000) });
    const latencyMs = Math.round(performance.now() - start);
    if (base !== hostUrl) throw new Error('Hub changed. Refresh before continuing.');
    if (res.status === 401) throw new Error('Sign in to your private hub in the browser, then retry.');
    if (res.status === 403) throw new Error('Access denied. Check your private connection and hub access settings.');
    if (!res.headers.get('content-type')?.includes('application/json')) throw new Error('This address did not return a RAIN response. Open the hub address and check its sign-in page.');
    const json = await res.json();
    if (res.status === 404 && endpoint.startsWith('/api/v1/agents/')) throw new Error('The host does not support this action or the session was removed. Refresh the page; restart the daemon if it is running an older build.');
    if (!res.ok || !json.success) {
      throw new Error(json.error || 'Request failed');
    }
    return { data: json.data as T, latencyMs };
  } catch (err: any) {
    if (err.name === 'TimeoutError') throw new Error('The hub took too long to respond. Check your connection and retry.');
    if (err instanceof TypeError) throw new Error('Cannot reach your hub. Check that it is running and your private connection is active.');
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

