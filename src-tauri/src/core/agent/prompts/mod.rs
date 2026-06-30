pub mod writer_prompts;
pub mod planner_prompts;
pub mod settler_prompts;
pub mod observer_prompts;
pub mod auditor_prompts;
pub mod reviser_prompts;
pub mod architect_prompts;
pub mod shared_sections;

pub use shared_sections::{
    assemble_with_identity, output_discipline, react_discipline,
    OUTPUT_DISCIPLINE_EN, OUTPUT_DISCIPLINE_ZH,
    REACT_DISCIPLINE_EN, REACT_DISCIPLINE_ZH,
};
