<script lang="ts">
  import type { ContextBreakdown, ChatTree } from '../types';

  export let breakdown: ContextBreakdown | null = null;
  export let chatTree: ChatTree | null = null;
  export let onOpenAuthorsNote: () => void;
  export let onRefresh: () => void;

  let showBreakdown = false;

  $: percentage = breakdown ? Math.min(Math.max(breakdown.percentage, 0), 100) : 0;
  $: barColor =
    percentage > 90 ? '#f38ba8' : percentage > 70 ? '#f9e2af' : '#a6e3a1';

  $: hasAuthorsNote =
    !!(chatTree?.authors_note && chatTree.authors_note.trim().length > 0);
</script>

<div class="context-bar-container">
  <div class="context-bar-row">
    <!-- Usage Meter & Progress -->
    <div
      class="meter-section"
      on:click={() => (showBreakdown = !showBreakdown)}
      on:keydown={(e) => e.key === 'Enter' && (showBreakdown = !showBreakdown)}
      role="button"
      tabindex="0"
      title="Click to toggle token breakdown details"
    >
      <div class="meter-track">
        <div
          class="meter-fill"
          style="width: {percentage}%; background: {barColor};"
        ></div>
      </div>

      <div class="meter-info">
        <span class="meter-icon">⚡</span>
        {#if breakdown}
          <span class="meter-text">
            <strong>{breakdown.total_tokens.toLocaleString()}</strong> / {breakdown.max_context_tokens.toLocaleString()}
            <span class="pct-badge" style="color: {barColor};">({percentage.toFixed(0)}%)</span>
          </span>
          <span class="free-badge">
            ~{breakdown.free_tokens.toLocaleString()} free
          </span>
        {:else}
          <span class="meter-text">Calculating context...</span>
        {/if}
        <span class="toggle-arrow">{showBreakdown ? '▲' : '▼'}</span>
      </div>
    </div>

    <!-- Actions -->
    <div class="context-actions">
      <button
        type="button"
        class="action-pill-btn {hasAuthorsNote ? 'active-an' : ''}"
        on:click={onOpenAuthorsNote}
        title="Edit Author's Note (A/N) for this chat"
      >
        <span class="an-icon">📝</span>
        <span class="an-label">A/N</span>
        {#if hasAuthorsNote}
          <span class="an-depth-badge">d:{chatTree?.authors_note_depth ?? 1}</span>
        {/if}
      </button>

      <button
        type="button"
        class="action-pill-btn icon-only"
        on:click={onRefresh}
        title="Recalculate token count"
      >
        🔄
      </button>
    </div>
  </div>

  <!-- Detailed Breakdown Dropdown -->
  {#if showBreakdown && breakdown}
    <div class="breakdown-card">
      <div class="breakdown-header">
        <h4>📊 Context Token Breakdown</h4>
        <button
          class="close-btn"
          on:click={() => (showBreakdown = false)}
          type="button"
        >
          ✕
        </button>
      </div>

      <div class="breakdown-grid">
        <div class="breakdown-item">
          <span class="item-label">🖥️ System Prompt</span>
          <span class="item-val">{breakdown.system_tokens.toLocaleString()}</span>
        </div>
        <div class="breakdown-item">
          <span class="item-label">👤 Character Data</span>
          <span class="item-val">{breakdown.character_tokens.toLocaleString()}</span>
        </div>
        <div class="breakdown-item">
          <span class="item-label">🧑 User Persona</span>
          <span class="item-val">{breakdown.user_persona_tokens.toLocaleString()}</span>
        </div>
        <div class="breakdown-item">
          <span class="item-label">📖 Lorebooks</span>
          <span class="item-val">{breakdown.lorebook_tokens.toLocaleString()}</span>
        </div>
        <div class="breakdown-item">
          <span class="item-label">📝 Author's Note</span>
          <span class="item-val">{breakdown.authors_note_tokens.toLocaleString()}</span>
        </div>
        <div class="breakdown-item">
          <span class="item-label">💬 Chat History</span>
          <span class="item-val">{breakdown.history_tokens.toLocaleString()}</span>
        </div>
        {#if breakdown.examples_tokens > 0}
          <div class="breakdown-item">
            <span class="item-label">🎭 Examples</span>
            <span class="item-val">{breakdown.examples_tokens.toLocaleString()}</span>
          </div>
        {/if}
        {#if breakdown.draft_tokens > 0}
          <div class="breakdown-item draft-highlight">
            <span class="item-label">✏️ Draft Input</span>
            <span class="item-val">{breakdown.draft_tokens.toLocaleString()}</span>
          </div>
        {/if}
        <div class="breakdown-item reserved-item">
          <span class="item-label">🎯 Response Budget</span>
          <span class="item-val">+{breakdown.response_tokens.toLocaleString()}</span>
        </div>
      </div>

      <div class="breakdown-footer">
        <span>Total Used: <strong>{breakdown.total_tokens.toLocaleString()}</strong> tokens</span>
        <span>Free Remaining: <strong>{breakdown.free_tokens.toLocaleString()}</strong> tokens</span>
      </div>
    </div>
  {/if}
</div>

<style>
  .context-bar-container {
    background: #181825;
    border-top: 1px solid #28293d;
    padding: 0.35rem 1.2rem;
    font-size: 0.78rem;
    position: relative;
    user-select: none;
  }

  .context-bar-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.8rem;
  }

  .meter-section {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    flex: 1;
    cursor: pointer;
    border-radius: 6px;
    padding: 0.2rem 0.4rem;
    transition: background 0.15s ease;
  }

  .meter-section:hover {
    background: rgba(255, 255, 255, 0.04);
  }

  .meter-track {
    width: 70px;
    height: 5px;
    background: #313244;
    border-radius: 3px;
    overflow: hidden;
    flex-shrink: 0;
  }

  .meter-fill {
    height: 100%;
    transition: width 0.3s ease, background 0.3s ease;
    border-radius: 3px;
  }

  .meter-info {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    color: #a6adc8;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .meter-icon {
    font-size: 0.85rem;
  }

  .meter-text {
    color: #cdd6f4;
  }

  .pct-badge {
    font-weight: 700;
  }

  .free-badge {
    color: #6c7086;
    font-size: 0.73rem;
  }

  .toggle-arrow {
    font-size: 0.65rem;
    color: #6c7086;
  }

  .context-actions {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    flex-shrink: 0;
  }

  .action-pill-btn {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    background: #252538;
    color: #cdd6f4;
    border: 1px solid #3b3d54;
    padding: 0.2rem 0.55rem;
    border-radius: 12px;
    font-size: 0.72rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .action-pill-btn:hover {
    background: #313244;
    border-color: #585b70;
  }

  .action-pill-btn.active-an {
    background: rgba(203, 166, 247, 0.15);
    border-color: #cba6f7;
    color: #cba6f7;
  }

  .an-icon {
    font-size: 0.75rem;
  }

  .an-depth-badge {
    background: rgba(203, 166, 247, 0.25);
    color: #cba6f7;
    padding: 0.05rem 0.3rem;
    border-radius: 4px;
    font-size: 0.65rem;
  }

  .icon-only {
    padding: 0.2rem 0.4rem;
  }

  /* Breakdown Card */
  .breakdown-card {
    margin-top: 0.4rem;
    background: #1e1e2e;
    border: 1px solid #45475a;
    border-radius: 10px;
    padding: 0.7rem 0.9rem;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
    animation: fadeIn 0.15s ease-out;
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .breakdown-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.6rem;
    border-bottom: 1px solid #313244;
    padding-bottom: 0.4rem;
  }

  .breakdown-header h4 {
    margin: 0;
    font-size: 0.82rem;
    color: #cba6f7;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: #6c7086;
    cursor: pointer;
    font-size: 0.8rem;
  }

  .close-btn:hover {
    color: #cdd6f4;
  }

  .breakdown-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 0.45rem;
  }

  .breakdown-item {
    background: rgba(49, 50, 68, 0.4);
    border: 1px solid rgba(69, 71, 90, 0.4);
    border-radius: 6px;
    padding: 0.35rem 0.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }

  .item-label {
    font-size: 0.68rem;
    color: #a6adc8;
  }

  .item-val {
    font-size: 0.78rem;
    font-weight: 700;
    color: #cdd6f4;
  }

  .draft-highlight {
    border-color: #89b4fa;
    background: rgba(137, 180, 250, 0.1);
  }

  .reserved-item {
    border-color: #f9e2af;
    background: rgba(249, 226, 175, 0.08);
  }

  .breakdown-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 0.6rem;
    padding-top: 0.4rem;
    border-top: 1px solid #313244;
    font-size: 0.72rem;
    color: #a6adc8;
  }

  .breakdown-footer strong {
    color: #cdd6f4;
  }

  @media (max-width: 640px) {
    .context-bar-container {
      padding: 0.3rem 0.6rem;
    }

    .meter-track {
      width: 45px;
    }

    .free-badge {
      display: none;
    }

    .breakdown-grid {
      grid-template-columns: 1fr 1fr;
    }
  }
</style>
