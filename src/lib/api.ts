import { invoke } from '@tauri-apps/api/core';
import type {
  Character,
  Group,
  ChatTree,
  ChatSummary,
  MessageViewNode,
  AppSettings,
  UserPersona,
  AuthorRole,
  Lorebook,
  SyncDeviceInfo,
  DiscoveredPeer,
  SyncStats,
} from './types';
// Character APIs
export async function getAllCharacters(): Promise<Character[]> {
  return await invoke<Character[]>('get_all_characters');
}

export async function getCharacter(id: string): Promise<Character> {
  return await invoke<Character>('get_character', { id });
}

export async function saveCharacter(character: Character): Promise<Character> {
  return await invoke<Character>('save_character', { character });
}

export async function deleteCharacter(id: string): Promise<void> {
  await invoke('delete_character', { id });
}

export async function importCharacterCard(fileBytes: number[] | Uint8Array): Promise<Character> {
  const bytesArray = Array.from(fileBytes);
  return await invoke<Character>('import_character_card', { fileBytes: bytesArray });
}

export async function exportCardPng(characterId: string): Promise<number[]> {
  return await invoke<number[]>('export_card_png', { characterId });
}

export async function exportCardJson(characterId: string): Promise<string> {
  return await invoke<string>('export_card_json', { characterId });
}


// Group APIs
export async function getAllGroups(): Promise<Group[]> {
  return await invoke<Group[]>('get_all_groups');
}

export async function getGroup(id: string): Promise<Group> {
  return await invoke<Group>('get_group', { id });
}

export async function saveGroup(group: Group): Promise<Group> {
  return await invoke<Group>('save_group', { group });
}

export async function deleteGroup(id: string): Promise<void> {
  await invoke('delete_group', { id });
}

export async function createGroupChat(
  groupId: string,
  title?: string,
  firstMes?: string,
  firstSpeakerId?: string
): Promise<ChatTree> {
  return await invoke<ChatTree>('create_group_chat', {
    groupId,
    title: title || null,
    firstMes: firstMes || null,
    firstSpeakerId: firstSpeakerId || null,
  });
}

export async function listGroupChats(groupId: string): Promise<ChatSummary[]> {
  return await invoke<ChatSummary[]>('list_group_chats', { groupId });
}

export async function setMessageAuthor(
  id: string,
  characterId: string | null,
  name: string | null
): Promise<MessageViewNode[]> {
  return await invoke<MessageViewNode[]>('set_message_author', {
    id,
    characterId,
    name,
  });
}
// Chat APIs
export async function createChat(
  characterId: string,
  firstMes?: string,
  title?: string
): Promise<ChatTree> {
  return await invoke<ChatTree>('create_chat', {
    characterId,
    firstMes: firstMes || null,
    title: title || null,
  });
}

export async function loadChat(chatId: string): Promise<ChatTree> {
  return await invoke<ChatTree>('load_chat', { chatId });
}

export async function listChats(characterId: string): Promise<ChatSummary[]> {
  return await invoke<ChatSummary[]>('list_chats', { characterId });
}

export async function deleteChat(chatId: string): Promise<void> {
  await invoke('delete_chat', { chatId });
}

export async function importChatJsonl(
  fileBytes: number[] | Uint8Array,
  characterId?: string,
  title?: string
): Promise<ChatTree> {
  const bytesArray = Array.from(fileBytes);
  return await invoke<ChatTree>('import_chat_jsonl', {
    fileBytes: bytesArray,
    characterId: characterId || null,
    title: title || null,
  });
}

export async function exportChatJsonl(chatId: string): Promise<string> {
  return await invoke<string>('export_chat_jsonl', { chatId });
}

export async function getActiveMessages(): Promise<MessageViewNode[]> {
  return await invoke<MessageViewNode[]>('get_active_messages');
}

export async function appendMessage(
  role: AuthorRole,
  content: string,
  parentId: string | null,
  characterId?: string | null,
  name?: string | null
): Promise<string> {
  return await invoke<string>('append_message', {
    role,
    content,
    parentId,
    characterId: characterId || null,
    name: name || null,
  });
}

