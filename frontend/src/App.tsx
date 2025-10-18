import React, { useMemo, useRef, useState } from 'react'
import ConnectionForm from './components/ConnectionForm'
import TerminalView from './components/TerminalView'
import { buildWsUrl, createSession, type SessionRequest } from './api/client'

export default function App() {
  const [connecting, setConnecting] = useState(false)
  const [ws, setWs] = useState<WebSocket | null>(null)

  async function onConnect(req: SessionRequest) {
    setConnecting(true)
    try {
      const res = await createSession(req)
      const token = res.token ?? 'PLACEHOLDER_TOKEN'
      const wsUrl = res.wsUrl ?? buildWsUrl(token)
      if (ws) {
        try { ws.close() } catch {}
      }
      const socket = new WebSocket(wsUrl)
      socket.binaryType = 'arraybuffer'
      socket.onopen = () => {
        console.log('WebSocket connected')
      }
      socket.onclose = () => {
        console.log('WebSocket closed')
      }
      setWs(socket)
    } catch (e) {
      console.error('Failed to connect', e)
    } finally {
      setConnecting(false)
    }
  }

  return (
    <div className="app-container">
      <header className="header">
        <ConnectionForm onConnect={onConnect} connecting={connecting} />
      </header>
      <TerminalView ws={ws} />
    </div>
  )
}
