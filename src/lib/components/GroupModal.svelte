<script lang="ts">
  import type { Character, Group, GroupMember, TurnMode } from '../types';

  export let group: Group | null = null;
  export let characters: Character[] = [];
  export let isOpen = false;
  export let onSave: (group: Group, startChat?: boolean) => void;
  export let onDelete: ((groupId: string) => void) | undefined = undefined;
  export let onClose: () => void;

  let draftId: string | null = null;
  let draftName = '';
  let draftDescription = '';
  let draftAvatar: string | null = null;
  let draftTurnMode: TurnMode = 'Natural';
  let draftAllowSelfResponses = false;
  let draftAutoMode = true;
  let draftMembers: GroupMember[] = [];
  let memberSearch = '';
  let fileInputEl: HTMLInputElement;

  let wasOpen = false;
  $: if (isOpen && !wasOpen) {
    wasOpen = true;
    initDraft();
  } else if (!isOpen) {
    wasOpen = false;
  }

  function initDraft() {
    if (group) {
      draftId = group.id;
      draftName = group.name;
      draftDescription = group.description || '';
      draftAvatar = group.avatar_data_url || null;
      draftTurnMode = group.turn_mode || 'Natural';
      draftAllowSelfResponses = group.allow_self_responses ?? false;
      draftAutoMode = group.auto_mode ?? true;
      draftMembers = group.members ? JSON.parse(JSON.stringify(group.members)) : [];
    } else {
      draftId = null;
      draftName = '';
      draftDescription = '';
      draftAvatar = null;
      draftTurnMode = 'Natural';
      draftAllowSelfResponses = false;
      draftAutoMode = true;
      draftMembers = [];
    }
  }

  function getCharacter(id: string): Character | undefined {
    return characters.find((c) => c.id === id);
  }

  function isMemberSelected(id: string): boolean {
    return draftMembers.some((m) => m.character_id === id);
  }

  function toggleMember(id: string) {
    const idx = draftMembers.findIndex((m) => m.character_id === id);
    if (idx >= 0) {
      draftMembers = draftMembers.filter((m) => m.character_id !== id);
    } else {
      draftMembers = [...draftMembers, { character_id: id, enabled: true, mute: false }];
    }
  }

  function toggleEnabled(id: string) {
    draftMembers = draftMembers.map((m) =>
      m.character_id === id ? { ...m, enabled: !m.enabled } : m
    );
  }

  function toggleMute(id: string) {
    draftMembers = draftMembers.map((m) =>
      m.character_id === id ? { ...m, mute: !m.mute } : m
    );
  }

  function moveMember(index: number, direction: 'up' | 'down') {
    const newIdx = direction === 'up' ? index - 1 : index + 1;
    if (newIdx < 0 || newIdx >= draftMembers.length) return;
    const updated = [...draftMembers];
    const [moved] = updated.splice(index, 1);
    updated.splice(newIdx, 0, moved);
    draftMembers = updated;
  }

  function handleAvatarChange(e: Event) {
    const target = e.target as HTMLInputElement;
    if (target.files && target.files[0]) {
      const reader = new FileReader();
      reader.onload = (ev) => {
        draftAvatar = ev.target?.result as string;
      };
      reader.readAsDataURL(target.files[0]);
    }
  }

  function handleSave(startChat = false) {
    if (!draftName.trim()) return;
    const now = new Date().toISOString();
    const finalGroup: Group = {
      id: draftId || crypto.randomUUID(),
      name: draftName.trim(),
      description: draftDescription.trim(),
      avatar_data_url: draftAvatar,
      members: draftMembers,
      turn_mode: draftTurnMode,
      allow_self_responses: draftAllowSelfResponses,
      auto_mode: draftAutoMode,
      created_at: group?.created_at || now,
      updated_at: now,
    };
    onSave(finalGroup, startChat);
    onClose();
  }

  function handleDelete() {
    if (draftId && onDelete && confirm(`Delete group "${draftName}" and its chats?`)) {
      onDelete(draftId);
      onClose();
    }
  }

  $: filteredAvailableChars = characters.filter((c) => {
    if (!memberSearch.trim()) return true;
    const q = memberSearch.toLowerCase();
    return (
      c.card.data.name.toLowerCase().includes(q) ||
      c.card.data.description?.toLowerCase().includes(q) ||
      c.card.data.tags?.some((t) => t.toLowerCase().includes(q))
    );
  });
