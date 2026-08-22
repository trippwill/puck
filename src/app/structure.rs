// SPDX-FileCopyrightText: 2026 Charles Willis <5862883+trippwill@users.noreply.github.com>
// SPDX-License-Identifier: MPL-2.0

use iced::widget::{self, text_editor};
use iced::{Element, Length, Task};

use super::notes::SelectedNote;
use super::{AppModel, Message as AppMessage, Workspace};
use crate::core::prelude::*;
use crate::data::prelude::*;
use crate::data::query::{self, CollectionFieldDef};
use crate::fl;

pub(super) struct StructureDraft {
    source_note: PileNote,
    source: text_editor::Content,
    step: Step,
    collections: Vec<Collection>,
    selected_collection_id: Option<CollectionId>,
    new_collection_id: Option<CollectionId>,
    new_collection_name: String,
    label: String,
    record: Option<Record>,
    definitions: Vec<CollectionFieldDef>,
    selected_definition_id: Option<FieldDefId>,
    new_definition: bool,
    new_definition_name: String,
    new_kind: FieldKind,
    value: String,
    fields: Vec<DraftField>,
    field_error: Option<String>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Step {
    ChooseCollection,
    Build,
}

#[derive(Debug, Clone)]
struct DraftField {
    definition: AnyFieldDef,
    field: AnyField,
    raw_value: String,
    definition_is_new: bool,
}

/// Messages emitted while structuring a note.
#[derive(Debug, Clone)]
pub enum StructureMessage {
    AddField,
    Cancel,
    CollectionsLoaded(Result<Vec<Collection>, String>),
    Continue,
    CreateCollection,
    DefinitionsLoaded(CollectionId, Result<Vec<CollectionFieldDef>, String>),
    Finish,
    LabelChanged(String),
    NewCollectionChanged(String),
    NewDefinitionNameChanged(String),
    NewKindChanged(FieldKind),
    SaveCompleted(CollectionId, RecordId, Result<(), String>),
    SelectCollection(CollectionId),
    SelectDefinition(FieldDefId),
    SelectNewDefinition,
    SourceAction(text_editor::Action),
    UseSelection,
    ValueChanged(String),
}

impl AppModel {
    pub(super) fn start_structure(&mut self) -> Task<AppMessage> {
        let Some(SelectedNote::Pile(note)) = &self.selected_note else {
            return Task::none();
        };
        let label = note
            .body()
            .lines()
            .find(|line| !line.trim().is_empty())
            .map_or_else(String::new, |line| line.trim().to_owned());
        self.structure = Some(StructureDraft {
            source_note: note.clone(),
            source: text_editor::Content::with_text(note.body()),
            step: Step::ChooseCollection,
            collections: Vec::new(),
            selected_collection_id: None,
            new_collection_id: None,
            new_collection_name: String::new(),
            label,
            record: None,
            definitions: Vec::new(),
            selected_definition_id: None,
            new_definition: false,
            new_definition_name: String::new(),
            new_kind: FieldKind::Text,
            value: String::new(),
            fields: Vec::new(),
            field_error: None,
        });
        self.workspace = Workspace::Structure;
        self.load_structure_collections()
    }

