<script lang="ts">
  import type { Character, Group, TurnMode } from '../types';

  export let group: Group;
  export let characters: Character[] = [];
  export let nextSpeakerId: string | null = null;
  export let isGenerating = false;
  export let currentStreamingCharacterId: string | null = null;

  export let onTriggerSpeaker: (characterId: string) => void;
  export let onNextTurn: () => void;
  export let onEditGroup: () => void;
  export let onToggleTurnMode: () => void;
  export let onToggleAutoMode: () => void;
  export let onToggleMute: (characterId: string) => void;

  function getCharacter(id: string): Character | undefined {
    return characters.find((c) => c.id === id);
  }

  function getTurnModeLabel(mode: TurnMode): string {
    switch (mode) {
      case 'Natural':
        return '🔄 Sequential';
      case 'Manual':
        return '🎯 Manual';
      case 'Random':
        return '🎲 Random';
      default:
        return mode;
    }
  }

  $: nextSpeakerChar = nextSpeakerId ? getCharacter(nextSpeakerId) : null;
</script>

<div class="group-turn-bar">
  <!-- Left info: Group Avatar, Name, Mode Badges -->
  <div class="group-identity">
    <div class="group-avatar">
      {#if group.avatar_data_url}
        <img src={group.avatar_data_url} alt={group.name} />
      {:else}
        <span>👥</span>
      {/if}
    </div>

    <div class="group-text">
      <div class="group-name-row">
        <span class="group-name" title={group.name}>{group.name}</span>
        <button
          type="button"
          class="badge-btn turn-mode-badge"
          on:click={onToggleTurnMode}
          title="Click to switch turn mode (Sequential / Manual / Random)"
        >
          {getTurnModeLabel(group.turn_mode)}
        </button>
        <button
          type="button"
          class="badge-btn auto-badge {group.auto_mode ? 'on' : 'off'}"
          on:click={onToggleAutoMode}
          title="Auto-trigger response after sending message"
        >
          {group.auto_mode ? '⚡ Auto' : '⏸ Manual'}
        </button>
      </div>
      {#if group.description}
        <span class="group-desc-snippet" title={group.description}>
          {group.description}
        </span>
      {/if}
    </div>
  </div>

  <!-- Center: Member Character Turn Pills -->
  <div class="turn-queue-container">
    <span class="queue-label">Speakers:</span>
    <div class="turn-queue">
      {#each group.members as member, idx}
        {@const char = getCharacter(member.character_id)}
        {@const isNext = member.character_id === nextSpeakerId}
        {@const isStreaming = isGenerating && member.character_id === currentStreamingCharacterId}
        <div
          class="character-turn-pill {!member.enabled ? 'disabled' : ''} {member.mute ? 'muted' : ''} {isNext ? 'next-speaker' : ''} {isStreaming ? 'streaming' : ''}"
        >
          <button
            type="button"
            class="pill-main-btn"
            disabled={isGenerating || !member.enabled}
            on:click={() => onTriggerSpeaker(member.character_id)}
            title={`Trigger ${char?.card.data.name || 'Character'} to speak`}
          >
            <div class="pill-avatar">
              {#if char?.avatar_data_url}
                <img src={char.avatar_data_url} alt={char.card.data.name} />
              {:else}
                <span>{(char?.card.data.name || '?').slice(0, 2).toUpperCase()}</span>
              {/if}
              {#if isStreaming}
                <span class="pulsing-dot"></span>
              {/if}
            </div>

            <div class="pill-info">
              <span class="pill-name">{char?.card.data.name || 'Unknown'}</span>
              {#if isStreaming}
                <span class="pill-status typing">Speaking...</span>
              {:else if isNext}
                <span class="pill-status next">Next ➔</span>
              {/if}
            </div>
          </button>

          <button
            type="button"
            class="pill-mute-btn"
            on:click|stopPropagation={() => onToggleMute(member.character_id)}
            title={member.mute ? 'Unmute character' : 'Mute character'}
          >
            {member.mute ? '🔇' : '🔊'}
          </button>
        </div>
      {/each}
    </div>
  </div>

  <!-- Right Actions: Next Turn & Edit Group -->
  <div class="group-actions">
    <button
      type="button"
      class="next-turn-btn"
      disabled={isGenerating}
      on:click={onNextTurn}
      title={nextSpeakerChar ? `Trigger next turn (${nextSpeakerChar.card.data.name})` : 'Trigger next character turn'}
    >
      <span class="play-icon">▶</span>
      <span class="btn-text">
        {#if isGenerating}
          Generating...
        {:else if nextSpeakerChar}
          Turn: {nextSpeakerChar.card.data.name}
        {:else}
          Next Turn
        {/if}
      </span>
    </button>

    <button
      type="button"
      class="icon-action-btn"
      on:click={onEditGroup}
      title="Edit Group Settings & Members"
    >
      ⚙
    </button>
  </div>
</div>

<style>
  .group-turn-bar {
    background: #181825;
    border-bottom: 1px solid #313244;
    padding: 0.5rem 1rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    position: relative;
    z-index: 10;
    flex-wrap: wrap;
  }

  .group-identity {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    min-width: 0;
    flex-shrink: 0;
  }

  .group-avatar {
    width: 36px;
    height: 36px;
    border-radius: 8px;
    overflow: hidden;
    background: #313244;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    font-size: 1.2rem;
  }

  .group-avatar img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .group-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .group-name-row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    flex-wrap: wrap;
  }

  .group-name {
    font-size: 0.95rem;
    font-weight: 700;
    color: #cdd6f4;
    max-width: 150px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .badge-btn {
    border: none;
    border-radius: 12px;
    font-size: 0.7rem;
    font-weight: 600;
    padding: 0.15rem 0.5rem;
    cursor: pointer;
    transition: all 0.15s;
    white-space: nowrap;
  }

  .turn-mode-badge {
    background: #313244;
    color: #cba6f7;
    border: 1px solid #45475a;
  }

  .turn-mode-badge:hover {
    background: #45475a;
  }

  .auto-badge.on {
    background: rgba(166, 227, 161, 0.15);
    color: #a6e3a1;
    border: 1px solid #a6e3a1;
  }

  .auto-badge.off {
    background: rgba(249, 226, 175, 0.15);
    color: #f9e2af;
    border: 1px solid #f9e2af;
  }

  .group-desc-snippet {
    font-size: 0.72rem;
    color: #a6adc8;
    max-width: 220px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .turn-queue-container {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex: 1;
    min-width: 0;
    overflow: hidden;
  }

  .queue-label {
    font-size: 0.75rem;
    font-weight: 700;
    color: #a6adc8;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    flex-shrink: 0;
  }

  .turn-queue {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    overflow-x: auto;
    padding: 0.2rem 0;
    scrollbar-width: thin;
  }

  .character-turn-pill {
    display: flex;
    align-items: center;
    background: #1e1e2e;
    border: 1px solid #313244;
    border-radius: 20px;
    padding: 0.15rem 0.3rem 0.15rem 0.2rem;
    gap: 0.2rem;
    transition: all 0.15s;
    flex-shrink: 0;
  }

  .character-turn-pill:hover {
    border-color: #585b70;
  }

  .character-turn-pill.next-speaker {
    border-color: #cba6f7;
    background: rgba(203, 166, 247, 0.1);
    box-shadow: 0 0 8px rgba(203, 166, 247, 0.2);
  }

  .character-turn-pill.streaming {
    border-color: #a6e3a1;
    background: rgba(166, 227, 161, 0.12);
    animation: pulseBorder 1.5s infinite;
  }

  @keyframes pulseBorder {
    0%, 100% {
      box-shadow: 0 0 4px rgba(166, 227, 161, 0.3);
    }
    50% {
      box-shadow: 0 0 12px rgba(166, 227, 161, 0.6);
    }
  }

  .character-turn-pill.muted {
    opacity: 0.65;
    border-color: #45475a;
  }

  .character-turn-pill.disabled {
    opacity: 0.4;
    pointer-events: none;
  }

  .pill-main-btn {
    background: transparent;
    border: none;
    display: flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.1rem 0.3rem;
    cursor: pointer;
    color: inherit;
  }

  .pill-avatar {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    overflow: hidden;
    background: #313244;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    position: relative;
    font-size: 0.65rem;
    color: #cdd6f4;
    font-weight: 700;
  }

  .pill-avatar img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .pulsing-dot {
    position: absolute;
    bottom: 0;
    right: 0;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #a6e3a1;
    border: 1px solid #181825;
    animation: pulse 1s infinite;
  }

  @keyframes pulse {
    0%, 100% { transform: scale(1); opacity: 1; }
    50% { transform: scale(1.4); opacity: 0.7; }
  }

  .pill-info {
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }

  .pill-name {
    font-size: 0.8rem;
    font-weight: 600;
    color: #cdd6f4;
    white-space: nowrap;
  }

  .pill-status {
    font-size: 0.68rem;
    font-weight: 700;
    padding: 0.05rem 0.3rem;
    border-radius: 8px;
    white-space: nowrap;
  }

  .pill-status.next {
    background: #cba6f7;
    color: #11111b;
  }

  .pill-status.typing {
    background: #a6e3a1;
    color: #11111b;
  }

  .pill-mute-btn {
    background: transparent;
    border: none;
    font-size: 0.75rem;
    cursor: pointer;
    padding: 0.1rem 0.2rem;
    border-radius: 50%;
    opacity: 0.7;
    transition: opacity 0.15s;
  }

  .pill-mute-btn:hover {
    opacity: 1;
  }

  .group-actions {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    flex-shrink: 0;
  }

  .next-turn-btn {
    background: #cba6f7;
    border: 1px solid #cba6f7;
    color: #11111b;
    border-radius: 8px;
    padding: 0.35rem 0.75rem;
    font-size: 0.8rem;
    font-weight: 700;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 0.35rem;
    transition: all 0.15s;
    white-space: nowrap;
  }

  .next-turn-btn:hover:not(:disabled) {
    background: #b4befe;
    border-color: #b4befe;
  }

  .next-turn-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .play-icon {
    font-size: 0.7rem;
  }

  .icon-action-btn {
    background: #313244;
    border: 1px solid #45475a;
    color: #cdd6f4;
    border-radius: 8px;
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.95rem;
    cursor: pointer;
    transition: all 0.15s;
  }

  .icon-action-btn:hover {
    border-color: #cba6f7;
    color: #cba6f7;
  }

  @media (max-width: 768px) {
    .group-turn-bar {
      padding: 0.4rem 0.6rem;
      gap: 0.5rem;
    }

    .group-desc-snippet {
      display: none;
    }

    .queue-label {
      display: none;
    }
  }
</style>