</script>

{#if isOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="modal-backdrop" on:click={onClose} role="presentation">
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div
      class="modal-card"
      on:click|stopPropagation
      role="dialog"
      tabindex="-1"
      aria-modal="true"
      aria-labelledby="modal-title"
    >
      <header class="modal-header">
        <h2 id="modal-title" class="title">
          {group ? 'Edit Roleplay Group' : 'Create Roleplay Group'}
        </h2>
        <button class="close-btn" on:click={onClose} title="Close modal">✕</button>
      </header>

      <div class="modal-body">
        <!-- Top row: Avatar + Name + Description -->
        <div class="top-row">
          <div class="avatar-col">
            <input
              type="file"
              accept="image/*"
              bind:this={fileInputEl}
              on:change={handleAvatarChange}
              style="display: none;"
            />
            <button
              type="button"
              class="avatar-preview-btn"
              on:click={() => fileInputEl?.click()}
              title="Upload group avatar"
            >
              {#if draftAvatar}
                <img src={draftAvatar} alt="Group avatar" class="avatar-img" />
              {:else}
                <div class="avatar-placeholder">
                  <span class="icon">👥</span>
                  <span class="label">Avatar</span>
                </div>
              {/if}
            </button>
            {#if draftAvatar}
              <button
                type="button"
                class="remove-avatar-btn"
                on:click={() => (draftAvatar = null)}
              >
                Clear
              </button>
            {/if}
          </div>

          <div class="fields-col">
            <div class="form-group">
              <label for="group-name">Group Name <span class="req">*</span></label>
              <input
                id="group-name"
                type="text"
                bind:value={draftName}
                placeholder="e.g. The Adventuring Party, Council of Mages..."
                class="input-field"
                required
              />
            </div>

            <div class="form-group">
              <label for="group-desc">Group Description / Scenario Context</label>
              <textarea
                id="group-desc"
                bind:value={draftDescription}
                placeholder="Describe the group dynamic, setting, or purpose..."
                rows="2"
                class="input-field textarea-field"
              ></textarea>
            </div>
          </div>
        </div>

        <!-- Turn Management Configuration -->
        <div class="section-box">
          <h3 class="section-title">Turn Management & Ordering</h3>
          <p class="section-desc">
            Define how characters take turns in roleplay responses.
          </p>

          <div class="turn-modes-grid">
            <label class="turn-mode-card {draftTurnMode === 'Natural' ? 'selected' : ''}">
              <input
                type="radio"
                name="turn-mode"
                value="Natural"
                bind:group={draftTurnMode}
              />
              <div class="card-content">
                <div class="mode-header">
                  <span class="mode-icon">🔄</span>
                  <strong>Natural / Sequential</strong>
                </div>
                <p class="mode-desc">
                  Characters respond in order round-robin (Char 1 → Char 2 → Char 3).
                </p>
              </div>
            </label>

            <label class="turn-mode-card {draftTurnMode === 'Manual' ? 'selected' : ''}">
              <input
                type="radio"
                name="turn-mode"
                value="Manual"
                bind:group={draftTurnMode}
              />
              <div class="card-content">
                <div class="mode-header">
                  <span class="mode-icon">🎯</span>
                  <strong>Manual Selection</strong>
                </div>
                <p class="mode-desc">
                  You trigger specific characters on demand via the Turn Bar or message buttons.
                </p>
              </div>
            </label>

            <label class="turn-mode-card {draftTurnMode === 'Random' ? 'selected' : ''}">
              <input
                type="radio"
                name="turn-mode"
                value="Random"
                bind:group={draftTurnMode}
              />
              <div class="card-content">
                <div class="mode-header">
                  <span class="mode-icon">🎲</span>
                  <strong>Random</strong>
                </div>
                <p class="mode-desc">
                  Randomly selects an active member of the group for each turn.
                </p>
              </div>
            </label>
          </div>

          <div class="toggle-options-row">
            <label class="toggle-label">
              <input type="checkbox" bind:checked={draftAutoMode} />
              <span>
                <strong>Auto-generate Turn</strong> — Trigger AI response automatically after sending a message
              </span>
            </label>
            <label class="toggle-label">
              <input type="checkbox" bind:checked={draftAllowSelfResponses} />
              <span>
                <strong>Allow Consecutive Turns</strong> — Permit the same character to respond twice in a row
              </span>
            </label>
          </div>
        </div>

        <!-- Group Members Selection -->
        <div class="section-box">
          <div class="members-header">
            <div>
              <h3 class="section-title">
                Group Members ({draftMembers.length} selected)
              </h3>
              <p class="section-desc">
                Select and order characters who participate in this group chat.
              </p>
            </div>
          </div>

          <!-- Selected Members Order List -->
          {#if draftMembers.length > 0}
            <div class="selected-members-list">
              <span class="list-subtitle">Turn Sequence Order:</span>
              {#each draftMembers as member, idx}
                {@const char = getCharacter(member.character_id)}
                <div class="member-item-row {!member.enabled ? 'disabled' : ''} {member.mute ? 'muted' : ''}">
                  <span class="order-num">#{idx + 1}</span>

                  <div class="member-avatar">
                    {#if char?.avatar_data_url}
                      <img src={char.avatar_data_url} alt={char.card.data.name} />
                    {:else}
                      <span class="char-init">{(char?.card.data.name || '?').slice(0, 2).toUpperCase()}</span>
                    {/if}
                  </div>

                  <div class="member-info">
                    <span class="member-name">{char?.card.data.name || 'Unknown Character'}</span>
                    {#if char?.card.data.personality}
                      <span class="member-snippet">{char.card.data.personality}</span>
                    {/if}
                  </div>

                  <div class="member-controls">
                    <button
                      type="button"
                      class="pill-toggle {member.enabled ? 'active' : ''}"
                      on:click={() => toggleEnabled(member.character_id)}
                      title={member.enabled ? 'Enabled in turns' : 'Disabled / Inactive'}
                    >
                      {member.enabled ? 'Active' : 'Inactive'}
                    </button>

                    <button
                      type="button"
                      class="pill-toggle mute-toggle {member.mute ? 'active' : ''}"
                      on:click={() => toggleMute(member.character_id)}
                      title={member.mute ? 'Muted (Listening only)' : 'Can speak in auto turns'}
                    >
                      {member.mute ? '🔇 Muted' : '🔊 Speaking'}
                    </button>

                    <button
                      type="button"
                      class="reorder-btn"
                      disabled={idx === 0}
                      on:click={() => moveMember(idx, 'up')}
                      title="Move up in turn order"
                    >
                      ▲
                    </button>
                    <button
                      type="button"
                      class="reorder-btn"
                      disabled={idx === draftMembers.length - 1}
                      on:click={() => moveMember(idx, 'down')}
                      title="Move down in turn order"
                    >
                      ▼
                    </button>

                    <button
                      type="button"
                      class="remove-member-btn"
                      on:click={() => toggleMember(member.character_id)}
                      title="Remove from group"
                    >
                      ✕
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          {:else}
            <div class="empty-members-hint">
              <span class="hint-icon">⚠️</span> No members selected yet. Click characters below to add them to this group.
            </div>
          {/if}

          <!-- Available Characters Selector -->
          <div class="available-chars-section">
            <div class="available-header">
              <span class="list-subtitle">Add Characters:</span>
              <input
                type="text"
                placeholder="Filter characters..."
                bind:value={memberSearch}
                class="filter-input"
              />
            </div>

            <div class="chars-picker-grid">
              {#each filteredAvailableChars as c}
                {@const isSelected = isMemberSelected(c.id)}
                <button
                  type="button"
                  class="char-pick-card {isSelected ? 'selected' : ''}"
                  on:click={() => toggleMember(c.id)}
                >
                  <div class="char-pick-avatar">
                    {#if c.avatar_data_url}
                      <img src={c.avatar_data_url} alt={c.card.data.name} />
                    {:else}
                      <span>{c.card.data.name.slice(0, 2).toUpperCase()}</span>
                    {/if}
                  </div>
                  <div class="char-pick-meta">
                    <span class="char-pick-name">{c.card.data.name}</span>
                    <span class="char-pick-status">
                      {isSelected ? '✓ Added' : '+ Add'}
                    </span>
                  </div>
                </button>
              {/each}
            </div>
          </div>
        </div>
      </div>

      <footer class="modal-footer">
        {#if draftId && onDelete}
          <button type="button" class="danger-btn" on:click={handleDelete}>
            Delete Group
          </button>
        {/if}
        <div class="right-actions">
          <button type="button" class="cancel-btn" on:click={onClose}>
            Cancel
          </button>
          <button
            type="button"
            class="secondary-btn"
            disabled={!draftName.trim() || draftMembers.length === 0}
            on:click={() => handleSave(false)}
          >
            Save Group
          </button>
          <button
            type="button"
            class="primary-btn"
            disabled={!draftName.trim() || draftMembers.length === 0}
            on:click={() => handleSave(true)}
          >
            Save & Start Chat ➔
          </button>
        </div>
      </footer>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(17, 17, 27, 0.75);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1100;
    padding: 1rem;
  }

  .modal-card {
    background: #1e1e2e;
    border: 1px solid #313244;
    border-radius: 12px;
    width: 100%;
    max-width: 800px;
    max-height: 90vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 16px 36px rgba(0, 0, 0, 0.4);
    animation: modalPop 0.15s cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes modalPop {
    from {
      transform: scale(0.96);
      opacity: 0;
    }
    to {
      transform: scale(1);
      opacity: 1;
    }
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1.2rem 1.5rem;
    border-bottom: 1px solid #313244;
  }

  .title {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 700;
    color: #cdd6f4;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: #a6adc8;
    font-size: 1.2rem;
    cursor: pointer;
    padding: 0.2rem 0.5rem;
    border-radius: 6px;
    transition: all 0.15s;
  }

  .close-btn:hover {
    background: #313244;
    color: #cdd6f4;
  }

  .modal-body {
    padding: 1.5rem;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 1.2rem;
  }

  .top-row {
    display: flex;
    gap: 1.2rem;
    align-items: flex-start;
  }

  .avatar-col {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.4rem;
  }

  .avatar-preview-btn {
    width: 90px;
    height: 90px;
    border-radius: 12px;
    border: 2px dashed #45475a;
    background: #181825;
    padding: 0;
    cursor: pointer;
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s;
  }

  .avatar-preview-btn:hover {
    border-color: #cba6f7;
  }

  .avatar-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .avatar-placeholder {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.2rem;
  }

  .avatar-placeholder .icon {
    font-size: 1.8rem;
  }

  .avatar-placeholder .label {
    font-size: 0.75rem;
    color: #a6adc8;
  }

  .remove-avatar-btn {
    background: transparent;
    border: none;
    color: #f38ba8;
    font-size: 0.75rem;
    cursor: pointer;
  }

  .remove-avatar-btn:hover {
    text-decoration: underline;
  }

  .fields-col {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .form-group label {
    font-size: 0.85rem;
    font-weight: 600;
    color: #cdd6f4;
  }

  .req {
    color: #f38ba8;
  }

  .input-field {
    background: #181825;
    border: 1px solid #313244;
    border-radius: 8px;
    color: #cdd6f4;
    padding: 0.6rem 0.8rem;
    font-size: 0.9rem;
    outline: none;
    transition: border-color 0.15s;
  }

  .input-field:focus {
    border-color: #cba6f7;
  }

  .textarea-field {
    resize: vertical;
    font-family: inherit;
  }

  .section-box {
    background: #181825;
    border: 1px solid #313244;
    border-radius: 10px;
    padding: 1.2rem;
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
  }

  .section-title {
    margin: 0;
    font-size: 1rem;
    font-weight: 700;
    color: #cdd6f4;
  }

  .section-desc {
    margin: 0;
    font-size: 0.82rem;
    color: #a6adc8;
  }

  .turn-modes-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 0.8rem;
    margin-top: 0.4rem;
  }

  .turn-mode-card {
    border: 1px solid #313244;
    border-radius: 8px;
    background: #1e1e2e;
    padding: 0.8rem;
    cursor: pointer;
    display: flex;
    align-items: flex-start;
    gap: 0.6rem;
    transition: all 0.15s;
  }

  .turn-mode-card:hover {
    border-color: #585b70;
  }

  .turn-mode-card.selected {
    border-color: #cba6f7;
    background: rgba(203, 166, 247, 0.08);
  }

  .turn-mode-card input {
    margin-top: 0.2rem;
    accent-color: #cba6f7;
  }

  .card-content {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .mode-header {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.9rem;
    color: #cdd6f4;
  }

  .mode-desc {
    margin: 0;
    font-size: 0.78rem;
    color: #a6adc8;
    line-height: 1.3;
  }

  .toggle-options-row {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding-top: 0.4rem;
    border-top: 1px solid #313244;
  }

  .toggle-label {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    font-size: 0.85rem;
    color: #cdd6f4;
    cursor: pointer;
  }

  .toggle-label input {
    accent-color: #cba6f7;
  }

  .list-subtitle {
    font-size: 0.85rem;
    font-weight: 600;
    color: #bac2de;
    display: block;
    margin-bottom: 0.4rem;
  }

  .selected-members-list {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .member-item-row {
    display: flex;
    align-items: center;
    gap: 0.8rem;
    background: #1e1e2e;
    border: 1px solid #313244;
    border-radius: 8px;
    padding: 0.5rem 0.8rem;
    transition: all 0.15s;
  }

  .member-item-row.disabled {
    opacity: 0.6;
    border-color: #45475a;
  }

  .member-item-row.muted {
    border-left: 3px solid #f9e2af;
  }

  .order-num {
    font-size: 0.8rem;
    font-weight: 700;
    color: #cba6f7;
    min-width: 24px;
  }

  .member-avatar {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    overflow: hidden;
    background: #313244;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .member-avatar img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .char-init {
    font-size: 0.75rem;
    font-weight: 700;
    color: #cdd6f4;
  }

  .member-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .member-name {
    font-size: 0.9rem;
    font-weight: 600;
    color: #cdd6f4;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .member-snippet {
    font-size: 0.75rem;
    color: #a6adc8;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .member-controls {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .pill-toggle {
    background: #313244;
    border: 1px solid #45475a;
    border-radius: 12px;
    color: #a6adc8;
    font-size: 0.75rem;
    padding: 0.2rem 0.6rem;
    cursor: pointer;
    transition: all 0.15s;
  }

  .pill-toggle.active {
    background: rgba(166, 227, 161, 0.15);
    border-color: #a6e3a1;
    color: #a6e3a1;
  }

  .mute-toggle.active {
    background: rgba(249, 226, 175, 0.15);
    border-color: #f9e2af;
    color: #f9e2af;
  }

  .reorder-btn {
    background: #313244;
    border: none;
    border-radius: 4px;
    color: #cdd6f4;
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    font-size: 0.7rem;
  }

  .reorder-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .remove-member-btn {
    background: transparent;
    border: none;
    color: #f38ba8;
    font-size: 0.9rem;
    cursor: pointer;
    padding: 0.2rem 0.4rem;
    border-radius: 4px;
  }

  .remove-member-btn:hover {
    background: rgba(243, 139, 168, 0.15);
  }

  .empty-members-hint {
    background: rgba(249, 226, 175, 0.08);
    border: 1px dashed #f9e2af;
    border-radius: 8px;
    padding: 0.8rem;
    font-size: 0.85rem;
    color: #f9e2af;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .available-chars-section {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-top: 0.5rem;
  }

  .available-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .filter-input {
    background: #1e1e2e;
    border: 1px solid #313244;
    border-radius: 6px;
    color: #cdd6f4;
    padding: 0.3rem 0.6rem;
    font-size: 0.8rem;
    outline: none;
  }

  .chars-picker-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
    gap: 0.5rem;
    max-height: 180px;
    overflow-y: auto;
    padding-right: 0.2rem;
  }

  .char-pick-card {
    background: #1e1e2e;
    border: 1px solid #313244;
    border-radius: 8px;
    padding: 0.4rem 0.6rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
    text-align: left;
    transition: all 0.15s;
  }

  .char-pick-card:hover {
    border-color: #cba6f7;
  }

  .char-pick-card.selected {
    border-color: #a6e3a1;
    background: rgba(166, 227, 161, 0.08);
  }

  .char-pick-avatar {
    width: 30px;
    height: 30px;
    border-radius: 50%;
    overflow: hidden;
    background: #313244;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    font-size: 0.7rem;
    color: #cdd6f4;
  }

  .char-pick-avatar img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .char-pick-meta {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1;
  }

  .char-pick-name {
    font-size: 0.82rem;
    font-weight: 600;
    color: #cdd6f4;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .char-pick-status {
    font-size: 0.72rem;
    color: #a6adc8;
  }

  .char-pick-card.selected .char-pick-status {
    color: #a6e3a1;
    font-weight: 600;
  }

  .modal-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1rem 1.5rem;
    border-top: 1px solid #313244;
    background: #181825;
    border-bottom-left-radius: 12px;
    border-bottom-right-radius: 12px;
  }

  .right-actions {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    margin-left: auto;
  }

  .danger-btn {
    background: rgba(243, 139, 168, 0.15);
    border: 1px solid #f38ba8;
    color: #f38ba8;
    padding: 0.55rem 1rem;
    border-radius: 8px;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s;
  }

  .danger-btn:hover {
    background: #f38ba8;
    color: #11111b;
  }

  .cancel-btn {
    background: transparent;
    border: 1px solid #45475a;
    color: #cdd6f4;
    padding: 0.55rem 1rem;
    border-radius: 8px;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s;
  }

  .cancel-btn:hover {
    background: #313244;
  }

  .secondary-btn {
    background: #313244;
    border: 1px solid #45475a;
    color: #cdd6f4;
    padding: 0.55rem 1rem;
    border-radius: 8px;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s;
  }

  .secondary-btn:hover:not(:disabled) {
    border-color: #cba6f7;
    color: #cba6f7;
  }

  .primary-btn {
    background: #cba6f7;
    border: 1px solid #cba6f7;
    color: #11111b;
    padding: 0.55rem 1.2rem;
    border-radius: 8px;
    font-size: 0.85rem;
    font-weight: 700;
    cursor: pointer;
    transition: all 0.15s;
  }

  .primary-btn:hover:not(:disabled) {
    background: #b4befe;
    border-color: #b4befe;
  }

  .primary-btn:disabled,
  .secondary-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
