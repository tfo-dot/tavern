<script lang="ts">
  import type {
    Character,
    MessageViewNode,
    RegexRule,
    AppSettings,
  } from "../types";
  import { formatMessageContent } from "../formatter";
  export let message: MessageViewNode;
  export let characters: Character[] = [];
  export let characterName = "Character";
  export let characterAvatar: string | null = null;
  export let userName = "You";
  export let userAvatar: string | null = null;
  export let isGenerating = false;
  export let isGroupChat = false;

  export let onSwipe: (parentId: string | null, newIndex: number) => void;
  export let onEdit: (id: string, newContent: string) => void;
  export let onDelete: (id: string) => void;
  export let onOpenImage: ((src: string, alt: string) => void) | undefined =
    undefined;
  export let continuingText: string = "";
  export let onForkChat: ((id: string) => void) | undefined = undefined;
  export let regexRules: RegexRule[] = [];

  let isEditing = false;
  let editDraft = "";
  let copied = false;
  let isChangingSpeaker = false;

  // Touch Swipe Gesture State
  let touchStartX = 0;
  let touchStartY = 0;
  let touchCurrentX = 0;
  let touchCurrentY = 0;
  let isDragging = false;
  let offsetX = 0;

  $: isUser = message.role === "User";
  $: matchedChar =
    !isUser && message.character_id
      ? characters.find((c) => c.id === message.character_id)
      : !isUser && message.name
        ? characters.find(
            (c) =>
              c.card.data.name.toLowerCase() === message.name?.toLowerCase(),
          )
        : null;

  $: displayName = isUser
    ? userName
    : matchedChar?.card.data.name || message.name || characterName;
  $: avatarSrc = isUser
    ? userAvatar
    : matchedChar?.avatar_data_url || characterAvatar;

  $: initials = displayName.slice(0, 2).toUpperCase();
  $: displayContent = message.content + (continuingText || "");
  $: formattedHtml = formatMessageContent(
    displayContent,
    displayName,
    userName,
    regexRules,
  );

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
      console.error("Failed to copy text", e);
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
    if (target && target.tagName === "IMG") {
      const img = target as HTMLImageElement;
      if (img.src && onOpenImage) {
        onOpenImage(img.src, img.alt || "Chat Image");
      }
    }
  }

  function handleTouchStart(e: TouchEvent) {
    if (isGenerating || isEditing || message.sibling_total <= 1) return;
    touchStartX = e.touches[0].clientX;
    touchStartY = e.touches[0].clientY;
    touchCurrentX = touchStartX;
    touchCurrentY = touchStartY;
    isDragging = true;
    offsetX = 0;
  }

  function handleTouchMove(e: TouchEvent) {
    if (!isDragging) return;
    touchCurrentX = e.touches[0].clientX;
    touchCurrentY = e.touches[0].clientY;
    const deltaX = touchCurrentX - touchStartX;
    const deltaY = touchCurrentY - touchStartY;

    // Only capture if primarily horizontal
    if (Math.abs(deltaX) > Math.abs(deltaY) && Math.abs(deltaX) > 8) {
      if (deltaX > 0 && !message.can_swipe_left) {
        offsetX = deltaX * 0.15; // resistance
      } else if (deltaX < 0 && !message.can_swipe_right) {
        offsetX = deltaX * 0.15; // resistance
      } else {
        offsetX = deltaX * 0.45;
      }
    }
  }

  function handleTouchEnd() {
    if (!isDragging) return;
    isDragging = false;
    const deltaX = touchCurrentX - touchStartX;
    const deltaY = touchCurrentY - touchStartY;

    if (Math.abs(deltaX) > Math.abs(deltaY) && Math.abs(deltaX) > 40) {
      if (deltaX < -40 && message.can_swipe_right) {
        onSwipe(message.parent_id, message.sibling_index + 1);
      } else if (deltaX > 40 && message.can_swipe_left) {
        onSwipe(message.parent_id, message.sibling_index - 1);
      }
    }

    offsetX = 0;
  }

  function handleTouchCancel() {
    isDragging = false;
    offsetX = 0;
  }
</script>

<div
  class="message-wrapper {isUser ? 'user-msg' : 'assistant-msg'} {isDragging
    ? 'is-dragging'
    : ''}"
  style="transform: translateX({offsetX}px); transition: {isDragging
    ? 'none'
    : 'transform 0.2s ease'};"
  on:touchstart={handleTouchStart}
  on:touchmove={handleTouchMove}
  on:touchend={handleTouchEnd}
  on:touchcancel={handleTouchCancel}
  role="region"
  aria-label="chat message"
