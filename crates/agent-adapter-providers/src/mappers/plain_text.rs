use super::*;

/// Plain text mapper for providers that output plain text.
pub struct PlainTextMapper {
    final_text: String,
}

impl PlainTextMapper {
    pub fn new() -> Self {
        Self {
            final_text: String::new(),
        }
    }

    pub fn map_line(&mut self, line: &str) -> Vec<AdapterResult<AgentEvent>> {
        if line.is_empty() {
            return vec![];
        }

        self.final_text.push_str(line);
        self.final_text.push('\n');

        vec![Ok(AgentEvent::TextDelta {
            role: Role::Assistant,
            text: line.to_string(),
        })]
    }

    pub fn finish(self) -> Vec<AdapterResult<AgentEvent>> {
        if self.final_text.is_empty() {
            vec![]
        } else {
            vec![Ok(AgentEvent::RunCompleted {
                final_text: Some(self.final_text.trim_end().to_string()),
                external_session: None,
            })]
        }
    }
}

impl Default for PlainTextMapper {
    fn default() -> Self {
        Self::new()
    }
}