export async function editMessage(id: string, newContent: string): Promise<MessageViewNode[]> {
  return await invoke<MessageViewNode[]>('edit_message', {
    id,
    newContent,
  });
}

export async function deleteMessage(id: string): Promise<MessageViewNode[]> {
  return await invoke<MessageViewNode[]>('delete_message', { id });
}

export async function switchBranch(
  parentId: string | null,
  index: number
): Promise<MessageViewNode[]> {
  return await invoke<MessageViewNode[]>('switch_branch', {
    parentId,
    index,
  });
}

// Settings & Persona APIs
export async function getSettings(): Promise<AppSettings> {
  return await invoke<AppSettings>('get_settings');
}

export async function saveSettings(newSettings: AppSettings): Promise<void> {
  await invoke('save_settings', { newSettings });
}

export async function getAllUserPersonas(): Promise<UserPersona[]> {
  return await invoke<UserPersona[]>('get_all_user_personas');
}

export async function getActiveUserPersona(): Promise<UserPersona> {
  return await invoke<UserPersona>('get_active_user_persona');
}

export async function saveUserPersona(newPersona: UserPersona): Promise<UserPersona> {
  return await invoke<UserPersona>('save_user_persona', { newPersona });
}

export async function deleteUserPersona(id: string): Promise<void> {
  await invoke('delete_user_persona', { id });
}

export async function setActiveUserPersona(id: string): Promise<UserPersona> {
  return await invoke<UserPersona>('set_active_user_persona', { id });
}

export async function fetchEndpointModels(
  endpoint: string,
  apiKey: string
): Promise<string[]> {
  return await invoke<string[]>('fetch_endpoint_models', { endpoint, apiKey });
}

// Generation APIs
export async function generateReply(
  isSwipe = false,
  isContinue = false,
  targetCharacterId?: string | null
): Promise<void> {
  await invoke('generate_reply', {
    isSwipe,
    isContinue,
    targetCharacterId: targetCharacterId || null,
  });
}

export async function continueReply(): Promise<void> {
  await invoke('generate_reply', { isSwipe: false, isContinue: true, targetCharacterId: null });
}

export async function abortGeneration(): Promise<void> {
  await invoke('abort_generation');
}

// Lorebook APIs
export async function getAllLorebooks(): Promise<Lorebook[]> {
  return await invoke<Lorebook[]>('get_all_lorebooks');
}

export async function getLorebook(id: string): Promise<Lorebook> {
  return await invoke<Lorebook>('get_lorebook', { id });
}

export async function saveLorebook(lorebook: Lorebook): Promise<Lorebook> {
  return await invoke<Lorebook>('save_lorebook', { lorebook });
}

export async function deleteLorebook(id: string): Promise<void> {
  await invoke('delete_lorebook', { id });
}

export async function importLorebook(fileBytes: number[] | Uint8Array): Promise<Lorebook> {
  const bytesArray = Array.from(fileBytes);
  return await invoke<Lorebook>('import_lorebook', { fileBytes: bytesArray });
}

export async function exportLorebookJson(id: string): Promise<string> {
  return await invoke<string>('export_lorebook_json', { id });
}

// Local CRDT Sync APIs
export async function getSyncDeviceInfo(): Promise<SyncDeviceInfo> {
  return await invoke<SyncDeviceInfo>('get_sync_device_info');
}

export async function scanSyncPeers(timeoutMs?: number): Promise<DiscoveredPeer[]> {
  return await invoke<DiscoveredPeer[]>('scan_sync_peers', { timeoutMs });
}

export async function triggerSync(targetAddress: string, pin?: string): Promise<SyncStats> {
  return await invoke<SyncStats>('trigger_sync', { targetAddress, pin });
}

export async function updateSyncSettings(deviceName?: string, syncPin?: string): Promise<void> {
  await invoke('update_sync_settings', { deviceName, syncPin });
}
