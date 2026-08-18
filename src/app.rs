// SPDX-License-Identifier: MPL-2.0

use std::collections::HashMap;

use cosmic::app::context_drawer;
use cosmic::dialog::file_chooser::{self, FileFilter};
use cosmic::iced::widget::text_editor;
use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget::about::About;
use cosmic::widget::{self, menu};

use crate::core::{NoteId, NoteSummary, PileNote};
use crate::data::prelude::*;
use crate::fl;

const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const APP_ICON: &[u8] = include_bytes!("../resources/icons/hicolor/scalable/apps/icon.svg");

/// The application model for an open Puck document.
pub struct AppModel {
    core: cosmic::Core,
    context_page: ContextPage,
    about: About,
    #[allow(clippy::zero_sized_map_values)]
    key_binds: HashMap<menu::KeyBind, MenuAction>,
    document: Option<Document>,
    summaries: Vec<NoteSummary>,
    selected_id: Option<NoteId>,
    selected_note: Option<PileNote>,
    draft: text_editor::Content,
    edit_draft: text_editor::Content,
    editing: bool,
    busy: bool,
    error: Option<String>,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    AddNote,
    CancelEditing,
    ClearError,
    DocumentLoaded(Result<Option<Document>, String>),
    DraftEdited(text_editor::Action),
    EditDraftEdited(text_editor::Action),
    EditNote,
    LaunchUrl(String),
    NewDocument,
    NoteAdded(Result<PileNote, String>),
    NoteEdited(Result<PileNote, String>),
    NoteLoaded(NoteId, Result<Option<PileNote>, String>),
    OpenDocument,
    SaveNote,
    SelectNote(NoteId),
    SummariesLoaded(Result<Vec<NoteSummary>, String>),
    ToggleContextPage(ContextPage),
}

impl cosmic::Application for AppModel {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = "dev.terranul.puck";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        let mut app = Self {
            core,
            context_page: ContextPage::default(),
            about: About::default()
                .name(fl!("app-title"))
                .icon(widget::icon::from_svg_bytes(APP_ICON))
                .version(env!("CARGO_PKG_VERSION"))
                .links([(fl!("repository"), REPOSITORY)])
                .license(env!("CARGO_PKG_LICENSE")),
            key_binds: HashMap::new(),
            document: None,
            summaries: Vec::new(),
            selected_id: None,
            selected_note: None,
            draft: text_editor::Content::new(),
            edit_draft: text_editor::Content::new(),
            editing: false,
            busy: false,
            error: None,
        };
        let command = app.update_title();

