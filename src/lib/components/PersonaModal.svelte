<script lang="ts">
  import type { UserPersona } from '../types';
  import {
    getAllUserPersonas,
    saveUserPersona,
    deleteUserPersona,
    setActiveUserPersona,
  } from '../api';

  export let isOpen = false;
  export let activePersona: UserPersona;
  export let onPersonaChanged: (updated: UserPersona) => void;
  export let onClose: () => void;

  let personas: UserPersona[] = [];
  let selectedPersonaId: string = '';
  let draft: UserPersona;
  let fileInputEl: HTMLInputElement;
  let isSaving = false;
  let wasOpen = false;
  $: if (isOpen && !wasOpen) {
    wasOpen = true;
    loadPersonas();
  } else if (!isOpen) {
    wasOpen = false;
  }
  async function loadPersonas() {
    try {
      personas = await getAllUserPersonas();
      if (!selectedPersonaId || !personas.some((p) => p.id === selectedPersonaId)) {
        selectedPersonaId = activePersona?.id || personas[0]?.id || 'default_user';
      }
      const current = personas.find((p) => p.id === selectedPersonaId) || personas[0];
      if (current) {
        draft = JSON.parse(JSON.stringify(current));
      }
    } catch (e) {
      console.error('Failed to load personas:', e);
    }
  }

  function selectPersona(p: UserPersona) {
    selectedPersonaId = p.id;
    draft = JSON.parse(JSON.stringify(p));
  }

  function createNewPersona() {
    const newId = crypto.randomUUID();
    const newP: UserPersona = {
      id: newId,
      name: `Persona ${personas.length + 1}`,
      description: '',
      avatar_data_url: null,
    };
    personas = [...personas, newP];
    selectedPersonaId = newId;
    draft = JSON.parse(JSON.stringify(newP));
  }

  function handleAvatarUpload(e: Event) {
    const target = e.target as HTMLInputElement;
    if (target.files && target.files[0]) {
      const file = target.files[0];
      const reader = new FileReader();
      reader.onload = (event) => {
        if (event.target?.result) {
          draft.avatar_data_url = event.target.result as string;
        }
      };
      reader.readAsDataURL(file);
      target.value = '';
    }
  }

  function removeAvatar() {
    draft.avatar_data_url = null;
  }

  async function handleSetActive(id: string) {
    try {
      const activated = await setActiveUserPersona(id);
      activePersona = activated;
      onPersonaChanged(activated);
    } catch (e) {
      console.error('Failed to set active persona:', e);
    }
  }

  async function handleDeletePersona(id: string) {
    if (personas.length <= 1) {
      alert('You must keep at least one persona.');
      return;
    }
    if (!confirm('Delete this user persona?')) return;

    try {
      await deleteUserPersona(id);
      personas = personas.filter((p) => p.id !== id);
      selectedPersonaId = personas[0].id;
      draft = JSON.parse(JSON.stringify(personas[0]));
      if (activePersona.id === id) {
        activePersona = personas[0];
        onPersonaChanged(personas[0]);
      }
    } catch (e) {
      alert(`Delete failed: ${e}`);
    }
  }

  async function save() {
    if (!draft.name.trim()) draft.name = 'User';
    isSaving = true;
    try {
      const saved = await saveUserPersona(draft);
      const idx = personas.findIndex((p) => p.id === saved.id);
      if (idx !== -1) {
        personas[idx] = saved;
      } else {
        personas.push(saved);
      }
      if (activePersona.id === saved.id || personas.length === 1) {
        activePersona = saved;
        onPersonaChanged(saved);
      }
      onClose();
    } catch (e) {
      alert(`Save failed: ${e}`);
    } finally {
      isSaving = false;
    }
  }
</script>

