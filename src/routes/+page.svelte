<script lang="ts">
  import { tick, untrack } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";

  import type {
    Character,
    Group,
    TurnMode,
    ChatSummary,
    MessageViewNode,
    AppSettings,
    UserPersona,
    ContextBreakdown,
    ChatTree,
  } from "$lib/types";
  import {
    getAllCharacters,
    saveCharacter,
    deleteCharacter,
    importCharacterCard,
    getAllGroups,
    saveGroup,
    deleteGroup,
    createGroupChat,
    listGroupChats,
    createChat,
    loadChat,
    listChats,
    deleteChat,
    importChatJsonl,
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
    calculateContextBreakdown,
    updateChatAuthorsNote,
    forkChatAtMessage,
  } from "$lib/api";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import ChatMessage from "$lib/components/ChatMessage.svelte";
  import ChatInput from "$lib/components/ChatInput.svelte";
  import SettingsModal from "$lib/components/SettingsModal.svelte";
  import PersonaModal from "$lib/components/PersonaModal.svelte";
  import CharacterEditorModal from "$lib/components/CharacterEditorModal.svelte";
  import CreatorNotesModal from "$lib/components/CreatorNotesModal.svelte";
  import ImagePreviewModal from "$lib/components/ImagePreviewModal.svelte";
  import LorebookModal from "$lib/components/LorebookModal.svelte";
  import SyncModal from "$lib/components/SyncModal.svelte";
  import GroupModal from "$lib/components/GroupModal.svelte";
  import GroupTurnBar from "$lib/components/GroupTurnBar.svelte";
  import ContextBar from "$lib/components/ContextBar.svelte";
  import AuthorsNoteModal from "$lib/components/AuthorsNoteModal.svelte";

  let characters: Character[] = $state([]);
  let activeCharacter: Character | null = $state(null);
  let groups: Group[] = $state([]);
  let activeGroup: Group | null = $state(null);
  let chats: ChatSummary[] = $state([]);
  let activeChatId: string | null = $state(null);
  let messages: MessageViewNode[] = $state([]);
  let settings: AppSettings = $state({
    endpoint: "",
    api_key: "",
    active_model: "",
    temperature: 0,
    top_p: 0,
    frequency_penalty: 0,
    presence_penalty: 0,
    max_tokens: 0,
    max_context_tokens: 0,
    system_template: "",
    stop_sequences: [],
    active_character_id: null,
    active_chat_id: null,
  });

  let userPersona: UserPersona | null = $state(null);

  let isSidebarOpen = $state(true);
  let isGenerating = $state(false);
  let streamingText = $state("");
  let continuingMessageId: string | null = $state(null);
  let streamingSpeakerName = $state("");
  let streamingSpeakerAvatar: string | null = $state(null);
  let currentStreamingCharacterId: string | null = $state(null);

  // Modals
  let isSettingsOpen = $state(false);
  let isPersonaOpen = $state(false);
  let isCharEditorOpen = $state(false);
  let isGroupModalOpen = $state(false);
  let isCreatorNotesOpen = $state(false);
  let isImagePreviewOpen = $state(false);
  let isLorebookOpen = $state(false);
  let isSyncOpen = $state(false);
  let previewImageSrc: string | null = $state(null);
  let previewImageAlt: string = $state("");
  let editingCharacter: Character | null = $state(null);
  let editingGroup: Group | null = $state(null);
  let chatContainerEl: HTMLElement;
  let inputDraftText = $state("");
  let contextBreakdown: ContextBreakdown | null = $state(null);
  let activeChatTree: ChatTree | null = $state(null);
  let isAuthorsNoteModalOpen = $state(false);
  let currentDraftText = "";
  let draftDebounceTimer: ReturnType<typeof setTimeout> | null = null;

  async function refreshContextBreakdown(draft?: string) {
    if (!activeCharacter && !activeGroup) {
      contextBreakdown = null;
      return;
    }

    try {
      contextBreakdown = await calculateContextBreakdown(
        draft !== undefined ? draft : currentDraftText,
      );
    } catch (e) {
      console.error("Failed to calculate context breakdown:", e);
    }
  }

  function handleDraftChange(newDraft: string) {
    currentDraftText = newDraft;
    if (draftDebounceTimer) clearTimeout(draftDebounceTimer);
    draftDebounceTimer = setTimeout(() => {
      refreshContextBreakdown(newDraft);
    }, 250);
  }

  async function handleSaveAuthorsNote(
    note: string,
    depth: number,
    interval: number,
  ) {
    await updateChatAuthorsNote(note, depth, interval);
    if (activeChatTree) {
      activeChatTree.authors_note = note;
      activeChatTree.authors_note_depth = depth;
      activeChatTree.authors_note_interval = interval;
    }
    await refreshContextBreakdown();
  }

  async function handleForkChat(messageId: string) {
    const titlePrompt = prompt(
      "Enter a title for the new forked chat (or leave blank):",
    );
    if (titlePrompt === null) return;
    try {
      const forked = await forkChatAtMessage(
        messageId,
        titlePrompt.trim() || undefined,
      );
      activeChatTree = forked;
      activeChatId = forked.id;
      if (activeGroup) {
        await refreshChatsForGroup(activeGroup.id);
      } else if (activeCharacter) {
        await refreshChats(activeCharacter.id);
      }
      await refreshMessages();
    } catch (e) {
      console.error("Fork chat failed:", e);
      alert(`Failed to fork chat: ${e}`);
    }
  }

  let nextSpeakerId = $derived(computeNextSpeaker(activeGroup, messages));

  function computeNextSpeaker(
    grp: Group | null,
    msgs: MessageViewNode[],
  ): string | null {
    if (!grp || grp.members.length === 0) return null;
    const enabled = grp.members.filter((m) => m.enabled && !m.mute);
    if (enabled.length === 0) return null;

    if (grp.turn_mode === "Manual") {
      return enabled[0].character_id;
    }

    if (grp.turn_mode === "Natural") {
      const lastSpeaker = [...msgs]
        .reverse()
        .find((m) => m.role === "Assistant")?.character_id;
      if (lastSpeaker) {
        const idx = enabled.findIndex((m) => m.character_id === lastSpeaker);
        if (idx >= 0) {
          return enabled[(idx + 1) % enabled.length].character_id;
        }
      }
      return enabled[0].character_id;
    }

    return enabled[0].character_id;
  }

  $effect(() => {
    untrack(() => {
      // 0. Auto-close sidebar on mobile devices on startup
      if (typeof window !== "undefined" && window.innerWidth <= 768) {
        isSidebarOpen = false;
      }
    });

    let active = true;
    const cleanups: UnlistenFn[] = [];

    // 2. Setup listeners IMMEDIATELY (do not wait for DB queries)
    const setupListeners = async () => {
      try {
        const u1 = await listen<{
          character_id: string;
          character_name: string;
        }>("llm-start", (event) => {
          if (!active) return;
          currentStreamingCharacterId = event.payload.character_id;
          streamingSpeakerName = event.payload.character_name;
          const found = characters.find(
            (c) => c.id === event.payload.character_id,
          );
          streamingSpeakerAvatar = found?.avatar_data_url || null;
        });
        cleanups.push(u1);

        const u2 = await listen<string>("llm-token", async (event) => {
          if (!active) return;
          streamingText += event.payload;
          await scrollToBottom();
        });
        cleanups.push(u2);

        const u3 = await listen("llm-done", async () => {
          if (!active) return;
          isGenerating = false;
          continuingMessageId = null;
          streamingText = "";
          currentStreamingCharacterId = null;
          streamingSpeakerName = "";
          streamingSpeakerAvatar = null;
          await refreshMessages();
          if (activeGroup) await refreshChatsForGroup(activeGroup.id);
          else if (activeCharacter) await refreshChats(activeCharacter.id);
        });
        cleanups.push(u3);
      } catch (err) {
        console.error(
          "Failed to register Tauri event listeners on Android:",
          err,
        );
      }
    };

    // 3. Load initial data in parallel
    const bootstrapData = async () => {
      try {
        const [loadedSettings, loadedPersona] = await Promise.all([
          getSettings(),
          getActiveUserPersona(),
        ]);
        if (!active) return;

        settings = loadedSettings;
        userPersona = loadedPersona;

        await Promise.all([refreshCharacters(), refreshGroups()]);
        if (!active) return;

        // Selection logic
        if (settings.active_character_id) {
          if (settings.active_character_id.startsWith("group:")) {
            const gid = settings.active_character_id.replace("group:", "");
            const foundGrp = groups.find((g) => g.id === gid);
            if (foundGrp) {
              activeGroup = foundGrp;
              activeCharacter = null;
              await refreshChatsForGroup(gid);
            }
          } else {
            const found = characters.find(
              (c) => c.id === settings.active_character_id,
            );
            if (found) {
              activeCharacter = found;
              activeGroup = null;
              await refreshChats(activeCharacter.id);
            }
          }
        }

        if (!activeCharacter && !activeGroup) {
          if (characters.length > 0) {
            activeCharacter = characters[0];
            await refreshChats(activeCharacter.id);
          } else if (groups.length > 0) {
            activeGroup = groups[0];
            await refreshChatsForGroup(activeGroup.id);
          }
        }

        const currentTargetId = activeGroup
          ? activeGroup.id
          : activeCharacter?.id;
        if (currentTargetId) {
          if (
            settings.active_chat_id &&
            chats.some((c) => c.id === settings.active_chat_id)
          ) {
            await handleSelectChat(settings.active_chat_id);
          } else if (chats.length > 0) {
            await handleSelectChat(chats[0].id);
          } else {
            if (activeGroup) await handleNewGroupChat(activeGroup.id);
            else if (activeCharacter) await handleNewChat(activeCharacter.id);
          }
        }
      } catch (err) {
        console.error("Bootstrap data loading error:", err);
      }
    };

    // Run both tasks
    setupListeners();
    bootstrapData();

    // Synchronous teardown
    return () => {
      active = false;
      cleanups.forEach((fn) => fn());
    };
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
      activeCharacter =
        updated || (characters.length > 0 ? characters[0] : null);
    }
  }

  async function refreshGroups() {
    groups = await getAllGroups();
    if (activeGroup) {
      const updated = groups.find((g) => g.id === activeGroup?.id);
      activeGroup = updated || (groups.length > 0 ? groups[0] : null);
    }
  }

  async function refreshChats(characterId: string) {
    chats = await listChats(characterId);
  }

  async function refreshChatsForGroup(groupId: string) {
    chats = await listGroupChats(groupId);
  }

  async function refreshMessages() {
    messages = await getActiveMessages();
    await scrollToBottom();
    await refreshContextBreakdown();
  }

  async function handleSyncSuccess() {
    await refreshCharacters();
    await refreshGroups();
    if (activeGroup) {
      await refreshChatsForGroup(activeGroup.id);
    } else if (activeCharacter) {
      await refreshChats(activeCharacter.id);
    }
    if (activeChatId) {
      await refreshMessages();
    }
    try {
      userPersona = await getActiveUserPersona();
      settings = await getSettings();
    } catch (e) {
      console.error("Error refreshing state after sync:", e);
    }
  }

  // --- Character Handlers ---
  async function handleSelectCharacter(id: string) {
    const found = characters.find((c) => c.id === id);
    if (!found) return;
    activeCharacter = found;
    activeGroup = null;

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

  // --- Group Handlers ---
  async function handleSelectGroup(id: string) {
    const found = groups.find((g) => g.id === id);
    if (!found) return;
    activeGroup = found;
    activeCharacter = null;

    await refreshChatsForGroup(id);
    if (chats.length > 0) {
      await handleSelectChat(chats[0].id);
    } else {
      await handleNewGroupChat(id);
    }
  }

  function handleCreateGroup() {
    editingGroup = null;
    isGroupModalOpen = true;
  }

  function handleEditGroup(grp: Group) {
    editingGroup = grp;
    isGroupModalOpen = true;
  }

  async function handleSaveGroup(grp: Group, startChat = false) {
    const saved = await saveGroup(grp);
    await refreshGroups();
    activeGroup = saved;
    activeCharacter = null;
    await refreshChatsForGroup(saved.id);
    if (startChat || chats.length === 0) {
      await handleNewGroupChat(saved.id);
    }
  }

  async function handleDeleteGroup(id: string) {
    await deleteGroup(id);
    await refreshGroups();
    if (activeGroup?.id === id) {
      activeGroup = groups.length > 0 ? groups[0] : null;
      if (activeGroup) {
        await refreshChatsForGroup(activeGroup.id);
        if (chats.length > 0) await handleSelectChat(chats[0].id);
        else await handleNewGroupChat(activeGroup.id);
      } else if (characters.length > 0) {
        await handleSelectCharacter(characters[0].id);
      } else {
        messages = [];
        chats = [];
        activeChatId = null;
      }
    }
  }

  async function handleNewGroupChat(groupId: string) {
    const tree = await createGroupChat(groupId);
    activeChatTree = tree;
    activeChatId = tree.id;
    await refreshChatsForGroup(groupId);
    await refreshMessages();
  }

  async function handleTriggerSpeaker(characterId: string) {
    if (isGenerating) return;
    isGenerating = true;
    continuingMessageId = null;
    streamingText = "";
    const charObj = characters.find((c) => c.id === characterId);
    streamingSpeakerName = charObj?.card.data.name || "Character";
    streamingSpeakerAvatar = charObj?.avatar_data_url || null;
    currentStreamingCharacterId = characterId;

    try {
      await generateReply(false, false, characterId);
    } catch (e) {
      console.error("Trigger speaker failed:", e);
      isGenerating = false;
      alert(`Generation failed: ${e}`);
    }
  }

  async function handleNextTurn() {
    if (isGenerating) return;
    if (activeGroup) {
      const targetId =
        nextSpeakerId ||
        (activeGroup.members.length > 0
          ? activeGroup.members[0].character_id
          : null);
      if (targetId) {
        await handleTriggerSpeaker(targetId);
      } else {
        await handleSendMessage("");
      }
    } else {
      await handleSendMessage("");
    }
  }

  async function handleToggleTurnMode() {
    if (!activeGroup) return;
    const modes: TurnMode[] = ["Natural", "Manual", "Random"];
    const curIdx = modes.indexOf(activeGroup.turn_mode);
    const nextMode = modes[(curIdx + 1) % modes.length];
    const updated: Group = {
      ...activeGroup,
      turn_mode: nextMode,
      updated_at: new Date().toISOString(),
    };
    activeGroup = updated;
    await saveGroup(updated);
    await refreshGroups();
  }

  async function handleToggleAutoMode() {
    if (!activeGroup) return;
    const updated: Group = {
      ...activeGroup,
      auto_mode: !activeGroup.auto_mode,
      updated_at: new Date().toISOString(),
    };
    activeGroup = updated;
    await saveGroup(updated);
    await refreshGroups();
  }

  async function handleToggleMute(characterId: string) {
    if (!activeGroup) return;
    const updatedMembers = activeGroup.members.map((m) =>
      m.character_id === characterId ? { ...m, mute: !m.mute } : m,
    );
    const updated: Group = {
      ...activeGroup,
      members: updatedMembers,
      updated_at: new Date().toISOString(),
    };
    activeGroup = updated;
    await saveGroup(updated);
    await refreshGroups();
  }

  // --- Chat Handlers ---
  async function handleSelectChat(chatId: string) {
    activeChatId = chatId;
    activeChatTree = await loadChat(chatId);
    await refreshMessages();
  }

  async function handleNewChat(characterId: string) {
    const tree = await createChat(characterId);
    activeChatTree = tree;
    activeChatId = tree.id;
    await refreshChats(characterId);
    await refreshMessages();
  }

  async function handleDeleteChat(chatId: string) {
    await deleteChat(chatId);
    if (activeGroup) {
      await refreshChatsForGroup(activeGroup.id);
      if (activeChatId === chatId) {
        if (chats.length > 0) {
          await handleSelectChat(chats[0].id);
        } else {
          await handleNewGroupChat(activeGroup.id);
        }
      }
    } else if (activeCharacter) {
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

  async function handleImportChat(file: File) {
    try {
      const buffer = await file.arrayBuffer();
      const bytes = new Uint8Array(buffer);
      const title = file.name.replace(/\.jsonl$/i, "").replace(/\.json$/i, "");
      const importedTree = await importChatJsonl(
        bytes,
        activeCharacter?.id,
        title,
      );

      // Update character or group if the imported chat belongs to a different entity
      if (importedTree.group_id) {
        const foundGrp = groups.find((g) => g.id === importedTree.group_id);
        if (foundGrp) {
          activeGroup = foundGrp;
          activeCharacter = null;
          await refreshChatsForGroup(foundGrp.id);
        }
      } else if (
        !activeCharacter ||
        activeCharacter.id !== importedTree.character_id
      ) {
        const found = characters.find(
          (c) => c.id === importedTree.character_id,
        );
        if (found) {
          activeCharacter = found;
          activeGroup = null;
          await refreshChats(activeCharacter.id);
        }
      }
      activeChatId = importedTree.id;
      await refreshMessages();
    } catch (e) {
      alert(`Import chat failed: ${e}`);
    }
  }

  // --- Message Actions ---
  async function handleSendMessage(userText: string) {
    if (isGenerating || (!activeCharacter && !activeGroup)) return;

    const trimmed = userText.trim();
    if (trimmed) {
      const parentId =
        messages.length > 0 ? messages[messages.length - 1].id : null;
      await appendMessage("User", trimmed, parentId);
      await refreshMessages();
    }

    // If in group chat and auto_mode is off, and user sent text, let user manually trigger next turn
    if (activeGroup && !activeGroup.auto_mode && trimmed) {
      return;
    }

    // Trigger LLM generation
    isGenerating = true;
    continuingMessageId = null;
    streamingText = "";
    try {
      await generateReply(false, false, null);
    } catch (e) {
      console.error("Generation failed:", e);
      isGenerating = false;
      alert(`Generation failed: ${e}`);
    }
  }

  async function handleContinueGeneration() {
    if (
      isGenerating ||
      (!activeCharacter && !activeGroup) ||
      messages.length === 0
    )
      return;
    const lastMsg = messages[messages.length - 1];
    if (lastMsg.role !== "Assistant") return;

    isGenerating = true;
    try {
      await generateReply(false, true);
    } catch (e) {
      console.error("Continue generation failed:", e);
      isGenerating = false;
      continuingMessageId = null;
      alert(`Continue failed: ${e}`);
    }
  }

  async function handleRegenerateSwipe() {
    if (
      isGenerating ||
      (!activeCharacter && !activeGroup) ||
      messages.length === 0
    )
      return;
    continuingMessageId = null;
    streamingText = "";
    try {
      await generateReply(true, false);
    } catch (e) {
      console.error("Swipe generation failed:", e);
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
    continuingMessageId = null;
  }

  // --- Settings & Persona ---
  async function handleSaveSettings(updated: AppSettings) {
    await saveSettings(updated);

    settings = updated;

    try {
      await refreshContextBreakdown();
    } catch (e) {
      console.error("Error refreshing state after sync:", e);
    }
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
    {groups}
    activeGroupId={activeGroup?.id || null}
    {chats}
    {activeChatId}
    isOpen={isSidebarOpen}
    onSelectCharacter={handleSelectCharacter}
    onCreateCharacter={handleOpenNewCharacter}
    onImportCard={handleImportCard}
    onSelectGroup={handleSelectGroup}
    onCreateGroup={handleCreateGroup}
    onEditGroup={handleEditGroup}
    onDeleteGroup={handleDeleteGroup}
    onNewGroupChat={handleNewGroupChat}
    onSelectChat={handleSelectChat}
    onNewChat={handleNewChat}
    onDeleteChat={handleDeleteChat}
    onDeleteCharacter={handleDeleteCharacter}
    onImportChat={handleImportChat}
    onOpenLorebooks={() => (isLorebookOpen = true)}
    onOpenSync={() => (isSyncOpen = true)}
    onClose={() => (isSidebarOpen = false)}
  />

  <!-- Main View -->
  <div class="main-content">
    <!-- Top Header Bar -->
    <header class="top-nav">
      <div class="nav-left">
        <button
          class="icon-nav-btn hamburger-btn"
          onclick={() => (isSidebarOpen = !isSidebarOpen)}
          title="Toggle sidebar"
        >
          ☰
        </button>

        {#if activeGroup}
          <div
            class="active-char-badge"
            onclick={() => {
              editingGroup = activeGroup;
              isGroupModalOpen = true;
            }}
            onkeydown={(e) => {
              if (e.key === "Enter") {
                editingGroup = activeGroup;
                isGroupModalOpen = true;
              }
            }}
            role="button"
            tabindex="0"
          >
            {#if activeGroup.avatar_data_url}
              <img
                src={activeGroup.avatar_data_url}
                alt={activeGroup.name}
                class="nav-avatar"
              />
            {:else}
              <div class="nav-avatar-placeholder">👥</div>
            {/if}
            <div class="nav-char-details">
              <span class="nav-char-name">{activeGroup.name}</span>
              <span class="nav-char-subtitle"
                >Group &bull; {activeGroup.members.length} members</span
              >
            </div>
          </div>
        {:else if activeCharacter}
          <div
            class="active-char-badge"
            onclick={handleEditActiveCharacter}
            onkeydown={(e) => e.key === "Enter" && handleEditActiveCharacter()}
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
              <span class="nav-char-name">{activeCharacter.card.data.name}</span
              >
              <span class="nav-char-subtitle">Edit card</span>
            </div>
          </div>
        {:else}
          <span class="nav-char-name">No character or group</span>
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
            onclick={() => (isCreatorNotesOpen = true)}
            title="Creator Notes"
          >
            <span class="btn-icon">📜</span>
            <span class="btn-text">Notes</span>
          </button>
        {/if}
        <button
          class="icon-nav-btn action-btn-compact lorebook-nav-btn"
          onclick={() => (isLorebookOpen = true)}
          title="World Info & Lorebooks"
        >
          <span class="btn-icon">📖</span>
          <span class="btn-text">Lorebooks</span>
        </button>

        <button
          class="icon-nav-btn action-btn-compact"
          onclick={() => (isPersonaOpen = true)}
          title="User Persona ({userPersona?.name || 'User'})"
        >
          <span class="btn-icon">👤</span>
          <span class="btn-text">Persona</span>
        </button>

        <button
          class="icon-nav-btn action-btn-compact"
          onclick={() => (isSettingsOpen = true)}
          title="API & Generation Settings"
        >
          <span class="btn-icon">⚙️</span>
          <span class="btn-text">Settings</span>
        </button>
      </div>
    </header>

    <!-- Group Turn Bar (Active in Group Chat) -->
    {#if activeGroup}
      <GroupTurnBar
        group={activeGroup}
        {characters}
        {nextSpeakerId}
        {isGenerating}
        {currentStreamingCharacterId}
        onTriggerSpeaker={handleTriggerSpeaker}
        onNextTurn={handleNextTurn}
        onEditGroup={() => {
          editingGroup = activeGroup;
          isGroupModalOpen = true;
        }}
        onToggleTurnMode={handleToggleTurnMode}
        onToggleAutoMode={handleToggleAutoMode}
        onToggleMute={handleToggleMute}
      />
    {/if}

    <!-- Chat Messages Stream -->
    <main class="chat-viewport" bind:this={chatContainerEl}>
      {#if !activeCharacter && !activeGroup}
        <div class="hero-empty">
          <div class="hero-icon">🏰</div>
          <h2>Welcome to Tavern</h2>
          <p>
            Import a character card, create a character, or start a group
            roleplay.
          </p>
          <div class="hero-actions-row">
            <button class="hero-btn" onclick={handleOpenNewCharacter}>
              + Create Character
            </button>
            <button
              class="hero-btn hero-btn-secondary"
              onclick={handleCreateGroup}
            >
              👥 Create Group
            </button>
          </div>
        </div>
      {:else if messages.length === 0}
        <div class="hero-empty">
          <div class="hero-icon">💬</div>
          {#if activeGroup}
            <h2>Start group roleplay in {activeGroup.name}</h2>
            <p>
              {activeGroup.description ||
                "Type a message or trigger a character turn above to begin."}
            </p>
          {:else if activeCharacter}
            <h2>Start a conversation with {activeCharacter.card.data.name}</h2>
            <p>
              {activeCharacter.card.data.scenario ||
                "Type a message below to begin."}
            </p>
          {/if}
        </div>
      {:else}
        <div class="messages-container">
          {#each messages as msg, i (msg.id)}
            <ChatMessage
              message={msg}
              {characters}
              isGroupChat={!!activeGroup}
              characterName={activeCharacter?.card.data.name || "Character"}
              characterAvatar={activeCharacter?.avatar_data_url || null}
              userName={userPersona?.name || "You"}
              userAvatar={userPersona?.avatar_data_url || null}
              {isGenerating}
              continuingText={continuingMessageId === msg.id
                ? streamingText
                : ""}
              onSwipe={handleSwipe}
              onEdit={handleEditMessage}
              onDelete={handleDeleteMessage}
              onForkChat={handleForkChat}
              regexRules={settings?.regex_rules || []}
              onOpenImage={handleOpenImagePreview}
            />
            {#if i === messages.length - 1 && msg.role !== "User" && !isGenerating}
              <div>
                <button
                  class="action-icon-btn continue-btn"
                  disabled={isGenerating}
                  onclick={() => handleContinueGeneration}
                  title="Continue AI message generation"
                >
                  ▶ Continue
                </button>
                <button
                  class="action-icon-btn swipe-new-btn"
                  disabled={isGenerating}
                  onclick={handleRegenerateSwipe}
                  title="Generaate alternate reply"
                >
                  🔄 Swipe
                </button>
              </div>
            {/if}
          {/each}

          <!-- Live Streaming Token Display -->
          {#if isGenerating && streamingText && !continuingMessageId}
            <ChatMessage
              message={{
                id: "temp-streaming",
                parent_id: null,
                role: "Assistant",
                content: streamingText,
                created_at: new Date().toISOString(),
                sibling_index: 0,
                sibling_total: 1,
                can_swipe_left: false,
                can_swipe_right: false,
                character_id: currentStreamingCharacterId,
                name: streamingSpeakerName,
              }}
              {characters}
              isGroupChat={!!activeGroup}
              characterName={streamingSpeakerName ||
                activeCharacter?.card.data.name ||
                "Character"}
              characterAvatar={streamingSpeakerAvatar ||
                activeCharacter?.avatar_data_url ||
                null}
              userName={userPersona?.name || "You"}
              userAvatar={userPersona?.avatar_data_url || null}
              isGenerating={true}
              onSwipe={() => {}}
              onEdit={() => {}}
              onDelete={() => {}}
              regexRules={settings?.regex_rules || []}
              onOpenImage={handleOpenImagePreview}
            />
          {/if}

          <!-- Typing / Thinking Dots Indicator -->
          {#if isGenerating && !streamingText}
            <div class="typing-indicator">
              <div class="dot"></div>
              <div class="dot"></div>
              <div class="dot"></div>
              <span>
                {streamingSpeakerName ||
                  (activeGroup &&
                    characters.find((c) => c.id === nextSpeakerId)?.card.data
                      .name) ||
                  activeCharacter?.card.data.name ||
                  "Character"}
                {continuingMessageId ? "is continuing..." : "is typing..."}
              </span>
            </div>
          {/if}
        </div>
      {/if}
    </main>

    <!-- Bottom Input Bar -->
    {#if activeCharacter || activeGroup}
      <ContextBar
        breakdown={contextBreakdown}
        chatTree={activeChatTree}
        onOpenAuthorsNote={() => (isAuthorsNoteModalOpen = true)}
        onRefresh={() => refreshContextBreakdown(currentDraftText)}
      />
      <ChatInput
        {isGenerating}
        placeholder={activeGroup
          ? `Message ${activeGroup.name}... (Next turn: ${characters.find((c) => c.id === nextSpeakerId)?.card.data.name || "Auto"})`
          : `Message ${activeCharacter?.card.data.name}...`}
        bind:text={inputDraftText}
        onDraftChange={handleDraftChange}
        onSend={handleSendMessage}
        onStop={handleStopGeneration}
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
<AuthorsNoteModal
  isOpen={isAuthorsNoteModalOpen}
  chatTree={activeChatTree}
  onSave={handleSaveAuthorsNote}
  onClose={() => (isAuthorsNoteModalOpen = false)}
/>

<PersonaModal
  activePersona={userPersona!}
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
  characterName={activeCharacter?.card.data.name || "Character"}
  creatorNotes={activeCharacter?.card.data.creator_notes || ""}
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

<SyncModal
  isOpen={isSyncOpen}
  onClose={() => (isSyncOpen = false)}
  onSyncSuccess={handleSyncSuccess}
/>

<GroupModal
  group={editingGroup}
  {characters}
  isOpen={isGroupModalOpen}
  onSave={handleSaveGroup}
  onDelete={handleDeleteGroup}
  onClose={() => (isGroupModalOpen = false)}
/>

<style>
  :global(body) {
    margin: 0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto,
      Helvetica, Arial, sans-serif;
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

  .dot:nth-child(1) {
    animation-delay: -0.32s;
  }
  .dot:nth-child(2) {
    animation-delay: -0.16s;
  }

  @keyframes bounce {
    0%,
    80%,
    100% {
      transform: scale(0);
    }
    40% {
      transform: scale(1);
    }
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

  .hero-actions-row {
    display: flex;
    align-items: center;
    gap: 0.8rem;
    margin-top: 0.5rem;
    flex-wrap: wrap;
    justify-content: center;
  }

  .hero-btn {
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

  .hero-btn-secondary {
    background: #313244;
    border: 1px solid #45475a;
    color: #cdd6f4;
  }

  .hero-btn-secondary:hover {
    background: #45475a;
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

  .continue-btn {
    background: rgba(137, 180, 250, 0.15);
    border-color: rgba(137, 180, 250, 0.4);
    color: #89b4fa;
    font-weight: 600;
  }

  .continue-btn:hover:not(:disabled) {
    background: rgba(137, 180, 250, 0.3);
    color: #b4befe;
  }
</style>
