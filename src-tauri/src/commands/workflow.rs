use crate::models::WorkflowType;
use std::collections::HashMap;
use std::sync::LazyLock;

#[derive(Clone, Copy)]
pub struct AgentDefinition {
    pub name: &'static str,
    pub role_description: &'static str,
}

pub struct WorkflowStep {
    pub step_key: &'static str,
    pub step_order: i32,
    pub agent_name: &'static str,
    pub step_type: &'static str,
    pub system_prompt: &'static str,
    pub user_prompt_template: &'static str,
    pub save_to_memory: bool,
    pub use_rag: bool,
    pub max_previous_output_chars: usize,
    pub max_rag_chars: usize,
}

pub struct WorkflowDefinition {
    pub workflow_type: WorkflowType,
    pub name: &'static str,
    pub steps: &'static [WorkflowStep],
}

static AGENT_REGISTRY: LazyLock<HashMap<&'static str, AgentDefinition>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("ProducerAgent", AgentDefinition {
        name: "ProducerAgent",
        role_description: "Analyzes game creation tasks, decomposes into components, selects workflow and coordinates agents.",
    });
    m.insert("GameDesignerAgent", AgentDefinition {
        name: "GameDesignerAgent",
        role_description: "Creates detailed game design documents covering mechanics, core loop, rules, and content.",
    });
    m.insert("QAAgent", AgentDefinition {
        name: "QAAgent",
        role_description: "Reviews game design content for issues, inconsistencies, balance problems, and scope risks.",
    });
    m
});

pub fn get_agent(agent_name: &str) -> Option<AgentDefinition> {
    AGENT_REGISTRY.get(agent_name).copied()
}

pub fn list_agents() -> Vec<AgentDefinition> {
    AGENT_REGISTRY.values().copied().collect()
}

pub fn get_workflow(workflow_type: &WorkflowType) -> Option<&'static WorkflowDefinition> {
    match workflow_type {
        WorkflowType::CardGameConcept => Some(&CARD_GAME_CONCEPT),
        WorkflowType::VisualNovelConcept => Some(&VISUAL_NOVEL_CONCEPT),
        WorkflowType::GameDesignDoc => Some(&GAME_DESIGN_DOC),
        WorkflowType::Custom(_) => None,
    }
}

static CARD_GAME_CONCEPT: WorkflowDefinition = WorkflowDefinition {
    workflow_type: WorkflowType::CardGameConcept,
    name: "Card Game Concept",
    steps: &[
        WorkflowStep {
            step_key: "producer.plan",
            step_order: 1, agent_name: "ProducerAgent", step_type: "planning",
            system_prompt: "You are a ProducerAgent. Analyze game creation tasks and break them down into structured components. Output JSON with: overview, components, agent_assignments, execution_plan.",
            user_prompt_template: "Analyze this game creation task:\n\nTask: {task_description}\nWorkflow Type: card_game_concept\n\nOutput your analysis as JSON.",
            save_to_memory: false, use_rag: false,
            max_previous_output_chars: 0, max_rag_chars: 0,
        },
        WorkflowStep {
            step_key: "designer.design",
            step_order: 2, agent_name: "GameDesignerAgent", step_type: "design",
            system_prompt: "You are a GameDesignerAgent. Create detailed game design documents. Focus on card mechanics, core loop, card types, resource systems, combat rules. Output with clear markdown sections.",
            user_prompt_template: "Based on this producer plan, create a detailed card game design:\n\n{previous_output}\n\nOutput comprehensive game design content with clear markdown sections.",
            save_to_memory: true, use_rag: true,
            max_previous_output_chars: crate::commands::security::MAX_PREVIOUS_OUTPUT_CHARS,
            max_rag_chars: crate::commands::security::MAX_RAG_CONTEXT_CHARS,
        },
        WorkflowStep {
            step_key: "qa.review",
            step_order: 3, agent_name: "QAAgent", step_type: "review",
            system_prompt: "You are a QAAgent. Review game design content and identify issues, inconsistencies, missing elements, balance problems, scope risks. Be constructive and specific.",
            user_prompt_template: "Review this card game design and identify issues:\n\n{previous_output}\n\nProvide structured review: 1) Issues Found 2) Missing Elements 3) Balance/Consistency Concerns 4) Scope Risks 5) Improvement Suggestions.",
            save_to_memory: true, use_rag: true,
            max_previous_output_chars: crate::commands::security::MAX_PREVIOUS_OUTPUT_CHARS,
            max_rag_chars: crate::commands::security::MAX_RAG_CONTEXT_CHARS,
        },
    ],
};

