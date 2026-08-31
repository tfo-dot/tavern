<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';

  import type {
    Character,
    ChatSummary,
    MessageViewNode,
    AppSettings,
    UserPersona,
  } from '$lib/types';

  import {
    getAllCharacters,
    saveCharacter,
    deleteCharacter,
    importCharacterCard,
    createChat,
    loadChat,
    listChats,
    deleteChat,
    getActiveMessages,
    appendMessage,
    editMessage,
    deleteMessage,
    switchBranch,
    getSettings,
    saveSettings,
    getActiveUserPersona,
    saveUserPersona,
    generateReply,
    abortGeneration,
  } from '$lib/api';

  import Sidebar from '$lib/components/Sidebar.svelte';
  import ChatMessage from '$lib/components/ChatMessage.svelte';
  import ChatInput from '$lib/components/ChatInput.svelte';
  import SettingsModal from '$lib/components/SettingsModal.svelte';
  import PersonaModal from '$lib/components/PersonaModal.svelte';
  import CharacterEditorModal from '$lib/components/CharacterEditorModal.svelte';
  import CreatorNotesModal from '$lib/components/CreatorNotesModal.svelte';
  import ImagePreviewModal from '$lib/components/ImagePreviewModal.svelte';
  import LorebookModal from '$lib/components/LorebookModal.svelte';
  let characters: Character[] = [];
  let activeCharacter: Character | null = null;
  let chats: ChatSummary[] = [];
  let activeChatId: string | null = null;
  let messages: MessageViewNode[] = [];
  let settings: AppSettings;
  let userPersona: UserPersona;

  let isSidebarOpen = true;
  let isGenerating = false;
  let streamingText = '';

  // Modals
  let isSettingsOpen = false;
  let isPersonaOpen = false;
  let isCharEditorOpen = false;
  let isCreatorNotesOpen = false;
  let isImagePreviewOpen = false;
  let isLorebookOpen = false;
  let previewImageSrc: string | null = null;
  let previewImageAlt: string = '';
  let editingCharacter: Character | null = null;
  let chatContainerEl: HTMLElement;
  let unlistenToken: UnlistenFn | null = null;
  let unlistenDone: UnlistenFn | null = null;

  onMount(async () => {
    // 0. Auto-close sidebar on mobile devices on startup
    if (typeof window !== 'undefined' && window.innerWidth <= 768) {
      isSidebarOpen = false;
    }

    // 1. Load initial settings and persona
    settings = await getSettings();
    userPersona = await getActiveUserPersona();

    // 2. Load characters
    await refreshCharacters();

    // 3. Select active or initial character & chat
    if (settings.active_character_id) {
      const found = characters.find((c) => c.id === settings.active_character_id);
      if (found) {
        activeCharacter = found;
      }
    }
    if (!activeCharacter && characters.length > 0) {
      activeCharacter = characters[0];
    }

    if (activeCharacter) {
      await refreshChats(activeCharacter.id);
      if (settings.active_chat_id && chats.some((c) => c.id === settings.active_chat_id)) {
        await handleSelectChat(settings.active_chat_id);
      } else if (chats.length > 0) {
        await handleSelectChat(chats[0].id);
      } else {
        await handleNewChat(activeCharacter.id);
      }
    }

    // 4. Setup LLM streaming listeners
    unlistenToken = await listen<string>('llm-token', async (event) => {
      streamingText += event.payload;
      await scrollToBottom();
    });

    unlistenDone = await listen('llm-done', async () => {
      isGenerating = false;
      streamingText = '';
      await refreshMessages();
      if (activeCharacter) {
        await refreshChats(activeCharacter.id);
      }
    });
  });

  onDestroy(() => {
    if (unlistenToken) unlistenToken();
    if (unlistenDone) unlistenDone();
  });

  async function scrollToBottom() {
    await tick();
    if (chatContainerEl) {
      chatContainerEl.scrollTop = chatContainerEl.scrollHeight;
    }
  }

  async function refreshCharacters() {
    characters = await getAllCharacters();
    if (activeCharacter) {
      const updated = characters.find((c) => c.id === activeCharacter?.id);
      activeCharacter = updated || (characters.length > 0 ? characters[0] : null);
    }
  }

  async function refreshChats(characterId: string) {
    chats = await listChats(characterId);
  }

  async function refreshMessages() {
    messages = await getActiveMessages();
    await scrollToBottom();
  }

  // --- Character Handlers ---
  async function handleSelectCharacter(id: string) {
    const found = characters.find((c) => c.id === id);
    if (!found) return;
    activeCharacter = found;

    await refreshChats(id);
    if (chats.length > 0) {
      await handleSelectChat(chats[0].id);
    } else {
      await handleNewChat(id);
    }
  }

  function handleOpenNewCharacter() {
    editingCharacter = null;
    isCharEditorOpen = true;
  }

  function handleEditActiveCharacter() {
    if (activeCharacter) {
      editingCharacter = activeCharacter;
      isCharEditorOpen = true;
    }
  }

  async function handleSaveCharacter(charToSave: Character) {
    const saved = await saveCharacter(charToSave);
    await refreshCharacters();
    activeCharacter = saved;
    await refreshChats(saved.id);
    if (chats.length === 0) {
      await handleNewChat(saved.id);
    }
  }

  async function handleDeleteCharacter(id: string) {
    await deleteCharacter(id);
    await refreshCharacters();
    if (activeCharacter?.id === id) {
      activeCharacter = characters.length > 0 ? characters[0] : null;
      if (activeCharacter) {
        await refreshChats(activeCharacter.id);
        if (chats.length > 0) await handleSelectChat(chats[0].id);
        else await handleNewChat(activeCharacter.id);
      } else {
        messages = [];
        chats = [];
        activeChatId = null;
      }
    }
  }

  async function handleImportCard(file: File) {
    try {
      const buffer = await file.arrayBuffer();
      const bytes = new Uint8Array(buffer);
      const imported = await importCharacterCard(bytes);
      await refreshCharacters();
      await handleSelectCharacter(imported.id);
    } catch (e) {
      alert(`Import failed: ${e}`);
    }
  }

  // --- Chat Handlers ---
  async function handleSelectChat(chatId: string) {
    activeChatId = chatId;
    await loadChat(chatId);
    await refreshMessages();
  }

  async function handleNewChat(characterId: string) {
    const tree = await createChat(characterId);
    activeChatId = tree.id;
    await refreshChats(characterId);
    await refreshMessages();
  }

  async function handleDeleteChat(chatId: string) {
    await deleteChat(chatId);
    if (activeCharacter) {
      await refreshChats(activeCharacter.id);
      if (activeChatId === chatId) {
        if (chats.length > 0) {
          await handleSelectChat(chats[0].id);
        } else {
          await handleNewChat(activeCharacter.id);
        }
      }
    }
  }

  // --- Message Actions ---
  async function handleSendMessage(userText: string) {
    if (!userText.trim() || isGenerating || !activeCharacter) return;

    const parentId = messages.length > 0 ? messages[messages.length - 1].id : null;
    await appendMessage('User', userText, parentId);
    await refreshMessages();

    // Trigger LLM generation
    isGenerating = true;
    streamingText = '';
    try {
      await generateReply(false);
    } catch (e) {
      console.error('Generation failed:', e);
      isGenerating = false;
      alert(`Generation failed: ${e}`);
    }
  }

  async function handleRegenerateSwipe() {
    if (isGenerating || !activeCharacter || messages.length === 0) return;

    isGenerating = true;
    streamingText = '';
    try {
      await generateReply(true);
    } catch (e) {
      console.error('Swipe generation failed:', e);
      isGenerating = false;
      alert(`Swipe failed: ${e}`);
    }
  }

  async function handleSwipe(parentId: string | null, newIndex: number) {
    if (isGenerating) return;
    messages = await switchBranch(parentId, newIndex);
    await scrollToBottom();
  }

  async function handleEditMessage(id: string, newContent: string) {
    messages = await editMessage(id, newContent);
  }

  async function handleDeleteMessage(id: string) {
    messages = await deleteMessage(id);
    if (activeCharacter) {
      await refreshChats(activeCharacter.id);
    }
  }

  async function handleStopGeneration() {
    await abortGeneration();
    isGenerating = false;
  }

  // --- Settings & Persona ---
  async function handleSaveSettings(updated: AppSettings) {
    settings = updated;
    await saveSettings(updated);
  }

  async function handleSavePersona(updated: UserPersona) {
    userPersona = updated;
    await saveUserPersona(updated);
  }
  function handleOpenImagePreview(src: string, alt: string) {
    previewImageSrc = src;
    previewImageAlt = alt;
    isImagePreviewOpen = true;
  }
