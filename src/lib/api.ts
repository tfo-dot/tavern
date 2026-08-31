import { invoke } from '@tauri-apps/api/core';
import type {
  Character,
  ChatTree,
  ChatSummary,
  MessageViewNode,
  AppSettings,
  UserPersona,
  AuthorRole,
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

export async function getActiveMessages(): Promise<MessageViewNode[]> {
  return await invoke<MessageViewNode[]>('get_active_messages');
}

export async function appendMessage(
  role: AuthorRole,
  content: string,
  parentId: string | null
): Promise<string> {
  return await invoke<string>('append_message', {
    role,
    content,
    parentId,
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
export async function generateReply(isSwipe = false): Promise<void> {
  await invoke('generate_reply', { isSwipe });
}

export async function abortGeneration(): Promise<void> {
  await invoke('abort_generation');
}
