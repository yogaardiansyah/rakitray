import { For, Show, createSignal } from "solid-js";
import { useSupervisor } from "../../store/supervisorStore";

export function SiteRoutingPanel() {
  const { state, saveProjectSite } = useSupervisor();

  const [name, setName] = createSignal("");
  const [domain, setDomain] = createSignal("");
  const [rootPath, setRootPath] = createSignal("");
  const [webServerId, setWebServerId] = createSignal("nginx_1");
  const [runtimeId, setRuntimeId] = createSignal("php_83");
  const [port, setPort] = createSignal(8088);
  const [isSaving, setIsSaving] = createSignal(false);
  const [statusMsg, setStatusMsg] = createSignal<string | null>(null);
  const [statusType, setStatusType] = createSignal<"success" | "error">("success");
  const [copiedSnippet, setCopiedSnippet] = createSignal(false);

  const phpInstances = () => state.instances.filter((i) => i.category === "runtime");

  const webInstances = () => state.instances.filter((i) => i.category === "web_server");

  const handleSubmit = async (e: Event) => {
    e.preventDefault();
    setIsSaving(true);
    setStatusMsg(null);

    const success = await saveProjectSite({
      name: name(),
      domain: domain(),
      rootPath: rootPath(),
      webServerId: webServerId(),
      runtimeId: runtimeId(),
      port: Number(port()),
    });

    setIsSaving(false);
    if (success) {
      setStatusType("success");
      setStatusMsg("✓ Routing applied & Nginx reloaded");
      setTimeout(() => setStatusMsg(null), 4000);
    } else {
      setStatusType("error");
      setStatusMsg("✕ Failed to apply routing to Nginx");
    }
  };

  const copyHostsSnippet = () => {
    const cmd = `echo "127.0.0.1 ${domain()}" | sudo tee -a /etc/hosts`;
    navigator.clipboard.writeText(cmd);
    setCopiedSnippet(true);
    setTimeout(() => setCopiedSnippet(false), 3000);
  };

  return (
    <div class="site-routing-panel dense-card">
      <div class="panel-header-dense">
        <div>
          <h3>Project Upstream & Virtual Hosts</h3>
          <p class="subtitle">
            Maps domain and document root to supervised web and runtime engines
          </p>
        </div>
        <span class="preset-pill font-mono">Laravel-First</span>
      </div>

      <form onSubmit={handleSubmit} class="dense-form">
        <Show when={statusMsg()}>
          <div
            class={`routing-banner ${statusType() === "success" ? "banner-success" : "banner-error"}`}
          >
            {statusMsg()}
          </div>
        </Show>

        <div class="form-row-dense">
          <div class="form-field">
            <label for="org-site-name">Project Name</label>
            <input
              id="org-site-name"
              type="text"
              class="dense-input"
              value={name()}
              onInput={(e) => setName(e.currentTarget.value)}
              placeholder="e.g. my-app"
            />
          </div>

          <div class="form-field">
            <label for="org-site-domain">Local Domain (.test)</label>
            <input
              id="org-site-domain"
              type="text"
              class="dense-input"
              value={domain()}
              onInput={(e) => setDomain(e.currentTarget.value)}
              placeholder="e.g. my-app.test"
            />
          </div>
        </div>

        <div class="form-field">
          <label for="org-site-root">Public Document Root</label>
          <input
            id="org-site-root"
            type="text"
            class="dense-input font-mono text-xs"
            value={rootPath()}
            onInput={(e) => setRootPath(e.currentTarget.value)}
            placeholder="/path/to/project/public"
          />
        </div>

        <div class="form-row-dense">
          <div class="form-field">
            <label for="org-select-web">Web Server</label>
            <select
              id="org-select-web"
              class="dense-select"
              value={webServerId()}
              onInput={(e) => setWebServerId(e.currentTarget.value)}
              onChange={(e) => setWebServerId(e.currentTarget.value)}
            >
              <For
                each={webInstances()}
                fallback={
                  <option class="dense-option" value="nginx_1">
                    Nginx Web Server (:8088)
                  </option>
                }
              >
                {(inst) => (
                  <option class="dense-option" value={inst.instanceId}>
                    {inst.displayName} (port {inst.port})
                  </option>
                )}
              </For>
            </select>
          </div>

          <div class="form-field">
            <label for="org-select-php">PHP Upstream Engine</label>
            <select
              id="org-select-php"
              class="dense-select"
              value={runtimeId()}
              onInput={(e) => setRuntimeId(e.currentTarget.value)}
              onChange={(e) => setRuntimeId(e.currentTarget.value)}
            >
              <For
                each={phpInstances()}
                fallback={
                  <option class="dense-option" value="php_83">
                    PHP 8.3 (Laravel Default)
                  </option>
                }
              >
                {(inst) => (
                  <option class="dense-option" value={inst.instanceId}>
                    {inst.displayName} (port {inst.port})
                  </option>
                )}
              </For>
            </select>
          </div>

          <div class="form-field" style={{ "max-width": "120px" }}>
            <label for="org-site-port">Listen Port</label>
            <input
              id="org-site-port"
              type="number"
              class="dense-input font-mono"
              value={port()}
              onInput={(e) => setPort(Number(e.currentTarget.value))}
            />
          </div>
        </div>

        <div class="form-actions-dense">
          <button
            type="submit"
            class="dense-action-btn btn-primary-solid h-9 px-4"
            disabled={isSaving()}
          >
            {isSaving() ? "Applying..." : "Apply Routing to Nginx"}
          </button>
        </div>
      </form>

      {/* Active Applied Routes Table */}
      <div class="active-routes-section">
        <div class="routes-header-flex">
          <h4 class="routes-title">Active Configured Routes</h4>
          <span class="loopback-hint text-xs">
            Tip: <code>http://127.0.0.1:{port()}</code> works immediately without modifying{" "}
            <code>/etc/hosts</code>
          </span>
        </div>

        <div class="routes-table font-mono">
          <div class="routes-header">
            <span>PROJECT</span>
            <span>DOMAIN</span>
            <span>WEB SERVER</span>
            <span>UPSTREAM</span>
            <span>LOOPBACK URL</span>
          </div>

          <For
            each={state.sites}
            fallback={
              <div class="routes-empty text-xs text-slate-500 py-4 text-center">
                No project sites routed yet. Add a route above.
              </div>
            }
          >
            {(site) => (
              <div class="routes-row">
                <span class="route-name">{site.name}</span>
                <span class="route-domain">{site.domain}</span>
                <span class="route-server">{site.webServerId}</span>
                <span class="route-upstream">{site.runtimeId}</span>
                <span class="route-link">
                  <a
                    href={`http://127.0.0.1:${site.port}`}
                    target="_blank"
                    rel="noreferrer"
                    class="link-badge"
                  >
                    http://127.0.0.1:{site.port} ↗
                  </a>
                </span>
              </div>
            )}
          </For>
        </div>

        {/* /etc/hosts Helper Snippet */}
        <div class="hosts-helper-card">
          <div class="hosts-helper-header">
            <span class="font-bold text-xs">DNS Resolver Setup for Local Domain:</span>
            <button type="button" class="copy-snippet-btn" onClick={copyHostsSnippet}>
              {copiedSnippet() ? "✓ Copied Command" : "📋 Copy Command"}
            </button>
          </div>
          <code class="hosts-code-snippet font-mono">
            echo "127.0.0.1 {domain()}" | sudo tee -a /etc/hosts
          </code>
        </div>
      </div>
    </div>
  );
}
