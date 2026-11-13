import { For, Show, createEffect, createMemo, createSignal, onMount } from "solid-js";
import { usePhpMatrix } from "../../store/phpMatrixStore";
import { useSupervisor } from "../../store/supervisorStore";
import type {
  ExtensionCategory,
  PhpExtension,
  PhpIniDirective,
  PhpProfileMode,
} from "../../types/phpMatrix";
import { ActionButton, ExtensionCheckbox, Icon, ProfileBadge, StatusLed } from "../atoms";
import { PendingBar, ProfileToggle } from "../molecules";

const CATEGORY_METADATA: { id: ExtensionCategory; title: string; subtitle: string }[] = [
  {
    id: "laravelCore",
    title: "Laravel Core (Tier 1)",
    subtitle: "Mandatory extensions for Laravel 12/13 runtime boot",
  },
  {
    id: "databaseCache",
    title: "Database & Cache (Tier 2/3)",
    subtitle: "Relational drivers, Redis, and high-performance caching",
  },
  {
    id: "performanceDebug",
    title: "Performance & Debugging",
    subtitle: "Zend engine accelerators, profilers, and dev utilities",
  },
  {
    id: "mediaFormat",
    title: "Media & Data Formats",
    subtitle: "Image processing, archive handling, and internationalization",
  },
  {
    id: "other",
    title: "Additional Extensions",
    subtitle: "Discovered shared modules available in ext-dir",
  },
];