    pub(super) fn update_structure(&mut self, message: StructureMessage) -> Task<AppMessage> {
        match message {
            StructureMessage::AddField => self.add_draft_field(),
            StructureMessage::Cancel => {
                self.structure = None;
                self.workspace = Workspace::Notes;
                Task::none()
            }
            StructureMessage::CollectionsLoaded(result) => {
                self.structure_collections_loaded(result)
            }
            StructureMessage::Continue => self.continue_structure(),
            StructureMessage::CreateCollection => {
                self.create_structure_collection();
                Task::none()
            }
            StructureMessage::DefinitionsLoaded(id, result) => {
                self.structure_definitions_loaded(id, result)
            }
            StructureMessage::Finish => self.finish_structure(),
            StructureMessage::LabelChanged(label) => {
                if let Some(draft) = &mut self.structure {
                    draft.label = label;
                    draft.field_error = None;
                }
                Task::none()
            }
            StructureMessage::NewCollectionChanged(name) => {
                if let Some(draft) = &mut self.structure {
                    draft.new_collection_name = name;
                }
                Task::none()
            }
            StructureMessage::NewDefinitionNameChanged(name) => {
                if let Some(draft) = &mut self.structure {
                    draft.new_definition_name = name;
                    draft.field_error = None;
                }
                Task::none()
            }
            StructureMessage::NewKindChanged(kind) => {
                if let Some(draft) = &mut self.structure {
                    draft.new_kind = kind;
                    draft.field_error = None;
                }
                Task::none()
            }
            StructureMessage::SaveCompleted(collection_id, record_id, result) => {
                self.structure_saved(collection_id, record_id, result)
            }
            StructureMessage::SelectCollection(id) => {
                if let Some(draft) = &mut self.structure {
                    draft.selected_collection_id = Some(id);
                }
                Task::none()
            }
            StructureMessage::SelectDefinition(id) => {
                if let Some(draft) = &mut self.structure {
                    draft.selected_definition_id = Some(id);
                    draft.new_definition = false;
                    draft.field_error = None;
                }
                Task::none()
            }
            StructureMessage::SelectNewDefinition => {
                if let Some(draft) = &mut self.structure {
                    draft.selected_definition_id = None;
                    draft.new_definition = true;
                    draft.field_error = None;
                }
                Task::none()
            }
            StructureMessage::SourceAction(action) => {
                if let Some(draft) = &mut self.structure {
                    perform_source_action(&mut draft.source, action);
                }
                Task::none()
            }
            StructureMessage::UseSelection => {
                if let Some(draft) = &mut self.structure
                    && let Some(selection) = draft.source.selection()
                    && !selection.is_empty()
                {
                    draft.value = selection;
                    draft.field_error = None;
                }
                Task::none()
            }
            StructureMessage::ValueChanged(value) => {
                if let Some(draft) = &mut self.structure {
                    draft.value = value;
                    draft.field_error = None;
                }
                Task::none()
            }
        }
    }

    fn load_structure_collections(&mut self) -> Task<AppMessage> {
        let Some(document) = self.document.clone() else {
            return Task::none();
        };
        self.busy = true;
        self.error = None;
        Task::future(async move {
            AppMessage::Structure(StructureMessage::CollectionsLoaded(
                document
                    .query(query::Collections)
                    .await
                    .map_err(|error| error.to_string()),
            ))
        })
    }

    fn structure_collections_loaded(
        &mut self,
        result: Result<Vec<Collection>, String>,
    ) -> Task<AppMessage> {
        self.busy = false;
        match result {
            Ok(collections) => {
                if let Some(draft) = &mut self.structure {
                    let selected = draft
                        .selected_collection_id
                        .filter(|id| collections.iter().any(|item| item.id() == *id))
                        .or_else(|| collections.first().map(Collection::id));
                    draft.collections = collections;
                    draft.selected_collection_id = selected;
                }
            }
            Err(error) => self.error = Some(error),
        }
        Task::none()
    }

    fn create_structure_collection(&mut self) {
        let Some(draft) = &mut self.structure else {
            return;
        };
        let name = draft.new_collection_name.trim();
        if self.busy || name.is_empty() {
            return;
        }
        let collection = Collection::new(name);
        let id = collection.id();
        if let Some(previous) = draft.new_collection_id {
            draft
                .collections
                .retain(|collection| collection.id() != previous);
        }
        draft.collections.push(collection);
        draft.selected_collection_id = Some(id);
        draft.new_collection_id = Some(id);
        draft.new_collection_name.clear();
    }

