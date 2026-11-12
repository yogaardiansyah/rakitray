import { For, createSignal } from "solid-js";
import type { ProjectSite, ServiceInstance } from "../types/ipc";

interface SiteRoutingPanelProps {
  instances: Record<string, ServiceInstance>;
  sites: ProjectSite[];
  onSaveSite: (site: ProjectSite) => void;
}

export function SiteRoutingPanel(props: SiteRoutingPanelProps) {
  const [name, setName] = createSignal("laravel-demo");
  const [domain, setDomain] = createSignal("laravel-demo.test");
  const [rootPath, setRootPath] = createSignal("/var/www/laravel-demo/public");
  const [webServerId, setWebServerId] = createSignal("nginx_1");
  const [runtimeId, setRuntimeId] = createSignal("php_83");
  const [port, setPort] = createSignal(80);

  const phpInstances = () => Object.values(props.instances).filter((i) => i.category === "runtime");

  const webInstances = () =>
    Object.values(props.instances).filter((i) => i.category === "web_server");

  const handleSubmit = (e: Event) => {
    e.preventDefault();
    props.onSaveSite({
      name: name(),
      domain: domain(),
      rootPath: rootPath(),
      webServerId: webServerId(),
      runtimeId: runtimeId(),
      port: Number(port()),
    });
  };

  return (
    <div class="card routing-panel">
      <h3>Laravel Site Routing & Upstream</h3>
      <form onSubmit={handleSubmit} class="site-form">
        <div class="form-row">
          <div class="form-group">
            <label for="site-name">Project Name</label>
            <input
              id="site-name"
              type="text"
              value={name()}
              onInput={(e) => setName(e.currentTarget.value)}
              placeholder="e.g. my-app"
            />
          </div>
          <div class="form-group">
            <label for="site-domain">Domain</label>
            <input
              id="site-domain"
              type="text"
              value={domain()}
              onInput={(e) => setDomain(e.currentTarget.value)}
              placeholder="e.g. my-app.test"
            />
          </div>
        </div>

        <div class="form-group">
          <label for="site-root">Public Document Root</label>
          <input
            id="site-root"
            type="text"
            value={rootPath()}
            onInput={(e) => setRootPath(e.currentTarget.value)}
            placeholder="/path/to/laravel/public"
          />
        </div>

        <div class="form-row">
          <div class="form-group">
            <label for="select-web">Web Server</label>
            <select
              id="select-web"
              value={webServerId()}
              onChange={(e) => setWebServerId(e.currentTarget.value)}
            >
              <For each={webInstances()}>
                {(inst) => (
                  <option value={inst.instanceId}>
                    {inst.displayName} (:{inst.port})
                  </option>
                )}
              </For>
            </select>
          </div>

          <div class="form-group">
            <label for="select-php">PHP Upstream Engine</label>
            <select
              id="select-php"
              value={runtimeId()}
              onChange={(e) => setRuntimeId(e.currentTarget.value)}
            >
              <For each={phpInstances()}>
                {(inst) => (
                  <option value={inst.instanceId}>
                    {inst.displayName} (port {inst.port})
                  </option>
                )}
              </For>
            </select>
          </div>

          <div class="form-group" style={{ "max-width": "100px" }}>
            <label for="site-port">Port</label>
            <input
              id="site-port"
              type="number"
              value={port()}
              onInput={(e) => setPort(Number(e.currentTarget.value))}
            />
          </div>
        </div>

        <div style={{ "margin-top": "1rem" }}>
          <button type="submit" class="btn btn-primary">
            Apply Routing to Nginx
          </button>
        </div>
      </form>
    </div>
  );
}
