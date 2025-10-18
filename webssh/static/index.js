(() => {
  const term = new window.Terminal({
    convertEol: true,
    cursorBlink: true,
    fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace',
    theme: { background: '#111827' }
  });
  const fit = new window.FitAddon.FitAddon();
  const links = new window.WebLinksAddon.WebLinksAddon();
  term.loadAddon(fit);
  term.loadAddon(links);
  term.open(document.getElementById('terminal'));
  fit.fit();
  window.addEventListener('resize', () => fit.fit());

  const form = document.getElementById('connect-form');

  function wsURL() {
    const proto = location.protocol === 'https:' ? 'wss://' : 'ws://';
    return proto + location.host + '/api/ws';
  }

  let socket = null;
  function connect(sessionId) {
    socket = new WebSocket(wsURL());
    socket.addEventListener('open', () => {
      term.writeln('\u001b[1;32mConnected to server (stub)\u001b[0m');
      const hello = { type: 'init', sessionId, cols: term.cols, rows: term.rows };
      socket.send(JSON.stringify(hello));
      setTimeout(() => socket.send('ping'), 500);
    });
    socket.addEventListener('message', (evt) => {
      const data = evt.data;
      if (typeof data === 'string') {
        term.write(data + '\r\n');
      }
    });
    socket.addEventListener('close', () => {
      term.writeln('\r\n\u001b[1;31mDisconnected\u001b[0m');
    });

    term.onData((d) => {
      if (socket && socket.readyState === WebSocket.OPEN) {
        socket.send(d);
      }
    });
  }

  form.addEventListener('submit', async (e) => {
    e.preventDefault();
    const payload = {
      host: document.getElementById('host').value,
      port: parseInt(document.getElementById('port').value, 10) || 22,
      username: document.getElementById('username').value,
      password: document.getElementById('password').value,
    };

    try {
      const res = await fetch('/api/session', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload),
      });
      const data = await res.json();
      connect(data.id);
    } catch (err) {
      console.error(err);
      term.writeln('\u001b[1;31mFailed to create session\u001b[0m');
    }
  });
})();
