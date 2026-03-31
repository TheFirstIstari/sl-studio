# Full-Text Search

## Overview

SL Studio uses SQLite FTS5 (Full-Text Search version 5) for fast text search across facts and entities.

## Search Syntax

FTS5 supports Boolean operators:

| Operator | Example                             | Description         |
| -------- | ----------------------------------- | ------------------- |
| AND      | `fraud AND money`                   | Both terms required |
| OR       | `fraud OR embezzlement`             | Either term         |
| NOT      | `fraud NOT money`                   | Exclude term        |
| Phrase   | `"money laundering"`                | Exact phrase        |
| Prefix   | `lau*`                              | Prefix matching     |
| Grouping | `(fraud OR embezzlement) AND money` | Complex queries     |

## Search Commands

### search_facts

Searches the `facts_fts` virtual table:

```
Parameters:
  - query: String (FTS5 query syntax)
  - category: Option<String>
  - severity: Option<String>
  - date_from: Option<String>
  - date_to: Option<String>

Returns: Vec<SearchResult>
```

### search_entities

Searches the `entities_fts` virtual table:

```
Parameters:
  - query: String
  - entity_type: Option<String>

Returns: Vec<EntityResult>
```

### search_combined

Combines results from both facts and entities:

```
Parameters:
  - query: String

Returns: Vec<CombinedResult>
```

### search_by_tags

Filters facts by tags:

```
Parameters:
  - tags: Vec<String>

Returns: Vec<SearchResult>
```

## Performance

- **Simple queries**: ~200-900ns depending on length
- **Complex queries**: Scale linearly with query length (~15ns per character)
- **Indexed searches**: Sub-millisecond for typical datasets
