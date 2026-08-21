// SPDX-FileCopyrightText: 2026 Charles Willis <5862883+trippwill@users.noreply.github.com>
// SPDX-License-Identifier: MPL-2.0

use std::path::PathBuf;

use iced::widget::{self, text_editor};
use iced::{Alignment, Element, Length, Task};
use rfd::AsyncFileDialog;

use crate::core::{ArchiveNote, NoteId, NoteSummary, PileNote};
use crate::data::prelude::*;
use crate::fl;

/// The application model for an open Puck document.
pub struct AppModel {
    document: Option<Document>,
    list: NoteList,
    summaries: Vec<NoteSummary>,
    selected_id: Option<NoteId>,
    selected_note: Option<SelectedNote>,
    draft: text_editor::Content,
    edit_draft: text_editor::Content,
    search: String,
    editing: bool,
    busy: bool,
    error: Option<String>,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    AddNote,
    ArchiveNote,
    ArchivedNoteLoaded(NoteId, Result<Option<ArchiveNote>, String>),
    CancelEditing,
    CloseDocument,
    ClearSearch,
    ClearError,
    DocumentLoaded(Result<Option<Document>, String>),
    DraftEdited(text_editor::Action),
    EditDraftEdited(text_editor::Action),
    EditNote,
    NewDocument,
    NoteAdded(Result<PileNote, String>),
    NoteEdited(Result<PileNote, String>),
    NoteLoaded(NoteId, Result<Option<PileNote>, String>),
    NoteMoved(Result<(), String>),
    OpenDocument,
    RestoreNote,
    SaveNote,
    SearchChanged(String),
    SearchNotes,
    SelectNote(NoteId),
    ShowNotes(NoteList),
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
enum SelectedNote {
    Pile(PileNote),
    Archive(ArchiveNote),
}

impl AppModel {
    pub fn new(document_path: Option<PathBuf>) -> (Self, Task<Message>) {
        let busy = document_path.is_some();
        let app = Self {
            document: None,
            list: NoteList::Pile,
            summaries: Vec::new(),
            selected_id: None,
            selected_note: None,
            draft: text_editor::Content::new(),
            edit_draft: text_editor::Content::new(),
            search: String::new(),
            editing: false,
            busy,
            error: None,
        };
        let command = match document_path {
            Some(path) => Task::future(async move {
                Message::DocumentLoaded(
                    Document::open(path)
                        .await
                        .map(Some)
                        .map_err(|error| error.to_string()),
                )
            }),
            None => Task::none(),
        };

        (app, command)
    }

    pub fn title(&self) -> String {
        let mut title = fl!("app-title");
        if let Some(document) = &self.document
            && let Some(name) = document.path().file_name()
        {
            title.push_str(" — ");
            title.push_str(&name.to_string_lossy());
        }
        title
    }

    pub fn view(&self) -> Element<'_, Message> {
        let mut content = widget::Column::with_capacity(4)
            .spacing(16)
            .padding(24)
            .width(Length::Fill)
            .height(Length::Fill);

        if let Some(error) = &self.error {
            content = content.push(
                widget::container(
                    widget::row![
                        widget::text(error).style(widget::text::danger),
                        widget::Space::new().width(Length::Fill),
                        widget::button("×")
                            .style(widget::button::text)
                            .on_press(Message::ClearError),
                    ]
                    .align_y(Alignment::Center),
                )
                .padding(12)
                .style(widget::container::bordered_box),
            );
        }

        if self.document.is_some() {
            content = content.push(self.document_actions());
            content = content.push(self.document_view());
        } else {
            let landing = widget::Column::with_capacity(4)
                .push(widget::text(fl!("app-title")).size(40))
                .push(widget::text(fl!("landing-description")))
                .push(
                    widget::button(widget::text(fl!("new-document")))
                        .style(widget::button::primary)
                        .on_press_maybe((!self.busy).then_some(Message::NewDocument)),
                )
                .push(
                    widget::button(widget::text(fl!("open-document")))
                        .style(widget::button::secondary)
                        .on_press_maybe((!self.busy).then_some(Message::OpenDocument)),
                )
                .spacing(16)
                .align_x(Alignment::Center);

            content = content.push(
                widget::container(landing)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill),
            );
        }

