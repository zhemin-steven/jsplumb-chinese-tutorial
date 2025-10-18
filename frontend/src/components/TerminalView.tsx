import React, { useEffect, useMemo, useRef, useState } from 'react'
import { Terminal as XTerm } from 'xterm'
import { FitAddon } from 'xterm-addon-fit'
import 'xterm/css/xterm.css'

export interface TerminalViewProps {
  ws?: WebSocket | null
}

export default function TerminalView({ ws }: TerminalViewProps) {
  const containerRef = useRef<HTMLDivElement | null>(null)
  const wrapperRef = useRef<HTMLDivElement | null>(null)
  const termRef = useRef<XTerm | null>(null)
  const fitAddon = useMemo(() => new FitAddon(), [])
  const [isFullscreen, setIsFullscreen] = useState(false)

  // Initialize terminal
  useEffect(() => {
    if (!containerRef.current) return

    const term = new XTerm({
      fontFamily: "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace",
      fontSize: 14,
      cursorBlink: true,
      scrollback: 5000,
      theme: { background: '#1e1e1e' },
    })
    termRef.current = term
    term.loadAddon(fitAddon)
    term.options.bracketedPasteMode = true

    term.open(containerRef.current)
    fitAddon.fit()

    term.focus()

    return () => {
      term.dispose()
      termRef.current = null
    }
  }, [fitAddon])

  // Attach websocket
  useEffect(() => {
    const term = termRef.current
    if (!term) return

    function handleIncoming(ev: MessageEvent) {
      const data = ev.data
      if (data instanceof Blob) {
        data.text().then((t) => term.write(t))
      } else if (data instanceof ArrayBuffer) {
        const t = new TextDecoder().decode(new Uint8Array(data))
        term.write(t)
      } else {
        term.write(String(data))
      }
    }

    function handleTermData(data: string) {
      ws?.send(data)
    }

    ws?.addEventListener('message', handleIncoming)
    const disposeOnData = term.onData(handleTermData)

    // Send initial size using public cols/rows
    sendResize()

    function sendResize() {
      if (!ws || !term) return
      const cols = term.cols
      const rows = term.rows
      const payload = { t: 'resize', cols, rows }
      try { ws.send(JSON.stringify(payload)) } catch {}
    }

    // Fit on resize
    const onWindowResize = () => {
      try { fitAddon.fit() } catch {}
      sendResize()
    }

    window.addEventListener('resize', onWindowResize)
    document.addEventListener('fullscreenchange', onWindowResize)

    // Observe container size changes
    const ro = new ResizeObserver(onWindowResize)
    if (wrapperRef.current) ro.observe(wrapperRef.current)

    return () => {
      window.removeEventListener('resize', onWindowResize)
      document.removeEventListener('fullscreenchange', onWindowResize)
      ro.disconnect()
      ws?.removeEventListener('message', handleIncoming)
      disposeOnData.dispose()
    }
  }, [ws, fitAddon])

  function toggleFullscreen() {
    const el = wrapperRef.current
    if (!el) return

    // Use Fullscreen API for true fullscreen when available
    if (!document.fullscreenElement) {
      el.classList.add('fullscreen')
      el.requestFullscreen?.().catch(() => {/* ignore */})
      setIsFullscreen(true)
    } else {
      document.exitFullscreen?.().catch(() => {/* ignore */})
      el.classList.remove('fullscreen')
      setIsFullscreen(false)
    }
    // Fit after transition
    setTimeout(() => {
      try { fitAddon.fit() } catch {}
    }, 50)
  }

  return (
    <div ref={wrapperRef} className="terminal-wrapper">
      <div className="terminal-actions">
        <button className="btn" onClick={toggleFullscreen}>{isFullscreen ? 'Exit Fullscreen' : 'Fullscreen'}</button>
      </div>
      <div ref={containerRef} className="terminal-container" />
    </div>
  )
}