    fn continue_structure(&mut self) -> Task<AppMessage> {
        let Some(document) = self.document.clone() else {
            return Task::none();
        };
        let Some(draft) = &mut self.structure else {
            return Task::none();
        };
        let Some(collection_id) = draft.selected_collection_id else {
            return Task::none();
        };
        let Some(collection) = draft
            .collections
            .iter()
            .find(|collection| collection.id() == collection_id)
        else {
            return Task::none();
        };
        let mut record = match collection.new_record(&draft.label) {
            Ok(record) => record,
            Err(error) => {
                draft.field_error = Some(error.to_string());
                return Task::none();
            }
        };
        record.set_source_note_id(Some(draft.source_note.id()));
        draft.record = Some(record);
        self.busy = true;
        self.error = None;
        Task::future(async move {
            AppMessage::Structure(StructureMessage::DefinitionsLoaded(
                collection_id,
                document
                    .query(query::FieldDefsForCollection(collection_id))
                    .await
                    .map_err(|error| error.to_string()),
            ))
        })
    }

    fn structure_definitions_loaded(
        &mut self,
        collection_id: CollectionId,
        result: Result<Vec<CollectionFieldDef>, String>,
    ) -> Task<AppMessage> {
        let Some(draft) = &mut self.structure else {
            return Task::none();
        };
        if draft.selected_collection_id != Some(collection_id) {
            return Task::none();
        }
        self.busy = false;
        match result {
            Ok(definitions) => {
                draft.definitions = definitions;
                draft.step = Step::Build;
            }
            Err(error) => self.error = Some(error),
        }
        Task::none()
    }

    fn add_draft_field(&mut self) -> Task<AppMessage> {
        let Some(draft) = &mut self.structure else {
            return Task::none();
        };
        let Some(record) = &draft.record else {
            return Task::none();
        };
        let (definition, definition_is_new) = if draft.new_definition {
            let name = draft.new_definition_name.trim();
            if name.is_empty() {
                draft.field_error = Some(fl!("field-name-required"));
                return Task::none();
            }
            if draft.definitions.iter().any(|choice| {
                choice.definition.name() == name && choice.definition.kind() == draft.new_kind
            }) || draft.fields.iter().any(|field| {
                field.definition.name() == name && field.definition.kind() == draft.new_kind
            }) {
                draft.field_error = Some(fl!("field-definition-exists"));
                return Task::none();
            }
            (new_definition(draft.new_kind, name), true)
        } else {
            let Some(id) = draft.selected_definition_id else {
                draft.field_error = Some(fl!("select-field-definition"));
                return Task::none();
            };
            let Some(definition) = draft
                .definitions
                .iter()
                .find(|choice| choice.definition.id() == id)
                .map(|choice| choice.definition.clone())
            else {
                return Task::none();
            };
            (definition, false)
        };
        if draft
            .fields
            .iter()
            .any(|field| field.definition.id() == definition.id())
        {
            draft.field_error = Some(fl!("field-already-added"));
            return Task::none();
        }
        match definition.new_field_from_str(record, &draft.value) {
            Ok(field) => {
                draft.fields.push(DraftField {
                    definition,
                    field,
                    raw_value: draft.value.clone(),
                    definition_is_new,
                });
                draft.selected_definition_id = None;
                draft.new_definition = false;
                draft.new_definition_name.clear();
                draft.value.clear();
                draft.field_error = None;
            }
            Err(error) => draft.field_error = Some(error.to_string()),
        }
        Task::none()
    }

