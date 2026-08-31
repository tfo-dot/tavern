<script lang="ts">
  import type { MessageViewNode } from '../types';
  import { formatMessageContent } from '../formatter';

  export let message: MessageViewNode;
  export let characterName = 'Character';
  export let characterAvatar: string | null = null;
  export let userName = 'You';
  export let userAvatar: string | null = null;
  export let isGenerating = false;
  export let isLastMessage = false;

  export let onSwipe: (parentId: string | null, newIndex: number) => void;
  export let onRegenerateSwipe: () => void;
  export let onEdit: (id: string, newContent: string) => void;
  export let onDelete: (id: string) => void;
  export let onOpenImage: ((src: string, alt: string) => void) | undefined = undefined;

  let isEditing = false;
  let editDraft = '';
  let copied = false;

  $: isUser = message.role === 'User';
  $: displayName = isUser ? userName : characterName;
  $: avatarSrc = isUser ? userAvatar : characterAvatar;
  $: initials = displayName.slice(0, 2).toUpperCase();
  $: formattedHtml = formatMessageContent(message.content, characterName, userName);

  function startEdit() {
    editDraft = message.content;
    isEditing = true;
  }

  function saveEdit() {
    if (editDraft.trim() && editDraft !== message.content) {
      onEdit(message.id, editDraft.trim());
    }
    isEditing = false;
  }

  function cancelEdit() {
    isEditing = false;
  }

  async function copyText() {
    try {
      await navigator.clipboard.writeText(message.content);
      copied = true;
      setTimeout(() => (copied = false), 2000);
    } catch (e) {
      console.error('Failed to copy text', e);
    }
  }

  function handlePrevSwipe() {
    if (message.can_swipe_left && !isGenerating) {
      onSwipe(message.parent_id, message.sibling_index - 1);
    }
  }

  function handleNextSwipe() {
    if (message.can_swipe_right && !isGenerating) {
      onSwipe(message.parent_id, message.sibling_index + 1);
    }
  }

  function handleRenderedTextClick(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (target && target.tagName === 'IMG') {
      const img = target as HTMLImageElement;
      if (img.src && onOpenImage) {
        onOpenImage(img.src, img.alt || 'Chat Image');
      }
    }
  }
</script>

