// SPDX-FileCopyrightText: 2026 Charles Willis <5862883+trippwill@users.noreply.github.com>
// SPDX-License-Identifier: MPL-2.0

use iced::widget::{self, text_editor};
use iced::{Element, Length, Task};

use super::{AppModel, Message as AppMessage};
use crate::core::prelude::*;
use crate::data::prelude::*;
use crate::data::query;
use crate::fl;

/// Messages emitted by the notes workspace.
#[derive(Debug, Clone)]
pub enum Message {
    Add,
    Added(PileNote, Result<(), String>),
    Archive,
    ArchivedLoaded(NoteId, Result<Option<ArchiveNote>, String>),
    CancelEditing,
    ClearSearch,
    DraftEdited(text_editor::Action),
    Edit,
    EditDraftEdited(text_editor::Action),
    Edited(PileNote, Result<(), String>),
    Loaded(NoteId, Result<Option<PileNote>, String>),
    Moved(Result<(), String>),
    Restore,
    Save,
    Search,
    SearchChanged(String),
    Select(NoteId),
    Show(NoteList),
    SummariesLoaded(NoteList, Result<Vec<NoteSummary>, String>),
}

/// The note list shown in the document sidebar.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum NoteList {
    /// Notes in the active pile.
    Pile,
    /// Notes retained in the archive.
    Archive,
    /// Active notes whose bodies contain the given string.
    Search(String),
}

#[derive(Debug, Clone)]
pub(super) enum SelectedNote {
    Pile(PileNote),
    Archive(ArchiveNote),
}

impl AppModel {
    pub(super) fn reset_notes(&mut self) {
        self.list = NoteList::Pile;
        self.summaries.clear();
        self.selected_id = None;
        self.selected_note = None;
        self.draft = text_editor::Content::new();
        self.edit_draft = text_editor::Content::new();
        self.search.clear();
        self.editing = false;
    }

    pub(super) fn update_notes(&mut self, message: Message) -> Task<AppMessage> {
        match message {
            Message::Add => self.add_note(),
            Message::Added(note, result) => self.note_added(note, result),
            Message::Archive => self.archive_note(),
            Message::ArchivedLoaded(id, result) => self.archived_note_loaded(id, result),
            Message::CancelEditing => self.cancel_editing(),
            Message::ClearSearch => self.clear_search(),
            Message::DraftEdited(action) => {
                self.draft.perform(action);
                Task::none()
            }
            Message::Edit => self.edit_note(),
            Message::EditDraftEdited(action) => {
                self.edit_draft.perform(action);
                Task::none()
            }
            Message::Edited(note, result) => self.note_edited(note, result),
            Message::Loaded(id, result) => self.note_loaded(id, result),
            Message::Moved(result) => self.note_moved(result),
            Message::Restore => self.restore_note(),
            Message::Save => self.save_note(),
            Message::Search => self.search_notes(),
            Message::SearchChanged(search) => {
                self.search = search;
                Task::none()
            }
            Message::Select(id) => self.select_note(id),
            Message::Show(list) => self.show_notes(list),
            Message::SummariesLoaded(list, result) => self.summaries_loaded(&list, result),
        }
    }

    fn summaries_loaded(
        &mut self,
        list: &NoteList,
        result: Result<Vec<NoteSummary>, String>,
    ) -> Task<AppMessage> {
        if list != &self.list {
            return Task::none();
        }
        self.busy = false;
        match result {
            Ok(summaries) => {
                self.summaries = summaries;
                if self
                    .selected_id
                    .is_none_or(|id| !self.summaries.iter().any(|note| note.id == id))
                {
                    self.selected_id = None;
                    self.selected_note = None;
                    self.edit_draft = text_editor::Content::new();
                    self.editing = false;
                    if let Some(summary) = self.summaries.first() {
                        return self.select_note(summary.id);
                    }
                }
            }
            Err(error) => self.error = Some(error),
        }
        Task::none()
    }

