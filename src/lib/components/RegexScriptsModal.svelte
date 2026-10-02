<script lang="ts">
  import type { RegexRule } from '../types';

  export let isOpen = false;
  export let rules: RegexRule[] = [];
  export let onSave: (updated: RegexRule[]) => void;
  export let onClose: () => void;

  let draftRules: RegexRule[] = [];
  let editingRuleId: string | null = null;
  let testInputText = "Akane: Hello Master! (OOC: Just testing the regex engine)";
  let testResultText = "";

  let newRule: RegexRule = {
    id: '',
    name: '',
    pattern: '',
    replacement: '',
    enabled: true,
    case_insensitive: false,
    run_on_output: true,
    run_on_input: false,
    run_on_display: true,
  };

  $: if (isOpen) {
    draftRules = JSON.parse(JSON.stringify(rules || []));
    updateTest();
  }

  $: if (draftRules || testInputText) {
    updateTest();
  }

  function updateTest() {
    let res = testInputText;
    for (const r of draftRules) {
      if (r.enabled && r.pattern) {
        try {
          const flags = r.case_insensitive ? 'gi' : 'g';
          const re = new RegExp(r.pattern, flags);
          res = res.replace(re, r.replacement);
        } catch {
          // Ignore regex syntax errors during interactive typing
        }
      }
    }
    testResultText = res;
  }

  function toggleRule(id: string) {
    draftRules = draftRules.map((r) =>
      r.id === id ? { ...r, enabled: !r.enabled } : r
    );
  }

  function startEditRule(r: RegexRule) {
    editingRuleId = r.id;
    newRule = JSON.parse(JSON.stringify(r));
  }

  function startAddRule() {
    editingRuleId = 'new';
    newRule = {
      id: crypto.randomUUID(),
      name: '',
      pattern: '',
      replacement: '',
      enabled: true,
      case_insensitive: false,
      run_on_output: true,
      run_on_input: false,
      run_on_display: true,
    };
  }

  function cancelRuleEdit() {
    editingRuleId = null;
  }

  function saveRuleEdit() {
    if (!newRule.name.trim() || !newRule.pattern.trim()) {
      alert('Please provide at least a Rule Name and Pattern.');
      return;
    }

    try {
      new RegExp(newRule.pattern);
    } catch (e) {
      alert(`Invalid Regular Expression: ${e}`);
      return;
    }

    if (editingRuleId === 'new') {
      draftRules = [...draftRules, newRule];
    } else {
      draftRules = draftRules.map((r) => (r.id === newRule.id ? newRule : r));
    }
    editingRuleId = null;
  }

  function deleteRule(id: string) {
    draftRules = draftRules.filter((r) => r.id !== id);
    if (editingRuleId === id) editingRuleId = null;
  }

  function applyPreset(presetType: 'strip_name' | 'format_ooc' | 'strip_user') {
    if (presetType === 'strip_name') {
      draftRules = [
        ...draftRules,
        {
          id: crypto.randomUUID(),
          name: 'Strip Character Prefix',
          pattern: '^(?:[A-Za-z0-9_]+:\\s*)+',
          replacement: '',
          enabled: true,
          case_insensitive: true,
          run_on_output: true,
          run_on_input: false,
          run_on_display: true,
        },
      ];
    } else if (presetType === 'format_ooc') {
      draftRules = [
        ...draftRules,
        {
          id: crypto.randomUUID(),
          name: 'Format OOC Brackets',
          pattern: '\\((?:OOC|ooc):?\\s*(.*?)\\)',
          replacement: '<span class="rp-ooc">($1)</span>',
          enabled: true,
          case_insensitive: false,
          run_on_output: false,
          run_on_input: false,
          run_on_display: true,
        },
      ];
    } else if (presetType === 'strip_user') {
      draftRules = [
        ...draftRules,
        {
          id: crypto.randomUUID(),
          name: 'Strip User / You Prefix',
          pattern: '^(?:User|You):\\s*',
          replacement: '',
          enabled: true,
          case_insensitive: true,
          run_on_output: true,
          run_on_input: false,
          run_on_display: true,
        },
      ];
    }
  }

  function handleSaveAll() {
    onSave(draftRules);
    onClose();
  }
</script>