    fn finish_structure(&mut self) -> Task<AppMessage> {
        let Some(document) = self.document.clone() else {
            return Task::none();
        };
        let Some(draft) = &self.structure else {
            return Task::none();
        };
        let Some(mut record) = draft.record.clone() else {
            return Task::none();
        };
        if let Err(error) = record.set_label(&draft.label) {
            if let Some(draft) = &mut self.structure {
                draft.field_error = Some(error.to_string());
            }
            return Task::none();
        }
        let collection_id = record.collection_id();
        let record_id = record.id();
        let mut commands = Vec::with_capacity(2 + draft.fields.len() * 2);
        if draft.new_collection_id == Some(collection_id)
            && let Some(collection) = draft
                .collections
                .iter()
                .find(|collection| collection.id() == collection_id)
        {
            commands.push(Command::UpsertCollection(collection.clone()));
        }
        commands.push(Command::UpsertRecord(record));
        for draft_field in &draft.fields {
            if draft_field.definition_is_new {
                commands.push(Command::UpsertFieldDef(draft_field.definition.clone()));
            }
            commands.push(Command::UpsertField(draft_field.field.clone()));
        }
        self.busy = true;
        self.error = None;
        Task::future(async move {
            AppMessage::Structure(StructureMessage::SaveCompleted(
                collection_id,
                record_id,
                document
                    .execute(commands)
                    .await
                    .map_err(|error| error.to_string()),
            ))
        })
    }

    fn structure_saved(
        &mut self,
        collection_id: CollectionId,
        record_id: RecordId,
        result: Result<(), String>,
    ) -> Task<AppMessage> {
        self.busy = false;
        match result {
            Ok(()) => {
                self.structure = None;
                self.workspace = Workspace::Collections;
                let task = self.select_collection(collection_id);
                self.collections.selected_record_id = Some(record_id);
                task
            }
            Err(error) => {
                if let Some(draft) = &mut self.structure {
                    draft.field_error = Some(error);
                }
                Task::none()
            }
        }
    }

    pub(super) fn structure_view(&self) -> Element<'_, AppMessage> {
        let draft = self
            .structure
            .as_ref()
            .expect("structure workspace requires a draft");
        match draft.step {
            Step::ChooseCollection => self.choose_collection_view(draft),
            Step::Build => self.build_structure_view(draft),
        }
    }