    fn show_notes(&mut self, list: NoteList) -> Task<AppMessage> {
        if self.busy || self.editing || self.list == list {
            return Task::none();
        }
        let Some(document) = self.document.clone() else {
            return Task::none();
        };

        self.list = list.clone();
        self.summaries.clear();
        self.selected_id = None;
        self.selected_note = None;
        if !matches!(list, NoteList::Search(_)) {
            self.search.clear();
        }
        self.busy = true;
        self.error = None;
        load_summaries(document, list)
    }

    fn select_note(&mut self, id: NoteId) -> Task<AppMessage> {
        if self.editing {
            return Task::none();
        }
        self.selected_id = Some(id);
        self.selected_note = None;
        let Some(document) = self.document.clone() else {
            return Task::none();
        };

        match self.list {
            NoteList::Pile | NoteList::Search(_) => Task::future(async move {
                AppMessage::Notes(Message::Loaded(
                    id,
                    document
                        .query(query::NoteById(id))
                        .await
                        .map_err(|error| error.to_string()),
                ))
            }),
            NoteList::Archive => Task::future(async move {
                AppMessage::Notes(Message::ArchivedLoaded(
                    id,
                    document
                        .query(query::ArchivedNoteById(id))
                        .await
                        .map_err(|error| error.to_string()),
                ))
            }),
        }
    }

    fn note_loaded(
        &mut self,
        id: NoteId,
        result: Result<Option<PileNote>, String>,
    ) -> Task<AppMessage> {
        if matches!(self.list, NoteList::Pile | NoteList::Search(_)) && self.selected_id == Some(id)
        {
            match result {
                Ok(Some(note)) => {
                    self.selected_note = Some(SelectedNote::Pile(note));
                    self.edit_draft = text_editor::Content::new();
                    self.editing = false;
                }
                Ok(None) => self.error = Some(fl!("note-not-found")),
                Err(error) => self.error = Some(error),
            }
        }
        Task::none()
    }

    fn archived_note_loaded(
        &mut self,
        id: NoteId,
        result: Result<Option<ArchiveNote>, String>,
    ) -> Task<AppMessage> {
        if self.list == NoteList::Archive && self.selected_id == Some(id) {
            match result {
                Ok(Some(note)) => {
                    self.selected_note = Some(SelectedNote::Archive(note));
                    self.edit_draft = text_editor::Content::new();
                    self.editing = false;
                }
                Ok(None) => self.error = Some(fl!("note-not-found")),
                Err(error) => self.error = Some(error),
            }
        }
        Task::none()
    }

    fn edit_note(&mut self) -> Task<AppMessage> {
        if self.busy || self.editing {
            return Task::none();
        }
        let Some(SelectedNote::Pile(note)) = &self.selected_note else {
            return Task::none();
        };
        self.edit_draft = text_editor::Content::with_text(note.body());
        self.editing = true;
        Task::none()
    }

    fn cancel_editing(&mut self) -> Task<AppMessage> {
        if !self.busy {
            self.edit_draft = text_editor::Content::new();
            self.editing = false;
        }
        Task::none()
    }

    fn save_note(&mut self) -> Task<AppMessage> {
        if self.busy || !self.editing {
            return Task::none();
        }
        let Some(document) = self.document.clone() else {
            return Task::none();
        };
        let Some(SelectedNote::Pile(note)) = &self.selected_note else {
            return Task::none();
        };
        let body = self.edit_draft.text();
        if body == note.body() {
            return Task::none();
        }
        let note = match note.edit(body) {
            Ok(note) => note,
            Err(error) => {
                self.error = Some(error.to_string());
                return Task::none();
            }
        };

        self.busy = true;
        self.error = None;
        Task::future(async move {
            let result = document
                .execute(vec![Command::EditNote(note.clone())])
                .await
                .map_err(|error| error.to_string());
            AppMessage::Notes(Message::Edited(note, result))
        })
    }

