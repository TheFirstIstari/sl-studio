# SL Studio - Forensic Document Analysis Platform

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in RFC 2119.

---

# Part I: Introduction

## 1. Purpose and Scope

### 1.1 Purpose
This document specifies the requirements for **SL Studio**, a forensic document analysis workstation that enables investigators to process large volumes of evidence (PDFs, images, audio/video) through a pipeline of extraction and AI-powered reasoning to extract structured facts.

### 1.2 Scope
SL Studio provides:
- Local-only processing (no cloud dependencies)
- Two-stage pipeline: text extraction + LLM inference
- Multi-pass LLM pipelines for structured fact extraction
- Forensic chain of custody with audit logging
- Interactive analysis UI with timeline, network graphs, and maps

---

## 2. Definitions and Acronyms

| Term | Definition |
|------|------------|
| **LLM** | Large Language Model |
| **FTS5** | SQLite Full-Text Search v5 |
| **NER** | Named Entity Recognition |
| **IQR** | Interquartile Range (for outlier detection) |
| **SRS** | Software Requirements Specification |
| **NFR** | Non-Functional Requirement |
| **FR** | Functional Requirement |

---

## 3. References

- RFC 2119 - Key Words for Use in RFCs
- ISO/IEC/IEEE 29148:2018 - Systems and software engineering - Life cycle processes - Requirements engineering

---

## 4. Overall Description

### 4.1 Product Perspective
SL Studio is a desktop application built with Tauri (Rust backend + SvelteKit frontend) that processes evidence files locally without cloud dependencies.

### 4.2 User Characteristics
- **Forensic Investigators**: Primary users who analyze evidence
- **Law Enforcement**: Handle large volumes of evidence
- **System Administrators**: Configure hardware and processing

### 4.3 Product Functions
- Text extraction from PDFs, images, audio
- Multi-pass LLM inference pipelines
- Entity extraction and relationship analysis
- Timeline and network visualization
- Evidence chain tracking
- Case comparison across projects

### 4.4 Operating Environment
- Windows, macOS, Linux (via Tauri)
- Local SQLite databases
- GPU-accelerated inference (optional)

### 4.5 Design Constraints
- All processing MUST run locally (NFR-PRIV-001)
- Original evidence files MUST remain unmodified (NFR-FOR-001)
- Database operations MUST use soft deletes (NFR-FOR-006)

---

## 5. Considerations

### 5.1 Security Considerations

#### Input Validation
- **All user inputs MUST be validated** before processing
- File paths MUST be sanitized to prevent path traversal attacks
- SQL queries MUST use parameterized statements to prevent injection
- File type validation MUST occur before processing

#### File Handling
- Files MUST be validated before reading (magic bytes, not just extension)
- Dangerous file types MUST be rejected with clear error messages
- File size limits MUST be enforced (NFR-FILE-002)

#### Memory Safety
- LLM inference MUST have memory limits to prevent system exhaustion
- Large file processing MUST use streaming to avoid loading entire files into memory

### 5.2 Performance Constraints

#### Resource Limits
| Resource | Limit | Configurable |
|----------|-------|--------------|
| Max file size | 500 MB | Yes (NFR-FILE-002) |
| LLM context | Model limit | No |
| GPU memory | 80% available | Yes (NFR-PERF-002) |
| Batch size | 32 files | Yes (NFR-PERF-003) |
| Concurrent extractions | CPU cores | Yes |

#### Large File Handling
- Files exceeding context limits MUST be chunked (NFR-ADV-003)
- Chunk overlap MUST be maintained for context continuity
- Progress MUST be reported for long operations

### 5.3 Edge Cases

#### File Handling
| Edge Case | Handling |
|-----------|----------|
| Empty file | Skip with warning logged |
| Zero bytes | Reject with error |
| Corrupted file | Log error, continue processing (NFR-FILE-001) |
| Unsupported format | Skip with warning (NFR-FILE-003) |
| Password-protected | Skip with specific error message |
| Very long lines | Truncate at 10MB per line |
| Binary content in text | Detect and skip |

#### LLM Inference
| Edge Case | Handling |
|-----------|----------|
| Timeout | Retry with shorter context |
| Invalid JSON output | Retry with different temperature |
| Empty response | Flag for manual review |
| Model not loaded | Queue and retry |

### 5.4 Legal and Compliance

#### Chain of Custody
- All file access MUST be logged (NFR-AUDIT-001)
- Timestamps MUST use UTC for consistency
- Hash verification MUST occur on import

#### Court Admissibility
- Export formats MUST preserve source attribution
- Audit log MUST be exportable
- Quality scores SHOULD be included in reports

#### Evidence Handling
- Original files MUST never be modified (NFR-FOR-001)
- All operations MUST be reproducible (FR-SOURCE-003)

### 5.5 Licensing

#### LLM Model Licensing
- Users MUST accept model licenses before download
- Some models may have commercial restrictions
- License MUST be stored with model metadata

#### Dependencies
- All open source licenses MUST be compliant
- License audit SHOULD be performed before release

### 5.6 Operational Considerations

#### Logging
- All operations MUST log to structured log format
- Log levels: ERROR, WARN, INFO, DEBUG
- Sensitive data MUST NOT be logged (passwords, secrets)

#### Diagnostics
- System info export MUST be available for debugging
- Crash reports MUST include stack traces
- Performance profiling data SHOULD be collectible

#### Recovery
- Database integrity checks MUST run on startup (NFR-DATA-003)
- Corrupted databases SHOULD offer recovery options
- Checkpoints MUST enable resume after crash (NFR-REL-001)

---

# Part II: Requirements Specification

## 6. Stakeholder Needs

