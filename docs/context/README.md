# Context Notes

This folder contains long-form intent and design history.

- `concept-zero.md` is the design bible reference document.
- It includes conflicting historical directions (early rsync-centric plan vs later HTTP pull design).
- The current codebase follows the later appendix direction ("No-Bullshit Edition": single binary, HTTP pull, active playset sync, launcher bypass).

For as-built behavior and implementation gaps, see:

- `docs/architecture.md`
- `docs/gap-findings.md`
