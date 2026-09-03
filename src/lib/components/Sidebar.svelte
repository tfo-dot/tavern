<script lang="ts">
  import type { Character, ChatSummary, Group } from '../types';

  export let characters: Character[] = [];
  export let activeCharacterId: string | null = null;
  export let groups: Group[] = [];
  export let activeGroupId: string | null = null;
  export let chats: ChatSummary[] = [];
  export let activeChatId: string | null = null;
  export let isOpen = true;

  export let onSelectCharacter: (id: string) => void;
  export let onCreateCharacter: () => void;
  export let onImportCard: (file: File) => void;
  export let onSelectGroup: (id: string) => void;
  export let onCreateGroup: () => void;
  export let onEditGroup: ((group: Group) => void) | undefined = undefined;
  export let onDeleteGroup: ((id: string) => void) | undefined = undefined;
  export let onNewGroupChat: ((groupId: string) => void) | undefined = undefined;
  export let onSelectChat: (id: string) => void;
  export let onNewChat: (characterId: string) => void;
  export let onDeleteChat: (id: string) => void;
  export let onDeleteCharacter: (id: string) => void;
  export let onOpenLorebooks: (() => void) | undefined = undefined;
  export let onOpenSync: (() => void) | undefined = undefined;
  export let onImportChat: ((file: File) => void) | undefined = undefined;
  export let onExportChat: ((id: string, title: string) => void) | undefined = undefined;
  export let onClose: () => void;

  let activeTab: 'characters' | 'groups' | 'chats' = 'characters';
  let searchQuery = '';
  let groupSearchQuery = '';
  let fileInputEl: HTMLInputElement;
  let chatFileInputEl: HTMLInputElement;

  $: filteredCharacters = characters.filter((c) => {
    if (!searchQuery.trim()) return true;
    const q = searchQuery.toLowerCase();
    const nameMatch = c.card.data.name.toLowerCase().includes(q);
    const descMatch = c.card.data.description.toLowerCase().includes(q);
    const tagMatch = c.card.data.tags?.some((t) => t.toLowerCase().includes(q));
    return nameMatch || descMatch || tagMatch;
  });

  $: filteredGroups = groups.filter((g) => {
    if (!groupSearchQuery.trim()) return true;
    const q = groupSearchQuery.toLowerCase();
    return g.name.toLowerCase().includes(q) || g.description.toLowerCase().includes(q);
  });

  $: activeChar = characters.find((c) => c.id === activeCharacterId);
  $: activeGroup = groups.find((g) => g.id === activeGroupId);

  function handleFileSelected(e: Event) {
    const target = e.target as HTMLInputElement;
    if (target.files && target.files[0]) {
      onImportCard(target.files[0]);
      target.value = '';
      if (window.innerWidth <= 768) onClose();
    }
  }
  function triggerFileInput() {
    if (fileInputEl) fileInputEl.click();
  }

  function handleSelectChar(id: string) {
    onSelectCharacter(id);
    if (window.innerWidth <= 768) onClose();
  }

  function handleSelectGroup(id: string) {
    onSelectGroup(id);
    if (window.innerWidth <= 768) onClose();
  }

  function handleSelectChat(id: string) {
    onSelectChat(id);
    if (window.innerWidth <= 768) onClose();
  }

  function handleNewChat(characterId: string) {
    onNewChat(characterId);
    if (window.innerWidth <= 768) onClose();
  }

  function handleNewGrpChat(groupId: string) {
    if (onNewGroupChat) onNewGroupChat(groupId);
    if (window.innerWidth <= 768) onClose();
  }
  function handleChatFileSelected(e: Event) {
    const target = e.target as HTMLInputElement;
    if (target.files && target.files[0] && onImportChat) {
      onImportChat(target.files[0]);
      target.value = '';
      if (window.innerWidth <= 768) onClose();
    }
  }

  function triggerChatFileInput() {
    if (chatFileInputEl) chatFileInputEl.click();
  }
</script>

<input
  type="file"
  accept=".png,.json"
  bind:this={fileInputEl}
  on:change={handleFileSelected}
  style="display: none;"
