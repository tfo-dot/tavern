<script lang="ts">
  export let src: string | null = null;
  export let alt = 'Image preview';
  export let isOpen = false;
  export let onClose: () => void;

  let copied = false;

  async function handleCopyUrl(e: MouseEvent) {
    e.stopPropagation();
    if (!src) return;
    try {
      await navigator.clipboard.writeText(src);
      copied = true;
      setTimeout(() => (copied = false), 2000);
    } catch (e) {
      console.error('Failed to copy URL', e);
    }
  }

  function handleDownload(e: MouseEvent) {
    e.stopPropagation();
    if (!src) return;
    const a = document.createElement('a');
    a.href = src;
    a.download = alt || 'image';
    a.target = '_blank';
    a.click();
  }
</script>

{#if isOpen && src}
  <div
    class="lightbox-backdrop"
    on:click={onClose}
    on:keydown={(e) => e.key === 'Escape' && onClose()}
    role="presentation"
  >
    <!-- Always visible floating close button -->
    <button class="floating-close-btn" on:click|stopPropagation={onClose} title="Close">
      ✕
    </button>

    <div
      class="lightbox-container"
      role="dialog"
      aria-modal="true"
      tabindex="-1"
    >
      <!-- Top Action Controls -->
      <div class="lightbox-toolbar" on:click|stopPropagation role="presentation">
        <span class="lightbox-title">{alt}</span>
        <div class="toolbar-btns">
          {#if src.startsWith('http')}
            <button class="tool-btn" on:click={handleCopyUrl} title="Copy Image URL">
              {copied ? '✓' : '🔗 Link'}
            </button>
          {/if}
          <button class="tool-btn" on:click={handleDownload} title="Save / Download Image">
            📥 Save
          </button>
        </div>
      </div>

      <!-- Preview Image -->
      <div class="image-wrapper" on:click={onClose} role="presentation">
        <img {src} {alt} class="preview-img" on:click|stopPropagation role="presentation" />
      </div>
    </div>
  </div>
{/if}

<style>
  .lightbox-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.92);
    backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 300;
    padding: env(safe-area-inset-top) env(safe-area-inset-right) env(safe-area-inset-bottom)
      env(safe-area-inset-left);
  }

  .floating-close-btn {
    position: fixed;
    top: max(1rem, env(safe-area-inset-top));
    right: max(1rem, env(safe-area-inset-right));
    z-index: 310;
    width: 44px;
    height: 44px;
    border-radius: 50%;
    background: rgba(24, 24, 37, 0.85);
    border: 1px solid rgba(255, 255, 255, 0.2);
    color: #f38ba8;
    font-size: 1.2rem;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.5);
    transition: all 0.15s ease;
  }

  .floating-close-btn:hover {
    background: #f38ba8;
    color: #11111b;
    transform: scale(1.08);
  }

  .lightbox-container {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    max-width: 95vw;
    max-height: 95vh;
    max-height: 95dvh;
    outline: none;
  }

  .lightbox-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.6rem 1rem;
    padding-right: 3.5rem;
    background: rgba(24, 24, 37, 0.85);
    border-radius: 12px;
    margin-bottom: 0.5rem;
    flex-shrink: 0;
    border: 1px solid rgba(255, 255, 255, 0.15);
  }

  .lightbox-title {
    font-weight: 600;
    font-size: 0.88rem;
    color: #cdd6f4;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 60%;
  }

  .toolbar-btns {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .tool-btn {
    background: #313244;
    border: 1px solid #45475a;
    color: #cdd6f4;
    border-radius: 8px;
    padding: 0.35rem 0.75rem;
    font-size: 0.82rem;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .tool-btn:hover {
    background: #45475a;
  }

  .image-wrapper {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    padding: 0.5rem;
    cursor: pointer;
  }

  .preview-img {
    max-width: 94vw;
    max-height: 84vh;
    max-height: 84dvh;
    object-fit: contain;
    border-radius: 12px;
    box-shadow: 0 10px 40px rgba(0, 0, 0, 0.7);
    animation: zoomIn 0.2s ease-out;
  }

  @keyframes zoomIn {
    from {
      transform: scale(0.92);
      opacity: 0;
    }
    to {
      transform: scale(1);
      opacity: 1;
    }
  }

  @media (max-width: 640px) {
    .floating-close-btn {
      width: 38px;
      height: 38px;
      font-size: 1.1rem;
    }

    .lightbox-toolbar {
      padding: 0.5rem 0.8rem;
      padding-right: 3.2rem;
    }

    .lightbox-title {
      font-size: 0.8rem;
      max-width: 50%;
    }

    .tool-btn {
      padding: 0.3rem 0.55rem;
      font-size: 0.75rem;
    }

    .preview-img {
      max-width: 98vw;
      max-height: 80dvh;
    }
  }
</style>
