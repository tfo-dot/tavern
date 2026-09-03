<script lang="ts">
  import { formatCreatorNotes } from "../formatter";

  export let characterName = "Character";
  export let creatorNotes = "";
  export let characterAvatar: string | null = null;
  export let isOpen = false;
  export let onOpenImagePreview:
    | ((src: string, alt: string) => void)
    | undefined = undefined;
  export let onClose: () => void;

  $: formattedHtml = formatCreatorNotes(creatorNotes);

  function handleContentClick(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (target && target.tagName === "IMG") {
      const img = target as HTMLImageElement;
      if (img.src && onOpenImagePreview) {
        onOpenImagePreview(img.src, img.alt || `${characterName} Notes Image`);
      }
    }
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
        <div class="header-title-group">
          {#if characterAvatar}
            <img
              src={characterAvatar}
              alt={characterName}
              class="header-avatar"
            />
          {/if}
          <div class="title-details">
            <h2>📜 Creator Notes</h2>
            <span class="char-subtitle">{characterName}</span>
          </div>
        </div>
        <button class="close-btn" on:click={onClose} title="Close">✕</button>
      </div>

      <div class="modal-body">
        {#if !creatorNotes.trim()}
          <div class="empty-notes">
            <p>No creator notes provided for this character.</p>
          </div>
        {:else}
          <div
            class="rendered-notes-container"
            on:click={handleContentClick}
            role="presentation"
          >
            <!-- eslint-disable-next-line svelte/no-at-html-tags -->
            {@html formattedHtml}
          </div>
        {/if}
      </div>

      <div class="modal-footer">
        <button class="btn-close-bottom" on:click={onClose}>Close</button>
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
    z-index: 150;
    backdrop-filter: blur(4px);
  }

  .modal-card {
    background: #1e1e2e;
    border: 1px solid #45475a;
    border-radius: 16px;
    width: 92%;
    max-width: 820px;
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

  .header-title-group {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .header-avatar {
    width: 38px;
    height: 38px;
    border-radius: 50%;
    object-fit: cover;
    border: 2px solid #cba6f7;
  }

  .title-details {
    display: flex;
    flex-direction: column;
  }

  .modal-header h2 {
    margin: 0;
    font-size: 1.2rem;
    color: #cdd6f4;
  }

  .char-subtitle {
    font-size: 0.78rem;
    color: #a6adc8;
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
    padding: 1.2rem;
    overflow-y: auto;
    flex: 1;
  }

  .rendered-notes-container {
    color: #cdd6f4;
    line-height: 1.6;
    word-break: break-word;
  }

  .rendered-notes-container :global(img) {
    max-width: 100%;
    height: auto;
    border-radius: 8px;
    cursor: pointer;
    transition: transform 0.15s ease;
  }

  .rendered-notes-container :global(img:hover) {
    transform: scale(1.01);
  }

  .rendered-notes-container :global(table) {
    width: 100%;
    border-collapse: collapse;
    margin: 0.8rem 0;
  }

  .rendered-notes-container :global(th),
  .rendered-notes-container :global(td) {
    border: 1px solid #45475a;
    padding: 0.5rem 0.75rem;
  }

  .rendered-notes-container :global(th) {
    background: #181825;
    font-weight: 700;
  }

  .rendered-notes-container :global(a) {
    color: #89b4fa;
    text-decoration: underline;
  }

  .empty-notes {
    padding: 3rem 1rem;
    text-align: center;
    color: #6c7086;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    padding: 1rem 1.5rem;
    border-top: 1px solid #313244;
  }

  .btn-close-bottom {
    background: #313244;
    color: #cdd6f4;
    border: none;
    padding: 0.5rem 1.2rem;
    border-radius: 8px;
    cursor: pointer;
    font-weight: 600;
  }

  .btn-close-bottom:hover {
    background: #45475a;
  }

  @media (max-width: 640px) {
    .modal-card {
      width: 95%;
      max-height: 94dvh;
      border-radius: 12px;
    }

    .modal-header {
      padding: 0.8rem 1rem;
    }

    .modal-body {
      padding: 0.8rem;
    }

    .modal-footer {
      padding: 0.8rem 1rem;
    }
  }
</style>
