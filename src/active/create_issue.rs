use std::{
    fmt::{Display, Formatter, Result as FmtResult, Write},
    mem,
};

use eyre::{ContextCompat, Result, WrapErr};
use futures::future::{self, BoxFuture};
use octocrab::models::issues::Issue;
use twilight_model::{
    channel::{
        message::{
            component::{ActionRow, Button, ButtonStyle, SelectMenu, SelectMenuOption},
            embed::EmbedField,
            Component,
        },
        Message,
    },
    id::{
        marker::{ChannelMarker, GuildMarker, MessageMarker},
        Id,
    },
};

use crate::{
    core::Context,
    util::{
        builder::{EmbedBuilder, ModalBuilder, TextInputBuilder},
        interaction::{InteractionComponent, InteractionModal},
    },
    CONFIG,
};

use super::{ComponentResult, IActiveMessage};

pub struct CreateIssue {
    author: Option<String>,
    origin_content: String,
    source: SourceMessageUrl,
    attachments: Vec<String>,
    title: Option<String>,
    repositories: Vec<String>,
    active_repository: String,
    labels: Vec<Label>,
    status: CreateIssueStatus,
}

enum CreateIssueStatus {
    Creating,
    Ready,
    Done { url: String },
}

impl CreateIssue {
    pub fn new(msg: Message, guild: Id<GuildMarker>) -> Self {
        Self {
            author: Some(msg.author.name),
            origin_content: msg.content,
            source: SourceMessageUrl {
                guild,
                channel: msg.channel_id,
                msg: msg.id,
            },
            attachments: msg
                .attachments
                .into_iter()
                .map(|attachment| attachment.url)
                .collect(),
            title: None,
            repositories: Vec::new(),
            active_repository: String::new(),
            labels: Vec::new(),
            status: CreateIssueStatus::Creating,
        }
    }

    async fn create_issue(&self, ctx: &Context) -> Result<Issue> {
        let Some(title) = self.title.clone() else {
            bail!("Missing issue title");
        };

        let labels: Vec<_> = self.labels.iter().map(Label::to_string).collect();
        let builder = ctx.github.issues(
            ctx.config.github_config.owner.clone(),
            self.active_repository.clone(),
        );
        let mut body = String::with_capacity(self.origin_content.len() + 32);

        if !self.origin_content.is_empty() {
            for line in self.origin_content.lines() {
                let _ = writeln!(body, "> {line}");
            }

            body.push('\n');
        }

        if !self.attachments.is_empty() {
            body.push_str("Attachments:\n");

            for url in self.attachments.iter() {
                let _ = writeln!(body, "![attachment]({url})");
            }
        }

        let _ = match self.author {
            Some(ref author) => {
                write!(body, "[Original message by @{author}]({})", self.source)
            }
            None => write!(body, "[Original message]({})", self.source),
        };

        builder
            .create(title.to_owned())
            .body(body)
            .labels(labels)
            .send()
            .await
            .wrap_err("Failed to create issue")
    }
}

impl IActiveMessage for CreateIssue {
    fn build_page<'a>(&'a mut self, ctx: &'a Context) -> BoxFuture<'a, Result<EmbedBuilder>> {
        let issue_url = match self.status {
            CreateIssueStatus::Creating => None,
            CreateIssueStatus::Done { ref mut url } => Some(mem::take(url)),
            CreateIssueStatus::Ready => {
                let fut = async move {
                    let issue = self.create_issue(ctx).await?;

                    self.status = CreateIssueStatus::Done {
                        url: issue.html_url.to_string(),
                    };

                    self.build_page(ctx).await
                };

                return Box::pin(fut);
            }
        };

        let msg = EmbedField {
            inline: true,
            name: "Message".to_owned(),
            value: format!("[Jump]({})", self.source),
        };

        let author = self.author.as_ref().map(|author| EmbedField {
            inline: true,
            name: "Author".to_owned(),
            value: format!("`@{author}`"),
        });

        let title = EmbedField {
            inline: false,
            name: "Issue title".to_owned(),
            value: match self.title.as_ref() {
                Some(title) => title.to_owned(),
                None => "-".to_owned(),
            },
        };

        let labels = EmbedField {
            inline: false,
            name: "Labels".to_owned(),
            value: match self.labels.as_slice() {
                [] => "-".to_owned(),
                labels => Label::list_to_str(labels),
            },
        };

        let repository = EmbedField {
            inline: false,
            name: "Repository".to_owned(),
            value: self.active_repository.clone(),
        };

        let mut fields = match author {
            Some(author) => vec![msg, author, title, labels, repository],
            None => vec![msg, title, labels],
        };

        if let Some(url) = issue_url {
            let issue = EmbedField {
                inline: false,
                name: "Issue created".to_owned(),
                value: format!("[**Link**]({url})"),
            };

            fields.push(issue);
        }

        let embed = EmbedBuilder::new()
            .title("Github issue builder")
            .fields(fields);

        Box::pin(future::ready(Ok(embed)))
    }

