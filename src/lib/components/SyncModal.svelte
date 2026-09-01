<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import type { DiscoveredPeer, SyncDeviceInfo, SyncStats } from '$lib/types';
  import {
    getSyncDeviceInfo,
    scanSyncPeers,
    triggerSync,
    updateSyncSettings,
    getSettings,
  } from '$lib/api';

  export let isOpen = false;
  export let onClose: () => void;
  export let onSyncSuccess: () => Promise<void>;

  let deviceInfo: SyncDeviceInfo | null = null;
  let discoveredPeers: DiscoveredPeer[] = [];
  let peerPins: Record<string, string> = {};

  let isScanning = false;
  let isSyncing = false;
  let isSavingSettings = false;
  let syncingTarget: string | null = null;

  let manualAddress = '';
  let manualPin = '';

  let deviceNameDraft = '';
  let syncPinDraft = '';
  let copiedText = false;

  let alertMessage: { type: 'success' | 'error'; text: string } | null = null;
  let unlistenSync: UnlistenFn | null = null;

  $: if (isOpen) {
    loadInfo();
    handleScan();
  }

  onMount(async () => {
    unlistenSync = await listen<SyncStats>('sync-completed', async (event) => {
      alertMessage = {
        type: 'success',
        text: `✓ ${event.payload.message || 'Sync completed successfully!'}`,
      };
      if (onSyncSuccess) {
        await onSyncSuccess();
      }
    });
  });

  onDestroy(() => {
    if (unlistenSync) {
      unlistenSync();
    }
  });

  async function loadInfo() {
    try {
      deviceInfo = await getSyncDeviceInfo();
      deviceNameDraft = deviceInfo.device_name;
      const settings = await getSettings();
      syncPinDraft = settings.sync_pin || '';
    } catch (e) {
      console.error('Failed to load sync info:', e);
    }
  }

  async function handleSaveDeviceSettings() {
    isSavingSettings = true;
    alertMessage = null;
    try {
      await updateSyncSettings(deviceNameDraft.trim(), syncPinDraft.trim());
      await loadInfo();
      alertMessage = {
        type: 'success',
        text: 'Sync settings updated successfully.',
      };
    } catch (e) {
      alertMessage = {
        type: 'error',
        text: `Failed to save sync settings: ${String(e)}`,
      };
    } finally {
      isSavingSettings = false;
    }
  }

  async function handleScan() {
    if (isScanning) return;
    isScanning = true;
    alertMessage = null;
    try {
      discoveredPeers = await scanSyncPeers(1500);
    } catch (e) {
      console.error('Scan error:', e);
    } finally {
      isScanning = false;
    }
  }

  async function handleSyncWithAddress(targetAddress: string, pin?: string) {
    if (isSyncing || !targetAddress.trim()) return;
    isSyncing = true;
    syncingTarget = targetAddress;
    alertMessage = null;

    try {
      const stats = await triggerSync(targetAddress.trim(), pin?.trim() || undefined);
      alertMessage = {
        type: 'success',
        text: `✓ Synced with ${targetAddress}! ${stats.message}`,
      };
      if (onSyncSuccess) {
        await onSyncSuccess();
      }
    } catch (e) {
      alertMessage = {
        type: 'error',
        text: `Sync failed: ${String(e)}`,
      };
    } finally {
      isSyncing = false;
      syncingTarget = null;
    }
  }

  async function handleCopyAddress(addr: string) {
    try {
      await navigator.clipboard.writeText(addr);
      copiedText = true;
      setTimeout(() => (copiedText = false), 2000);
    } catch (e) {
      console.error('Failed to copy:', e);
    }
  }
</script>

