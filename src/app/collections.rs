// SPDX-FileCopyrightText: 2026 Charles Willis <5862883+trippwill@users.noreply.github.com>
// SPDX-License-Identifier: MPL-2.0

use iced::{Element, Length, Task, widget};

use super::{AppModel, Message as AppMessage};
use crate::core::prelude::*;
use crate::data::prelude::*;
use crate::data::query::{self, RecordDetail, RecordSummary, SourceNote};
use crate::fl;

#[derive(Debug, Default)]
pub(super) struct CollectionsState {
    items: Vec<Collection>,
    selected_id: Option<CollectionId>,
    records: Vec<RecordSummary>,
    pub(super) selected_record_id: Option<RecordId>,
    selected_record: Option<RecordDetail>,
    new_name: String,
}

/// Messages emitted by the collections workspace.
#[derive(Debug, Clone)]
pub enum CollectionMessage {
    CollectionsLoaded(Result<Vec<Collection>, String>),
    Create,
    Created(CollectionId, Result<(), String>),
    NewNameChanged(String),
    RecordLoaded(RecordId, Result<Option<RecordDetail>, String>),
    RecordsLoaded(CollectionId, Result<Vec<RecordSummary>, String>),
    SelectCollection(CollectionId),
    SelectRecord(RecordId),
    SourceLoaded(NoteId, Result<Option<SourceNote>, String>),
    ViewSource(NoteId),
}

impl AppModel {
    pub(super) fn reset_collections(&mut self) {
        self.collections = CollectionsState::default();
    }

    pub(super) fn update_collections(&mut self, message: CollectionMessage) -> Task<AppMessage> {
        match message {
            CollectionMessage::CollectionsLoaded(result) => self.collections_loaded(result),
            CollectionMessage::Create => self.create_collection(),
            CollectionMessage::Created(id, result) => self.collection_created(id, result),
            CollectionMessage::NewNameChanged(name) => {
                self.collections.new_name = name;
                Task::none()
            }
            CollectionMessage::RecordLoaded(id, result) => self.record_loaded(id, result),
            CollectionMessage::RecordsLoaded(id, result) => self.records_loaded(id, result),
            CollectionMessage::SelectCollection(id) => self.select_collection(id),
            CollectionMessage::SelectRecord(id) => self.select_record(id),
            CollectionMessage::SourceLoaded(id, result) => self.source_loaded(id, result),
            CollectionMessage::ViewSource(id) => self.view_source(id),
        }
    }

    pub(super) fn load_collections(&mut self) -> Task<AppMessage> {
        let Some(document) = self.document.clone() else {
            return Task::none();
        };
        self.busy = true;
        self.error = None;
        Task::future(async move {
            AppMessage::Collections(CollectionMessage::CollectionsLoaded(
                document
                    .query(query::Collections)
                    .await
                    .map_err(|error| error.to_string()),
            ))
        })
    }

    fn collections_loaded(&mut self, result: Result<Vec<Collection>, String>) -> Task<AppMessage> {
        self.busy = false;
        match result {
            Ok(collections) => {
                self.collections.items = collections;
                let selected = self
                    .collections
                    .selected_id
                    .filter(|id| self.collections.items.iter().any(|item| item.id() == *id))
                    .or_else(|| self.collections.items.first().map(Collection::id));
                self.collections.selected_id = None;
                self.collections.records.clear();
                self.collections.selected_record_id = None;
                self.collections.selected_record = None;
                if let Some(id) = selected {
                    return self.select_collection(id);
                }
            }
            Err(error) => self.error = Some(error),
        }
        Task::none()
    }

    pub(super) fn select_collection(&mut self, id: CollectionId) -> Task<AppMessage> {
        let Some(document) = self.document.clone() else {
            return Task::none();
        };
        self.collections.selected_id = Some(id);
        self.collections.records.clear();
        self.collections.selected_record_id = None;
        self.collections.selected_record = None;
        self.busy = true;
        self.error = None;
        Task::future(async move {
            AppMessage::Collections(CollectionMessage::RecordsLoaded(
                id,
                document
                    .query(query::RecordSummariesByCollection(id))
                    .await
                    .map_err(|error| error.to_string()),
            ))
        })
    }

    fn records_loaded(
        &mut self,
        id: CollectionId,
        result: Result<Vec<RecordSummary>, String>,
    ) -> Task<AppMessage> {
        if self.collections.selected_id != Some(id) {
            return Task::none();
        }
        self.busy = false;
        match result {
            Ok(records) => {
                self.collections.records = records;
                let selected = self
                    .collections
                    .selected_record_id
                    .filter(|id| {
                        self.collections
                            .records
                            .iter()
                            .any(|record| record.id == *id)
                    })
                    .or_else(|| self.collections.records.first().map(|record| record.id));
                if let Some(id) = selected {
                    return self.select_record(id);
                }
            }
            Err(error) => self.error = Some(error),
        }
        Task::none()
    }

