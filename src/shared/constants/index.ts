export const GAME_TYPES = [
  { value: 'card_game', label: 'Card Game / Roguelike Card' },
  { value: 'visual_novel', label: 'Galgame / Visual Novel' },
  { value: 'rpg', label: 'RPG (Coming Soon)' },
  { value: 'puzzle', label: 'Puzzle (Coming Soon)' },
] as const;

export const WORKFLOW_TYPES = [
  { value: 'card_game_concept', label: 'Card Game Concept' },
  { value: 'visual_novel_concept', label: 'Visual Novel Concept' },
  { value: 'game_design_doc', label: 'Game Design Document' },
] as const;

export const AGENT_NAMES = [
  'ProducerAgent',
  'GameDesignerAgent',
  'NarrativeAgent',
  'RuleAgent',
  'ArtDirectorAgent',
  'CardGameAgent',
  'VNAgent',
  'QAAgent',
  'MemoryAgent',
] as const;

export const MEMORY_TYPES = [
  'world_setting',
  'character',
  'plot',
  'rule',
  'card',
  'item',
  'level',
  'art_style',
  'rejected_idea',
  'export_record',
  'qa_review',
  'system_internal',
] as const;

export const LAYER_DEFINITIONS = [
  { layer: 'L1' as const, name: 'Session Memory', description: 'Current task context, conversation goals, temporary constraints, agent execution state.', allowed_scopes: ['session'] },
  { layer: 'L2' as const, name: 'Project Memory', description: 'World setting, characters, plot, rules, cards/items/levels, art style, rejected ideas, export records.', allowed_scopes: ['project'] },
  { layer: 'L3' as const, name: 'User Preference Memory', description: 'Preferred game types, platforms, art styles, models, narrative length, plot pacing, rule complexity.', allowed_scopes: ['global'] },
  { layer: 'L4' as const, name: 'System Evolution Memory', description: 'Agent workflow success rate, prompt template effectiveness, user-accepted/rejected system improvements.', allowed_scopes: ['project', 'global'] },
] as const;

export const PREFERENCE_KEYS = [
  'preferred_game_types',
  'preferred_platforms',
  'preferred_art_styles',
  'favorite_models',
  'accepted_output_types',
  'rejected_output_types',
  'narrative_length_preference',
  'plot_pacing_preference',
  'rule_complexity_preference',
] as const;
