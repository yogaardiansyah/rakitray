import { useRakiStore } from "../../store/storeState";
import { useSupervisor } from "../../store/supervisorStore";

export function EmptyRackOnboarding() {
  const { setActiveTab, scanRuntimes } = useSupervisor();
  const { installStack } = useRakiStore();

  const handleInstallStack = async (templateId: string) => {
    await installStack(templateId, async () => {
      await scanRuntimes();
    });
  };

  return (
    <div class="onboarding-panel">
      <div class="onboarding-header">
        <span class="onboarding-eyebrow font-mono">CLEAN CORE WORKSTATION</span>
        <h2 class="onboarding-title">Empty Rack — Quick-Start Stacks</h2>
        <p class="onboarding-subtitle">
          Select a curated 1-click runtime stack or browse individual verified packages in
          RakiStore.
        </p>
      </div>

      <div class="onboarding-grid">
        <div class="onboarding-card onboarding-card-featured">
          <div class="onboarding-card-body">
            <div class="onboarding-card-top">
              <span class="onboarding-badge badge-recommended font-mono">RECOMMENDED</span>
              <span class="onboarding-pkg-count font-mono">4 Packages</span>
            </div>
            <h3 class="onboarding-card-title">Laravel Modern</h3>
            <p class="onboarding-card-desc">
              Production-matched environment with high-performance web, PHP 8.3, database, and cache
              services.
            </p>
            <div class="onboarding-tag-list font-mono">
              <span class="onboarding-tag">Nginx 1.26</span>
              <span class="onboarding-tag">PHP 8.3</span>
              <span class="onboarding-tag">MariaDB 11.4</span>
              <span class="onboarding-tag">Redis 7.2</span>
            </div>
          </div>
          <button
            type="button"
            class="onboarding-cta-btn cta-primary"
            onClick={() => handleInstallStack("laravel-modern")}
          >
            1-Click Install Stack
          </button>
        </div>

        <div class="onboarding-card">
          <div class="onboarding-card-body">
            <div class="onboarding-card-top">
              <span class="onboarding-badge badge-compat font-mono">COMPATIBILITY</span>
              <span class="onboarding-pkg-count font-mono">3 Packages</span>
            </div>
            <h3 class="onboarding-card-title">Legacy PHP Stack</h3>
            <p class="onboarding-card-desc">
              Optimized for existing PHP 8.2 projects and standard relational database workloads.
            </p>
            <div class="onboarding-tag-list font-mono">
              <span class="onboarding-tag">Nginx 1.26</span>
              <span class="onboarding-tag">PHP 8.2</span>
              <span class="onboarding-tag">MariaDB 11.4</span>
            </div>
          </div>
          <button
            type="button"
            class="onboarding-cta-btn cta-secondary"
            onClick={() => handleInstallStack("legacy-php")}
          >
            Install PHP 8.2 Stack
          </button>
        </div>

        <div class="onboarding-card">
          <div class="onboarding-card-body">
            <div class="onboarding-card-top">
              <span class="onboarding-badge badge-custom font-mono">CUSTOM</span>
              <span class="onboarding-pkg-count font-mono">Modular</span>
            </div>
            <h3 class="onboarding-card-title">Custom Stack</h3>
            <p class="onboarding-card-desc">
              Browse the official RakiStore package catalog and select individual runtime binaries.
            </p>
            <div class="onboarding-tag-list font-mono">
              <span class="onboarding-tag">Caddy</span>
              <span class="onboarding-tag">PostgreSQL</span>
              <span class="onboarding-tag">Mailpit</span>
            </div>
          </div>
          <button
            type="button"
            class="onboarding-cta-btn cta-secondary"
            onClick={() => setActiveTab("store")}
          >
            Browse RakiStore →
          </button>
        </div>
      </div>
    </div>
  );
}
