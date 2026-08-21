# MVP widget map

| Need | MVP widget/layout |
|---|---|
| Notes and collections navigation | `Row`, `Column`, `Scrollable`, `Button` |
| Choose a collection | Iced `pick_list` |
| Select source text | Iced `text_editor`; perform non-edit actions and read `Content::selection()` |
| Capture the selection | Fixed Iced `Button` enabled when selection is non-empty |
| Name the field | Iced `text_input` |
| Choose its type | Iced `pick_list` |
| Boolean value | Iced `checkbox` |
| Date, time, and timestamp values | Iced `text_input` with domain parsing |
| Show accumulated fields | `Column` of bordered `Container` rows |
| Long panes | Independent Iced `Scrollable` widgets |
| Errors | Existing bordered `Container` banner |

`iced_aw` 0.14 is compatible with Iced 0.14. Its `drop_down` or `context_menu` can later move
**Use selection** nearer the selected text, but the MVP uses a fixed button because it needs no
overlay positioning or additional dependency.

The note editor must ignore `text_editor::Action::Edit` while structuring. Cursor, selection,
click, drag, and scroll actions are still performed, so `Content::selection()` remains available
without allowing the source note to change.
