<script lang="ts">
  import type { AppSettings } from "../types";
  import { fetchEndpointModels } from "../api";

  export let settings: AppSettings;
  export let isOpen = false;
  export let onSave: (updated: AppSettings) => void;
  export let onClose: () => void;

  let draft: AppSettings;
  let showApiKey = false;
  let isFetchingModels = false;
  let fetchedModels: string[] = [];
  let fetchStatus = "";
  let stopSequenceStr = "";

  const PRESETS = [
    { name: "Ollama", endpoint: "http://localhost:11434/v1", key: "none" },
    { name: "KoboldCpp", endpoint: "http://localhost:5001/v1", key: "none" },
    { name: "LM Studio", endpoint: "http://localhost:1234/v1", key: "none" },
    { name: "OpenRouter", endpoint: "https://openrouter.ai/api/v1", key: "" },
    { name: "OpenAI", endpoint: "https://api.openai.com/v1", key: "" },
  ];

  $: if (isOpen && settings) {
    draft = JSON.parse(JSON.stringify(settings));
    stopSequenceStr = (draft.stop_sequences || []).join(", ");
    fetchStatus = "";
  }

  function applyPreset(preset: {
    name: string;
    endpoint: string;
    key: string;
  }) {
    draft = {
      ...draft,
      endpoint: preset.endpoint,
      api_key: preset.key !== undefined ? preset.key : draft.api_key,
      active_model: "",
    };
    fetchedModels = [];
    fetchStatus = "";
  }

  async function handleFetchModels() {
    if (!draft.endpoint) return;
    isFetchingModels = true;
    fetchStatus = "Connecting...";
    try {
      fetchedModels = await fetchEndpointModels(draft.endpoint, draft.api_key);
      if (fetchedModels.length > 0) {
        fetchStatus = `Found ${fetchedModels.length} models!`;
        if (
          !draft.active_model ||
          !fetchedModels.includes(draft.active_model)
        ) {
          draft.active_model = fetchedModels[0];
        }
      } else {
        fetchStatus = "Connected, but no models found.";
      }
    } catch (e: unknown) {
      if (e instanceof Error) {
        fetchStatus = `Error: ${e.message}`;
      } else {
        fetchStatus = `Error: ${e}`;
      }
    } finally {
      isFetchingModels = false;
    }
  }

  function save() {
    // Parse stop sequences
    draft.stop_sequences = stopSequenceStr
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean);

    onSave(draft);
    onClose();
  }
</script>

