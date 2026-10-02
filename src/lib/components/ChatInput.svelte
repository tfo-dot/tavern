<script lang="ts">
  import { tick } from "svelte";

  export let isGenerating = false;
  export let placeholder = "Type a message...";

  export let onSend: (text: string) => void;
  export let onStop: () => void;
  export let onDraftChange: ((draft: string) => void) | undefined = undefined;

  export let text = "";
  let textareaEl: HTMLTextAreaElement;

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      submit();
    }
  }

  function submit() {
    if (isGenerating) return;
    const toSend = text;
    text = "";
    adjustHeight();
    onSend(toSend);
  }

  async function adjustHeight() {
    await tick();
    if (textareaEl) {
      textareaEl.style.height = "auto";
      textareaEl.style.height = Math.min(textareaEl.scrollHeight, 200) + "px";
    }
  }

  $: if (text !== undefined) {
    adjustHeight();
    if (onDraftChange) {
      onDraftChange(text);
    }
  }
</script>

<div class="input-container">
  <div class="input-wrapper">
    <textarea
      bind:this={textareaEl}
      bind:value={text}
      on:keydown={handleKeyDown}
      {placeholder}
      rows="1"
      disabled={isGenerating}
    ></textarea>

    <div class="buttons-row">
      {#if isGenerating}
        <button class="stop-btn" on:click={onStop} title="Stop generation">
          <span class="stop-icon">■</span> Stop
        </button>
      {:else}
        <button
          class="send-btn"
          disabled={isGenerating}
          on:click={submit}
          title={text.trim()
            ? "Send message (Enter)"
            : "Send without text / Trigger AI turn (Enter)"}
        >
          Send ➔
        </button>
      {/if}
    </div>
  </div>
  <div class="shortcuts-hint">
    <span
      ><strong>Enter</strong> to send (or generate next turn if empty) &bull;
      <strong>Shift+Enter</strong> for newline</span
    >
  </div>
</div>

<style>
  .input-container {
    padding: 0.8rem 1.2rem;
    padding-bottom: max(0.8rem, env(safe-area-inset-bottom));
    background: #181825;
    border-top: 1px solid #313244;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  .input-wrapper {
    display: flex;
    align-items: flex-end;
    gap: 0.6rem;
    background: #1e1e2e;
    border: 1px solid #45475a;
    border-radius: 12px;
    padding: 0.5rem 0.8rem;
    transition: border-color 0.15s ease;
  }

  .input-wrapper:focus-within {
    border-color: #cba6f7;
    box-shadow: 0 0 0 2px rgba(203, 166, 247, 0.15);
  }

  textarea {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: #cdd6f4;
    font-family: inherit;
    font-size: 0.95rem;
    line-height: 1.4;
    resize: none;
    max-height: 200px;
    padding: 0.3rem 0;
  }

  textarea::placeholder {
    color: #6c7086;
  }

  textarea:disabled {
    opacity: 0.6;
  }

  .buttons-row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .send-btn {
    background: #89b4fa;
    color: #11111b;
    border: none;
    padding: 0.45rem 1rem;
    border-radius: 8px;
    font-weight: 700;
    font-size: 0.88rem;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .send-btn:hover:not(:disabled) {
    background: #b4befe;
    transform: translateY(-1px);
  }

  .send-btn:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .stop-btn {
    background: #f38ba8;
    color: #11111b;
    border: none;
    padding: 0.45rem 1rem;
    border-radius: 8px;
    font-weight: 700;
    font-size: 0.88rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 0.35rem;
    animation: pulse 1.5s infinite ease-in-out;
  }

  .stop-icon {
    font-size: 0.8rem;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.75;
    }
  }

  .shortcuts-hint {
    font-size: 0.72rem;
    color: #6c7086;
    text-align: right;
    padding-right: 0.2rem;
  }
  @media (max-width: 640px) {
    .input-container {
      padding: 0.4rem 0.6rem;
      padding-bottom: max(0.5rem, env(safe-area-inset-bottom));
    }

    .input-wrapper {
      padding: 0.45rem 0.6rem;
      gap: 0.4rem;
      flex-direction: column;
      align-items: stretch;
    }

    textarea {
      font-size: 0.95rem;
      width: 100%;
    }

    .buttons-row {
      display: flex;
      align-items: center;
      justify-content: flex-end;
      gap: 0.35rem;
      flex-wrap: wrap;
      border-top: 1px solid rgba(255, 255, 255, 0.05);
      padding-top: 0.35rem;
    }

    .send-btn,
    .stop-btn {
      padding: 0.35rem 0.85rem;
      font-size: 0.82rem;
    }

    .shortcuts-hint {
      display: none;
    }
  }
</style>
