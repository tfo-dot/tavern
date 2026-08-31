export interface CharacterData {
  name: string;
  description: string;
  personality: string;
  scenario: string;
  first_mes: string;
  mes_example: string;
  creator_notes?: string;
  system_prompt?: string;
  post_history_instructions?: string;
  alternate_greetings?: string[];
  tags?: string[];
  creator?: string;
  character_version?: string;
  extensions?: Record<string, unknown>;
}

export interface CharacterCardV2 {
  spec: string;
  spec_version: string;
  data: CharacterData;
}

export interface Character {
  id: string;
  card: CharacterCardV2;
  avatar_data_url: string | null;
  created_at: string;
  updated_at: string;
}

export type AuthorRole = 'User' | 'Assistant' | 'System';

export interface MessageViewNode {
  id: string;
  parent_id: string | null;
  role: AuthorRole;
  content: string;
  created_at: string;
  sibling_index: number;
  sibling_total: number;
  can_swipe_left: boolean;
  can_swipe_right: boolean;
}

export interface ChatTree {
  id: string;
  character_id: string;
  title: string;
  created_at: string;
  updated_at: string;
  root_message_ids: string[];
  nodes: Record<string, unknown>;
  active_root_index: number;
}

export interface ChatSummary {
  id: string;
  character_id: string;
  title: string;
  created_at: string;
  updated_at: string;
  message_count: number;
  last_message_preview: string;
}

export interface AppSettings {
  endpoint: string;
  api_key: string;
  active_model: string;
  temperature: number;
  top_p: number;
  frequency_penalty: number;
  presence_penalty: number;
  max_tokens: number;
  max_context_tokens: number;
  system_template: string;
  stop_sequences: string[];
  active_character_id: string | null;
  active_chat_id: string | null;
  active_persona_id?: string | null;
}

export interface UserPersona {
  id: string;
  name: string;
  description: string;
  avatar_data_url: string | null;
}