{#if isOpen}
  <div
    class="modal-backdrop"
    on:click={onClose}
    on:keydown={(e) => e.key === "Escape" && onClose()}
    role="presentation"
  >
    <div
      class="modal-card"
      on:click|stopPropagation
      on:keydown|stopPropagation
      role="dialog"
      aria-modal="true"
      tabindex="-1"
    >
      <div class="modal-header">
        <h2>⚙️ API & Generation Settings</h2>
        <button class="close-btn" on:click={onClose}>✕</button>
      </div>

      <div class="modal-body">
        <!-- Provider Presets -->
        <div class="form-group">
          <span class="label-heading">Quick Presets</span>
          <div class="presets-row">
            {#each PRESETS as p (p.name)}
              <button
                class="preset-btn {draft.endpoint === p.endpoint
                  ? 'active'
                  : ''}"
                on:click={() => applyPreset(p)}
              >
                {p.name}
              </button>
            {/each}
          </div>
        </div>

        <!-- Endpoint URL -->
        <div class="form-group">
          <label for="endpoint">API Endpoint URL</label>
          <input
            id="endpoint"
            type="text"
            bind:value={draft.endpoint}
            placeholder="http://localhost:11434/v1"
          />
        </div>

        <!-- API Key -->
        <div class="form-group">
          <label for="apikey"
            >API Key (use "none" for local Ollama/Kobold)</label
          >
          <div class="input-with-action">
            <input
              id="apikey"
              type={showApiKey ? "text" : "password"}
              bind:value={draft.api_key}
              placeholder="sk-..."
            />
            <button
              class="toggle-key-btn"
              on:click={() => (showApiKey = !showApiKey)}
              type="button"
            >
              {showApiKey ? "Hide" : "Show"}
            </button>
          </div>
        </div>

        <!-- Model Selection & Fetch -->
        <div class="form-group">
          <div class="label-with-action">
            <label for="model">Model ID</label>
            <button
              class="fetch-btn"
              on:click={handleFetchModels}
              disabled={isFetchingModels || !draft.endpoint}
              type="button"
            >
              {isFetchingModels ? "Fetching..." : "⚡ Fetch Models"}
            </button>
          </div>

          {#if fetchedModels.length > 0}
            <select id="model" bind:value={draft.active_model}>
              {#each fetchedModels as m (m)}
                <option value={m}>{m}</option>
              {/each}
            </select>
          {:else}
            <input
              id="model"
              type="text"
              bind:value={draft.active_model}
              placeholder="e.g. llama3.2, mistral-nemo, gpt-4o-mini"
            />
          {/if}

          {#if fetchStatus}
            <span class="status-hint">{fetchStatus}</span>
          {/if}
        </div>

        <hr class="divider" />

        <!-- Sampling Parameters Grid -->
        <div class="params-grid">
          <div class="param-card">
            <div class="param-header">
              <label for="temp">Temperature</label>
              <span class="param-val">{draft.temperature.toFixed(2)}</span>
            </div>
            <input
              id="temp"
              type="range"
              min="0.0"
              max="2.0"
              step="0.05"
              bind:value={draft.temperature}
            />
          </div>

          <div class="param-card">
            <div class="param-header">
              <label for="top_p">Top P</label>
              <span class="param-val">{draft.top_p.toFixed(2)}</span>
            </div>
            <input
              id="top_p"
              type="range"
              min="0.0"
              max="1.0"
              step="0.05"
              bind:value={draft.top_p}
            />
          </div>

          <div class="param-card">
            <div class="param-header">
              <label for="max_tokens">Max Output Tokens</label>
              <span class="param-val">{draft.max_tokens}</span>
            </div>
            <input
              id="max_tokens"
              type="range"
              min="64"
              max="4096"
              step="32"
              bind:value={draft.max_tokens}
            />
          </div>

          <div class="param-card">
            <div class="param-header">
              <label for="context">Context Limit (Tokens)</label>
              <span class="param-val">{draft.max_context_tokens}</span>
            </div>
            <input
              id="context"
              type="range"
              min="1024"
              max="32768"
              step="512"
              bind:value={draft.max_context_tokens}
            />
          </div>

          <div class="param-card">
            <div class="param-header">
              <label for="freq">Frequency Penalty</label>
              <span class="param-val">{draft.frequency_penalty.toFixed(2)}</span
              >
            </div>
            <input
              id="freq"
              type="range"
              min="-2.0"
              max="2.0"
              step="0.1"
              bind:value={draft.frequency_penalty}
            />
          </div>

          <div class="param-card">
            <div class="param-header">
              <label for="pres">Presence Penalty</label>
              <span class="param-val">{draft.presence_penalty.toFixed(2)}</span>
            </div>
            <input
              id="pres"
              type="range"
              min="-2.0"
              max="2.0"
              step="0.1"
              bind:value={draft.presence_penalty}
            />
          </div>
        </div>

        <!-- Stop Sequences -->
        <div class="form-group">
          <label for="stops">Custom Stop Sequences (comma separated)</label>
          <input
            id="stops"
            type="text"
            bind:value={stopSequenceStr}
            placeholder="e.g. \nUser:, </s>, [END]"
          />
        </div>

        <!-- Global System Template -->
        <div class="form-group">
          <label for="systmpl">
            Global System Prompt Template (Supports <code
              >&#123;&#123;char&#125;&#125;</code
            >, <code>&#123;&#123;user&#125;&#125;</code>,
            <code>&#123;&#123;scenario&#125;&#125;</code>)
          </label>
          <textarea id="systmpl" rows="3" bind:value={draft.system_template}
          ></textarea>
        </div>
      </div>

      <div class="modal-footer">
        <button class="btn-cancel" on:click={onClose}>Cancel</button>
        <button class="btn-save" on:click={save}>Save Settings</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
    backdrop-filter: blur(4px);
  }

  .modal-card {
    background: #1e1e2e;
    border: 1px solid #45475a;
    border-radius: 16px;
    width: 90%;
    max-width: 680px;
    max-height: 90vh;
    max-height: 90dvh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1.2rem 1.5rem;
    border-bottom: 1px solid #313244;
  }

  .modal-header h2 {
    margin: 0;
    font-size: 1.25rem;
    color: #cdd6f4;
  }
  .close-btn {
    background: transparent;
    border: none;
    color: #a6adc8;
    font-size: 1.4rem;
    cursor: pointer;
    padding: 0.5rem 0.8rem;
    border-radius: 8px;
  }

  .modal-body {
    padding: 1.5rem;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 1.2rem;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  label {
    font-size: 0.85rem;
    font-weight: 600;
    color: #a6adc8;
  }

  input[type="text"],
  input[type="password"],
  select,
  textarea {
    background: #181825;
    border: 1px solid #45475a;
    border-radius: 8px;
    padding: 0.6rem 0.8rem;
    color: #cdd6f4;
    font-family: inherit;
    font-size: 0.9rem;
    outline: none;
  }
  input:focus,
  select:focus,
  textarea:focus {
    border-color: #cba6f7;
  }

  .input-with-action {
    display: flex;
    gap: 0.5rem;
  }

  .input-with-action input {
    flex: 1;
  }

  .toggle-key-btn {
    background: #313244;
    border: 1px solid #45475a;
    color: #cdd6f4;
    border-radius: 8px;
    padding: 0 0.8rem;
    cursor: pointer;
    font-size: 0.82rem;
  }

  .presets-row {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
  }

  .preset-btn {
    background: #181825;
    border: 1px solid #313244;
    color: #cdd6f4;
    padding: 0.35rem 0.75rem;
    border-radius: 6px;
    font-size: 0.82rem;
    cursor: pointer;
  }

  .preset-btn.active,
  .preset-btn:hover {
    background: rgba(203, 166, 247, 0.2);
    border-color: #cba6f7;
    color: #cba6f7;
  }

  .label-with-action {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .fetch-btn {
    background: #89b4fa;
    color: #11111b;
    border: none;
    border-radius: 6px;
    padding: 0.25rem 0.6rem;
    font-size: 0.78rem;
    font-weight: 700;
    cursor: pointer;
  }

  .fetch-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .status-hint {
    font-size: 0.78rem;
    color: #cba6f7;
  }

  .divider {
    border: 0;
    border-top: 1px solid #313244;
    margin: 0.2rem 0;
  }

  .params-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
  }

  .param-card {
    background: #181825;
    border: 1px solid #313244;
    border-radius: 8px;
    padding: 0.7rem 0.9rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .param-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .param-val {
    font-family: monospace;
    font-weight: 700;
    color: #cba6f7;
    font-size: 0.88rem;
  }

  input[type="range"] {
    accent-color: #cba6f7;
    cursor: pointer;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.8rem;
    padding: 1.2rem 1.5rem;
    border-top: 1px solid #313244;
  }

  .btn-cancel {
    background: #313244;
    color: #cdd6f4;
    border: none;
    padding: 0.55rem 1.2rem;
    border-radius: 8px;
    cursor: pointer;
    font-weight: 600;
  }

  .btn-save {
    background: #a6e3a1;
    color: #11111b;
    border: none;
    padding: 0.55rem 1.4rem;
    border-radius: 8px;
    cursor: pointer;
    font-weight: 700;
  }
  @media (max-width: 640px) {
    .modal-card {
      width: 95%;
      max-height: 94dvh;
      border-radius: 12px;
    }

    .modal-header {
      padding: 0.9rem 1rem;
    }

    .modal-body {
      padding: 1rem 0.8rem;
      gap: 1rem;
    }

    .params-grid {
      grid-template-columns: 1fr;
      gap: 0.8rem;
    }

    .modal-footer {
      padding: 0.9rem 1rem;
    }
  }
</style>