        if self.busy {
            content = content.push(widget::text(fl!("working")));
        }

        content.into()
    }

    #[allow(clippy::too_many_lines)]
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::NewDocument => {
                if self.busy || self.editing {
                    return Task::none();
                }
                self.busy = true;
                self.error = None;
                let title = fl!("new-document");
                let filter_name = fl!("puck-documents");

                return Task::future(async move {
                    let file = AsyncFileDialog::new()
                        .set_title(title)
                        .set_file_name("notes.puck")
                        .add_filter(filter_name, &["puck"])
                        .save_file()
                        .await;
                    Message::DocumentLoaded(match file {
                        Some(file) => Document::create(file.path())
                            .await
                            .map(Some)
                            .map_err(|error| error.to_string()),
                        None => Ok(None),
                    })
                });
            }
            Message::OpenDocument => {
                if self.busy || self.editing {
                    return Task::none();
                }
                self.busy = true;
                self.error = None;
                let title = fl!("open-document");
                let filter_name = fl!("puck-documents");

                return Task::future(async move {
                    let file = AsyncFileDialog::new()
                        .set_title(title)
                        .add_filter(filter_name, &["puck"])
                        .pick_file()
                        .await;
                    Message::DocumentLoaded(match file {
                        Some(file) => Document::open(file.path())
                            .await
                            .map(Some)
                            .map_err(|error| error.to_string()),
                        None => Ok(None),
                    })
                });
            }
            Message::DocumentLoaded(result) => {
                self.busy = false;
                match result {
                    Ok(Some(document)) => {
                        self.document = Some(document.clone());
                        self.list = NoteList::Pile;
                        self.summaries.clear();
                        self.selected_id = None;
                        self.selected_note = None;
                        self.draft = text_editor::Content::new();
                        self.edit_draft = text_editor::Content::new();
                        self.search.clear();
                        self.editing = false;
                        self.busy = true;
                        return load_summaries(document, NoteList::Pile);
                    }
                    Ok(None) => {}
                    Err(error) => self.error = Some(error),
                }
            }
            Message::CloseDocument => {
                if self.busy || self.editing {
                    return Task::none();
                }
                self.document = None;
                self.list = NoteList::Pile;
                self.summaries.clear();
                self.selected_id = None;
                self.selected_note = None;
                self.draft = text_editor::Content::new();
                self.edit_draft = text_editor::Content::new();
                self.search.clear();
                self.error = None;
                return Task::none();
            }
            Message::SummariesLoaded(list, result) => {
                if list != self.list {
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
                                return self.update(Message::SelectNote(summary.id));
                            }
                        }
                    }
                    Err(error) => self.error = Some(error),
                }
            }
            Message::ShowNotes(list) => {
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
                return load_summaries(document, list);
            }
            Message::SelectNote(id) => {
                if self.editing {
                    return Task::none();
                }
                self.selected_id = Some(id);
                self.selected_note = None;
                let Some(document) = self.document.clone() else {
                    return Task::none();
                };

                return match self.list {
                    NoteList::Pile | NoteList::Search(_) => Task::future(async move {
                        Message::NoteLoaded(
                            id,
                            document
                                .query(NoteById(id))
                                .await
                                .map_err(|error| error.to_string()),
                        )
                    }),
                    NoteList::Archive => Task::future(async move {
                        Message::ArchivedNoteLoaded(
                            id,
                            document
                                .query(ArchivedNoteById(id))
                                .await
                                .map_err(|error| error.to_string()),
                        )
                    }),
                };
            }
            Message::NoteLoaded(id, result) => {
                if matches!(self.list, NoteList::Pile | NoteList::Search(_))
                    && self.selected_id == Some(id)
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
            }
            Message::ArchivedNoteLoaded(id, result) => {
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
            }
            Message::DraftEdited(action) => self.draft.perform(action),
            Message::EditDraftEdited(action) => self.edit_draft.perform(action),
            Message::EditNote => {
                if self.busy || self.editing {
                    return Task::none();
                }
                let Some(SelectedNote::Pile(note)) = &self.selected_note else {
                    return Task::none();
                };
                self.edit_draft = text_editor::Content::with_text(note.body());
                self.editing = true;
            }
            Message::CancelEditing => {
                if !self.busy {
                    self.edit_draft = text_editor::Content::new();
                    self.editing = false;
                }
            }
            Message::SaveNote => {
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
                return Task::future(async move {
                    let result = document
                        .execute(vec![Command::EditNote(note.clone())])
                        .await
                        .map(|()| note)
                        .map_err(|error| error.to_string());
                    Message::NoteEdited(result)
                });
            }
            Message::AddNote => {
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

                return Task::future(async move {
                    let result = document
                        .execute(vec![Command::AddNote(note.clone())])
                        .await
                        .map(|()| note)
                        .map_err(|error| error.to_string());
                    Message::NoteAdded(result)
                });
            }
            Message::NoteAdded(result) => {
                self.busy = false;
                match result {
                    Ok(note) => {
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
            }
            Message::NoteEdited(result) => {
                self.busy = false;
                match result {
                    Ok(note) => {
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
            }
            Message::ArchiveNote => {
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
                return Task::future(async move {
                    Message::NoteMoved(
                        document
                            .execute(vec![Command::ArchiveNote(note)])
                            .await
                            .map_err(|error| error.to_string()),
                    )
                });
            }
            Message::RestoreNote => {
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
                return Task::future(async move {
                    Message::NoteMoved(
                        document
                            .execute(vec![Command::UnarchiveNote(note)])
                            .await
                            .map_err(|error| error.to_string()),
                    )
                });
            }
            Message::NoteMoved(result) => {
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
            }
            Message::SearchChanged(search) => self.search = search,
            Message::ClearSearch => {
                self.search.clear();
                return self.update(Message::ShowNotes(NoteList::Pile));
            }
            Message::SearchNotes => {
                if self.busy || self.editing || self.list == NoteList::Archive {
                    return Task::none();
                }
                let list = if self.search.is_empty() {
                    NoteList::Pile
                } else {
                    NoteList::Search(self.search.clone())
                };
                return self.update(Message::ShowNotes(list));
            }
            Message::ClearError => self.error = None,
        }

        Task::none()
    }
}

impl AppModel {
    #[allow(clippy::too_many_lines)]
    fn document_view(&self) -> Element<'_, Message> {
        let document = self
            .document
            .as_ref()
            .expect("document view requires document");
        let name = document
            .path()
            .file_name()
            .unwrap_or_else(|| document.path().as_os_str())
            .to_string_lossy();

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
                        (!self.editing && !self.busy).then_some(Message::SelectNote(summary.id)),
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
                .then_some(Message::ShowNotes(NoteList::Pile)),
        );
        let archive_button = if self.list == NoteList::Archive {
            widget::button(widget::text(fl!("archive"))).style(widget::button::primary)
        } else {
            widget::button(widget::text(fl!("archive"))).style(widget::button::text)
        }
        .on_press_maybe(
            (!self.busy && !self.editing && self.list != NoteList::Archive)
                .then_some(Message::ShowNotes(NoteList::Archive)),
        );

        let mut sidebar = widget::Column::with_capacity(3);
        if self.list != NoteList::Archive {
            let placeholder = fl!("search-placeholder");
            let search = widget::text_input(&placeholder, &self.search);
            let search = if self.busy || self.editing {
                search
            } else {
                search
                    .on_input(Message::SearchChanged)
                    .on_submit(Message::SearchNotes)
            };
            let mut search_row = widget::Row::with_capacity(3).push(search).push(
                widget::button(widget::text(fl!("search-notes")))
                    .style(widget::button::primary)
                    .on_press_maybe((!self.busy && !self.editing).then_some(Message::SearchNotes)),
            );
            if !self.search.is_empty() {
                search_row = search_row.push(
                    widget::button("×")
                        .style(widget::button::text)
                        .on_press_maybe(
                            (!self.busy && !self.editing).then_some(Message::ClearSearch),
                        ),
                );
            }
            sidebar = sidebar.push(search_row.spacing(8));
        }
        let sidebar = sidebar
            .push(
                widget::Row::with_capacity(2)
                    .push(pile_button)
                    .push(archive_button)
                    .spacing(8),
            )
            .push(notes)
            .spacing(8)
            .width(260)
            .height(Length::Fill);

        let body: Element<_> = match (&self.selected_note, self.editing) {
            (Some(SelectedNote::Pile(note)), true) => {
                let editor = widget::text_editor(&self.edit_draft).height(Length::Fill);
                let editor = if self.busy {
                    editor
                } else {
                    editor.on_action(Message::EditDraftEdited)
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
                                            .then_some(Message::SaveNote),
                                    ),
                            )
                            .push(
                                widget::button(widget::text(fl!("cancel")))
                                    .style(widget::button::text)
                                    .on_press_maybe((!self.busy).then_some(Message::CancelEditing)),
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
                                .on_press_maybe((!self.busy).then_some(Message::EditNote)),
                        )
                        .push(
                            widget::button(widget::text(fl!("archive")))
                                .style(widget::button::text)
                                .on_press_maybe((!self.busy).then_some(Message::ArchiveNote)),
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
                        .on_press_maybe((!self.busy).then_some(Message::RestoreNote)),
                )
                .spacing(8)
                .height(Length::Fill)
                .into(),
            (None, _) => widget::container(widget::text(fl!("select-note")))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into(),
        };

        let composer = widget::Column::with_capacity(3)
            .push(widget::text(fl!("new-note")).size(20))
            .push(
                widget::text_editor(&self.draft)
                    .placeholder(fl!("note-placeholder"))
                    .height(140)
                    .on_action(Message::DraftEdited),
            )
            .push(
                widget::button(widget::text(fl!("add-note")))
                    .style(widget::button::primary)
                    .on_press_maybe((!self.busy && !self.editing).then_some(Message::AddNote)),
            )
            .spacing(8);

        let mut main = widget::Column::with_capacity(2).push(body);
        if !self.editing && self.list == NoteList::Pile {
            main = main.push(composer);
        }
        let main = main.spacing(16).width(Length::Fill).height(Length::Fill);

        widget::Column::with_capacity(2)
            .push(widget::text(name).size(28))
            .push(
                widget::Row::with_capacity(2)
                    .push(sidebar)
                    .push(main)
                    .spacing(24)
                    .height(Length::Fill),
            )
            .spacing(16)
            .height(Length::Fill)
            .into()
    }

    fn document_actions(&self) -> Element<'_, Message> {
        widget::row![
            widget::Space::new().width(Length::Fill),
            widget::button(widget::text(fl!("close-document")))
                .style(widget::button::text)
                .on_press_maybe((!self.busy && !self.editing).then_some(Message::CloseDocument),),
            widget::button(widget::text(fl!("new-document")))
                .style(widget::button::text)
                .on_press_maybe((!self.busy && !self.editing).then_some(Message::NewDocument)),
            widget::button(widget::text(fl!("open-document")))
                .style(widget::button::primary)
                .on_press_maybe((!self.busy && !self.editing).then_some(Message::OpenDocument)),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    }
}

fn load_summaries(document: Document, list: NoteList) -> Task<Message> {
    Task::future(async move {
        let result = match &list {
            NoteList::Pile => document.query(NoteSummaries).await,
            NoteList::Archive => document.query(ArchivedNoteSummaries).await,
            NoteList::Search(query) => document.query(NoteSearch(query.clone())).await,
        }
        .map_err(|error| error.to_string());

        Message::SummariesLoaded(list, result)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_document_button_emits_message() {
        let (app, _) = AppModel::new(None);
        let mut ui = iced_test::simulator(app.view());
        let label = fl!("new-document");

        ui.click(label.as_str()).expect("click New Document");

        assert!(matches!(
            ui.into_messages().next(),
            Some(Message::NewDocument)
        ));
    }
}
