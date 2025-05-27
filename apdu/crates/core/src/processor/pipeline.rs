//! Processor pipeline for command processing
//!
//! This module provides a pipeline for command processors, allowing
//! multiple processors to be chained together.

use std::fmt;

use super::{CommandProcessor, TransportAdapter, TransportAdapterTrait};
use crate::{
    Command, Error, Response, command::ApduCommand, error::ResultExt, transport::CardTransport,
};

/// Command processor pipeline
///
/// A pipeline of command processors that are executed in sequence.
#[derive(Default)]
pub struct ProcessorPipeline {
    processors: Vec<Box<dyn CommandProcessor>>,
}

impl fmt::Debug for ProcessorPipeline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProcessorPipeline")
            .field("processor_count", &self.processors.len())
            .finish()
    }
}

impl ProcessorPipeline {
    /// Create a new empty processor pipeline
    pub fn new() -> Self {
        Self {
            processors: Vec::new(),
        }
    }

    /// Add a processor to the pipeline
    pub fn add_processor(&mut self, processor: Box<dyn CommandProcessor>) -> &mut Self {
        self.processors.push(processor);
        self
    }

    /// Clear all processors from the pipeline
    pub fn clear(&mut self) {
        self.processors.clear();
    }

    /// Process a command through the pipeline with a transport adapter
    pub fn process_command_with_adapter<T: CardTransport>(
        &self,
        command: &Command,
        adapter: &mut TransportAdapter<'_, T>,
    ) -> Result<Response, Error> {
        if self.processors.is_empty() {
            // Direct transmit using raw transport
            let command_bytes = command.to_bytes();
            let response_bytes = <TransportAdapter<'_, T> as TransportAdapterTrait>::transmit_raw(
                adapter,
                &command_bytes,
            )
            .context("Transport error during raw transmission")?;
            Response::from_bytes(&response_bytes).context("Failed to parse response")
        } else {
            // Find the first applicable processor
            if let Some(processor) = self.processors.first() {
                let result = processor.process_command_with_adapter(
                    command,
                    adapter as &mut dyn TransportAdapterTrait,
                );
                // If processor handles it, return its result
                // Don't continue to subsequent processors
                return result;
            }

            // Should never reach here as we already checked for empty
            Err(Error::protocol("No processor handled the command"))
        }
    }

    /// Process a command through the pipeline
    pub fn process_command<T: CardTransport>(
        &self,
        command: &Command,
        transport: &mut T,
    ) -> Result<Response, Error> {
        let mut adapter = TransportAdapter::new(transport);
        self.process_command_with_adapter(command, &mut adapter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processor::processors::IdentityProcessor;
    use crate::transport::MockTransport;
    use bytes::Bytes;

    #[test]
    fn test_empty_pipeline() {
        let pipeline = ProcessorPipeline::new();
        let response_data = Bytes::from_static(&[0x90, 0x00]);
        let mut transport = MockTransport::with_response(response_data.clone());

        let command = Command::new(0x00, 0xA4, 0x04, 0x00);
        let response = pipeline.process_command(&command, &mut transport).unwrap();

        assert_eq!(response.status.sw1, 0x90);
        assert_eq!(response.status.sw2, 0x00);
        assert!(response.data.is_none());
    }

    #[test]
    fn test_pipeline_with_processor() {
        let mut pipeline = ProcessorPipeline::new();
        pipeline.add_processor(Box::new(IdentityProcessor));

        let response_data = Bytes::from_static(&[0x90, 0x00]);
        let mut transport = MockTransport::with_response(response_data.clone());

        let command = Command::new(0x00, 0xA4, 0x04, 0x00);
        let response = pipeline.process_command(&command, &mut transport).unwrap();

        assert_eq!(response.status.sw1, 0x90);
        assert_eq!(response.status.sw2, 0x00);
        assert!(response.data.is_none());
    }
}