    fn choose_collection_view<'a>(&'a self, draft: &'a StructureDraft) -> Element<'a, AppMessage> {
        let mut collections = widget::Column::with_capacity(draft.collections.len()).spacing(4);
        for collection in &draft.collections {
            let selected = draft.selected_collection_id == Some(collection.id());
            collections = collections.push(
                widget::button(widget::text(collection.name()))
                    .style(if selected {
                        crate::theme::selected
                    } else {
                        widget::button::text
                    })
                    .width(Length::Fill)
                    .on_press_maybe((!self.busy && !selected).then_some(AppMessage::Structure(
                        StructureMessage::SelectCollection(collection.id()),
                    ))),
            );
        }
        let collections: Element<_> = if draft.collections.is_empty() {
            widget::text(fl!("no-collections")).into()
        } else {
            widget::scrollable(collections).height(Length::Fill).into()
        };

        widget::column![
            widget::text(fl!("add-structure")).size(32),
            widget::text(fl!("record-label")),
            widget::text_input(&fl!("record-label"), &draft.label)
                .on_input(|label| AppMessage::Structure(StructureMessage::LabelChanged(label))),
            widget::text(fl!("choose-collection")).size(20),
            collections,
            widget::row![
                widget::text_input(&fl!("collection-name"), &draft.new_collection_name).on_input(
                    |name| AppMessage::Structure(StructureMessage::NewCollectionChanged(name))
                ),
                widget::button(widget::text(fl!("new-collection"))).on_press_maybe(
                    (!self.busy && !draft.new_collection_name.trim().is_empty())
                        .then_some(AppMessage::Structure(StructureMessage::CreateCollection,)),
                ),
            ]
            .spacing(8),
            error_view(draft.field_error.as_deref()),
            widget::row![
                widget::button(widget::text(fl!("cancel")))
                    .style(widget::button::text)
                    .on_press(AppMessage::Structure(StructureMessage::Cancel)),
                widget::Space::new().width(Length::Fill),
                widget::button(widget::text(fl!("continue")))
                    .style(crate::theme::primary_pill)
                    .on_press_maybe(
                        (!self.busy
                            && draft.selected_collection_id.is_some()
                            && !draft.label.trim().is_empty())
                        .then_some(AppMessage::Structure(StructureMessage::Continue)),
                    ),
            ]
            .align_y(iced::Alignment::Center),
        ]
        .spacing(12)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn build_structure_view<'a>(&'a self, draft: &'a StructureDraft) -> Element<'a, AppMessage> {
        widget::responsive(move |size| {
            if size.width < 760.0 {
                widget::column![
                    self.structure_source_view(draft, Length::Fill),
                    self.field_builder_view(draft, Length::Fill),
                ]
                .spacing(16)
                .height(Length::Fill)
                .into()
            } else {
                widget::row![
                    self.structure_source_view(draft, Length::FillPortion(3)),
                    self.field_builder_view(draft, Length::FillPortion(2)),
                ]
                .spacing(24)
                .height(Length::Fill)
                .into()
            }
        })
        .into()
    }

    fn structure_source_view<'a>(
        &'a self,
        draft: &'a StructureDraft,
        width: Length,
    ) -> Element<'a, AppMessage> {
        widget::column![
            widget::text(fl!("source-note")).size(16),
            widget::text_editor(&draft.source)
                .height(Length::Fill)
                .on_action(|action| {
                    AppMessage::Structure(StructureMessage::SourceAction(action))
                }),
            widget::button(widget::text(fl!("use-selection")))
                .style(crate::theme::primary_pill)
                .on_press_maybe(
                    (!self.busy
                        && draft
                            .source
                            .selection()
                            .is_some_and(|text| !text.is_empty()))
                    .then_some(AppMessage::Structure(StructureMessage::UseSelection)),
                ),
        ]
        .spacing(8)
        .width(width)
        .height(Length::Fill)
        .into()
    }

    fn field_builder_view<'a>(
        &'a self,
        draft: &'a StructureDraft,
        width: Length,
    ) -> Element<'a, AppMessage> {
        widget::column![
            widget::text_input(&fl!("record-label"), &draft.label)
                .on_input(|label| AppMessage::Structure(StructureMessage::LabelChanged(label))),
            widget::text(fl!("choose-field-definition")).size(16),
            widget::scrollable(self.definition_choices(draft)).height(130),
            self.field_form(draft),
            error_view(draft.field_error.as_deref()),
            widget::text(fl!("record-fields")).size(16),
            widget::scrollable(Self::draft_fields(draft)).height(Length::Fill),
            widget::row![
                widget::button(widget::text(fl!("cancel")))
                    .style(widget::button::text)
                    .on_press(AppMessage::Structure(StructureMessage::Cancel)),
                widget::Space::new().width(Length::Fill),
                widget::button(widget::text(fl!("finish")))
                    .style(crate::theme::primary_pill)
                    .on_press_maybe(
                        (!self.busy && !draft.label.trim().is_empty())
                            .then_some(AppMessage::Structure(StructureMessage::Finish)),
                    ),
            ],
        ]
        .spacing(8)
        .width(width)
        .height(Length::Fill)
        .into()
    }

    fn definition_choices<'a>(&'a self, draft: &'a StructureDraft) -> Element<'a, AppMessage> {
        let mut definitions = widget::Column::with_capacity(draft.definitions.len() + 1).spacing(4);
        for choice in &draft.definitions {
            let id = choice.definition.id();
            let added = draft.fields.iter().any(|field| field.definition.id() == id);
            let selected = draft.selected_definition_id == Some(id) && !draft.new_definition;
            let label = if choice.used_in_collection {
                format!(
                    "{} · {} · {}",
                    choice.definition.name(),
                    choice.definition.kind(),
                    fl!("used-here")
                )
            } else {
                format!(
                    "{} · {}",
                    choice.definition.name(),
                    choice.definition.kind()
                )
            };
            definitions = definitions.push(
                widget::button(widget::text(label))
                    .style(if selected {
                        crate::theme::primary_pill
                    } else if choice.used_in_collection {
                        crate::theme::selected
                    } else {
                        widget::button::text
                    })
                    .width(Length::Fill)
                    .on_press_maybe((!self.busy && !added && !selected).then_some(
                        AppMessage::Structure(StructureMessage::SelectDefinition(id)),
                    )),
            );
        }
        definitions = definitions.push(
            widget::button(widget::text(fl!("new-field-definition")))
                .style(if draft.new_definition {
                    crate::theme::primary_pill
                } else {
                    widget::button::text
                })
                .width(Length::Fill)
                .on_press_maybe(
                    (!self.busy && !draft.new_definition)
                        .then_some(AppMessage::Structure(StructureMessage::SelectNewDefinition)),
                ),
        );
        definitions.into()
    }

    fn field_form<'a>(&'a self, draft: &'a StructureDraft) -> Element<'a, AppMessage> {
        let mut form = widget::Column::new().spacing(8);
        if draft.new_definition {
            form = form
                .push(
                    widget::text_input(&fl!("field-name"), &draft.new_definition_name).on_input(
                        |name| {
                            AppMessage::Structure(StructureMessage::NewDefinitionNameChanged(name))
                        },
                    ),
                )
                .push(widget::pick_list(
                    FieldKind::ALL,
                    Some(draft.new_kind),
                    |kind| AppMessage::Structure(StructureMessage::NewKindChanged(kind)),
                ));
        }
        form = form
            .push(
                widget::text_input(&fl!("field-value"), &draft.value)
                    .on_input(|value| AppMessage::Structure(StructureMessage::ValueChanged(value))),
            )
            .push(
                widget::button(widget::text(fl!("add-field")))
                    .style(crate::theme::primary_pill)
                    .on_press_maybe(
                        (!self.busy
                            && !draft.value.is_empty()
                            && (draft.new_definition || draft.selected_definition_id.is_some()))
                        .then_some(AppMessage::Structure(StructureMessage::AddField)),
                    ),
            );
        form.into()
    }

    fn draft_fields(draft: &StructureDraft) -> Element<'_, AppMessage> {
        let mut fields = widget::Column::with_capacity(draft.fields.len()).spacing(4);
        for field in &draft.fields {
            fields = fields.push(
                widget::container(widget::row![
                    widget::text(field.definition.name()).width(140),
                    widget::text(&field.raw_value),
                ])
                .padding(8)
                .style(crate::theme::panel),
            );
        }
        if draft.fields.is_empty() {
            fields = fields.push(widget::text(fl!("no-fields")));
        }
        fields.into()
    }
}

fn new_definition(kind: FieldKind, name: &str) -> AnyFieldDef {
    match kind {
        FieldKind::Text => AnyFieldDef::Text(Text::def(name)),
        FieldKind::Boolean => AnyFieldDef::Boolean(Boolean::def(name)),
        FieldKind::Integer => AnyFieldDef::Integer(Integer::def(name)),
        FieldKind::Date => AnyFieldDef::Date(Date::def(name)),
        FieldKind::Time => AnyFieldDef::Time(Time::def(name)),
        FieldKind::Timestamp => AnyFieldDef::Timestamp(Timestamp::def(name)),
    }
}

fn error_view(error: Option<&str>) -> Element<'_, AppMessage> {
    match error {
        Some(error) => widget::text(error).style(widget::text::danger).into(),
        None => widget::text("").into(),
    }
}

fn perform_source_action(content: &mut text_editor::Content, action: text_editor::Action) {
    if !action.is_edit() {
        content.perform(action);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_editor_ignores_edit_actions() {
        let mut content = text_editor::Content::with_text("Keep me");

        perform_source_action(
            &mut content,
            text_editor::Action::Edit(text_editor::Edit::Insert('!')),
        );

        assert_eq!(content.text(), "Keep me");
    }
}
