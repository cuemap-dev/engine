# Changelog

All notable changes to the CueMap Rust Engine will be documented in this file.

## [0.5.0] - 2025-12-27

### Added
- **Selective Set Intersection**: A new, more exhaustive search strategy that replaces legacy tiered search. It scans the most selective cue list and uses O(1) probes to gather intersection data.
- **Continuous Gradient Scoring**: Replaced discrete search tiers with a smooth scoring gradient based on recency and reinforcement frequency.
- **Asynchronous Intelligence Pipeline**: Background job system for LLM-based fact extraction, cue proposal, and automatic alias discovery.
- **Explainable AI**: Support for the `explain=true` flag in recall requests, providing detailed breakdowns of intersection, recency, and frequency components.
- **Expanded Chunker Support**: Added native support for 14+ new formats:
    - **Documents**: PDF, Word (DOCX), Excel (XLSX).
    - **Data**: CSV, JSON, YAML, XML.
    - **Languages**: HTML, CSS, PHP, Java, JavaScript, Go (in addition to Python, Rust, TS).
- **Binary Ingestion**: The agent now handles binary files gracefully, computing hashes and extracting text for ingestion.
- **Multi-Tenant Isolation**: Full isolation between projects, including independent taxonomies, lexicons, and memory stores.
- **Advanced Text Normalization**: Improved NLP normalization that better handles special characters and word boundaries.
- **Lexicon Resolution**: Support for training a lexicon from existing memories to map natural language tokens to canonical cues.

### Changed
- **Memory Storage**: Optimized `OrderedSet` with `get_index_of` for O(1) recency lookup.
- **Recall Weighting**: Intersection scores are now weighted by cue relevance, improving precision for complex queries.
- **Persistence**: Enhanced snapshot mechanism with reliable roundtrip verification.

### Fixed
- **Recall Boundary Issues**: Fixed cases where niche items deep in a cue list were missed by tiered search.
- **Reinforcement Precision**: Corrected log-frequency scaling to ensure exact reinforcement scores.
- **NLP Tokenization**: Fixed edge cases in `normalize_text` involving punctuation.

### Removed
- Legacy iterative search tiers (`TIER_1_DEPTH`, `TIER_2_DEPTH`).
- Unused `BinaryHeap` implementation in favor of faster unstable sorting.

---

## [0.4.0] - 2025-11-20
### Added
- Initial support for multiple projects.
- Batch ingestion optimizations for high-throughput scenarios.
- Basic telemetry and logging infrastructure.

## [0.3.0] - 2025-10-15
### Added
- REST API layer using Axum.
- Tiered search strategy (v1).
- Concurrent indexing with DashMap.

## [0.2.0] - 2025-09-05
### Added
- Persistent storage via binary snapshots.
- CLI tool for local debugging and management.
- Improved memory synchronization.

## [0.1.0] - 2025-08-10
### Added
- Initial core engine prototype.
- In-memory memory storage and basic tokenization.
- Fundamental scoring based on exact match.

---
*Note: This version represents a significant architectural shift towards more intelligent, non-blocking asynchronous operations.*