{#if isOpen}
  <div
    class="modal-backdrop"
    on:click={onClose}
    on:keydown={(e) => e.key === 'Escape' && onClose()}
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
        <div class="header-title">
          <span class="header-icon">⚙️</span>
          <h2>Regex Scripts & Post-processing</h2>
        </div>
        <button class="close-btn" on:click={onClose}>✕</button>
      </div>

      <div class="modal-body">
        <p class="intro-desc">
          Define regular expression rules to clean, format, or transform text on LLM generation output, user input, or chat display.
        </p>

        <!-- Presets Row -->
        <div class="presets-bar">
          <span class="presets-label">Quick Presets:</span>
          <button class="preset-chip" on:click={() => applyPreset('strip_name')}>
            + Strip Name Prefix
          </button>
          <button class="preset-chip" on:click={() => applyPreset('format_ooc')}>
            + Format OOC
          </button>
          <button class="preset-chip" on:click={() => applyPreset('strip_user')}>
            + Strip User Prefix
          </button>
        </div>

        <!-- Rules List -->
        <div class="rules-section">
          <div class="rules-header">
            <h3>Active Rules ({draftRules.length})</h3>
            <button class="add-rule-btn" on:click={startAddRule}>
              + Add Rule
            </button>
          </div>

          {#if draftRules.length === 0}
            <div class="empty-rules">
              <p>No regex rules configured. Add one or pick a quick preset above.</p>
            </div>
          {:else}
            <div class="rules-list">
              {#each draftRules as r (r.id)}
                <div class="rule-item {r.enabled ? '' : 'disabled'}">
                  <div class="rule-toggle-col">
                    <input
                      type="checkbox"
                      checked={r.enabled}
                      on:change={() => toggleRule(r.id)}
                      title="Toggle rule enable/disable"
                    />
                  </div>
                  <div class="rule-main-col">
                    <div class="rule-top-row">
                      <span class="rule-name">{r.name}</span>
                      <div class="rule-scope-badges">
                        {#if r.run_on_output}
                          <span class="badge badge-out">Output</span>
                        {/if}
                        {#if r.run_on_display}
                          <span class="badge badge-display">Display</span>
                        {/if}
                        {#if r.run_on_input}
                          <span class="badge badge-in">Input</span>
                        {/if}
                        {#if r.case_insensitive}
                          <span class="badge badge-flag">Aa</span>
                        {/if}
                      </div>
                    </div>
                    <div class="rule-pattern-row">
                      <code>{r.pattern}</code> ➔ <code>{r.replacement || '(empty)'}</code>
                    </div>
                  </div>
                  <div class="rule-actions-col">
                    <button class="icon-action-btn" on:click={() => startEditRule(r)} title="Edit rule">
                      ✏️
                    </button>
                    <button class="icon-action-btn delete-icon" on:click={() => deleteRule(r.id)} title="Delete rule">
                      🗑️
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </div>

        <!-- Inline Rule Editor -->
        {#if editingRuleId !== null}
          <div class="rule-editor-box">
            <h4>{editingRuleId === 'new' ? 'Create New Regex Rule' : 'Edit Regex Rule'}</h4>
            <div class="editor-grid">
              <div class="form-field">
                <label for="rule-name">Rule Name</label>
                <input
                  id="rule-name"
                  type="text"
                  bind:value={newRule.name}
                  placeholder="e.g. Strip Character Prefix"
                />
              </div>
              <div class="form-field">
                <label for="rule-pattern">Regex Pattern</label>
                <input
                  id="rule-pattern"
                  type="text"
                  bind:value={newRule.pattern}
                  placeholder="e.g. ^(?:[A-Za-z0-9_]+:\s*)+"
                />
              </div>
              <div class="form-field">
                <label for="rule-replacement">Replacement Text</label>
                <input
                  id="rule-replacement"
                  type="text"
                  bind:value={newRule.replacement}
                  placeholder="Leave blank for deletion, or $1 for capture groups"
                />
              </div>
            </div>

            <div class="scopes-checkboxes-row">
              <label class="checkbox-label">
                <input type="checkbox" bind:checked={newRule.run_on_output} />
                <span>Run on Output (Clean before saving)</span>
              </label>
              <label class="checkbox-label">
                <input type="checkbox" bind:checked={newRule.run_on_display} />
                <span>Run on Display (Render formatting)</span>
              </label>
              <label class="checkbox-label">
                <input type="checkbox" bind:checked={newRule.run_on_input} />
                <span>Run on Input (Prompt pre-processing)</span>
              </label>
              <label class="checkbox-label">
                <input type="checkbox" bind:checked={newRule.case_insensitive} />
                <span>Case Insensitive (i flag)</span>
              </label>
            </div>

            <div class="editor-actions">
              <button class="btn-cancel" on:click={cancelRuleEdit} type="button">Cancel</button>
              <button class="btn-save" on:click={saveRuleEdit} type="button">Done</button>
            </div>
          </div>
        {/if}

        <!-- Interactive Live Tester -->
        <div class="tester-box">
          <h4>🧪 Live Regex Rule Tester</h4>
          <div class="tester-grid">
            <div class="tester-col">
              <label for="test-input">Test Sample Input</label>
              <textarea id="test-input" rows="2" bind:value={testInputText}></textarea>
            </div>
            <div class="tester-col">
              <label for="test-output">Result Output</label>
              <div id="test-output" class="result-display">{testResultText}</div>
            </div>
          </div>
        </div>
      </div>

      <div class="modal-footer">
        <button class="btn-cancel" on:click={onClose} type="button">Cancel</button>
        <button class="btn-save-all" on:click={handleSaveAll} type="button">Save Rules</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.75);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 120;
    backdrop-filter: blur(4px);
    padding: 1rem;
  }

  .modal-card {
    background: #1e1e2e;
    border: 1px solid #45475a;
    border-radius: 16px;
    width: 100%;
    max-width: 680px;
    max-height: 92vh;
    max-height: 92dvh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.6);
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1rem 1.4rem;
    border-bottom: 1px solid #313244;
  }

  .header-title {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .header-icon {
    font-size: 1.1rem;
  }

  .header-title h2 {
    margin: 0;
    font-size: 1.1rem;
    color: #cdd6f4;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: #6c7086;
    font-size: 1.1rem;
    cursor: pointer;
  }

  .close-btn:hover {
    color: #cdd6f4;
  }

  .modal-body {
    padding: 1.2rem 1.4rem;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 1.1rem;
  }

  .intro-desc {
    margin: 0;
    font-size: 0.82rem;
    color: #a6adc8;
    line-height: 1.4;
  }

  .presets-bar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
    background: rgba(49, 50, 68, 0.3);
    padding: 0.5rem 0.8rem;
    border-radius: 8px;
  }

  .presets-label {
    font-size: 0.75rem;
    color: #a6adc8;
    font-weight: 600;
  }

  .preset-chip {
    background: #181825;
    border: 1px solid #3b3d54;
    color: #cba6f7;
    padding: 0.25rem 0.6rem;
    border-radius: 12px;
    font-size: 0.72rem;
    cursor: pointer;
    font-weight: 600;
    transition: all 0.15s ease;
  }

  .preset-chip:hover {
    background: #313244;
    border-color: #cba6f7;
  }

  .rules-section {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }

  .rules-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .rules-header h3 {
    margin: 0;
    font-size: 0.9rem;
    color: #cdd6f4;
  }

  .add-rule-btn {
    background: #cba6f7;
    color: #11111b;
    border: none;
    padding: 0.3rem 0.75rem;
    border-radius: 6px;
    font-size: 0.75rem;
    font-weight: 700;
    cursor: pointer;
  }

  .add-rule-btn:hover {
    background: #f5c2e7;
  }

  .empty-rules {
    text-align: center;
    padding: 1rem;
    background: #181825;
    border: 1px dashed #45475a;
    border-radius: 8px;
    color: #6c7086;
    font-size: 0.82rem;
  }

  .rules-list {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
  }

  .rule-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    background: #181825;
    border: 1px solid #313244;
    border-radius: 8px;
    padding: 0.6rem 0.8rem;
    transition: all 0.15s ease;
  }

  .rule-item.disabled {
    opacity: 0.6;
    background: rgba(24, 24, 37, 0.5);
  }

  .rule-main-col {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    overflow: hidden;
  }

  .rule-top-row {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }

  .rule-name {
    font-weight: 600;
    font-size: 0.84rem;
    color: #cdd6f4;
  }

  .rule-scope-badges {
    display: flex;
    gap: 0.3rem;
  }

  .badge {
    font-size: 0.65rem;
    padding: 0.1rem 0.35rem;
    border-radius: 4px;
    font-weight: 600;
  }

  .badge-out {
    background: rgba(166, 227, 161, 0.15);
    color: #a6e3a1;
  }

  .badge-display {
    background: rgba(137, 180, 250, 0.15);
    color: #89b4fa;
  }

  .badge-in {
    background: rgba(249, 226, 175, 0.15);
    color: #f9e2af;
  }

  .badge-flag {
    background: rgba(203, 166, 247, 0.15);
    color: #cba6f7;
  }

  .rule-pattern-row {
    font-size: 0.75rem;
    color: #a6adc8;
  }

  .rule-pattern-row code {
    background: #11111b;
    padding: 0.1rem 0.3rem;
    border-radius: 4px;
    font-family: monospace;
    color: #f5c2e7;
  }

  .rule-actions-col {
    display: flex;
    gap: 0.35rem;
  }

  .icon-action-btn {
    background: transparent;
    border: none;
    cursor: pointer;
    font-size: 0.85rem;
    padding: 0.25rem;
    border-radius: 4px;
  }

  .icon-action-btn:hover {
    background: #313244;
  }

  /* Inline Rule Editor */
  .rule-editor-box {
    background: #181825;
    border: 1px solid #cba6f7;
    border-radius: 10px;
    padding: 0.9rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .rule-editor-box h4 {
    margin: 0;
    font-size: 0.88rem;
    color: #cba6f7;
  }

  .editor-grid {
    display: grid;
    grid-template-columns: 1fr;
    gap: 0.6rem;
  }

  .form-field {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  label {
    font-size: 0.75rem;
    font-weight: 600;
    color: #a6adc8;
  }

  input[type="text"],
  textarea {
    background: #11111b;
    border: 1px solid #45475a;
    border-radius: 6px;
    padding: 0.45rem 0.65rem;
    color: #cdd6f4;
    font-size: 0.85rem;
    outline: none;
  }

  input[type="text"]:focus,
  textarea:focus {
    border-color: #cba6f7;
  }

  .scopes-checkboxes-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.45rem;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.75rem;
    color: #cdd6f4;
    cursor: pointer;
  }

  .editor-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    margin-top: 0.25rem;
  }

  .btn-cancel {
    background: #313244;
    color: #cdd6f4;
    border: none;
    padding: 0.35rem 0.85rem;
    border-radius: 6px;
    font-size: 0.78rem;
    font-weight: 600;
    cursor: pointer;
  }

  .btn-save {
    background: #cba6f7;
    color: #11111b;
    border: none;
    padding: 0.35rem 1rem;
    border-radius: 6px;
    font-size: 0.78rem;
    font-weight: 700;
    cursor: pointer;
  }

  /* Tester Box */
  .tester-box {
    background: rgba(49, 50, 68, 0.25);
    border: 1px solid #313244;
    border-radius: 10px;
    padding: 0.8rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }

  .tester-box h4 {
    margin: 0;
    font-size: 0.82rem;
    color: #89b4fa;
  }

  .tester-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.75rem;
  }

  .tester-col {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .result-display {
    background: #11111b;
    border: 1px solid #313244;
    border-radius: 6px;
    padding: 0.45rem 0.65rem;
    font-size: 0.85rem;
    color: #a6e3a1;
    min-height: 52px;
    line-height: 1.4;
    word-break: break-word;
  }

  .modal-footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0.6rem;
    padding: 0.9rem 1.4rem;
    border-top: 1px solid #313244;
    background: #181825;
  }

  .btn-save-all {
    background: #a6e3a1;
    color: #11111b;
    border: none;
    padding: 0.45rem 1.3rem;
    border-radius: 8px;
    font-weight: 700;
    font-size: 0.85rem;
    cursor: pointer;
  }

  @media (max-width: 640px) {
    .scopes-checkboxes-row {
      grid-template-columns: 1fr;
    }
    .tester-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
