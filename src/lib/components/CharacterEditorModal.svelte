<script lang="ts">
  import type { Character, CharacterCardV2, CharacterBook, Lorebook, LorebookEntry } from '../types';
  import { exportCardJson, exportCardPng, getAllLorebooks } from '../api';

  export let character: Character | null = null;
  export let isOpen = false;
  export let onSave: (saved: Character) => void;
  export let onClose: () => void;

  let activeTab: 'basic' | 'greetings' | 'lorebook' | 'advanced' = 'basic';
  let fileInputEl: HTMLInputElement;
  let tagInputStr = '';

  let draftCard: CharacterCardV2;
  let draftAvatar: string | null = null;
  let draftId: string | null = null;
  let alternateGreetings: string[] = [];

  // Lorebook state
  let availableLorebooks: Lorebook[] = [];
  let linkedLorebookIds: string[] = [];
  let hasEmbeddedBook = false;
  let embeddedBook: CharacterBook = {
    name: null,
    description: null,
    scan_depth: 2,
    token_budget: 2048,
    recursive_scanning: false,
    entries: [],
  };
  let selectedEmbeddedEntryId: string | null = null;
  let embeddedKeyStr = '';
  let embeddedSecKeyStr = '';

  let wasOpen = false;
  $: if (isOpen && !wasOpen) {
    wasOpen = true;
    initEditor();
  } else if (!isOpen) {
    wasOpen = false;
  }

  async function initEditor() {
    try {
      availableLorebooks = await getAllLorebooks();
    } catch (e) {
      console.error('Failed to fetch lorebooks:', e);
    }

    if (character) {
      draftId = character.id;
      draftCard = JSON.parse(JSON.stringify(character.card));
      draftAvatar = character.avatar_data_url;
      alternateGreetings = [...(character.card.data.alternate_greetings || [])];
      tagInputStr = (character.card.data.tags || []).join(', ');
      linkedLorebookIds = [...(character.card.data.lorebook_ids || [])];

      if (character.card.data.character_book) {
        hasEmbeddedBook = true;
        embeddedBook = JSON.parse(JSON.stringify(character.card.data.character_book));
        if (embeddedBook.entries.length > 0) {
          selectEmbeddedEntry(embeddedBook.entries[0].id);
        }
      } else {
        hasEmbeddedBook = false;
        embeddedBook = {
          name: null,
          description: null,
          scan_depth: 2,
          token_budget: 2048,
          recursive_scanning: false,
          entries: [],
        };
        selectedEmbeddedEntryId = null;
      }
    } else {
      draftId = null;
      draftCard = {
        spec: 'chara_card_v2',
        spec_version: '2.0',
        data: {
          name: '',
          description: '',
          personality: '',
          scenario: '',
          first_mes: '',
          mes_example: '',
          creator_notes: '',
          system_prompt: '',
          post_history_instructions: '',
          alternate_greetings: [],
          tags: [],
          character_book: null,
          lorebook_ids: [],
        },
      };
      draftAvatar = null;
      alternateGreetings = [];
      tagInputStr = '';
      linkedLorebookIds = [];
      hasEmbeddedBook = false;
      embeddedBook = {
        name: null,
        description: null,
        scan_depth: 2,
        token_budget: 2048,
        recursive_scanning: false,
        entries: [],
      };
      selectedEmbeddedEntryId = null;
    }
  }

  $: selectedEmbeddedEntry = embeddedBook.entries.find((e) => e.id === selectedEmbeddedEntryId) || null;

  function selectEmbeddedEntry(id: string) {
    selectedEmbeddedEntryId = id;
    const entry = embeddedBook.entries.find((e) => e.id === id);
    if (entry) {
      embeddedKeyStr = (entry.keys || []).join(', ');
      embeddedSecKeyStr = (entry.secondary_keys || []).join(', ');
    }
  }

  function handleAddEmbeddedEntry() {
    const newEntry: LorebookEntry = {
      id: crypto.randomUUID(),
      keys: ['keyword'],
      secondary_keys: [],
      content: '',
      comment: `Lore Entry ${embeddedBook.entries.length + 1}`,
      enabled: true,
      constant: false,
      selective: false,
      selective_logic: 0,
      position: 'before_char',
      depth: 4,
      order: 100,
      case_sensitive: false,
      use_regex: false,
      prevent_recursion: false,
      scan_depth: null,
    };
    embeddedBook.entries = [...embeddedBook.entries, newEntry];
    selectEmbeddedEntry(newEntry.id);
  }

  function handleDeleteEmbeddedEntry(id: string) {
    embeddedBook.entries = embeddedBook.entries.filter((e) => e.id !== id);
    if (selectedEmbeddedEntryId === id) {
      selectedEmbeddedEntryId = embeddedBook.entries.length > 0 ? embeddedBook.entries[0].id : null;
      if (selectedEmbeddedEntryId) selectEmbeddedEntry(selectedEmbeddedEntryId);
    }
  }

  function handleEmbeddedKeyChange() {
    if (!selectedEmbeddedEntry) return;
    selectedEmbeddedEntry.keys = embeddedKeyStr
      .split(',')
      .map((k) => k.trim())
      .filter(Boolean);
  }

  function handleEmbeddedSecKeyChange() {
    if (!selectedEmbeddedEntry) return;
    selectedEmbeddedEntry.secondary_keys = embeddedSecKeyStr
      .split(',')
      .map((k) => k.trim())
      .filter(Boolean);
  }

  function toggleLinkedLorebook(lbId: string) {
    if (linkedLorebookIds.includes(lbId)) {
      linkedLorebookIds = linkedLorebookIds.filter((id) => id !== lbId);
    } else {
      linkedLorebookIds = [...linkedLorebookIds, lbId];
    }
  }

  function handleAvatarUpload(e: Event) {
    const target = e.target as HTMLInputElement;
    if (target.files && target.files[0]) {
      const file = target.files[0];
      const reader = new FileReader();
      reader.onload = (event) => {
        if (event.target?.result) {
          draftAvatar = event.target.result as string;
        }
      };
      reader.readAsDataURL(file);
      target.value = '';
    }
  }

  function removeAvatar() {
    draftAvatar = null;
  }

  function addAlternateGreeting() {
    alternateGreetings = [...alternateGreetings, ''];
  }

  function removeAlternateGreeting(index: number) {
    alternateGreetings = alternateGreetings.filter((_, i) => i !== index);
  }

  async function handleExportPng() {
    if (!draftId) return;
    try {
      const bytes = await exportCardPng(draftId);
      const blob = new Blob([new Uint8Array(bytes)], { type: 'image/png' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `${draftCard.data.name || 'character'}_card.png`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      alert(`Export failed: ${e}`);
    }
  }

  async function handleExportJson() {
    if (!draftId) return;
    try {
      const jsonStr = await exportCardJson(draftId);
      const blob = new Blob([jsonStr], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `${draftCard.data.name || 'character'}_card.json`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      alert(`Export failed: ${e}`);
    }
  }

  function save() {
    if (!draftCard.data.name.trim()) {
      alert('Please enter a character name.');
      return;
    }

    draftCard.data.tags = tagInputStr
      .split(',')
      .map((t) => t.trim())
      .filter(Boolean);

    draftCard.data.alternate_greetings = alternateGreetings.filter((g) => g.trim().length > 0);

    // Save embedded character book
    if (hasEmbeddedBook) {
      draftCard.data.character_book = embeddedBook;
    } else {
      draftCard.data.character_book = null;
    }

    // Save linked lorebook IDs
    draftCard.data.lorebook_ids = linkedLorebookIds;

    const now = new Date().toISOString();
    const characterToSave: Character = {
      id: draftId || crypto.randomUUID(),
      card: draftCard,
      avatar_data_url: draftAvatar,
      created_at: character?.created_at || now,
      updated_at: now,
    };

    onSave(characterToSave);
    onClose();
  }
</script>

{#if isOpen && draftCard}
  <input
    type="file"
    accept="image/png,image/jpeg,image/webp"
    bind:this={fileInputEl}
    on:change={handleAvatarUpload}
    style="display: none;"
  />
  <div class="modal-backdrop" on:click={onClose} on:keydown={(e) => e.key === 'Escape' && onClose()} role="presentation">
    <div class="modal-card" on:click|stopPropagation on:keydown|stopPropagation role="dialog" aria-modal="true" tabindex="-1">
      <div class="modal-header">
        <h2>{draftId ? `Edit: ${draftCard.data.name || 'Character'}` : 'Create New Character'}</h2>
        <div class="header-actions">
          {#if draftId}
            <button class="export-btn" on:click={handleExportPng} title="Export as PNG Character Card">
              🖼️ Export PNG
            </button>
            <button class="export-btn" on:click={handleExportJson} title="Export as JSON Card">
              📄 Export JSON
            </button>
          {/if}
          <button class="close-btn" on:click={onClose}>✕</button>
        </div>
      </div>

      <!-- Tab Buttons -->
      <div class="tabs-row">
        <button
          class="tab-btn {activeTab === 'basic' ? 'active' : ''}"
          on:click={() => (activeTab = 'basic')}
        >
          Basic Details
        </button>
        <button
          class="tab-btn {activeTab === 'greetings' ? 'active' : ''}"
          on:click={() => (activeTab = 'greetings')}
        >
          Greetings & Dialogue ({1 + alternateGreetings.length})
        </button>
        <button
          class="tab-btn {activeTab === 'lorebook' ? 'active' : ''}"
          on:click={() => (activeTab = 'lorebook')}
        >
          📖 Lorebook / World Info {hasEmbeddedBook ? `(${embeddedBook.entries.length})` : ''}
        </button>
        <button
          class="tab-btn {activeTab === 'advanced' ? 'active' : ''}"
          on:click={() => (activeTab = 'advanced')}
        >
          Advanced & System
        </button>
      </div>

      <!-- Modal Body -->
      <div class="modal-body">
        <!-- Tab: Basic -->
        {#if activeTab === 'basic'}
          <!-- Avatar & Name Row -->
          <div class="avatar-and-name">
            <div class="avatar-col">
              <div class="avatar-preview">
                {#if draftAvatar}
                  <img src={draftAvatar} alt="Character Avatar" class="avatar-img" />
                {:else}
                  <div class="avatar-placeholder">
                    {(draftCard.data.name || 'CH').slice(0, 2).toUpperCase()}
                  </div>
                {/if}
              </div>
              <div class="avatar-btns">
                <button
                  class="btn-sm"
                  type="button"
                  on:click={() => fileInputEl && fileInputEl.click()}
                >
                  Upload
                </button>
                {#if draftAvatar}
                  <button class="btn-sm text-danger" type="button" on:click={removeAvatar}>
                    Remove
                  </button>
                {/if}
              </div>
            </div>

            <div class="name-col">
              <div class="form-group">
                <label for="char-name">Character Name *</label>
                <input
                  id="char-name"
                  type="text"
                  bind:value={draftCard.data.name}
                  placeholder="e.g. Seraphina, Viktor, Lyra"
                />
              </div>
              <div class="form-group">
                <label for="char-tags">Tags (comma separated)</label>
                <input
                  id="char-tags"
                  type="text"
                  bind:value={tagInputStr}
                  placeholder="Fantasy, Tavern, Mysterious, Sci-Fi"
                />
              </div>
            </div>
          </div>

          <!-- Description -->
          <div class="form-group">
            <label for="char-desc">
              Description (Physical appearance, background, lore &mdash; maps to <code>&#123;&#123;description&#125;&#125;</code>)
            </label>
            <textarea
              id="char-desc"
              rows="4"
              bind:value={draftCard.data.description}
              placeholder="Describe what the character looks like, their history, attire, and general vibe."
            ></textarea>
          </div>

          <!-- Personality -->
          <div class="form-group">
            <label for="char-pers">
              Personality (Traits, psychological profile, tone &mdash; maps to <code>&#123;&#123;personality&#125;&#125;</code>)
            </label>
            <textarea
              id="char-pers"
              rows="3"
              bind:value={draftCard.data.personality}
              placeholder="e.g. Playful, observant, sarcastic, deeply loyal, speaks with an archaic lilt."
            ></textarea>
          </div>

          <!-- Scenario -->
          <div class="form-group">
            <label for="char-scen">
              Scenario / Setting (Current circumstances &mdash; maps to <code>&#123;&#123;scenario&#125;&#125;</code>)
            </label>
            <textarea
              id="char-scen"
              rows="3"
              bind:value={draftCard.data.scenario}
              placeholder="e.g. &#123;&#123;user&#125;&#125; enters a secluded potion shop on a rainy midnight."
            ></textarea>
          </div>
        {/if}

        <!-- Tab: Greetings & Dialogue -->
        {#if activeTab === 'greetings'}
          <!-- First Message -->
          <div class="form-group">
            <label for="first-mes">
              First Message (Primary Greeting / Opening Turn)
            </label>
            <textarea
              id="first-mes"
              rows="5"
              bind:value={draftCard.data.first_mes}
              placeholder="*The bell rings as you step inside.* Welcome! What brings you here today?"
            ></textarea>
          </div>

          <!-- Alternate Greetings -->
          <div class="form-group">
            <div class="section-header-row">
              <span class="label-heading">Alternate Greetings ({alternateGreetings.length})</span>
              <button class="btn-sm btn-accent" type="button" on:click={addAlternateGreeting}>
                + Add Alternate Greeting
              </button>
            </div>

            {#if alternateGreetings.length === 0}
              <p class="hint-text">No alternate greetings added yet.</p>
            {:else}
              <div class="alt-greetings-list">
                {#each alternateGreetings as altGreeting, i}
                  <div class="alt-greeting-card">
                    <div class="alt-header">
                      <span>Alternate Greeting #{i + 1}</span>
                      <button
                        class="btn-sm text-danger"
                        type="button"
                        on:click={() => removeAlternateGreeting(i)}
                      >
                        Remove
                      </button>
                    </div>
                    <textarea
                      rows="3"
                      bind:value={alternateGreetings[i]}
                      placeholder="Alternate scenario or seasonal greeting..."
                    ></textarea>
                  </div>
                {/each}
              </div>
            {/if}
          </div>

          <!-- Example Dialogue -->
          <div class="form-group">
            <label for="mes-example">
              Example Dialogue (<code>&lt;START&gt;</code>, <code>&#123;&#123;user&#125;&#125;</code>, <code>&#123;&#123;char&#125;&#125;</code>)
            </label>
            <textarea
              id="mes-example"
              rows="6"
              bind:value={draftCard.data.mes_example}
              placeholder="&lt;START&gt;&#10;&#123;&#123;user&#125;&#125;: How did you learn magic?&#10;&#123;&#123;char&#125;&#125;: *smiles mysteriously* In the ancient woods, beneath the starlight."
            ></textarea>
          </div>
        {/if}

        <!-- Tab: Lorebook / World Info -->
        {#if activeTab === 'lorebook'}
          <div class="lorebook-tab-content">
            <!-- Section 1: Linked Standalone Lorebooks -->
            <div class="section-box">
              <div class="section-box-header">
                <span class="label-heading">🌐 Linked Global / Standalone Lorebooks</span>
                <span class="section-hint">Attach existing world info books to this character</span>
              </div>

              {#if availableLorebooks.length === 0}
                <p class="hint-text">No standalone lorebooks found. You can create them in the Lorebooks manager.</p>
              {:else}
                <div class="linked-lorebooks-grid">
                  {#each availableLorebooks as book}
                    <label class="linked-book-item {linkedLorebookIds.includes(book.id) ? 'checked' : ''}">
                      <input
                        type="checkbox"
                        checked={linkedLorebookIds.includes(book.id)}
                        on:change={() => toggleLinkedLorebook(book.id)}
                      />
                      <div class="linked-book-info">
                        <span class="linked-book-name">{book.name}</span>
                        <span class="linked-book-entries">{book.entries.length} entries</span>
                      </div>
                    </label>
                  {/each}
                </div>
              {/if}
            </div>

            <!-- Section 2: Character-Embedded Lorebook (Character Card V2) -->
            <div class="section-box">
              <div class="section-box-header">
                <div class="header-with-toggle">
                  <label class="toggle-container">
                    <input
                      type="checkbox"
                      bind:checked={hasEmbeddedBook}
                    />
                    <span class="toggle-label-text">Embedded Character Lorebook</span>
                  </label>
                  <span class="section-hint">Saved directly inside character card PNG / JSON</span>
                </div>
                {#if hasEmbeddedBook}
                  <button class="btn-sm btn-accent" type="button" on:click={handleAddEmbeddedEntry}>
                    + Add Entry
                  </button>
                {/if}
              </div>

              {#if hasEmbeddedBook}
                <div class="embedded-book-layout">
                  <!-- Entry List -->
                  <div class="embedded-entries-col">
                    {#if embeddedBook.entries.length === 0}
                      <div class="empty-entries">
                        <p>No embedded lorebook entries yet.</p>
                        <button class="btn-sm btn-accent" type="button" on:click={handleAddEmbeddedEntry}>
                          + Add Entry
                        </button>
                      </div>
                    {:else}
                      {#each embeddedBook.entries as entry (entry.id)}
                        <div
                          class="embedded-entry-pill {entry.id === selectedEmbeddedEntryId ? 'active' : ''}"
                          on:click={() => selectEmbeddedEntry(entry.id)}
                          on:keydown={(e) => e.key === 'Enter' && selectEmbeddedEntry(entry.id)}
                          role="button"
                          tabindex="0"
                        >
                          <span class="entry-title-text">{entry.comment || 'Untitled Entry'}</span>
                          <span class="entry-pos-tag">{entry.position}</span>
                          <button
                            class="del-entry-btn"
                            type="button"
                            on:click|stopPropagation={() => handleDeleteEmbeddedEntry(entry.id)}
                            title="Delete Entry"
                          >
                            ✕
                          </button>
                        </div>
                      {/each}
                    {/if}
                  </div>

                  <!-- Entry Editor -->
                  {#if selectedEmbeddedEntry}
                    <div class="embedded-entry-editor">
                      <div class="form-row-dual">
                        <div class="form-group">
                          <label for="embed-comment">Title / Comment</label>
                          <input
                            id="embed-comment"
                            type="text"
                            bind:value={selectedEmbeddedEntry.comment}
                            placeholder="e.g. Tavern History, Magic Sword"
                          />
                        </div>
                        <div class="form-group">
                          <label for="embed-position">Position</label>
                          <select id="embed-position" bind:value={selectedEmbeddedEntry.position}>
                            <option value="before_char">Before Character Definition</option>
                            <option value="after_char">After Character Definition</option>
                            <option value="before_scenario">Before Scenario</option>
                            <option value="after_scenario">After Scenario</option>
                            <option value="top_system">Top of System Prompt</option>
                            <option value="bottom_system">Bottom of System Prompt</option>
                            <option value="at_depth">In Chat History at Depth</option>
                          </select>
                        </div>
                      </div>

                      <div class="form-group">
                        <label for="embed-keys">Trigger Keywords (comma separated)</label>
                        <input
                          id="embed-keys"
                          type="text"
                          bind:value={embeddedKeyStr}
                          on:input={handleEmbeddedKeyChange}
                          placeholder="e.g. tavern, inn, drink"
                          disabled={selectedEmbeddedEntry.constant}
                        />
                      </div>

                      <div class="form-row-flags">
                        <label class="checkbox-label">
                          <input
                            type="checkbox"
                            bind:checked={selectedEmbeddedEntry.constant}
                          />
                          <span>Constant (Always Active)</span>
                        </label>
                        <label class="checkbox-label">
                          <input
                            type="checkbox"
                            bind:checked={selectedEmbeddedEntry.selective}
                            disabled={selectedEmbeddedEntry.constant}
                          />
                          <span>Selective Logic</span>
                        </label>
                      </div>

                      {#if selectedEmbeddedEntry.selective && !selectedEmbeddedEntry.constant}
                        <div class="form-group">
                          <label for="embed-sec-keys">Secondary Keywords</label>
                          <input
                            id="embed-sec-keys"
                            type="text"
                            bind:value={embeddedSecKeyStr}
                            on:input={handleEmbeddedSecKeyChange}
                            placeholder="e.g. dragon, fire"
                          />
                        </div>
                      {/if}

                      <div class="form-group">
                        <label for="embed-content">Content</label>
                        <textarea
                          id="embed-content"
                          rows="4"
                          bind:value={selectedEmbeddedEntry.content}
                          placeholder="Text to inject when triggered..."
                        ></textarea>
                      </div>
                    </div>
                  {/if}
                </div>
              {/if}
            </div>
          </div>
        {/if}

        <!-- Tab: Advanced & System -->
        {#if activeTab === 'advanced'}
          <!-- Custom System Prompt -->
          <div class="form-group">
            <label for="char-sys">
              Character System Prompt Override
            </label>
            <textarea
              id="char-sys"
              rows="4"
              bind:value={draftCard.data.system_prompt}
              placeholder="Custom instructions injected directly into the LLM system prompt for this character."
            ></textarea>
          </div>

          <!-- Post-history instructions -->
          <div class="form-group">
            <label for="char-post">
              Post-History Instructions (Jailbreak / Reminders / Formatting Rules)
            </label>
            <textarea
              id="char-post"
              rows="3"
              bind:value={draftCard.data.post_history_instructions}
              placeholder="Injected right at the end of the context before generation to reinforce formatting or tone."
            ></textarea>
          </div>

          <!-- Creator Notes -->
          <div class="form-group">
            <label for="char-notes">Creator Notes</label>
            <textarea
              id="char-notes"
              rows="3"
              bind:value={draftCard.data.creator_notes}
              placeholder="Notes for users or yourself about this character card."
            ></textarea>
          </div>
        {/if}
      </div>

      <!-- Footer -->
      <div class="modal-footer">
        <button class="btn-cancel" on:click={onClose}>Cancel</button>
        <button class="btn-save" on:click={save}>
          {draftId ? 'Save Changes' : 'Create Character'}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
    backdrop-filter: blur(4px);
  }

  .modal-card {
    background: #1e1e2e;
    border: 1px solid #45475a;
    border-radius: 16px;
    width: 92%;
    max-width: 820px;
    max-height: 92vh;
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

  .modal-header h2 {
    margin: 0;
    font-size: 1.25rem;
    color: #cdd6f4;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }

  .export-btn {
    background: #313244;
    color: #cdd6f4;
    border: 1px solid #45475a;
    border-radius: 6px;
    padding: 0.3rem 0.65rem;
    font-size: 0.78rem;
    font-weight: 600;
    cursor: pointer;
  }
  .export-btn:hover {
    background: #45475a;
  }

  .close-btn {
    background: none;
    border: none;
    color: #6c7086;
    font-size: 1.2rem;
    cursor: pointer;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
  }
  .close-btn:hover {
    color: #cdd6f4;
    background: #313244;
  }

  .tabs-row {
    display: flex;
    border-bottom: 1px solid #313244;
    background: #181825;
    overflow-x: auto;
  }

  .tab-btn {
    flex: 1;
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    padding: 0.75rem 1rem;
    color: #a6adc8;
    font-size: 0.88rem;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
    transition: all 0.15s ease;
  }
  .tab-btn:hover {
    color: #cdd6f4;
    background: rgba(255, 255, 255, 0.02);
  }
  .tab-btn.active {
    color: #89b4fa;
    border-bottom-color: #89b4fa;
    background: rgba(137, 180, 250, 0.05);
  }

  .modal-body {
    padding: 1.5rem;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 1.2rem;
  }

  .avatar-and-name {
    display: flex;
    gap: 1.5rem;
    align-items: center;
  }

  .avatar-col {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
  }

  .avatar-preview {
    width: 80px;
    height: 80px;
    border-radius: 50%;
    overflow: hidden;
    background: #313244;
    border: 2px solid #45475a;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .avatar-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .avatar-placeholder {
    font-size: 1.6rem;
    font-weight: 700;
    color: #89b4fa;
  }

  .avatar-btns {
    display: flex;
    gap: 0.4rem;
  }

  .name-col {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .form-group label {
    font-size: 0.85rem;
    font-weight: 600;
    color: #cdd6f4;
  }

  .form-group input,
  .form-group textarea,
  .form-group select {
    background: #181825;
    border: 1px solid #313244;
    border-radius: 8px;
    color: #cdd6f4;
    padding: 0.6rem 0.8rem;
    font-size: 0.88rem;
    font-family: inherit;
  }
  .form-group input:focus,
  .form-group textarea:focus,
  .form-group select:focus {
    outline: none;
    border-color: #89b4fa;
  }

  .form-row-dual {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.8rem;
  }

  .form-row-flags {
    display: flex;
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

  /* Lorebook Tab Styling */
  .lorebook-tab-content {
    display: flex;
    flex-direction: column;
    gap: 1.2rem;
  }

  .section-box {
    background: #181825;
    border: 1px solid #313244;
    border-radius: 10px;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
  }

  .section-box-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    border-bottom: 1px solid #313244;
    padding-bottom: 0.5rem;
  }

  .header-with-toggle {
    display: flex;
    align-items: center;
    gap: 0.8rem;
  }

  .toggle-container {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
    font-weight: 700;
    color: #cdd6f4;
  }
  .toggle-container input[type='checkbox'] {
    accent-color: #89b4fa;
    width: 16px;
    height: 16px;
  }

  .section-hint {
    font-size: 0.75rem;
    color: #6c7086;
  }

  .linked-lorebooks-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 0.5rem;
  }

  .linked-book-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.7rem;
    background: #1e1e2e;
    border: 1px solid #313244;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s ease;
  }
  .linked-book-item.checked {
    border-color: #89b4fa;
    background: rgba(137, 180, 250, 0.08);
  }

  .linked-book-info {
    display: flex;
    flex-direction: column;
  }
  .linked-book-name {
    font-size: 0.82rem;
    font-weight: 600;
    color: #cdd6f4;
  }
  .linked-book-entries {
    font-size: 0.7rem;
    color: #a6adc8;
  }

  .embedded-book-layout {
    display: grid;
    grid-template-columns: 200px 1fr;
    gap: 1rem;
    min-height: 250px;
  }

  .embedded-entries-col {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    overflow-y: auto;
    max-height: 300px;
    background: #1e1e2e;
    padding: 0.5rem;
    border-radius: 6px;
    border: 1px solid #313244;
  }

  .embedded-entry-pill {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.35rem 0.5rem;
    background: #181825;
    border: 1px solid transparent;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.78rem;
  }
  .embedded-entry-pill:hover {
    background: #313244;
  }
  .embedded-entry-pill.active {
    border-color: #89b4fa;
    background: #313244;
  }

  .entry-title-text {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: #cdd6f4;
  }
  .entry-pos-tag {
    font-size: 0.65rem;
    color: #6c7086;
    margin: 0 0.3rem;
  }
  .del-entry-btn {
    background: none;
    border: none;
    color: #6c7086;
    cursor: pointer;
    font-size: 0.75rem;
    padding: 0 0.2rem;
  }
  .del-entry-btn:hover {
    color: #f38ba8;
  }

  .embedded-entry-editor {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    background: #1e1e2e;
    padding: 0.8rem;
    border-radius: 6px;
    border: 1px solid #313244;
  }

  .section-header-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .label-heading {
    font-size: 0.88rem;
    font-weight: 700;
    color: #cdd6f4;
  }

  .hint-text {
    color: #6c7086;
    font-size: 0.82rem;
    font-style: italic;
    margin: 0.4rem 0;
  }

  .alt-greetings-list {
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
    margin-top: 0.4rem;
  }

  .alt-greeting-card {
    background: #181825;
    border: 1px solid #313244;
    border-radius: 8px;
    padding: 0.8rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .alt-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 0.78rem;
    color: #a6adc8;
  }

  .btn-sm {
    background: #313244;
    border: 1px solid #45475a;
    border-radius: 6px;
    color: #cdd6f4;
    padding: 0.25rem 0.5rem;
    font-size: 0.78rem;
    cursor: pointer;
    font-weight: 600;
  }
  .btn-sm:hover {
    background: #45475a;
  }
  .btn-accent {
    background: #89b4fa;
    color: #11111b;
    border: none;
  }
  .btn-accent:hover {
    background: #b4befe;
  }
  .text-danger {
    color: #f38ba8;
  }

  .modal-footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0.8rem;
    padding: 1rem 1.5rem;
    border-top: 1px solid #313244;
    background: #181825;
  }

  .btn-cancel {
    background: #313244;
    border: none;
    border-radius: 8px;
    color: #cdd6f4;
    padding: 0.6rem 1.2rem;
    font-size: 0.88rem;
    font-weight: 600;
    cursor: pointer;
  }
  .btn-cancel:hover {
    background: #45475a;
  }

  .btn-save {
    background: #89b4fa;
    border: none;
    border-radius: 8px;
    color: #11111b;
    padding: 0.6rem 1.5rem;
    font-size: 0.88rem;
    font-weight: 700;
    cursor: pointer;
  }
  .btn-save:hover {
    background: #b4befe;
  }
</style>