<div class="message-wrapper {isUser ? 'user-msg' : 'assistant-msg'}">
  <!-- Avatar -->
  <div class="avatar-col">
    {#if avatarSrc}
      <img
        src={avatarSrc}
        alt={displayName}
        class="avatar-img clickable-avatar"
        on:click={() => avatarSrc && onOpenImage && onOpenImage(avatarSrc, displayName)}
        role="presentation"
      />
    {:else}
      <div
        class="avatar-placeholder {isUser ? 'user-avatar' : 'char-avatar'} clickable-avatar"
        on:click={() => avatarSrc && onOpenImage && onOpenImage(avatarSrc, displayName)}
        role="presentation"
      >
        {initials}
      </div>
    {/if}
  </div>

  <!-- Message Content Column -->
  <div class="message-content-col">
    <!-- Header -->
    <div class="message-header">
      <span class="sender-name">{displayName}</span>
      <span class="role-badge {isUser ? 'badge-user' : 'badge-assistant'}">
        {isUser ? 'User' : 'Character'}
      </span>

      <!-- Swipe Pagination if multiple branches exist -->
      {#if !isUser && message.sibling_total > 1}
        <div class="swipe-controls">
          <button
            class="swipe-btn"
            disabled={!message.can_swipe_left || isGenerating}
            on:click={handlePrevSwipe}
            title="Previous response variation"
          >
            &lt;
          </button>
          <span class="swipe-indicator">
            {message.sibling_index + 1} / {message.sibling_total}
          </span>
          <button
            class="swipe-btn"
            disabled={!message.can_swipe_right || isGenerating}
            on:click={handleNextSwipe}
            title="Next response variation"
          >
            &gt;
          </button>
        </div>
      {/if}

      <!-- Message Actions Toolbar -->
      <div class="message-actions">
        {#if !isUser && isLastMessage}
          <button
            class="action-icon-btn swipe-new-btn"
            disabled={isGenerating}
            on:click={onRegenerateSwipe}
            title="Regenerate / Swipe alternate reply"
          >
            🔄 Swipe
          </button>
        {/if}
        <button
          class="action-icon-btn"
          on:click={copyText}
          title={copied ? 'Copied!' : 'Copy message'}
        >
          {copied ? '✓' : '📋'}
        </button>
        {#if !isEditing}
          <button class="action-icon-btn" on:click={startEdit} title="Edit message">
            ✏️
          </button>
        {/if}
        <button
          class="action-icon-btn delete-btn"
          on:click={() => {
            if (confirm('Delete this message and its branch?')) onDelete(message.id);
          }}
          title="Delete message"
        >
          🗑️
        </button>
      </div>
    </div>

    <!-- Body / Text -->
    <div class="message-body">
      {#if isEditing}
        <div class="edit-box">
          <textarea bind:value={editDraft} rows="4"></textarea>
          <div class="edit-actions">
            <button class="btn-cancel" on:click={cancelEdit}>Cancel</button>
            <button class="btn-save" on:click={saveEdit}>Save</button>
          </div>
        </div>
      {:else}
        <div class="rendered-text" on:click={handleRenderedTextClick} role="presentation">
          {@html formattedHtml}
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .message-wrapper {
    display: flex;
    gap: 0.85rem;
    padding: 0.9rem 1.1rem;
    border-radius: 12px;
    transition: background 0.15s ease;
  }

  .message-wrapper:hover {
    background: rgba(255, 255, 255, 0.025);
  }

  .user-msg {
    background: rgba(137, 180, 250, 0.05);
    border-left: 3px solid #89b4fa;
  }

  .assistant-msg {
    background: rgba(49, 50, 68, 0.35);
    border-left: 3px solid #cba6f7;
  }

  .avatar-col {
    flex-shrink: 0;
  }

  .avatar-img {
    width: 42px;
    height: 42px;
    border-radius: 50%;
    object-fit: cover;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.3);
  }

  .clickable-avatar {
    cursor: pointer;
    transition: transform 0.15s ease;
  }

  .clickable-avatar:hover {
    transform: scale(1.08);
  }

  .avatar-placeholder {
    width: 42px;
    height: 42px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 700;
    font-size: 0.85rem;
    color: #11111b;
  }

  .user-avatar {
    background: linear-gradient(135deg, #89b4fa, #74c7ec);
  }

  .char-avatar {
    background: linear-gradient(135deg, #cba6f7, #f5c2e7);
  }

  .message-content-col {
    flex: 1;
    min-width: 0;
  }

  .message-header {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    margin-bottom: 0.4rem;
    flex-wrap: wrap;
  }

  .sender-name {
    font-weight: 700;
    font-size: 0.95rem;
    color: #cdd6f4;
  }

  .role-badge {
    font-size: 0.7rem;
    padding: 0.15rem 0.45rem;
    border-radius: 4px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .badge-user {
    background: rgba(137, 180, 250, 0.2);
    color: #89b4fa;
  }

  .badge-assistant {
    background: rgba(203, 166, 247, 0.2);
    color: #cba6f7;
  }

  .swipe-controls {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    background: #181825;
    border-radius: 6px;
    padding: 0.1rem 0.35rem;
    border: 1px solid #313244;
  }

  .swipe-btn {
    background: none;
    border: none;
    color: #a6adc8;
    cursor: pointer;
    font-size: 0.8rem;
    padding: 0.1rem 0.3rem;
    border-radius: 3px;
  }

  .swipe-btn:hover:not(:disabled) {
    background: #313244;
    color: #cdd6f4;
  }

  .swipe-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .swipe-indicator {
    font-size: 0.75rem;
    color: #cba6f7;
    font-weight: 600;
    min-width: 35px;
    text-align: center;
  }

  .message-actions {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 0.3rem;
    opacity: 0.8;
  }

  .message-wrapper:hover .message-actions {
    opacity: 1;
  }

  .action-icon-btn {
    background: rgba(24, 24, 37, 0.6);
    border: 1px solid #313244;
    color: #a6adc8;
    padding: 0.2rem 0.45rem;
    border-radius: 6px;
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .action-icon-btn:hover {
    background: #313244;
    color: #cdd6f4;
  }

  .swipe-new-btn {
    background: rgba(203, 166, 247, 0.15);
    border-color: rgba(203, 166, 247, 0.4);
    color: #cba6f7;
    font-weight: 600;
  }

  .swipe-new-btn:hover:not(:disabled) {
    background: rgba(203, 166, 247, 0.3);
    color: #f5c2e7;
  }

  .delete-btn:hover {
    color: #f38ba8;
    border-color: #f38ba8;
  }

  .message-body {
    color: #cdd6f4;
    line-height: 1.6;
    font-size: 0.95rem;
    word-break: break-word;
  }

  .rendered-text :global(p) {
    margin: 0.4rem 0;
  }

  .rendered-text :global(p:first-child) {
    margin-top: 0;
  }

  .rendered-text :global(p:last-child) {
    margin-bottom: 0;
  }

  .rendered-text :global(.rp-action) {
    font-style: italic;
    color: #bac2de;
  }

  .rendered-text :global(.rp-speech) {
    color: #ffe066;
    font-weight: 600;
    text-shadow: 0 0 1px rgba(255, 224, 102, 0.4);
  }
  .rendered-text :global(img) {
    max-width: 100%;
    max-height: 480px;
    height: auto;
    border-radius: 10px;
    margin: 0.6rem 0;
    cursor: pointer;
    object-fit: contain;
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.35);
    display: block;
    transition: filter 0.15s ease, transform 0.15s ease;
  }

  .rendered-text :global(img:hover) {
    filter: brightness(1.05);
    transform: scale(1.01);
  }

  .rendered-text :global(pre) {
    background: #11111b;
    padding: 0.75rem;
    border-radius: 8px;
    overflow-x: auto;
    border: 1px solid #313244;
  }

  .rendered-text :global(code) {
    font-family: monospace;
    font-size: 0.88rem;
    background: #181825;
    padding: 0.15rem 0.35rem;
    border-radius: 4px;
    color: #f9e2af;
  }

  .rendered-text :global(table) {
    width: 100%;
    border-collapse: collapse;
    margin: 0.6rem 0;
  }

  .rendered-text :global(th),
  .rendered-text :global(td) {
    border: 1px solid #45475a;
    padding: 0.4rem 0.6rem;
  }

  .rendered-text :global(th) {
    background: #181825;
  }

  .edit-box {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .edit-box textarea {
    width: 100%;
    background: #181825;
    border: 1px solid #45475a;
    border-radius: 8px;
    color: #cdd6f4;
    padding: 0.6rem;
    font-family: inherit;
    font-size: 0.95rem;
    resize: vertical;
    outline: none;
    box-sizing: border-box;
  }

  .edit-box textarea:focus {
    border-color: #cba6f7;
  }

  .edit-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }

  .btn-cancel {
    background: #313244;
    border: none;
    color: #cdd6f4;
    padding: 0.35rem 0.8rem;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.85rem;
  }

  .btn-save {
    background: #a6e3a1;
    border: none;
    color: #11111b;
    font-weight: 600;
    padding: 0.35rem 0.8rem;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.85rem;
  }

  @media (max-width: 640px) {
    .message-wrapper {
      padding: 0.75rem 0.7rem;
      gap: 0.6rem;
      border-radius: 10px;
    }

    .avatar-img,
    .avatar-placeholder {
      width: 36px;
      height: 36px;
      font-size: 0.78rem;
    }

    .message-header {
      gap: 0.35rem;
      margin-bottom: 0.3rem;
    }

    .sender-name {
      font-size: 0.88rem;
    }

    .role-badge {
      font-size: 0.65rem;
      padding: 0.1rem 0.35rem;
    }

    .swipe-controls {
      padding: 0.15rem 0.3rem;
      gap: 0.2rem;
    }

    .swipe-btn {
      padding: 0.25rem 0.45rem;
      font-size: 0.85rem;
      min-width: 28px;
    }

    .message-actions {
      opacity: 1;
      gap: 0.25rem;
    }

    .action-icon-btn {
      padding: 0.3rem 0.45rem;
      font-size: 0.75rem;
    }

    .message-body {
      font-size: 0.92rem;
      line-height: 1.5;
    }

    .rendered-text :global(img) {
      max-height: 360px;
    }
  }
</style>
