# Bottom-Up Record Creation

## Purpose

Puck lets users discover structure while capturing information. A user may begin with an
unstructured note, extract useful values into a record, and name the fields only after seeing the
data.

This is **bottom-up record creation**: structure emerges from the fields users add to records instead
of requiring the available fields to be defined in advance.

## User-facing concepts

- A **note** contains free-form text.
- A **field** gives a value a type and a name, such as `Hostname` or `IPv4`.
- A **record** contains field values for one subject.
- A **collection** contains related records.
- A collection's available fields are the fields currently used by its records.

Records may leave fields unset. Adding a field does not require values to be added to existing
records.

## Creating the first record

Starting from an empty Puck file:

1. The user creates a note containing unstructured information about hosts on a LAN.
2. The user highlights `alpha-01`.
3. The user invokes **Add to Record**, then chooses **New Text Field**.
4. A record editor opens with:
   - a new, unnamed text field;
   - `alpha-01` as its value;
   - a new, untitled collection as the destination.
5. The user names the field `Hostname`.
6. The user highlights the IP address for `alpha-01`.
7. The user invokes **Add to Record**, then chooses **New Text Field**.
8. Another unnamed text field appears with the selected address as its value.
9. The user names the field `IPv4`.
10. The user names the collection `LAN Hosts`.
11. The user chooses **Done**.

The source note remains unchanged. Extracting a value copies the selected text; it does not remove
or rewrite it.

An unnamed field must be named or removed before the record can be completed. The user may edit an
extracted value before choosing **Done**.

## Creating another record

The user can begin another record in either of two ways:

- choose **New Record** while viewing `LAN Hosts`; or
- highlight text in a note, choose **Add to Record**, and select `LAN Hosts`.

The new record shows the collection's existing fields with empty values:

- `Hostname`
- `IPv4`

When text is selected, **Add to Record** offers compatible empty fields before offering a new one:

- **Set Hostname**
- **Set IPv4**
- **New Text Field**

For `alpha-02`, the user chooses **Set Hostname** rather than **New Text Field**. The same applies to
its IP address. This creates another record using the existing fields instead of adding duplicate
`Hostname` and `IPv4` fields to the collection.

If a destination field already has a value, replacing it must be presented as an explicit
**Replace** action rather than silently overwriting it.

## Extending the collection

While creating the `alpha-02` record, the user also captures its open ports:

1. The user highlights the open-port information.
2. The user chooses **Add to Record**, then **New Text Field**.
3. A new unnamed field appears with the selected value.
4. The user names the field `Open Ports`.
5. The user chooses **Done**.

`Open Ports` becomes available when editing records in `LAN Hosts` because one of its records now
uses that field. The earlier `alpha-01` record remains valid with `Open Ports` unset.

## Completion actions

### Done

**Done** completes the current record and returns the user to the previous view.

Use **Done** or **Add Record**, not **Save Record**. Saving is not a separate concept the user needs
to manage for each record.

### New Record

**New Record** completes the current record when it is ready, then opens an empty record in the same
collection.

If an unnamed field remains, the user must name or remove it before continuing.

### Save Collection

There is no **Save Collection** action. Collection changes are part of completing the record that
introduced them.

### Leaving an unfinished record

If the user leaves an unfinished record, Puck offers:

- **Keep Editing**
- **Discard**

Discarding the record also discards fields introduced only by that unfinished record. It never
changes the source note.

## Empty states

- An empty file offers **New Note**, **New Record**, and **New Collection**.
- **New Record** with no existing collection starts an untitled collection and an empty record.
- A collection name may remain untitled while editing, but the user is prompted to name it before
  completing its first record.
- An empty record is allowed while editing. Completing one without any fields should require an
  explicit user decision rather than happen accidentally.

## Product rules

- Structure may emerge from captured data.
- Existing fields are reused for subsequent records.
- New fields extend the available structure without forcing values onto older records.
- Source notes remain intact after extraction.
- Record completion is the only explicit completion boundary in this flow.
- Collections do not have a separate save action.
- A collection's available fields emerge from the fields used by its records.
