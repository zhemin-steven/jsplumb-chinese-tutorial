import React, { useState } from 'react'
import type { AuthMethod, SessionRequest } from '../api/client'

export interface ConnectionFormProps {
  onConnect: (req: SessionRequest) => void
  connecting?: boolean
}

export default function ConnectionForm({ onConnect, connecting }: ConnectionFormProps) {
  const [host, setHost] = useState('localhost')
  const [port, setPort] = useState(22)
  const [username, setUsername] = useState('root')
  const [authMethod, setAuthMethod] = useState<AuthMethod>('password')

  function submit(e: React.FormEvent) {
    e.preventDefault()
    onConnect({ host, port: Number(port), username, authMethod })
  }

  return (
    <form className="form-grid" onSubmit={submit}>
      <input
        type="text"
        placeholder="Host"
        value={host}
        onChange={(e) => setHost(e.target.value)}
      />
      <input
        type="number"
        placeholder="Port"
        value={port}
        onChange={(e) => setPort(Number(e.target.value))}
      />
      <input
        type="text"
        placeholder="Username"
        value={username}
        onChange={(e) => setUsername(e.target.value)}
      />
      <select value={authMethod} onChange={(e) => setAuthMethod(e.target.value as AuthMethod)}>
        <option value="password">Password</option>
        <option value="key">SSH Key</option>
      </select>
      <div style={{ gridColumn: 'span 3' }} />
      <button className="btn" type="submit" disabled={!!connecting}>
        {connecting ? 'Connecting…' : 'Connect'}
      </button>
    </form>
  )
}
