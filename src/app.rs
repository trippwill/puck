// SPDX-FileCopyrightText: 2026 Charles Willis <5862883+trippwill@users.noreply.github.com>
// SPDX-License-Identifier: MPL-2.0

//! The puck application root.

mod collections;
mod notes;
mod structure;

use std::path::PathBuf;

use iced::widget::{self, text_editor};
use iced::{Alignment, Element, Length, Task};
use rfd::AsyncFileDialog;

use self::collections::{CollectionMessage, CollectionsState};
pub use self::notes::Message as NoteMessage;
use self::notes::{NoteList, SelectedNote};
use self::structure::{StructureDraft, StructureMessage};
use crate::core::prelude::*;
use crate::data::prelude::*;
use crate::fl;

/// The application model for an open Puck document.
pub struct AppModel {
    document: Option<Document>,
    workspace: Workspace,
    collections: CollectionsState,
    structure: Option<StructureDraft>,
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
    ClearError,
    CloseDocument,
    Collections(CollectionMessage),
    DocumentLoaded(Result<Option<Document>, String>),
    NewDocument,
    Notes(NoteMessage),
    OpenDocument,
    ShowWorkspace(Workspace),
    Structure(StructureMessage),
}

/// A top-level workspace in an open document.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Workspace {
    /// Free-form note capture and retrieval.
    Notes,
    /// Structured collection and record retrieval.
    Collections,
    /// Building a record from selected note text.
    Structure,
}

impl AppModel {
    pub fn new(document_path: Option<PathBuf>) -> (Self, Task<Message>) {
        let busy = document_path.is_some();
        let app = Self {
            document: None,
            workspace: Workspace::Notes,
            collections: CollectionsState::default(),
            structure: None,
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
                .style(crate::theme::panel),
            );
        }

        if self.document.is_some() {
            content = content.push(self.document_actions());
            content = content.push(match self.workspace {
                Workspace::Notes => self.notes_view(),
                Workspace::Collections => self.collections_view(),
                Workspace::Structure => self.structure_view(),
            });
        } else {
            content = content.push(self.landing_view());
        }

        if self.busy {
            content = content.push(widget::text(fl!("working")));
        }

        content.into()
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::NewDocument => self.new_document(),
            Message::OpenDocument => self.open_document(),
            Message::DocumentLoaded(result) => self.document_loaded(result),
            Message::CloseDocument => self.close_document(),
            Message::Notes(message) => self.update_notes(message),
            Message::Collections(message) => self.update_collections(message),
            Message::ShowWorkspace(workspace) => self.show_workspace(workspace),
            Message::Structure(message) => self.update_structure(message),
            Message::ClearError => {
                self.error = None;
                Task::none()
            }
        }
    }

    fn landing_view(&self) -> Element<'_, Message> {
        let landing = widget::Column::with_capacity(4)
            .push(widget::text(fl!("app-title")).size(40))
            .push(widget::text(fl!("landing-description")))
            .push(
                widget::button(widget::text(fl!("new-document")))
                    .style(crate::theme::primary_pill)
                    .on_press_maybe((!self.busy).then_some(Message::NewDocument)),
            )
            .push(
                widget::button(widget::text(fl!("open-document")))
                    .style(widget::button::secondary)
                    .on_press_maybe((!self.busy).then_some(Message::OpenDocument)),
            )
            .spacing(16)
            .align_x(Alignment::Center);

        widget::container(landing)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }

    fn document_actions(&self) -> Element<'_, Message> {
        widget::row![
            widget::button(widget::text(fl!("notes")))
                .style(if self.workspace == Workspace::Notes {
                    crate::theme::selected
                } else {
                    widget::button::text
                })
                .on_press_maybe(
                    (!self.busy
                        && !self.editing
                        && self.structure.is_none()
                        && self.workspace != Workspace::Notes)
                        .then_some(Message::ShowWorkspace(Workspace::Notes)),
                ),
            widget::button(widget::text(fl!("collections")))
                .style(if self.workspace == Workspace::Collections {
                    crate::theme::selected
                } else {
                    widget::button::text
                })
                .on_press_maybe(
                    (!self.busy
                        && !self.editing
                        && self.structure.is_none()
                        && self.workspace != Workspace::Collections)
                        .then_some(Message::ShowWorkspace(Workspace::Collections)),
                ),
            widget::Space::new().width(Length::Fill),
            widget::button(widget::text(fl!("close-document")))
                .style(widget::button::text)
                .on_press_maybe(
                    (!self.busy && !self.editing && self.structure.is_none())
                        .then_some(Message::CloseDocument),
                ),
            widget::button(widget::text(fl!("new-document")))
                .style(widget::button::text)
                .on_press_maybe(
                    (!self.busy && !self.editing && self.structure.is_none())
                        .then_some(Message::NewDocument),
                ),
            widget::button(widget::text(fl!("open-document")))
                .style(crate::theme::primary_pill)
                .on_press_maybe(
                    (!self.busy && !self.editing && self.structure.is_none())
                        .then_some(Message::OpenDocument),
                ),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    }

    fn new_document(&mut self) -> Task<Message> {
        if self.busy || self.editing {
            return Task::none();
        }
        self.busy = true;
        self.error = None;
        let title = fl!("new-document");
        let filter_name = fl!("puck-documents");

        Task::future(async move {
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
        })
    }

    fn open_document(&mut self) -> Task<Message> {
        if self.busy || self.editing {
            return Task::none();
        }
        self.busy = true;
        self.error = None;
        let title = fl!("open-document");
        let filter_name = fl!("puck-documents");

        Task::future(async move {
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
        })
    }

    fn document_loaded(&mut self, result: Result<Option<Document>, String>) -> Task<Message> {
        self.busy = false;
        match result {
            Ok(Some(document)) => {
                self.document = Some(document.clone());
                self.workspace = Workspace::Notes;
                self.reset_notes();
                self.reset_collections();
                self.structure = None;
                self.busy = true;
                notes::load_summaries(document, NoteList::Pile)
            }
            Ok(None) => Task::none(),
            Err(error) => {
                self.error = Some(error);
                Task::none()
            }
        }
    }

    fn close_document(&mut self) -> Task<Message> {
        if self.busy || self.editing {
            return Task::none();
        }
        self.document = None;
        self.workspace = Workspace::Notes;
        self.reset_notes();
        self.reset_collections();
        self.structure = None;
        self.error = None;
        Task::none()
    }

    fn show_workspace(&mut self, workspace: Workspace) -> Task<Message> {
        if self.busy || self.editing || self.workspace == workspace {
            return Task::none();
        }
        self.workspace = workspace;
        match workspace {
            Workspace::Collections => self.load_collections(),
            Workspace::Notes | Workspace::Structure => Task::none(),
        }
    }
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
