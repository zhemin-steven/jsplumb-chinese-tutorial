export type AuthMethod = 'password' | 'key'

export interface SessionRequest {
  host: string
  port: number
  username: string
  authMethod: AuthMethod
  // Optional fields for later expansion
  password?: string
  privateKey?: string
}

export interface SessionResponse {
  token?: string
  wsUrl?: string
}

const API_BASE = '' // same-origin, prefix with /api in requests

export async function createSession(payload: SessionRequest): Promise<SessionResponse> {
  try {
    const res = await fetch(`/api/session`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload)
    })
    if (!res.ok) throw new Error(`HTTP ${res.status}`)
    return await res.json()
  } catch (e) {
    console.warn('createSession failed, falling back to placeholder token', e)
    return { token: 'PLACEHOLDER_TOKEN' }
  }
}

export function buildWsUrl(token: string): string {
  const protocol = location.protocol === 'https:' ? 'wss' : 'ws'
  const host = location.host
  const path = `/api/session/ws?token=${encodeURIComponent(token)}`
  return `${protocol}://${host}${path}`
}
