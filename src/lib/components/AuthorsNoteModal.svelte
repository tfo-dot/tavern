<script lang="ts">
  import type { ChatTree } from '../types';

  export let isOpen = false;
  export let chatTree: ChatTree | null = null;
  export let onSave: (note: string, depth: number, interval: number) => Promise<void>;
  export let onClose: () => void;

  let noteText = '';
  let depth = 1;
  let interval = 1;
  let isSaving = false;

  $: if (isOpen && chatTree) {
    noteText = chatTree.authors_note || '';
    depth = chatTree.authors_note_depth ?? 1;
    interval = chatTree.authors_note_interval ?? 1;
  }

  async function handleSave() {
    isSaving = true;
    try {
      await onSave(noteText.trim(), depth, interval);
      onClose();
    } catch (e) {
      console.error('Failed to save Author Note:', e);
    } finally {
      isSaving = false;
    }
  }

  function handleClear() {
    noteText = '';
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
          <span class="header-icon">📝</span>
          <h2>Author's Note (A/N)</h2>
        </div>
        <button class="close-btn" on:click={onClose}>✕</button>
      </div>

      <div class="modal-body">
        <p class="modal-intro">
          Author's Note steers the AI's persona, scenario, or tone by dynamically injecting instructions deep into the conversation history.
        </p>

        <!-- A/N Content -->
        <div class="form-group">
          <div class="label-row">
            <label for="an-text">Note Instructions</label>
            {#if noteText}
              <button class="btn-clear" on:click={handleClear} type="button">Clear</button>
            {/if}
          </div>
          <textarea
            id="an-text"
            rows="4"
            bind:value={noteText}
            placeholder="e.g. Write in slow, atmospheric prose. Emphasize dialogue and body language."
          ></textarea>
        </div>

        <!-- Controls Grid -->
        <div class="controls-grid">
          <div class="control-item">
            <div class="control-header">
              <label for="an-depth">Insertion Depth: <strong>{depth}</strong></label>
            </div>
            <input
              id="an-depth"
              type="range"
              min="0"
              max="10"
              step="1"
              bind:value={depth}
            />
            <span class="control-hint">
              {depth === 0 ? '0 = At the very end of history (strongest steering)' : `${depth} message${depth > 1 ? 's' : ''} from the bottom`}
            </span>
          </div>

          <div class="control-item">
            <div class="control-header">
              <label for="an-interval">Update Interval: <strong>{interval}</strong></label>
            </div>
            <input
              id="an-interval"
              type="range"
              min="1"
              max="10"
              step="1"
              bind:value={interval}
            />
            <span class="control-hint">
              {interval === 1 ? '1 = Active every turn' : `Injected every ${interval} turns`}
            </span>
          </div>
        </div>
      </div>

      <div class="modal-footer">
        <button class="btn-cancel" on:click={onClose} type="button">Cancel</button>
        <button
          class="btn-save"
          on:click={handleSave}
          disabled={isSaving}
          type="button"
        >
          {isSaving ? 'Saving...' : 'Save Author Note'}
        </button>
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
    max-width: 520px;
    display: flex;
    flex-direction: column;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.6);
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1rem 1.3rem;
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
    font-size: 1.05rem;
    color: #cdd6f4;
    font-weight: 700;
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
    padding: 1.2rem 1.3rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .modal-intro {
    margin: 0;
    font-size: 0.82rem;
    color: #a6adc8;
    line-height: 1.45;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .label-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  label {
    font-size: 0.82rem;
    font-weight: 600;
    color: #cdd6f4;
  }

  .btn-clear {
    background: transparent;
    border: none;
    color: #f38ba8;
    font-size: 0.75rem;
    cursor: pointer;
  }

  .btn-clear:hover {
    text-decoration: underline;
  }

  textarea {
    background: #181825;
    border: 1px solid #45475a;
    border-radius: 8px;
    padding: 0.6rem 0.8rem;
    color: #cdd6f4;
    font-family: inherit;
    font-size: 0.9rem;
    line-height: 1.4;
    resize: vertical;
    outline: none;
    transition: border-color 0.15s ease;
  }

  textarea:focus {
    border-color: #cba6f7;
  }

  .controls-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
  }

  .control-item {
    background: rgba(49, 50, 68, 0.3);
    border: 1px solid #313244;
    border-radius: 10px;
    padding: 0.75rem 0.85rem;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .control-header strong {
    color: #cba6f7;
  }

  input[type="range"] {
    accent-color: #cba6f7;
    cursor: pointer;
  }

  .control-hint {
    font-size: 0.7rem;
    color: #6c7086;
  }

  .modal-footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0.6rem;
    padding: 0.9rem 1.3rem;
    border-top: 1px solid #313244;
    background: #181825;
  }

  .btn-cancel {
    background: #313244;
    color: #cdd6f4;
    border: none;
    padding: 0.45rem 1rem;
    border-radius: 8px;
    font-weight: 600;
    font-size: 0.85rem;
    cursor: pointer;
  }

  .btn-cancel:hover {
    background: #45475a;
  }

  .btn-save {
    background: #cba6f7;
    color: #11111b;
    border: none;
    padding: 0.45rem 1.2rem;
    border-radius: 8px;
    font-weight: 700;
    font-size: 0.85rem;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .btn-save:hover:not(:disabled) {
    background: #d9bbf9;
    transform: translateY(-1px);
  }

  .btn-save:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  @media (max-width: 640px) {
    .controls-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