{#if isOpen && draft}
  <input
    type="file"
    accept="image/png,image/jpeg,image/webp"
    bind:this={fileInputEl}
    on:change={handleAvatarUpload}
    style="display: none;"
  />

  <div
    class="modal-backdrop"
    on:click={onClose}
    on:keydown={(e) => e.key === 'Escape' && onClose()}
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
        <div class="header-title-row">
          <h2>👤 User Personas</h2>
          <span class="header-sub">Manage and switch your roleplay identities</span>
        </div>
        <button class="close-btn" on:click={onClose}>✕</button>
      </div>

      <div class="modal-body-layout">
        <!-- Left: Persona List -->
        <div class="persona-sidebar">
          <div class="persona-sidebar-header">
            <span class="list-label">Your Personas</span>
            <button class="new-persona-btn" on:click={createNewPersona}>+ New</button>
          </div>

          <div class="persona-list">
            {#each personas as p}
              <div
                class="persona-item {p.id === selectedPersonaId ? 'selected' : ''}"
                on:click={() => selectPersona(p)}
                role="button"
                tabindex="0"
                on:keydown={(e) => e.key === 'Enter' && selectPersona(p)}
              >
                {#if p.avatar_data_url}
                  <img src={p.avatar_data_url} alt={p.name} class="p-item-avatar" />
                {:else}
                  <div class="p-item-avatar-placeholder">
                    {p.name.slice(0, 2).toUpperCase()}
                  </div>
                {/if}

                <div class="p-item-info">
                  <span class="p-item-name">{p.name}</span>
                  {#if p.id === activePersona?.id}
                    <span class="active-badge">Active</span>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        </div>

        <!-- Right: Active Persona Form -->
        <div class="persona-editor">
          <!-- Active Status Bar -->
          <div class="active-status-bar">
            {#if draft.id === activePersona?.id}
              <span class="current-active-label">✓ Currently Active Persona</span>
            {:else}
              <button
                class="set-active-btn"
                type="button"
                on:click={() => handleSetActive(draft.id)}
              >
                ★ Set as Active Persona
              </button>
            {/if}

            {#if personas.length > 1}
              <button
                class="btn-delete-persona"
                type="button"
                on:click={() => handleDeletePersona(draft.id)}
                title="Delete this persona"
              >
                🗑️ Delete
              </button>
            {/if}
          </div>

          <!-- Avatar Upload -->
          <div class="avatar-section">
            <div class="avatar-preview-wrap">
              {#if draft.avatar_data_url}
                <img src={draft.avatar_data_url} alt="User Avatar" class="avatar-img" />
              {:else}
                <div class="avatar-placeholder">
                  {(draft.name || 'US').slice(0, 2).toUpperCase()}
                </div>
              {/if}
            </div>
            <div class="avatar-buttons">
              <button
                class="btn-upload"
                type="button"
                on:click={() => fileInputEl && fileInputEl.click()}
              >
                Upload Avatar
              </button>
              {#if draft.avatar_data_url}
                <button class="btn-remove-avatar" type="button" on:click={removeAvatar}>
                  Remove
                </button>
              {/if}
            </div>
          </div>

          <!-- Name -->
          <div class="form-group">
            <label for="username">Persona Name (substituted for <code>&#123;&#123;user&#125;&#125;</code>)</label>
            <input
              id="username"
              type="text"
              bind:value={draft.name}
              placeholder="e.g. Master, Detective, Wanderer, Alex"
            />
          </div>

          <!-- Description -->
          <div class="form-group">
            <label for="userdesc">
              Persona Description (substituted for <code>&#123;&#123;persona&#125;&#125;</code>)
            </label>
            <textarea
              id="userdesc"
              rows="5"
              bind:value={draft.description}
              placeholder="Describe your role, backstory, attire, appearance, or demeanor. The LLM will incorporate this into &#123;&#123;persona&#125;&#125; and roleplay context."
            ></textarea>
          </div>
        </div>
      </div>

      <div class="modal-footer">
        <button class="btn-cancel" on:click={onClose}>Cancel</button>
        <button class="btn-save" on:click={save} disabled={isSaving}>
          {isSaving ? 'Saving...' : 'Save Persona'}
        </button>
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
    z-index: 120;
    backdrop-filter: blur(4px);
  }

  .modal-card {
    background: #1e1e2e;
    border: 1px solid #45475a;
    border-radius: 16px;
    width: 92%;
    max-width: 760px;
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

  .header-title-row {
    display: flex;
    flex-direction: column;
  }

  .modal-header h2 {
    margin: 0;
    font-size: 1.2rem;
    color: #cdd6f4;
  }

  .header-sub {
    font-size: 0.75rem;
    color: #a6adc8;
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
  .modal-body-layout {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .persona-sidebar {
    width: 220px;
    background: #181825;
    border-right: 1px solid #313244;
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
  }

  .persona-sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.8rem 1rem;
    border-bottom: 1px solid #313244;
  }

  .list-label {
    font-size: 0.8rem;
    font-weight: 700;
    color: #a6adc8;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .new-persona-btn {
    background: #cba6f7;
    color: #11111b;
    border: none;
    border-radius: 4px;
    padding: 0.2rem 0.5rem;
    font-size: 0.75rem;
    font-weight: 700;
    cursor: pointer;
  }

  .persona-list {
    flex: 1;
    overflow-y: auto;
    padding: 0.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .persona-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.6rem;
    border-radius: 8px;
    background: #1e1e2e;
    border: 1px solid #313244;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .persona-item:hover {
    background: #2b2d42;
  }

  .persona-item.selected {
    background: rgba(137, 180, 250, 0.15);
    border-color: #89b4fa;
  }

  .p-item-avatar {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    object-fit: cover;
  }

  .p-item-avatar-placeholder {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: linear-gradient(135deg, #89b4fa, #74c7ec);
    color: #11111b;
    font-weight: 700;
    font-size: 0.75rem;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .p-item-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .p-item-name {
    font-size: 0.85rem;
    font-weight: 600;
    color: #cdd6f4;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .active-badge {
    font-size: 0.65rem;
    color: #a6e3a1;
    font-weight: 700;
  }

  .persona-editor {
    flex: 1;
    padding: 1.2rem 1.5rem;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .active-status-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.5rem 0.8rem;
    background: #181825;
    border: 1px solid #313244;
    border-radius: 8px;
  }

  .current-active-label {
    color: #a6e3a1;
    font-weight: 700;
    font-size: 0.82rem;
  }

  .set-active-btn {
    background: rgba(166, 227, 161, 0.15);
    border: 1px solid #a6e3a1;
    color: #a6e3a1;
    border-radius: 6px;
    padding: 0.3rem 0.7rem;
    font-size: 0.78rem;
    font-weight: 700;
    cursor: pointer;
  }

  .btn-delete-persona {
    background: transparent;
    color: #f38ba8;
    border: 1px solid #f38ba8;
    border-radius: 6px;
    padding: 0.25rem 0.6rem;
    font-size: 0.75rem;
    cursor: pointer;
  }

  .avatar-section {
    display: flex;
    align-items: center;
    gap: 1.2rem;
  }

  .avatar-preview-wrap {
    width: 64px;
    height: 64px;
    border-radius: 50%;
    overflow: hidden;
    flex-shrink: 0;
    border: 2px solid #89b4fa;
  }

  .avatar-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .avatar-placeholder {
    width: 100%;
    height: 100%;
    background: linear-gradient(135deg, #89b4fa, #74c7ec);
    color: #11111b;
    font-weight: 700;
    font-size: 1.2rem;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .avatar-buttons {
    display: flex;
    gap: 0.5rem;
  }

  .btn-upload {
    background: #313244;
    color: #cdd6f4;
    border: 1px solid #45475a;
    padding: 0.4rem 0.8rem;
    border-radius: 6px;
    font-size: 0.82rem;
    cursor: pointer;
  }

  .btn-remove-avatar {
    background: transparent;
    color: #f38ba8;
    border: 1px solid #f38ba8;
    padding: 0.4rem 0.8rem;
    border-radius: 6px;
    font-size: 0.82rem;
    cursor: pointer;
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
    border-color: #89b4fa;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.8rem;
    padding: 1rem 1.5rem;
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
    background: #89b4fa;
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

    .modal-body-layout {
      flex-direction: column;
    }

    .persona-sidebar {
      width: 100%;
      max-height: 140px;
      border-right: none;
      border-bottom: 1px solid #313244;
    }

    .persona-list {
      flex-direction: row;
      overflow-x: auto;
    }

    .persona-item {
      min-width: 120px;
    }

    .persona-editor {
      padding: 0.8rem;
    }

    .modal-footer {
      padding: 0.8rem 1rem;
    }
  }
</style>