static VISUAL_NOVEL_CONCEPT: WorkflowDefinition = WorkflowDefinition {
    workflow_type: WorkflowType::VisualNovelConcept,
    name: "Visual Novel Concept",
    steps: &[
        WorkflowStep {
            step_key: "producer.plan",
            step_order: 1, agent_name: "ProducerAgent", step_type: "planning",
            system_prompt: "You are a ProducerAgent. Analyze game creation tasks and break them down into structured components. Output JSON with: overview, components, agent_assignments, execution_plan.",
            user_prompt_template: "Analyze this game creation task:\n\nTask: {task_description}\nWorkflow Type: visual_novel_concept\n\nOutput your analysis as JSON.",
            save_to_memory: false, use_rag: false,
            max_previous_output_chars: 0, max_rag_chars: 0,
        },
        WorkflowStep {
            step_key: "designer.design",
            step_order: 2, agent_name: "GameDesignerAgent", step_type: "design",
            system_prompt: "You are a GameDesignerAgent. Create detailed game design documents. Focus on story structure, branching narrative, character relationships, emotional pacing. Output with clear markdown sections.",
            user_prompt_template: "Based on this producer plan, create a detailed visual novel design:\n\n{previous_output}\n\nOutput comprehensive game design content with clear markdown sections.",
            save_to_memory: true, use_rag: true,
            max_previous_output_chars: crate::commands::security::MAX_PREVIOUS_OUTPUT_CHARS,
            max_rag_chars: crate::commands::security::MAX_RAG_CONTEXT_CHARS,
        },
        WorkflowStep {
            step_key: "qa.review",
            step_order: 3, agent_name: "QAAgent", step_type: "review",
            system_prompt: "You are a QAAgent. Review game design content and identify issues, inconsistencies, missing elements, balance problems, scope risks. Be constructive and specific.",
            user_prompt_template: "Review this visual novel design and identify issues:\n\n{previous_output}\n\nProvide structured review: 1) Issues Found 2) Missing Elements 3) Balance/Consistency Concerns 4) Scope Risks 5) Improvement Suggestions.",
            save_to_memory: true, use_rag: true,
            max_previous_output_chars: crate::commands::security::MAX_PREVIOUS_OUTPUT_CHARS,
            max_rag_chars: crate::commands::security::MAX_RAG_CONTEXT_CHARS,
        },
    ],
};

static GAME_DESIGN_DOC: WorkflowDefinition = WorkflowDefinition {
    workflow_type: WorkflowType::GameDesignDoc,
    name: "Game Design Document",
    steps: &[
        WorkflowStep {
            step_key: "producer.plan",
            step_order: 1, agent_name: "ProducerAgent", step_type: "planning",
            system_prompt: "You are a ProducerAgent. Analyze game creation tasks and break them down into structured components. Output JSON with: overview, components, agent_assignments, execution_plan.",
            user_prompt_template: "Analyze this game creation task:\n\nTask: {task_description}\nWorkflow Type: game_design_doc\n\nOutput your analysis as JSON.",
            save_to_memory: false, use_rag: false,
            max_previous_output_chars: 0, max_rag_chars: 0,
        },
        WorkflowStep {
            step_key: "designer.design",
            step_order: 2, agent_name: "GameDesignerAgent", step_type: "design",
            system_prompt: "You are a GameDesignerAgent. Create detailed game design documents. Cover game mechanics, systems, rules, and content structure. Output with clear markdown sections.",
            user_prompt_template: "Based on this producer plan, create a detailed game design document:\n\n{previous_output}\n\nOutput comprehensive game design content with clear markdown sections.",
            save_to_memory: true, use_rag: true,
            max_previous_output_chars: crate::commands::security::MAX_PREVIOUS_OUTPUT_CHARS,
            max_rag_chars: crate::commands::security::MAX_RAG_CONTEXT_CHARS,
        },
        WorkflowStep {
            step_key: "qa.review",
            step_order: 3, agent_name: "QAAgent", step_type: "review",
            system_prompt: "You are a QAAgent. Review game design content and identify issues, inconsistencies, missing elements, balance problems, scope risks. Be constructive and specific.",
            user_prompt_template: "Review this game design document and identify issues:\n\n{previous_output}\n\nProvide structured review: 1) Issues Found 2) Missing Elements 3) Balance/Consistency Concerns 4) Scope Risks 5) Improvement Suggestions.",
            save_to_memory: true, use_rag: true,
            max_previous_output_chars: crate::commands::security::MAX_PREVIOUS_OUTPUT_CHARS,
            max_rag_chars: crate::commands::security::MAX_RAG_CONTEXT_CHARS,
        },
    ],
};