> All requirements in this document are traced to stakeholder needs in [Section 5](#5-stakeholder-needs). {#stakeholder-needs}

| ID | Stakeholder | Need | Addressed By |
|-----|-------------|------|--------------|
| SN-001 | Forensic Investigators | Privacy - data must not leave local machine | [NFR-PRIV-001](#nfr-priv-001) |
| SN-002 | Forensic Investigators | Non-destructive operations - original evidence must never be modified | [NFR-FOR-001](#nfr-forensic-001), [NFR-FOR-002](#nfr-forensic-002) |
| SN-003 | Forensic Investigators | Forensic rigor - every fact linked to source evidence | [FR-EVD-001](#fr-evd-001) |
| SN-004 | Law Enforcement | Handle large volumes of evidence efficiently | [FR-PER-001](#fr-per-001), [FR-INC-001](#fr-inc-001) |
| SN-005 | Investigators | Cross-platform support (Windows, macOS, Linux) | [NFR-PLAT-001](#nfr-plat-001) |
| SN-006 | Investigators | Process only new/modified files to save time | [FR-INC-001](#fr-inc-001) |
| SN-007 | Investigators | Review quality of extracted facts | [FR-QUAL-001](#fr-qual-001) |
| SN-008 | Investigators | Export results in standard formats | [FR-EXP-001](#fr-exp-001) |
| SN-009 | Investigators | Track relationships between evidence | [FR-CHAIN-001](#fr-chain-001) |
| SN-010 | System Administrators | Auto-detect and utilize available hardware | [NFR-PER-003](#nfr-per-003) |
| SN-011 | Investigators | Compare facts across multiple cases | [FR-CMP-001](#fr-cmp-001) |
| SN-012 | Investigators | View evidence timeline chronologically | [FR-TIME-001](#fr-timeline-001) |
| SN-013 | Investigators | Visualize evidence locations on map | [FR-MAP-001](#fr-map-001) |
| SN-014 | Investigators | Manage local LLM models | [FR-MODEL-001](#fr-model-001) |
| SN-015 | Forensic Investigators | Audit trail of all actions for chain of custody | [NFR-AUDIT-001](#nfr-audit-001) |
| SN-016 | Investigators | Securely handle sensitive extracted data | [NFR-SEC-001](#nfr-sec-001) |
| SN-017 | Investigators | Backup and restore project data | [NFR-DATA-001](#nfr-data-001), [NFR-DATA-002](#nfr-data-002) |
| SN-018 | Power Users | Command-line interface for automation | [NFR-INT-001](#nfr-int-001) |
| SN-019 | Investigators | Process large volumes efficiently | [NFR-ADV-001](#nfr-adv-001), [NFR-ADV-002](#nfr-adv-002) |
| SN-020 | Investigators | Verify and validate extracted facts | [NFR-VALID-001](#nfr-valid-001), [NFR-VALID-002](#nfr-valid-002) |
| SN-021 | Investigators | Tune performance for their hardware | [NFR-PERF-001](#nfr-perf-001), [NFR-PERF-002](#nfr-perf-002) |
| SN-022 | Investigators | Handle corrupted or problematic files | [NFR-FILE-001](#nfr-file-001) |
| SN-023 | Investigators | Interactive timeline with zoom/pan | [FR-TIME-003](#fr-timeline-003), [FR-TIME-004](#fr-timeline-004), [FR-TIME-005](#fr-timeline-005) |
| SN-024 | Investigators | Filter search results by multiple criteria | [FR-FACET-001](#fr-facet-001), [FR-FACET-002](#fr-facet-002) |
| SN-025 | Investigators | Match related entities across files | [FR-ER-001](#fr-er-001), [FR-ER-002](#fr-er-002) |
| SN-026 | Investigators | Analyze patterns and anomalies in evidence | [FR-ANALYSIS-001](#fr-analysis-001), [FR-ANALYSIS-002](#fr-analysis-002), [FR-ANALYSIS-003](#fr-analysis-003) |
| SN-027 | Investigators | Receive alerts for processing status | [FR-NOTIF-001](#fr-notif-001), [FR-NOTIF-002](#fr-notif-002), [FR-NOTIF-003](#fr-notif-003) |
| SN-028 | Investigators | Validate extracted facts against source | [FR-VAL-001](#fr-val-001), [FR-VAL-002](#fr-val-002), [FR-VAL-004](#fr-val-004) |
| SN-029 | Investigators | Analyze temporal patterns in evidence | [FR-TEMP-001](#fr-temp-001), [FR-TEMP-002](#fr-temp-002) |
| SN-030 | Investigators | Detect backdated or forged documents | [FR-TEMP-003](#fr-temp-003) |
| SN-031 | Investigators | Visualize entity communication networks | [FR-NET-001](#fr-net-001), [FR-NET-003](#fr-net-003) |
| SN-032 | Investigators | Identify key communication hubs | [FR-NET-002](#fr-net-002), [FR-NET-004](#fr-net-004) |
| SN-033 | Investigators | Detect anomalies in evidence set | [FR-ANOM-001](#fr-anom-001), [FR-ANOM-002](#fr-anom-002) |
| SN-034 | Investigators | Weight evidence by importance | [FR-WEIGHT-001](#fr-weight-001), [FR-WEIGHT-002](#fr-weight-002) |
| SN-035 | Investigators | Track which model extracted each fact | [FR-SOURCE-001](#fr-source-001), [FR-SOURCE-002](#fr-source-002) |
| SN-036 | Investigators | Reproduce extraction with specific model version | [FR-SOURCE-003](#fr-source-003) |
| SN-037 | Investigators | Extract metadata from evidence files | [FR-META-001](#fr-meta-001), [FR-META-002](#fr-meta-002) |
| SN-038 | Investigators | Extract structured data from documents | [FR-STRUCT-001](#fr-struct-001), [FR-STRUCT-002](#fr-struct-002) |
| SN-039 | Investigators | Analyze audio and video files | [FR-MEDIA-001](#fr-media-001), [FR-MEDIA-002](#fr-media-002) |
| SN-040 | Investigators | Detect manipulated images | [FR-IMAGE-001](#fr-image-001), [FR-IMAGE-002](#fr-image-002) |
| SN-041 | Investigators | Link entities across cases | [FR-LINK-001](#fr-link-001), [FR-LINK-002](#fr-link-002) |
| SN-042 | Investigators | Normalize documents before processing | [FR-PREPROC-001](#fr-preproc-001), [FR-PREPROC-002](#fr-preproc-002) |
| SN-043 | Investigators | Verify facts against multiple sources | [FR-VERIF-001](#fr-verif-001), [FR-VERIF-002](#fr-verif-002) |

---

## 7. Requirements Specification {#requirements}

### Functional Requirements

#### Evidence Management {#fr-evd}
- **FR-EVD-001** {#fr-evd-001}: The system SHALL link every extracted fact to its source evidence with an exact supporting quote. [SN-002](#stakeholder-needs)
- **FR-EVD-002** {#fr-evd-002}: The system MUST support the following file types: PDF, images (JPG, PNG, BMP), audio (MP3, WAV, M4A, MP4), text files (TXT, MD), and DOCX.

#### Processing {#fr-per}
- **FR-PER-001** {#fr-per-001}: The system MUST process evidence files through a two-stage pipeline: text extraction followed by LLM inference.
- **FR-INC-001** {#fr-inc-001}: The system SHALL support incremental processing, detecting new and modified files since the last run. [SN-005](#stakeholder-needs)
- **FR-INC-002** {#fr-inc-002}: The system MUST prioritize processing in this order: new files (highest), modified files, extracted-only files, rerun candidates.

#### Pipeline {#fr-plp}
- **FR-PLP-001** {#fr-plp-001}: The system MUST support multi-pass LLM pipelines where each pass can have different prompt templates and output schemas.
- **FR-PLP-002** {#fr-plp-002}: The system SHALL provide built-in pipelines for: Basic Facts, Financial Crimes, Document Analysis, Image OCR Analysis, and Audio Transcription.
- **FR-PLP-003** {#fr-plp-003}: The system MUST allow users to create custom pipelines with configurable passes.

#### Quality {#fr-qual}
- **FR-QUAL-001** {#fr-qual-001}: The system SHALL calculate and store quality scores for each extraction, including confidence, text coverage, entity density, and quote quality. [SN-006](#stakeholder-needs)
- **FR-QUAL-002** {#fr-qual-002}: The system MUST flag extractions with quality below 0.5 for manual review.

#### Deduplication {#fr-dedup}
- **FR-DEDUP-001** {#fr-dedup-001}: The system SHALL implement fact deduplication with configurable similarity threshold (default 0.85).
- **FR-DEDUP-002** {#fr-dedup-002}: The system MUST support multiple merge strategies: KeepHighestConfidence, KeepMostSevere, MergeAll.

#### Export {#fr-exp}
- **FR-EXP-001** {#fr-exp-001}: The system SHALL support export in JSON, CSV, PDF, and Excel formats. [SN-007](#stakeholder-needs)
- **FR-EXP-002** {#fr-exp-002}: JSON export MUST include: facts, entities, timeline, and quality metrics.

#### Search {#fr-srch}
- **FR-SRCH-001** {#fr-srch-001}: The system MUST provide full-text search across facts and entities using FTS5.
- **FR-SRCH-002** {#fr-srch-002}: The system SHALL support Boolean operators, phrase search, and filtered searches.

#### Evidence Chains {#fr-chain}
- **FR-CHAIN-001** {#fr-chain-001}: The system SHALL track relationships between evidence files: references, similar_to, duplicate_of, responds_to, derived_from. [SN-008](#stakeholder-needs)
- **FR-CHAIN-002** {#fr-chain-002}: The system MUST allow both automatic and manual evidence linking.

#### Annotation {#fr-ann}
- **FR-ANN-001** {#fr-ann-001}: The system SHALL support manual annotations on facts with types: review, note, flag, tag.
- **FR-ANN-002** {#fr-ann-002}: The system MUST provide default tags: reviewed, important, followup, questionable, confirmed.

#### Multi-Language {#fr-lang}
- **FR-LANG-001** {#fr-lang-001}: The system SHOULD support language detection and provide language-specific LLM prompts.
- **FR-LANG-002** {#fr-lang-002}: The system MUST support at minimum: English, Spanish, French, German, Portuguese, Italian, Chinese, Japanese, Korean, Arabic.

#### Project Management {#fr-proj}
- **FR-PROJ-001** {#fr-proj-001}: The system MUST save/load investigations as `.sls` project files.
- **FR-PROJ-002** {#fr-proj-002}: Project files MUST contain: investigator info, paths, model configuration, hardware settings, processing options, and metadata.

#### Case Comparison {#fr-cmp}
- **FR-CMP-001** {#fr-cmp-001}: The system SHALL support comparing facts and entities across multiple project files.
- **FR-CMP-002** {#fr-cmp-002}: The system MUST identify common entities appearing across compared projects.
- **FR-CMP-003** {#fr-cmp-003}: The system SHALL detect timeline overlap between cases.
- **FR-CMP-004** {#fr-cmp-004}: The system MUST provide geographic comparison of locations across cases.

#### Timeline Visualization {#fr-timeline}
- **FR-TIME-001** {#fr-timeline-001}: The system SHALL generate chronological timelines from extracted facts with dates.
- **FR-TIME-002** {#fr-timeline-002}: The system MUST allow filtering timeline by date range, category, and severity.
- **FR-TIME-003** {#fr-timeline-003}: The system MUST display facts on an interactive timeline with chronological ordering.
- **FR-TIME-004** {#fr-timeline-004}: The system SHALL support zoom in/out on the timeline (e.g., by day, week, month, year).
- **FR-TIME-005** {#fr-timeline-005}: The system SHALL support pan navigation across the timeline.
- **FR-TIME-006** {#fr-timeline-006}: The system MUST allow filtering timeline by category, severity, and entity.

#### Map Visualization {#fr-map}
- **FR-MAP-001** {#fr-map-001}: The system SHALL display evidence files and extracted locations on an interactive map.
- **FR-MAP-002** {#fr-map-002}: The system MUST support OpenStreetMap as the map provider.

#### Model Management {#fr-model}
- **FR-MODEL-001** {#fr-model-001}: The system MUST support downloading LLM models from Hugging Face.
- **FR-MODEL-002** {#fr-model-002}: The system SHALL allow users to select quantization level (Q4_K_M, Q5_K_S, Q8_0, F16).
- **FR-MODEL-003** {#fr-model-003}: The system MUST display model download progress and verify integrity.
- **NFR-MODEL-001** {#nfr-model-001}: Model files SHOULD be stored in a dedicated models directory within the project.

#### Faceted Search {#fr-facet}
- **FR-FACET-001** {#fr-facet-001}: The system SHALL support faceted search with filters for: category, severity, date range, confidence, quality.
- **FR-FACET-002** {#fr-facet-002}: The system MUST allow combining multiple facets with AND/OR logic.
- **FR-FACET-003** {#fr-facet-003}: The system SHALL display facet counts showing number of matching results.
- **FR-FACET-004** {#fr-facet-004}: The system MUST support saving facet filter combinations as presets.

#### Entity Resolution {#fr-entity-res}
- **FR-ER-001** {#fr-er-001}: The system SHALL match entities across different files to identify potential duplicates.
- **FR-ER-002** {#fr-er-002}: The system MUST support alias handling (e.g., "John Smith" = "J. Smith" = "John J. Smith").
- **FR-ER-003** {#fr-er-003}: The system SHALL link related entities automatically based on name variations.
- **FR-ER-004** {#fr-er-004}: The system MUST allow manual review and correction of entity resolution matches.

#### Analysis Features {#fr-analysis}
- **FR-ANALYSIS-001** {#fr-analysis-001}: The system SHALL perform fact frequency analysis to identify recurring themes.
- **FR-ANALYSIS-002** {#fr-analysis-002}: The system MUST detect patterns across evidence (e.g., same actors, locations, dates).
- **FR-ANALYSIS-003** {#fr-analysis-003}: The system SHALL flag anomalies or outliers in extracted facts.
- **FR-ANALYSIS-004** {#fr-analysis-004}: The system MUST provide statistical summaries: fact counts by category, entity distribution, timeline distribution.

#### Notifications {#fr-notif}
- **FR-NOTIF-001** {#fr-notif-001}: The system SHALL notify users when processing completes.
- **FR-NOTIF-002** {#fr-notif-002}: The system MUST alert users when errors occur during processing.
- **FR-NOTIF-003** {#fr-notif-003}: The system SHALL warn users when extractions fall below quality thresholds.
- **FR-NOTIF-004** {#fr-notif-004}: The system SHOULD support configurable notification preferences.

#### Data Validation {#fr-validation}
- **FR-VAL-001** {#fr-val-001}: The system SHALL verify extracted facts against source text for consistency.
- **FR-VAL-002** {#fr-val-002}: The system MUST perform cross-reference consistency checks across facts.
- **FR-VAL-003** {#fr-val-003}: The system SHALL flag contradictory facts for manual review.
- **FR-VAL-004** {#fr-val-004}: The system MUST validate that source quotes contain the fact summary text.

#### Temporal Analysis {#fr-temporal}
- **FR-TEMP-001** {#fr-temp-001}: The system SHALL analyze time-of-day patterns in documents to identify when communications occurred.
- **FR-TEMP-002** {#fr-temp-002}: The system MUST calculate date extraction accuracy scores based on context consistency.
- **FR-TEMP-003** {#fr-temp-003}: The system SHALL flag backdated or future-dated documents that appear inconsistent with their content.
- **FR-TEMP-004** {#fr-temp-004}: The system MUST provide temporal confidence based on date source (explicit date, relative date, inferred).

#### Network Analysis {#fr-network}
- **FR-NET-001** {#fr-net-001}: The system SHALL map communication patterns between extracted entities (e.g., sender-receiver relationships).
- **FR-NET-002** {#fr-net-002}: The system MUST identify communication hubs - entities that connect to many other entities.
- **FR-NET-003** {#fr-net-003}: The system SHALL visualize entity connections as an interactive network graph.
- **FR-NET-004** {#fr-net-004}: The system MUST calculate network metrics: degree centrality, betweenness, clustering coefficient.
- **FR-NET-005** {#fr-net-005}: The system SHALL detect communities within entity networks.

#### Anomaly Detection {#fr-anomaly}
- **FR-ANOM-001** {#fr-anom-001}: The system SHALL detect unusual file sizes or formats that deviate from the evidence set norm.
- **FR-ANOM-002** {#fr-anom-002}: The system MUST identify outlier facts based on severity or confidence scores.
- **FR-ANOM-003** {#fr-anom-003}: The system SHALL flag suspicious patterns (e.g., same fact repeated across many files, unusual entity co-occurrences).
- **FR-ANOM-004** {#fr-anom-004}: The system MUST provide anomaly scores with explanations for flagged items.

#### Evidence Weighting {#fr-weight}
- **FR-WEIGHT-001** {#fr-weight-001}: The system SHALL allow tagging facts as primary, secondary, or supplementary evidence.
- **FR-WEIGHT-002** {#fr-weight-002}: The system MUST allow weighting evidence by source reliability (e.g., official document vs. hearsay).
- **FR-WEIGHT-003** {#fr-weight-003}: The system SHALL calculate aggregate confidence scores for conclusions based on weighted evidence.
- **FR-WEIGHT-004** {#fr-weight-004}: The system MUST support custom weighting schemes configurable by investigators.

#### Source Attribution {#fr-source}
- **FR-SOURCE-001** {#fr-source-001}: The system SHALL track which LLM pass extracted each fact.
- **FR-SOURCE-002** {#fr-source-002}: The system MUST store the model version and configuration used for each extraction.
- **FR-SOURCE-003** {#fr-source-003}: The system SHALL enable reproducible extraction by allowing selection of specific model versions.
- **FR-SOURCE-004** {#fr-source-004}: The system MUST log extraction parameters (temperature, max tokens, prompt) for audit purposes.

#### Metadata Extraction {#fr-meta}
- **FR-META-001** {#fr-meta-001}: The system SHALL extract EXIF metadata from image files (camera, date, GPS, software).
- **FR-META-002** {#fr-meta-002}: The system MUST extract document metadata (author, creation date, modification date, software).
- **FR-META-003** {#fr-meta-003}: The system SHALL track which software and version created each document.
- **FR-META-004** {#fr-meta-004}: The system MUST store all extracted metadata in structured format for analysis.

#### Structured Data Extraction {#fr-structured}
- **FR-STRUCT-001** {#fr-struct-001}: The system SHALL extract tables from PDF documents with column/row structure.
- **FR-STRUCT-002** {#fr-struct-002}: The system MUST parse form fields and key-value pairs from documents.
- **FR-STRUCT-003** {#fr-struct-003}: The system SHALL identify relationships between extracted structured data elements.
- **FR-STRUCT-004** {#fr-struct-004}: The system MUST export structured data in CSV/JSON formats.

#### Audio/Video Analysis {#fr-media}
- **FR-MEDIA-001** {#fr-media-001}: The system SHALL perform speaker identification in audio/video files.
- **FR-MEDIA-002** {#fr-media-002}: The system MUST detect language in audio content.
- **FR-MEDIA-003** {#fr-media-003}: The system SHALL align transcripts with audio timestamps.
- **FR-MEDIA-004** {#fr-media-004}: The system MUST correlate timestamps across multiple media files.

#### Image Analysis {#fr-image}
- **FR-IMAGE-001** {#fr-image-001}: The system SHALL detect manipulated or photoshopped images.
- **FR-IMAGE-002** {#fr-image-002}: The system SHALL extract embedded text from images using OCR.
- **FR-IMAGE-003** {#fr-image-003}: The system MUST classify images into categories (document, photo, screenshot, etc.).
- **FR-IMAGE-004** {#fr-image-004}: The system SHALL perform object detection to identify items in images.

#### Link Analysis {#fr-link}
- **FR-LINK-001** {#fr-link-001}: The system SHALL cross-reference entities across different cases.
- **FR-LINK-002** {#fr-link-002}: The system MUST correlate entities by timeline (same time period).
- **FR-LINK-003** {#fr-link-003}: The system SHALL cluster entities by location proximity.
- **FR-LINK-004** {#fr-link-004}: The system MUST identify recurring patterns in entity relationships.
- **FR-LINK-005** {#fr-link-005}: The system SHALL provide visual link charts for investigation.

#### Pre-processing {#fr-preproc}
- **FR-PREPROC-001** {#fr-preproc-001}: The system SHALL normalize documents to standard format before processing.
- **FR-PREPROC-002** {#fr-preproc-002}: The system MUST detect language of input text automatically.
- **FR-PREPROC-003** {#fr-preproc-003}: The system SHALL detect character encoding and convert to UTF-8.
- **FR-PREPROC-004** {#fr-preproc-004}: The system MUST convert between file formats where necessary.

#### Verification & QC {#fr-verif}
- **FR-VERIF-001** {#fr-verif-001}: The system SHALL cross-validate facts against multiple source documents.
- **FR-VERIF-002** {#fr-verif-002}: The system MUST calculate consensus confidence scores when same fact appears in multiple sources.
- **FR-VERIF-003** {#fr-verif-003}: The system SHALL highlight disagreements between sources for manual review.
- **FR-VERIF-004** {#fr-verif-004}: The system MUST provide verification status: unverified, verified, disputed.

### Non-Functional Requirements {#nfr}

#### Privacy {#nfr-priv}
- **NFR-PRIV-001** {#nfr-priv-001}: All processing MUST run locally without cloud dependencies. [SN-001](#stakeholder-needs)

#### Forensic Integrity {#nfr-forensic}
- **NFR-FOR-001** {#nfr-forensic-001}: The system MUST treat all original evidence files as read-only. Original files SHALL never be modified or deleted.
- **NFR-FOR-002** {#nfr-forensic-002}: All processing operations MUST be non-destructive. The system SHALL only create new derived data (extracted text, facts, annotations) without altering source files.
- **NFR-FOR-003** {#nfr-forensic-003}: The system MUST preserve file timestamps and metadata of original evidence files.
- **NFR-FOR-004** {#nfr-forensic-004}: The system SHALL store all derived data in separate directories from original evidence.
- **NFR-FOR-005** {#nfr-forensic-005}: When evidence is removed from a project, original files on disk MUST remain untouched.
- **NFR-FOR-006** {#nfr-forensic-006}: Database operations MUST use soft deletes (marking records as deleted) rather than hard deletes to preserve audit trail.

#### Performance {#nfr-per}
- **NFR-PER-001** {#nfr-per-001}: The system MUST be implemented in Rust for maximum throughput.
- **NFR-PER-002** {#nfr-per-002}: The system SHOULD implement batch size tuning to optimize processing time.
- **NFR-PER-003** {#nfr-per-003}: The system SHALL auto-detect available hardware (GPU, CPU cores) and scale processing accordingly. [SN-009](#stakeholder-needs)

#### Platform {#nfr-plat}
- **NFR-PLAT-001** {#nfr-plat-001}: The system MUST support Windows, macOS, and Linux via Tauri. [SN-004](#stakeholder-needs)

#### Reliability {#nfr-rel}
- **NFR-REL-001** {#nfr-rel-001}: The system MUST implement job checkpointing to resume interrupted processing.
- **NFR-REL-002** {#nfr-rel-002}: The system MUST implement an error queue with exponential backoff retry logic (max 3 retries).
- **NFR-REL-003** {#nfr-rel-003}: The system SHOULD mark jobs exceeding max retries for manual review.
- **NFR-AUDIT-001** {#nfr-audit-001}: The system MUST maintain an audit log of all user actions for forensic chain of custody.

#### Usability {#nfr-use}
- **NFR-USE-001** {#nfr-use-001}: The system MUST provide a dashboard with hardware status and quick actions.
- **NFR-USE-002** {#nfr-use-002}: The system SHALL display quality indicators with color-coded badges: green (>=0.7), yellow (0.5-0.7), red (<0.5).
- **NFR-USE-003** {#nfr-use-003}: The system SHOULD support undo/redo for annotation operations.
- **NFR-USE-004** {#nfr-use-004}: The system SHALL support bulk selection and bulk operations on facts and entities.
- **NFR-USE-005** {#nfr-use-005}: The system SHOULD provide keyboard shortcuts for common operations.

#### Security {#nfr-sec}
- **NFR-SEC-001** {#nfr-sec-001}: The system MUST securely delete extracted text files when removed from project.
- **NFR-SEC-002** {#nfr-sec-002}: The system SHALL sanitize memory after processing sensitive data.

#### Data Management {#nfr-data}
- **NFR-DATA-001** {#nfr-data-001}: The system SHOULD support project backup to ZIP archive.
- **NFR-DATA-002** {#nfr-data-002}: The system MUST support restoring project from backup archive.
- **NFR-DATA-003** {#nfr-data-003}: The system SHALL perform database integrity checks on startup.
- **NFR-DATA-004** {#nfr-data-004}: The system SHOULD support data migration between project versions.

#### Integration {#nfr-int}
- **NFR-INT-001** {#nfr-int-001}: The system SHOULD provide a command-line interface (CLI) for automation.
- **NFR-INT-002** {#nfr-int-002}: The system MAY support a plugin/extension system for custom extractors or pipelines.

#### Advanced Processing {#nfr-adv}
- **NFR-ADV-001** {#nfr-adv-001}: The system SHOULD support scheduled automated processing runs.
- **NFR-ADV-002** {#nfr-adv-002}: The system MUST implement parallel file processing for extraction.
- **NFR-ADV-003** {#nfr-adv-003}: The system SHALL implement intelligent chunking for large files exceeding context limits.

#### Validation & Review {#nfr-valid}
- **NFR-VALID-001** {#nfr-valid-001}: The system SHALL support a fact verification workflow with status: unverified, confirmed, disputed.
- **NFR-VALID-002** {#nfr-valid-002}: The system SHOULD support human-in-the-loop review for low-confidence extractions.
- **NFR-VALID-003** {#nfr-valid-003}: The system SHALL provide confidence calibration based on user feedback.

#### Performance Tuning {#nfr-perf}
- **NFR-PERF-001** {#nfr-perf-001}: The system SHOULD implement in-memory caching for frequently accessed data.
- **NFR-PERF-002** {#nfr-perf-002}: The system SHALL allow configuration of memory limits per operation.
- **NFR-PERF-003** {#nfr-perf-003}: The system MUST allow configuration of maximum batch size.

#### File Handling {#nfr-file}
- **NFR-FILE-001** {#nfr-file-001}: The system MUST handle corrupted files gracefully without crashing.
- **NFR-FILE-002** {#nfr-file-002}: The system SHALL define maximum file size limits with user-configurable threshold.
- **NFR-FILE-003** {#nfr-file-003}: The system MUST skip unsupported file types with warning logged.
- **NFR-FILE-004**: The system MUST validate files by magic bytes before processing, not just extension.
- **NFR-FILE-005**: The system MUST reject files with dangerous content types.

#### Security {#nfr-sec}
- **NFR-SEC-003** {#nfr-sec-003}: The system MUST use parameterized queries to prevent SQL injection.
- **NFR-SEC-004**: The system MUST sanitize file paths to prevent path traversal attacks.
- **NFR-SEC-005**: The system MUST NOT log sensitive data (passwords, API keys, evidence content).
- **NFR-SEC-006**: The system MUST implement memory limits for LLM inference to prevent exhaustion.

#### Operational {#nfr-ops}
- **NFR-OPS-001** {#nfr-ops-001}: The system MUST provide structured logging with configurable levels.
- **NFR-OPS-002**: The system MUST export diagnostic information for debugging.
- **NFR-OPS-003**: The system MUST perform database integrity validation on startup.

---

# Part III: Technical Specification

## 8. System Architecture

```
+-------------------------------------------------------------+
|                   SvelteKit Frontend                        |
|  Dashboard | Analysis | Pipeline | Results | Map | Settings |
+----------------------------+--------------------------------+
                             | Tauri Commands (IPC)
+----------------------------v--------------------------------+
|                      Rust Backend                           |
|  +-------------+  +-------------+  +--------------------+   |
|  |   Text      |  |    LLM      |  |   Database         |   |
|  | Extraction  |→ | Pipeline    |→ |   Manager          |   |
|  | (Separate)  |  | (Multi-pass)|  |   (SQLite)         |   |
|  +-------------+  +-------------+  +--------------------+   |
|  +-------------+  +-------------+  +--------------------+   |
|  | Text Cache  |  | Deduplicator|  | Job Checkpoints    |   |
|  | (.txt files)|  |             |  |                    |   |
|  +-------------+  +-------------+  +--------------------+   |
|  +-------------+  +-------------+  +--------------------+   |
|  |GPU Detection|  |  Profiler   |  | Error Queue        |   |
|  +-------------+  +-------------+  +--------------------+   |
+-------------------------------------------------------------+
```

### Processing Priority Order
1. **New files** - Never processed before (highest priority)
2. **Modified files** - Changed since last extraction
3. **Extracted only** - Text cached, no inference
4. **Rerun candidates** - Already processed, for accuracy improvement

---

## 9. Technology Stack

### Frontend
- **SvelteKit** with TypeScript
- **Tauri 2** for native desktop integration
- **SVG Icons** (no emojis)
- **Leaflet.js** for map visualization (OpenStreetMap)

### Backend (Rust)
| Component | Crate | Purpose |
|-----------|-------|---------|
| PDF Extract | `pdf-extract` | Fast PDF text extraction |
| OCR | `ocrs` | Pure Rust OCR engine |
| Audio | `whisper-rs` | Whisper.cpp Rust bindings |
| LLM | `llama_cpp` | Local GGUF inference |
| Database | `rusqlite` | SQLite operations |
| Hardware | `sysinfo` | System probing |
| HTTP | `reqwest` | Model downloads |
| Serialization | `serde` | JSON handling |

---

## 10. Database Schema

### Registry DB
```sql
CREATE TABLE registry (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    fingerprint TEXT NOT NULL UNIQUE,
    path TEXT NOT NULL,
    file_size INTEGER,
    file_type TEXT,
    file_name TEXT,
    -- Incremental processing fields
    last_modified DATETIME,           -- File modification time
    last_hash_check DATETIME,        -- When we verified hash
    -- Processing status
    has_extracted_text BOOLEAN DEFAULT FALSE,
    extracted_at DATETIME,
    processed_at DATETIME,
    processed BOOLEAN DEFAULT 0,
    processing_priority INTEGER DEFAULT 0,  -- 0=new, 1=modified, 2=extracted, 3=rerun
    retry_count INTEGER DEFAULT 0,
    -- Quality metrics
    extraction_quality REAL,          -- 0.0-1.0
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
-- Indexes
CREATE INDEX idx_registry_fingerprint ON registry(fingerprint);
CREATE INDEX idx_registry_filetype ON registry(file_type);
CREATE INDEX idx_registry_processed ON registry(processed);
CREATE INDEX idx_registry_priority ON registry(processing_priority);
CREATE INDEX idx_registry_modified ON registry(last_modified);
```

### Text Cache (Extracted Text Files)
```sql
-- Text is stored as .txt files alongside originals
-- Schema tracks what's cached
CREATE TABLE text_cache (
    fingerprint TEXT PRIMARY KEY,
    text_file_path TEXT NOT NULL,
    char_count INTEGER,
    text_hash TEXT,                    -- Hash of text for change detection
    extraction_method TEXT,            -- pdf, ocr, audio
    extraction_quality REAL,           -- Quality score 0-1
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME
);
CREATE INDEX idx_textcache_hash ON text_cache(text_hash);
```

### Intelligence DB (Facts)
```sql
CREATE TABLE intelligence (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    registry_id INTEGER NOT NULL,
    fingerprint TEXT NOT NULL,
    filename TEXT NOT NULL,
    -- Evidence linking (forensic rigor)
    source_quote TEXT NOT NULL,       -- Exact text supporting the fact (REQUIRED)
    page_number INTEGER,              -- Page in source document
    evidence_full TEXT,               -- Full extracted text chunk
    evidence_hash TEXT,               -- Hash for deduplication
    -- Extracted fact
    associated_date TEXT,
    fact_summary TEXT NOT NULL,
    category TEXT,
    identified_crime TEXT,
    severity_score INTEGER DEFAULT 1,
    confidence REAL,                  -- LLM confidence score
    quality_score REAL,               -- Overall extraction quality
    -- Multi-language support
    source_language TEXT,             -- ISO 639-1 language code
    translated_quote TEXT,            -- Translated quote if enabled
    -- Pipeline tracking
    pipeline_id TEXT,
    pass_name TEXT,
    -- Soft delete for forensic integrity (NFR-FOR-006)
    is_deleted BOOLEAN DEFAULT FALSE,
    deleted_at DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
-- Indexes
CREATE INDEX idx_intelligence_fingerprint ON intelligence(fingerprint);
CREATE INDEX idx_intelligence_category ON intelligence(category);
CREATE INDEX idx_intelligence_date ON intelligence(associated_date);
CREATE INDEX idx_intelligence_evidence_hash ON intelligence(evidence_hash);
CREATE INDEX idx_intelligence_severity ON intelligence(severity_score DESC);
CREATE INDEX idx_intelligence_quality ON intelligence(quality_score);
CREATE INDEX idx_intelligence_deleted ON intelligence(is_deleted) WHERE is_deleted = FALSE;
```

### Entities Table (Named Entity Recognition)
```sql
CREATE TABLE entities (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    intelligence_id INTEGER,
    fingerprint TEXT NOT NULL,
    entity_type TEXT NOT NULL,       -- person, location, date, money, organization, phone, email
    value TEXT NOT NULL,
    normalized_value TEXT,
    confidence REAL,
    position_start INTEGER,
    position_end INTEGER,
    -- Pipeline tracking
    pipeline_id TEXT,
    pass_name TEXT,
    -- Soft delete for forensic integrity (NFR-FOR-006)
    is_deleted BOOLEAN DEFAULT FALSE,
    deleted_at DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_entities_fingerprint ON entities(fingerprint);
CREATE INDEX idx_entities_type ON entities(entity_type);
CREATE INDEX idx_entities_value ON entities(value);
CREATE INDEX idx_entities_pipeline ON entities(pipeline_id, pass_name);
CREATE INDEX idx_entities_deleted ON entities(is_deleted) WHERE is_deleted = FALSE;
```

### LLM Passes Table (Multi-pass Pipeline)
```sql
CREATE TABLE llm_passes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    fingerprint TEXT NOT NULL,
    pipeline_id TEXT,
    pass_name TEXT NOT NULL,         -- "facts", "entities", "summary"
    input_text TEXT,                 -- Text sent to LLM (or hash)
    input_hash TEXT,                 -- Hash of input for caching
    output_json TEXT NOT NULL,       -- Raw LLM response
    processing_time_ms INTEGER,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_passes_fingerprint ON llm_passes(fingerprint);
CREATE INDEX idx_passes_pipeline ON llm_passes(pipeline_id);
CREATE INDEX idx_passes_input_hash ON llm_passes(input_hash);
```

### Job Checkpoints (Progress Persistence)
```sql
CREATE TABLE job_checkpoints (
    id TEXT PRIMARY KEY,
    job_type TEXT NOT NULL,          -- "registry_scan", "extraction", "pipeline"
    pipeline_id TEXT,
    status TEXT NOT NULL,            -- "running", "paused", "completed", "failed"
    -- Progress tracking
    total_items INTEGER,
    processed_items INTEGER,
    current_item_id TEXT,
    -- State serialization
    state_json TEXT,
    -- Timing
    started_at DATETIME NOT NULL,
    updated_at DATETIME,
    completed_at DATETIME,
    error_message TEXT
);
CREATE INDEX idx_job_status ON job_checkpoints(status);
CREATE INDEX idx_job_type ON job_checkpoints(job_type);
```

### Error Queue (Retry Logic)
```sql
CREATE TABLE error_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    fingerprint TEXT NOT NULL,
    job_type TEXT NOT NULL,          -- "extraction", "inference"
    error_message TEXT NOT NULL,
    error_details TEXT,               -- Full stack trace
    retry_count INTEGER DEFAULT 0,
    max_retries INTEGER DEFAULT 3,
    last_attempt DATETIME,
    next_attempt DATETIME,
    resolved BOOLEAN DEFAULT FALSE,
    resolution TEXT,                  -- "retry_success", "manual_resolved", "skipped"
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_error_fingerprint ON error_queue(fingerprint);
CREATE INDEX idx_error_pending ON error_queue(resolved, next_attempt);
```

### Profiling / Performance Metrics
```sql
CREATE TABLE profiling_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id TEXT NOT NULL UNIQUE,
    job_type TEXT NOT NULL,
    started_at DATETIME NOT NULL,
    completed_at DATETIME,
    total_files INTEGER,
    sampled_files INTEGER,
    full_dataset BOOLEAN DEFAULT FALSE,
    -- Phase timing in JSON
    phase_stats TEXT,                 -- { "hashing": {avg_ms, min, max, count}, ... }
    bottleneck_phase TEXT,
    errors_json TEXT,
    export_data TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_profiling_run_id ON profiling_runs(run_id);
```

### Audit Log (Chain of Custody)
```sql
CREATE TABLE audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    user_name TEXT,
    action TEXT NOT NULL,           -- "file_imported", "extraction_completed", "pipeline_run", "fact_reviewed", "annotation_added", "export_created", etc.
    target_type TEXT,               -- "file", "fact", "entity", "project", "pipeline"
    target_id TEXT,
    details_json TEXT,              -- Additional action details
    ip_address TEXT,
    session_id TEXT
);
CREATE INDEX idx_audit_timestamp ON audit_log(timestamp);
CREATE INDEX idx_audit_action ON audit_log(action);
CREATE INDEX idx_audit_target ON audit_log(target_type, target_id);
```

### Pipelines Table
```sql
CREATE TABLE pipelines (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    is_builtin BOOLEAN DEFAULT FALSE,
    is_file_type_specific BOOLEAN DEFAULT FALSE,  -- Different per file type
    file_type_filter TEXT,            -- Apply only to: pdf, image, audio, doc
    passes_json TEXT NOT NULL,        -- Array of PipelinePass configs
    sample_size INTEGER,              -- Default sample size
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    modified_at DATETIME
);
```

### Evidence Chains Table
```sql
CREATE TABLE evidence_chains (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    chain_name TEXT NOT NULL,
    fingerprint TEXT NOT NULL,
    relationship TEXT NOT NULL,  -- "references", "similar_to", "duplicate_of", "responds_to", "derived_from"
    relationship_strength REAL,    -- 0.0-1.0 confidence in relationship
    chain_order INTEGER,          -- Order in chain
    manual BOOLEAN DEFAULT FALSE, -- TRUE if manually linked
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_chains_fingerprint ON evidence_chains(fingerprint);
CREATE INDEX idx_chains_name ON evidence_chains(chain_name);

CREATE TABLE chain_summaries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    chain_name TEXT NOT NULL UNIQUE,
    summary TEXT,
    key_entities JSON,
    key_facts JSON,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### Case Comparisons Table
```sql
CREATE TABLE case_comparisons (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    projects_json TEXT NOT NULL,  -- Array of project paths/IDs
    common_entities JSON,
    matching_facts JSON,
    common_dates JSON,
    statistics_json TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    created_by TEXT
);
CREATE INDEX idx_comparisons_name ON case_comparisons(name);
```

### Fact Annotations Table
```sql
CREATE TABLE fact_annotations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    intelligence_id INTEGER NOT NULL,
    annotation_text TEXT NOT NULL,
    annotation_type TEXT NOT NULL,  -- "review", "note", "flag", "tag"
    tags_json TEXT,
    annotated_by TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME
);
CREATE INDEX idx_annotations_fact ON fact_annotations(intelligence_id);

CREATE TABLE annotation_tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    color TEXT NOT NULL,
    description TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
-- Default tags
INSERT INTO annotation_tags (name, color, description) VALUES
    ('reviewed', '#22C55E', 'Fact has been reviewed'),
    ('important', '#EF4444', 'Important fact'),
    ('followup', '#F59E0B', 'Requires followup'),
    ('questionable', '#6B7280', 'Fact may be inaccurate'),
    ('confirmed', '#3B82F6', 'Fact confirmed by investigator');
```

### Entity Aliases Table (Entity Resolution)
```sql
CREATE TABLE entity_aliases (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    canonical_entity_id INTEGER NOT NULL,
    alias_value TEXT NOT NULL,
    confidence REAL DEFAULT 1.0,
    is_manual BOOLEAN DEFAULT FALSE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_aliases_canonical ON entity_aliases(canonical_entity_id);
CREATE INDEX idx_aliases_value ON entity_aliases(alias_value);

CREATE TABLE entity_resolution (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_a_id INTEGER NOT NULL,
    entity_b_id INTEGER NOT NULL,
    resolution_type TEXT NOT NULL,  -- "same_as", "related", "possible_match"
    confidence REAL,
    status TEXT DEFAULT "pending",  -- "pending", "confirmed", "rejected"
    resolved_by TEXT,               -- "auto" or user name
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_resolve_a ON entity_resolution(entity_a_id);
CREATE INDEX idx_resolve_b ON entity_resolution(entity_b_id);
```

### Notifications Preferences Table
```sql
CREATE TABLE notification_preferences (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_name TEXT,
    notify_on_complete BOOLEAN DEFAULT TRUE,
    notify_on_error BOOLEAN DEFAULT TRUE,
    notify_on_quality_warning BOOLEAN DEFAULT TRUE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME
);
```

### Fact Validation Table
```sql
CREATE TABLE fact_validations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    intelligence_id INTEGER NOT NULL,
    validation_type TEXT NOT NULL,  -- "source_check", "consistency_check", "contradiction_check"
    is_valid BOOLEAN,
    details TEXT,
    checked_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_validations_fact ON fact_validations(intelligence_id);
```

### Evidence Weighting Table
```sql
CREATE TABLE evidence_weights (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    intelligence_id INTEGER NOT NULL,
    weight_type TEXT NOT NULL,        -- "primary", "secondary", "supplementary", "custom"
    reliability_score REAL DEFAULT 1.0,  -- 0.0-1.0 source reliability
    custom_weight REAL,               -- Custom weight if weight_type = "custom"
    weighted_confidence REAL,         -- Calculated weighted confidence
    set_by TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME
);
CREATE INDEX idx_weights_fact ON evidence_weights(intelligence_id);
```

### Source Attribution Table
```sql
CREATE TABLE extraction_sources (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    intelligence_id INTEGER NOT NULL,
    pipeline_id TEXT NOT NULL,
    pass_name TEXT NOT NULL,
    model_id TEXT NOT NULL,
    model_version TEXT,
    quantization TEXT,
    temperature REAL,
    max_tokens INTEGER,
    prompt_hash TEXT,
    extraction_hash TEXT,              -- Hash of output for reproducibility
    extraction_config_json TEXT,       -- Full config snapshot
    extracted_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_sources_fact ON extraction_sources(intelligence_id);
CREATE INDEX idx_sources_model ON extraction_sources(model_id, model_version);

CREATE TABLE model_versions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    model_id TEXT NOT NULL UNIQUE,
    version TEXT NOT NULL,
    source TEXT NOT NULL,              -- "huggingface", "local"
    local_path TEXT,
    quantization TEXT,
    context_length INTEGER,
    downloaded_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    is_active BOOLEAN DEFAULT TRUE
);
```

### Anomaly Detection Table
```sql
CREATE TABLE anomaly_detections (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    anomaly_type TEXT NOT NULL,        -- "file_size", "file_format", "fact_outlier", "pattern"
    target_type TEXT NOT NULL,         -- "file", "fact", "entity"
    target_id TEXT NOT NULL,
    anomaly_score REAL NOT NULL,       -- 0.0-1.0
    details TEXT,                      -- JSON with anomaly specifics
    is_resolved BOOLEAN DEFAULT FALSE,
    resolved_at DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_anomaly_type ON anomaly_detections(anomaly_type);
CREATE INDEX idx_anomaly_target ON anomaly_detections(target_type, target_id);
```

### Anomaly Detection Algorithms

#### File Size Anomaly Detection
```rust
fn detect_file_size_anomalies(files: &[FileInfo]) -> Vec<Anomaly> {
    if files.is_empty() {
        return Vec::new();
    }
    
    let sizes: Vec<u64> = files.iter().map(|f| f.size).collect();
    let n = sizes.len() as f64;
    let mean = sizes.iter().sum::<u64>() as f64 / n;
    
    // Handle edge case: all files same size
    let variance = sizes.iter().map(|s| (*s as f64 - mean).powi(2)).sum::<f64>() / n;
    let stddev = variance.sqrt();
    
    if stddev == 0.0 {
        return Vec::new(); // No anomalies if all files same size
    }
    
    let threshold = 2.5;
    
    files.iter()
        .filter(|f| {
            let z_score = (f.size as f64 - mean).abs() / stddev;
            z_score > threshold
        })
        .map(|f| {
            let z_score = (f.size as f64 - mean).abs() / stddev;
            Anomaly {
                anomaly_type: "file_size",
                target_id: f.fingerprint.clone(),
                score: (z_score / 5.0).min(1.0), // Normalize to 0-1
                details: format!("File size {} bytes (z={:.2})", f.size, z_score),
            }
        })
        .collect()
}
```

#### Fact Outlier Detection
```rust
fn detect_fact_outliers(facts: &[Fact]) -> Vec<Anomaly> {
    if facts.len() < 4 {
        return Vec::new(); // Need at least 4 facts for IQR
    }
    
    // Use IQR method for severity outliers
    let mut severities: Vec<i32> = facts.iter().map(|f| f.severity_score).collect();
    severities.sort();
    let n = severities.len();
    
    let q1 = severities[n / 4];
    let q3 = severities[n * 3 / 4];
    let iqr = (q3 - q1) as f64;
    let lower = (q1 as f64 - 1.5 * iqr).max(0.0) as i32; // Severity can't be negative
    let upper = (q3 as f64 + 1.5 * iqr).min(10.0) as i32; // Cap at max severity
    
    facts.iter()
        .filter(|f| f.severity_score < lower || f.severity_score > upper)
        .map(|f| {
            let deviation = if f.severity_score < lower {
                (lower - f.severity_score) as f64
            } else {
                (f.severity_score - upper) as f64
            };
            Anomaly {
                anomaly_type: "fact_outlier",
                target_id: f.id.to_string(),
                score: (deviation / 5.0).min(1.0),
                details: format!("Severity {} outside normal range [{}, {}]", f.severity_score, lower, upper),
            }
        })
        .collect()
}
```

#### Suspicious Pattern Detection
```rust
fn detect_suspicious_patterns(facts: &[Fact]) -> Vec<Anomaly> {
    if facts.is_empty() {
        return Vec::new();
    }
    
    let mut anomalies = Vec::new();
    
    // Detect: same fact repeated across many files (using manual group_by)
    let mut fact_groups: HashMap<String, Vec<&Fact>> = HashMap::new();
    for fact in facts {
        if let Some(hash) = &fact.evidence_hash {
            fact_groups.entry(hash.clone()).or_default().push(fact);
        }
    }
    
    for (hash, group) in fact_groups.iter().filter(|(_, g)| g.len() > 5) {
        anomalies.push(Anomaly {
            anomaly_type: "pattern",
            target_id: hash.clone(),
            score: (group.len() as f64 / 20.0).min(1.0),
            details: format!("Fact repeated in {} files - possible copy-paste", group.len()),
        });
    }
    
    // Detect: unusual entity co-occurrences
    let entity_pairs = count_entity_cooccurrences(facts);
    if entity_pairs.is_empty() {
        return anomalies;
    }
    
    let total_pairs: usize = entity_pairs.values().sum();
    let mean_pairs = total_pairs as f64 / entity_pairs.len() as f64;
    
    for ((e1, e2), count) in entity_pairs.iter().filter(|(_, c)| **c as f64 > mean_pairs * 3.0) {
        anomalies.push(Anomaly {
            anomaly_type: "pattern",
            target_id: format!("{}->{}", e1, e2),
            score: (*count as f64 / (mean_pairs * 5.0)).min(1.0),
            details: format!("Unusual co-occurrence: {} times", count),
        });
    }
    
    anomalies
}

fn count_entity_cooccurrences(facts: &[Fact]) -> HashMap<(i32, i32), usize> {
    let mut pairs: HashMap<(i32, i32), usize> = HashMap::new();
    
    // Build entity index per fact
    let mut fact_entities: HashMap<i32, Vec<i32>> = HashMap::new();
    for fact in facts {
        let entity_ids: Vec<i32> = fact.entity_ids.clone();
        fact_entities.insert(fact.id, entity_ids);
    }
    
    // Count co-occurrences
    for entities in fact_entities.values() {
        let mut sorted = entities.clone();
        sorted.sort();
        sorted.dedup();
        
        for i in 0..sorted.len() {
            for j in (i + 1)..sorted.len() {
                let pair = (sorted[i], sorted[j]);
                *pairs.entry(pair).or_insert(0) += 1;
            }
        }
    }
    
    pairs
}
```

### Network Analysis Tables
```sql
CREATE TABLE entity_relationships (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_entity_id INTEGER NOT NULL,
    target_entity_id INTEGER NOT NULL,
    relationship_type TEXT NOT NULL,  -- "communicated_with", "mentioned", "located_at"
    strength REAL DEFAULT 1.0,         -- 0.0-1.0
    evidence_count INTEGER DEFAULT 1,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_rel_source ON entity_relationships(source_entity_id);
CREATE INDEX idx_rel_target ON entity_relationships(target_entity_id);

CREATE TABLE network_metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_id INTEGER NOT NULL,
    metric_date DATE NOT NULL,
    degree_centrality REAL,
    betweenness_centrality REAL,
    clustering_coefficient REAL,
    community_id INTEGER,
    computed_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_metrics_entity ON network_metrics(entity_id, metric_date);
```

### Network Analysis Algorithms

#### Building Entity Relationships
```rust
fn build_entity_relationships(facts: &[Fact]) -> Vec<EntityRelationship> {
    let mut relationships: HashMap<(i32, i32), EntityRelationship> = HashMap::new();
    
    for fact in facts {
        let mut entity_ids = fact.entity_ids.clone();
        entity_ids.sort();
        entity_ids.dedup();
        
        // Create relationships between all pairs in this fact
        for i in 0..entity_ids.len() {
            for j in (i + 1)..entity_ids.len() {
                let (e1, e2) = (entity_ids[i], entity_ids[j]);
                let key = (e1.min(e2), e1.max(e2)); // Normalize direction
                
                relationships.entry(key).and_modify(|r| {
                    r.evidence_count += 1;
                    r.strength = (r.evidence_count as f64 / facts.len() as f64).min(1.0);
                }).or_insert(EntityRelationship {
                    source_entity_id: e1,
                    target_entity_id: e2,
                    relationship_type: "mentioned_together".to_string(),
                    strength: 1.0 / facts.len() as f64,
                    evidence_count: 1,
                });
            }
        }
    
    relationships.into_iter().map(|(_k, v)| v).collect()
}
```

#### Degree Centrality
```rust
fn calculate_degree_centrality(edges: &[EntityRelationship]) -> HashMap<i32, f64> {
    let mut degree: HashMap<i32, usize> = HashMap::new();
    
    // Build adjacency once - O(E)
    for edge in edges {
        *degree.entry(edge.source_entity_id).or_insert(0) += 1;
        *degree.entry(edge.target_entity_id).or_insert(0) += 1;
    }
    
    let n = degree.len();
    if n <= 1 {
        return degree.into_iter().map(|(k, _)| (k, 0.0)).collect();
    }
    
    let max_degree = (n - 1) as f64;
    
    degree.into_iter()
        .map(|(node, d)| (node, d as f64 / max_degree))
        .collect()
}
```

#### Betweenness Centrality (Brandes Algorithm)
```rust
fn calculate_betweenness(edges: &[EntityRelationship], nodes: &[i32]) -> HashMap<i32, f64> {
    let mut betweenness: HashMap<i32, f64> = nodes.iter().map(|&n| (n, 0.0)).collect();
    let n = nodes.len() as f64;
    
    // Build adjacency list for efficient traversal
    let adj: HashMap<i32, Vec<i32>> = {
        let mut m: HashMap<i32, Vec<i32>> = HashMap::new();
        for edge in edges {
            m.entry(edge.source_entity_id).or_default().push(edge.target_entity_id);
            m.entry(edge.target_entity_id).or_default().push(edge.source_entity_id);
        }
        m
    };
    
    for &s in nodes {
        // BFS from source s
        let (stack, predecessors, distances, sigma) = bfs_brandes(&adj, s, nodes);
        
        // Accumulation
        let mut delta: HashMap<i32, f64> = nodes.iter().map(|&n| (n, 0.0)).collect();
        
        while let Some(w) = stack.pop() {
            for &v in &predecessors[&w] {
                delta.insert(v, delta[&v] + (sigma[&v] as f64 / sigma[&w] as f64) * (1.0 + delta[&w]));
            }
            if w != s {
                betweenness.insert(w, betweenness[&w] + delta[&w]);
            }
        }
    }
    
    // Normalize
    let norm = if n > 2.0 { 2.0 / (n * (n - 1.0)) } else { 1.0 };
    betweenness.iter_mut().map(|(&k, &v)| (k, v * norm)).collect()
}

fn bfs_brandes(adj: &HashMap<i32, Vec<i32>>, source: i32, nodes: &[i32]) -> (Vec<i32>, HashMap<i32, Vec<i32>>, HashMap<i32, i32>, HashMap<i32, i32>) {
    let mut stack = Vec::new();
    let mut predecessors: HashMap<i32, Vec<i32>> = HashMap::new();
    let mut distances: HashMap<i32, i32> = HashMap::new();
    let mut sigma: HashMap<i32, i32> = HashMap::new();
    
    for &n in nodes {
        distances.insert(n, -1);
        sigma.insert(n, 0);
        predecessors.insert(n, Vec::new());
    }
    
    distances.insert(source, 0);
    sigma.insert(source, 1);
    
    let mut queue = vec![source];
    
    while let Some(v) = queue.pop() {
        stack.push(v);
        if let Some(neighbors) = adj.get(&v) {
            for &w in neighbors {
                if distances[&w] == -1 {
                    distances.insert(w, distances[&v] + 1);
                    queue.push(w);
                }
                if distances[&w] == distances[&v] + 1 {
                    sigma.entry(w).and_modify(|s| *s += sigma[&v]);
                    predecessors.entry(w).or_default().push(v);
                }
            }
        }
    }
    
    (stack, predecessors, distances, sigma)
}
```

#### Community Detection (Louvain Algorithm)
```rust
fn detect_communities(edges: &[EntityRelationship], nodes: &[i32]) -> HashMap<i32, i32> {
    let mut communities: HashMap<i32, i32> = nodes.iter().enumerate().map(|(i, &n)| (n, i as i32)).collect();
    let mut modularity = calculate_modularity(edges, nodes, &communities);
    
    let max_iterations = 100;
    
    for _iter in 0..max_iterations {
        let mut improved = false;
        
        for &node in nodes {
            let current_community = communities[&node];
            
            // Get neighbor communities
            let neighbor_communities: HashSet<i32> = edges.iter()
                .filter(|e| e.source_entity_id == node || e.target_entity_id == node)
                .map(|e| {
                    if e.source_entity_id == node { communities[&e.target_entity_id] }
                    else { communities[&e.source_entity_id] }
                })
                .collect();
            
            // Try moving to each neighbor's community
            for neighbor_community in neighbor_communities {
                if neighbor_community == current_community { continue; }
                
                let mut new_communities = communities.clone();
                new_communities.insert(node, neighbor_community);
                
                let new_modularity = calculate_modularity(edges, nodes, &new_communities);
                
                if new_modularity > modularity {
                    communities = new_communities;
                    modularity = new_modularity;
                    improved = true;
                }
            }
        }
        
        if !improved { break; }
    }
    
    communities
}

fn calculate_modularity(edges: &[EntityRelationship], nodes: &[i32], communities: &HashMap<i32, i32>) -> f64 {
    let m = edges.len() as f64;
    if m == 0.0 { return 0.0; }
    
    let mut sum = 0.0;
    for edge in edges {
        let c1 = communities[&edge.source_entity_id];
        let c2 = communities[&edge.target_entity_id];
        
        if c1 == c2 {
            // Calculate expected edges: k_i * k_j / 2m
            let ki = edges.iter().filter(|e| e.source_entity_id == edge.source_entity_id || e.target_entity_id == edge.source_entity_id).count() as f64;
            let kj = edges.iter().filter(|e| e.source_entity_id == edge.target_entity_id || e.target_entity_id == edge.target_entity_id).count() as f64;
            sum += 1.0 - (ki * kj) / (2.0 * m);
        }
    }
    
    sum / m
}
```

### Metadata Extraction Table
```sql
CREATE TABLE file_metadata (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    fingerprint TEXT NOT NULL,
    metadata_type TEXT NOT NULL,        -- "exif", "document", "audio", "video"
    metadata_json TEXT NOT NULL,        -- Full metadata as JSON
    -- Common fields
    author TEXT,
    created_date DATETIME,
    modified_date DATETIME,
    software TEXT,
    -- EXIF specific
    camera_make TEXT,
    camera_model TEXT,
    gps_latitude REAL,
    gps_longitude REAL,
    -- Audio/Video specific
    duration_seconds REAL,
    codec TEXT,
    bitrate INTEGER,
    sample_rate INTEGER,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_metadata_fingerprint ON file_metadata(fingerprint);
CREATE INDEX idx_metadata_type ON file_metadata(metadata_type);
```

### Structured Data Extraction Table
```sql
CREATE TABLE structured_data (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    fingerprint TEXT NOT NULL,
    data_type TEXT NOT NULL,          -- "table", "form", "key_value"
    structure_json TEXT NOT NULL,      -- Table/Form structure
    row_count INTEGER,
    column_count INTEGER,
    headers_json TEXT,                 -- Column headers
    rows_json TEXT,                   -- Data rows
    extracted_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_structured_fingerprint ON structured_data(fingerprint);
CREATE INDEX idx_structured_type ON structured_data(data_type);
```

### Media Analysis Table
```sql
CREATE TABLE media_analysis (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    fingerprint TEXT NOT NULL,
    analysis_type TEXT NOT NULL,      -- "speaker", "language", "object"
    result_json TEXT NOT NULL,         -- Analysis results
    confidence REAL,
    timestamps_json TEXT,             -- For timestamp correlation
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_media_fingerprint ON media_analysis(fingerprint);
CREATE INDEX idx_media_type ON media_analysis(analysis_type);
```

### Image Analysis Table
```sql
CREATE TABLE image_analysis (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    fingerprint TEXT NOT NULL,
    analysis_type TEXT NOT NULL,      -- "manipulation", "classification", "objects"
    is_manipulated BOOLEAN,
    manipulation_regions_json TEXT,   -- Coordinates of suspected edits
    classification TEXT,
    objects_json TEXT,                 -- Detected objects with confidence
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_image_fingerprint ON image_analysis(fingerprint);
```

### Link Analysis Table
```sql
CREATE TABLE cross_case_links (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_project_id TEXT NOT NULL,
    target_project_id TEXT NOT NULL,
    entity_id INTEGER NOT NULL,
    link_type TEXT NOT NULL,          -- "same_entity", "timeline_overlap", "location_proximity"
    strength REAL DEFAULT 1.0,
    details_json TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_links_projects ON cross_case_links(source_project_id, target_project_id);
CREATE INDEX idx_links_entity ON cross_case_links(entity_id);
```

### Preprocessing Log Table
```sql
CREATE TABLE preprocessing_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    fingerprint TEXT NOT NULL,
    operation TEXT NOT NULL,          -- "normalize", "encode_convert", "format_convert"
    input_format TEXT,
    output_format TEXT,
    status TEXT NOT NULL,             -- "success", "failed", "skipped"
    details TEXT,
    processed_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_preproc_fingerprint ON preprocessing_log(fingerprint);
```

### Verification & QC Table
```sql
CREATE TABLE verification_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    intelligence_id INTEGER NOT NULL,
    verification_type TEXT NOT NULL,  -- "cross_validate", "consensus", "disagreement"
    is_verified BOOLEAN,
    verification_score REAL,          -- Confidence in verification
    supporting_sources_json TEXT,     -- IDs of supporting facts
    conflicting_sources_json TEXT,    -- IDs of conflicting facts
    details TEXT,
    verified_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_verif_fact ON verification_results(intelligence_id);
CREATE INDEX idx_verif_status ON verification_results(is_verified);
```

### Full-Text Search Tables (FTS5)
```sql
-- Facts full-text search
CREATE VIRTUAL TABLE facts_fts USING fts5(
    fact_summary,
    source_quote,
    category,
    content='intelligence',
    content_rowid='id'
);

-- Entities full-text search
CREATE VIRTUAL TABLE entities_fts USING fts5(
    value,
    entity_type,
    content='entities',
    content_rowid='id'
);
```

---

## 11. Processing Architecture

### Stage 1: Text Extraction (Independent)
```
Input Files → [PDF Extractor / OCR / Audio Transcriber] → .txt files
                                    ↓
                              text_cache table
                                    ↓
                              Evidence Root / .slstudio / text_cache /
```

**Characteristics**:
- Runs independently of LLM inference
- Results cached as .txt files
- Can run on all files (fast)
- Re-runs only if file modified

**Extraction Methods**:
| File Type | Method | Output |
|-----------|--------|--------|
| .pdf | pdf-extract | Plain text |
| .pdf (scanned) | ocrs + pdf-extract | OCR text |
| .jpg, .png | ocrs | OCR text |
| .mp3, .wav | whisper.cpp | Transcription |
| .txt, .md | Copy | Plain text |
| .docx | Extract | Plain text |

### Stage 2: LLM Inference (Independent)
```
Cached .txt files → [LLM Pipeline] → Facts + Entities
                                    ↓
                              intelligence table
                                    ↓
                              entities table
```

**Characteristics**:
- Uses cached text (no re-extraction)
- Multiple passes per text
- Can run on sample or full dataset
- Quality scoring per extraction

---

## 12. Incremental Processing

### Default Behavior
1. **Scan** - Check for new/modified files
2. **Extract** - Extract text from new files only
3. **Infer** - Run inference (sample or full)
4. **Rerun** - Optional accuracy improvement passes

### Priority Queue
```rust
enum ProcessingPriority {
    New = 0,        // Never processed
    Modified = 1,  // File changed since last run
    Extracted = 2,  // Has text, no inference
    Rerun = 3,      // For accuracy improvement
}

// Query order for processing
SELECT * FROM registry 
ORDER BY processing_priority ASC, 
         last_modified DESC;
```

### Incremental Logic
```rust
fn scan_for_changes(db: &Database, evidence_root: &Path) -> Vec<RegistryEntry> {
    // 1. Get existing fingerprints
    let existing: HashSet<String> = db.get_all_fingerprints();
    
    // 2. Walk evidence root
    let files = walkdir(evidence_root);
    
    // 3. Classify each file
    for file in files {
        let fingerprint = hash_file(&file);
        
        if !existing.contains(&fingerprint) {
            // New file
            priority = New;
        } else if file.modified > registry.last_modified {
            // Modified file
            priority = Modified;
        } else if registry.has_extracted_text && !registry.processed {
            // Text extracted, no inference
            priority = Extracted;
        } else {
            // Already processed, may rerun
            priority = Rerun;
        }
    }
    
    return sorted_by_priority(priority);
}
```

---

## 13. Multi-Pass LLM Pipeline

### Architecture
```rust
struct Pipeline {
    id: String,
    name: String,
    description: String,
    is_file_type_specific: bool,
    file_type_filter: Option<String>,  // Apply only to pdf/image/audio/doc
    passes: Vec<PipelinePass>,
    sample_size: Option<u32>,
    is_builtin: bool,
}

struct PipelinePass {
    name: String,
    description: String,
    prompt_template: String,
    output_schema: String,
}
```

### Built-in Pipelines

#### Basic Facts (1 pass) - All file types
```json
{
  "name": "Basic Facts",
  "description": "Extract factual findings",
  "passes": [{ "name": "facts", "prompt": "..." }]
}
```

#### Financial Crimes (2 passes) - All types
```json
{
  "name": "Financial Crimes",
  "description": "Focus on money, transactions, dates",
  "passes": [
    { "name": "facts", "prompt": "Extract financial facts: amounts, dates, parties..." },
    { "name": "entities", "prompt": "Extract: money amounts, dates, organizations..." }
  ]
}
```

#### Document Analysis (PDF/DOC) - 3 passes
```json
{
  "name": "Document Analysis",
  "file_type_filter": "pdf,doc,docx",
  "passes": [
    { "name": "facts" },
    { "name": "entities" },
    { "name": "summary" }
  ]
}
```

#### Image OCR Analysis - 2 passes
```json
{
  "name": "Image Analysis", 
  "file_type_filter": "jpg,jpeg,png,bmp",
  "passes": [
    { "name": "ocr_text" },
    { "name": "facts" }
  ]
}
```

#### Audio Transcription - 2 passes
```json
{
  "name": "Audio Analysis",
  "file_type_filter": "mp3,wav,m4a,mp4",
  "passes": [
    { "name": "transcription" },
    { "name": "facts" }
  ]
}
```

---

## 14. Fact Deduplication

### Why Deduplication?
- Same fact extracted from multiple passes
- Similar facts from different files
- Reduces noise in results

### Implementation
```rust
struct DeduplicationConfig {
    similarity_threshold: f32,    // 0.0-1.0, default 0.85
    match_on_fields: Vec<String>, // summary, date, category
    merge_strategy: MergeStrategy, // keep_highest_confidence, merge_all
}

enum MergeStrategy {
    KeepHighestConfidence,
    KeepMostSevere,
    MergeAll,
}

// Deduplication query
SELECT * FROM intelligence 
WHERE fingerprint = :fp 
  AND fact_summary LIKE :summary_pattern
  AND category = :category;
```

### Pipeline Step
```
After LLM inference → Deduplication Pass → Clean intelligence table
```

---

## 15. Quality Scoring

### Metrics Tracked
```rust
struct ExtractionQuality {
    confidence: f32,          // LLM's stated confidence
    text_coverage: f32,      // % of source text processed
    entity_density: f32,     // Entities per 1000 chars
    quote_quality: f32,      // Length/relevance of source quote
    overall: f32,            // Weighted average
    retry_recommended: bool,  // Flag for manual review
    issues: Vec<QualityIssue>,
}

enum QualityIssue {
    LowConfidence,
    ShortQuote,
    PoorCoverage,
    EntityMismatch,
}
```

### Quality Thresholds
```rust
const QUALITY_EXCELLENT: f32 = 0.9;
const QUALITY_GOOD: f32 = 0.7;
const QUALITY_POOR: f32 = 0.5;
// Below 0.5 → queue for manual review
```

### UI Indicators
- Color-coded badges: green (good), yellow (marginal), red (poor)
- Filter by quality score
- Export with quality flags

---

## 16. Error Recovery

### Error Queue Processing
```rust
fn process_error_queue(db: &Database) {
    let pending = db.get_pending_errors();
    
    for error in pending {
        if error.retry_count < error.max_retries {
            // Exponential backoff
            let delay = 2_u64.pow(error.retry_count) * 60;
            
            // Retry
            match retry_operation(&error) {
                Ok(_) => error.mark_resolved("retry_success"),
                Err(e) => {
                    error.retry_count += 1;
                    error.last_attempt = now();
                    error.error_message = e.to_string();
                }
            }
        } else {
            // Max retries exceeded → mark for manual review
            error.mark_for_manual_review();
        }
    }
}
```

### Error Types Handled
| Error Type | Handling |
|------------|----------|
| Corrupt PDF | Skip, log, continue |
| OCR Failure | Retry with different engine |
| LLM Timeout | Retry with shorter chunk |
| Database Lock | Wait and retry |
| Out of Memory | Reduce batch size, retry |

---

## 17. Progress Persistence

### Checkpoint Saving
```rust
fn save_checkpoint(job: &JobState) {
    db.upsert_checkpoint(JobCheckpoint {
        id: job.id,
        status: job.status,
        processed_items: job.processed,
        current_item: job.current_item,
        state_json: serde_json::to_string(&job.state)?,
        updated_at: now(),
    });
}
```

### Resume Logic
```rust
fn resume_job(job_id: &str) -> Option<JobState> {
    let checkpoint = db.get_checkpoint(job_id)?;
    
    if checkpoint.status == "running" {
        // Previous run crashed - resume from last checkpoint
        return Some(serde_json::from_str(&checkpoint.state_json)?);
    }
    None
}
```

---

## 18. Export Formats

### JSON (Full Data)
```json
{
  "export_metadata": {
    "project": "Case 2024-001",
    "exported_at": "2024-01-15T12:00:00Z",
    "total_facts": 150,
    "export_type": "full"
  },
  "facts": [
    {
      "id": 1,
      "fingerprint": "abc123...",
      "source_file": "invoice.pdf",
      "page": 3,
      "quote": "Payment of $50,000 to Acme Corp",
      "summary": "Payment detected",
      "category": "Financial",
      "date": "2024-01-15",
      "severity": 7,
      "confidence": 0.92,
      "quality": 0.88
    }
  ],
  "entities": [...],
  "timeline": [...]
}
```

### CSV (Spreadsheet)
- facts.csv: All facts with columns
- entities.csv: All entities
- timeline.csv: Date-sorted facts

### PDF Report
- Formatted investigation summary
- Facts by category
- Timeline visualization
- Entity summary
- Quality flags

### Excel (Multi-sheet)
- Sheet 1: Facts
- Sheet 2: Entities
- Sheet 3: Timeline
- Sheet 4: Quality Report
- Sheet 5: Errors

---

## 19. Project File Format

```json
{
  "version": "1.0.0",
  "created_at": "2024-01-15T10:30:00Z",
  "modified_at": "2024-01-15T12:45:00Z",
  "investigator": {
    "name": "",
    "agency": "",
    "case_number": "",
    "notes": ""
  },
  "paths": {
    "evidence_root": "",
    "registry_db": "",
    "intelligence_db": "",
    "text_cache_dir": "",
    "export_dir": "",
    "models_dir": ""
  },
  "model": {
    "source": "huggingface",
    "model_id": "",
    "quantization": "Q4_K_M",
    "context_length": 16384,
    "local_path": ""
  },
  "hardware": {
    "gpu_backend": "cpu",
    "gpu_memory_fraction": 0.45,
    "cpu_workers": 8
  },
  "processing": {
    "incremental_by_default": true,
    "default_pipeline": "basic-facts",
    "auto_deduplicate": true,
    "max_retries": 3
  },
  "metadata": {
    "total_files": 0,
    "processed_files": 0,
    "facts_extracted": 0,
    "last_scan_date": null,
    "last_analysis_date": null,
    "tags": []
  }
}
```

---

## 20. User Interface

| Page | Purpose |
|------|---------|
| **Dashboard** | Stats, hardware status, quick actions, recent projects, processing queue |
| **Analysis** | Configure extraction + inference separately |
| **Pipeline** | Select pipeline, sample size, run passes, custom pipeline editor |
| **Results** | Facts, entities, timeline, filtering, export |
| **Map** | File visualization grid, location map, geographic entity clustering |
| **Timeline** | Interactive timeline with zoom/pan, fact markers, date filtering |
| **Network** | Entity relationship graph, hub highlighting, community visualization |
| **Anomalies** | Anomaly dashboard, outlier facts, suspicious patterns |
| **Verification** | Fact verification workflow, cross-reference checks, consensus view |
| **Metadata** | File metadata browser, EXIF viewer, document properties |
| **Structured Data** | Extracted tables, form data, key-value pairs |
| **Media** | Audio/video analysis, speaker segments, transcript alignment |
| **Comparison** | Case comparison, entity overlap, timeline correlation |
| **Settings** | Model download, hardware config, processing options, backup/restore |

### Dashboard Features
- Hardware metrics (GPU, CPU, RAM, disk)
- Processing queue status with progress bars
- Quick action buttons (scan, extract, run pipeline)
- Recent projects list
- Error/warning alerts

### Timeline Features (FR-TIME-003, FR-TIME-004, FR-TIME-005)
- Chronological view of facts with dates
- Zoom controls: day, week, month, year, decade
- Pan navigation with drag
- Filter by category, severity, entity
- Click fact to view details and source
- Color-coded by confidence/severity

### Network Graph Features (FR-NET-003, FR-NET-004)
- Force-directed graph layout
- Node size by connection count (hubs)
- Edge thickness by relationship strength
- Community detection with color coding
- Click node to view entity details
- Filter by entity type, relationship type
- Export graph as image

### Anomaly Dashboard Features (FR-ANOM-001, FR-ANOM-004)
- Anomaly score distribution chart
- List of flagged items with explanations
- Filter by anomaly type
- Mark as resolved/acknowledged
- Anomaly trends over time

### Verification Features (FR-VERIF-001, FR-VERIF-002, FR-VERIF-003)
- Facts grouped by verification status
- Supporting/conflicting sources side-by-side
- Consensus score display
- Quick verify/dispute actions
- Bulk verification workflow

---

## 21. Application Programming Interface

### Project Management
| Command | Description |
|---------|-------------|
| `create_project` | Create new .sls project file |
| `open_project` | Open existing project |
| `save_project` | Save current project |
| `get_project_info` | Get project metadata |
| `import_files` | Import evidence files to project |

### Extraction
| Command | Description |
|---------|-------------|
| `extract_text` | Extract text from single file |
| `extract_batch` | Extract text from multiple files |
| `get_extraction_status` | Check text cache status |

### Pipeline
| Command | Description |
|---------|-------------|
| `run_pipeline` | Run multi-pass pipeline |
| `run_pipeline_sample` | Run on sampled files |
| `get_pipelines` | List available pipelines |
| `create_pipeline` | Create custom pipeline |
| `update_pipeline` | Update existing pipeline |
| `delete_pipeline` | Delete custom pipeline |

### Processing
| Command | Description |
|---------|-------------|
| `scan_incremental` | Scan for new/modified files |
| `get_priority_queue` | Get files by processing priority |
| `deduplicate_facts` | Run deduplication pass |

### Jobs
| Command | Description |
|---------|-------------|
| `save_checkpoint` | Save job progress |
| `resume_job` | Resume from checkpoint |
| `get_job_status` | Get current job status |
| `get_error_queue` | Get pending errors |
| `retry_error` | Retry specific error |

### Evidence Chains
| Command | Description |
|---------|-------------|
| `get_chains` | Get all evidence chains |
| `create_chain` | Create new evidence chain |
| `link_evidence` | Add evidence to chain |
| `unlink_evidence` | Remove evidence from chain |
| `detect_chains` | Auto-detect relationships |

### Case Comparison
| Command | Description |
|---------|-------------|
| `compare_cases` | Compare multiple projects |
| `get_comparisons` | List saved comparisons |
| `delete_comparison` | Delete saved comparison |

### Annotation
| Command | Description |
|---------|-------------|
| `add_annotation` | Add annotation to fact |
| `update_annotation` | Update existing annotation |
| `delete_annotation` | Delete annotation |
| `get_annotations` | Get annotations for fact |
| `create_tag` | Create custom tag |
| `get_tags` | List all tags |

### Search
| Command | Description |
|---------|-------------|
| `search_facts` | Full-text search facts |
| `search_entities` | Full-text search entities |
| `save_search` | Save search query |
| `get_saved_searches` | List saved searches |

### Model Management
| Command | Description |
|---------|-------------|
| `list_models` | List available local models |
| `download_model` | Download model from Hugging Face |
| `get_download_progress` | Check download progress |
| `delete_model` | Remove downloaded model |
| `set_default_model` | Set default model for pipeline |

### System Monitoring
| Command | Description |
|---------|-------------|
| `get_system_metrics` | Get current hardware metrics |
| `get_gpu_info` | Get GPU information |
| `get_processing_stats` | Get processing statistics |

### Export
| Command | Description |
|---------|-------------|
| `export_json` | Export as JSON |
| `export_csv` | Export as CSV |
| `export_pdf` | Generate PDF report |
| `export_excel` | Generate Excel file |

### Audit
| Command | Description |
|---------|-------------|
| `get_audit_log` | Get audit trail |
| `export_audit_log` | Export audit log |

---

## 22. System Monitoring {#system-monitoring}

Real-time hardware and processing monitoring for investigators and system administrators.

### Hardware Detection
- **NFR-PER-003**: The system SHALL auto-detect available hardware (GPU, CPU cores) and scale processing accordingly.

### GPU Backend Options
| Backend | Description | Platforms |
|---------|-------------|-----------|
| `cpu` | CPU-only inference | All |
| `cuda` | NVIDIA GPUs via CUDA | Linux, Windows |
| `metal` | Apple Silicon / Metal | macOS |
| `opencl` | OpenCL GPUs | Linux, Windows |

### Real-time Metrics
```rust
struct SystemMetrics {
    gpu_available: bool,
    gpu_utilization: f32,      // 0.0-1.0
    gpu_memory_used_mb: u64,
    gpu_memory_total_mb: u64,
    cpu_count: usize,
    cpu_usage: f32,            // 0.0-1.0
    ram_used_mb: u64,
    ram_total_mb: u64,
    disk_space_available_mb: u64,
}
```

### Monitoring Dashboard (NFR-USE-001)
- GPU utilization graph over time
- CPU/RAM usage
- Processing queue status
- Estimated time remaining
- Current throughput (files/minute)

### Processing Statistics
| Metric | Description |
|--------|-------------|
| Files processed | Total files completed |
| Facts extracted | Total facts in database |
| Average quality | Mean quality score |
| Processing rate | Files per minute |
| Error rate | Failed/total ratio |

---

## 23. Performance Optimization

Auto-tune batch size during processing based on performance metrics:

```rust
struct BatchTuner {
    target_time_ms: u64,      // Target time per batch
    min_batch_size: usize,
    max_batch_size: usize,
    tune_interval: u32,       // Tune every N batches
}

impl BatchTuner {
    fn tune(&mut self, metrics: &[BatchMetrics]) {
        let avg_time = metrics.iter().map(|m| m.duration_ms).sum::<u64>() / metrics.len() as u64;
        let current_memory = metrics.last().map(|m| m.memory_mb).unwrap_or(0);
        
        if avg_time > self.target_time_ms * 1.5 || current_memory > MEMORY_LIMIT {
            // Slowing down or running high on memory - decrease batch
            self.current_batch_size = (self.current_batch_size * 8) / 10;
        } else if avg_time < self.target_time_ms * 0.7 && current_memory < MEMORY_LIMIT * 0.5 {
            // Faster than target and low memory - increase batch
            self.current_batch_size = (self.current_batch_size * 12) / 10;
        }
    }
}
```

### Tuning Triggers
| Condition | Action |
|-----------|--------|
| Avg time > 150% target | Decrease batch 20% |
| Avg time < 70% target | Increase batch 20% |
| Memory > 80% limit | Decrease batch 30% |
| Stable for 10 batches | Gradual increase 10% |

---

## 24. Evidence Chain Tracking

Link related evidence together to understand relationships:

> Database schema defined in [Section 4 - Evidence Chains Table](#evidence-chains-table)

### Relationship Types
| Type | Description | Direction |
|------|-------------|-----------|
| `references` | A mentions B | A → B |
| `similar_to` | Content similar | A ↔ B |
| `duplicate_of` | Same content | A ↔ B |
| `responds_to` | Reply chain | A → B |
| `derived_from` | Created from | A ← B |
| `contains` | A contains B | A → B |

### Chain Detection
```
Evidence → Extract entities (names, dates, IDs)
         → Match: same entity = related
         → Build chains automatically
         → Allow manual linking
```

### UI Display
- Visual chain diagram
- Click node → View evidence
- Add/remove manual links

---

## 25. Search Index

Full-text search across facts and entities using SQLite FTS5:

### Schema
```sql
-- Facts full-text search
CREATE VIRTUAL TABLE facts_fts USING fts5(
    fact_summary,
    source_quote,
    category,
    content='intelligence',
    content_rowid='id'
);

-- Triggers to keep FTS in sync
CREATE TRIGGER facts_ai AFTER INSERT ON intelligence BEGIN
    INSERT INTO facts_fts(rowid, fact_summary, source_quote, category)
    VALUES (NEW.id, NEW.fact_summary, NEW.source_quote, NEW.category);
END;

CREATE TRIGGER facts_ad AFTER DELETE ON intelligence BEGIN
    INSERT INTO facts_fts(facts_fts, rowid, fact_summary, source_quote, category)
    VALUES ('delete', OLD.id, OLD.fact_summary, OLD.source_quote, OLD.category);
END;

-- Entities full-text search  
CREATE VIRTUAL TABLE entities_fts USING fts5(
    value,
    entity_type,
    content='entities',
    content_rowid='id'
);

-- Triggers to keep entities FTS in sync
CREATE TRIGGER entities_ai AFTER INSERT ON entities BEGIN
    INSERT INTO entities_fts(rowid, value, entity_type)
    VALUES (NEW.id, NEW.value, NEW.entity_type);
END;

CREATE TRIGGER entities_ad AFTER DELETE ON entities BEGIN
    INSERT INTO entities_fts(entities_fts, rowid, value, entity_type)
    VALUES ('delete', OLD.id, OLD.value, OLD.entity_type);
END;

CREATE TRIGGER entities_au AFTER UPDATE ON entities BEGIN
    INSERT INTO entities_fts(entities_fts, rowid, value, entity_type)
    VALUES ('delete', OLD.id, OLD.value, OLD.entity_type);
    INSERT INTO entities_fts(rowid, value, entity_type)
    VALUES (NEW.id, NEW.value, NEW.entity_type);
END;
```

### Search Examples
```sql
-- Simple search
SELECT * FROM facts_fts WHERE facts_fts MATCH 'payment*';

-- Boolean operators
SELECT * FROM facts_fts WHERE facts_fts MATCH 'fraud AND NOT tax';

-- Phrase search
SELECT * FROM facts_fts WHERE facts_fts MATCH '"payment of"';

-- Filtered search
SELECT i.* FROM intelligence i
JOIN facts_fts f ON i.id = f.rowid
WHERE facts_fts MATCH 'money' AND i.severity > 5;
```

### UI Search Bar
- Quick search: facts, entities
- Advanced: filters + search
- Search history
- Save searches

---

## 26. Case Comparison

Compare facts and entities across multiple projects/cases:

> Database schema defined in [Section 4 - Case Comparisons Table](#case-comparisons-table)

### Comparison Logic
```rust
fn compare_cases(projects: &[Project]) -> CaseComparison {
    // 1. Get all entities from each project
    let entities_by_project: HashMap<ProjectId, Vec<Entity>> = ...;
    
    // 2. Find common entities
    let common: Vec<Entity> = find_intersection(entities_by_project.values());
    
    // 3. Get facts for common entities
    let matching_facts: Vec<Fact> = ...;
    
    // 4. Timeline overlap
    let common_dates: Vec<DateRange> = find_overlap(projects.timelines());
    
    CaseComparison {
        common_entities: common,
        matching_facts,
        common_dates,
        statistics: compute_stats(...),
    }
}
```

### Comparison Types
| Type | Description |
|------|-------------|
| Entity overlap | What names/orgs appear in both |
| Fact similarity | Similar facts across cases |
| Timeline | Date ranges that overlap |
| Geographic | Locations that appear in both |

### UI
- Select 2+ projects to compare
- Side-by-side results
- Export comparison report

---

## 27. Annotation and Tagging

Manual annotations and tagging for facts:

> Database schema defined in [Section 4 - Fact Annotations Table](#fact-annotations-table)

### Annotation Types
| Type | Purpose |
|------|---------|
| `review` | Mark as reviewed |
| `note` | Add investigation note |
| `flag` | Flag for attention |
| `tag` | Apply custom tags |

### UI Features
- Select fact → Add annotation
- Bulk annotation (select multiple)
- Filter by tags
- Tag management (create/edit/delete)

---

## 28. Multi-Language Support

Process evidence in multiple languages:

### Language Configuration
```rust
struct LanguageConfig {
    default_language: String,    // "en", "es", "fr", "de", etc.
    detect_language: bool,     // Auto-detect from text
    translate_quotes: bool,     // Translate to default language
    supported_languages: Vec<String>,
}

impl LanguageConfig {
    fn detect(text: &str) -> Option<String> {
        // Use langdetect or n-gram analysis
        // Return ISO 639-1 code
    }
}
```

### LLM Prompt Variants
```json
{
  "prompts": {
    "en": "Extract facts from the following English text. ...",
    "es": "Extraiga hechos del siguiente texto en espanol. ...",
    "fr": "Extraire les faits du texte francais suivant. ...",
    "de": "Extrahieren Sie Fakten aus dem folgenden deutschen Text. ..."
  }
}
```

### Processing Flow
```
Input File → Detect Language → Select Prompt → LLM → Results
                     ↓
              If unknown → Default prompt
```

### Supported Languages (Initial)
| Code | Language |
|------|----------|
| en | English |
| es | Spanish |
| fr | French |
| de | German |
| pt | Portuguese |
| it | Italian |
| zh | Chinese |
| ja | Japanese |
| ko | Korean |
| ar | Arabic |

### Evidence Linking
```sql
-- Store original language
ALTER TABLE intelligence ADD COLUMN source_language TEXT;

-- Store translations if enabled
ALTER TABLE intelligence ADD COLUMN translated_quote TEXT;
```

---

## 29. File Structure

```
steinline/
├── SPEC.md
├── README.md
├── package.json
├── src/
│   └── routes/
│       ├── +layout.svelte
│       ├── +page.svelte (Dashboard)
│       ├── analysis/
│       ├── pipeline/
│       ├── results/
│       ├── map/
│       └── settings/
├── src-tauri/
│   └── src/
│       ├── lib.rs
│       ├── config/
│       ├── core/           (database, registry)
│       ├── extractors/     (pdf, ocr, audio)
│       ├── gpu/
│       ├── inference/     (llm, reasoner)
│       ├── pipeline/      (runner, sampler)
│       ├── extraction/    (text extraction stage)
│       ├── deduplication/
│       ├── profiling/
│       └── utils/
    └── tests/
```

---

## 30. Development Plan

## Phase 1: Foundation (Weeks 1-4) ✅ COMPLETE

### 1.1 Project Setup
- [x] Initialize Tauri 2 + SvelteKit project
- [x] Configure TypeScript and Rust tooling
- [x] Set up ESLint, Prettier, Cargo clippy
- [x] Configure GitHub Actions CI/CD pipeline
- [x] Set up logging infrastructure (tracing)

### 1.2 Core Infrastructure
- [x] Set up SQLite database with rusqlite
- [x] Create database migration system
- [x] Implement registry table schema
- [x] Implement text_cache table schema
- [x] Create config management module

### 1.3 File System Basics
- [x] Implement file walker with walkdir
- [x] Create fingerprinting (MD5/SHA256)
- [x] Implement file import/registry logic
- [x] Add basic file metadata extraction

**Milestone: Basic file import and tracking working** ✅

---

## Phase 2: Text Extraction (Weeks 5-8)

### 2.1 PDF Extraction
- [x] Integrate pdf-extract crate
- [x] Implement PDF text extraction command
- [x] Handle extraction errors gracefully
- [x] Add text caching to filesystem
- [x] Implement extraction quality scoring

### 2.2 Image OCR
- [x] Integrate ocrs crate
- [x] Implement image OCR pipeline
- [ ] Add image preprocessing (contrast, rotation)
- [x] Handle multi-page TIFF/images

### 2.3 Audio Transcription
- [x] Integrate whisper-rs crate (optional, requires cmake)
- [ ] Implement audio transcription
- [ ] Add progress tracking for long audio
- [x] Support MP3, WAV, M4A, MP4

### 2.4 Document Parsing
- [x] Integrate docx-rs for DOCX
- [x] Add plain text file handling (TXT, MD)
- [x] Implement encoding detection (UTF-8, Latin-1)

**Milestone: All file types can be extracted to text** ✅

---

## Phase 3: LLM Integration (Weeks 9-12)

### 3.1 LLM Infrastructure
- [ ] Integrate llama.cpp bindings
- [ ] Implement model download from HuggingFace
- [ ] Create model manager (list, select, delete)
- [ ] Add quantization selection (Q4, Q5, Q8)
- [ ] Implement model loading/unloading

### 3.2 Pipeline Framework
- [ ] Design pipeline data structures
- [ ] Implement pass runner
- [ ] Create prompt template system
- [ ] Add output schema validation
- [ ] Implement sample size configuration

### 3.3 Built-in Pipelines
- [ ] Implement Basic Facts pipeline (1 pass)
- [ ] Implement Financial Crimes pipeline (2 passes)
- [ ] Create pipeline configuration UI
- [ ] Add custom pipeline editor

**Milestone: LLM can extract facts from text**

---

## Phase 4: Data Management (Weeks 13-16)

### 4.1 Intelligence Database
- [ ] Implement intelligence table schema
- [ ] Create fact storage and retrieval
- [ ] Add entity extraction (NER)
- [ ] Implement multi-language support
- [ ] Add source attribution tracking

### 4.2 Quality & Deduplication
- [ ] Implement quality scoring algorithm
- [ ] Create deduplication logic
- [ ] Add merge strategy options
- [ ] Implement quality threshold warnings

### 4.3 Incremental Processing
- [ ] Implement file change detection
- [ ] Create priority queue system
- [ ] Add checkpoint/resume logic
- [ ] Implement error queue with retry

**Milestone: Complete two-stage pipeline working**

---

## Phase 5: Search & Analysis (Weeks 17-20)

### 5.1 Search Infrastructure
- [ ] Implement FTS5 for facts
- [ ] Implement FTS5 for entities
- [ ] Create full-text search API
- [ ] Add Boolean and phrase search
- [ ] Implement faceted filtering

### 5.2 Analysis Features
- [ ] Implement temporal analysis
- [ ] Add entity resolution/aliasing
- [ ] Create network analysis algorithms
- [ ] Implement anomaly detection
- [ ] Add evidence weighting system

### 5.3 Evidence Chains
- [ ] Implement chain tracking database
- [ ] Create automatic chain detection
- [ ] Add manual chain linking UI
- [ ] Implement relationship visualization

**Milestone: Search and analysis features complete**

---

## Phase 6: User Interface (Weeks 21-26)

### 6.1 Core UI
- [ ] Build Dashboard page
- [ ] Create Analysis configuration page
- [ ] Implement Pipeline builder UI
- [ ] Build Results viewer with filtering
- [ ] Add Settings page

### 6.2 Visualization
- [ ] Implement Timeline visualization
- [ ] Add Network graph (Cytoscape.js)
- [ ] Integrate Leaflet.js for maps
- [ ] Create Charts for statistics
- [ ] Add Anomaly dashboard

### 6.3 User Experience
- [ ] Implement bulk operations
- [ ] Add keyboard shortcuts
- [ ] Create annotation system
- [ ] Add tagging and filtering
- [ ] Implement undo/redo

**Milestone: Full UI operational**

---

## Phase 7: Export & Reporting (Weeks 27-28)

### 7.1 Export Formats
- [ ] Implement JSON export
- [ ] Create CSV export
- [ ] Add PDF report generation
- [ ] Implement Excel multi-sheet export

### 7.2 Case Comparison
- [ ] Implement cross-project comparison
- [ ] Add entity overlap detection
- [ ] Create timeline correlation
- [ ] Build comparison UI

**Milestone: Export and reporting complete**

---

## Phase 8: System Integration (Weeks 29-30)

### 8.1 System Features
- [ ] Implement system monitoring (GPU/CPU)
- [ ] Add batch size tuning
- [ ] Create hardware auto-detection
- [ ] Implement memory management

### 8.2 CLI & Automation
- [ ] Build Tauri CLI for automation
- [ ] Add scheduled processing
- [ ] Implement backup/restore

### 8.3 Desktop Integration
- [ ] Add system tray support
- [ ] Implement notifications
- [ ] Add file associations (.sls)

**Milestone: System integration complete**

---

## Phase 9: Polish & Release (Weeks 31-32)

### 9.1 Testing
- [ ] Write unit tests for core modules
- [ ] Create integration tests
- [ ] Perform E2E testing with Playwright
- [ ] Conduct usability testing

### 9.2 Documentation
- [ ] Write user documentation
- [ ] Create API documentation
- [ ] Add inline code comments
- [ ] Prepare release notes

### 9.3 Release
- [ ] Build Windows installer
- [ ] Build macOS DMG
- [ ] Build Linux AppImage
- [ ] Perform security audit

**Milestone: Production release**

---

## Development Dependencies

### Required Tools
| Tool | Purpose |
|------|---------|
| Rust 1.75+ | Backend development |
| Node.js 20+ | Frontend development |
| Visual Studio Build Tools | Windows compilation |
| Xcode | macOS compilation |

### Key Dependencies
| Crate | Version | Purpose |
|-------|---------|---------|
| tauri | 2.x | Desktop framework |
| sveltekit | 2.x | Frontend framework |
| rusqlite | 0.31 | Database |
| pdf-extract | 0.7 | PDF text extraction |
| ocrs | 2 | OCR engine |
| whisper-rs | 1 | Audio transcription |
| llama-cpp | 0.2 | LLM inference |

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| LLM model issues | Test with multiple models |
| OCR accuracy | Allow manual correction |
| Performance | Implement batching, caching |
| Memory usage | Add limits, monitoring |
| Database size | Implement archiving |

---

## Success Criteria

Each phase is complete when:
- All tests pass
- No clippy warnings
- Documentation updated
- Feature works end-to-end
