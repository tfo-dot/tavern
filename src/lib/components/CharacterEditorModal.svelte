<script lang="ts">
  import type { Character, CharacterCardV2 } from '../types';
  import { exportCardJson, exportCardPng } from '../api';

  export let character: Character | null = null;
  export let isOpen = false;
  export let onSave: (saved: Character) => void;
  export let onClose: () => void;

  let activeTab: 'basic' | 'greetings' | 'advanced' = 'basic';
  let fileInputEl: HTMLInputElement;
  let tagInputStr = '';

  let draftCard: CharacterCardV2;
  let draftAvatar: string | null = null;
  let draftId: string | null = null;
  let alternateGreetings: string[] = [];

  let wasOpen = false;
  $: if (isOpen && !wasOpen) {
    wasOpen = true;
    if (character) {
      draftId = character.id;
      draftCard = JSON.parse(JSON.stringify(character.card));
      draftAvatar = character.avatar_data_url;
      alternateGreetings = [...(character.card.data.alternate_greetings || [])];
      tagInputStr = (character.card.data.tags || []).join(', ');
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
        },
      };
      draftAvatar = null;
      alternateGreetings = [];
      tagInputStr = '';
    }
  } else if (!isOpen) {
    wasOpen = false;
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
    max-width: 780px;
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
    background: transparent;
    border: none;
    color: #a6adc8;
    font-size: 1.4rem;
    cursor: pointer;
    padding: 0.4rem 0.7rem;
    border-radius: 8px;
  }

  .tabs-row {
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
    background: #1e1e2e;
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
    gap: 1.2rem;
    align-items: flex-start;
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
    border: 2px solid #cba6f7;
  }

  .avatar-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .avatar-placeholder {
    width: 100%;
    height: 100%;
    background: linear-gradient(135deg, #cba6f7, #89b4fa);
    color: #11111b;
    font-weight: 700;
    font-size: 1.5rem;
    display: flex;
    align-items: center;
    justify-content: center;
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

  label {
    font-size: 0.85rem;
    font-weight: 600;
    color: #a6adc8;
  }

  input[type='text'],
  textarea {
    background: #181825;
    border: 1px solid #45475a;
    border-radius: 8px;
    padding: 0.6rem 0.8rem;
    color: #cdd6f4;
    font-family: inherit;
    font-size: 0.9rem;
    outline: none;
  }

  input:focus,
  textarea:focus {
    border-color: #cba6f7;
  }

  .section-header-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .btn-sm {
    background: #313244;
    color: #cdd6f4;
    border: 1px solid #45475a;
    border-radius: 6px;
    padding: 0.25rem 0.6rem;
    font-size: 0.78rem;
    font-weight: 600;
    cursor: pointer;
  }

  .btn-sm:hover {
    background: #45475a;
  }

  .btn-accent {
    background: #cba6f7;
    color: #11111b;
    border: none;
  }

  .btn-accent:hover {
    background: #f5c2e7;
  }

  .text-danger {
    color: #f38ba8;
    border-color: #f38ba8;
  }

  .hint-text {
    font-size: 0.82rem;
    color: #6c7086;
    margin: 0.2rem 0;
  }

  .alt-greetings-list {
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
    margin-top: 0.3rem;
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
    justify-content: space-between;
    align-items: center;
    font-size: 0.8rem;
    font-weight: 600;
    color: #cba6f7;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.8rem;
    padding: 1.2rem 1.5rem;
    border-top: 1px solid #313244;
  }

  .btn-cancel {
    background: #313244;
    color: #cdd6f4;
    border: none;
    padding: 0.55rem 1.2rem;
    border-radius: 8px;
    cursor: pointer;
    font-weight: 600;
  }

  .btn-save {
    background: #a6e3a1;
    color: #11111b;
    border: none;
    padding: 0.55rem 1.4rem;
    border-radius: 8px;
    cursor: pointer;
    font-weight: 700;
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

    .header-actions {
      gap: 0.3rem;
    }

    .export-btn {
      padding: 0.25rem 0.45rem;
      font-size: 0.72rem;
    }

    .tabs-row {
      overflow-x: auto;
    }

    .tab-btn {
      padding: 0.55rem 0.4rem;
      font-size: 0.78rem;
      white-space: nowrap;
    }

    .modal-body {
      padding: 1rem 0.8rem;
      gap: 1rem;
    }

    .avatar-and-name {
      flex-direction: column;
      align-items: center;
    }

    .name-col {
      width: 100%;
    }

    .modal-footer {
      padding: 0.8rem 1rem;
    }
  }
</style>