    fn add_note(&mut self) -> Task<AppMessage> {
        if self.list != NoteList::Pile {
            return Task::none();
        }
        let Some(document) = self.document.clone() else {
            return Task::none();
        };
        if self.busy || self.editing {
            return Task::none();
        }

        self.busy = true;
        self.error = None;
        let note = PileNote::create(self.draft.text());

        Task::future(async move {
            let result = document
                .execute(vec![Command::AddNote(note.clone())])
                .await
                .map_err(|error| error.to_string());
            AppMessage::Notes(Message::Added(note, result))
        })
    }

    fn note_added(&mut self, note: PileNote, result: Result<(), String>) -> Task<AppMessage> {
        self.busy = false;
        match result {
            Ok(()) => {
                self.selected_id = Some(note.id());
                self.selected_note = Some(SelectedNote::Pile(note));
                self.draft = text_editor::Content::new();
                if let Some(document) = self.document.clone() {
                    self.busy = true;
                    return load_summaries(document, self.list.clone());
                }
            }
            Err(error) => self.error = Some(error),
        }
        Task::none()
    }

    fn note_edited(&mut self, note: PileNote, result: Result<(), String>) -> Task<AppMessage> {
        self.busy = false;
        match result {
            Ok(()) => {
                self.selected_id = Some(note.id());
                self.selected_note = Some(SelectedNote::Pile(note));
                self.edit_draft = text_editor::Content::new();
                self.editing = false;
                if let Some(document) = self.document.clone() {
                    self.busy = true;
                    return load_summaries(document, self.list.clone());
                }
            }
            Err(error) => self.error = Some(error),
        }
        Task::none()
    }

    fn archive_note(&mut self) -> Task<AppMessage> {
        if self.busy || self.editing {
            return Task::none();
        }
        let Some(document) = self.document.clone() else {
            return Task::none();
        };
        let Some(SelectedNote::Pile(note)) = &self.selected_note else {
            return Task::none();
        };
        let note = note.clone().archive();

        self.busy = true;
        self.error = None;
        Task::future(async move {
            AppMessage::Notes(Message::Moved(
                document
                    .execute(vec![Command::ArchiveNote(note)])
                    .await
                    .map_err(|error| error.to_string()),
            ))
        })
    }

    fn restore_note(&mut self) -> Task<AppMessage> {
        if self.busy || self.editing {
            return Task::none();
        }
        let Some(document) = self.document.clone() else {
            return Task::none();
        };
        let Some(SelectedNote::Archive(note)) = &self.selected_note else {
            return Task::none();
        };
        let note = note.clone().unarchive();

        self.busy = true;
        self.error = None;
        Task::future(async move {
            AppMessage::Notes(Message::Moved(
                document
                    .execute(vec![Command::UnarchiveNote(note)])
                    .await
                    .map_err(|error| error.to_string()),
            ))
        })
    }

    fn note_moved(&mut self, result: Result<(), String>) -> Task<AppMessage> {
        self.busy = false;
        match result {
            Ok(()) => {
                self.selected_id = None;
                self.selected_note = None;
                if let Some(document) = self.document.clone() {
                    self.busy = true;
                    return load_summaries(document, self.list.clone());
                }
            }
            Err(error) => self.error = Some(error),
        }
        Task::none()
    }

    fn clear_search(&mut self) -> Task<AppMessage> {
        self.search.clear();
        self.show_notes(NoteList::Pile)
    }

    fn search_notes(&mut self) -> Task<AppMessage> {
        if self.busy || self.editing || self.list == NoteList::Archive {
            return Task::none();
        }
        let list = if self.search.is_empty() {
            NoteList::Pile
        } else {
            NoteList::Search(self.search.clone())
        };
        self.show_notes(list)
    }

    pub(super) fn notes_view(&self) -> Element<'_, AppMessage> {
        let document = self
            .document
            .as_ref()
            .expect("notes view requires document");
        let name = document
            .path()
            .file_name()
            .unwrap_or_else(|| document.path().as_os_str())
            .to_string_lossy();