    fn build_components(&self) -> Vec<Component> {
        if matches!(
            self.status,
            CreateIssueStatus::Ready | CreateIssueStatus::Done { .. }
        ) {
            return Vec::new();
        }

        let title = Button {
            custom_id: Some("issue_title".to_owned()),
            disabled: false,
            emoji: None,
            label: Some("Issue title".to_owned()),
            style: ButtonStyle::Primary,
            url: None,
        };

        let author = Button {
            custom_id: Some("issue_author".to_owned()),
            disabled: false,
            emoji: None,
            label: Some("Author".to_owned()),
            style: ButtonStyle::Secondary,
            url: None,
        };

        let create = Button {
            custom_id: Some("issue_create".to_owned()),
            disabled: self.title.is_none() || self.labels.is_empty(),
            emoji: None,
            label: Some("Create".to_owned()),
            style: ButtonStyle::Success,
            url: None,
        };

        let button_row = ActionRow {
            components: vec![
                Component::Button(title),
                Component::Button(author),
                Component::Button(create),
            ],
        };

        let repository_options: Vec<_> = CONFIG
            .get()
            .unwrap()
            .github_config
            .repositories
            .clone()
            .into_iter()
            .map(|(k, v)| SelectMenuOption {
                default: false,
                description: None,
                emoji: None,
                value: v.to_string(),
                label: k.to_string(),
            })
            .collect();

        let repositories = SelectMenu {
            custom_id: "issue_repository".to_owned(),
            disabled: false,
            max_values: Some(1),
            min_values: Some(1),
            options: repository_options,
            placeholder: Some("Select one repository".to_owned()),
        };

        let label_options: Vec<_> = CONFIG
            .get()
            .unwrap()
            .issue_labels
            .issue_types
            .clone()
            .into_iter()
            .map(|label| SelectMenuOption {
                default: false,
                description: None,
                emoji: None,
                value: label.replace(' ', "_"),
                label: label.clone(),
            })
            .collect();

        let labels = SelectMenu {
            custom_id: "issue_labels".to_owned(),
            disabled: false,
            max_values: Some(label_options.len() as u8),
            min_values: Some(1),
            options: label_options,
            placeholder: Some("Select at least one label".to_owned()),
        };

        let labels_row = ActionRow {
            components: vec![Component::SelectMenu(labels)],
        };

        let repositories_row = ActionRow {
            components: vec![Component::SelectMenu(repositories)],
        };

        vec![
            Component::ActionRow(button_row),
            Component::ActionRow(labels_row),
            Component::ActionRow(repositories_row),
        ]
    }

    fn handle_component(
        &mut self,
        component: &mut InteractionComponent,
    ) -> BoxFuture<'static, ComponentResult> {
        fn inner(this: &mut CreateIssue, component: &mut InteractionComponent) -> ComponentResult {
            match component.data.custom_id.as_str() {
                "issue_title" => {
                    let input = TextInputBuilder::new("title", "Title")
                        .required(true)
                        .max_len(64);

                    let modal = ModalBuilder::new("issue_title", "Specify a title for the issue")
                        .input(input);

                    ComponentResult::CreateModal(modal)
                }
                "issue_author" => {
                    let input = TextInputBuilder::new("author", "Author")
                        .required(false)
                        .max_len(32);

                    let modal =
                        ModalBuilder::new("issue_author", "Specify an author for the issue")
                            .input(input);

                    ComponentResult::CreateModal(modal)
                }
                "issue_repository" => {
                    this.active_repository.clear();

                    this.active_repository = component.data.values[0].clone();

                    ComponentResult::BuildPage
                }
                "issue_create" => {
                    this.status = CreateIssueStatus::Ready;

                    ComponentResult::BuildPage
                }
                "issue_labels" => {
                    this.labels.clear();

                    this.labels
                        .extend(component.data.values.drain(..).map(Label::new));

                    ComponentResult::BuildPage
                }
                other => ComponentResult::Err(eyre!("Unknown component `{other}`")),
            }
        }

        Box::pin(future::ready(inner(self, component)))
    }

    fn handle_modal(&mut self, modal: &mut InteractionModal) -> BoxFuture<'static, Result<()>> {
        fn inner(this: &mut CreateIssue, modal: &mut InteractionModal) -> Result<()> {
            match modal.data.custom_id.as_str() {
                "issue_title" => {
                    this.title = modal
                        .data
                        .components
                        .first_mut()
                        .and_then(|row| row.components.first_mut())
                        .wrap_err("Missing modal input")?
                        .value
                        .take();

                    Ok(())
                }
                "issue_author" => {
                    this.author = modal
                        .data
                        .components
                        .first_mut()
                        .and_then(|row| row.components.first_mut())
                        .and_then(|component| component.value.take())
                        .filter(|value| !value.is_empty());

                    Ok(())
                }
                other => Err(eyre!("Unknown modal `{other}`")),
            }
        }

        Box::pin(future::ready(inner(self, modal)))
    }
}

struct SourceMessageUrl {
    guild: Id<GuildMarker>,
    channel: Id<ChannelMarker>,
    msg: Id<MessageMarker>,
}

impl Display for SourceMessageUrl {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let Self {
            guild,
            channel,
            msg,
        } = self;

        write!(f, "https://discord.com/channels/{guild}/{channel}/{msg}")
    }
}

pub struct Label(String);

impl Label {
    fn new(label: String) -> Self {
        Self(label.replace('_', " "))
    }

    fn list_to_str(labels: &[Self]) -> String {
        let mut res = String::new();
        let mut labels = labels.iter();

        if let Some(Label(label)) = labels.next() {
            let _ = write!(res, "{label}");

            for Label(label) in labels {
                let _ = write!(res, ", {label}");
            }
        }

        res
    }
}

impl Display for Label {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str(&self.0)
    }
}
