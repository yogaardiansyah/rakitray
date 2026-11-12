import { invoke } from "@tauri-apps/api/core";
import { createSignal, onMount } from "solid-js";

export function App() {
  const [version, setVersion] = createSignal("0.1.0");
  const [systemInfo, setSystemInfo] = createSignal("Initializing...");

  onMount(async () => {
    try {
      const ver = await invoke<string>("get_app_version");
      setVersion(ver);
      const info = await invoke<string>("get_system_info");
      setSystemInfo(info);
    } catch {
      setSystemInfo("Local development mode (Rust core ready)");
    }
  });

  return (
    <div class="container">
      <header class="header">
        <div>
          <h1>Rakitray</h1>
          <p>Ultra-fast, modular Local Development Environment</p>
        </div>
        <span class="badge">v{version()}</span>
      </header>

      <main>
        <div class="card-grid">
          <div class="card">
            <h3>Supervisor Core</h3>
            <p>Win32 Job Objects & Unix Process Groups preventing zombie processes.</p>
          </div>
          <div class="card">
            <h3>Zero-RAM Tray Mode</h3>
            <p>Destroys WebView memory on window close; runs lightweight in background.</p>
          </div>
          <div class="card">
            <h3>Lean Extractor</h3>
            <p>Automatically filters .pdb and test suites from database archives.</p>
          </div>
        </div>

        <div class="card">
          <h3>System Status</h3>
          <p>{systemInfo()}</p>
        </div>
      </main>

      <footer class="footer">
        <span>Powered by Rakitray — Created by Yoga ardiansyah</span>
        <a href="https://github.com/yogaardiansyah/rakitray" target="_blank" rel="noreferrer">
          GitHub Repository
        </a>
      </footer>
    </div>
  );
}
