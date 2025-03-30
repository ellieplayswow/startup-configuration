// SPDX-License-Identifier: GPL-3

use crate::apps::{get_installed_programs, get_startup_applications, DirectoryType};
use crate::config::Config;
use crate::fl;
use cosmic::app::{context_drawer, Core, Task};
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, Length, Subscription};
use cosmic::iced_core::widget::Text;
use cosmic::style::Button;
use cosmic::theme::Container::List;
use cosmic::widget::{self, button, column, icon, list_column, row, vertical_space};
use cosmic::{theme, Application, ApplicationExt, Element, Renderer, Theme};
use freedesktop_desktop_entry::DesktopEntry;
use futures_util::SinkExt;
use std::collections::HashMap;
//const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
//const APP_ICON: &[u8] = include_bytes!("../resources/icons/hicolor/scalable/apps/icon.svg");

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime.
    core: Core,
    // Configuration data that persists between application runs.
    config: Config,

    context_page: ContextPage,
    application_search: String,

    locales: Vec<String>,
    installed_apps: Vec<DesktopEntry>,

    apps_per_type: HashMap<DirectoryType, Vec<DesktopEntry>>,

    selected_type: Option<DirectoryType>,
    selected_index: Option<usize>,

    // global search
    global_search: Option<String>,
    global_search_id: widget::Id,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    SubscriptionChannel,
    UpdateConfig(Config),
    ToggleContextPage(ContextPage),

    ApplicationSearch(String),
    AddApplication(DesktopEntry),

    RemoveApplication(usize),
    RemoveApplicationConfirm(usize),
    RemoveApplicationCancel,

    // global search
    GlobalSearchActivate,
    GlobalSearchInput(String),
    GlobalSearchClear,
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum ContextPage {
    #[default]
    AddApplication,
}

/// Create a COSMIC application from the app model
impl Application for AppModel {
    /// The async executor that will be used to run your application's commands.
    type Executor = cosmic::executor::Default;

    /// Data that your application receives to its init method.
    type Flags = ();

    /// Messages which the application and its widgets will emit.
    type Message = Message;

