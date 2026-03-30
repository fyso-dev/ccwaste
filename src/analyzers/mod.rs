pub mod broad_searches;
pub mod claudemd_bloat;
pub mod context_accumulation;
pub mod file_rereads;
pub mod killed_subagents;
pub mod metadata_bloat;
pub mod missing_claudeignore;
pub mod model_overkill;
pub mod repeated_toolsearch;
pub mod review_cycles;
pub mod self_inflicted_diffs;
pub mod tool_errors;

use crate::types::{Session, WasteFinding};

pub trait WasteAnalyzer {
    fn name(&self) -> &str;
    fn analyze(&self, session: &Session) -> Vec<WasteFinding>;
}

pub fn all_analyzers() -> Vec<Box<dyn WasteAnalyzer>> {
    vec![
        Box::new(metadata_bloat::MetadataBloatAnalyzer),
        Box::new(file_rereads::FileRereadsAnalyzer),
        Box::new(tool_errors::ToolErrorsAnalyzer),
        Box::new(killed_subagents::KilledSubagentsAnalyzer),
        Box::new(review_cycles::ReviewCyclesAnalyzer),
        Box::new(repeated_toolsearch::RepeatedToolSearchAnalyzer),
        Box::new(model_overkill::ModelOverkillAnalyzer),
        Box::new(context_accumulation::ContextAccumulationAnalyzer),
        Box::new(self_inflicted_diffs::SelfInflictedDiffsAnalyzer),
        Box::new(claudemd_bloat::ClaudeMdBloatAnalyzer),
        Box::new(broad_searches::BroadSearchesAnalyzer),
        Box::new(missing_claudeignore::MissingClaudeignoreAnalyzer),
    ]
}

pub fn run_all(session: &Session) -> Vec<WasteFinding> {
    all_analyzers()
        .iter()
        .flat_map(|a| a.analyze(session))
        .collect()
}