>
  {#if isDragging && offsetX > 25 && message.can_swipe_left}
    <div class="swipe-hint hint-left">
      ◀ Prev ({message.sibling_index} / {message.sibling_total})
    </div>
  {/if}
  {#if isDragging && offsetX < -25 && message.can_swipe_right}
    <div class="swipe-hint hint-right">
      Next ({message.sibling_index + 2} / {message.sibling_total}) ▶
    </div>
  {/if}
  <!-- Avatar -->
  <div class="avatar-col">
    <div class="avatar-wrapper">
      {#if avatarSrc}
        <img
          src={avatarSrc}
          alt={displayName}
          class="avatar-img clickable-avatar"
          on:click={() =>
            avatarSrc && onOpenImage && onOpenImage(avatarSrc, displayName)}
          role="presentation"
        />
      {:else}
        <div
          class="avatar-placeholder {isUser
            ? 'user-avatar'
            : 'char-avatar'} clickable-avatar"
          on:click={() =>
            avatarSrc && onOpenImage && onOpenImage(avatarSrc, displayName)}
          role="presentation"
        >
          {initials}
        </div>
      {/if}
    </div>
  </div>
  <!-- Message Content Column -->
  <div class="message-content-col">
    <!-- Header -->
    <div class="message-header">
      {#if isGroupChat && !isUser && characters.length > 0}
        <div class="speaker-selector-container">
          <button
            type="button"
            class="sender-name-btn"
            on:click={() => (isChangingSpeaker = !isChangingSpeaker)}
            title="Click to reassign speaker"
          >
            <span class="sender-name">{displayName}</span>
            <span class="dropdown-caret">▾</span>
          </button>

          {#if isChangingSpeaker}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <div
              class="speaker-dropdown"
              on:click|stopPropagation
              role="menu"
              tabindex="-1"
            >
              <span class="dropdown-header">Reassign Speaker:</span>
              {#each characters as ch (ch.id)}
                <button
                  type="button"
                  class="dropdown-item {ch.id ===
                  (message.character_id || matchedChar?.id)
                    ? 'active'
                    : ''}"
                >
                  {#if ch.avatar_data_url}
                    <img
                      src={ch.avatar_data_url}
                      alt={ch.card.data.name}
                      class="item-avatar"
                    />
                  {:else}
                    <span class="item-avatar-init"
                      >{ch.card.data.name.slice(0, 2).toUpperCase()}</span
                    >
                  {/if}
                  <span>{ch.card.data.name}</span>
                </button>
              {/each}
            </div>
          {/if}
        </div>
      {:else}
        <span class="sender-name">{displayName}</span>
      {/if}
      <span class="role-badge {isUser ? 'badge-user' : 'badge-assistant'}">
        {isUser ? "User" : "Bot"}
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
        <button
          class="action-icon-btn"
          on:click={copyText}
          title={copied ? "Copied!" : "Copy message"}
        >
          {copied ? "✓" : "📋"}
        </button>
        {#if onForkChat}
          <button
            class="action-icon-btn fork-btn"
            on:click={() => onForkChat && onForkChat(message.id)}
            title="Fork chat from this message"
          >
            🔀
          </button>
        {/if}
        {#if !isEditing}
          <button
            class="action-icon-btn"
            on:click={startEdit}
            title="Edit message"
          >
            ✏️
          </button>
        {/if}
        <button
          class="action-icon-btn delete-btn"
          on:click={() => {
            if (confirm("Delete this message and its branch?"))
              onDelete(message.id);
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
        <div
          class="rendered-text"
          on:click={handleRenderedTextClick}
          role="presentation"
        >
          <!-- eslint-disable-next-line svelte/no-at-html-tags -->
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
    position: relative;
    touch-action: pan-y;
  }

  .swipe-hint {
    position: absolute;
    top: 50%;
    transform: translateY(-50%);
    background: #cba6f7;
    color: #11111b;
    font-weight: 700;
    font-size: 0.78rem;
    padding: 0.35rem 0.75rem;
    border-radius: 20px;
    pointer-events: none;
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.5);
    z-index: 20;
    white-space: nowrap;
  }

  .hint-left {
    left: 0.8rem;
  }

  .hint-right {
    right: 0.8rem;
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
  .rendered-text :global(center) {
    text-align: center;
    margin: 0.5rem 0;
  }

  .rendered-text :global(img) {
    max-width: 100%;
    max-height: 480px;
    height: auto;
    border-radius: 10px;
    margin: 0.6rem auto;
    cursor: pointer;
    object-fit: contain;
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.35);
    display: inline-block;
    vertical-align: middle;
    transition:
      filter 0.15s ease,
      transform 0.15s ease;
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

  .speaker-selector-container {
    position: relative;
    display: inline-block;
  }

  .sender-name-btn {
    background: transparent;
    border: none;
    padding: 0;
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    cursor: pointer;
    color: inherit;
  }

  .sender-name-btn:hover .sender-name {
    color: #cba6f7;
  }

  .dropdown-caret {
    font-size: 0.7rem;
    color: #a6adc8;
  }

  .speaker-dropdown {
    position: absolute;
    top: 100%;
    left: 0;
    background: #181825;
    border: 1px solid #313244;
    border-radius: 8px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
    padding: 0.4rem;
    z-index: 100;
    min-width: 180px;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .dropdown-header {
    font-size: 0.72rem;
    font-weight: 700;
    color: #a6adc8;
    padding: 0.2rem 0.4rem;
    text-transform: uppercase;
  }

  .dropdown-item {
    background: transparent;
    border: none;
    border-radius: 6px;
    color: #cdd6f4;
    font-size: 0.82rem;
    padding: 0.35rem 0.5rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
    text-align: left;
    transition: all 0.15s;
  }

  .dropdown-item:hover {
    background: #313244;
  }

  .dropdown-item.active {
    background: rgba(203, 166, 247, 0.15);
    color: #cba6f7;
    font-weight: 600;
  }

  .item-avatar {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    object-fit: cover;
  }

  .item-avatar-init {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: #313244;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.6rem;
    color: #cdd6f4;
    font-weight: 700;
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
  }

  .avatar-wrapper {
    position: relative;
    display: inline-block;
  }

  @media (max-width: 640px) {
    .message-body {
      font-size: 0.92rem;
      line-height: 1.5;
    }

    .rendered-text :global(img) {
      max-height: 360px;
    }
  }
  :global(.thinking-block) {
    margin: 0.5rem 0 0.8rem 0;
    border: 1px solid rgba(203, 166, 247, 0.22);
    background: rgba(30, 30, 46, 0.6);
    border-radius: 10px;
    overflow: hidden;
    font-size: 0.85rem;
    transition:
      border-color 0.2s ease,
      background 0.2s ease;
  }

  :global(.thinking-block:hover) {
    border-color: rgba(203, 166, 247, 0.4);
    background: rgba(30, 30, 46, 0.8);
  }

  :global(.thinking-summary) {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.45rem 0.75rem;
    cursor: pointer;
    user-select: none;
    background: rgba(49, 50, 68, 0.3);
    font-weight: 600;
    color: #cba6f7;
    font-size: 0.8rem;
    list-style: none;
  }

  :global(.thinking-summary::-webkit-details-marker) {
    display: none;
  }

  :global(.thinking-icon) {
    font-size: 0.9rem;
    flex-shrink: 0;
  }

  :global(.thinking-label) {
    flex: 1;
    letter-spacing: 0.02em;
  }

  :global(.thinking-badge) {
    font-size: 0.68rem;
    padding: 0.12rem 0.4rem;
    border-radius: 6px;
    background: rgba(203, 166, 247, 0.15);
    color: #cba6f7;
    font-weight: 500;
  }

  :global(.thinking-streaming-indicator) {
    font-size: 0.7rem;
    padding: 0.12rem 0.45rem;
    border-radius: 6px;
    background: rgba(137, 180, 250, 0.2);
    color: #89b4fa;
    font-weight: 600;
    animation: thinking-pulse 1.5s infinite ease-in-out;
  }

  @keyframes thinking-pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.45;
    }
  }

  :global(.thinking-content) {
    padding: 0.65rem 0.85rem;
    color: #a6adc8;
    font-style: italic;
    border-top: 1px solid rgba(49, 50, 68, 0.4);
    line-height: 1.5;
    background: rgba(17, 17, 27, 0.25);
  }

  :global(.thinking-content p) {
    margin: 0.25rem 0;
  }
</style>