    /// Unique identifier in RDNN (reverse domain name notation) format.
    const APP_ID: &'static str = "best.ellie.CosmicStartup";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    /// Initializes the application with any given flags and startup commands.
    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Self::Message>) {
        let locales = freedesktop_desktop_entry::get_languages_from_env();

        let mut apps_hash = HashMap::with_capacity(2);
        apps_hash.insert(
            DirectoryType::User,
            get_startup_applications(DirectoryType::User, locales.clone()),
        );
        apps_hash.insert(
            DirectoryType::System,
            get_startup_applications(DirectoryType::System, locales.clone()),
        );

        // Construct the app model with the runtime's core.
        let mut app = AppModel {
            core,
            context_page: ContextPage::default(),
            locales: locales.clone(),
            installed_apps: get_installed_programs(locales),
            application_search: String::new(),
            // Optional configuration file for an application.
            config: cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
                .map(|context| {
                    Config::get_entry(&context).unwrap_or_else(|(_errors, config)| config)
                })
                .unwrap_or_default(),

            apps_per_type: apps_hash,

            selected_type: None,
            selected_index: None,

            global_search: None,
            global_search_id: widget::Id::unique(),
        };

        // Create a startup command that sets the window title.
        let command = app.update_title();

        (app, command)
    }

    fn dialog(&self) -> Option<Element<Self::Message>> {
        if self.selected_index.is_some() {
            return Some(
                widget::dialog()
                    .title(fl!("dialog-remove-application-title"))
                    .icon(icon::from_name("dialog-error-symbolic").size(64))
                    .body(fl!("dialog-remove-application-body"))
                    .secondary_action(widget::button::destructive(fl!("action-yes")).on_press(
                        Message::RemoveApplicationConfirm(self.selected_index.unwrap()),
                    ))
                    .primary_action(
                        widget::button::suggested(fl!("action-no"))
                            .on_press(Message::RemoveApplicationCancel),
                    )
                    .into(),
            );
        }
        None
    }

    fn context_drawer(&self) -> Option<context_drawer::ContextDrawer<Self::Message>> {
        if !self.core.window.show_context {
            return None;
        }

        Some(match self.context_page {
            ContextPage::AddApplication => {
                let search = widget::search_input("Type to search...", &self.application_search)
                    .on_input(Message::ApplicationSearch)
                    .on_clear(Message::ApplicationSearch(String::new()));

                let search_input = &self.application_search.trim().to_lowercase();

                let mut list = widget::list_column()
                    .padding(theme::active().cosmic().space_xs())
                    .list_item_padding(0);

                for program in self.installed_apps.iter() {
                    if search_input.is_empty()
                        || (program
                            .name(&freedesktop_desktop_entry::get_languages_from_env())
                            .unwrap_or("".into()))
                        .to_lowercase()
                        .contains(search_input)
                    {
                        let mut app_name = program.clone().appid;

                        if let Some(name) = &program.name(&self.locales) {
                            app_name = name.to_string();
                        }

                        let icon_name = program.icon().unwrap_or("application-default");

                        let app_item_row = cosmic::iced::widget::row![
                            icon::from_name(icon_name).size(32),
                            cosmic::iced::widget::column![
                                widget::text::title4(app_name).size(24),
                                widget::text::caption(program.exec().unwrap_or(""))
                            ]
                        ]
                        .spacing(theme::active().cosmic().space_m())
                        .align_y(Vertical::Center);

                        list = list.add(
                            widget::button::custom(app_item_row)
                                .on_press(Message::AddApplication(program.clone()))
                                .width(Length::Fill)
                                .class(Button::ListItem),
                        );
                    }
                }

                context_drawer::context_drawer(
                    cosmic::iced::widget::column![search, list]
                        .spacing(theme::active().cosmic().space_m()),
                    Message::ToggleContextPage(ContextPage::AddApplication),
                )
                .title(fl!("add-application"))
            }
        })
    }

    fn header_end(&self) -> Vec<Element<Self::Message>> {
        let mut elements = Vec::with_capacity(2);

        if let Some(search) = &self.global_search {
            elements.push(
                widget::text_input::search_input("", search)
                    .width(Length::Fixed(240.0))
                    .id(self.global_search_id.clone())
                    .on_clear(Message::GlobalSearchClear)
                    .on_input(Message::GlobalSearchInput)
                    .into(),
            );
        } else {
            elements.push(
                widget::button::icon(icon::from_name("system-search-symbolic"))
                    .on_press(Message::GlobalSearchActivate)
                    .padding(8)
                    .selected(true)
                    .into(),
            );
        }

        elements
    }

    /// Describes the interface based on the current state of the application model.
    ///
    /// Application events will be processed through the view. Any messages emitted by
    /// events received by widgets will be passed to the update method.
    fn view(&self) -> Element<Self::Message> {
        let cosmic::cosmic_theme::Spacing {
            space_s,
            space_xs,
            space_l,
            ..
        } = cosmic::theme::active().cosmic().spacing;

        let mut sections = column().spacing(space_l);

        let header = column()
            .push(widget::text::title1(fl!("window-title")))
            .push(widget::text(fl!("application-description")));

        sections = sections.push(header);

        let available_types = vec![DirectoryType::User, DirectoryType::System];

        for directory_type in available_types {
            let mut section = column().spacing(space_s);

            let (section_name, section_description) = match directory_type {
                DirectoryType::User => (
                    fl!("user-applications"),
                    fl!("user-applications", "description"),
                ),
                DirectoryType::System => (
                    fl!("system-applications"),
                    fl!("system-applications", "description"),
                ),
            };

            section = section.push(
                column()
                    .push(widget::text::heading(section_name).size(18.0))
                    .push(widget::text(section_description)),
            );

            let mut valid_apps = 0;
            let search_input = match &self.global_search {
                None => "",
                Some(search) => &search.trim().to_lowercase(),
            };

            if let Some(apps) = self.apps_per_type.get(&directory_type) {
                if apps.len() > 0 {
                    let mut list_col = list_column().style(List);
                    for app in apps {
                        let app_name = match app.name(&self.locales) {
                            Some(name) => name.to_string(),
                            None => app.appid.to_owned(),
                        };

                        let app_exec = app.exec().expect("invalid state");

                        if search_input.is_empty()
                            || app_name.to_lowercase().contains(search_input)
                            || app_exec.to_lowercase().contains(search_input)
                        {
                            valid_apps = valid_apps + 1;

                            let mut row = row::with_capacity(3)
                                .spacing(space_xs)
                                .align_y(Alignment::Center);

                            row = row.push(
                                icon::from_name(app.icon().unwrap_or("application-default"))
                                    .size(32),
                            );

                            let mut name_col = column().align_x(Alignment::Start);

                            name_col =
                                name_col.push(widget::text::heading(app_name).width(Length::Fill));
                            name_col = name_col.push(exec_line(String::from(app_exec)));

                            row = row.push(name_col);

                            // actions
                            row = row.push(
                                widget::row()
                                    .spacing(space_xs)
                                    .push(
                                        button::icon(icon::from_name("edit-delete-symbolic"))
                                            .extra_small(),
                                    )
                                    .push(
                                        button::icon(icon::from_name("view-more-symbolic"))
                                            .extra_small(),
                                    ),
                            );

                            list_col = list_col.add(row);
                        }
                    }

                    if valid_apps > 0 {
                        section = section.push(list_col);

                        // @todo: get directory type
                        if search_input.is_empty() {
                            let controls = widget::container(
                                row()
                                    .spacing(space_xs)
                                    .push(
                                        widget::button::standard(fl!("add-script")).trailing_icon(
                                            icon::from_name("window-pop-out-symbolic"),
                                        ),
                                    )
                                    .push(
                                        widget::button::suggested(fl!("add-application"))
                                            .trailing_icon(icon::from_name("list-add-symbolic"))
                                            .on_press(Message::ToggleContextPage(
                                                ContextPage::AddApplication,
                                            )),
                                    ),
                            )
                            .width(Length::Fill)
                            .align_x(Alignment::End);
                            section = section.push(controls);
                        }
                    } else {
                        section = section.push(
                            list_column()
                                .style(List)
                                .add(widget::text::heading(fl!("no-applications-found"))),
                        );
                    }
                } else {
                    section = section.push(
                        list_column().style(List).add(
                            cosmic::iced::widget::column![
                                widget::text::title3(fl!("no-applications-selected")),
                                widget::text::caption(fl!("no-applications-caption"))
                            ]
                            .width(Length::Fill)
                            .align_x(Horizontal::Center),
                        ),
                    );
                }
            }

            sections = sections.push(section);
        }

        sections = sections.push(vertical_space().height(Length::Fixed(64.0)));

        widget::container(
            widget::scrollable(sections)
                .height(Length::Fill)
                .spacing(space_l),
        )
        // fill the full application window
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Horizontal::Left)
        .align_y(Vertical::Top)
        .padding([0, 0, 0, space_l])
        .into()
    }

    /// Register subscriptions for this application.
    ///
    /// Subscriptions are long-running async tasks running in the background which
    /// emit messages to the application through a channel. They are started at the
    /// beginning of the application, and persist through its lifetime.
    fn subscription(&self) -> Subscription<Self::Message> {
        struct MySubscription;

        Subscription::batch(vec![
            // Create a subscription which emits updates through a channel.
            Subscription::run_with_id(
                std::any::TypeId::of::<MySubscription>(),
                cosmic::iced::stream::channel(4, move |mut channel| async move {
                    _ = channel.send(Message::SubscriptionChannel).await;

                    futures_util::future::pending().await
                }),
            ),
            // Watch for application configuration changes.
            self.core()
                .watch_config::<Config>(Self::APP_ID)
                .map(|update| {
                    // for why in update.errors {
                    //     tracing::error!(?why, "app config error");
                    // }

                    Message::UpdateConfig(update.config)
                }),
        ])
    }

    /// Handles messages emitted by the application and its widgets.
    ///
    /// Tasks may be returned for asynchronous execution of code in the background
    /// on the application's async runtime.
    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            Message::SubscriptionChannel => {
                // For example purposes only.
            }
            Message::UpdateConfig(config) => {
                self.config = config;
            }
            Message::ToggleContextPage(context_page) => {
                if self.context_page == context_page {
                    self.core.window.show_context = !self.core.window.show_context;
                } else {
                    self.context_page = context_page;
                    self.core.window.show_context = true;
                }
            }
            Message::ApplicationSearch(search) => {
                self.application_search = search;
            }
            Message::AddApplication(desktop_entry) => {
                /*let mut new_programs = self.selected_programs.take().unwrap_or(Vec::new());
                new_programs.push(program);

                self.selected_programs = Some(new_programs);

                let mut settings = Vec::new();
                for program in self.selected_programs.as_deref().unwrap() {
                    settings.push(&program.settings);
                }

                save_settings(&self.selected_programs.clone().unwrap());

                if self.context_page == ContextPage::AddApplication {
                    return cosmic::task::message(ToggleContextPage(ContextPage::AddApplication));
                }*/
            }
            Message::RemoveApplication(idx) => {
                /*if self.selected_programs.is_some() && self.program_to_remove.is_none() {
                    self.program_to_remove = Some(idx);
                }*/
            }
            Message::RemoveApplicationConfirm(idx) => {
                /*if self.selected_programs.is_some() && self.program_to_remove.is_some() {
                    let mut new_programs = self.selected_programs.take().unwrap();
                    if new_programs.len() > idx {
                        new_programs.remove(idx);

                        self.selected_programs = Some(new_programs.clone());
                        save_settings(&self.selected_programs.clone().unwrap());

                        // help reset UI
                        if new_programs.len() == 0 {
                            self.selected_programs = None;
                        }
                    }

                    // always unset program_to_remove, in case we end up in some weird funky edge case
                    self.program_to_remove = None;
                }*/
            }
            Message::RemoveApplicationCancel => {
                self.selected_index = None;
            }
            Message::GlobalSearchActivate => {
                self.global_search = Some(String::new());
                return widget::text_input::focus(self.global_search_id.clone());
            }
            Message::GlobalSearchInput(search) => {
                self.global_search = Some(search);
            }
            Message::GlobalSearchClear => {
                self.global_search = None;
            }
        }
        Task::none()
    }
}

impl AppModel {
    /// Updates the header and window titles.
    pub fn update_title(&mut self) -> Task<Message> {
        let window_title = fl!("app-title");

        if let Some(id) = self.core.main_window_id() {
            self.set_window_title(window_title, id)
        } else {
            Task::none()
        }
    }
}

fn exec_line<'a>(text: String) -> Text<'a, Theme, Renderer> {
    widget::text::monotext(text).size(10.0)
}
