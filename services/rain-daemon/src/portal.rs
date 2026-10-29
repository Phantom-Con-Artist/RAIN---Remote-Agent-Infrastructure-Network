use rain_core::RemoteManager;

pub(crate) fn render_hub_portal_html() -> String {
    let hostname = sysinfo::System::host_name().unwrap_or_else(|| "RAIN-Primary-Hub".to_string());
    let battery = RemoteManager::read_real_battery();
    let bat_str = match &battery {
        Some(b) if b.is_critical => format!("⚠️ {}% CRITICAL (DISCHARGING)", b.percent),
        Some(b) if b.is_ac_connected => format!("⚡ {}% ({})", b.percent, b.power_source),
        Some(b) => format!("🔋 {}% ({})", b.percent, b.power_source),
        None => "⚡ AC Power".to_string(),
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>RAIN Sovereign Hub — Active</title>
  <style>
    :root {{
      --bg: #090d16;
      --card-bg: #111827;
      --border: #1f2937;
      --accent: #38bdf8;
      --text: #f3f4f6;
      --text-muted: #9ca3af;
      --green: #10b981;
      --font-mono: 'JetBrains Mono', 'Fira Code', monospace;
    }}
    * {{ box-sizing: border-box; margin: 0; padding: 0; }}
    body {{
      background: var(--bg);
      color: var(--text);
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
      padding: 32px 20px;
      display: flex;
      justify-content: center;
      line-height: 1.5;
    }}
    .container {{ max-width: 860px; width: 100%; }}
    .header {{
      background: linear-gradient(180deg, rgba(31, 41, 55, 0.6) 0%, rgba(17, 24, 39, 0.8) 100%);
      border: 1px solid var(--border);
      border-radius: 16px;
      padding: 28px;
      margin-bottom: 24px;
      box-shadow: 0 8px 32px rgba(0,0,0,0.4);
    }}
    .brand-row {{
      display: flex;
      align-items: center;
      justify-content: space-between;
      flex-wrap: wrap;
      gap: 12px;
      margin-bottom: 12px;
    }}
    .logo {{
      font-size: 24px;
      font-weight: 800;
      letter-spacing: -0.5px;
      color: #fff;
      display: flex;
      align-items: center;
      gap: 10px;
    }}
    .badge-status {{
      display: inline-flex;
      align-items: center;
      gap: 6px;
      background: rgba(16, 185, 129, 0.15);
      color: var(--green);
      border: 1px solid rgba(16, 185, 129, 0.3);
      padding: 4px 12px;
      border-radius: 20px;
      font-size: 12px;
      font-weight: 700;
    }}
    .pulse-dot {{
      width: 8px;
      height: 8px;
      background: var(--green);
      border-radius: 50%;
      box-shadow: 0 0 10px var(--green);
    }}
    .subtitle {{ color: var(--text-muted); font-size: 14px; }}
    .grid {{
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
      gap: 16px;
      margin-bottom: 24px;
    }}
    .card {{
      background: var(--card-bg);
      border: 1px solid var(--border);
      border-radius: 12px;
      padding: 20px;
    }}
    .card-title {{
      font-size: 12px;
      font-weight: 600;
      color: var(--text-muted);
      text-transform: uppercase;
      letter-spacing: 0.5px;
      margin-bottom: 8px;
    }}
    .card-val {{ font-size: 17px; font-weight: 700; color: #fff; }}
    .action-box {{
      background: var(--card-bg);
      border: 1px solid var(--border);
      border-radius: 16px;
      padding: 24px;
      margin-bottom: 24px;
    }}
    .action-box h2 {{
      font-size: 18px;
      margin-bottom: 12px;
      display: flex;
      align-items: center;
      gap: 8px;
    }}
    .action-box p {{
      color: var(--text-muted);
      font-size: 14px;
      margin-bottom: 14px;
    }}
    .code-box {{
      background: #060911;
      border: 1px solid #1f293d;
      border-radius: 8px;
      padding: 12px 14px;
      font-family: var(--font-mono);
      font-size: 13px;
      color: #38bdf8;
      overflow-x: auto;
      margin-bottom: 14px;
      user-select: all;
    }}
    .btn-row {{ display: flex; gap: 10px; flex-wrap: wrap; }}
    .btn {{
      display: inline-flex;
      align-items: center;
      gap: 8px;
      padding: 10px 18px;
      border-radius: 8px;
      font-size: 13px;
      font-weight: 600;
      text-decoration: none;
      cursor: pointer;
      border: 1px solid transparent;
      transition: all 0.2s;
    }}
    .btn-primary {{ background: #0284c7; color: #fff; }}
    .btn-primary:hover {{ background: #0369a1; }}
    .btn-secondary {{ background: #1f2937; color: #e5e7eb; border-color: #374151; }}
    .btn-secondary:hover {{ background: #374151; }}
    .btn-danger {{
      background: rgba(239, 68, 68, 0.15);
      color: #ef4444;
      border-color: rgba(239, 68, 68, 0.3);
    }}
    .btn-danger:hover {{ background: rgba(239, 68, 68, 0.25); }}
    .links-list {{
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
      gap: 10px;
      margin-top: 12px;
    }}
    .api-link {{
      background: #090e1a;
      border: 1px solid #1f293d;
      padding: 10px;
      border-radius: 8px;
      color: #93c5fd;
      text-decoration: none;
      font-family: var(--font-mono);
      font-size: 12px;
      display: block;
      transition: all 0.2s;
    }}
    .api-link:hover {{
      background: #111a2e;
      border-color: #38bdf8;
      color: #fff;
    }}
  </style>
</head>
<body>
  <div class="container">
    <div class="header">
      <div class="brand-row">
        <div class="logo">🌧️ RAIN Hub Daemon</div>
        <div class="badge-status">
          <span class="pulse-dot"></span>
          HUB SERVER ONLINE (PORT 7890)
        </div>
      </div>
      <p class="subtitle">Remote Agent Infrastructure Network — Sovereign Star-Mesh Coordinator</p>
    </div>

    <div class="grid">
      <div class="card">
        <div class="card-title">Host Machine</div>
        <div class="card-val">{hostname}</div>
      </div>
      <div class="card">
        <div class="card-title">Hardware Power Telemetry</div>
        <div class="card-val">{bat_str}</div>
      </div>
      <div class="card">
        <div class="card-title">Topology Mode</div>
        <div class="card-val">Central Sovereign Hub</div>
      </div>
    </div>

    <div class="action-box">
      <h2>💻 Opening the RAIN Web Client Interface</h2>
      <p>The backend API daemon is running on port 7890. To interact with the full graphical workbench:</p>
      
      <p><strong>Option A — Start Vite Dev Server (Recommended for Live HMR):</strong></p>
      <div class="code-box">cd apps/rain-client && npm install && npm run dev</div>
      <div class="btn-row">
        <a href="http://localhost:5173" target="_blank" class="btn btn-primary">🔗 Open Web Client (http://localhost:5173)</a>
      </div>

      <div style="height: 18px;"></div>
      <p><strong>Option B — Serve Web Client Directly from this URL (Port 7890):</strong></p>
      <div class="code-box">cd apps/rain-client && npm install && npm run build</div>
      <p>Once built, visiting <code>http://localhost:7890/</code> in any browser automatically loads the full client app directly with zero extra dev servers!</p>
    </div>

    <div class="action-box">
      <h2>⚡ Connect a Second Machine (Spoke Worker Node)</h2>
      <p>Run this command on another laptop, desktop, or a second terminal to connect as a live spoke:</p>
      <div class="code-box">cargo run --bin rain-daemon -- --spoke --hub-url http://localhost:7890 --name "Worker-PC"</div>
    </div>

    <div class="action-box">
      <h2>🔋 Autonomous 5% Battery Evacuation Testing</h2>
      <p>Test the autonomous Git checkpoint and remote push failover pipeline:</p>
      <div class="btn-row">
        <button class="btn btn-danger" onclick="triggerEvac()">⚡ Trigger Autonomous Git Evacuation</button>
        <button class="btn btn-secondary" onclick="simulateBattery()">⚠️ Set Simulated 5% Battery</button>
        <button class="btn btn-secondary" onclick="resetBattery()">🔄 Reset Live Hardware Sensor</button>
      </div>
      <div id="action-output" style="margin-top: 12px; font-family: var(--font-mono); font-size: 13px; color: #38bdf8;"></div>
    </div>

    <div class="action-box">
      <h2>📡 Sovereign Hub REST Endpoints</h2>
      <div class="links-list">
        <a href="/api/v1/health" class="api-link" target="_blank">GET /api/v1/health</a>
        <a href="/api/v1/machines/overview" class="api-link" target="_blank">GET /api/v1/machines/overview</a>
        <a href="/api/v1/hub/topology" class="api-link" target="_blank">GET /api/v1/hub/topology</a>
        <a href="/api/v1/system/status" class="api-link" target="_blank">GET /api/v1/system/status</a>
        <a href="/api/v1/conversations" class="api-link" target="_blank">GET /api/v1/conversations</a>
        <a href="/api/v1/system/capabilities" class="api-link" target="_blank">GET /api/v1/system/capabilities</a>
      </div>
    </div>
  </div>

  <script>
    async function triggerEvac() {{
      const el = document.getElementById('action-output');
      el.innerText = 'Staging uncommitted files, committing, and pushing to GitHub...';
      try {{
        const res = await fetch('/api/v1/power/failover', {{ method: 'POST' }});
        const data = await res.json();
        el.innerText = '✅ Evacuation completed: ' + (data.data || JSON.stringify(data));
      }} catch(e) {{
        el.innerText = '❌ Error: ' + e.message;
      }}
    }}
    async function simulateBattery() {{
      const el = document.getElementById('action-output');
      const res = await fetch('/api/v1/power/simulate', {{ method: 'POST' }});
      const data = await res.json();
      el.innerText = '⚠️ ' + (data.data || '5% Battery simulation active');
    }}
    async function resetBattery() {{
      const el = document.getElementById('action-output');
      const res = await fetch('/api/v1/power/reset', {{ method: 'POST' }});
      const data = await res.json();
      el.innerText = '⚡ ' + (data.data || 'Sensor reset to live hardware');
    }}
  </script>
</body>
</html>"#,
        hostname = hostname,
        bat_str = bat_str
    )
}
