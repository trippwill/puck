# Domain and Data Boundaries

## GUI state is not data-bound

The GUI does not hold live database objects or references into `Document`. Queries materialize owned
values shaped for the current view. The GUI stores those values in its own state and renders them
until an application message changes or replaces them.

Widgets emit messages containing small, opaque domain IDs and user input. `update` changes GUI state
and returns a task that executes a query or command through `Document`. The task yields another
message carrying its result. Database changes do not update widgets automatically; refreshes and
command results are handled explicitly by `update`.

Caching query results is an application concern and does not change this boundary.

## There is no record schema

Puck has no domain object that declares the fields allowed in a collection or record. In particular,
there is no `Schema`, `SchemaId`, shared validation set, or schema membership check.

A collection's available fields are an emergent view of the field definitions used by its records.
Queries derive that view from persisted records. Any record may use a new field definition, and other
records are not required to provide a value for it.

`SchemaVersion` remains the version of Puck's SQLite storage format. It is unrelated to record
structure.

## Field values have no domain identity

Field definitions have stable domain identity because records refer to them and users may reuse or
rename them. Collections and records also have stable domain identity for lookup and persistence.

A field value does not have its own domain ID. It is the value for one field definition in one
record, identified by that relationship:

```text
(RecordId, FieldDefId) -> value
```

Setting the field again replaces that value. Introduce field-value identity only if a concrete
feature, such as independently addressable value history, requires it.

## Persistence boundary remains open

The exact `Document` query and write API is intentionally undecided until persistence is implemented.
Modeled commands and a CQRS-style split are current possibilities, not committed architecture.

In particular, no decision has been made about whether writes carry domain models, raw IDs and values,
or another minimal representation. The implementation should choose the smallest interface that
preserves domain invariants and works cleanly with the GUI's asynchronous task boundary.

Do not add command or query buses, event sourcing, repositories, separate read and write databases, or
a caching layer merely to prepare for that decision.
