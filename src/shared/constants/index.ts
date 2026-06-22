export const GAME_TYPES = [
  { value: 'card_game', label: '卡牌游戏 / Roguelike Card' },
  { value: 'visual_novel', label: '视觉小说 / Visual Novel' },
] as const;

export const WORKFLOW_TYPES = [
  { value: 'card_game_concept', label: '卡牌游戏概念' },
  { value: 'visual_novel_concept', label: '视觉小说概念' },
  { value: 'game_design_doc', label: '游戏设计文档' },
] as const;

export const LANGUAGES = [
  { value: 'zh', label: '中文' },
  { value: 'en', label: 'English' },
] as const;

export const DEFAULT_LANGUAGE = 'zh';

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
  { layer: 'L1' as const, name: '会话记忆', description: '当前任务上下文、对话目标、临时约束、Agent 执行状态。', allowed_scopes: ['session'] },
  { layer: 'L2' as const, name: '项目记忆', description: '世界观、角色、剧情、规则、卡牌/道具/关卡、美术风格、已否定方案。', allowed_scopes: ['project'] },
  { layer: 'L3' as const, name: '用户偏好记忆', description: '偏好游戏类型、平台、美术风格、常用模型、文案长度偏好等。', allowed_scopes: ['global'] },
  { layer: 'L4' as const, name: '系统进化记忆', description: 'Agent 工作流成功率、Prompt 模板效果、用户接受/拒绝的系统改进。', allowed_scopes: ['project', 'global'] },
] as const;
