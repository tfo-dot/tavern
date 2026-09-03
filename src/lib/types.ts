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
  character_book?: CharacterBook | null;
  lorebook_ids?: string[];
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

export type TurnMode = 'Manual' | 'Natural' | 'Random';

export interface GroupMember {
  character_id: string;
  enabled: boolean;
  mute: boolean;
}

export interface Group {
  id: string;
  name: string;
  description: string;
  avatar_data_url: string | null;
  members: GroupMember[];
  turn_mode: TurnMode;
  allow_self_responses: boolean;
  auto_mode: boolean;
  created_at: string;
  updated_at: string;
}

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
  character_id?: string | null;
  name?: string | null;
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
  group_id?: string | null;
}

export interface ChatSummary {
  id: string;
  character_id: string;
  title: string;
  created_at: string;
  updated_at: string;
  message_count: number;
  last_message_preview: string;
  group_id?: string | null;
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
  global_lorebook_ids?: string[];
  device_name?: string;
  sync_port?: number;
  sync_pin?: string;
}

export interface SyncDeviceInfo {
  device_id: string;
  device_name: string;
  port: number;
  version: string;
  local_ips: string[];
}

export interface DiscoveredPeer {
  device_id: string;
  device_name: string;
  address: string;
  port: number;
  last_seen_epoch_ms: number;
}

export interface SyncStats {
  characters_synced: number;
  chats_synced: number;
  personas_synced: number;
  lorebooks_synced: number;
  message: string;
}
export interface UserPersona {
  id: string;
  name: string;
  description: string;
  avatar_data_url: string | null;
}

export type LorebookPosition =
  | 'before_char'
  | 'after_char'
  | 'before_scenario'
  | 'after_scenario'
  | 'top_system'
  | 'bottom_system'
  | 'at_depth';

export type SelectiveLogic = 0 | 1 | 2 | 3;

export interface LorebookEntry {
  id: string;
  keys: string[];
  secondary_keys: string[];
  content: string;
  comment: string;
  enabled: boolean;
  constant: boolean;
  selective: boolean;
  selective_logic: SelectiveLogic;
  position: LorebookPosition;
  depth: number;
  order: number;
  case_sensitive: boolean;
  use_regex: boolean;
  prevent_recursion: boolean;
  scan_depth?: number | null;
  extensions?: Record<string, unknown>;
}

export interface CharacterBook {
  name?: string | null;
  description?: string | null;
  scan_depth?: number | null;
  token_budget?: number | null;
  recursive_scanning?: boolean | null;
  entries: LorebookEntry[];
  extensions?: Record<string, unknown>;
}

export interface Lorebook {
  id: string;
  name: string;
  description: string;
  scan_depth: number;
  token_budget: number;
  recursive_scanning: boolean;
  global: boolean;
  entries: LorebookEntry[];
  created_at: string;
  updated_at: string;
  extensions?: Record<string, unknown>;
}
