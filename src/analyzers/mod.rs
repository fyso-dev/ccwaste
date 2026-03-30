pub mod file_rereads;
pub mod killed_subagents;
pub mod metadata_bloat;
pub mod repeated_toolsearch;
pub mod review_cycles;
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
    ]
}

pub fn run_all(session: &Session) -> Vec<WasteFinding> {
    all_analyzers()
        .iter()
        .flat_map(|a| a.analyze(session))
        .collect()
}