        (app, command)
    }

    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        vec![
            menu::bar(vec![menu::Tree::with_children(
                menu::root(fl!("view")).apply(Element::from),
                menu::items(
                    &self.key_binds,
                    vec![menu::Item::Button(fl!("about"), None, MenuAction::About)],
                ),
            )])
            .into(),
        ]
    }

    fn header_end(&self) -> Vec<Element<'_, Self::Message>> {
        vec![
            widget::button::text(fl!("new-document"))
                .on_press_maybe((!self.busy && !self.editing).then_some(Message::NewDocument))
                .into(),
            widget::button::suggested(fl!("open-document"))
                .on_press_maybe((!self.busy && !self.editing).then_some(Message::OpenDocument))
                .into(),
        ]
    }

    fn context_drawer(&self) -> Option<context_drawer::ContextDrawer<'_, Self::Message>> {
        if !self.core.window.show_context {
            return None;
        }

        Some(match self.context_page {
            ContextPage::About => context_drawer::about(
                &self.about,
                |url| Message::LaunchUrl(url.to_string()),
                Message::ToggleContextPage(ContextPage::About),
            ),
        })
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let spacing = cosmic::theme::spacing();
        let mut content = widget::column::with_capacity(3)
            .spacing(spacing.space_m)
            .padding(spacing.space_l)
            .width(Length::Fill)
            .height(Length::Fill);

        if let Some(error) = &self.error {
            content = content.push(widget::warning(error).on_close(Message::ClearError));
        }

        if self.document.is_some() {
            content = content.push(self.document_view());
        } else {
            let landing = widget::column::with_capacity(4)
                .push(widget::text::title1(fl!("app-title")))
                .push(widget::text(fl!("landing-description")))
                .push(
                    widget::button::suggested(fl!("new-document"))
                        .on_press_maybe((!self.busy).then_some(Message::NewDocument)),
                )
                .push(
                    widget::button::text(fl!("open-document"))
                        .on_press_maybe((!self.busy).then_some(Message::OpenDocument)),
                )
                .spacing(spacing.space_m)
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
    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::NewDocument => {
                if self.busy || self.editing {
                    return Task::none();
                }
                self.busy = true;
                self.error = None;
                let title = fl!("new-document");
                let filter_name = fl!("puck-documents");

                return cosmic::task::future(async move {
                    let filter = FileFilter::new(&filter_name).glob("*.puck");
                    let result = file_chooser::save::Dialog::new()
                        .title(title)
                        .file_name(String::from("notes.puck"))
                        .filter(filter)
                        .save_file()
                        .await;

                    let result = match result {
                        Ok(response) => match response.url() {
                            Some(url) => match url.to_file_path() {
                                Ok(path) => Document::create(path)
                                    .await
                                    .map(Some)
                                    .map_err(|error| error.to_string()),
                                Err(()) => Err(fl!("invalid-document-path")),
                            },
                            None => Ok(None),
                        },
                        Err(file_chooser::Error::Cancelled) => Ok(None),
                        Err(error) => Err(format!("{error:?}")),
                    };

                    Message::DocumentLoaded(result)
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

                return cosmic::task::future(async move {
                    let filter = FileFilter::new(&filter_name).glob("*.puck");
                    let result = file_chooser::open::Dialog::new()
                        .title(title)
                        .filter(filter)
                        .open_file()
                        .await;

                    let result = match result {
                        Ok(response) => match response.url().to_file_path() {
                            Ok(path) => Document::open(path)
                                .await
                                .map(Some)
                                .map_err(|error| error.to_string()),
                            Err(()) => Err(fl!("invalid-document-path")),
                        },
                        Err(file_chooser::Error::Cancelled) => Ok(None),
                        Err(error) => Err(format!("{error:?}")),
                    };

                    Message::DocumentLoaded(result)
                });
            }
            Message::DocumentLoaded(result) => {
                self.busy = false;
                match result {
                    Ok(Some(document)) => {
                        self.document = Some(document.clone());
                        self.summaries.clear();
                        self.selected_id = None;
                        self.selected_note = None;
                        self.draft = text_editor::Content::new();
                        self.edit_draft = text_editor::Content::new();
                        self.editing = false;
                        self.busy = true;
                        return cosmic::task::batch([
                            self.update_title(),
                            load_summaries(document),
                        ]);
                    }
                    Ok(None) => {}
                    Err(error) => self.error = Some(error),
                }
            }
            Message::SummariesLoaded(result) => {
                self.busy = false;
                match result {
                    Ok(summaries) => {
                        self.summaries = summaries;
                        if self.selected_id.is_none()
                            && let Some(summary) = self.summaries.first()
                        {
                            return self.update(Message::SelectNote(summary.id));
                        }
                    }
                    Err(error) => self.error = Some(error),
                }
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

                return cosmic::task::future(async move {
                    Message::NoteLoaded(
                        id,
                        document
                            .query(NoteById(id))
                            .await
                            .map_err(|error| error.to_string()),
                    )
                });
            }
            Message::NoteLoaded(id, result) => {
                if self.selected_id == Some(id) {
                    match result {
                        Ok(Some(note)) => {
                            self.selected_note = Some(note);
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
                let Some(note) = &self.selected_note else {
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
                let Some(note) = &self.selected_note else {
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
                return cosmic::task::future(async move {
                    let result = document
                        .execute(vec![Command::EditNote(note.clone())])
                        .await
                        .map(|()| note)
                        .map_err(|error| error.to_string());
                    Message::NoteEdited(result)
                });
            }
            Message::AddNote => {
                let Some(document) = self.document.clone() else {
                    return Task::none();
                };
                if self.busy || self.editing {
                    return Task::none();
                }

                self.busy = true;
                self.error = None;
                let note = PileNote::create(self.draft.text());

                return cosmic::task::future(async move {
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
                        self.selected_note = Some(note);
                        self.draft = text_editor::Content::new();
                        if let Some(document) = self.document.clone() {
                            self.busy = true;
                            return load_summaries(document);
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
                        self.selected_note = Some(note);
                        self.edit_draft = text_editor::Content::new();
                        self.editing = false;
                        if let Some(document) = self.document.clone() {
                            self.busy = true;
                            return load_summaries(document);
                        }
                    }
                    Err(error) => self.error = Some(error),
                }
            }
            Message::ClearError => self.error = None,
            Message::ToggleContextPage(context_page) => {
                if self.context_page == context_page {
                    self.core.window.show_context = !self.core.window.show_context;
                } else {
                    self.context_page = context_page;
                    self.core.window.show_context = true;
                }
            }
            Message::LaunchUrl(url) => {
                if let Err(error) = open::that_detached(&url) {
                    self.error = Some(error.to_string());
                }
            }
        }

        Task::none()
    }
}

impl AppModel {
    #[allow(clippy::too_many_lines)]
    fn document_view(&self) -> Element<'_, Message> {
        let spacing = cosmic::theme::spacing();
        let document = self
            .document
            .as_ref()
            .expect("document view requires document");
        let name = document
            .path()
            .file_name()
            .unwrap_or_else(|| document.path().as_os_str())
            .to_string_lossy();

        let mut list = widget::list_column::with_capacity(self.summaries.len());
        for summary in &self.summaries {
            let preview = if summary.preview.is_empty() {
                fl!("untitled-note")
            } else {
                summary.preview.clone()
            };
            list = list.add(
                widget::list_column::button(widget::text(preview))
                    .selected(self.selected_id == Some(summary.id))
                    .on_press_maybe(
                        (!self.editing && !self.busy).then_some(Message::SelectNote(summary.id)),
                    ),
            );
        }

        let notes: Element<_> = if self.summaries.is_empty() {
            widget::text(fl!("no-notes")).into()
        } else {
            widget::scrollable(list).height(Length::Fill).into()
        };

        let sidebar = widget::column::with_capacity(2)
            .push(widget::text::title3(fl!("pile")))
            .push(notes)
            .spacing(spacing.space_s)
            .width(260)
            .height(Length::Fill);

        let body: Element<_> = match (&self.selected_note, self.editing) {
            (Some(note), true) => {
                let editor = widget::text_editor(&self.edit_draft).height(Length::Fill);
                let editor = if self.busy {
                    editor
                } else {
                    editor.on_action(Message::EditDraftEdited)
                };

                widget::column::with_capacity(2)
                    .push(editor)
                    .push(
                        widget::row::with_capacity(2)
                            .push(
                                widget::button::suggested(fl!("save-note")).on_press_maybe(
                                    (!self.busy && self.edit_draft.text() != note.body())
                                        .then_some(Message::SaveNote),
                                ),
                            )
                            .push(
                                widget::button::text(fl!("cancel"))
                                    .on_press_maybe((!self.busy).then_some(Message::CancelEditing)),
                            )
                            .spacing(spacing.space_s),
                    )
                    .spacing(spacing.space_s)
                    .height(Length::Fill)
                    .into()
            }
            (Some(note), false) => widget::column::with_capacity(2)
                .push(
                    widget::scrollable(widget::text(note.body()).width(Length::Fill))
                        .height(Length::Fill),
                )
                .push(
                    widget::button::text(fl!("edit-note"))
                        .on_press_maybe((!self.busy).then_some(Message::EditNote)),
                )
                .spacing(spacing.space_s)
                .height(Length::Fill)
                .into(),
            (None, _) => widget::container(widget::text(fl!("select-note")))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into(),
        };

        let composer = widget::column::with_capacity(3)
            .push(widget::text::title3(fl!("new-note")))
            .push(
                widget::text_editor(&self.draft)
                    .placeholder(fl!("note-placeholder"))
                    .height(140)
                    .on_action(Message::DraftEdited),
            )
            .push(
                widget::button::suggested(fl!("add-note"))
                    .on_press_maybe((!self.busy && !self.editing).then_some(Message::AddNote)),
            )
            .spacing(spacing.space_s);

        let mut main = widget::column::with_capacity(2).push(body);
        if !self.editing {
            main = main.push(composer);
        }
        let main = main
            .spacing(spacing.space_m)
            .width(Length::Fill)
            .height(Length::Fill);

        widget::column::with_capacity(2)
            .push(widget::text::title2(name))
            .push(
                widget::row::with_capacity(2)
                    .push(sidebar)
                    .push(main)
                    .spacing(spacing.space_l)
                    .height(Length::Fill),
            )
            .spacing(spacing.space_m)
            .height(Length::Fill)
            .into()
    }

    fn update_title(&mut self) -> Task<cosmic::Action<Message>> {
        let mut title = fl!("app-title");
        if let Some(document) = &self.document
            && let Some(name) = document.path().file_name()
        {
            title.push_str(" — ");
            title.push_str(&name.to_string_lossy());
        }

        if let Some(id) = self.core.main_window_id() {
            self.set_window_title(title, id)
        } else {
            Task::none()
        }
    }
}

fn load_summaries(document: Document) -> Task<cosmic::Action<Message>> {
    cosmic::task::future(async move {
        Message::SummariesLoaded(
            document
                .query(NoteSummaries)
                .await
                .map_err(|error| error.to_string()),
        )
    })
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum ContextPage {
    #[default]
    About,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuAction {
    About,
}

impl menu::action::MenuAction for MenuAction {
    type Message = Message;

    fn message(&self) -> Self::Message {
        match self {
            Self::About => Message::ToggleContextPage(ContextPage::About),
        }
    }
}
