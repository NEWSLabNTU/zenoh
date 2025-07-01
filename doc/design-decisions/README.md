# Design Decisions

This directory contains Architecture Decision Records (ADRs) for the Zenoh project.

## What is an ADR?

An Architecture Decision Record captures an important architectural decision made along with its context and consequences.

## Template

Use the [template.md](template.md) file when creating new ADRs.

## Decision Log

| ADR | Title | Status | Date |
|-----|-------|--------|------|
| [template](template.md) | ADR Template | - | - |

## Categories

### Protocol Design
- Wire protocol format
- Message types and semantics
- Compatibility considerations

### Routing Architecture
- Routing algorithm choices
- HAT design decisions
- Scalability trade-offs

### API Design
- Public API surface
- Backward compatibility
- Language bindings

### Performance
- Zero-copy architecture
- Memory management strategies
- Optimization decisions

### Security
- Authentication mechanisms
- Encryption choices
- Access control design

## Process

1. Copy the template to a new file: `decisions/ADR-XXX-title.md`
2. Fill out all sections
3. Submit for review
4. Update status after decision
5. Add entry to decision log above