    fn select_record(&mut self, id: RecordId) -> Task<AppMessage> {
        let Some(document) = self.document.clone() else {
            return Task::none();
        };
        self.collections.selected_record_id = Some(id);
        self.collections.selected_record = None;
        self.busy = true;
        self.error = None;
        Task::future(async move {
            AppMessage::Collections(CollectionMessage::RecordLoaded(
                id,
                document
                    .query(query::RecordDetailById(id))
                    .await
                    .map_err(|error| error.to_string()),
            ))
        })
    }

    fn record_loaded(
        &mut self,
        id: RecordId,
        result: Result<Option<RecordDetail>, String>,
    ) -> Task<AppMessage> {
        if self.collections.selected_record_id != Some(id) {
            return Task::none();
        }
        self.busy = false;
        match result {
            Ok(Some(record)) => self.collections.selected_record = Some(record),
            Ok(None) => self.error = Some(fl!("record-not-found")),
            Err(error) => self.error = Some(error),
        }
        Task::none()
    }

    fn view_source(&mut self, id: NoteId) -> Task<AppMessage> {
        let Some(document) = self.document.clone() else {
            return Task::none();
        };
        self.busy = true;
        self.error = None;
        Task::future(async move {
            AppMessage::Collections(CollectionMessage::SourceLoaded(
                id,
                document
                    .query(query::SourceNoteById(id))
                    .await
                    .map_err(|error| error.to_string()),
            ))
        })
    }

    fn source_loaded(
        &mut self,
        id: NoteId,
        result: Result<Option<SourceNote>, String>,
    ) -> Task<AppMessage> {
        self.busy = false;
        let Some(note) = (match result {
            Ok(note) => note,
            Err(error) => {
                self.error = Some(error);
                return Task::none();
            }
        }) else {
            self.error = Some(fl!("note-not-found"));
            return Task::none();
        };
        self.workspace = super::Workspace::Notes;
        self.selected_id = Some(id);
        self.editing = false;
        match note {
            SourceNote::Pile(note) => {
                self.list = super::notes::NoteList::Pile;
                self.selected_note = Some(super::notes::SelectedNote::Pile(note));
            }
            SourceNote::Archive(note) => {
                self.list = super::notes::NoteList::Archive;
                self.selected_note = Some(super::notes::SelectedNote::Archive(note));
            }
        }
        let Some(document) = self.document.clone() else {
            return Task::none();
        };
        self.busy = true;
        super::notes::load_summaries(document, self.list.clone())
    }

    fn create_collection(&mut self) -> Task<AppMessage> {
        if self.busy || self.collections.new_name.trim().is_empty() {
            return Task::none();
        }
        let Some(document) = self.document.clone() else {
            return Task::none();
        };
        let collection = Collection::new(self.collections.new_name.trim());
        let id = collection.id();
        self.busy = true;
        self.error = None;
        Task::future(async move {
            AppMessage::Collections(CollectionMessage::Created(
                id,
                document
                    .execute(vec![Command::UpsertCollection(collection)])
                    .await
                    .map_err(|error| error.to_string()),
            ))
        })
    }

    fn collection_created(
        &mut self,
        id: CollectionId,
        result: Result<(), String>,
    ) -> Task<AppMessage> {
        self.busy = false;
        match result {
            Ok(()) => {
                self.collections.selected_id = Some(id);
                self.collections.new_name.clear();
                self.load_collections()
            }
            Err(error) => {
                self.error = Some(error);
                Task::none()
            }
        }
    }

