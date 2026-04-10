## SeaORM And SQL Boundaries

1. Default rule: all database reads and writes go through SeaORM entities or SeaQuery builders.
1. Raw SQL is allowed only for PostgreSQL-native primitives that SeaORM or SeaQuery cannot express cleanly or safely.
1. Raw SQL must be isolated to backend-only modules and must not appear in service, handler, or domain code.
1. Every raw SQL call must include a short comment explaining why SeaORM or SeaQuery was insufficient.
1. Every raw SQL call must be parameterized. Do not build SQL from dynamic values except trusted static identifiers that cannot be bound.
1. Raw SQL must not encode business logic. It may express storage primitives only.
1. If raw SQL is needed more than once, wrap it behind a typed helper with a narrow API.
1. Migrations should prefer SeaQuery first. Raw SQL in migrations is allowed only for backend-specific DDL.
1. PostgreSQL-specific behavior must remain inside the PostgreSQL adapter. Shared storage traits must stay database-agnostic.
1. Any new raw SQL requires a test that proves the behavior could not be implemented cleanly through SeaORM alone.

## Repository Application

- Use SeaORM for tables, CRUD, filtering, transactions, and updates.
- If sequence-backed IDs remain, raw SQL is allowed only for sequence operations.
- Sequence operations should be wrapped in a small PostgreSQL-only helper so SQL strings do not spread through the backend.