</script>

<div class="app-layout">
  <!-- Sidebar -->
  <Sidebar
    {characters}
    activeCharacterId={activeCharacter?.id || null}
    {chats}
    {activeChatId}
    isOpen={isSidebarOpen}
    onSelectCharacter={handleSelectCharacter}
    onCreateCharacter={handleOpenNewCharacter}
    onImportCard={handleImportCard}
    onSelectChat={handleSelectChat}
    onNewChat={handleNewChat}
    onDeleteChat={handleDeleteChat}
    onDeleteCharacter={handleDeleteCharacter}
    onOpenLorebooks={() => (isLorebookOpen = true)}
    onClose={() => (isSidebarOpen = false)}
  />

  <!-- Main View -->
  <div class="main-content">
    <!-- Top Header Bar -->
    <header class="top-nav">
      <div class="nav-left">
        <button
          class="icon-nav-btn hamburger-btn"
          on:click={() => (isSidebarOpen = !isSidebarOpen)}
          title="Toggle sidebar"
        >
          ☰
        </button>

        {#if activeCharacter}
          <div
            class="active-char-badge"
            on:click={handleEditActiveCharacter}
            on:keydown={(e) => e.key === 'Enter' && handleEditActiveCharacter()}
            role="button"
            tabindex="0"
          >
            {#if activeCharacter.avatar_data_url}
              <img
                src={activeCharacter.avatar_data_url}
                alt={activeCharacter.card.data.name}
                class="nav-avatar"
              />
            {:else}
              <div class="nav-avatar-placeholder">
                {activeCharacter.card.data.name.slice(0, 2).toUpperCase()}
              </div>
            {/if}
            <div class="nav-char-details">
              <span class="nav-char-name">{activeCharacter.card.data.name}</span>
              <span class="nav-char-subtitle">Edit card</span>
            </div>
          </div>
        {:else}
          <span class="nav-char-name">No character</span>
        {/if}
      </div>

      <div class="nav-right">
        {#if settings?.active_model}
          <div class="model-badge" title="Active Model">
            ⚡ {settings.active_model}
          </div>
        {/if}
        {#if activeCharacter?.card?.data?.creator_notes}
          <button
            class="icon-nav-btn action-btn-compact notes-nav-btn"
            on:click={() => (isCreatorNotesOpen = true)}
            title="Creator Notes"
          >
            <span class="btn-icon">📜</span>
            <span class="btn-text">Notes</span>
          </button>
        {/if}
        <button
          class="icon-nav-btn action-btn-compact lorebook-nav-btn"
          on:click={() => (isLorebookOpen = true)}
          title="World Info & Lorebooks"
        >
          <span class="btn-icon">📖</span>
          <span class="btn-text">Lorebooks</span>
        </button>

        <button
          class="icon-nav-btn action-btn-compact"
          on:click={() => (isPersonaOpen = true)}
          title="User Persona ({userPersona?.name || 'User'})"
        >
          <span class="btn-icon">👤</span>
          <span class="btn-text">Persona</span>
        </button>

        <button
          class="icon-nav-btn action-btn-compact"
          on:click={() => (isSettingsOpen = true)}
          title="API & Generation Settings"
        >
          <span class="btn-icon">⚙️</span>
          <span class="btn-text">Settings</span>
        </button>
      </div>
    </header>

    <!-- Chat Messages Stream -->
    <main class="chat-viewport" bind:this={chatContainerEl}>
      {#if !activeCharacter}
        <div class="hero-empty">
          <div class="hero-icon">🏰</div>
          <h2>Welcome to Tavern</h2>
          <p>Import a character card or create a new character to begin chatting.</p>
          <button class="hero-btn" on:click={handleOpenNewCharacter}>
            + Create Character
          </button>
        </div>
      {:else if messages.length === 0}
        <div class="hero-empty">
          <div class="hero-icon">💬</div>
          <h2>Start a conversation with {activeCharacter.card.data.name}</h2>
          <p>{activeCharacter.card.data.scenario || 'Type a message below to begin.'}</p>
        </div>
      {:else}
        <div class="messages-container">
          {#each messages as msg, i (msg.id)}
            <ChatMessage
              message={msg}
              characterName={activeCharacter?.card.data.name || 'Character'}
              characterAvatar={activeCharacter?.avatar_data_url || null}
              userName={userPersona?.name || 'You'}
              userAvatar={userPersona?.avatar_data_url || null}
              {isGenerating}
              isLastMessage={i === messages.length - 1}
              onSwipe={handleSwipe}
              onRegenerateSwipe={handleRegenerateSwipe}
              onEdit={handleEditMessage}
              onDelete={handleDeleteMessage}
              onOpenImage={handleOpenImagePreview}
            />
          {/each}

          <!-- Live Streaming Token Display -->
          {#if isGenerating && streamingText}
            <ChatMessage
              message={{
                id: 'temp-streaming',
                parent_id: null,
                role: 'Assistant',
                content: streamingText,
                created_at: new Date().toISOString(),
                sibling_index: 0,
                sibling_total: 1,
                can_swipe_left: false,
                can_swipe_right: false,
              }}
              characterName={activeCharacter?.card.data.name || 'Character'}
              characterAvatar={activeCharacter?.avatar_data_url || null}
              userName={userPersona?.name || 'You'}
              userAvatar={userPersona?.avatar_data_url || null}
              isGenerating={true}
              isLastMessage={true}
              onSwipe={() => {}}
              onRegenerateSwipe={() => {}}
              onEdit={() => {}}
              onDelete={() => {}}
              onOpenImage={handleOpenImagePreview}
            />
          {/if}

          <!-- Typing / Thinking Dots Indicator -->
          {#if isGenerating && !streamingText}
            <div class="typing-indicator">
              <div class="dot"></div>
              <div class="dot"></div>
              <div class="dot"></div>
              <span>{activeCharacter?.card.data.name || 'Character'} is thinking...</span>
            </div>
          {/if}
        </div>
      {/if}
    </main>

    <!-- Bottom Input Bar -->
    {#if activeCharacter}
      <ChatInput
        {isGenerating}
        placeholder={`Message ${activeCharacter.card.data.name}...`}
        hasMessages={messages.length > 0}
        onSend={handleSendMessage}
        onStop={handleStopGeneration}
        onRegenerate={handleRegenerateSwipe}
      />
    {/if}
  </div>
</div>

<!-- Modals -->
<SettingsModal
  {settings}
  isOpen={isSettingsOpen}
  onSave={handleSaveSettings}
  onClose={() => (isSettingsOpen = false)}
/>

<PersonaModal
  activePersona={userPersona}
  isOpen={isPersonaOpen}
  onPersonaChanged={(updated) => {
    userPersona = updated;
  }}
  onClose={() => (isPersonaOpen = false)}
/>

<CharacterEditorModal
  character={editingCharacter}
  isOpen={isCharEditorOpen}
  onSave={handleSaveCharacter}
  onClose={() => (isCharEditorOpen = false)}
/>

<CreatorNotesModal
  characterName={activeCharacter?.card.data.name || 'Character'}
  creatorNotes={activeCharacter?.card.data.creator_notes || ''}
  characterAvatar={activeCharacter?.avatar_data_url || null}
  isOpen={isCreatorNotesOpen}
  onOpenImagePreview={handleOpenImagePreview}
  onClose={() => (isCreatorNotesOpen = false)}
/>

<ImagePreviewModal
  src={previewImageSrc}
  alt={previewImageAlt}
  isOpen={isImagePreviewOpen}
  onClose={() => (isImagePreviewOpen = false)}
/>

<LorebookModal
  isOpen={isLorebookOpen}
  {settings}
  onSettingsUpdated={(updated) => {
    settings = updated;
  }}
  onClose={() => (isLorebookOpen = false)}
/>

<style>
  :global(body) {
    margin: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
    background: #11111b;
    color: #cdd6f4;
    overflow: hidden;
    -webkit-tap-highlight-color: transparent;
  }

  :global(*),
  :global(*::before),
  :global(*::after) {
    box-sizing: border-box;
  }

  .app-layout {
    display: flex;
    height: 100vh;
    height: 100dvh;
    width: 100vw;
    overflow: hidden;
    background: #11111b;
  }

  .main-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    height: 100vh;
    height: 100dvh;
    min-width: 0;
    position: relative;
  }

  .top-nav {
    min-height: 56px;
    background: #181825;
    border-bottom: 1px solid #313244;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 1rem;
    padding-top: env(safe-area-inset-top);
    flex-shrink: 0;
    z-index: 10;
  }

  .nav-left {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    min-width: 0;
  }

  .active-char-badge {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.3rem 0.5rem;
    border-radius: 8px;
    cursor: pointer;
    transition: background 0.15s ease;
    min-width: 0;
  }

  .active-char-badge:hover {
    background: #313244;
  }

  .nav-avatar {
    width: 34px;
    height: 34px;
    border-radius: 50%;
    object-fit: cover;
    flex-shrink: 0;
  }

  .nav-avatar-placeholder {
    width: 34px;
    height: 34px;
    border-radius: 50%;
    background: linear-gradient(135deg, #cba6f7, #89b4fa);
    color: #11111b;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.82rem;
    flex-shrink: 0;
  }

  .nav-char-details {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .nav-char-name {
    font-weight: 700;
    font-size: 0.95rem;
    color: #cdd6f4;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 200px;
  }

  .nav-char-subtitle {
    font-size: 0.7rem;
    color: #a6adc8;
  }

  .nav-right {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-shrink: 0;
  }

  .model-badge {
    background: #1e1e2e;
    border: 1px solid #45475a;
    color: #a6e3a1;
    font-size: 0.78rem;
    font-weight: 600;
    padding: 0.3rem 0.65rem;
    border-radius: 6px;
    max-width: 160px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .icon-nav-btn {
    background: #313244;
    border: 1px solid #45475a;
    color: #cdd6f4;
    border-radius: 8px;
    padding: 0.45rem 0.75rem;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 0.3rem;
    transition: background 0.15s ease;
  }

  .icon-nav-btn:hover {
    background: #45475a;
  }

  .chat-viewport {
    flex: 1;
    overflow-y: auto;
    padding: 1rem;
    display: flex;
    flex-direction: column;
  }

  .messages-container {
    max-width: 860px;
    width: 100%;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
  }

  .typing-indicator {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.6rem 1rem;
    background: #181825;
    border-radius: 12px;
    border-left: 3px solid #cba6f7;
    width: fit-content;
    font-size: 0.85rem;
    color: #a6adc8;
    margin-top: 0.5rem;
  }

  .dot {
    width: 6px;
    height: 6px;
    background: #cba6f7;
    border-radius: 50%;
    animation: bounce 1.4s infinite ease-in-out both;
  }

  .dot:nth-child(1) { animation-delay: -0.32s; }
  .dot:nth-child(2) { animation-delay: -0.16s; }

  @keyframes bounce {
    0%, 80%, 100% { transform: scale(0); }
    40% { transform: scale(1); }
  }

  .hero-empty {
    margin: auto;
    text-align: center;
    max-width: 460px;
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.8rem;
  }

  .hero-icon {
    font-size: 3.2rem;
  }

  .hero-empty h2 {
    margin: 0;
    font-size: 1.4rem;
    color: #cdd6f4;
  }

  .hero-empty p {
    margin: 0;
    color: #a6adc8;
    line-height: 1.5;
  }

  .hero-btn {
    margin-top: 0.5rem;
    background: #cba6f7;
    color: #11111b;
    border: none;
    padding: 0.6rem 1.4rem;
    border-radius: 8px;
    font-weight: 700;
    font-size: 0.95rem;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .hero-btn:hover {
    background: #f5c2e7;
  }

  /* Mobile screen optimization */
  @media (max-width: 640px) {
    .top-nav {
      padding: 0 0.5rem;
      padding-top: env(safe-area-inset-top);
      gap: 0.3rem;
    }

    .model-badge {
      display: none;
    }

    .nav-char-subtitle {
      display: none;
    }

    .nav-char-name {
      max-width: 120px;
      font-size: 0.88rem;
    }

    .action-btn-compact .btn-text {
      display: none;
    }

    .icon-nav-btn {
      padding: 0.4rem 0.55rem;
    }

    .chat-viewport {
      padding: 0.6rem 0.4rem;
    }

    .messages-container {
      gap: 0.75rem;
    }
  }
</style>
