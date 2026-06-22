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
