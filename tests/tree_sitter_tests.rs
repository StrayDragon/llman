use llman::tool::tree_sitter_processor::TreeSitterProcessor;

/// Basic test for Tree-sitter processor functionality
/// Tests that the processor can be created successfully with all supported languages

#[test]
fn test_tree_sitter_processor_creation() {
    // Should successfully create processor with all supported languages
    let processor = TreeSitterProcessor::new();
    assert!(processor.is_ok(), "Failed to create TreeSitterProcessor");
}