{#if isOpen}
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
      role="presentation"
    >
      <header class="modal-header">
        <div class="header-title">
          <span class="header-icon">🔄</span>
          <div>
            <h2>Local Device Sync</h2>
            <p class="subtitle">Seamless CRDT peer-to-peer sync between devices on your local Wi-Fi</p>
          </div>
        </div>
        <button class="btn-close" on:click={onClose} aria-label="Close modal">✕</button>
      </header>

      <div class="modal-body">
        {#if alertMessage}
          <div class="alert-banner {alertMessage.type}">
            <span>{alertMessage.text}</span>
            <button class="alert-dismiss" on:click={() => (alertMessage = null)}>✕</button>
          </div>
        {/if}

        <!-- Section 1: Discovered Peers on Local Network -->
        <div class="section-card">
          <div class="section-header">
            <div class="section-title">
              <span class="section-icon">📡</span>
              <h3>Nearby Devices (Local Network)</h3>
            </div>
            <button
              class="btn-scan {isScanning ? 'spinning' : ''}"
              on:click={handleScan}
              disabled={isScanning}
            >
              {#if isScanning}
                Scanning...
              {:else}
                🔍 Scan Nearby
              {/if}
            </button>
          </div>

          <div class="peers-list">
            {#if isScanning && discoveredPeers.length === 0}
              <div class="empty-state">
                <div class="spinner"></div>
                <p>Searching for Tavern instances on your local Wi-Fi...</p>
              </div>
            {:else if discoveredPeers.length === 0}
              <div class="empty-state">
                <span class="empty-icon">📱</span>
                <p>No nearby devices discovered automatically.</p>
                <span class="empty-hint">
                  Make sure Tavern is open on your other device connected to the same Wi-Fi. You can also connect manually below.
                </span>
              </div>
            {:else}
              {#each discoveredPeers as peer (peer.device_id)}
                <div class="peer-item">
                  <div class="peer-info">
                    <div class="peer-badge">
                      <span class="peer-dot">●</span>
                      <span class="peer-name">{peer.device_name}</span>
                    </div>
                    <span class="peer-address">{peer.address}</span>
                  </div>

                  <div class="peer-actions">
                    <input
                      type="password"
                      placeholder="PIN (if set)"
                      class="peer-pin-input"
                      bind:value={peerPins[peer.device_id]}
                    />
                    <button
                      class="btn-sync"
                      disabled={isSyncing}
                      on:click={() => handleSyncWithAddress(peer.address, peerPins[peer.device_id])}
                    >
                      {#if isSyncing && syncingTarget === peer.address}
                        Syncing...
                      {:else}
                        ⚡ Sync Now
                      {/if}
                    </button>
                  </div>
                </div>
              {/each}
            {/if}
          </div>
        </div>

        <!-- Section 2: Manual Direct Connect -->
        <div class="section-card">
          <div class="section-header">
            <div class="section-title">
              <span class="section-icon">🔗</span>
              <h3>Manual Connect & Sync</h3>
            </div>
          </div>
          <p class="section-desc">
            If UDP discovery is blocked by your router's AP isolation, enter your other device's IP & Port directly:
          </p>
          <div class="manual-sync-row">
            <input
              type="text"
              class="input-field flex-grow"
              placeholder="e.g. 192.168.1.50:24567"
              bind:value={manualAddress}
            />
            <input
              type="password"
              class="input-field pin-field"
              placeholder="PIN"
              bind:value={manualPin}
            />
            <button
              class="btn-sync"
              disabled={isSyncing || !manualAddress.trim()}
              on:click={() => handleSyncWithAddress(manualAddress, manualPin)}
            >
              {#if isSyncing && syncingTarget === manualAddress}
                Syncing...
              {:else}
                Connect & Sync
              {/if}
            </button>
          </div>
        </div>

        <!-- Section 3: This Device Info & Settings -->
        <div class="section-card">
          <div class="section-header">
            <div class="section-title">
              <span class="section-icon">💻</span>
              <h3>This Device</h3>
            </div>
            <div class="status-chip">
              <span class="status-dot">●</span>
              <span>Sync Port: {deviceInfo?.port || 24567}</span>
            </div>
          </div>

          <div class="device-details-grid">
            <div class="form-group">
              <label for="dev-name">Device Name</label>
              <input
                id="dev-name"
                type="text"
                class="input-field"
                bind:value={deviceNameDraft}
                placeholder="e.g. Linux Desktop, Phone"
              />
            </div>

            <div class="form-group">
              <label for="dev-pin">Pairing PIN (Optional Security)</label>
              <input
                id="dev-pin"
                type="password"
                class="input-field"
                bind:value={syncPinDraft}
                placeholder="Leave blank for open LAN sync"
              />
            </div>
          </div>

          {#if deviceInfo?.local_ips && deviceInfo.local_ips.length > 0}
            <div class="ip-list-group">
              <label for="ip-list-label">Your Local IP Addresses (for other devices to connect):</label>
              <div id="ip-list-label" class="ip-chips">
                {#each deviceInfo.local_ips as ip}
                  {@const fullAddr = `${ip}:${deviceInfo.port}`}
                  <button
                    class="ip-chip"
                    on:click={() => handleCopyAddress(fullAddr)}
                    title="Click to copy address"
                  >
                    <span>{fullAddr}</span>
                    <span class="chip-copy-icon">📋</span>
                  </button>
                {/each}
                {#if copiedText}
                  <span class="copy-toast">Copied to clipboard!</span>
                {/if}
              </div>
            </div>
          {/if}

          <div class="device-save-footer">
            <button
              class="btn-save-settings"
              disabled={isSavingSettings}
              on:click={handleSaveDeviceSettings}
            >
              {isSavingSettings ? 'Saving...' : 'Save Device Settings'}
            </button>
          </div>
        </div>
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
    z-index: 999;
    padding: 16px;
  }

  .modal-card {
    background: var(--bg-primary, #1e1e2e);
    color: var(--text-primary, #cdd6f4);
    border: 1px solid var(--border-color, #313244);
    border-radius: 12px;
    width: 100%;
    max-width: 680px;
    max-height: 90vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 16px 36px rgba(0, 0, 0, 0.5);
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-color, #313244);
    background: var(--bg-secondary, #181825);
  }

  .header-title {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .header-icon {
    font-size: 1.6rem;
  }

  .header-title h2 {
    margin: 0;
    font-size: 1.2rem;
    font-weight: 600;
  }

  .subtitle {
    margin: 2px 0 0;
    font-size: 0.8rem;
    color: var(--text-secondary, #a6adc8);
  }

  .btn-close {
    background: transparent;
    border: none;
    color: var(--text-secondary, #a6adc8);
    font-size: 1.2rem;
    cursor: pointer;
    padding: 4px 8px;
    border-radius: 6px;
    transition: all 0.15s ease;
  }

  .btn-close:hover {
    color: var(--text-primary, #cdd6f4);
    background: rgba(255, 255, 255, 0.1);
  }

  .modal-body {
    padding: 20px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .alert-banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    border-radius: 8px;
    font-size: 0.88rem;
    line-height: 1.4;
  }

  .alert-banner.success {
    background: rgba(166, 227, 161, 0.15);
    border: 1px solid #a6e3a1;
    color: #a6e3a1;
  }

  .alert-banner.error {
    background: rgba(243, 139, 168, 0.15);
    border: 1px solid #f38ba8;
    color: #f38ba8;
  }

  .alert-dismiss {
    background: transparent;
    border: none;
    color: inherit;
    font-size: 1rem;
    cursor: pointer;
  }

  .section-card {
    background: var(--bg-secondary, #181825);
    border: 1px solid var(--border-color, #313244);
    border-radius: 10px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .section-title {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .section-icon {
    font-size: 1.1rem;
  }

  .section-title h3 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
  }

  .section-desc {
    margin: 0;
    font-size: 0.84rem;
    color: var(--text-secondary, #a6adc8);
  }

  .btn-scan {
    background: var(--accent-color, #cba6f7);
    color: #11111b;
    border: none;
    border-radius: 6px;
    padding: 6px 12px;
    font-size: 0.82rem;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.15s ease;
  }

  .btn-scan:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-sync {
    background: #89b4fa;
    color: #11111b;
    border: none;
    border-radius: 6px;
    padding: 8px 14px;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
    transition: filter 0.15s ease;
  }

  .btn-sync:hover:not(:disabled) {
    filter: brightness(1.1);
  }

  .btn-sync:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .peers-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-height: 60px;
  }

  .peer-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px;
    background: var(--bg-primary, #1e1e2e);
    border: 1px solid var(--border-color, #313244);
    border-radius: 8px;
    gap: 12px;
    flex-wrap: wrap;
  }

  .peer-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .peer-badge {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .peer-dot {
    color: #a6e3a1;
    font-size: 0.8rem;
  }

  .peer-name {
    font-weight: 600;
    font-size: 0.95rem;
  }

  .peer-address {
    font-size: 0.78rem;
    color: var(--text-secondary, #a6adc8);
    font-family: monospace;
  }

  .peer-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .peer-pin-input {
    width: 90px;
    background: var(--bg-secondary, #181825);
    border: 1px solid var(--border-color, #313244);
    color: var(--text-primary, #cdd6f4);
    border-radius: 6px;
    padding: 6px 8px;
    font-size: 0.82rem;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 16px;
    text-align: center;
    color: var(--text-secondary, #a6adc8);
    font-size: 0.85rem;
    gap: 6px;
  }

  .empty-icon {
    font-size: 1.5rem;
    margin-bottom: 4px;
  }

  .empty-hint {
    font-size: 0.76rem;
    opacity: 0.8;
  }

  .manual-sync-row {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }

  .input-field {
    background: var(--bg-primary, #1e1e2e);
    border: 1px solid var(--border-color, #313244);
    color: var(--text-primary, #cdd6f4);
    border-radius: 6px;
    padding: 8px 10px;
    font-size: 0.88rem;
  }

  .flex-grow {
    flex: 1;
    min-width: 200px;
  }

  .pin-field {
    width: 100px;
  }

  .device-details-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  @media (max-width: 600px) {
    .device-details-grid {
      grid-template-columns: 1fr;
    }
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .form-group label {
    font-size: 0.8rem;
    color: var(--text-secondary, #a6adc8);
  }

  .status-chip {
    display: flex;
    align-items: center;
    gap: 6px;
    background: rgba(166, 227, 161, 0.1);
    color: #a6e3a1;
    padding: 3px 8px;
    border-radius: 12px;
    font-size: 0.75rem;
    font-weight: 500;
  }

  .status-dot {
    font-size: 0.7rem;
  }

  .ip-list-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .ip-list-group label {
    font-size: 0.8rem;
    color: var(--text-secondary, #a6adc8);
  }

  .ip-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
  }

  .ip-chip {
    background: var(--bg-primary, #1e1e2e);
    border: 1px solid var(--border-color, #313244);
    color: #89b4fa;
    border-radius: 6px;
    padding: 4px 8px;
    font-size: 0.8rem;
    font-family: monospace;
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .ip-chip:hover {
    border-color: #89b4fa;
    background: rgba(137, 180, 250, 0.1);
  }

  .chip-copy-icon {
    font-size: 0.75rem;
    opacity: 0.7;
  }

  .copy-toast {
    font-size: 0.75rem;
    color: #a6e3a1;
  }

  .device-save-footer {
    display: flex;
    justify-content: flex-end;
    margin-top: 4px;
  }

  .btn-save-settings {
    background: transparent;
    border: 1px solid var(--border-color, #313244);
    color: var(--text-primary, #cdd6f4);
    border-radius: 6px;
    padding: 6px 12px;
    font-size: 0.82rem;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .btn-save-settings:hover:not(:disabled) {
    background: var(--bg-primary, #1e1e2e);
    border-color: var(--text-secondary, #a6adc8);
  }

  .spinner {
    width: 20px;
    height: 20px;
    border: 2px solid var(--border-color, #313244);
    border-top-color: var(--accent-color, #cba6f7);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
