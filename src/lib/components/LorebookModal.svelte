<script lang="ts">
  import { onMount } from "svelte";
  import type {
    Lorebook,
    LorebookEntry,
    LorebookPosition,
    SelectiveLogic,
    AppSettings,
  } from "../types";
  import {
    getAllLorebooks,
    saveLorebook,
    deleteLorebook,
    importLorebook,
    exportLorebookJson,
    saveSettings,
  } from "../api";
  import {
    SvelteSet
  } from 'svelte/reactivity';

  export let isOpen = false;
  export let settings: AppSettings | null = null;
  export let onSettingsUpdated: ((updated: AppSettings) => void) | null = null;
  export let onClose: () => void;

  let lorebooks: Lorebook[] = [];
  let activeLorebookId: string | null = null;
  let selectedEntryId: string | null = null;
  let searchQuery = "";
  let fileInputEl: HTMLInputElement;

  // Working drafts
  let activeLorebook: Lorebook | null = null;
  let selectedEntry: LorebookEntry | null = null;
  let keyInputStr = "";
  let secKeyInputStr = "";
  let isSaving = false;

  $: activeLorebook = lorebooks.find((b) => b.id === activeLorebookId) || null;
  $: selectedEntry =
    activeLorebook?.entries.find((e) => e.id === selectedEntryId) || null;

  $: filteredEntries = (activeLorebook?.entries || []).filter((entry) => {
    if (!searchQuery.trim()) return true;
    const q = searchQuery.toLowerCase();
    const commentMatch = entry.comment.toLowerCase().includes(q);
    const contentMatch = entry.content.toLowerCase().includes(q);
    const keysMatch = entry.keys.some((k) => k.toLowerCase().includes(q));
    const secKeysMatch = entry.secondary_keys.some((k) =>
      k.toLowerCase().includes(q),
    );
    return commentMatch || contentMatch || keysMatch || secKeysMatch;
  });

  $: isGlobalActive = (settings?.global_lorebook_ids || []).includes(
    activeLorebookId || "",
  );

  $: if (isOpen) {
    loadAllLorebooks();
  }

  async function loadAllLorebooks() {
    try {
      lorebooks = await getAllLorebooks();
      if (!activeLorebookId && lorebooks.length > 0) {
        selectLorebook(lorebooks[0].id);
      } else if (
        activeLorebookId &&
        !lorebooks.some((b) => b.id === activeLorebookId)
      ) {
        activeLorebookId = lorebooks.length > 0 ? lorebooks[0].id : null;
        if (activeLorebookId) selectLorebook(activeLorebookId);
      } else if (activeLorebookId) {
        const found = lorebooks.find((b) => b.id === activeLorebookId);
        if (found && !selectedEntryId && found.entries.length > 0) {
          selectEntry(found.entries[0].id);
        }
      }
    } catch (e) {
      console.error("Failed to load lorebooks:", e);
    }
  }

  function selectLorebook(id: string) {
    activeLorebookId = id;
    const book = lorebooks.find((b) => b.id === id);
    if (book && book.entries.length > 0) {
      selectEntry(book.entries[0].id);
    } else {
      selectedEntryId = null;
    }
  }

  function selectEntry(id: string) {
    selectedEntryId = id;
    const entry = activeLorebook?.entries.find((e) => e.id === id);
    if (entry) {
      keyInputStr = (entry.keys || []).join(", ");
      secKeyInputStr = (entry.secondary_keys || []).join(", ");
    }
  }

  async function handleCreateLorebook() {
    const newBook: Lorebook = {
      id: crypto.randomUUID(),
      name: "New Lorebook",
      description: "",
      scan_depth: 2,
      token_budget: 2048,
      recursive_scanning: false,
      global: false,
      entries: [],
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    };
    await saveLorebook(newBook);
    await loadAllLorebooks();
    selectLorebook(newBook.id);
    handleCreateEntry();
  }

  async function handleCreateEntry() {
    if (!activeLorebook) return;
    const newEntry: LorebookEntry = {
      id: crypto.randomUUID(),
      keys: ["keyword"],
      secondary_keys: [],
      content: "",
      comment: `New Entry ${activeLorebook.entries.length + 1}`,
      enabled: true,
      constant: false,
      selective: false,
      selective_logic: 0,
      position: "before_char",
      depth: 4,
      order: 100,
      case_sensitive: false,
      use_regex: false,
      prevent_recursion: false,
      scan_depth: null,
    };
    activeLorebook.entries = [...activeLorebook.entries, newEntry];
    await persistActiveLorebook();
    selectEntry(newEntry.id);
  }

  async function handleDeleteEntry(id: string) {
    if (!activeLorebook) return;
    activeLorebook.entries = activeLorebook.entries.filter((e) => e.id !== id);
    await persistActiveLorebook();
    if (selectedEntryId === id) {
      selectedEntryId =
        activeLorebook.entries.length > 0 ? activeLorebook.entries[0].id : null;
      if (selectedEntryId) selectEntry(selectedEntryId);
    }
  }

  async function handleDuplicateEntry(entry: LorebookEntry) {
    if (!activeLorebook) return;
    const dup: LorebookEntry = {
      ...JSON.parse(JSON.stringify(entry)),
      id: crypto.randomUUID(),
      comment: `${entry.comment} (Copy)`,
    };
    activeLorebook.entries = [...activeLorebook.entries, dup];
    await persistActiveLorebook();
    selectEntry(dup.id);
  }

  async function handleToggleEntryEnabled(entry: LorebookEntry) {
    entry.enabled = !entry.enabled;
    await persistActiveLorebook();
  }

  async function handleDeleteActiveLorebook() {
    if (!activeLorebook) return;
    if (!confirm(`Delete lorebook "${activeLorebook.name}"?`)) return;
    await deleteLorebook(activeLorebook.id);
    activeLorebookId = null;
    selectedEntryId = null;
    await loadAllLorebooks();
  }

  async function handleDuplicateLorebook() {
    if (!activeLorebook) return;
    const dup: Lorebook = {
      ...JSON.parse(JSON.stringify(activeLorebook)),
      id: crypto.randomUUID(),
      name: `${activeLorebook.name} (Copy)`,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      entries: activeLorebook.entries.map((e) => ({
        ...e,
        id: crypto.randomUUID(),
      })),
    };
    await saveLorebook(dup);
    await loadAllLorebooks();
    selectLorebook(dup.id);
  }

  async function handleToggleGlobal() {
    if (!settings || !activeLorebookId) return;
    const current = new SvelteSet(settings.global_lorebook_ids || []);
    if (current.has(activeLorebookId)) {
      current.delete(activeLorebookId);
    } else {
      current.add(activeLorebookId);
    }
    const updated = {
      ...settings,
      global_lorebook_ids: Array.from(current),
    };
    await saveSettings(updated);
    if (onSettingsUpdated) {
      onSettingsUpdated(updated);
    }
    settings = updated;
  }

  async function persistActiveLorebook() {
    if (!activeLorebook) return;
    isSaving = true;
    try {
      activeLorebook.updated_at = new Date().toISOString();
      await saveLorebook(activeLorebook);
      const idx = lorebooks.findIndex((b) => b.id === activeLorebook?.id);
      if (idx !== -1) {
        lorebooks[idx] = { ...activeLorebook };
      }
    } catch (e) {
      console.error("Failed to save lorebook:", e);
    } finally {
      isSaving = false;
    }
  }

  function handleKeyChange() {
    if (!selectedEntry) return;
    selectedEntry.keys = keyInputStr
      .split(",")
      .map((k) => k.trim())
      .filter(Boolean);
    persistActiveLorebook();
  }

  function handleSecKeyChange() {
    if (!selectedEntry) return;
    selectedEntry.secondary_keys = secKeyInputStr
      .split(",")
      .map((k) => k.trim())
      .filter(Boolean);
    persistActiveLorebook();
  }

  async function handleImportFile(e: Event) {
    const target = e.target as HTMLInputElement;
    if (target.files && target.files[0]) {
      try {
        const file = target.files[0];
        const buffer = await file.arrayBuffer();
        const imported = await importLorebook(new Uint8Array(buffer));
        await loadAllLorebooks();
        selectLorebook(imported.id);
      } catch (err) {
        alert(`Import failed: ${err}`);
      } finally {
        target.value = "";
      }
    }
  }

  async function handleExportJson() {
    if (!activeLorebook) return;
    try {
      const jsonStr = await exportLorebookJson(activeLorebook.id);
      const blob = new Blob([jsonStr], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `${activeLorebook.name.toLowerCase().replace(/[^a-z0-9]/g, "_")}_lorebook.json`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      alert(`Export failed: ${e}`);
    }
  }

  function estimateTokens(text: string): number {
    return Math.ceil((text.length + 3) / 4);
  }
</script>

<input
  type="file"
  accept=".json,.png"
  bind:this={fileInputEl}
  on:change={handleImportFile}
  style="display: none;"
/>

{#if isOpen}
  <div
    class="modal-backdrop"
    on:click={onClose}
    on:keydown={(e) => e.key === "Escape" && onClose()}
    role="presentation"
  >
    <div
      class="lorebook-modal-card"
      on:click|stopPropagation
      on:keydown|stopPropagation
      role="dialog"
      aria-modal="true"
      tabindex="-1"
    >
      <!-- Modal Header -->
      <div class="modal-header">
        <div class="header-left">
          <h2>📖 World Info & Lorebooks</h2>
          <span class="header-subtitle"
            >SillyTavern compatible dynamic context injector</span
          >
        </div>
        <div class="header-actions">
          <button
            class="action-btn import-btn"
            on:click={() => fileInputEl?.click()}
            title="Import JSON or PNG Card with embedded Lorebook"
          >
            📥 Import
          </button>
          <button class="action-btn new-btn" on:click={handleCreateLorebook}>
            + New Lorebook
          </button>
          <button class="close-btn" on:click={onClose} title="Close">✕</button>
        </div>
      </div>

      <!-- 3-Column Layout: Lorebook List | Entry List | Entry Editor -->
      <div class="modal-body-layout">
        <!-- Column 1: Lorebooks Navigation Sidebar -->
        <aside class="col-lorebooks">
          <div class="col-header">
            <span class="col-title">Lorebooks ({lorebooks.length})</span>
          </div>

          <div class="lorebook-list">
            {#if lorebooks.length === 0}
              <div class="empty-col">
                <p>No lorebooks yet.</p>
                <button class="link-btn" on:click={handleCreateLorebook}
                  >+ Create one</button
                >
              </div>
            {:else}
              {#each lorebooks as book (book.id)}
                <div
                  class="lorebook-item {book.id === activeLorebookId
                    ? 'active'
                    : ''}"
                  on:click={() => selectLorebook(book.id)}
                  on:keydown={(e) =>
                    e.key === "Enter" && selectLorebook(book.id)}
                  role="button"
                  tabindex="0"
                >
                  <div class="book-info">
                    <div class="book-title-row">
                      <span class="book-name">{book.name}</span>
                      {#if (settings?.global_lorebook_ids || []).includes(book.id)}
                        <span class="global-badge" title="Active Globally"
                          >Global</span
                        >
                      {/if}
                    </div>
                    <span class="book-meta">
                      {book.entries.length}
                      {book.entries.length === 1 ? "entry" : "entries"}
                    </span>
                  </div>
                </div>
              {/each}
            {/if}
          </div>
        </aside>

        <!-- Column 2: Active Lorebook Settings & Entries List -->
        {#if activeLorebook}
          <section class="col-entries">
            <!-- Lorebook Metadata Panel -->
            <div class="lorebook-meta-panel">
              <div class="meta-row-main">
                <input
                  type="text"
                  class="lorebook-title-input"
                  bind:value={activeLorebook.name}
                  on:input={persistActiveLorebook}
                  placeholder="Lorebook Name"
                />
                <div class="meta-btn-group">
                  <button
                    class="toggle-global-btn {isGlobalActive ? 'active' : ''}"
                    on:click={handleToggleGlobal}
                    title="Toggle active globally for all characters & chats"
                  >
                    🌐 {isGlobalActive ? "Global: ON" : "Global: OFF"}
                  </button>
                  <button
                    class="icon-tool-btn"
                    on:click={handleExportJson}
                    title="Export as JSON"
                  >
                    💾
                  </button>
                  <button
                    class="icon-tool-btn"
                    on:click={handleDuplicateLorebook}
                    title="Duplicate Lorebook"
                  >
                    📑
                  </button>
                  <button
                    class="icon-tool-btn delete-btn"
                    on:click={handleDeleteActiveLorebook}
                    title="Delete Lorebook"
                  >
                    🗑️
                  </button>
                </div>
              </div>

              <!-- Lorebook Settings Quick Grid -->
              <div class="meta-settings-grid">
                <div class="meta-field">
                  <label for="scan-depth-input">Scan Depth</label>
                  <input
                    id="scan-depth-input"
                    type="number"
                    min="0"
                    max="100"
                    bind:value={activeLorebook.scan_depth}
                    on:change={persistActiveLorebook}
                    title="0 = scan all chat history; otherwise scans last N messages"
                  />
                </div>
                <div class="meta-field">
                  <label for="token-budget-input">Token Budget</label>
                  <input
                    id="token-budget-input"
                    type="number"
                    min="0"
                    step="128"
                    bind:value={activeLorebook.token_budget}
                    on:change={persistActiveLorebook}
                    title="Max tokens for activated entries (0 = unlimited)"
                  />
                </div>
                <div class="meta-field checkbox-field">
                  <label
                    class="checkbox-label"
                    title="Activated entries can trigger other entries"
                  >
                    <input
                      type="checkbox"
                      bind:checked={activeLorebook.recursive_scanning}
                      on:change={persistActiveLorebook}
                    />
                    <span>Recursive Scan</span>
                  </label>
                </div>
              </div>
            </div>

            <!-- Entries List Header & Search -->
            <div class="entries-toolbar">
              <input
                type="text"
                class="search-entries-input"
                placeholder="Search entries, keys, content..."
                bind:value={searchQuery}
              />
              <button class="add-entry-btn" on:click={handleCreateEntry}>
                + Entry
              </button>
            </div>

            <!-- Entries Scroll Area -->
            <div class="entries-list">
              {#if filteredEntries.length === 0}
                <div class="empty-col">
                  <p>No entries found.</p>
                  <button class="link-btn" on:click={handleCreateEntry}
                    >+ Add first entry</button
                  >
                </div>
              {:else}
                {#each filteredEntries as entry (entry.id)}
                  <div
                    class="entry-card {entry.id === selectedEntryId
                      ? 'active'
                      : ''} {!entry.enabled ? 'disabled' : ''}"
                    on:click={() => selectEntry(entry.id)}
                    on:keydown={(e) =>
                      e.key === "Enter" && selectEntry(entry.id)}
                    role="button"
                    tabindex="0"
                  >
                    <div class="entry-card-header">
                      <button
                        class="status-dot {entry.enabled
                          ? 'enabled'
                          : 'disabled'}"
                        on:click|stopPropagation={() =>
                          handleToggleEntryEnabled(entry)}
                        title={entry.enabled
                          ? "Click to disable"
                          : "Click to enable"}
                      ></button>
                      <span class="entry-title"
                        >{entry.comment || "Untitled Entry"}</span
                      >
                      <span class="position-badge"
                        >{entry.position}{entry.position === "at_depth"
                          ? ` (D=${entry.depth})`
                          : ""}</span
                      >
                    </div>

                    <div class="entry-keys-row">
                      {#if entry.constant}
                        <span class="key-pill constant"
                          >Constant (Always On)</span
                        >
                      {:else if entry.keys.length > 0}
                        {#each entry.keys.slice(0, 3) as key (key)}
                          <span class="key-pill">{key}</span>
                        {/each}
                        {#if entry.keys.length > 3}
                          <span class="key-pill more"
                            >+{entry.keys.length - 3}</span
                          >
                        {/if}
                      {:else}
                        <span class="key-pill no-keys">No keys</span>
                      {/if}
                    </div>

                    <p class="entry-preview">
                      {entry.content
                        ? entry.content.slice(0, 75) +
                          (entry.content.length > 75 ? "..." : "")
                        : "No content"}
                    </p>
                  </div>
                {/each}
              {/if}
            </div>
          </section>

          <!-- Column 3: Selected Entry Detail Editor -->
          {#if selectedEntry}
            <main class="col-editor">
              <div class="editor-header">
                <div class="editor-title-row">
                  <input
                    type="text"
                    class="entry-comment-input"
                    bind:value={selectedEntry.comment}
                    on:input={persistActiveLorebook}
                    placeholder="Entry Title / Comment"
                  />
                  <div class="editor-actions">
                    <button
                      class="entry-toggle-btn {selectedEntry.enabled
                        ? 'active'
                        : ''}"
                      on:click={() => handleToggleEntryEnabled(selectedEntry)}
                    >
                      {selectedEntry.enabled ? "Active" : "Disabled"}
                    </button>
                    <button
                      class="icon-tool-btn"
                      on:click={() => handleDuplicateEntry(selectedEntry)}
                      title="Duplicate Entry"
                    >
                      📑
                    </button>
                    <button
                      class="icon-tool-btn delete-btn"
                      on:click={() => handleDeleteEntry(selectedEntry.id)}
                      title="Delete Entry"
                    >
                      🗑️
                    </button>
                  </div>
                </div>
              </div>

              <div class="editor-scroll-body">
                <!-- Primary Trigger Keys -->
                <div class="form-group">
                  <div class="form-label-row">
                    <label for="primary-keys-input"
                      >Trigger Keywords (Primary)</label
                    >
                    <span class="field-hint">Comma separated</span>
                  </div>
                  <input
                    id="primary-keys-input"
                    type="text"
                    bind:value={keyInputStr}
                    on:input={handleKeyChange}
                    placeholder="e.g. tavern, inn, bartender"
                    disabled={selectedEntry.constant}
                  />
                </div>

                <!-- Activation Mode Flags -->
                <div class="form-row-flags">
                  <label
                    class="checkbox-label"
                    title="Always insert this entry regardless of chat keywords"
                  >
                    <input
                      type="checkbox"
                      bind:checked={selectedEntry.constant}
                      on:change={persistActiveLorebook}
                    />
                    <span>Constant (Always Active)</span>
                  </label>

                  <label
                    class="checkbox-label"
                    title="Require secondary keys to trigger this entry"
                  >
                    <input
                      type="checkbox"
                      bind:checked={selectedEntry.selective}
                      on:change={persistActiveLorebook}
                      disabled={selectedEntry.constant}
                    />
                    <span>Selective Logic</span>
                  </label>
                </div>

                <!-- Secondary Keys (Selective Logic) -->
                {#if selectedEntry.selective && !selectedEntry.constant}
                  <div class="selective-box">
                    <div class="form-group">
                      <div class="form-label-row">
                        <label for="sec-keys-input">Secondary Keywords</label>
                        <span class="field-hint">Comma separated</span>
                      </div>
                      <input
                        id="sec-keys-input"
                        type="text"
                        bind:value={secKeyInputStr}
                        on:input={handleSecKeyChange}
                        placeholder="e.g. sword, relic, secret"
                      />
                    </div>

                    <div class="form-group">
                      <label for="selective-logic-select"
                        >Selective Logic Condition</label
                      >
                      <select
                        id="selective-logic-select"
                        bind:value={selectedEntry.selective_logic}
                        on:change={persistActiveLorebook}
                      >
                        <option value={0}
                          >AND ANY — Require at least one secondary key</option
                        >
                        <option value={1}
                          >NOT ANY — Exclude if any secondary key is present</option
                        >
                        <option value={2}
                          >AND ALL — Require ALL secondary keys to be present</option
                        >
                        <option value={3}
                          >NOT ALL — Exclude if all secondary keys are present</option
                        >
                      </select>
                    </div>
                  </div>
                {/if}

                <!-- Placement & Ordering -->
                <div class="form-row-dual">
                  <div class="form-group">
                    <label for="position-select"
                      >Insertion Strategy / Position</label
                    >
                    <select
                      id="position-select"
                      bind:value={selectedEntry.position}
                      on:change={persistActiveLorebook}
                    >
                      <option value="before_char"
                        >Before Character Definition</option
                      >
                      <option value="after_char"
                        >After Character Definition</option
                      >
                      <option value="before_scenario">Before Scenario</option>
                      <option value="after_scenario">After Scenario</option>
                      <option value="top_system">Top of System Message</option>
                      <option value="bottom_system"
                        >Bottom of System Message</option
                      >
                      <option value="at_depth">In Chat History at Depth</option>
                    </select>
                  </div>

                  {#if selectedEntry.position === "at_depth"}
                    <div class="form-group">
                      <label for="depth-input">History Depth</label>
                      <input
                        id="depth-input"
                        type="number"
                        min="0"
                        max="50"
                        bind:value={selectedEntry.depth}
                        on:change={persistActiveLorebook}
                        title="0 = after latest message; 1 = before latest message, etc."
                      />
                    </div>
                  {:else}
                    <div class="form-group">
                      <label for="order-input">Insertion Order / Priority</label
                      >
                      <input
                        id="order-input"
                        type="number"
                        bind:value={selectedEntry.order}
                        on:change={persistActiveLorebook}
                      />
                    </div>
                  {/if}
                </div>

                <!-- Advanced Options Collapsible -->
                <details class="advanced-details">
                  <summary>Advanced Matching Options</summary>
                  <div class="advanced-grid">
                    <label
                      class="checkbox-label"
                      title="Distinguish uppercase and lowercase characters"
                    >
                      <input
                        type="checkbox"
                        bind:checked={selectedEntry.case_sensitive}
                        on:change={persistActiveLorebook}
                      />
                      <span>Case Sensitive</span>
                    </label>

                    <label
                      class="checkbox-label"
                      title="Treat trigger keywords as Regular Expressions"
                    >
                      <input
                        type="checkbox"
                        bind:checked={selectedEntry.use_regex}
                        on:change={persistActiveLorebook}
                      />
                      <span>Regex Match</span>
                    </label>

                    <label
                      class="checkbox-label"
                      title="Do not scan this entry's content during recursive scanning"
                    >
                      <input
                        type="checkbox"
                        bind:checked={selectedEntry.prevent_recursion}
                        on:change={persistActiveLorebook}
                      />
                      <span>Prevent Recursion</span>
                    </label>
                  </div>
                </details>

                <!-- Entry Content -->
                <div class="form-group content-group">
                  <div class="form-label-row">
                    <label for="entry-content-textarea"
                      >Entry Content (Injected Text)</label
                    >
                    <span class="token-count">
                      ~{estimateTokens(selectedEntry.content)} tokens ({selectedEntry
                        .content.length} chars)
                    </span>
                  </div>
                  <textarea
                    id="entry-content-textarea"
                    rows="8"
                    bind:value={selectedEntry.content}
                    on:input={persistActiveLorebook}
                    placeholder="Enter lore context to be injected when triggered. Supports macros like &#123;&#123;char&#125;&#125;, &#123;&#123;user&#125;&#125;, &#123;&#123;scenario&#125;&#125;..."
                  ></textarea>
                  <div class="macro-chips">
                    <button
                      type="button"
                      class="chip"
                      on:click={() => {
                        if (selectedEntry) {
                          selectedEntry.content += " {{char}}";
                          persistActiveLorebook();
                        }
                      }}>+ &#123;&#123;char&#125;&#125;</button
                    >
                    <button
                      type="button"
                      class="chip"
                      on:click={() => {
                        if (selectedEntry) {
                          selectedEntry.content += " {{user}}";
                          persistActiveLorebook();
                        }
                      }}>+ &#123;&#123;user&#125;&#125;</button
                    >
                    <button
                      type="button"
                      class="chip"
                      on:click={() => {
                        if (selectedEntry) {
                          selectedEntry.content += " {{scenario}}";
                          persistActiveLorebook();
                        }
                      }}>+ &#123;&#123;scenario&#125;&#125;</button
                    >
                  </div>
                </div>
              </div>
            </main>
          {:else}
            <div class="col-editor empty-state-box">
              <div class="empty-icon">📝</div>
              <h3>No Entry Selected</h3>
              <p>
                Select an entry from the list or create a new one to edit its
                details.
              </p>
              <button class="add-entry-btn" on:click={handleCreateEntry}
                >+ Create Entry</button
              >
            </div>
          {/if}
        {:else}
          <div class="empty-state-full">
            <div class="empty-icon">📚</div>
            <h3>No Lorebook Selected</h3>
            <p>
              Select or create a lorebook to view and edit its world info
              entries.
            </p>
            <button class="action-btn new-btn" on:click={handleCreateLorebook}
              >+ Create Lorebook</button
            >
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
    padding: 1rem;
  }

  .lorebook-modal-card {
    background: #181825;
    border: 1px solid #313244;
    border-radius: 12px;
    width: 100%;
    max-width: 1200px;
    height: 88vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 20px 50px rgba(0, 0, 0, 0.6);
  }

  /* Header */
  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.9rem 1.4rem;
    border-bottom: 1px solid #313244;
    background: #1e1e2e;
  }

  .header-left h2 {
    margin: 0;
    font-size: 1.15rem;
    font-weight: 700;
    color: #cdd6f4;
  }

  .header-subtitle {
    font-size: 0.78rem;
    color: #a6adc8;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }

  .action-btn {
    padding: 0.4rem 0.8rem;
    border-radius: 6px;
    font-size: 0.82rem;
    font-weight: 600;
    cursor: pointer;
    border: none;
    transition: background 0.15s;
  }

  .import-btn {
    background: #313244;
    color: #cdd6f4;
  }
  .import-btn:hover {
    background: #45475a;
  }

  .new-btn {
    background: #89b4fa;
    color: #11111b;
  }
  .new-btn:hover {
    background: #b4befe;
  }

  .close-btn {
    background: none;
    border: none;
    color: #6c7086;
    font-size: 1.1rem;
    cursor: pointer;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
  }
  .close-btn:hover {
    color: #cdd6f4;
    background: #313244;
  }

  /* 3-Column Layout */
  .modal-body-layout {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  /* Column 1: Lorebooks List */
  .col-lorebooks {
    width: 240px;
    border-right: 1px solid #313244;
    display: flex;
    flex-direction: column;
    background: #11111b;
    flex-shrink: 0;
  }

  .col-header {
    padding: 0.7rem 0.9rem;
    border-bottom: 1px solid #313244;
    font-size: 0.8rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: #a6adc8;
  }

  .lorebook-list {
    flex: 1;
    overflow-y: auto;
    padding: 0.4rem;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }

  .lorebook-item {
    padding: 0.6rem 0.8rem;
    border-radius: 8px;
    background: #181825;
    border: 1px solid transparent;
    cursor: pointer;
    transition: all 0.15s ease;
  }
  .lorebook-item:hover {
    background: #313244;
  }
  .lorebook-item.active {
    background: #1e1e2e;
    border-color: #89b4fa;
  }

  .book-title-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.4rem;
  }

  .book-name {
    font-size: 0.85rem;
    font-weight: 600;
    color: #cdd6f4;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .global-badge {
    font-size: 0.65rem;
    padding: 0.1rem 0.35rem;
    border-radius: 4px;
    background: #a6e3a1;
    color: #11111b;
    font-weight: 700;
  }

  .book-meta {
    font-size: 0.72rem;
    color: #6c7086;
  }

  /* Column 2: Entries List */
  .col-entries {
    width: 320px;
    border-right: 1px solid #313244;
    display: flex;
    flex-direction: column;
    background: #181825;
    flex-shrink: 0;
  }

  .lorebook-meta-panel {
    padding: 0.7rem 0.9rem;
    border-bottom: 1px solid #313244;
    background: #1e1e2e;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .meta-row-main {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }

  .lorebook-title-input {
    flex: 1;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 4px;
    color: #cdd6f4;
    font-size: 0.95rem;
    font-weight: 700;
    padding: 0.2rem 0.4rem;
  }
  .lorebook-title-input:focus {
    background: #11111b;
    border-color: #89b4fa;
    outline: none;
  }

  .meta-btn-group {
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }

  .toggle-global-btn {
    padding: 0.25rem 0.5rem;
    border-radius: 6px;
    font-size: 0.72rem;
    font-weight: 700;
    cursor: pointer;
    border: 1px solid #45475a;
    background: #313244;
    color: #a6adc8;
    transition: all 0.15s;
  }
  .toggle-global-btn.active {
    background: #a6e3a1;
    color: #11111b;
    border-color: #a6e3a1;
  }

  .icon-tool-btn {
    background: #313244;
    border: none;
    color: #cdd6f4;
    padding: 0.3rem 0.5rem;
    border-radius: 6px;
    font-size: 0.8rem;
    cursor: pointer;
    transition: background 0.15s;
  }
  .icon-tool-btn:hover {
    background: #45475a;
  }
  .icon-tool-btn.delete-btn:hover {
    background: #f38ba8;
    color: #11111b;
  }

  .meta-settings-grid {
    display: grid;
    grid-template-columns: 1fr 1fr 1.2fr;
    gap: 0.5rem;
    align-items: end;
  }

  .meta-field {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }
  .meta-field label {
    font-size: 0.68rem;
    color: #a6adc8;
    text-transform: uppercase;
    font-weight: 600;
  }
  .meta-field input[type="number"] {
    background: #11111b;
    border: 1px solid #313244;
    border-radius: 4px;
    color: #cdd6f4;
    padding: 0.25rem 0.4rem;
    font-size: 0.78rem;
  }

  .meta-field.checkbox-field {
    justify-content: center;
    padding-bottom: 0.2rem;
  }

  .entries-toolbar {
    padding: 0.5rem 0.8rem;
    border-bottom: 1px solid #313244;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: #181825;
  }

  .search-entries-input {
    flex: 1;
    background: #11111b;
    border: 1px solid #313244;
    border-radius: 6px;
    padding: 0.35rem 0.6rem;
    color: #cdd6f4;
    font-size: 0.8rem;
  }
  .search-entries-input:focus {
    outline: none;
    border-color: #89b4fa;
  }

  .add-entry-btn {
    background: #cba6f7;
    color: #11111b;
    border: none;
    border-radius: 6px;
    padding: 0.35rem 0.7rem;
    font-size: 0.8rem;
    font-weight: 700;
    cursor: pointer;
  }
  .add-entry-btn:hover {
    background: #f5c2e7;
  }

  .entries-list {
    flex: 1;
    overflow-y: auto;
    padding: 0.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .entry-card {
    padding: 0.6rem 0.75rem;
    background: #11111b;
    border: 1px solid #313244;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.15s ease;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  .entry-card:hover {
    border-color: #45475a;
  }
  .entry-card.active {
    border-color: #cba6f7;
    background: #1e1e2e;
  }
  .entry-card.disabled {
    opacity: 0.6;
  }

  .entry-card-header {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .status-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    border: none;
    padding: 0;
    cursor: pointer;
  }
  .status-dot.enabled {
    background: #a6e3a1;
    box-shadow: 0 0 6px rgba(166, 227, 161, 0.5);
  }
  .status-dot.disabled {
    background: #6c7086;
  }

  .entry-title {
    font-size: 0.85rem;
    font-weight: 700;
    color: #cdd6f4;
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .position-badge {
    font-size: 0.65rem;
    padding: 0.1rem 0.35rem;
    border-radius: 4px;
    background: #313244;
    color: #a6adc8;
  }

  .entry-keys-row {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
  }

  .key-pill {
    font-size: 0.68rem;
    padding: 0.1rem 0.4rem;
    border-radius: 4px;
    background: #313244;
    color: #89b4fa;
    font-weight: 600;
  }
  .key-pill.constant {
    background: #f9e2af;
    color: #11111b;
  }
  .key-pill.more {
    background: transparent;
    color: #6c7086;
  }
  .key-pill.no-keys {
    color: #6c7086;
    font-style: italic;
  }

  .entry-preview {
    font-size: 0.75rem;
    color: #a6adc8;
    margin: 0;
    line-height: 1.25;
  }

  /* Column 3: Entry Detail Editor */
  .col-editor {
    flex: 1;
    display: flex;
    flex-direction: column;
    background: #1e1e2e;
    overflow: hidden;
  }

  .editor-header {
    padding: 0.8rem 1.2rem;
    border-bottom: 1px solid #313244;
    background: #181825;
  }

  .editor-title-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
  }

  .entry-comment-input {
    flex: 1;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 4px;
    color: #cdd6f4;
    font-size: 1.05rem;
    font-weight: 800;
    padding: 0.2rem 0.4rem;
  }
  .entry-comment-input:focus {
    background: #11111b;
    border-color: #cba6f7;
    outline: none;
  }

  .editor-actions {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .entry-toggle-btn {
    padding: 0.3rem 0.7rem;
    border-radius: 6px;
    font-size: 0.78rem;
    font-weight: 700;
    cursor: pointer;
    border: 1px solid #45475a;
    background: #313244;
    color: #a6adc8;
  }
  .entry-toggle-btn.active {
    background: #a6e3a1;
    color: #11111b;
    border-color: #a6e3a1;
  }

  .editor-scroll-body {
    flex: 1;
    overflow-y: auto;
    padding: 1.2rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .form-label-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .form-group label {
    font-size: 0.8rem;
    font-weight: 700;
    color: #cdd6f4;
  }

  .field-hint {
    font-size: 0.72rem;
    color: #a6adc8;
  }

  .token-count {
    font-size: 0.72rem;
    color: #89b4fa;
    font-family: monospace;
  }

  .form-group input,
  .form-group select,
  .form-group textarea {
    background: #11111b;
    border: 1px solid #313244;
    border-radius: 6px;
    color: #cdd6f4;
    padding: 0.5rem 0.7rem;
    font-size: 0.85rem;
    font-family: inherit;
  }
  .form-group input:focus,
  .form-group select:focus,
  .form-group textarea:focus {
    outline: none;
    border-color: #cba6f7;
  }

  .form-row-flags {
    display: flex;
    align-items: center;
    gap: 1.5rem;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.82rem;
    color: #cdd6f4;
    cursor: pointer;
  }
  .checkbox-label input[type="checkbox"] {
    accent-color: #cba6f7;
    width: 15px;
    height: 15px;
  }

  .selective-box {
    background: #11111b;
    border: 1px solid #313244;
    border-radius: 8px;
    padding: 0.8rem;
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
  }

  .form-row-dual {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
  }

  .advanced-details {
    background: #11111b;
    border: 1px solid #313244;
    border-radius: 8px;
    padding: 0.6rem 0.8rem;
  }
  .advanced-details summary {
    font-size: 0.8rem;
    font-weight: 700;
    color: #a6adc8;
    cursor: pointer;
  }
  .advanced-grid {
    margin-top: 0.8rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .content-group textarea {
    resize: vertical;
    line-height: 1.4;
  }

  .macro-chips {
    display: flex;
    gap: 0.4rem;
    margin-top: 0.2rem;
  }
  .chip {
    font-size: 0.72rem;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    background: #313244;
    color: #cdd6f4;
    cursor: pointer;
    font-family: monospace;
    transition: background 0.15s;
  }
  .chip:hover {
    background: #45475a;
  }

  /* Empty States */
  .empty-col {
    padding: 2rem 1rem;
    text-align: center;
    color: #6c7086;
    font-size: 0.85rem;
  }

  .empty-state-box,
  .empty-state-full {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 2rem;
    text-align: center;
    color: #a6adc8;
  }

  .empty-icon {
    font-size: 3rem;
    margin-bottom: 0.8rem;
  }

  .link-btn {
    background: none;
    border: none;
    color: #89b4fa;
    font-size: 0.82rem;
    cursor: pointer;
    text-decoration: underline;
    margin-top: 0.4rem;
  }

  @media (max-width: 900px) {
    .modal-body-layout {
      flex-direction: column;
    }
    .col-lorebooks,
    .col-entries {
      width: 100%;
      height: 200px;
      border-right: none;
      border-bottom: 1px solid #313244;
    }
  }
</style>