    pub(super) fn collections_view(&self) -> Element<'_, AppMessage> {
        widget::responsive(|size| {
            if size.width < 760.0 {
                widget::column![
                    self.collection_sidebar(Length::Fill),
                    self.record_list(Length::Fill),
                    self.record_detail(),
                ]
                .spacing(16)
                .height(Length::Fill)
                .into()
            } else {
                widget::row![
                    self.collection_sidebar(220),
                    self.record_list(260),
                    self.record_detail(),
                ]
                .spacing(24)
                .height(Length::Fill)
                .into()
            }
        })
        .into()
    }

    fn collection_sidebar(&self, width: impl Into<Length>) -> Element<'_, AppMessage> {
        let mut items = widget::Column::with_capacity(self.collections.items.len()).spacing(4);
        for collection in &self.collections.items {
            let selected = self.collections.selected_id == Some(collection.id());
            items = items.push(
                widget::button(widget::text(collection.name()))
                    .style(if selected {
                        crate::theme::selected
                    } else {
                        widget::button::text
                    })
                    .width(Length::Fill)
                    .on_press_maybe((!self.busy && !selected).then_some(AppMessage::Collections(
                        CollectionMessage::SelectCollection(collection.id()),
                    ))),
            );
        }
        let list: Element<_> = if self.collections.items.is_empty() {
            widget::text(fl!("no-collections")).into()
        } else {
            widget::scrollable(items).height(Length::Fill).into()
        };
        let input = widget::text_input(&fl!("collection-name"), &self.collections.new_name);
        let input = if self.busy {
            input
        } else {
            input
                .on_input(|name| AppMessage::Collections(CollectionMessage::NewNameChanged(name)))
                .on_submit(AppMessage::Collections(CollectionMessage::Create))
        };

        widget::column![
            widget::text(fl!("collections")).size(24),
            list,
            input,
            widget::button(widget::text(fl!("new-collection")))
                .style(crate::theme::primary_pill)
                .on_press_maybe(
                    (!self.busy && !self.collections.new_name.trim().is_empty())
                        .then_some(AppMessage::Collections(CollectionMessage::Create)),
                ),
        ]
        .spacing(8)
        .width(width)
        .height(Length::Fill)
        .into()
    }

    fn record_list(&self, width: impl Into<Length>) -> Element<'_, AppMessage> {
        let mut records = widget::Column::with_capacity(self.collections.records.len()).spacing(4);
        for record in &self.collections.records {
            let selected = self.collections.selected_record_id == Some(record.id);
            records = records.push(
                widget::button(
                    widget::column![
                        widget::text(&record.label),
                        widget::text(fl!("field-count", count = record.field_count)).size(12),
                    ]
                    .spacing(2),
                )
                .style(if selected {
                    crate::theme::selected
                } else {
                    widget::button::text
                })
                .width(Length::Fill)
                .on_press_maybe((!self.busy && !selected).then_some(AppMessage::Collections(
                    CollectionMessage::SelectRecord(record.id),
                ))),
            );
        }
        let records: Element<_> = if self.collections.selected_id.is_none() {
            widget::text(fl!("select-collection")).into()
        } else if self.collections.records.is_empty() {
            widget::text(fl!("no-records")).into()
        } else {
            widget::scrollable(records).height(Length::Fill).into()
        };

        widget::column![widget::text(fl!("records")).size(24), records]
            .spacing(8)
            .width(width)
            .height(Length::Fill)
            .into()
    }

    fn record_detail(&self) -> Element<'_, AppMessage> {
        let Some(detail) = &self.collections.selected_record else {
            return widget::container(widget::text(fl!("select-record")))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        };

        let mut fields = widget::Column::with_capacity(detail.fields.len()).spacing(8);
        if detail.fields.is_empty() {
            fields = fields.push(widget::text(fl!("no-fields")));
        } else {
            for field in &detail.fields {
                fields = fields.push(
                    widget::container(widget::row![
                        widget::text(&field.name).width(160),
                        widget::text(field_value(&field.field)),
                    ])
                    .padding(12)
                    .width(Length::Fill)
                    .style(crate::theme::panel),
                );
            }
        }

        let source: Element<_> = detail.record.source_note_id().map_or_else(
            || widget::text("").into(),
            |id| {
                widget::button(widget::text(fl!("view-source-note")))
                    .style(widget::button::text)
                    .on_press_maybe(
                        (!self.busy)
                            .then_some(AppMessage::Collections(CollectionMessage::ViewSource(id))),
                    )
                    .into()
            },
        );

        widget::column![
            widget::text(detail.record.label()).size(28),
            widget::text(fl!("field-count", count = detail.fields.len())),
            source,
            widget::scrollable(fields).height(Length::Fill),
        ]
        .spacing(12)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

fn field_value(field: &AnyField) -> String {
    match field {
        AnyField::Text(field) => field.val().clone(),
        AnyField::Boolean(field) => field.val().to_string(),
        AnyField::Integer(field) => field.val().to_string(),
        AnyField::Date(field) => field.val().to_string(),
        AnyField::Time(field) => format!(
            "{:02}:{:02}:{:02}",
            field.val().hour(),
            field.val().minute(),
            field.val().second()
        ),
        AnyField::Timestamp(field) => field.val().to_string(),
    }
}
