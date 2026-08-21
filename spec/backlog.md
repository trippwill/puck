# Backlog

## Copy records between collections

Status: deferred after record move.

`record copy RECORD COLLECTION` creates a new, independent record in the
destination collection and prints its new ID. It is a true copy, not a reference
to the source record.

- Keep the source record unchanged.
- Generate a new `RecordId`.
- Copy active visible field values into new field rows using the same field
  definition IDs.
- Do not copy pending-deletion fields or deletion history.
- Require active source and destination collections, an active source record,
  and active field definitions.
- Insert the record and fields atomically.
- Keep later edits and deletion of either record independent.

No schema migration is expected. Implement this only when duplication is a
confirmed workflow; move covers relocation without creating another identity.
