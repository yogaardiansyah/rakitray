import { For, Show, createMemo, createSignal } from "solid-js";
import { useSupervisor } from "../../store/supervisorStore";
import { Icon } from "../atoms";

export function SiteRoutingPanel() {
  const { state, saveProjectSite, deleteProjectSite } = useSupervisor();

  const [name, setName] = createSignal("");
  const [domain, setDomain] = createSignal("");
  const [domainTouched, setDomainTouched] = createSignal(false);
  const [rootPath, setRootPath] = createSignal("");
  const [webServerId, setWebServerId] = createSignal("nginx_1");
  const [runtimeId, setRuntimeId] = createSignal("php_83");
  const [port, setPort] = createSignal(8088);
  const [siteFilter, setSiteFilter] = createSignal("");
  const [isSaving, setIsSaving] = createSignal(false);
  const [statusMsg, setStatusMsg] = createSignal<string | null>(null);
  const [statusType, setStatusType] = createSignal<"success" | "error">("success");
  const [copiedSnippet, setCopiedSnippet] = createSignal(false);

  const phpInstances = () => state.instances.filter((i) => i.category === "runtime");
  const webInstances = () => state.instances.filter((i) => i.category === "web_server");
  const instanceLabel = (id: string) =>
    state.instances.find((i) => i.instanceId === id)?.displayName ?? id;
  const instanceStatus = (id: string) =>
    state.instances.find((i) => i.instanceId === id)?.status ?? "stopped";
  const isWebRunning = () => instanceStatus(webServerId()) === "running";
  const isPhpRunning = () => instanceStatus(runtimeId()) === "running";

  const handleEditSite = (site: (typeof state.sites)[number]) => {
    setName(site.name);
    setDomain(site.domain);
    setDomainTouched(true);
    setRootPath(site.rootPath);
    setWebServerId(site.webServerId);
    setRuntimeId(site.runtimeId);
    setPort(site.port);
  };

  const filteredSites = createMemo(() => {
    const q = siteFilter().trim().toLowerCase();
    if (!q) return state.sites;
    return state.sites.filter(
      (s) =>
        s.name.toLowerCase().includes(q) ||
        s.domain.toLowerCase().includes(q) ||
        String(s.port).includes(q),
    );
  });

  const handleNameInput = (val: string) => {
    setName(val);
    if (!domainTouched()) {
      const slug = val
        .trim()
        .toLowerCase()
        .replace(/[^a-z0-9-]+/g, "-")
        .replace(/^-|-$/g, "");
      setDomain(slug ? `${slug}.test` : "");
    }
  };

  const isFormValid = () =>
    name().trim().length > 0 && domain().trim().length > 0 && rootPath().trim().length > 0;

  const handleSubmit = async (e: Event) => {
    e.preventDefault();
    if (!isFormValid()) return;
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
      setStatusMsg(
        isWebRunning()
          ? "Site route saved and web server reloaded."
          : "Site route saved. Start Nginx in Service Rack to serve.",
      );
      setTimeout(() => setStatusMsg(null), 4000);
    } else {
      setStatusType("error");
      setStatusMsg("Failed to save site route.");
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
          <h3>Local Sites & Domains</h3>
          <p class="subtitle">Route a local domain and folder to a web server and PHP version</p>
        </div>
        <span class="preset-pill font-mono">Nginx · FPM</span>
      </div>

      <form onSubmit={handleSubmit} class="dense-form">
        <Show when={statusMsg()}>
          <output
            aria-live="polite"
            class={`routing-banner ${statusType() === "success" ? "banner-success" : "banner-error"}`}
          >
            {statusMsg()}
          </output>
        </Show>

        <div class="form-row-dense">
          <div class="form-field">
            <label for="org-site-name">Project Name</label>
            <input
              id="org-site-name"
              type="text"
              class="dense-input"
              value={name()}
              onInput={(e) => handleNameInput(e.currentTarget.value)}
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
              onInput={(e) => {
                setDomainTouched(true);
                setDomain(e.currentTarget.value);
              }}
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
            <label for="org-select-web" class="field-label-row">
              <span>Web Server</span>
              <span class={`engine-status-inline ${isWebRunning() ? "is-up" : "is-down"}`}>
                {isWebRunning() ? "Running" : "Stopped"}
              </span>
            </label>
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
            <label for="org-select-php" class="field-label-row">
              <span>PHP Version</span>
              <span class={`engine-status-inline ${isPhpRunning() ? "is-up" : "is-down"}`}>
                {isPhpRunning() ? "Running" : "Stopped"}
              </span>
            </label>
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
            disabled={isSaving() || !isFormValid()}
          >
            {isSaving() ? "Applying..." : "Apply Routing to Nginx"}
          </button>
        </div>
      </form>

      {/* Active Applied Routes Table */}
      <div class="active-routes-section">
        <div class="routes-header-flex">
          <h4 class="routes-title">Active Configured Routes</h4>
          <div class="routes-header-right">
            <Show when={state.sites.length > 4}>
              <input
                type="search"
                class="dense-input font-mono"
                style={{ width: "180px", height: "28px", "font-size": "0.74rem" }}
                placeholder="Filter routes..."
                aria-label="Filter configured routes"
                value={siteFilter()}
                onInput={(e) => setSiteFilter(e.currentTarget.value)}
              />
            </Show>
            <span class="loopback-hint text-xs">
              Loopback: <code>http://127.0.0.1:{port()}</code>
            </span>
          </div>
        </div>

        <div class="routes-table font-mono">
          <div class="routes-header">
            <span>PROJECT</span>
            <span>DOMAIN</span>
            <span>WEB SERVER</span>
            <span>UPSTREAM</span>
            <span>LOOPBACK URL</span>
            <span>ACTIONS</span>
          </div>

          <For
            each={filteredSites()}
            fallback={
              <div class="routes-empty font-mono">
                {state.sites.length > 0
                  ? "No routes match the current filter."
                  : "No project sites routed yet. Add a route above."}
              </div>
            }
          >
            {(site) => (
              <div class="routes-row">
                <span class="route-name">{site.name}</span>
                <span class="route-domain">{site.domain}</span>
                <span class="route-server" title={site.webServerId}>
                  {instanceLabel(site.webServerId)}
                </span>
                <span class="route-upstream" title={site.runtimeId}>
                  {instanceLabel(site.runtimeId)}
                </span>
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
                <span class="route-actions-cell">
                  <button
                    type="button"
                    class="dense-action-btn btn-ghost-icon"
                    title={`Edit ${site.name}`}
                    aria-label={`Edit ${site.name}`}
                    onClick={() => handleEditSite(site)}
                  >
                    <Icon name="edit" size={12} />
                  </button>
                  <button
                    type="button"
                    class="dense-action-btn btn-ghost-icon"
                    title={`Delete ${site.name}`}
                    aria-label={`Delete ${site.name}`}
                    onClick={() => void deleteProjectSite(site.name)}
                  >
                    <Icon name="x" size={12} />
                  </button>
                </span>
              </div>
            )}
          </For>
        </div>

        {/* /etc/hosts Helper Snippet */}
        <div class="hosts-helper-card">
          <div class="hosts-helper-header">
            <span class="font-bold text-xs">
              Optional /etc/hosts entry (still use :{port()} if Nginx is not on port 80):
            </span>
            <button
              type="button"
              class="copy-snippet-btn"
              disabled={!domain().trim()}
              onClick={copyHostsSnippet}
            >
              <Icon name={copiedSnippet() ? "check" : "copy"} size={12} />
              <span>{copiedSnippet() ? "Copied Command" : "Copy Command"}</span>
            </button>
          </div>
          <code class="hosts-code-snippet font-mono">
            echo "127.0.0.1 {domain().trim() || "my-app.test"}" | sudo tee -a /etc/hosts
          </code>
        </div>
      </div>
    </div>
  );
}