        widget::Column::with_capacity(2)
            .push(widget::text(name).size(28))
            .push(
                widget::Row::with_capacity(2)
                    .push(self.notes_sidebar())
                    .push(self.notes_main())
                    .spacing(24)
                    .height(Length::Fill),
            )
            .spacing(16)
            .height(Length::Fill)
            .into()
    }

    fn notes_sidebar(&self) -> Element<'_, AppMessage> {
        let mut list = widget::Column::with_capacity(self.summaries.len()).spacing(4);
        for summary in &self.summaries {
            let preview = if summary.preview.is_empty() {
                fl!("untitled-note")
            } else {
                summary.preview.clone()
            };
            let selected = self.selected_id == Some(summary.id);
            list = list.push(
                widget::button(widget::text(preview))
                    .style(if selected {
                        widget::button::primary
                    } else {
                        widget::button::text
                    })
                    .width(Length::Fill)
                    .on_press_maybe(
                        (!self.editing && !self.busy)
                            .then_some(AppMessage::Notes(Message::Select(summary.id))),
                    ),
            );
        }

        let notes: Element<_> = if self.summaries.is_empty() {
            widget::text(match &self.list {
                NoteList::Pile => fl!("no-notes"),
                NoteList::Archive => fl!("no-archived-notes"),
                NoteList::Search(_) => fl!("no-search-results"),
            })
            .into()
        } else {
            widget::scrollable(list).height(Length::Fill).into()
        };

        let pile_button = if self.list == NoteList::Pile {
            widget::button(widget::text(fl!("pile"))).style(widget::button::primary)
        } else {
            widget::button(widget::text(fl!("pile"))).style(widget::button::text)
        }
        .on_press_maybe(
            (!self.busy && !self.editing && self.list != NoteList::Pile)
                .then_some(AppMessage::Notes(Message::Show(NoteList::Pile))),
        );
        let archive_button = if self.list == NoteList::Archive {
            widget::button(widget::text(fl!("archive"))).style(widget::button::primary)
        } else {
            widget::button(widget::text(fl!("archive"))).style(widget::button::text)
        }
        .on_press_maybe(
            (!self.busy && !self.editing && self.list != NoteList::Archive)
                .then_some(AppMessage::Notes(Message::Show(NoteList::Archive))),
        );

        let mut sidebar = widget::Column::with_capacity(3);
        if self.list != NoteList::Archive {
            sidebar = sidebar.push(self.search_view());
        }
        sidebar
            .push(
                widget::Row::with_capacity(2)
                    .push(pile_button)
                    .push(archive_button)
                    .spacing(8),
            )
            .push(notes)
            .spacing(8)
            .width(260)
            .height(Length::Fill)
            .into()
    }

    fn search_view(&self) -> Element<'_, AppMessage> {
        let placeholder = fl!("search-placeholder");
        let search = widget::text_input(&placeholder, &self.search);
        let search = if self.busy || self.editing {
            search
        } else {
            search
                .on_input(|search| AppMessage::Notes(Message::SearchChanged(search)))
                .on_submit(AppMessage::Notes(Message::Search))
        };
        let mut search_row = widget::Row::with_capacity(3).push(search).push(
            widget::button(widget::text(fl!("search-notes")))
                .style(widget::button::primary)
                .on_press_maybe(
                    (!self.busy && !self.editing).then_some(AppMessage::Notes(Message::Search)),
                ),
        );
        if !self.search.is_empty() {
            search_row = search_row.push(
                widget::button("×")
                    .style(widget::button::text)
                    .on_press_maybe(
                        (!self.busy && !self.editing)
                            .then_some(AppMessage::Notes(Message::ClearSearch)),
                    ),
            );
        }
        search_row.spacing(8).into()
    }

    fn notes_main(&self) -> Element<'_, AppMessage> {
        let mut main = widget::Column::with_capacity(2).push(self.note_body());
        if !self.editing && self.list == NoteList::Pile {
            main = main.push(self.note_composer());
        }
        main.spacing(16)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn note_body(&self) -> Element<'_, AppMessage> {
        match (&self.selected_note, self.editing) {
            (Some(SelectedNote::Pile(note)), true) => {
                let editor = widget::text_editor(&self.edit_draft).height(Length::Fill);
                let editor = if self.busy {
                    editor
                } else {
                    editor.on_action(|action| AppMessage::Notes(Message::EditDraftEdited(action)))
                };

                widget::Column::with_capacity(2)
                    .push(editor)
                    .push(
                        widget::Row::with_capacity(2)
                            .push(
                                widget::button(widget::text(fl!("save-note")))
                                    .style(widget::button::primary)
                                    .on_press_maybe(
                                        (!self.busy && self.edit_draft.text() != note.body())
                                            .then_some(AppMessage::Notes(Message::Save)),
                                    ),
                            )
                            .push(
                                widget::button(widget::text(fl!("cancel")))
                                    .style(widget::button::text)
                                    .on_press_maybe(
                                        (!self.busy)
                                            .then_some(AppMessage::Notes(Message::CancelEditing)),
                                    ),
                            )
                            .spacing(8),
                    )
                    .spacing(8)
                    .height(Length::Fill)
                    .into()
            }
            (Some(SelectedNote::Pile(note)), false) => widget::Column::with_capacity(2)
                .push(
                    widget::scrollable(widget::text(note.body()).width(Length::Fill))
                        .height(Length::Fill),
                )
                .push(
                    widget::Row::with_capacity(2)
                        .push(
                            widget::button(widget::text(fl!("edit-note")))
                                .style(widget::button::text)
                                .on_press_maybe(
                                    (!self.busy).then_some(AppMessage::Notes(Message::Edit)),
                                ),
                        )
                        .push(
                            widget::button(widget::text(fl!("archive")))
                                .style(widget::button::text)
                                .on_press_maybe(
                                    (!self.busy).then_some(AppMessage::Notes(Message::Archive)),
                                ),
                        )
                        .spacing(8),
                )
                .spacing(8)
                .height(Length::Fill)
                .into(),
            (Some(SelectedNote::Archive(note)), _) => widget::Column::with_capacity(2)
                .push(
                    widget::scrollable(widget::text(note.body()).width(Length::Fill))
                        .height(Length::Fill),
                )
                .push(
                    widget::button(widget::text(fl!("restore-note")))
                        .style(widget::button::primary)
                        .on_press_maybe(
                            (!self.busy).then_some(AppMessage::Notes(Message::Restore)),
                        ),
                )
                .spacing(8)
                .height(Length::Fill)
                .into(),
            (None, _) => widget::container(widget::text(fl!("select-note")))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into(),
        }
    }

    fn note_composer(&self) -> Element<'_, AppMessage> {
        widget::Column::with_capacity(3)
            .push(widget::text(fl!("new-note")).size(20))
            .push(
                widget::text_editor(&self.draft)
                    .placeholder(fl!("note-placeholder"))
                    .height(140)
                    .on_action(|action| AppMessage::Notes(Message::DraftEdited(action))),
            )
            .push(
                widget::button(widget::text(fl!("add-note")))
                    .style(widget::button::primary)
                    .on_press_maybe(
                        (!self.busy && !self.editing).then_some(AppMessage::Notes(Message::Add)),
                    ),
            )
            .spacing(8)
            .into()
    }
}

pub(super) fn load_summaries(document: Document, list: NoteList) -> Task<AppMessage> {
    Task::future(async move {
        let result = match &list {
            NoteList::Pile => document.query(query::NoteSummaries).await,
            NoteList::Archive => document.query(query::ArchivedNoteSummaries).await,
            NoteList::Search(search) => document.query(query::NoteSearch(search.clone())).await,
        }
        .map_err(|error| error.to_string());

        AppMessage::Notes(Message::SummariesLoaded(list, result))
    })
}