/>
<input
  type="file"
  accept=".jsonl,.json"
  bind:this={chatFileInputEl}
  on:change={handleChatFileSelected}
  style="display: none;"
/>


<!-- Mobile Backdrop -->
<div
  class="sidebar-backdrop {isOpen ? 'visible' : ''}"
  on:click={onClose}
  on:keydown={(e) => e.key === 'Escape' && onClose()}
  role="presentation"
></div>

<aside class="sidebar {isOpen ? 'open' : 'closed'}">
  <!-- Top Bar in Sidebar -->
  <div class="sidebar-header">
    <div class="brand">
      <span class="logo-icon">🏰</span>
      <span class="brand-title">Tavern</span>
    </div>
    <button class="icon-btn close-sidebar-btn" on:click={onClose} title="Close sidebar">✕</button>
  </div>

  <!-- Tab Switcher -->
  <div class="tab-row">
    <button
      class="tab-btn {activeTab === 'characters' ? 'active' : ''}"
      on:click={() => (activeTab = 'characters')}
    >
      Characters ({characters.length})
    </button>
    <button
      class="tab-btn {activeTab === 'groups' ? 'active' : ''}"
      on:click={() => (activeTab = 'groups')}
    >
      Groups ({groups.length})
    </button>
    <button
      class="tab-btn {activeTab === 'chats' ? 'active' : ''}"
      on:click={() => (activeTab = 'chats')}
      disabled={!activeCharacterId && !activeGroupId}
    >
      Chats ({chats.length})
    </button>
  </div>
  <!-- Tab Content: Characters -->
  {#if activeTab === 'characters'}
    <div class="characters-tab">
      <div class="search-and-actions">
        <input
          type="text"
          placeholder="Search characters or tags..."
          bind:value={searchQuery}
          class="search-input"
        />
        <div class="actions-row">
          <button
            class="primary-action-btn"
            on:click={() => {
              onCreateCharacter();
              if (window.innerWidth <= 768) onClose();
            }}
          >
            + Create
          </button>
          <button class="secondary-action-btn" on:click={triggerFileInput} title="Import PNG card or JSON">
            📥 Import
          </button>
        </div>
      </div>

      <div class="character-list">
        {#if filteredCharacters.length === 0}
          <div class="empty-state">
            <p>No characters found.</p>
            <button
              class="link-btn"
              on:click={() => {
                onCreateCharacter();
                if (window.innerWidth <= 768) onClose();
              }}
            >
              Create one now
            </button>
          </div>
        {:else}
          {#each filteredCharacters as char}
            <div
              class="character-card {char.id === activeCharacterId ? 'active' : ''}"
              role="button"
              tabindex="0"
              on:click={() => handleSelectChar(char.id)}
              on:keydown={(e) => e.key === 'Enter' && handleSelectChar(char.id)}
            >
              {#if char.avatar_data_url}
                <img src={char.avatar_data_url} alt={char.card.data.name} class="char-avatar-img" />
              {:else}
                <div class="char-avatar-placeholder">
                  {char.card.data.name.slice(0, 2).toUpperCase()}
                </div>
              {/if}
              <div class="char-info">
                <div class="char-name-row">
                  <span class="char-name">{char.card.data.name}</span>
                  {#if char.id === activeCharacterId}
                    <span class="active-pill">Active</span>
                  {/if}
                </div>
                <p class="char-desc">
                  {char.card.data.description || char.card.data.personality || 'No description'}
                </p>
                {#if char.card.data.tags && char.card.data.tags.length > 0}
                  <div class="char-tags">
                    {#each char.card.data.tags.slice(0, 3) as tag}
                      <span class="tag-pill">{tag}</span>
                    {/each}
                  </div>
                {/if}
              </div>
              <button
                class="char-delete-btn"
                on:click|stopPropagation={() => {
                  if (confirm(`Delete character "${char.card.data.name}"?`)) {
                    onDeleteCharacter(char.id);
                  }
                }}
                title="Delete character"
              >
                🗑️
              </button>
            </div>
          {/each}
        {/if}
      </div>
    </div>
  {/if}

  <!-- Tab Content: Groups -->
  {#if activeTab === 'groups'}
    <div class="characters-tab">
      <div class="search-and-actions">
        <input
          type="text"
          placeholder="Search roleplay groups..."
          bind:value={groupSearchQuery}
          class="search-input"
        />
        <div class="actions-row">
          <button
            class="primary-action-btn"
            on:click={() => {
              onCreateGroup();
              if (window.innerWidth <= 768) onClose();
            }}
          >
            + New Group
          </button>
        </div>
      </div>

      <div class="character-list">
        {#if filteredGroups.length === 0}
          <div class="empty-state">
            <p>No roleplay groups created yet.</p>
            <button
              class="link-btn"
              on:click={() => {
                onCreateGroup();
                if (window.innerWidth <= 768) onClose();
              }}
            >
              Create a group now
            </button>
          </div>
        {:else}
          {#each filteredGroups as grp}
            <div
              class="character-card {grp.id === activeGroupId ? 'active' : ''}"
              role="button"
              tabindex="0"
              on:click={() => handleSelectGroup(grp.id)}
              on:keydown={(e) => e.key === 'Enter' && handleSelectGroup(grp.id)}
            >
              {#if grp.avatar_data_url}
                <img src={grp.avatar_data_url} alt={grp.name} class="char-avatar-img" />
              {:else}
                <div class="char-avatar-placeholder">
                  👥
                </div>
              {/if}
              <div class="char-info">
                <div class="char-name-row">
                  <span class="char-name">{grp.name}</span>
                  {#if grp.id === activeGroupId}
                    <span class="active-pill">Active</span>
                  {/if}
                </div>
                <p class="char-desc">
                  {grp.description || `${grp.members.length} characters in group`}
                </p>
                <div class="char-tags">
                  <span class="tag-pill">{grp.members.length} members</span>
                  <span class="tag-pill mode-tag">{grp.turn_mode} turn</span>
                </div>
              </div>

              <div class="group-card-side-actions">
                {#if onEditGroup}
                  <button
                    class="char-edit-btn"
                    on:click|stopPropagation={() => {
                      if (onEditGroup) onEditGroup(grp);
                      if (window.innerWidth <= 768) onClose();
                    }}
                    title="Edit group settings and members"
                  >
                    ⚙
                  </button>
                {/if}
                {#if onDeleteGroup}
                  <button
                    class="char-delete-btn"
                    on:click|stopPropagation={() => {
                      if (confirm(`Delete group "${grp.name}"?`)) {
                        if (onDeleteGroup) onDeleteGroup(grp.id);
                      }
                    }}
                    title="Delete group"
                  >
                    🗑️
                  </button>
                {/if}
              </div>
            </div>
          {/each}
        {/if}
      </div>
    </div>
  {/if}

  <!-- Tab Content: Chats -->
  {#if activeTab === 'chats'}
    <div class="chats-tab">
      <div class="chats-header">
        <span class="chats-for-char">
          {#if activeGroup}
            Group: <strong>{activeGroup.name}</strong>
          {:else}
            Chats with <strong>{activeChar?.card.data.name || 'Character'}</strong>
          {/if}
        </span>
        <div class="chats-header-actions">
          {#if onImportChat}
            <button class="import-chat-btn" on:click={triggerChatFileInput} title="Import SillyTavern JSONL chat log">
              📥 Import
            </button>
          {/if}
          {#if activeGroup}
            <button class="new-chat-btn" on:click={() => handleNewGrpChat(activeGroup.id)}>
              + New
            </button>
          {:else if activeCharacterId}
            <button class="new-chat-btn" on:click={() => handleNewChat(activeCharacterId)}>
              + New
            </button>
          {/if}
        </div>
      </div>

      <div class="chat-list">
        {#if chats.length === 0}
          <div class="empty-state">
            <p>No chat sessions yet.</p>
            {#if activeGroup}
              <button class="link-btn" on:click={() => handleNewGrpChat(activeGroup.id)}>
                Start a new group conversation
              </button>
            {:else if activeCharacterId}
              <button class="link-btn" on:click={() => handleNewChat(activeCharacterId)}>
                Start a new conversation
              </button>
            {/if}
          </div>
        {:else}
          {#each chats as chat}
            <div
              class="chat-item {chat.id === activeChatId ? 'active' : ''}"
              role="button"
              tabindex="0"
              on:click={() => handleSelectChat(chat.id)}
              on:keydown={(e) => e.key === 'Enter' && handleSelectChat(chat.id)}
            >
              <div class="chat-item-main">
                <div class="chat-title-row">
                  <span class="chat-item-title">{chat.title}</span>
                  {#if chat.group_id}
                    <span class="chat-group-badge">👥 Group</span>
                  {/if}
                </div>
                <span class="chat-item-preview">{chat.last_message_preview}</span>
                <span class="chat-item-meta">{chat.message_count} messages</span>
              </div>
              <div class="chat-item-actions">
                {#if onExportChat}
                  <button
                    class="chat-action-btn chat-export-btn"
                    on:click|stopPropagation={() => onExportChat && onExportChat(chat.id, chat.title)}
                    title="Export as SillyTavern JSONL"
                  >
                    📤
                  </button>
                {/if}
                <button
                  class="chat-action-btn chat-delete-btn"
                  on:click|stopPropagation={() => {
                    if (confirm(`Delete chat "${chat.title}"?`)) onDeleteChat(chat.id);
                  }}
                  title="Delete chat"
                >
                  🗑️
                </button>
              </div>
            </div>
          {/each}
        {/if}
      </div>
    </div>
  {/if}

  <div class="sidebar-footer">
    {#if onOpenLorebooks}
      <button
        class="lorebook-sidebar-btn"
        on:click={() => {
          if (onOpenLorebooks) onOpenLorebooks();
          if (window.innerWidth <= 768) onClose();
        }}
      >
        <span class="footer-btn-icon">📖</span>
        <span class="footer-btn-text">Lorebooks</span>
      </button>
    {/if}
    {#if onOpenSync}
      <button
        class="lorebook-sidebar-btn sync-footer-btn"
        on:click={() => {
          if (onOpenSync) onOpenSync();
          if (window.innerWidth <= 768) onClose();
        }}
      >
        <span class="footer-btn-icon">🔄</span>
        <span class="footer-btn-text">Device Sync</span>
      </button>
    {/if}
  </div>
</aside>

<style>
  .sidebar-backdrop {
    display: none;
  }

  .sidebar {
    width: 320px;
    height: 100vh;
    height: 100dvh;
    background: #181825;
    border-right: 1px solid #313244;
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    transition: margin-left 0.2s ease;
    z-index: 20;
  }

  .sidebar.closed {
    margin-left: -320px;
  }

  .sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1rem 1.2rem;
    border-bottom: 1px solid #313244;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .logo-icon {
    font-size: 1.4rem;
  }

  .brand-title {
    font-size: 1.2rem;
    font-weight: 800;
    letter-spacing: 0.5px;
    background: linear-gradient(135deg, #cba6f7, #89b4fa);
    -webkit-background-clip: text;
    background-clip: text;
    -webkit-text-fill-color: transparent;
  }

  .icon-btn {
    background: transparent;
    border: none;
    color: #a6adc8;
    cursor: pointer;
    font-size: 1rem;
    padding: 0.4rem 0.6rem;
    border-radius: 6px;
  }

  .icon-btn:hover {
    background: #313244;
    color: #cdd6f4;
  }

  .tab-row {
    display: flex;
    border-bottom: 1px solid #313244;
    background: #11111b;
  }

  .tab-btn {
    flex: 1;
    background: transparent;
    border: none;
    color: #a6adc8;
    padding: 0.75rem 0.5rem;
    font-weight: 600;
    font-size: 0.85rem;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    transition: all 0.15s ease;
  }

  .tab-btn.active {
    color: #cba6f7;
    border-bottom-color: #cba6f7;
    background: #181825;
  }

  .tab-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .characters-tab,
  .chats-tab {
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow: hidden;
  }

  .search-and-actions {
    padding: 0.8rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    border-bottom: 1px solid #313244;
  }

  .search-input {
    background: #1e1e2e;
    border: 1px solid #45475a;
    border-radius: 8px;
    padding: 0.55rem 0.8rem;
    color: #cdd6f4;
    font-size: 0.88rem;
    outline: none;
  }

  .search-input:focus {
    border-color: #cba6f7;
  }

  .actions-row {
    display: flex;
    gap: 0.5rem;
  }

  .primary-action-btn {
    flex: 1;
    background: #cba6f7;
    color: #11111b;
    border: none;
    border-radius: 8px;
    padding: 0.5rem;
    font-weight: 700;
    font-size: 0.85rem;
    cursor: pointer;
  }

  .primary-action-btn:hover {
    background: #f5c2e7;
  }

  .secondary-action-btn {
    flex: 1;
    background: #313244;
    color: #cdd6f4;
    border: 1px solid #45475a;
    border-radius: 8px;
    padding: 0.5rem;
    font-weight: 600;
    font-size: 0.85rem;
    cursor: pointer;
  }

  .secondary-action-btn:hover {
    background: #45475a;
  }

  .character-list,
  .chat-list {
    flex: 1;
    overflow-y: auto;
    padding: 0.6rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .character-card {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem;
    background: #1e1e2e;
    border: 1px solid #313244;
    border-radius: 12px;
    cursor: pointer;
    transition: all 0.15s ease;
    position: relative;
  }

  .character-card:hover {
    background: #25263a;
    border-color: #45475a;
  }

  .character-card.active {
    background: rgba(203, 166, 247, 0.1);
    border-color: #cba6f7;
  }

  .char-avatar-img {
    width: 46px;
    height: 46px;
    border-radius: 50%;
    object-fit: cover;
    flex-shrink: 0;
  }

  .char-avatar-placeholder {
    width: 46px;
    height: 46px;
    border-radius: 50%;
    background: linear-gradient(135deg, #cba6f7, #89b4fa);
    color: #11111b;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.95rem;
    flex-shrink: 0;
  }

  .char-info {
    flex: 1;
    min-width: 0;
  }

  .char-name-row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .char-name {
    font-weight: 700;
    font-size: 0.92rem;
    color: #cdd6f4;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .active-pill {
    font-size: 0.65rem;
    background: #cba6f7;
    color: #11111b;
    font-weight: 700;
    padding: 0.1rem 0.35rem;
    border-radius: 4px;
  }

  .char-desc {
    font-size: 0.78rem;
    color: #a6adc8;
    margin: 0.2rem 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .char-tags {
    display: flex;
    gap: 0.3rem;
    margin-top: 0.2rem;
  }

  .tag-pill {
    font-size: 0.68rem;
    background: #11111b;
    color: #89b4fa;
    padding: 0.1rem 0.35rem;
    border-radius: 4px;
  }

  .tag-pill.mode-tag {
    color: #f9e2af;
  }

  .group-card-side-actions {
    display: flex;
    align-items: center;
    gap: 0.2rem;
  }

  .char-edit-btn {
    background: transparent;
    border: none;
    color: #a6adc8;
    cursor: pointer;
    font-size: 0.85rem;
    padding: 0.3rem 0.4rem;
    border-radius: 4px;
    opacity: 0.7;
    transition: opacity 0.15s ease, background 0.15s ease, color 0.15s ease;
  }

  .char-edit-btn:hover {
    opacity: 1;
    background: #313244;
    color: #cba6f7;
  }

  .chat-title-row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .chat-group-badge {
    font-size: 0.65rem;
    font-weight: 700;
    background: rgba(203, 166, 247, 0.15);
    color: #cba6f7;
    padding: 0.1rem 0.3rem;
    border-radius: 4px;
    white-space: nowrap;
  }

  .char-delete-btn,
  .chat-action-btn {
    background: transparent;
    border: none;
    color: #6c7086;
    cursor: pointer;
    font-size: 0.85rem;
    padding: 0.3rem 0.4rem;
    border-radius: 4px;
    opacity: 0.7;
    transition: opacity 0.15s ease, background 0.15s ease, color 0.15s ease;
  }

  .character-card:hover .char-delete-btn,
  .chat-item:hover .chat-action-btn {
    opacity: 1;
  }

  .chat-export-btn:hover {
    color: #89b4fa;
    background: rgba(137, 180, 250, 0.15);
  }

  .char-delete-btn:hover,
  .chat-delete-btn:hover {
    color: #f38ba8;
    background: rgba(243, 139, 168, 0.15);
  }

  .chats-header-actions {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .import-chat-btn {
    background: #313244;
    color: #cdd6f4;
    border: 1px solid #45475a;
    border-radius: 6px;
    padding: 0.4rem 0.65rem;
    font-weight: 600;
    font-size: 0.8rem;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .import-chat-btn:hover {
    background: #45475a;
  }

  .chat-item-actions {
    display: flex;
    align-items: center;
    gap: 0.2rem;
    margin-left: 0.5rem;
  }

  .chats-header {
    padding: 0.8rem 1rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
    border-bottom: 1px solid #313244;
  }

  .chats-for-char {
    font-size: 0.82rem;
    color: #a6adc8;
  }

  .new-chat-btn {
    background: #a6e3a1;
    color: #11111b;
    border: none;
    border-radius: 6px;
    padding: 0.4rem 0.8rem;
    font-weight: 700;
    font-size: 0.8rem;
    cursor: pointer;
  }

  .chat-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem;
    background: #1e1e2e;
    border: 1px solid #313244;
    border-radius: 10px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .chat-item:hover {
    background: #25263a;
    border-color: #45475a;
  }

  .chat-item.active {
    background: rgba(137, 180, 250, 0.1);
    border-color: #89b4fa;
  }

  .chat-item-main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .chat-item-title {
    font-weight: 600;
    font-size: 0.9rem;
    color: #cdd6f4;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .chat-item-preview {
    font-size: 0.78rem;
    color: #a6adc8;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .chat-item-meta {
    font-size: 0.7rem;
    color: #6c7086;
  }

  .empty-state {
    padding: 2rem 1rem;
    text-align: center;
    color: #6c7086;
    font-size: 0.88rem;
  }

  .link-btn {
    background: none;
    border: none;
    color: #cba6f7;
    font-size: 0.85rem;
    cursor: pointer;
    text-decoration: underline;
    margin-top: 0.5rem;
  }

  /* Mobile Responsive Drawer */
  @media (max-width: 768px) {
    .sidebar-backdrop {
      display: block;
      position: fixed;
      inset: 0;
      background: rgba(0, 0, 0, 0.65);
      backdrop-filter: blur(3px);
      z-index: 45;
      opacity: 0;
      pointer-events: none;
      transition: opacity 0.25s ease;
    }

    .sidebar-backdrop.visible {
      opacity: 1;
      pointer-events: auto;
    }

    .sidebar {
      position: fixed;
      left: 0;
      top: 0;
      bottom: 0;
      width: 82vw;
      max-width: 320px;
      z-index: 50;
      transform: translateX(0);
      transition: transform 0.25s cubic-bezier(0.4, 0, 0.2, 1);
      box-shadow: 4px 0 24px rgba(0, 0, 0, 0.6);
      padding-top: env(safe-area-inset-top);
      padding-bottom: env(safe-area-inset-bottom);
    }

    .sidebar.closed {
      transform: translateX(-100%);
      margin-left: 0;
      pointer-events: none;
    }
  }

  .sidebar-footer {
    padding: 0.8rem 1rem;
    border-top: 1px solid #313244;
    background: #11111b;
    display: flex;
    gap: 0.5rem;
  }
  .lorebook-sidebar-btn {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    background: #1e1e2e;
    border: 1px solid #313244;
    border-radius: 8px;
    color: #cdd6f4;
    padding: 0.55rem 0.8rem;
    font-size: 0.82rem;
    font-weight: 700;
    cursor: pointer;
    transition: all 0.15s ease;
  }
  .lorebook-sidebar-btn:hover {
    background: #313244;
    border-color: #89b4fa;
    color: #89b4fa;
  }
  .footer-btn-icon {
    font-size: 1rem;
  }
</style>