export function PhpMatrixPanel() {
  const { state: supervisorState } = useSupervisor();
  const {
    state: matrixState,
    initMatrixChannel,
    selectInstance,
    toggleExtension,
    stageProfile,
    discardPending,
    commitPending,
    runLaravelPreset,
    cloneToTarget,
    clearError,
    clearWarnings,
    toggleDirective,
    updateDirectiveValue,
    addCustomDirective,
    removeCustomDirective,
  } = usePhpMatrix();

  const [cloneTargetId, setCloneTargetId] = createSignal("");
  const [showCloneModal, setShowCloneModal] = createSignal(false);

  const [searchQuery, setSearchQuery] = createSignal("");
  const [runtimeFilter, setRuntimeFilter] = createSignal("");
  const [sectionFilter, setSectionFilter] = createSignal("All");
  const [showAddForm, setShowAddForm] = createSignal(false);
  const [newKey, setNewKey] = createSignal("");
  const [newValue, setNewValue] = createSignal("");
  const [newSection, setNewSection] = createSignal("Custom");
  const [editingKey, setEditingKey] = createSignal<string | null>(null);
  const [editingValue, setEditingValue] = createSignal("");

  onMount(() => {
    void initMatrixChannel();
  });

  const phpInstances = () => supervisorState.instances.filter((i) => i.category === "runtime");
  const filteredPhpInstances = createMemo(() => {
    const q = runtimeFilter().trim().toLowerCase();
    if (!q) return phpInstances();
    return phpInstances().filter(
      (i) => i.displayName.toLowerCase().includes(q) || i.version.toLowerCase().includes(q),
    );
  });

  // Auto-select first PHP instance if none selected
  createEffect(() => {
    const instances = phpInstances();
    if (instances.length > 0 && !matrixState.selectedInstanceId) {
      void selectInstance(instances[0].instanceId);
    }
  });

  const currentInstance = () =>
    phpInstances().find((i) => i.instanceId === matrixState.selectedInstanceId);

  const currentMatrix = () =>
    matrixState.selectedInstanceId
      ? matrixState.matrices[matrixState.selectedInstanceId]
      : undefined;

  const otherPhpInstances = () =>
    phpInstances().filter((i) => i.instanceId !== matrixState.selectedInstanceId);

  // Auto-set clone target if not set
  createEffect(() => {
    const others = otherPhpInstances();
    if (
      others.length > 0 &&
      (!cloneTargetId() || cloneTargetId() === matrixState.selectedInstanceId)
    ) {
      setCloneTargetId(others[0].instanceId);
    }
  });

  const activeProfile = () => matrixState.pendingProfile ?? currentMatrix()?.profile ?? "dev";

  const activeLimits = () =>
    matrixState.pendingLimits ??
    currentMatrix()?.limits ?? {
      memoryLimit: "512M",
      uploadMaxFilesize: "128M",
      postMaxSize: "128M",
      maxExecutionTime: 300,
    };

  const hasPending = () =>
    matrixState.pendingProfile !== null || matrixState.pendingLimits !== null;

  const limitsValid = createMemo(() => {
    const limits = activeLimits();
    const memValid = /^(-1|\d+[KMGkmg]?)$/.test(limits.memoryLimit.trim());
    const uploadValid = /^(-1|\d+[KMGkmg]?)$/.test(limits.uploadMaxFilesize.trim());
    const postValid = /^(-1|\d+[KMGkmg]?)$/.test(limits.postMaxSize.trim());
    const execValid = /^-?\d+$/.test(String(limits.maxExecutionTime).trim());
    return memValid && uploadValid && postValid && execValid;
  });

  const enabledCount = createMemo(() => {
    return currentMatrix()?.extensions.filter((e) => e.enabled).length ?? 0;
  });

  const totalCount = createMemo(() => {
    return currentMatrix()?.extensions.length ?? 0;
  });

  const getExtensionsForCategory = (cat: ExtensionCategory): PhpExtension[] => {
    const matrix = currentMatrix();
    if (!matrix) return [];
    return matrix.extensions.filter((e) => e.category === cat);
  };

  const getLaravelBadge = (version?: string) => {
    if (!version) return "Laravel 12/13";
    const parts = version.split(".");
    const maj = Number.parseInt(parts[0] || "0", 10);
    const min = Number.parseInt(parts[1] || "0", 10);
    if (maj > 8 || (maj === 8 && min >= 3)) return "Laravel 12/13 Ready";
    if (maj === 8 && min === 2) return "Laravel 12 Ready (PHP 8.2)";
    return `Legacy PHP ${version}`;
  };

  const handleOpenClone = () => {
    if (otherPhpInstances().length > 0) {
      setShowCloneModal(true);
    }
  };

  const handleConfirmClone = async () => {
    const target = cloneTargetId();
    if (target) {
      await cloneToTarget(target);
      setShowCloneModal(false);
    }
  };

  const directives = () => currentMatrix()?.directives ?? [];

  const filteredDirectives = createMemo(() => {
    const list = directives();
    const query = searchQuery().trim().toLowerCase();
    const sec = sectionFilter();

    return list.filter((d) => {
      if (sec !== "All" && d.section !== sec) return false;
      if (!query) return true;
      return (
        d.key.toLowerCase().includes(query) ||
        d.value.toLowerCase().includes(query) ||
        d.section.toLowerCase().includes(query)
      );
    });
  });

  const directiveSections = createMemo(() => {
    const list = directives();
    const set = new Set<string>();
    for (const d of list) {
      if (d.section) set.add(d.section);
    }
    return Array.from(set);
  });

  const enabledDirectivesCount = createMemo(() => {
    return directives().filter((d) => d.enabled).length;
  });

  const handleStartEdit = (d: PhpIniDirective) => {
    setEditingKey(d.key);
    setEditingValue(d.value);
  };

  const handleCommitEdit = async (key: string) => {
    const val = editingValue();
    setEditingKey(null);
    await updateDirectiveValue(key, val);
  };

  const handleCancelEdit = () => {
    setEditingKey(null);
    setEditingValue("");
  };

  const handleToggleDirective = (d: PhpIniDirective) => {
    void toggleDirective(d.key);
  };

  const handleRemoveDirective = (d: PhpIniDirective) => {
    if (d.custom) {
      void removeCustomDirective(d.key);
    } else {
      void toggleDirective(d.key, false);
    }
  };

  const handleAddDirective = async () => {
    const k = newKey().trim();
    const v = newValue().trim();
    const s = newSection().trim() || "Custom";
    if (!k) return;
    await addCustomDirective(k, v, s);
    setNewKey("");
    setNewValue("");
    setShowAddForm(false);
  };

  return (
    <div class="php-matrix-panel dense-card">
      {/* Header Toolbar */}
      <div class="matrix-top-bar">
        <div class="matrix-title-area">
          <div class="matrix-title-row">
            <h3 class="font-bold">PHP INI & Extension Matrix</h3>
            <span class="matrix-engine-pill font-mono">DEV / PROD</span>
          </div>
          <p class="subtitle">Manage per-version PHP extensions and php.ini directives</p>
        </div>

        {/* Action Controls */}
        <div class="matrix-controls-row">
          <div class="matrix-selector-group">
            <label for="runtime-select" class="control-label font-mono">
              RUNTIME:
            </label>
            <Show when={phpInstances().length > 4}>
              <input
                type="search"
                class="dense-input font-mono"
                style={{ width: "110px", height: "30px" }}
                placeholder="Filter PHP..."
                aria-label="Filter PHP versions"
                value={runtimeFilter()}
                onInput={(e) => setRuntimeFilter(e.currentTarget.value)}
              />
            </Show>
            <select
              id="runtime-select"
              class="dense-select font-mono"
              value={matrixState.selectedInstanceId}
              onChange={(e) => void selectInstance(e.currentTarget.value)}
              disabled={filteredPhpInstances().length === 0 || matrixState.isLoading}
            >
              <For each={filteredPhpInstances()}>
                {(inst) => (
                  <option value={inst.instanceId} class="dense-option">
                    {inst.displayName} (v{inst.version}) — {getLaravelBadge(inst.version)}
                  </option>
                )}
              </For>
            </select>
          </div>

          <ProfileToggle
            mode={activeProfile()}
            disabled={!currentMatrix() || matrixState.isLoading}
            onChange={(m: PhpProfileMode) => stageProfile(m)}
          />

          <ActionButton
            variant="primary"
            onClick={() => void runLaravelPreset()}
            disabled={!currentMatrix() || matrixState.isLoading}
            loading={matrixState.isLoading}
            title="Enable all Laravel 12/13 core and recommended extensions in one click"
          >
            <Icon name="zap" size={13} />
            <span>1-Click Laravel Preset</span>
          </ActionButton>

          <Show when={otherPhpInstances().length > 0}>
            <div class="clone-btn-group">
              <ActionButton
                variant="ghost"
                onClick={handleOpenClone}
                disabled={!currentMatrix() || matrixState.isLoading}
              >
                <Icon name="copy" size={13} />
                <span>Clone to…</span>
              </ActionButton>
            </div>
          </Show>
        </div>
      </div>

      {/* Status Notifications */}
      <Show when={matrixState.applyStatus === "applying"}>
        <div class="matrix-banner banner-info font-mono">
          <span class="banner-spinner" /> Validating INI syntax and reloading PHP-FPM workers…
        </div>
      </Show>

      <Show when={matrixState.applyStatus === "reloaded"}>
        <output aria-live="polite" class="matrix-banner banner-success font-mono">
          <span class="banner-inline-msg">
            <Icon name="check" size={14} />
            <span>
              Configuration validated successfully. PHP workers reloaded without downtime.
            </span>
          </span>
        </output>
      </Show>

      <Show when={matrixState.applyStatus === "rolledBack"}>
        <output aria-live="polite" class="matrix-banner banner-error font-mono">
          <span class="banner-inline-msg">
            <Icon name="x" size={14} />
            <span>INI validation failed. State automatically rolled back to previous backup.</span>
          </span>
          <Show when={matrixState.lastRollbackReason}>
            <div class="rollback-reason">{matrixState.lastRollbackReason}</div>
          </Show>
        </output>
      </Show>

      <Show when={matrixState.error}>
        <div role="alert" class="matrix-banner banner-error font-mono">
          <span class="banner-inline-msg">
            <Icon name="x" size={14} />
            <span>Error: {matrixState.error}</span>
          </span>
          <button
            type="button"
            class="banner-dismiss"
            onClick={clearError}
            aria-label="Dismiss error"
          >
            <Icon name="x" size={13} />
          </button>
        </div>
      </Show>

      <Show when={matrixState.warnings.length > 0}>
        <div class="matrix-banner banner-warning font-mono">
          <div class="warning-header">
            <span class="banner-inline-msg">
              <Icon name="alert" size={14} />
              <span>Presets & Compatibility Warnings:</span>
            </span>
            <button
              type="button"
              class="banner-dismiss"
              onClick={clearWarnings}
              aria-label="Dismiss warnings"
            >
              <Icon name="x" size={13} />
            </button>
          </div>
          <ul class="warning-list">
            <For each={matrixState.warnings}>{(warn) => <li>{warn}</li>}</For>
          </ul>
        </div>
      </Show>

      {/* Main Content Area */}
      <Show
        when={currentMatrix()}
        fallback={
          <div class="empty-state-dense font-mono">
            {matrixState.isLoading
              ? "Loading PHP extension matrix…"
              : phpInstances().length === 0
                ? "No PHP runtimes installed in runtime/bin/. Scan runtimes in Runtime Binaries tab."
                : "Select a PHP runtime to view and edit its extension matrix."}
          </div>
        }
      >
        {/* Runtime Meta & Metrics Strip */}
        <div class="matrix-meta-strip font-mono">
          <div class="meta-item">
            <span class="meta-label">STATUS:</span>
            <StatusLed status={currentInstance()?.status ?? "stopped"} size="sm" />
            <span class="meta-value">{(currentInstance()?.status ?? "stopped").toUpperCase()}</span>
          </div>
          <div class="meta-item">
            <span class="meta-label">PROFILE:</span>
            <ProfileBadge mode={activeProfile()} />
          </div>
          <div class="meta-item">
            <span class="meta-label">DIRECTIVES:</span>
            <span class="meta-value text-emerald">
              {enabledDirectivesCount()} / {directives().length} ACTIVE
            </span>
          </div>
          <div class="meta-item">
            <span class="meta-label">EXTENSIONS:</span>
            <span class="meta-value text-emerald">
              {enabledCount()} / {totalCount()} ENABLED
            </span>
          </div>
          <div class="meta-item">
            <span class="meta-label">COMPATIBILITY:</span>
            <span class="meta-value text-sky">{getLaravelBadge(currentMatrix()?.version)}</span>
          </div>
        </div>

        {/* Pending Changes Action Bar (when profile or limits staged) */}
        <Show when={hasPending()}>
          <div style={{ "margin-bottom": "0.75rem" }}>
            <PendingBar
              hasPending={hasPending()}
              isApplying={matrixState.isLoading || matrixState.applyStatus === "applying"}
              isValid={limitsValid()}
              onApply={() => void commitPending()}
              onDiscard={discardPending}
            />
          </div>
        </Show>

        {/* Terminal Commands Style: PHP INI Directives */}
        <div class="terminal-commands-section">
          <div class="terminal-section-header">
            <div class="terminal-title-area">
              <h4 class="terminal-heading font-mono">PHP INI Directives</h4>
              <span class="terminal-subtitle">
                Allow/deny and configure all runtime directives, limits, opcache, and custom
                parameters.
              </span>
            </div>

            <div class="terminal-header-actions">
              <input
                type="text"
                class="terminal-search-input font-mono"
                placeholder="Filter directives…"
                value={searchQuery()}
                onInput={(e) => setSearchQuery(e.currentTarget.value)}
              />

              <select
                class="terminal-section-select font-mono"
                value={sectionFilter()}
                onChange={(e) => setSectionFilter(e.currentTarget.value)}
              >
                <option value="All">All Sections</option>
                <For each={directiveSections()}>{(sec) => <option value={sec}>{sec}</option>}</For>
              </select>

              <button
                type="button"
                class="terminal-add-btn font-mono"
                onClick={() => setShowAddForm(!showAddForm())}
                title="Add custom PHP INI directive"
              >
                + Add
              </button>
            </div>
          </div>

          {/* Add Directive Inline Form */}
          <Show when={showAddForm()}>
            <div class="terminal-add-form font-mono">
              <input
                type="text"
                class="terminal-add-input"
                placeholder="Directive key (e.g. opcache.jit_buffer_size)"
                value={newKey()}
                onInput={(e) => setNewKey(e.currentTarget.value)}
              />
              <input
                type="text"
                class="terminal-add-input"
                placeholder="Value (e.g. 64M)"
                value={newValue()}
                onInput={(e) => setNewValue(e.currentTarget.value)}
              />
              <select
                class="terminal-section-select"
                value={newSection()}
                onChange={(e) => setNewSection(e.currentTarget.value)}
              >
                <option value="Custom">Custom</option>
                <option value="Limits & Uploads">Limits & Uploads</option>
                <option value="Error & Logging">Error & Logging</option>
                <option value="PHP Core">PHP Core</option>
                <option value="Session & Security">Session & Security</option>
                <option value="Performance & OPcache">Performance & OPcache</option>
                <option value="Debugging & Profiling">Debugging & Profiling</option>
              </select>
              <ActionButton
                variant="primary"
                onClick={() => void handleAddDirective()}
                disabled={!newKey().trim() || matrixState.isLoading}
              >
                Save
              </ActionButton>
              <ActionButton
                variant="ghost"
                onClick={() => {
                  setShowAddForm(false);
                  setNewKey("");
                  setNewValue("");
                }}
              >
                Cancel
              </ActionButton>
            </div>
          </Show>

          {/* Directive Rows List */}
          <div class="terminal-rows-list">
            <For
              each={filteredDirectives()}
              fallback={
                <div class="empty-state-dense font-mono" style={{ padding: "1rem" }}>
                  No directives found matching current filter.
                </div>
              }
            >
              {(d) => (
                <div class={`terminal-command-row ${!d.enabled ? "row-disabled" : ""}`}>
                  {/* Left Pill (✓ On / ✕ Off) */}
                  <button
                    type="button"
                    class={`terminal-toggle-pill ${d.enabled ? "pill-enabled" : "pill-disabled"}`}
                    onClick={() => handleToggleDirective(d)}
                    title={d.enabled ? "Click to disable" : "Click to enable"}
                    aria-label={`${d.enabled ? "Disable" : "Enable"} ${d.key}`}
                    disabled={matrixState.isLoading}
                  >
                    <Icon name={d.enabled ? "check" : "x"} size={11} />
                    <span class="pill-status">{d.enabled ? "On" : "Off"}</span>
                    <Icon name="chevronDown" size={11} />
                  </button>

                  {/* Right Compartment */}
                  <div class="terminal-command-content">
                    <Show
                      when={editingKey() === d.key}
                      fallback={
                        <>
                          <div
                            class="terminal-command-text font-mono"
                            title={`${d.key} = ${d.value}`}
                          >
                            <span class="directive-key">{d.key}</span>
                            <span class="directive-equals">=</span>
                            <span class="directive-val">{d.value}</span>
                          </div>

                          <div class="terminal-command-meta font-mono">
                            <span class="section-pill-badge">{d.section}</span>
                            <button
                              type="button"
                              class="terminal-action-btn"
                              onClick={() => handleStartEdit(d)}
                              title="Edit value"
                              aria-label={`Edit ${d.key}`}
                              disabled={matrixState.isLoading}
                            >
                              <Icon name="edit" size={12} />
                            </button>
                            <button
                              type="button"
                              class="terminal-action-btn btn-remove"
                              onClick={() => handleRemoveDirective(d)}
                              title={d.custom ? "Delete custom directive" : "Disable directive"}
                              aria-label={`${d.custom ? "Delete" : "Disable"} ${d.key}`}
                              disabled={matrixState.isLoading}
                            >
                              <Icon name="x" size={12} />
                            </button>
                          </div>
                        </>
                      }
                    >
                      <div class="terminal-inline-edit font-mono">
                        <span class="directive-key">{d.key}</span>
                        <span class="directive-equals">=</span>
                        <input
                          type="text"
                          class="terminal-edit-input"
                          value={editingValue()}
                          onInput={(e) => setEditingValue(e.currentTarget.value)}
                          onKeyDown={(e) => {
                            if (e.key === "Enter") void handleCommitEdit(d.key);
                            if (e.key === "Escape") handleCancelEdit();
                          }}
                        />
                        <button
                          type="button"
                          class="terminal-action-btn"
                          style={{ color: "#4ade80" }}
                          onClick={() => void handleCommitEdit(d.key)}
                          title="Save (Enter)"
                          aria-label={`Save ${d.key}`}
                        >
                          <Icon name="check" size={12} />
                        </button>
                        <button
                          type="button"
                          class="terminal-action-btn"
                          style={{ color: "var(--text-muted)" }}
                          onClick={handleCancelEdit}
                          title="Cancel (Esc)"
                          aria-label="Cancel edit"
                        >
                          <Icon name="x" size={12} />
                        </button>
                      </div>
                    </Show>
                  </div>
                </div>
              )}
            </For>
          </div>
        </div>

        {/* Categorized Extensions Grid */}
        <div class="matrix-categories-list" style={{ "margin-top": "1.25rem" }}>
          <For each={CATEGORY_METADATA}>
            {(cat) => {
              const exts = () => getExtensionsForCategory(cat.id);
              return (
                <Show when={exts().length > 0}>
                  <div class="category-block">
                    <div class="category-header">
                      <div class="category-title-flex">
                        <span class="category-name font-mono">{cat.title}</span>
                        <span class="category-count font-mono">
                          {exts().filter((e) => e.enabled).length}/{exts().length} active
                        </span>
                      </div>
                      <span class="category-subtitle">{cat.subtitle}</span>
                    </div>

                    <div class="ext-grid">
                      <For each={exts()}>
                        {(ext) => (
                          <ExtensionCheckbox
                            extension={ext}
                            disabled={matrixState.isLoading}
                            onToggle={(name, enabled) => void toggleExtension(name, enabled)}
                          />
                        )}
                      </For>
                    </div>
                  </div>
                </Show>
              );
            }}
          </For>
        </div>
      </Show>

      {/* Clone Confirmation Modal */}
      <Show when={showCloneModal()}>
        <div
          class="modal-backdrop"
          onKeyDown={(e) => {
            if (e.key === "Escape") setShowCloneModal(false);
          }}
        >
          <dialog
            open
            aria-modal="true"
            aria-labelledby="clone-modal-title"
            class="clone-modal font-mono"
          >
            <div class="modal-header">
              <h4 id="clone-modal-title" class="font-bold">
                Clone PHP Profile & Extensions
              </h4>
              <button
                type="button"
                class="modal-close-btn"
                aria-label="Close modal"
                onClick={() => setShowCloneModal(false)}
              >
                <Icon name="x" size={14} />
              </button>
            </div>

            <div class="modal-body">
              <p class="modal-desc">
                Copy the full INI profile, limits, and extension state from{" "}
                <span class="highlight-text">
                  {currentInstance()?.displayName} (PHP {currentMatrix()?.version})
                </span>{" "}
                to another installed runtime.
              </p>

              <div class="modal-field">
                <label for="target-php-select" class="modal-label">
                  TARGET PHP RUNTIME:
                </label>
                <select
                  id="target-php-select"
                  class="dense-select font-mono modal-select"
                  value={cloneTargetId()}
                  onChange={(e) => setCloneTargetId(e.currentTarget.value)}
                >
                  <For each={otherPhpInstances()}>
                    {(target) => (
                      <option value={target.instanceId} class="dense-option">
                        {target.displayName} (v{target.version})
                      </option>
                    )}
                  </For>
                </select>
              </div>

              <div class="clone-summary-box">
                <div class="summary-row">
                  <span class="summary-label">Profile Mode:</span>
                  <span class="summary-val text-sky">{currentMatrix()?.profile.toUpperCase()}</span>
                </div>
                <div class="summary-row">
                  <span class="summary-label">INI Limits:</span>
                  <span class="summary-val">
                    {currentMatrix()?.limits.memoryLimit} /{" "}
                    {currentMatrix()?.limits.uploadMaxFilesize} /{" "}
                    {currentMatrix()?.limits.maxExecutionTime}s
                  </span>
                </div>
                <div class="summary-row">
                  <span class="summary-label">Enabled Extensions:</span>
                  <span class="summary-val text-emerald">
                    {enabledCount()} extensions will be synchronized
                  </span>
                </div>
                <div class="summary-note text-slate">
                  Note: Any extension unavailable on the target PHP version will be safely skipped
                  with a warning banner.
                </div>
              </div>
            </div>

            <div class="modal-actions">
              <ActionButton variant="ghost" onClick={() => setShowCloneModal(false)}>
                Cancel
              </ActionButton>
              <ActionButton
                variant="primary"
                onClick={() => void handleConfirmClone()}
                loading={matrixState.isLoading}
              >
                Confirm & Clone
              </ActionButton>
            </div>
          </dialog>
        </div>
      </Show>
    </div>
  );
}
